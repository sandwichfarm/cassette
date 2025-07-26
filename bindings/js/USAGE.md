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

## WebAssembly Interface Requirements

If you're developing your own cassettes (rather than using the CLI to generate them), make sure they implement the following interface functions correctly:

```rust
#[no_mangle]
pub extern "C" fn describe() -> *mut u8;

#[no_mangle]
pub extern "C" fn get_schema() -> *mut u8;

#[no_mangle]
pub extern "C" fn req(request_ptr: *const u8, request_len: usize) -> *mut u8;

#[no_mangle]
pub extern "C" fn close(close_ptr: *const u8, close_len: usize) -> *mut u8;
```

**Important**: The `req` and `close` functions must accept both a pointer to the string and its length. Failing to implement this correctly will result in empty or incorrect strings being passed to your cassette.

It's recommended to use the `cassette-tools` crate for proper string handling between WebAssembly and JavaScript:

```rust
use cassette_tools::{string_to_ptr, ptr_to_string};

#[no_mangle]
pub extern "C" fn req(request_ptr: *const u8, request_len: usize) -> *mut u8 {
    // Convert WebAssembly pointer to Rust string
    let request_str = ptr_to_string(request_ptr, request_len);
    
    // Process request...
    
    // Convert response back to WebAssembly pointer
    string_to_ptr(response)
}
```

## Memory Management

Proper memory management is crucial when working with WebAssembly modules, especially when string data is passed between JavaScript and WebAssembly. The cassette-loader includes built-in memory tracking and leak detection functionality.

### Detecting Memory Leaks

Each cassette object includes a `getMemoryStats()` method that provides information about the current memory state:

```javascript
// Get memory statistics
const memStats = cassette.getMemoryStats();
console.log(`Allocation count: ${memStats.allocationCount}`);
console.log(`Memory usage: ${(memStats.memory.totalBytes / 1024 / 1024).toFixed(2)} MB`);

// Check for leaks
if (memStats.allocationCount > 0) {
  console.warn(`Potential memory leak detected: ${memStats.allocationCount} allocations`);
  console.log(`Allocated pointers: ${memStats.allocatedPointers.join(', ')}`);
}
```

### Automatic Leak Detection

When loading a cassette with debug mode enabled, automatic leak detection is activated:

```javascript
// Load a cassette with debug and leak detection enabled
const result = await loadCassette('path/to/cassette.wasm', undefined, { 
  debug: true 
});

// After 10 seconds, the loader will automatically check for leaks and log warnings if found
```

### Cleaning Up Resources

To properly clean up resources and prevent memory leaks, use the `dispose()` method when you're done with a cassette:

```javascript
// Clean up resources
const disposeResult = cassette.dispose();
console.log(`Cleaned up ${disposeResult.allocationsCleanedUp} allocations`);

// Verify cleanup was successful
const finalStats = cassette.getMemoryStats();
if (finalStats.allocationCount === 0) {
  console.log('Memory cleanup successful');
}
```

### Best Practices for Memory Management

1. **Always dispose cassettes when done**: Call `cassette.dispose()` when you're finished with a cassette.
2. **Enable debug mode during development**: Set `debug: true` when loading cassettes to enable leak detection.
3. **Monitor memory usage**: Periodically check `getMemoryStats()` for long-running applications.
4. **Use the WebAssembly memory interface consistently**: Ensure that memory allocation and deallocation functions match in WebAssembly modules.

### Memory Usage Pattern

The proper usage pattern to avoid memory leaks is:

```javascript
// Load the cassette
const result = await loadCassette('path/to/cassette.wasm');
const cassette = result.cassette;

try {
  // Use the cassette...
  const response = cassette.methods.req('["REQ", "sub1", {"kinds":[1]}]');
  // Process response...
} catch (error) {
  console.error('Error:', error);
} finally {
  // Always clean up resources
  cassette.dispose();
}
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