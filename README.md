# SandwichsFavs Cassette

A WASM-based Nostr relay implementation for the Cassette platform. This project demonstrates how to implement NIP-01 compliant Nostr relay functionality using WebAssembly.

## Project Overview

The project consists of:

- **sandwichs-favs**: A Rust implementation of a Nostr relay that gets compiled to WebAssembly
- **boombox**: A TypeScript server that loads and interacts with the WebAssembly module
- **nostr-proxy**: A WebSocket proxy for testing Nostr clients

## Build Instructions

### Prerequisites

- Rust and Cargo
- [Bun](https://bun.sh/) for running TypeScript code and scripts
- [nak](https://github.com/fiatjaf/nak) (recommended for testing)

### Building the WASM Module

1. Navigate to the `sandwichs-favs` directory and build the Rust project targeting WebAssembly:
   ```bash
   cd sandwichs-favs
   cargo build --target wasm32-unknown-unknown
   ```

2. Process the WASM file to generate JavaScript bindings:
   ```bash
   cd ../boombox
   bun scripts/process-wasm.js
   ```

That's it! The WebAssembly module is now built and ready to be used by the boombox server.

### Running the Integration Test

The project includes an integration test script that starts both the boombox server and nostr-proxy server:

```bash
./integration-test.sh
```

After the servers are running, you can test the Nostr relay functionality with various commands:

```bash
# Request 5 notes of kind 1
nak req -l 5 -k 1 localhost:3001

# Request notes with timestamps after a specific time
nak req -s 1741380000 localhost:3001

# Request notes with timestamps before a specific time
nak req -u 1741300000 localhost:3001

# Request notes by a specific ID
nak req '{"ids":["380c1dd962349cecbaf65eca3c66574f93ebbf7b1c1e5d7ed5bfc253c94c5211"]}' localhost:3001
```

### Logs and Debugging

Logs are stored in the `logs` directory:

```bash
# View boombox logs
tail -f ./logs/boombox.log

# View nostr-proxy logs
tail -f ./logs/nostr-proxy.log
```

To stop all servers:

```bash
pkill -f 'bun run'
```

## Nostr Protocol Implementation

The WebAssembly module implements the following NIP-01 message types:

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

## Advanced Customization

### Adding New Methods to the WebAssembly Module

1. Add the method to the Rust code in `sandwichs-favs/src/lib.rs`
2. Rebuild the WebAssembly module
3. Update the bindings (if needed) with:
   ```bash
   echo "method1,method2" | bun scripts/update-wasm-bindings.js
   ```

For more details on the WebAssembly binding structure and common issues, see the `WASM-QUICKSTART.md` file. 