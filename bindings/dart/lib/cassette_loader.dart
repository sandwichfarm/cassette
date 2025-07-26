import 'dart:convert';
import 'dart:typed_data';
import 'package:wasm/wasm.dart';

/// Event tracker for deduplication
class EventTracker {
  final Set<String> _eventIds = {};

  void reset() {
    _eventIds.clear();
  }

  bool addAndCheck(String eventId) {
    return _eventIds.add(eventId);
  }
}

/// Memory manager for WASM operations
class MemoryManager {
  final Memory memory;
  final WasmFunction allocFunc;

  MemoryManager({required this.memory, required this.allocFunc});

  int writeString(String str) {
    final bytes = utf8.encode(str);
    final ptr = allocFunc.call([bytes.length]) as int;
    
    if (ptr == 0) {
      throw Exception('Allocation failed');
    }

    final memoryData = memory.view;
    for (int i = 0; i < bytes.length; i++) {
      memoryData[ptr + i] = bytes[i];
    }

    return ptr;
  }

  String readString(int ptr) {
    if (ptr == 0) {
      throw Exception('Null pointer');
    }

    final memoryData = memory.view;
    
    // Check for MSGB format
    if (ptr + 8 <= memoryData.length) {
      final signature = utf8.decode(memoryData.sublist(ptr, ptr + 4));
      if (signature == 'MSGB') {
        // Read length (little endian)
        final lengthBytes = memoryData.sublist(ptr + 4, ptr + 8);
        final length = lengthBytes[0] | 
                      (lengthBytes[1] << 8) | 
                      (lengthBytes[2] << 16) | 
                      (lengthBytes[3] << 24);
        
        if (ptr + 8 + length <= memoryData.length) {
          return utf8.decode(memoryData.sublist(ptr + 8, ptr + 8 + length));
        }
      }
    }

    // Fall back to null-terminated string
    int end = ptr;
    while (end < memoryData.length && memoryData[end] != 0) {
      end++;
    }

    return utf8.decode(memoryData.sublist(ptr, end));
  }
}

/// Cassette loader for Dart
class Cassette {
  late final WasmInstance _instance;
  late final MemoryManager _memoryManager;
  late final EventTracker _eventTracker;
  late final WasmFunction _sendFunc;
  WasmFunction? _infoFunc;
  WasmFunction? _deallocFunc;
  WasmFunction? _getSizeFunc;
  
  final bool debug;

  Cassette._(this._instance, this.debug) {
    // Get memory
    final memory = _instance.exports.whereType<Memory>().firstWhere(
      (export) => export.name == 'memory',
      orElse: () => throw Exception('Memory export not found'),
    );

    // Get alloc function
    final allocFunc = _instance.exports.whereType<WasmFunction>().firstWhere(
      (export) => export.name == 'alloc_string',
      orElse: () => throw Exception('alloc_string function not found'),
    );

    _memoryManager = MemoryManager(memory: memory, allocFunc: allocFunc);
    _eventTracker = EventTracker();

    // Get required functions
    _sendFunc = _instance.exports.whereType<WasmFunction>().firstWhere(
      (export) => export.name == 'send',
      orElse: () => throw Exception('send function not found'),
    );

    // Get optional functions

    try {
      _infoFunc = _instance.exports.whereType<WasmFunction>().firstWhere(
        (export) => export.name == 'info',
      );
    } catch (_) {}

    try {
      _deallocFunc = _instance.exports.whereType<WasmFunction>().firstWhere(
        (export) => export.name == 'dealloc_string',
      );
    } catch (_) {}

    try {
      _getSizeFunc = _instance.exports.whereType<WasmFunction>().firstWhere(
        (export) => export.name == 'get_allocation_size',
      );
    } catch (_) {}
  }

  /// Load a cassette from a file
  static Future<Cassette> load(String path, {bool debug = false}) async {
    final module = await WasmModule.compileFromFile(path);
    final instance = module.instantiate();
    return Cassette._(instance, debug);
  }

  /// Load a cassette from bytes
  static Cassette loadFromBytes(Uint8List bytes, {bool debug = false}) {
    final module = WasmModule.compile(bytes);
    final instance = module.instantiate();
    return Cassette._(instance, debug);
  }

  /// Get cassette description
  String describe() {
    // Use info() to synthesize a description
    final infoStr = info();
    try {
      final infoData = jsonDecode(infoStr) as Map<String, dynamic>;
      final supportedNips = infoData['supported_nips'] as List<dynamic>? ?? [];
      final name = infoData['name'] as String? ?? 'Cassette';
      final description = infoData['description'] as String?;
      
      if (description != null) {
        return description;
      }
      
      if (supportedNips.isNotEmpty) {
        return '$name - Supports NIPs: ${supportedNips.join(', ')}';
      }
      
      return name;
    } catch (_) {
      return 'Cassette';
    }
  }

  /// Send any NIP-01 message
  /// For REQ messages, returns a List<String>. For other messages, returns a String.
  dynamic send(String message) {
    // Parse message to determine type
    bool isReqMessage = false;
    String subscriptionId = '';
    
    try {
      final msgData = jsonDecode(message) as List;
      if (msgData.length >= 2) {
        final msgType = msgData[0] as String;
        
        if (msgType == 'REQ') {
          _eventTracker.reset();
          if (debug) {
            print('[Cassette] New REQ, resetting event tracker');
          }
          isReqMessage = true;
          subscriptionId = msgData[1] as String;
        } else if (msgType == 'CLOSE') {
          // Handle CLOSE messages if needed
          _eventTracker.reset();
          if (debug) {
            print('[Cassette] Processing CLOSE message');
          }
        }
      }
    } catch (_) {}

    // If it's a REQ message, collect all events until EOSE
    if (isReqMessage) {
      return _collectAllEventsForReq(message, subscriptionId);
    }
    
    // For non-REQ messages, use single call
    return _sendSingle(message);
  }

  // Private method for single send call
  String _sendSingle(String message) {
    // Write message to memory
    final msgPtr = _memoryManager.writeString(message);

    // Call send function
    final resultPtr = _sendFunc.call([msgPtr, message.length]) as int;

    // Deallocate message
    _deallocFunc?.call([msgPtr, message.length]);

    if (resultPtr == 0) {
      return jsonEncode(['NOTICE', 'send() returned null pointer']);
    }

    // Read result
    final resultStr = _memoryManager.readString(resultPtr);

    // Deallocate result
    if (_deallocFunc != null) {
      int size = resultStr.length;
      if (_getSizeFunc != null) {
        try {
          size = _getSizeFunc!.call([resultPtr]) as int;
        } catch (_) {}
      }
      _deallocFunc!.call([resultPtr, size]);
    }

    return _processResults(resultStr);
  }
  
  // Process results with event deduplication
  String _processResults(String resultStr) {
    // Handle newline-separated messages
    if (resultStr.contains('\n')) {
      final messages = resultStr.trim().split('\n');
      if (debug) {
        print('[Cassette] Processing ${messages.length} newline-separated messages');
      }

      final filteredMessages = <String>[];
      for (final message in messages) {
        try {
          final parsed = jsonDecode(message) as List;
          
          if (parsed.length < 2) {
            if (debug) {
              print('[Cassette] Invalid message format: ${message.substring(0, message.length > 100 ? 100 : message.length)}');
            }
            continue;
          }

          final msgType = parsed[0] as String;
          if (!['NOTICE', 'EVENT', 'EOSE', 'OK', 'CLOSED'].contains(msgType)) {
            if (debug) {
              print('[Cassette] Unknown message type: $msgType');
            }
            continue;
          }

          // Filter duplicate events
          if (msgType == 'EVENT' && parsed.length >= 3) {
            final event = parsed[2] as Map<String, dynamic>;
            final eventId = event['id'] as String?;
            if (eventId != null && !_eventTracker.addAndCheck(eventId)) {
              if (debug) {
                print('[Cassette] Filtering duplicate event: $eventId');
              }
              continue;
            }
          }

          filteredMessages.add(message);
        } catch (e) {
          if (debug) {
            print('[Cassette] Failed to parse message: $e');
          }
        }
      }

      return filteredMessages.join('\n');
    }

    // Single message - check for duplicate
    try {
      final parsed = jsonDecode(resultStr) as List;
      if (parsed.length >= 3 && parsed[0] == 'EVENT') {
        final event = parsed[2] as Map<String, dynamic>;
        final eventId = event['id'] as String?;
        if (eventId != null && !_eventTracker.addAndCheck(eventId)) {
          if (debug) {
            print('[Cassette] Filtering duplicate event: $eventId');
          }
          return '';
        }
      }
    } catch (_) {}

    return resultStr;
  }
  
  // Private method to collect all events for REQ messages
  List<String> _collectAllEventsForReq(String message, String subscriptionId) {
    if (debug) {
      print('[Cassette] Collecting all events for REQ subscription: $subscriptionId');
    }
    
    final results = <String>[];
    
    // Keep calling until we get EOSE or terminating condition
    while (true) {
      final response = _sendSingle(message);
      
      // Empty response means no more events
      if (response.isEmpty) {
        if (debug) {
          print('[Cassette] Received empty response, stopping');
        }
        break;
      }
      
      // Try to parse the response
      try {
        final parsed = jsonDecode(response) as List;
        if (parsed.isNotEmpty) {
          final msgType = parsed[0] as String;
          
          if (msgType == 'EOSE') {
            if (debug) {
              print('[Cassette] Received EOSE for subscription $subscriptionId');
            }
            results.add(response);
            break;
          } else if (msgType == 'CLOSED') {
            if (debug) {
              print('[Cassette] Received CLOSED for subscription $subscriptionId');
            }
            results.add(response);
            break;
          }
        }
      } catch (e) {
        if (debug) {
          print('[Cassette] Failed to parse response: $e, stopping');
        }
        break;
      }
      
      // Add the response to results
      results.add(response);
    }
    
    // Check if we have an EOSE message
    bool hasEose = false;
    for (final r in results) {
      try {
        final parsed = jsonDecode(r) as List;
        if (parsed.isNotEmpty && parsed[0] == 'EOSE') {
          hasEose = true;
          break;
        }
      } catch (_) {}
    }
    
    // If no EOSE, add one
    if (!hasEose) {
      results.add(jsonEncode(['EOSE', subscriptionId]));
    }
    
    return results;
  }



  /// Get NIP-11 relay information
  String info() {
    if (_infoFunc == null) {
      // Return minimal info if function not found
      return jsonEncode({'supported_nips': []});
    }

    // Call info function
    final ptr = _infoFunc!.call([]) as int;

    if (ptr == 0) {
      return jsonEncode({'supported_nips': []});
    }

    // Read result
    final infoStr = _memoryManager.readString(ptr);

    // Try to deallocate
    _deallocFunc?.call([ptr, infoStr.length]);

    return infoStr;
  }
}