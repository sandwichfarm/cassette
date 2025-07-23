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
    
    // Send a REQ message
    let req = r#"["REQ", "sub1", {"limit": 10}]"#;
    
    // Loop to get all events
    loop {
        let result = cassette.req(req)?;
        
        if result.is_empty() {
            break;
        }
        
        println!("Result: {}", result);
        
        // Check for EOSE
        if result.contains(r#""EOSE""#) {
            break;
        }
    }
    
    Ok(())
}
```

## Features

- Full WebAssembly support via wasmtime
- MSGB format support for memory operations
- Event deduplication
- Newline-separated message handling
- Thread-safe event tracking
- Debug logging support

## Requirements

- Rust 1.70 or later
- wasmtime 23.0 or later