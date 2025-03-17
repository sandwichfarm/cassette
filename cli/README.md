# Cassette CLI

A command-line tool for creating and managing Cassette platform WebAssembly modules.

## Overview

Cassette CLI allows users to create WebAssembly cassettes from Nostr events. The CLI provides subcommands for different operations, with the first being `dub` which takes an events.json file or piped input and generates a WASM module for use with the Boombox server.

## Installation

### Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- `wasm-bindgen-cli` (optional, for generating JavaScript bindings)

To install the prerequisites:

```bash
# Install Rust if you don't have it already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the wasm32-unknown-unknown target
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen-cli (optional)
cargo install wasm-bindgen-cli
```

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/cassette-platform.git
cd cassette-platform/cli

# Build the CLI
cargo build --release

# Install the CLI (optional)
cargo install --path .
```

## Usage

### Creating a Cassette from an Events File

```bash
cassette dub events.json --name my-cassette --description "My custom Nostr cassette" --author "Your Name"
```

### Creating a Cassette from Piped Input

You can pipe events from another source, such as the Nostr Army Knife (nak):

```bash
nak req -l 5 -t t=value1 -t t=value2 localhost:3001 | cassette dub --name piped-cassette
```

### Command Line Options

```
USAGE:
    cassette dub [OPTIONS] [INPUT_FILE]

ARGS:
    <INPUT_FILE>  Path to input events.json file (if not provided, reads from stdin)

OPTIONS:
    -n, --name <NAME>               Name for the generated cassette
    -d, --description <DESCRIPTION> Description for the generated cassette
    -a, --author <AUTHOR>           Author of the cassette
    -o, --output <OUTPUT>           Output directory for the generated WASM module
    --generate                      Whether to actually generate the WASM module (default: true)
    -h, --help                      Print help information
```

## Generated Files

When you run the `dub` command, it generates the following files:

- `<NAME>.wasm` - The main WebAssembly module
- `<NAME>.js` - JavaScript bindings for the module
- `<NAME>.d.ts` - TypeScript type definitions
- `<NAME>_bg.wasm` - Background WebAssembly module
- `<NAME>_bg.wasm.d.ts` - TypeScript type definitions for the background module

## Using the Generated Cassette

The generated WebAssembly module can be loaded into the Boombox server. To use the cassette:

1. Copy the generated `.wasm` and JavaScript files to the `boombox/wasm` directory
2. Update the Boombox server configuration to load your custom cassette
3. Start the Boombox server

## Development

### Project Structure

- `src/main.rs` - CLI implementation
- `src/templates/` - Templates for generating Rust code
- `src/templates/cassette_template.rs` - Template for cassette Rust code
- `src/templates/Cargo.toml` - Template for cassette project configuration

### Adding New Subcommands

To add new subcommands to the CLI, modify the `Commands` enum in `src/main.rs` and add handlers for the new commands.

---

This project is part of the Cassette platform, a framework for building and deploying Nostr relays with WebAssembly modules. 