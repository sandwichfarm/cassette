# Cassette Rust Loader

A Rust implementation of the Cassette loader for loading and executing Nostr event cassettes.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cassette-loader = "0.1.0"
```

## Usage

```rust
use cassette_loader::Cassette;
use anyhow::Result;

fn main() -> Result<()> {
    // Load a cassette
    let mut cassette = Cassette::load("path/to/cassette.wasm", true)?;
    
    // Get cassette description
    let desc = cassette.describe()?;
    println!("Description: {}", desc);
    
    // Get relay info (NIP-11)
    let info = cassette.info()?;
    println!("Relay info: {}", info);
    
    // Send a REQ message
    let req = r#"["REQ", "sub1", {"limit": 10}]"#;
    let result = cassette.send(req)?;
    println!("Result: {}", result);
    
    // Send a CLOSE message
    let close = r#"["CLOSE", "sub1"]"#;
    let close_result = cassette.send(close)?;
    println!("Close result: {}", close_result);
    
    // Send a COUNT message (NIP-45)
    let count = r#"["COUNT", "count-sub", {"kinds": [1]}]"#;
    let count_result = cassette.send(count)?;
    println!("Count result: {}", count_result);
    
    Ok(())
}
```

## Features

- Full WebAssembly support via wasmtime
- Unified `send` method for all NIP-01 messages (v0.5.0+)
- MSGB format support for memory operations
- Event deduplication (automatically reset on new REQ messages)
- Newline-separated message handling
- Thread-safe event tracking
- Debug logging support
- Automatic synthesis of `describe()` from `info()` method

## Requirements

- Rust 1.70 or later
- wasmtime 23.0 or later