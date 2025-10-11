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
use cassette_loader::{Cassette, SendResult};
use anyhow::Result;

fn main() -> Result<()> {
    // Load a cassette
    let mut cassette = Cassette::load("path/to/cassette.cassette", true)?;
    
    // Get cassette description
    let desc = cassette.describe()?;
    println!("Description: {}", desc);
    
    // Get relay info (NIP-11)
    let info = cassette.info()?;
    println!("Relay info: {}", info);
    
    // Send a REQ message - automatically collects all events until EOSE
    let req = r#"["REQ", "sub1", {"limit": 10}]"#;
    match cassette.send(req)? {
        SendResult::Multiple(events) => {
            println!("Received {} events", events.len());
            for event in events {
                println!("Event: {}", event);
            }
        }
        SendResult::Single(response) => {
            println!("Single response: {}", response);
        }
    }
    
    // Send a CLOSE message - returns single response
    let close = r#"["CLOSE", "sub1"]"#;
    match cassette.send(close)? {
        SendResult::Single(response) => {
            println!("Close result: {}", response);
        }
        _ => unreachable!("CLOSE should return single response"),
    }
    
    // Send a COUNT message (NIP-45) - returns single response
    let count = r#"["COUNT", "count-sub", {"kinds": [1]}]"#;
    match cassette.send(count)? {
        SendResult::Single(response) => {
            println!("Count result: {}", response);
        }
        _ => unreachable!("COUNT should return single response"),
    }
    
    Ok(())
}
```

## Features

- Full WebAssembly support via wasmtime
- Unified `send` method for all NIP-01 messages
- **Automatic looping for REQ messages** - `send` returns `SendResult::Multiple` with all events until EOSE
- MSGB format support for memory operations
- Event deduplication (automatically reset on new REQ messages)
- Newline-separated message handling
- Thread-safe event tracking
- Debug logging support
- Automatic synthesis of `describe()` from `info()` method

## Important: Loop Behavior

Unlike WebSocket connections, cassettes return one message per `send` call. The `send` method now automatically detects REQ messages and loops until EOSE, returning all events as `SendResult::Multiple(Vec<String>)`. For other message types, it returns `SendResult::Single(String)`.

## Requirements

- Rust 1.70 or later
- wasmtime 23.0 or later