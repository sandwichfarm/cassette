# Nostr Proxy

A simple WebSocket proxy that forwards Nostr protocol messages between clients and the Cassette Boombox server.

## Overview

This proxy acts as a Nostr relay without implementing any Nostr-specific logic. It simply receives WebSocket messages from Nostr clients and forwards them to the Boombox server, which in turn passes them to the appropriate cassette implementation.

## Usage

### Starting the Proxy

```bash
# Default configuration (port 3000, forwarding to ws://localhost:3001)
bun run index.ts

# Custom port
PORT=4000 bun run index.ts

# Custom target
PORT=4000 TARGET_URL=ws://localhost:5000 bun run index.ts
```

### Connecting to the Proxy

Connect your Nostr client to:

```
ws://localhost:3000  # (or whatever port you configured)
```

## Environment Variables

- `PORT`: The port on which the proxy listens (default: 3000)
- `TARGET_URL`: The WebSocket URL to forward messages to (default: ws://localhost:3001)

## Development

The proxy is designed to be simple and lightweight, using Bun's built-in WebSocket support.
