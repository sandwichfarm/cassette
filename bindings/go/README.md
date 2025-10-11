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
    c, err := cassette.LoadCassette("path/to/cassette.cassette", true)
    if err != nil {
        log.Fatal(err)
    }
    
    // Get cassette description
    desc, err := c.Describe()
    if err != nil {
        log.Fatal(err)
    }
    fmt.Println("Description:", desc)
    
    // Send a REQ message - automatically collects all events until EOSE
    req := `["REQ", "sub1", {"limit": 10}]`
    result, err := c.Send(req)
    if err != nil {
        log.Fatal(err)
    }
    if result.IsSingle {
        fmt.Println("Single response:", result.Single)
    } else {
        fmt.Printf("Received %d events\n", len(result.Multiple))
        for _, event := range result.Multiple {
            fmt.Println("Event:", event)
        }
    }
    
    // Send a CLOSE message - returns single response
    closeMsg := `["CLOSE", "sub1"]`
    closeResult, err := c.Send(closeMsg)
    if err != nil {
        log.Fatal(err)
    }
    if closeResult.IsSingle {
        fmt.Println("CLOSE Result:", closeResult.Single)
    }
    
    // Send a COUNT message (NIP-45) - returns single response
    countMsg := `["COUNT", "count-sub", {"kinds": [1]}]`
    countResult, err := c.Send(countMsg)
    if err != nil {
        log.Fatal(err)
    }
    if countResult.IsSingle {
        fmt.Println("COUNT Result:", countResult.Single)
    }
    
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
- Unified `Send` method for all NIP-01 messages
- **Automatic looping for REQ messages** - `Send` returns `SendResult` with Multiple set for REQ messages
- MSGB format support for memory operations
- Event deduplication (automatically reset on new REQ messages)
- Newline-separated message handling
- Thread-safe operations
- Debug logging support
- Automatic synthesis of `Describe()` from `Info()` method

## Important: Loop Behavior

Unlike WebSocket connections, cassettes return one message per `send` call. The `Send` method now automatically detects REQ messages and loops until EOSE, returning all events in `SendResult.Multiple`. For other message types, it returns a single response in `SendResult.Single`.

## Requirements

- Go 1.21 or later
- wasmtime-go v23.0.0 or later