# SandwichsFavs Cassette

A WASM-based Nostr relay implementation for the Cassette platform. This project demonstrates how to implement NIP-01 and NIP-119 compliant Nostr relay functionality using WebAssembly.

## Project Overview

The project consists of:

- **sandwichs-favs**: A Rust implementation of a Nostr relay that gets compiled to WebAssembly
- **boombox**: A TypeScript server that loads and interacts with the WebAssembly module
- **nostr-proxy**: A WebSocket proxy for testing Nostr clients
- **cassettes**: Directory containing all WebAssembly cassette modules and their bindings
- **cassette-tools**: Rust library for building standardized WebAssembly cassettes
- **cli**: Command-line tool for creating and managing cassettes

## Directory Structure

- `cassettes/`: Contains all WebAssembly (.wasm) files and their JavaScript bindings
- `boombox/`: WebSocket server that loads and runs cassettes
- `cli/`: Command-line tool for creating cassettes from events or templates
- `cassette-tools/`: Shared Rust library for implementing the standardized interface
- `gui/`: Web interface for testing and managing cassettes
- `test-standardized-interface/`: Example implementation of the standardized interface

## Build Instructions

### Prerequisites

- Rust and Cargo
- [Bun](https://bun.sh/) for running TypeScript code and scripts
- [nak](https://github.com/fiatjaf/nak) (recommended for testing)

### Building the WASM Module

You can build the WebAssembly module using the Makefile:

```bash
make build-wasm
```

This will:
1. Navigate to the `sandwichs-favs` directory and build the Rust project targeting WebAssembly
2. Process the WASM file to generate JavaScript bindings and place them in the `cassettes` directory

If you prefer to do this manually:

1. Navigate to the `sandwichs-favs` directory and build the Rust project targeting WebAssembly:
   ```bash
   cd sandwichs-favs
   cargo build --target wasm32-unknown-unknown
   ```

2. Process the WASM file to generate JavaScript bindings in the cassettes directory:
   ```bash
   cd ..
   bun boombox/scripts/process-wasm.js
   ```

### Building Custom Cassettes

To build a custom cassette and place it in the cassettes directory:

1. Create a new cassette project or use an existing one:
   ```bash
   cd test-standardized-interface
   cargo build --target wasm32-unknown-unknown --release
   ```

2. Copy the WebAssembly file to the cassettes directory:
   ```bash
   cp target/wasm32-unknown-unknown/release/test_standardized_interface.wasm ../cassettes/
   ```

3. Generate JavaScript bindings:
   ```bash
   cd ..
   wasm-bindgen cassettes/test_standardized_interface.wasm --out-dir cassettes --target web
   ```

### Running the Integration Tests

The project includes several Makefile commands to simplify testing:

```bash
# Run all integration tests (starts services if needed)
make test

# Only start the boombox and nostr-proxy services without running tests
make start-services

# Run just the filter tests (assumes services are already running)
make test-filters

# Display all available commands
make help
```

After the servers are running, you can manually test the Nostr relay functionality with various commands:

```bash
# Request 5 notes of kind 1
nak req -l 5 -k 1 localhost:3001

# Request notes with timestamps after a specific time
nak req -s 1741380000 localhost:3001

# Request notes with timestamps before a specific time
nak req -u 1741400000 localhost:3001

# Request notes by a specific ID
nak req -i 380c1dd962349cecbaf65eca3c66574f93ebbf7b1c1e5d7ed5bfc253c94c5211 localhost:3001

# Request notes with NIP-119 AND tag filtering (notes that have both 'value1' AND 'value2' t-tags)
make test-filters
```

### Logs and Debugging

Logs are stored in the `logs` directory:

```bash
# View boombox logs
tail -f ./logs/boombox.log

# View nostr-proxy logs
tail -f ./logs/nostr-proxy.log
```

To stop all servers and clean up logs:

```bash
make clean
```

## Nostr Protocol Implementation

The WebAssembly module implements the following NIP specifications:

### NIP-01: Basic Protocol

1. **REQ**: For requesting notes with various filters:
   - `kinds`: Filter by event kind
   - `authors`: Filter by author public key
   - `ids`: Filter by specific event IDs
   - `since`: Filter by timestamps after a specific time
   - `until`: Filter by timestamps before a specific time
   - `limit`: Limit the number of results
   - `#e`, `#p`, etc.: Filter by tags

2. **EVENT**: For sending events from the relay to clients

3. **CLOSE**: For closing subscriptions

### NIP-119: AND Tag Queries

This implementation supports AND conditions for tag filtering using the '&' prefix:

- `&t`: Match events with ALL specified 't' tag values
- `&e`: Match events with ALL specified 'e' tag values
- etc.

This allows for more precise filtering by requiring that all specified values for a given tag type must be present, rather than just any one of them.

Example usage in a test script:
```javascript
// Request events that have both 'value1' AND 'value2' as 't' tags
const reqMsg = ['REQ', '1:', {'&t': ['value1', 'value2']}];
```

## Project Structure

- `sandwichs-favs/`: The Rust WebAssembly implementation
  - `src/lib.rs`: The core Rust implementation of NIP-01 functionality
  - `notes.json`: Sample note data used by the relay

- `boombox/`: The TypeScript server that loads the WebAssembly module
  - `index.ts`: The main server implementation
  - `wasm/`: Directory for processed WebAssembly files
  - `scripts/process-wasm.js`: Script for processing the WASM file

- `nostr-proxy/`: A WebSocket proxy for testing
  - `index.ts`: The proxy server implementation

- `tests/`: Integration tests
  - `integration-test.sh`: Main integration test script
  - `test-nip119.js`: Tests NIP-119 AND tag filtering

## Advanced Customization

### Adding New Methods to the WebAssembly Module

1. Add the method to the Rust code in `sandwichs-favs/src/lib.rs`
2. Rebuild the WebAssembly module with `make build-wasm`
3. Update the bindings (if needed) with:
   ```bash
   echo "method1,method2" | bun scripts/update-wasm-bindings.js
   ```

For more details on the WebAssembly binding structure and common issues, see the `WASM-QUICKSTART.md` file.

# Cassette Project

This project provides a framework for creating and testing Nostr relay cassettes - WebAssembly modules that can simulate relay behavior for testing client applications.

## Directory Structure

- **cli/** - Command-line interface for generating and managing cassettes
- **cassette-tools/** - Core library for cassette development
- **boombox/** - Node.js/Bun runtime for running cassettes in a server environment
- **gui/** - Web interface for testing cassettes directly in the browser

## Standardized WebAssembly Interface

The Cassette Project uses a standardized WebAssembly interface, which allows cassettes to be distributed as standalone `.wasm` files without requiring JavaScript bindings.

### Key Benefits

- **Simplified Distribution**: Only the `.wasm` file is required for distribution
- **No Binding Files**: No need to generate or distribute JavaScript binding files for each cassette
- **Consistent Interface**: All cassettes implement the same interface, making them easily interchangeable
- **More Efficient**: Smaller download size, simpler loading process

### Standard Interface Functions

All cassettes implement these standard functions, which are accessed directly by the framework:

- **describe()** - Returns JSON metadata about the cassette
- **getSchema()** - Returns JSON schema for the cassette
- **req(requestJson)** - Processes requests and returns responses
- **close(closeJson)** - Handles subscription closures
- **allocString(len)** and **deallocString(ptr, len)** - Optional memory management helpers

### Loading WebAssembly Cassettes

Both the Boombox server and the GUI interface can load WebAssembly cassettes directly:

```javascript
// Create a standardized import object
const importObject = {
  env: { memory: new WebAssembly.Memory({ initial: 16 }) },
  // Minimal required helpers
  __wbindgen_placeholder__: {
    __wbindgen_string_new: (ptr, len) => { /* String helper */ },
    __wbindgen_throw: (ptr, len) => { /* Error helper */ }
  }
};

// Instantiate the WebAssembly module
const result = await WebAssembly.instantiate(wasmArrayBuffer, importObject);
const exports = result.instance.exports;

// Access the standardized interface
const metadata = exports.describe();
const response = exports.req(requestJson);
```

### Building Cassettes

When building cassettes with the CLI, you can use the `--no-bindings` flag to skip JavaScript binding generation:

```bash
cassette dub --no-bindings -n "My Cassette" -d "Description" -a "Author" input.json
```

This will generate just the `.wasm` file, which can be directly loaded by the Boombox server or GUI interface.

For more details, see the [WASM-QUICKSTART.md](WASM-QUICKSTART.md) guide.

## Usage

- To create a new cassette, use the CLI: `cassette dub <events.json>`
- To test cassettes in the browser, use the GUI: http://localhost:8002/
- To run cassettes in a Node.js environment, use Boombox

## Development

See each subdirectory for specific development instructions.

### Running the Boombox Server

To run the Boombox server:

```bash
cd boombox
bun index.ts
```

The server will start on port 3001 and load all WebAssembly modules from the `cassettes` directory. The server will automatically detect and load any `.wasm` files in this directory that implement the standardized interface.

You should see output like:
```
Loading cassettes from: /path/to/cassettes
Loading cassette from WASM file: custom_cassette.wasm
Loading cassette from WASM file: test_standardized_interface.wasm
Successfully loaded cassette: custom_cassette
Successfully loaded cassette: test_standardized_interface
Loaded 2 cassettes
Boombox server running on port 3001
``` 