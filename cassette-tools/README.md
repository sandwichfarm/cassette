# Cassette Tools

A Rust library for creating and working with Cassettes for the Boombox relay server.

## Overview

Cassette Tools provides functionality for:

1. Creating JSON-based cassettes from event files (via the `cassette dub` command)
2. Implementing custom cassette functionality using Rust and WebAssembly (via the `RelayHandler` trait)

## JSON-Based Cassettes (via CLI)

The simplest way to create a cassette is using the `cassette dub` command, which takes a JSON file of events and transforms them into a cassette:

```bash
cassette dub -n "My Cassette" -d "Description of my cassette" -a "Author Name" -o ./output events.json
```

or pipe events directly:

```bash
cat events.json | cassette dub -n "My Cassette" -d "Description" -a "Author" -o ./output
```

## Custom Cassettes (via Rust Implementation)

For more dynamic behavior, you can implement a custom cassette by using this library as a dependency:

1. Add `cassette-tools` to your `Cargo.toml`:

```toml
[dependencies]
cassette-tools = "0.1.0"
wasm-bindgen = "0.2"
serde_json = "1.0"
```

2. Implement the `Cassette` and `RelayHandler` traits:

```rust
use cassette_tools::{Cassette, CassetteSchema, RelayHandler, RelayResult};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct MyCassette {
    // State variables
}

// Implement the Cassette trait for schema and description
impl Cassette for MyCassette {
    fn describe() -> String {
        "My custom cassette description".to_string()
    }
    
    fn get_schema() -> CassetteSchema {
        // Return schema describing your cassette
    }
}

// Implement RelayHandler to process relay messages
impl RelayHandler for MyCassette {
    fn handle_req(&self, req_json: &str) -> RelayResult {
        // Process REQ messages and return dynamic responses
        Ok("...".to_string())
    }
    
    // Optional: override handle_close
    fn handle_close(&self, close_json: &str) -> RelayResult {
        // Handle CLOSE messages
        Ok("...".to_string())
    }
}

// Expose to JavaScript with wasm-bindgen
#[wasm_bindgen]
impl MyCassette {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Initialize your cassette
    }
    
    #[wasm_bindgen]
    pub fn describe() -> String {
        <Self as Cassette>::describe()
    }
    
    #[wasm_bindgen]
    pub fn get_schema() -> String {
        <Self as Cassette>::get_schema_json()
    }
    
    // REQ handler (exposed to JS)
    #[wasm_bindgen]
    pub fn req(request_json: &str) -> String {
        let instance = Self::new();
        match instance.handle_req(request_json) {
            Ok(response) => response,
            Err(err) => format!("{{\"notice\": [\"NOTICE\", \"{}\"]}}", err)
        }
    }
    
    // CLOSE handler (exposed to JS)
    #[wasm_bindgen]
    pub fn close(close_json: &str) -> String {
        let instance = Self::new();
        match instance.handle_close(close_json) {
            Ok(response) => response,
            Err(err) => format!("{{\"notice\": [\"NOTICE\", \"{}\"]}}", err)
        }
    }
}
```

3. Build your cassette:

```bash
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/my_cassette.wasm --out-dir ./pkg --no-modules
```

## Example

For a complete example of a custom cassette implementation, see the [custom-cassette example](../cli/examples/custom-cassette/) in the CLI directory.

## Documentation

For more details on the available traits and types:

- `Cassette` - Base trait for all cassettes
- `CassetteSchema` - Schema definition for cassettes
- `RelayHandler` - Trait for handling relay messages
- `RelayResult` - Result type for relay operations

## License

This project is licensed under the terms of the LICENSE file in the repository. 