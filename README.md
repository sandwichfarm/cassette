# Cassette

Store Nostr events in NIP-01 compatible (read-only) portable WebAssembly relays.

## Quick Start

Download the latest binary from [releases](https://github.com/dskvr/cassette/releases/latest).

### Record a cassette

_Pipe the events or provide the path to a json file with events._

```bash
# From a relay
nak req -k 1 -l 100 wss://nos.lol | cassette record --name my-notes

# From a file
cassette record events.json --name my-notes

# Output: my-notes.wasm
```

### Play a cassette

_Dump all the events, or use filters_

```bash
# Get all events
cassette play my-notes.wasm

# Filter by kind
cassette play my-notes.wasm --kinds 1

# Filter by author
cassette play my-notes.wasm --authors npub1...

# Multiple filters
cassette play my-notes.wasm --kinds 1 --kinds 7 --limit 10
```

### Dub a Mixtape

```bash
# Merge cassettes
cassette dub alice.wasm bob.wasm combined.wasm

# Merge with filters
cassette dub *.wasm filtered.wasm --kinds 1 --since 1700000000
```

## What is a Cassette?

A cassette is a WebAssembly module containing Nostr events that implements the NIP-01 relay protocol. Think of it as a portable, queryable database that runs anywhere WebAssembly does - browsers, servers, edge workers, or CLI tools.

### Use Cases

- **Archival**: Store important events in a portable format
- **Testing**: Create deterministic test fixtures for Nostr clients
- **Offline**: Query events without network access
- **Distribution**: Share curated event collections
- **Privacy**: Keep events local while maintaining relay compatibility

## CLI Commands

### `record` - Record events onto cassettes

```bash
cassette record [OPTIONS] [INPUT_FILE]

# Options:
#   -n, --name         Name for the cassette
#   -d, --description  Description of contents
#   -a, --author       Author/curator name
#   -o, --output       Output directory (default: ./cassettes)
#   --no-bindings      Skip JavaScript bindings, WASM only

# Examples:
nak req -k 30023 wss://relay.nostr.band | cassette record -n "long-form"
cassette record my-events.json --name "my-backup"
```

### `play` - Play cassettes (send a `req`)

```bash
cassette play [OPTIONS] <CASSETTE>

# Options:
#   -s, --subscription  Subscription ID (default: sub1)
#   -f, --filter       Custom filter JSON
#   -k, --kinds        Event kinds to return
#   -a, --authors      Filter by authors
#   -l, --limit        Maximum events to return
#   --since            Events after timestamp
#   --until            Events before timestamp
#   -o, --output       Output format: json or ndjson

# Examples:
cassette play my-notes.wasm --kinds 1 --limit 50
cassette play archive.wasm --filter '{"#t": ["bitcoin", "lightning"]}'
cassette play events.wasm --output ndjson | grep "pattern"
```

### `dub` - Combine cassettes into a Mixtape

```bash
cassette dub [OPTIONS] <CASSETTES...> <OUTPUT>

# Options:
#   -n, --name         Name for output cassette
#   -d, --description  Description
#   -a, --author       Author/curator
#   -f, --filter       Apply filters when combining
#   -k, --kinds        Include only these kinds
#   --authors          Include only these authors
#   -l, --limit        Limit total events
#   --since            Events after timestamp
#   --until            Events before timestamp

# Examples:
cassette dub cassette1.wasm cassette2.wasm combined.wasm
cassette dub *.wasm all-events.wasm --name "Complete Archive"
cassette dub raw/*.wasm clean.wasm --kinds 1 --kinds 30023
```

## Building from Source

### Prerequisites

- Rust and Cargo
- wasm32-unknown-unknown target: `rustup target add wasm32-unknown-unknown`

### Build

```bash
git clone https://github.com/dskvr/cassette.git
cd cassette
cargo build --release

# Binary will be at: target/release/cassette
```

## Project Structure

```
cassette/
├── cli/                    # Command-line interface
├── cassette-tools/         # Core WASM functionality
├── loaders/                # Language-specific cassette loaders
│   ├── js/                 # JavaScript/TypeScript loader
│   └── py/                 # Python loader
├── boombox/               # WebSocket relay server for cassettes
└── gui/                   # Web interface for testing
```

### Components

- **CLI**: Command-line tool for creating and querying cassettes
- **Cassette Tools**: Rust library providing memory management and NIP-01 implementation helpers
- **Loaders**: Language-specific libraries for loading and executing cassettes
- **Boombox**: WebSocket server that serves cassettes as Nostr relays
- **GUI**: Web interface for testing cassettes in the browser

## WebAssembly Interface

Cassettes implement a standardized WebAssembly interface:

```rust
// Required exports
fn describe() -> String      // Metadata about the cassette
fn req(ptr, len) -> ptr     // Handle REQ messages
fn close(ptr, len) -> ptr   // Handle CLOSE messages

// Memory management
fn alloc_buffer(size) -> ptr
fn dealloc_string(ptr, len)
fn get_allocation_size(ptr) -> size
```

This allows cassettes to be loaded by any compatible runtime without language-specific bindings.

## Advanced Usage

### Running Cassettes as Relays

Using the Boombox server, cassettes can be served as WebSocket endpoints:

```bash
cd boombox
bun install
bun index.ts

# Cassettes in ./cassettes directory are now available at ws://localhost:3001
```

### Creating Custom Cassettes

Beyond recording existing events, you can create cassettes programmatically using `cassette-tools`:

```rust
use cassette_tools::{string_to_ptr, ptr_to_string};

#[no_mangle]
pub extern "C" fn req(ptr: *const u8, len: usize) -> *mut u8 {
    let request = ptr_to_string(ptr, len);
    // Process request, return response
    string_to_ptr(response)
}
```

See `cassette-tools/` for the full API.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT
