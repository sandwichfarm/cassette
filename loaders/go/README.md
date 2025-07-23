# Cassette Go Loader

A Go implementation of the Cassette loader for loading and executing Nostr event cassettes.

## Installation

```bash
go get github.com/cassette/loaders/go
```

## Usage

```go
package main

import (
    "fmt"
    "log"
    
    cassette "github.com/cassette/loaders/go"
)

func main() {
    // Load a cassette
    c, err := cassette.LoadCassette("path/to/cassette.wasm", true)
    if err != nil {
        log.Fatal(err)
    }
    
    // Get cassette description
    desc, err := c.Describe()
    if err != nil {
        log.Fatal(err)
    }
    fmt.Println("Description:", desc)
    
    // Send a REQ message
    req := `["REQ", "sub1", {"limit": 10}]`
    
    // Loop to get all events
    for {
        result, err := c.Req(req)
        if err != nil {
            log.Fatal(err)
        }
        
        if result == "" {
            break
        }
        
        fmt.Println("Result:", result)
        
        // Check for EOSE
        if strings.Contains(result, `"EOSE"`) {
            break
        }
    }
}
```

## Features

- Full WebAssembly support via wasmtime-go
- MSGB format support for memory operations
- Event deduplication
- Newline-separated message handling
- Thread-safe operations
- Debug logging support

## Requirements

- Go 1.21 or later
- wasmtime-go v23.0.0 or later