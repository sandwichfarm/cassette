# Cassette Test CLI

A command-line tool for testing and debugging cassette WASM modules using the cassette-loader.

## Installation

```bash
npm install
```

## Usage

### Basic Loading Test

Test loading a cassette and checking its metadata:

```bash
node cli.js load path/to/cassette.wasm
```

With debug output:
```bash
node cli.js load path/to/cassette.wasm --debug
```

With memory statistics:
```bash
node cli.js load path/to/cassette.wasm --memory
```

### Request Testing

Send a REQ message to a cassette:

```bash
# With a custom request
node cli.js req path/to/cassette.wasm '["REQ", "sub1", {"kinds": [1], "limit": 5}]'

# Uses default request if JSON is invalid
node cli.js req path/to/cassette.wasm test

# With memory leak detection
node cli.js req path/to/cassette.wasm '["REQ", "sub1", {}]' --memory
```

### Stress Testing

Run multiple requests to test for memory leaks:

```bash
# Run 100 iterations (default)
node cli.js stress path/to/cassette.wasm

# Run 1000 iterations
node cli.js stress path/to/cassette.wasm --iterations 1000

# With debug output
node cli.js stress path/to/cassette.wasm --debug
```

## Features

- **Load Testing**: Verify cassette loads correctly and reports metadata
- **Request Testing**: Send NIP-01 REQ messages and inspect responses
- **Memory Leak Detection**: Track memory allocations and detect leaks
- **Stress Testing**: Run multiple iterations to find memory issues
- **Debug Mode**: Detailed logging for troubleshooting
- **Pretty Output**: Color-coded terminal output for easy reading

## Example Output

```
🔧 Loading cassette: test_cassette.wasm
✅ Cassette loaded successfully
   ID: test_cassette_1234567890
   Name: Test Cassette
   Description: A test cassette
   Version: 1.0.0

📤 Sending request: ["REQ","test1",{"kinds":[1],"limit":10}]

📥 Response:
   Received 11 messages:
   [1] EVENT: 8f1c568d... (kind 1)
   [2] EVENT: 665c503e... (kind 1)
   [3] EVENT: 000001ce... (kind 1)
   [4] EVENT: 380c1dd9... (kind 1)
   [5] EVENT: 2cfe17ea... (kind 1)
   [6] EVENT: 07aae40d... (kind 1)
   [7] EOSE: test1

💾 Memory Statistics After Request:
   Allocated pointers: 0
   Memory pages: 16
   Memory bytes: 1048576
   Status: No leaks detected

🧹 Cleaning up resources...
   Allocations cleaned up: 0

✅ Test completed successfully
```