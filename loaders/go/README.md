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
    result, err := c.Send(req)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Println("REQ Result:", result)
    
    // Send a CLOSE message
    closeMsg := `["CLOSE", "sub1"]`
    closeResult, err := c.Send(closeMsg)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Println("CLOSE Result:", closeResult)
    
    // Send a COUNT message (NIP-45)
    countMsg := `["COUNT", "count-sub", {"kinds": [1]}]`
    countResult, err := c.Send(countMsg)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Println("COUNT Result:", countResult)
    
    // Get relay info (NIP-11)
    info, err := c.Info()
    if err != nil {
        log.Println("Info not available:", err)
    } else {
        fmt.Println("Relay Info:", info)
    }
}
```

## Features

- Full WebAssembly support via wasmtime-go
- Unified `Send` method for all NIP-01 messages (v0.5.0+)
- MSGB format support for memory operations
- Event deduplication (automatically reset on new REQ messages)
- Newline-separated message handling
- Thread-safe operations
- Debug logging support
- Automatic synthesis of `Describe()` from `Info()` method

## Requirements

- Go 1.21 or later
- wasmtime-go v23.0.0 or later