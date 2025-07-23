# Cassette Core

Core Rust library providing the foundational trait and utilities for building standardized WebAssembly cassettes.

## Overview

`cassette-core` defines the standard interface that all cassettes must implement. It provides:
- The `Cassette` trait with required methods
- Memory management utilities for WebAssembly
- Macros to simplify implementing the standard exports
- Schema definition structures

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
cassette-core = { path = "../cassette-core" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Implementing a Cassette

```rust
use cassette_core::{Cassette, CassetteSchema, implement_cassette_exports};
use serde_json::json;

pub struct MyCassette;

impl Cassette for MyCassette {
    fn describe() -> String {
        json!({
            "name": "My Cassette",
            "description": "A custom Nostr event cassette",
            "version": "1.0.0",
            "author": "Your Name"
        }).to_string()
    }
    
    fn get_schema() -> CassetteSchema {
        CassetteSchema {
            title: "My Cassette Schema".to_string(),
            description: "Schema for my cassette".to_string(),
            schema_type: "object".to_string(),
            properties: json!({
                "events": {
                    "type": "array",
                    "items": {
                        "type": "object"
                    }
                }
            }),
            required: vec!["events".to_string()],
            items: None,
        }
    }
}

// Generate standard WebAssembly exports
implement_cassette_exports!(MyCassette);

// Implement your custom request handler
#[no_mangle]
pub extern "C" fn req(ptr: *const u8, length: usize) -> *mut u8 {
    let request = cassette_core::ptr_to_string(ptr, length);
    // Process the request...
    let response = json!(["EVENT", "sub1", { /* event data */ }]);
    cassette_core::string_to_ptr(response.to_string())
}
```

## Exported Functions

The `implement_cassette_exports!` macro generates these WebAssembly exports:

- `describe() -> *mut u8` - Returns cassette metadata
- `get_schema() -> *mut u8` - Returns JSON schema
- `get_description_size() -> usize` - Size of description for chunked reading
- `get_description_chunk(start, length) -> *mut u8` - Read description in chunks
- `get_schema_size() -> usize` - Size of schema for chunked reading
- `get_schema_chunk(start, length) -> *mut u8` - Read schema in chunks
- `alloc_string(length) -> *mut u8` - Allocate memory for strings
- `dealloc_string(ptr, length)` - Deallocate string memory

## Memory Management

The library provides safe memory management utilities for passing strings between WebAssembly and host:

```rust
// Convert Rust string to WebAssembly pointer
let ptr = cassette_core::string_to_ptr("Hello from WASM".to_string());

// Convert WebAssembly pointer to Rust string
let string = cassette_core::ptr_to_string(ptr, length);
```

## Schema Definition

The `CassetteSchema` struct follows JSON Schema format:

```rust
CassetteSchema {
    title: "Event Collection".to_string(),
    description: "A collection of Nostr events".to_string(),
    schema_type: "object".to_string(),
    properties: json!({
        "events": {
            "type": "array",
            "items": {
                "$ref": "#/definitions/NostrEvent"
            }
        }
    }),
    required: vec!["events".to_string()],
    items: None,
}
```

## Relationship to cassette-tools

While `cassette-core` provides the basic trait and memory management, `cassette-tools` extends this with:
- MSGB (Message Buffer) format for efficient memory handling
- NIP-01 protocol helpers
- Event filtering utilities
- Higher-level abstractions for Nostr cassettes

Most cassettes should use `cassette-tools` rather than `cassette-core` directly.