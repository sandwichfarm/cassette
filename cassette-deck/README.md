# Boombox

A TypeScript server that loads and interacts with WebAssembly modules for the Cassette platform's Nostr relay implementation.

## Overview

Boombox is the core server component of the Cassette platform. It:

1. Loads WebAssembly modules compiled from the sandwichs-favs Rust crate
2. Provides a WebSocket server for clients to connect directly or via nostr-proxy
3. Implements the Nostr relay protocol by delegating functionality to the WebAssembly module
4. Handles request validation, filtering, and response delivery

## Installation

To install dependencies:

```bash
bun install
```

## Running the Server

To start the Boombox server:

```bash
bun run index.ts
```

By default, this will:
- Start a WebSocket server on port 3001
- Load the WebAssembly module from the `wasm` directory
- Log activity to stdout

### Environment Variables

- `PORT`: The port on which the server listens (default: 3001)
- `DEBUG`: Set to `true` for more verbose logging

## Testing

### Running Unit Tests

```bash
bun test
```

This runs the test suite covering:
- WebAssembly module loading and integration
- Schema validation
- Request handling logic

### Integration Testing

For full integration testing, it's recommended to use the project's root Makefile:

```bash
# From the project root
make test
```

Or to test just the Boombox server with manual requests:

```bash
# Start the server
bun run index.ts

# In another terminal, use nak or another Nostr client to test
nak req -l 5 -k 1 localhost:3001
```

## WASM Integration

### Building WASM

You can build the WebAssembly module using the project's root Makefile:

```bash
# From the project root
make build-wasm
```

Or directly using the bun script:

```bash
bun run build:wasm
```

This script will:
1. Build the sandwichs-favs crate with the wasm32-unknown-unknown target
2. Copy the WASM file to the boombox/wasm directory
3. Generate JavaScript bindings

### Updating WASM Bindings Manually

If you need to manually update the bindings after adding new exports to the Rust code:

```bash
bun run update:bindings
```

When prompted, enter the names of the methods to add, separated by commas. For example:

```
get_client_req_schema,get_relay_event_schema,get_relay_notice_schema
```

## Development

### Project Structure

- `index.ts`: Main server implementation and WebSocket handling
- `wasm/`: Directory containing the WebAssembly module and bindings
- `scripts/`: Utility scripts for WASM processing
- `schema-validator.ts`: JSON schema validation for Nostr messages
- `*.test.ts`: Test files for various components

### Debugging

To enable verbose logging, set the DEBUG environment variable:

```bash
DEBUG=true bun run index.ts
```

For examining the contents of the WebAssembly module, you can use the test script:

```bash
bun run wasm.test.ts
```

---

This project was created using `bun init` in bun v1.2.4. [Bun](https://bun.sh) is a fast all-in-one JavaScript runtime.
