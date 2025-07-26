# Cassette Dart Loader

A Dart implementation of the Cassette loader for loading and executing Nostr event cassettes.

## Installation

Add this to your `pubspec.yaml`:

```yaml
dependencies:
  cassette_loader: ^0.1.0
```

## Usage

```dart
import 'dart:convert';
import 'package:cassette_loader/cassette_loader.dart';

void main() async {
  // Load a cassette
  final cassette = await Cassette.load('path/to/cassette.wasm', debug: true);
  
  // Get cassette description
  final desc = cassette.describe();
  print('Description: $desc');
  
  // Send a REQ message - automatically collects all events until EOSE
  final req = jsonEncode(['REQ', 'sub1', {'limit': 10}]);
  final result = cassette.send(req);
  
  // Check if result is List (REQ) or String (other messages)
  if (result is List<String>) {
    print('Received ${result.length} events');
    for (final event in result) {
      print('Event: $event');
    }
  } else {
    print('Single response: $result');
  }
  
  // Send a CLOSE message - returns single response
  final closeMsg = jsonEncode(['CLOSE', 'sub1']);
  final closeResult = cassette.send(closeMsg) as String;
  print('CLOSE Result: $closeResult');
  
  // Send a COUNT message (NIP-45) - returns single response
  final countMsg = jsonEncode(['COUNT', 'count-sub', {'kinds': [1]}]);
  final countResult = cassette.send(countMsg) as String;
  print('COUNT Result: $countResult');
  
  // Get relay info (NIP-11)
  try {
    final info = cassette.info();
    print('Relay Info: $info');
  } catch (e) {
    print('Info not available: $e');
  }
}
```

## Features

- Full WebAssembly support via dart:wasm
- Unified `send` method for all NIP-01 messages
- **Automatic looping for REQ messages** - `send` returns `List<String>` for REQ, `String` for others
- MSGB format support for memory operations
- Event deduplication (automatically reset on new REQ messages)
- Newline-separated message handling
- Async file loading support
- Debug logging support
- Automatic synthesis of `describe()` from `info()` method

## Important: Loop Behavior

Unlike WebSocket connections, cassettes return one message per `send` call. The `send` method now automatically detects REQ messages and loops until EOSE, returning all events as `List<String>`. For other message types, it returns a single `String`.

## Web Support

For web applications, use `loadFromBytes` instead of `load`:

```dart
import 'dart:html' as html;
import 'dart:typed_data';
import 'package:cassette_loader/cassette_loader.dart';

void loadCassetteWeb() async {
  // Fetch the WASM file
  final response = await html.HttpRequest.request(
    'cassette.wasm',
    responseType: 'arraybuffer',
  );
  
  final bytes = Uint8List.view(response.response as ByteBuffer);
  final cassette = Cassette.loadFromBytes(bytes, debug: true);
  
  // Use cassette as normal
  final desc = cassette.describe();
  print('Description: $desc');
}
```

## Requirements

- Dart SDK 3.0.0 or later
- wasm package 3.0.0 or later