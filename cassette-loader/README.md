# Cassette Loader

A cross-platform TypeScript library for loading and interacting with Nostr WASM cassettes in both Node.js and browser environments.

## Features

- Load WASM cassettes from files, URLs, or ArrayBuffers
- Works in both Node.js and browser environments
- Standardized interface for interacting with cassettes
- Dynamic discovery of exported functions with support for various naming conventions
- Automatic handling of WASM memory management
- Event-based architecture for browser integration
- Comprehensive error handling and debugging

## Installation

```bash
npm install cassette-loader
```

## Basic Usage

```typescript
import { loadCassette, isWebAssemblySupported } from 'cassette-loader';

// Check if WebAssembly is supported
if (!isWebAssemblySupported()) {
  console.error("WebAssembly is not supported in this environment");
  process.exit(1);
}

// Load a cassette from a file or URL
async function loadMyCassette() {
  try {
    const result = await loadCassette('/path/to/cassette.wasm', 'cassette.wasm');
    
    if (result.success && result.cassette) {
      console.log(`Successfully loaded cassette: ${result.cassette.name}`);
      
      // Get cassette metadata
      const metadata = result.cassette.methods.describe();
      console.log('Cassette metadata:', metadata);
      
      // Process a request
      const request = '["REQ", "subscription-id", {"kinds": [1]}]';
      const response = result.cassette.methods.req(request);
      console.log('Response:', response);
      
      // Close a subscription (if supported)
      if (result.cassette.methods.close) {
        const closeResult = result.cassette.methods.close('["CLOSE", "subscription-id"]');
        console.log('Close result:', closeResult);
      }
    } else {
      console.error(`Failed to load cassette: ${result.error}`);
    }
  } catch (error) {
    console.error('Error loading cassette:', error);
  }
}

loadMyCassette();
```

## Browser Integration

The library includes a `CassetteManager` class for browser environments that provides a clean API for loading and managing cassettes:

```typescript
import { CassetteManager } from 'cassette-loader/browser';

// Create a new cassette manager
const manager = new CassetteManager();

// Add event listeners
manager.addEventListener('cassette-loaded', (cassette) => {
  console.log(`Cassette loaded: ${cassette.name}`);
});

manager.addEventListener('cassette-error', (error) => {
  console.error('Cassette error:', error);
});

// Load cassettes from standard locations
manager.loadStandardCassettes().then(cassettes => {
  console.log(`Loaded ${cassettes.length} standard cassettes`);
});

// Load a cassette from a URL
manager.loadCassetteFromUrl('/cassettes/my-cassette.wasm').then(cassette => {
  if (cassette) {
    console.log(`Loaded cassette: ${cassette.name}`);
  }
});

// Set up drag and drop for cassettes
const dropZone = document.getElementById('drop-zone');
dropZone.addEventListener('drop', async (event) => {
  event.preventDefault();
  
  if (event.dataTransfer.items) {
    for (const item of event.dataTransfer.items) {
      if (item.kind === 'file') {
        const file = item.getAsFile();
        if (file && file.name.endsWith('.wasm')) {
          const cassette = await manager.loadCassetteFromFile(file);
          if (cassette) {
            console.log(`Loaded cassette from drop: ${cassette.name}`);
          }
        }
      }
    }
  }
});

// Process a Nostr request with all loaded cassettes
const processRequest = (request) => {
  const responses = manager.processRequestAll(request);
  for (const [id, response] of responses) {
    if (response) {
      console.log(`Response from ${id}:`, response);
    }
  }
};

// Example request
processRequest('["REQ", "my-sub", {"kinds": [1]}]');
```

## Server Integration

The library can be used on the server side to load cassettes for processing Nostr requests:

```typescript
import { loadCassette, Cassette } from 'cassette-loader';
import { WebSocketServer } from 'ws';
import http from 'http';

// Load cassettes
const cassettes = new Map<string, Cassette>();

async function loadCassettes() {
  const result = await loadCassette('/path/to/cassette.wasm');
  if (result.success && result.cassette) {
    cassettes.set(result.cassette.id, result.cassette);
  }
}

// Set up WebSocket server
const server = http.createServer();
const wss = new WebSocketServer({ server });

wss.on('connection', (ws) => {
  ws.on('message', (message) => {
    try {
      const request = message.toString();
      const event = JSON.parse(request);
      
      if (Array.isArray(event) && event[0] === 'REQ') {
        // Process the request with all cassettes
        for (const [id, cassette] of cassettes) {
          try {
            const response = cassette.methods.req(request);
            ws.send(response);
          } catch (error) {
            console.error(`Error processing request with cassette ${id}:`, error);
          }
        }
      }
    } catch (error) {
      console.error('Error processing message:', error);
    }
  });
});

// Start the server
await loadCassettes();
server.listen(3000);
```

## Advanced Configuration

The `loadCassette` function accepts options for configuring the loader:

```typescript
const result = await loadCassette('/path/to/cassette.wasm', 'cassette.wasm', {
  // Initial memory size in pages (64KB per page)
  memoryInitialSize: 16,
  
  // Enable debug logging
  debug: true,
  
  // Whether to expose the WebAssembly exports in the returned cassette
  exposeExports: true,
  
  // Custom imports for the WebAssembly module
  customImports: {
    env: {
      // Custom environment imports
      myCustomFunction: (arg) => console.log('Custom function called with:', arg)
    }
  }
});
```

## Svelte Integration Example

Here's an example of how to use the library in a Svelte application:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { CassetteManager } from 'cassette-loader/browser';
  
  let manager: CassetteManager;
  let cassettes = [];
  let loading = true;
  let error = null;
  
  onMount(async () => {
    try {
      // Initialize the manager
      manager = new CassetteManager();
      
      // Set up event listeners
      manager.addEventListener('cassette-loaded', (cassette) => {
        cassettes = manager.getCassettes();
      });
      
      manager.addEventListener('cassette-error', (err) => {
        error = err;
      });
      
      // Load standard cassettes
      await manager.loadStandardCassettes();
      
      // Update the cassette list
      cassettes = manager.getCassettes();
    } catch (err) {
      error = err;
    } finally {
      loading = false;
    }
  });
  
  async function handleFileUpload(event) {
    const files = event.target.files;
    for (const file of files) {
      if (file.name.endsWith('.wasm')) {
        await manager.loadCassetteFromFile(file);
      }
    }
  }
  
  function processRequest(cassetteId, request) {
    const response = manager.processRequest(cassetteId, request);
    return response;
  }
</script>

<div>
  <h1>Nostr Cassettes</h1>
  
  {#if loading}
    <p>Loading cassettes...</p>
  {:else if error}
    <p class="error">Error: {error.message || error}</p>
  {:else}
    <div class="upload-zone">
      <h2>Upload Cassette</h2>
      <input type="file" accept=".wasm" on:change={handleFileUpload} multiple />
    </div>
    
    <div class="cassettes">
      <h2>Loaded Cassettes ({cassettes.length})</h2>
      {#if cassettes.length === 0}
        <p>No cassettes loaded. Upload a .wasm file or drag and drop it here.</p>
      {:else}
        {#each cassettes as cassette}
          <div class="cassette-card">
            <h3>{cassette.name}</h3>
            <p>{cassette.description}</p>
            <p>Version: {cassette.version}</p>
            <button on:click={() => manager.removeCassette(cassette.id)}>
              Remove
            </button>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .error {
    color: red;
  }
  .cassette-card {
    border: 1px solid #ccc;
    padding: 1rem;
    margin-bottom: 1rem;
    border-radius: 4px;
  }
  .upload-zone {
    border: 2px dashed #aaa;
    padding: 2rem;
    text-align: center;
    margin-bottom: 2rem;
    border-radius: 4px;
  }
</style>
```

## Browser Usage

The cassette-loader library is now bundled for browser use with esbuild. You can use it in your web applications in three ways:

### 1. ES Module Import (Recommended)

```html
<script type="module">
  import { CassetteManager, loadCassette } from './path/to/cassette-loader.js';
  
  // Initialize the CassetteManager
  const manager = new CassetteManager();
  
  // Load a cassette from a URL
  const cassette = await manager.loadCassetteFromUrl('path/to/your-cassette.wasm');
  
  // Process a request
  const request = JSON.stringify(['REQ', 'subscription-id', { kinds: [1], limit: 5 }]);
  const response = manager.processRequest(cassette.id, request);
  console.log('Response:', response);
</script>
```

### 2. UMD Bundle (For traditional script tags)

```html
<script src="./path/to/cassette-loader.umd.js"></script>
<script>
  // The library is available as the global variable CassetteLoader
  const { CassetteManager, loadCassette } = CassetteLoader;
  
  // Initialize the CassetteManager
  const manager = new CassetteManager();
  
  // Use the library as needed
  // ...
</script>
```

### 3. With a Module Bundler (webpack, rollup, etc.)

```javascript
// Install from npm
// npm install --save cassette-loader

// In your application code
import { CassetteManager, loadCassette } from 'cassette-loader/browser';

// Use the library as needed
// ...
```

### Example: Loading a cassette from a File Input

```javascript
// Setup a file input for selecting WASM cassettes
const fileInput = document.getElementById('cassette-file');
fileInput.addEventListener('change', async (event) => {
  if (event.target.files.length > 0) {
    const file = event.target.files[0];
    try {
      const cassette = await manager.loadCassetteFromFile(file);
      console.log('Loaded cassette:', cassette);
    } catch (error) {
      console.error('Error loading cassette:', error);
    }
  }
});
```

For a complete example, see the [browser-demo.html](./examples/browser-demo.html) file.

## License

ISC 

## Testing

To test the library with a sample cassette:

```bash
# Build the library
npm run clean && npm run build

# Run the test script
npm test
```

The test script will:
1. Check if WebAssembly is supported in your environment
2. Load a sample cassette from the `../cassettes/` directory
3. Attempt to get metadata and process a sample request
4. Report the results

You can also customize the test by editing the `test.js` file. 