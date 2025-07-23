# Cassette GUI

Web-based interface for testing and interacting with Nostr event cassettes directly in the browser.

## Overview

The Cassette GUI provides a browser-based environment to:
- Load and test WebAssembly cassettes
- Send NIP-01 protocol messages (REQ, CLOSE)
- View cassette metadata and schemas
- Debug cassette responses
- Test event filtering

## Features

- **Direct WASM Loading**: Load cassettes without a server
- **Interactive Testing**: Send custom REQ messages with various filters
- **Real-time Response Display**: See EVENT, EOSE, and NOTICE messages
- **Memory Management Visualization**: Monitor memory usage
- **Multiple Cassette Support**: Load and switch between different cassettes

## Usage

### Starting the Server

```bash
# Install dependencies
npm install

# Start the development server
npm start

# Or with auto-reload
npm run dev
```

Open http://localhost:8080 in your browser.

### Loading a Cassette

1. Click "Choose File" and select a `.wasm` cassette file
2. The cassette metadata will display once loaded
3. Use the interface to send test requests

### Testing Queries

#### Basic Request
```json
["REQ", "sub1", {}]
```

#### Filtered Request
```json
["REQ", "sub1", {
  "kinds": [1],
  "limit": 10,
  "authors": ["pubkey..."]
}]
```

#### Tag Filtering
```json
["REQ", "sub1", {
  "#t": ["bitcoin", "nostr"],
  "kinds": [1, 30023]
}]
```

## Interface Components

### Main View (`index.html`)
- Full-featured testing interface
- Cassette file loader
- Request builder with JSON editor
- Response viewer with syntax highlighting
- Memory usage statistics

### Legacy View (`cassette-test-original.html`)
- Simplified testing interface
- Basic request/response functionality
- Useful for debugging

## Architecture

The GUI uses the JavaScript loader library to:
1. Load WebAssembly modules in the browser
2. Handle memory management between JavaScript and WASM
3. Provide a consistent interface for cassette communication

```javascript
import { CassetteLoader } from '../loaders/js/dist/browser/cassette-loader.js';

// Load a cassette
const loader = new CassetteLoader();
const cassette = await loader.loadFromFile(wasmFile);

// Get metadata
const metadata = cassette.describe();

// Send a request
const response = cassette.req(["REQ", "sub1", { kinds: [1] }]);
```

## Example Requests

### Get All Events
```javascript
cassette.req(["REQ", "subscription-id", {}])
```

### Filter by Kind
```javascript
cassette.req(["REQ", "sub1", { kinds: [1, 7] }])
```

### Time-based Filtering
```javascript
cassette.req(["REQ", "sub1", {
  since: 1700000000,
  until: 1700100000
}])
```

### Complex Filter
```javascript
cassette.req(["REQ", "sub1", {
  kinds: [1],
  authors: ["pubkey1", "pubkey2"],
  "#t": ["bitcoin"],
  limit: 50
}])
```

## Development

### File Structure
```
gui/
├── index.html              # Main testing interface
├── cassette-test-original.html  # Legacy interface
├── nip01-examples.js       # Example NIP-01 requests
├── lib/                    # External libraries
├── package.json            # Node dependencies
└── server.js              # Express server (if needed)
```

### Adding Features

To extend the GUI:

1. Modify `index.html` for UI changes
2. Update request builders in the JavaScript sections
3. Add new example requests to `nip01-examples.js`

### Browser Compatibility

The GUI requires a modern browser with:
- WebAssembly support
- ES6 modules
- Async/await

Tested on:
- Chrome/Edge 90+
- Firefox 89+
- Safari 14.1+

## Tips

- Use browser DevTools to inspect WASM memory usage
- Check the Console for detailed debug information
- The Network tab shows cassette file loading
- Use the Performance tab to profile cassette queries

## Related Tools

- **CLI**: Command-line interface for cassette operations
- **Boombox**: WebSocket server for serving cassettes as relays
- **loaders/js**: JavaScript library for loading cassettes