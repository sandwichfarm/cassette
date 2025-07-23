# Cassette Tests

Comprehensive test suite for the Cassette platform, including unit tests, integration tests, and end-to-end tests.

## Overview

This directory contains various test scripts and utilities to verify:
- WebAssembly cassette functionality
- Memory management and leak detection
- NIP-01 protocol compliance
- WebSocket server integration
- CLI tool functionality

## Test Categories

### JavaScript Tests

#### Direct Cassette Tests
Tests that load and interact with cassettes directly:

- **`test-cassette-memory.js`** - Memory management and leak detection
- **`test-cli-cassette.js`** - Tests CLI-generated cassettes
- **`test-debug-cassette.js`** - Debug mode testing for all cassettes
- **`test-generated-cassette.js`** - Tests specific generated cassettes
- **`test-direct-cassette.js`** - Direct cassette loading without server

#### WebSocket Integration Tests
Tests that connect to the Boombox server:

- **`test-simple.js`** - Basic WebSocket connection test
- **`test-specific-cassette.js`** - Tests specific cassette via WebSocket
- **`test-close-handling.js`** - Tests CLOSE message handling
- **`test-single-close.js`** - Single subscription close testing
- **`test-deduplication.js`** - Event deduplication testing

#### Protocol Tests
- **`test-nip119.js`** - NIP-119 AND tag filter testing

### Shell Script Tests

- **`integration-test.sh`** - Comprehensive integration testing
- **`e2e-test.sh`** - End-to-end testing with server setup
- **`advanced-e2e-test.sh`** - Advanced scenarios and edge cases
- **`simple-e2e-test.sh`** - Basic end-to-end flow
- **`test-nak-format.sh`** - Tests nak output format compatibility

## Running Tests

### Prerequisites

```bash
# Install dependencies
npm install

# Build cassette-loader
cd ../cassette-loader && npm run build

# Ensure test cassettes exist
cd ../cli && cargo run record ../tests/test-events.json --name test-cassette
```

### Running Individual Tests

```bash
# Direct cassette tests
node test-cassette-memory.js
node test-cli-cassette.js
node test-debug-cassette.js

# WebSocket tests (requires running Boombox server)
cd ../boombox && bun index.ts &
node test-simple.js
node test-close-handling.js

# Shell script tests
./integration-test.sh
./e2e-test.sh
```

### Running All Tests

```bash
# Run integration test suite
./integration-test.sh

# Or run advanced E2E tests
./advanced-e2e-test.sh
```

## Test Data

The `cassettes/` subdirectory contains test cassette files used by various tests.

## Writing New Tests

### Direct Cassette Test Template

```javascript
import { loadCassette } from '../cassette-loader/dist/src/index.js';

async function test() {
  const cassette = await loadCassette('../cassettes/test.wasm');
  
  // Test describe method
  const metadata = cassette.describe();
  console.log('Metadata:', metadata);
  
  // Test req method
  const response = cassette.req(["REQ", "sub1", {}]);
  console.log('Response:', response);
  
  // Test close method
  cassette.close(["CLOSE", "sub1"]);
}

test().catch(console.error);
```

### WebSocket Test Template

```javascript
import WebSocket from 'ws';

const ws = new WebSocket('ws://localhost:3001');

ws.on('open', () => {
  ws.send(JSON.stringify(["REQ", "sub1", { kinds: [1] }]));
});

ws.on('message', (data) => {
  const msg = JSON.parse(data);
  console.log('Received:', msg);
  
  if (msg[0] === 'EOSE') {
    ws.close();
  }
});
```

## Debugging Tips

1. **Memory Issues**: Use `test-cassette-memory.js` with verbose logging
2. **Protocol Issues**: Check server logs in `../logs/boombox.log`
3. **WebSocket Issues**: Use browser DevTools or `wscat` for manual testing
4. **WASM Loading**: Enable debug mode in cassette-loader

## Common Issues

### "Cassette not found"
Ensure cassettes are built and in the correct location:
```bash
cd ../cli
cargo run record test-events.json --name test-cassette --output ../cassettes
```

### "Connection refused"
Start the Boombox server:
```bash
cd ../boombox && bun index.ts
```

### Memory leaks
Check for proper deallocation in test output:
```
✅ Memory properly deallocated for request
✅ Memory properly deallocated for response
```

## CI Integration

These tests can be integrated into CI pipelines. See `.github/workflows/test.yml` for GitHub Actions configuration.