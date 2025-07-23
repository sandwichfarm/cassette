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
  
  // Send a REQ message
  final req = jsonEncode(['REQ', 'sub1', {'limit': 10}]);
  
  // Loop to get all events
  while (true) {
    final result = cassette.req(req);
    
    if (result.isEmpty) {
      break;
    }
    
    print('Result: $result');
    
    // Check for EOSE
    if (result.contains('"EOSE"')) {
      break;
    }
  }
}
```

## Features

- Full WebAssembly support via dart:wasm
- MSGB format support for memory operations
- Event deduplication
- Newline-separated message handling
- Async file loading support
- Debug logging support

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