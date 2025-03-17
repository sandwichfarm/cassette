# Cassette Loader Usage Guide

## Basic Usage

The cassette-loader library provides a simple way to load and interact with WebAssembly cassettes for Nostr:

```javascript
import { loadCassette, isWebAssemblySupported, ENV_INFO } from 'cassette-loader';

// Check if WebAssembly is supported in this environment
if (!isWebAssemblySupported()) {
  console.error("WebAssembly is not supported");
  process.exit(1);
}

// Log environment information
console.log("Environment:", ENV_INFO);

// Load a cassette from a file path
async function loadMyCassette() {
  try {
    const result = await loadCassette('/path/to/my-cassette.wasm');
    
    if (result.success && result.cassette) {
      console.log('Cassette loaded successfully!');
      console.log('Cassette ID:', result.cassette.id);
      console.log('Cassette Name:', result.cassette.name);
      console.log('Cassette Description:', result.cassette.description);
      console.log('Cassette Version:', result.cassette.version);
      
      // Get metadata from the cassette
      const metadata = result.cassette.methods.describe();
      console.log('Metadata:', metadata);
      
      // Process a request with the cassette
      const reqStr = JSON.stringify(['REQ', 'my-sub-id', { kinds: [1], limit: 10 }]);
      const response = result.cassette.methods.req(reqStr);
      console.log('Response:', response);
      
      // Close a subscription if the cassette supports it
      if (result.cassette.methods.close) {
        const closeStr = JSON.stringify(['CLOSE', 'my-sub-id']);
        result.cassette.methods.close(closeStr);
      }
    } else {
      console.error('Failed to load cassette:', result.error);
    }
  } catch (error) {
    console.error('Error loading cassette:', error);
  }
}

loadMyCassette();
```

## Advanced Usage with Options

You can provide additional options when loading cassettes:

```javascript
const result = await loadCassette('/path/to/cassette.wasm', 'custom-filename.wasm', {
  // Enable debug logging
  debug: true,
  
  // Set initial memory size in pages (64KB per page)
  memoryInitialSize: 16,
  
  // Expose raw WebAssembly exports in the result
  exposeExports: true,
  
  // Custom imports for the WebAssembly module
  customImports: {
    env: {
      // Custom environment functions
      custom_function: (arg) => console.log('Custom function called with:', arg)
    }
  }
});
```

## Browser Integration

For browser environments, you can use the included `CassetteManager` class:

```javascript
import { CassetteManager } from 'cassette-loader/browser';

// Create a new cassette manager
const manager = new CassetteManager();

// Add event listeners
manager.addEventListener('cassette-loaded', (cassette) => {
  console.log(`Loaded cassette: ${cassette.name}`);
});

manager.addEventListener('cassette-error', (error) => {
  console.error('Error:', error);
});

// Load a cassette from a URL
const cassette = await manager.loadCassetteFromUrl('/cassettes/my-cassette.wasm');

// Load a cassette from a File object (e.g., from a file input or drag & drop)
const fileInput = document.getElementById('file-input');
fileInput.addEventListener('change', async (event) => {
  const files = event.target.files;
  for (const file of files) {
    if (file.name.endsWith('.wasm')) {
      await manager.loadCassetteFromFile(file);
    }
  }
});

// Process requests with all loaded cassettes
const responses = manager.processRequestAll('["REQ", "test-sub", {"kinds": [1], "limit": 10}]');
for (const [id, response] of responses) {
  console.log(`Response from ${id}:`, response);
}
```

## Environment Information

The library provides information about the current environment:

```javascript
import { ENV_INFO } from 'cassette-loader';

console.log(ENV_INFO);
// Output: {
//   isNode: true|false,
//   isBrowser: true|false,
//   webAssembly: true|false,
//   version: '1.0.0'
// }
```

## API Reference

### Main Functions

- `loadCassette(source, fileName?, options?)`: Loads a WebAssembly cassette from a file, URL, or ArrayBuffer
- `isWebAssemblySupported()`: Checks if WebAssembly is supported in the current environment

### Types

- `CassetteLoadResult`: Result of loading a cassette
  - `success`: Boolean indicating success
  - `cassette`: The loaded cassette (if success is true)
  - `error`: Error message (if success is false)

- `Cassette`: Loaded cassette object
  - `id`: Unique identifier
  - `fileName`: Original file name
  - `name`: Cassette name from metadata
  - `description`: Cassette description
  - `version`: Version string
  - `methods`: Methods to interact with the cassette
    - `describe()`: Get cassette metadata
    - `req(requestStr)`: Process a request
    - `close(closeStr)?`: Close a subscription (if supported)

### Browser Integration

- `CassetteManager`: Class for managing cassettes in browser environments
  - `loadCassetteFromUrl(url)`: Load a cassette from a URL
  - `loadCassetteFromFile(file)`: Load a cassette from a File object
  - `processRequest(cassetteId, request)`: Process a request with a specific cassette
  - `processRequestAll(request)`: Process a request with all loaded cassettes
  - `addEventListener(event, callback)`: Add an event listener
  - `removeEventListener(event, callback)`: Remove an event listener
  - `getCassettes()`: Get all loaded cassettes
  - `getCassette(id)`: Get a cassette by ID
  - `removeCassette(id)`: Remove a cassette 