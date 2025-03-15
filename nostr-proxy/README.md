# Nostr Proxy

A WebSocket proxy that forwards Nostr protocol messages between clients and the Cassette Boombox server, facilitating testing with standard Nostr clients.

## Overview

The Nostr Proxy is a lightweight relay facade that:

- Acts as a standard Nostr relay for clients to connect to
- Forwards all messages to the Boombox server without modification
- Returns responses from Boombox to the connected clients
- Provides a simplified way to test Nostr clients with the Cassette platform

This proxy is particularly useful for testing, as it allows you to use standard Nostr clients and tools while interacting with your Boombox server implementation.

## Installation

To install dependencies:

```bash
bun install
```

## Usage

### Starting the Proxy

The proxy can be run directly with bun:

```bash
# Default configuration (port 3000, forwarding to ws://localhost:3001)
bun run index.ts

# Custom port
PORT=4000 bun run index.ts

# Custom target
PORT=4000 TARGET_URL=ws://localhost:5000 bun run index.ts
```

Alternatively, you can use the project's root Makefile:

```bash
# From the project root
make start-services
```

### Connecting to the Proxy

Connect your Nostr client to:

```
ws://localhost:3000  # (or whatever port you configured)
```

Any standard Nostr client should work, including:
- Web clients like Iris or Damus
- Command-line tools like [nak](https://github.com/fiatjaf/nak)
- Custom clients you're developing

## Environment Variables

- `PORT`: The port on which the proxy listens (default: 3000)
- `TARGET_URL`: The WebSocket URL to forward messages to (default: ws://localhost:3001)
- `DEBUG`: Set to `true` for more verbose logging of messages

## Testing

### Integration Testing

You can test the proxy as part of the entire system:

```bash
# From the project root
make test
```

This will start both the Boombox server and the Nostr proxy, then run a series of tests to verify functionality.

### Manual Testing

For manual testing:

1. Start the proxy: `bun run index.ts`
2. Ensure the Boombox server is running (on port 3001 by default)
3. Connect a Nostr client to `ws://localhost:3000`
4. Send requests and observe the responses

## Development

The proxy is designed to be simple and lightweight, using Bun's built-in WebSocket support. The main functionality is implemented in `index.ts`, which:

1. Creates a WebSocket server for clients to connect to
2. Establishes connections to the Boombox server
3. Forwards messages between clients and the server
4. Handles connection management and error cases

---

This project was created using `bun init` in bun v1.2.4. [Bun](https://bun.sh) is a fast all-in-one JavaScript runtime.
