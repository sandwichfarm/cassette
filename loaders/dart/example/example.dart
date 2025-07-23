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
    
    // Loop to get all events
    int eventCount = 0;
    while (true) {
      final result = cassette.req(req);
      
      if (result.isEmpty) {
        print('No more events');
        break;
      }
      
      print('Received: $result');
      
      // Count events
      if (result.contains('"EVENT"')) {
        // Handle newline-separated events
        final lines = result.split('\n');
        for (final line in lines) {
          if (line.contains('"EVENT"')) {
            eventCount++;
          }
        }
      }
      
      // Check for EOSE
      if (result.contains('"EOSE"')) {
        print('End of stored events');
        break;
      }
    }
    
    print('');
    print('Total events received: $eventCount');
    
    // Test CLOSE
    final closeMsg = jsonEncode(['CLOSE', 'example-sub']);
    print('');
    print('Sending CLOSE: $closeMsg');
    final closeResult = cassette.close(closeMsg);
    print('CLOSE result: $closeResult');
    
  } catch (e) {
    print('Error: $e');
    exit(1);
  }
}