# Zig WASM Incrementer

A simple WebAssembly module written in Zig that provides basic increment and addition functions.

## Features

- `increment(value: i32) -> i32`: Increments a 32-bit integer by 1
- `add(a: i32, b: i32) -> i32`: Adds two 32-bit integers

## Prerequisites

- [Zig](https://ziglang.org/download/) (latest version recommended)

### Installing Zig on macOS ARM (Apple Silicon)

Using Homebrew:
```bash
brew install zig
```

Or using the official binary:
```bash
# Download latest version
curl -L https://ziglang.org/download/0.11.0/zig-macos-aarch64-0.11.0.tar.xz -o zig.tar.xz

# Extract
tar xf zig.tar.xz

# Move to a suitable location
sudo mv zig-macos-aarch64-0.11.0 /usr/local/zig

# Add to PATH (add this to your ~/.zshrc or ~/.bash_profile)
export PATH=$PATH:/usr/local/zig
```

Verify installation:
```bash
zig version
```

## Building

To build the WebAssembly module:

```bash
zig build -Doptimize=ReleaseSmall
```

The compiled WebAssembly file will be available in `zig-out/lib/incrementer.wasm`

## Usage in JavaScript

```javascript
// Load the WASM module
const wasmModule = await WebAssembly.instantiateStreaming(
    fetch('incrementer.wasm'),
    {}
);

const { increment, add } = wasmModule.instance.exports;

// Use the increment function
console.log(increment(41));  // Outputs: 42

// Use the add function
console.log(add(20, 22));   // Outputs: 42
```

## Project Structure

```
incrementer/
├── src/
│   └── main.zig    # Source code with WASM functions
├── build.zig       # Build configuration
└── README.md       # This file
```

## Build Configuration

The project is configured to:
- Target wasm32-unknown-unknown
- Use 64KB of initial memory
- Strip debug information for smaller binary size
- Enable memory import from JavaScript

## License

MIT License

## Contributing

Feel free to open issues and pull requests! 