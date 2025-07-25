> ATTENTION: Alpha! WASM interface is being standardized, cassettes you make today might be broken tomorrow.

# Cassette

Modular NIP-compatible, portable WebAssembly read-only relays with support for NIP-01, NIP-11, NIP-42, NIP-45, and NIP-50

## NIP Implementation Status

- [x] **NIP-01** - Basic Relay Protocol (REQ/EVENT/EOSE/CLOSE)
- [x] **NIP-11** - Relay Information Document (relay metadata and capabilities)
- [ ] **NIP-42** - Authentication (framework implemented, functionality planned)
- [x] **NIP-45** - Event Counts (COUNT queries for efficient event counting)
- [x] **NIP-50** - Search Capability (text search with relevance ranking)

## Quick Start

Download the latest `cli` binary from [releases](https://github.com/dskvr/cassette/releases/latest).

_**Knowledge is power:** The `cli` includes an ability to both `record` and `play` cassettes (create/read). Cassettes respond to `REQ` messages to `stdin` with `EVENT`, `NOTICE` and `EOSE` messages to `stdout`._

### Prerequisites
- [Rust and Cargo](https://www.rust-lang.org/)

### Record a cassette

_Pipe the events or provide the path to a json file with events._

```bash
# Basic cassette (NIP-01 only)
nak req -k 1 -l 100 wss://nos.lol | cassette record --name my-notes

# From a file
cassette record events.json --name my-notes

# With COUNT support (NIP-45)
cassette record events.json --name my-notes --nip-45

# With search support (NIP-50)
cassette record events.json --name my-notes --nip-50

# Full-featured with relay info (NIP-11 + NIP-45 + NIP-50)
cassette record events.json --name my-relay --nip-11 --nip-45 --nip-50 \
  --relay-name "My Archive" --relay-description "Personal event archive"

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

# Get relay information (NIP-11)
cassette play my-notes.wasm --info

# COUNT events (NIP-45)
cassette play my-notes.wasm --count --kinds 1

# Search events (NIP-50)  
cassette play my-notes.wasm --search "bitcoin lightning"

# Search with filters
cassette play my-notes.wasm --search "nostr" --kinds 1 --limit 10

# COUNT with custom relay info
cassette play my-notes.wasm --count --kinds 1 \
  --relay-name "Archive" --relay-description "Event archive"
```

### Dub a Mixtape

```bash
# Merge cassettes
cassette dub alice.wasm bob.wasm combined.wasm

# Merge with filters
cassette dub *.wasm filtered.wasm --kinds 1 --since 1700000000
```

### Cast to Relays

```bash
# Broadcast events to a relay
cassette cast my-notes.wasm --relays wss://relay.damus.io

# Broadcast to multiple relays
cassette cast *.wasm --relays wss://nos.lol wss://relay.nostr.band

# Test with dry-run
cassette cast archive.wasm --relays ws://localhost:7000 --dry-run
```

## What is a Cassette?

A cassette is a WebAssembly module containing Nostr events that implements the Nostr relay protocol. Cassettes support modular NIP implementations including NIP-01 (basic relay protocol), NIP-11 (relay information document), NIP-42 (authentication), and NIP-45 (event counts). Think of it as a portable, queryable database that runs anywhere WebAssembly does - browsers, servers, edge workers, or CLI tools.

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
#   --nip-11           Enable NIP-11 (Relay Information Document)
#   --nip-42           Enable NIP-42 (Authentication)
#   --nip-45           Enable NIP-45 (Event Counts)
#   --nip-50           Enable NIP-50 (Search Capability)
#   --relay-name       Relay name for NIP-11
#   --relay-description Relay description for NIP-11

# Examples:
nak req -k 30023 wss://relay.nostr.band | cassette record -n "long-form"
cassette record my-events.json --name "my-backup"
cassette record events.json --nip-45 --name "countable" # With COUNT support
cassette record events.json --nip-50 --name "searchable" # With search support
cassette record events.json --nip-11 --nip-45 --nip-50 --relay-name "Archive"
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
#   --info             Show NIP-11 relay information
#   --count            Perform COUNT query (NIP-45)
#   --search           Search query for NIP-50 text search
#   --relay-name       Set relay name for dynamic info

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

### `cast` - Broadcast events to Nostr relays

```bash
cassette cast [OPTIONS] <CASSETTES...> --relays <RELAYS...>

# Options:
#   -r, --relays       Target relay URLs (required)
#   -c, --concurrency  Max concurrent connections (default: 5)
#   -t, --throttle     Delay between events in ms (default: 100)
#   --timeout          Connection timeout in seconds (default: 30)
#   --dry-run          Preview without sending

# Examples:
cassette cast events.wasm --relays wss://relay.damus.io
cassette cast *.wasm --relays wss://nos.lol wss://relay.nostr.band
cassette cast archive.wasm --relays ws://localhost:7000 --dry-run
```

## Advanced Configuration

### Modular NIP Support

Cassettes support modular NIP (Nostr Implementation Possibilities) implementation, allowing you to build cassettes with exactly the features you need:

#### NIP-01 (Basic Relay Protocol)
Always included. Provides REQ/EVENT/EOSE/CLOSE message handling.

```bash
# Basic cassette with only NIP-01
cassette record events.json --name basic-relay
```

#### NIP-11 (Relay Information Document)
Always available for basic info. Enables dynamic relay metadata and capability discovery.

```bash
# With static relay information
cassette record events.json --name my-relay --nip-11 \
  --relay-name "Personal Archive" \
  --relay-description "My curated event collection" \
  --relay-contact "npub1abc..."

# View relay information
cassette play my-relay.wasm --info

# Dynamic relay info at runtime
cassette play any-cassette.wasm --info \
  --relay-name "Custom Name" \
  --relay-description "Runtime description"
```

#### NIP-45 (Event Counts)
Adds COUNT query support for efficient event counting without retrieving full events.

```bash
# Record with COUNT support
cassette record events.json --name countable --nip-45

# Query event counts
cassette play countable.wasm --count --kinds 1        # Count kind 1 events
cassette play countable.wasm --count --authors npub1...  # Count by author
cassette play countable.wasm --count --since 1700000000 # Count recent events
```

#### NIP-42 (Authentication)
Framework for authentication support (currently placeholder for future implementation).

```bash
# Record with auth framework
cassette record events.json --name secure --nip-42
```

#### NIP-50 (Search Capability)
Adds text search functionality with relevance-based ranking instead of chronological ordering.

```bash
# Record with search support
cassette record events.json --name searchable --nip-50

# Basic text search
cassette play searchable.wasm --search "bitcoin lightning"

# Search with additional filters
cassette play searchable.wasm --search "nostr protocol" --kinds 1 --limit 20

# Search supports extensions (advanced)
cassette play searchable.wasm --search "bitcoin domain:example.com"
cassette play searchable.wasm --search "news language:en"
```

### Combining NIPs

You can combine multiple NIPs for full-featured cassettes:

```bash
# Full-featured cassette
cassette record events.json --name full-relay \
  --nip-11 --nip-42 --nip-45 --nip-50 \
  --relay-name "Complete Archive" \
  --relay-description "Full-featured Nostr archive" \
  --relay-contact "contact@example.com"

# Test all features
cassette play full-relay.wasm --info                    # Show relay info
cassette play full-relay.wasm --count --kinds 1         # Count events
cassette play full-relay.wasm --search "bitcoin"        # Search events
cassette play full-relay.wasm --kinds 1 --limit 10      # Get events
```

### Filtering and Querying

Cassettes support comprehensive NIP-01 filtering:

```bash
# By event kind
cassette play relay.wasm --kinds 1 --kinds 30023

# By author (accepts npub, hex, or partial)
cassette play relay.wasm --authors npub1abc... --authors npub1def...

# Time-based filtering
cassette play relay.wasm --since 1700000000 --until 1700100000

# Combination filters
cassette play relay.wasm --kinds 1 --authors npub1abc... --limit 50

# Custom JSON filters (advanced)
cassette play relay.wasm --filter '{"#t": ["bitcoin"], "#p": ["npub1..."]}'

# Output formats
cassette play relay.wasm --kinds 1 --output ndjson | jq .
```

### Performance and Size Optimization

Different NIP combinations affect cassette size and capabilities:

- **NIP-01 only**: Smallest size, basic querying
- **+ NIP-11**: Adds ~2KB, relay metadata support  
- **+ NIP-45**: Adds ~5KB, efficient event counting
- **+ NIP-50**: Adds ~4KB, text search with relevance ranking
- **+ NIP-42**: Adds ~3KB, authentication framework

Choose NIPs based on your use case:
- **Archival**: NIP-01 + NIP-11 for basic archive with metadata
- **Analytics**: NIP-01 + NIP-11 + NIP-45 for counting and analysis
- **Search**: NIP-01 + NIP-11 + NIP-50 for text search capabilities
- **Full-featured**: All NIPs for maximum compatibility

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
├── cassette-tools/         # Core WASM functionality and modular NIP support
├── loaders/                # Language-specific cassette loaders
│   ├── js/                 # JavaScript/TypeScript loader
│   └── py/                 # Python loader
├── boombox/               # WebSocket relay server for cassettes
└── gui/                   # Web interface for testing
```

### Components

- **CLI**: Command-line tool for creating and querying cassettes
- **Cassette Tools**: Rust library providing memory management and modular NIP implementations (NIP-01, NIP-11, NIP-42, NIP-45, NIP-50)
- **Loaders**: Language-specific libraries for loading and executing cassettes
- **Boombox**: WebSocket server that serves cassettes as Nostr relays
- **GUI**: Web interface for testing cassettes in the browser

## WebAssembly Interface

Cassettes implement a standardized WebAssembly interface:

```rust
// Required exports
fn describe() -> String         // Metadata about the cassette
fn req(ptr, len) -> ptr        // Handle REQ and COUNT messages  
fn close(ptr, len) -> ptr      // Handle CLOSE messages

// NIP-11 support (always available)
fn info() -> ptr               // Relay information document

// NIP-11 dynamic configuration (when nip11 feature enabled)
fn set_relay_info(ptr, len) -> i32  // Set relay metadata

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
