# Cassette Loader for Python

A Python implementation of the cassette loader for loading and interacting with Nostr WASM cassettes.

## Installation

```bash
pip install -r requirements.txt
```

Or install the package:

```bash
pip install .
```

## Usage

### Basic Loading

```python
from cassette_loader import load_cassette

# Load WASM file
with open('path/to/cassette.wasm', 'rb') as f:
    wasm_bytes = f.read()

# Load cassette
result = load_cassette(wasm_bytes, name='my-cassette', debug=True)

if result['success']:
    cassette = result['cassette']
    print(f"Loaded: {cassette.info.name} v{cassette.info.version}")
else:
    print(f"Failed: {result['error']}")
```

### Making Requests

```python
# Send a REQ message
response = cassette.req('["REQ", "sub1", {"kinds": [1], "limit": 10}]')
print(response)

# Parse response
import json
data = json.loads(response)
if data[0] == "EVENT":
    print(f"Got event: {data[2]['id']}")
elif data[0] == "EOSE":
    print(f"End of stored events for subscription: {data[1]}")
elif data[0] == "NOTICE":
    print(f"Notice: {data[1]}")
```

### Memory Management

```python
# Check memory statistics
stats = cassette.get_memory_stats()
print(f"Allocated pointers: {stats.allocation_count}")
print(f"Memory pages: {stats.total_pages}")
print(f"Status: {stats.usage_estimate}")

# Clean up when done
dispose_result = cassette.dispose()
print(f"Cleaned up {dispose_result['allocationsCleanedUp']} allocations")
```

## Command Line Usage

```bash
# Test a cassette
python cassette_loader.py path/to/cassette.wasm
```

## Features

- **MSGB Format Support**: Handles the standardized memory format for string passing
- **Memory Tracking**: Tracks allocations to detect memory leaks
- **Debug Mode**: Detailed logging for troubleshooting
- **Event Deduplication**: Filters duplicate events across REQ calls
- **Type Safety**: Uses Python type hints and dataclasses

## API Reference

### `load_cassette(wasm_bytes, name, debug)`

Loads a cassette from WASM bytes.

- `wasm_bytes`: The WASM module bytes
- `name`: Name for the cassette (optional)
- `debug`: Enable debug logging (optional)

Returns a dict with either `{'success': True, 'cassette': Cassette}` or `{'success': False, 'error': str}`

### `Cassette` class

#### Methods:
- `describe()`: Get cassette metadata as JSON string
- `req(request)`: Process a REQ message, returns response as JSON string
- `close(close_msg)`: Process a CLOSE message
- `get_memory_stats()`: Get current memory statistics
- `dispose()`: Clean up resources

#### Properties:
- `info`: CassetteInfo object with metadata
- `id`: Unique instance ID
- `name`: Cassette name

## Differences from JavaScript Implementation

The Python implementation closely mirrors the JavaScript version with these adaptations:

1. Uses `wasmtime-py` instead of native WebAssembly APIs
2. Memory access through `memory.data_ptr()` instead of typed arrays
3. Python sets for tracking allocations instead of JavaScript Sets
4. Type hints and dataclasses for better type safety
5. Pythonic error handling with try/except blocks

## Memory Format

The loader supports the MSGB (Message Buffer) format:
- 4 bytes: 'MSGB' signature
- 4 bytes: Length (little-endian)
- N bytes: UTF-8 string data

This format ensures consistent memory handling across different language implementations.