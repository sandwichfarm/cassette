# Zig WASM Incrementer

A simple WebAssembly module written in Zig that provides basic increment and addition functions.

## Features

- `increment(value: i32) -> i32`: Increments a 32-bit integer by 1
- `add(a: i32, b: i32) -> i32`: Adds two 32-bit integers

## Prerequisites

- [Zig](https://ziglang.org/download/) (latest version recommended)
- Node.js (for testing with JavaScript)
- [wasmtime](https://wasmtime.dev/) (optional, for CLI testing)
- [wasmer](https://wasmer.io/) (optional, for CLI testing)

## Building

Build the WebAssembly module:

```bash
zig build
```

The WASM file will be output to `zig-out/bin/incrementer.wasm`.

## Testing

### Using Node.js (ESM)

Create a test file `test.mjs`:

```javascript
import { readFile } from 'node:fs/promises';

const wasmBuffer = await readFile('zig-out/bin/incrementer.wasm');
const wasmModule = await WebAssembly.instantiate(wasmBuffer);
const instance = wasmModule.instance;

console.log('increment(41) =', instance.exports.increment(41));
console.log('add(20, 22) =', instance.exports.add(20, 22));
```

Run the test:
```bash
node test.mjs
```

### Using wasmtime CLI

Test individual functions:
```bash
# Test increment
wasmtime zig-out/bin/incrementer.wasm --invoke increment 41 --show-return

# Test add
wasmtime zig-out/bin/incrementer.wasm --invoke add 20 22 --show-return
```

### Using wasmer CLI

Test individual functions:
```bash
# Test increment
wasmer run zig-out/bin/incrementer.wasm --invoke increment 41

# Test add
wasmer run zig-out/bin/incrementer.wasm --invoke add 20 22
```

## Project Structure

```
incrementer/
├── src/
│   ├── main.zig        # Source code with WASM functions
│   └── auto_describe.zig # Automatic schema generation
├── build.zig           # Build configuration
└── README.md           # This file
```

## Build Configuration

The project is configured to:
- Target wasm32-unknown-unknown
- Use 64KB of initial memory
- Strip debug information for smaller binary size
- Enable memory import from JavaScript
- Automatically generate function schemas and describe function

## License

MIT License

## Contributing

Feel free to open issues and pull requests! 