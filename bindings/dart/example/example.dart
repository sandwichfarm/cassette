import 'dart:convert';
import 'dart:io';
import 'package:cassette_loader/cassette_loader.dart';

void main(List<String> args) async {
  if (args.isEmpty) {
    print('Usage: dart example.dart <cassette.wasm>');
    exit(1);
  }

  try {
    // Load cassette with debug enabled
    final cassette = await Cassette.load(args[0], debug: true);
    
    // Get and display description
    final desc = cassette.describe();
    print('Cassette Description:');
    print(desc);
    print('');
    
    // Create a REQ message
    final req = jsonEncode(['REQ', 'example-sub', {'limit': 5}]);
    print('Sending REQ: $req');
    print('');
    
    // Send REQ - automatically collects all events until EOSE
    final result = cassette.scrub(req);
    
    int eventCount = 0;
    if (result is List<String>) {
      // REQ messages return a list of events
      for (final event in result) {
        print('Received: $event');
        if (event.contains('"EVENT"')) {
          eventCount++;
        }
      }
    } else {
      // Other messages return a single string
      print('Received: $result');
    }
    
    print('');
    print('Total events received: $eventCount');
    
    // Test CLOSE
    final closeMsg = jsonEncode(['CLOSE', 'example-sub']);
    print('');
    print('Sending CLOSE: $closeMsg');
    final closeResult = cassette.scrub(closeMsg);
    print('CLOSE result: $closeResult');
    
  } catch (e) {
    print('Error: $e');
    exit(1);
  }
}