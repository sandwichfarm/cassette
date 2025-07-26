> ATTENTION: Alpha! WASM interface has not yet been standardized, cassettes you make today might be broken tomorrow. 

# Cassette 📼

Portable nostr relays that you can scrub, dub and cast notes from. Mostly rust, compiled to WASM. 

## What's New in v0.8.0

- **🎛️ New `deck` command**: Run a cassette deck - continuously record and serve cassettes as a writable relay
- **📼 Deck relay mode**: Accept events via WebSocket and automatically compile them into cassettes
- **♻️ Event deduplication**: Proper handling of replaceable events (NIP-01) across all cassettes
- **🔍 Complete NIP-01 compliance**: Full filter support including ids, kinds, authors, since, until, limit, and tag filters
- **📊 Improved REQ handling**: Collects all events before applying replaceable event logic and filters
- **🎚️ Renamed `play` to `scrub`**: Better reflects the analog tape metaphor of moving through content
- **🌐 New `listen` command**: Serve cassettes as a WebSocket relay with NIP-11 support
- **🔧 NIP-11 improvements**: Added `software` and `version` fields to relay information
- **🐛 Bug fixes**: Fixed WebSocket connection state issues and event handling in relay mode

> **Note**: The `play` command is deprecated but still works with a warning. Please use `scrub` instead.

## NIPs Look:

- [x] **NIP-01** - Basic Relay Protocol (REQ/EVENT/EOSE/CLOSE)
- [x] **NIP-11** - Relay Information Document (relay metadata and capabilities)
- [ ] **NIP-42** - Authentication (framework implemented, functionality planned)
- [x] **NIP-45** - Event Counts (COUNT queries for efficient event counting)
- [x] **NIP-50** - Search Capability (text search with relevance ranking)

## Quick Start

Download the latest `cli` binary from [releases](https://github.com/dskvr/cassette/releases/latest).

_**Knowledge is power:** The `cli` includes an ability to both `record` and `scrub` cassettes (create/read). Cassettes respond to `REQ` messages to `stdin` with `EVENT`, `NOTICE` and `EOSE` messages to `stdout`._

### Prerequisites
- [Rust and Cargo](https://www.rust-lang.org/)

### Record a cassette

_Pipe the events or provide the path to a json file with events._

```bash
# Basic cassette (NIP-01 only)
nak req -k 1 -l 100 wss://nos.lol | cassette record --name my-notes

# From a file
cassette record events.json --name my-notes

# Full-featured with relay info via NIP-11, counts with NIP-45 and search with NIP-50
cassette record events.json --name my-relay --nip-45 --nip-50 \
  --relay-description "Personal event archive"

# Output: my-notes.wasm
```

### Scrub through a cassette

_Scrub through events like rewinding an analog tape - dump all events or use filters_

```bash
# Get all events
cassette scrub my-notes.wasm

# Filter by kind
cassette scrub my-notes.wasm --kinds 1

# Filter by author
cassette scrub my-notes.wasm --authors npub1...

# Multiple filters
cassette scrub my-notes.wasm --kinds 1 --kinds 7 --limit 10

# Get relay information (NIP-11)
cassette scrub my-notes.wasm --info

# COUNT events (NIP-45)
cassette scrub my-notes.wasm --count --kinds 1

# Search events (NIP-50)  
cassette scrub my-notes.wasm --search "bitcoin lightning"

# Search with filters
cassette scrub my-notes.wasm --search "nostr" --kinds 1 --limit 10

# COUNT with custom relay info
cassette scrub my-notes.wasm --count --kinds 1 \
  --name "Archive" --description "Event archive"
```

### Dub a Mixtape

```bash
# Merge cassettes
cassette dub alice.wasm bob.wasm combined.wasm

# Merge with filters
cassette dub *.wasm filtered.wasm --kinds 1 --since 1700000000
```

### Play to Relays

```bash
# Broadcast events to a relay
cassette play my-notes.wasm --relays wss://relay.damus.io

# Broadcast to multiple relays
cassette play *.wasm --relays wss://nos.lol wss://relay.nostr.band

# Test with dry-run
cassette play archive.wasm --relays ws://localhost:7000 --dry-run
```

### Listen - Serve cassettes as a read-only relay

```bash
# Serve a single cassette as a WebSocket relay (auto-selects port)
cassette listen my-notes.wasm

# Serve multiple cassettes on a specific port
cassette listen *.wasm --port 8080

# Serve cassettes from a directory on all interfaces
cassette listen cassettes/*.wasm --bind 0.0.0.0 --port 1337

# Verbose mode to see connections
cassette listen archive.wasm --verbose

# Connect with any Nostr client
nak req ws://localhost:7777 -k 1 -l 10
```

### Deck - Writable relay with cassette archiving

```bash
# Run a writable relay that creates cassettes (relay mode)
cassette deck

# Run with custom settings
cassette deck --port 1337 --event-limit 1000 --size-limit 10

# Record from other relays continuously (record mode)
cassette deck --mode record --relays wss://relay.damus.io --kinds 1

# Verbose mode to see all operations
cassette deck -v

# Send events to the deck
echo "hello world" | nak event -c - | nak publish ws://localhost:7777
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
#   --nip-42           Enable NIP-42 (Authentication) - placeholder
#   --nip-45           Enable NIP-45 (Event Counts)
#   --nip-50           Enable NIP-50 (Search Capability)
#   --relay-name       Name for NIP-11 relay info
#   --relay-description Description for NIP-11 relay info
#   --relay-contact    Contact for NIP-11 relay info
#   --relay-pubkey     Owner pubkey for NIP-11 relay info

# Examples:
nak req -k 30023 wss://relay.nostr.band | cassette record -n "long-form"
cassette record my-events.json --name "my-backup"
cassette record events.json --nip-45 --name "countable" # With COUNT support
cassette record events.json --nip-50 --name "searchable" # With search support
cassette record events.json --nip-45 --nip-50 --name "Archive"
```

### `scrub` - Scrub through cassettes (send a `req`)

```bash
cassette scrub [OPTIONS] <CASSETTE>

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
#   --relay-name       Set name for dynamic NIP-11 info
#   --relay-description Set description for dynamic NIP-11 info
#   --relay-contact    Set contact for dynamic NIP-11 info
#   --relay-pubkey     Set owner pubkey for dynamic NIP-11 info

# Examples:
cassette scrub my-notes.wasm --kinds 1 --limit 50
cassette scrub archive.wasm --filter '{"#t": ["bitcoin", "lightning"]}'
cassette scrub events.wasm --output ndjson | grep "pattern"
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

### `play` - Broadcast events to Nostr relays

```bash
cassette play [OPTIONS] <CASSETTES...> --relays <RELAYS...>

# Options:
#   -r, --relays       Target relay URLs (required)
#   -c, --concurrency  Max concurrent connections (default: 5)
#   -t, --throttle     Delay between events in ms (default: 100)
#   --timeout          Connection timeout in seconds (default: 30)
#   --dry-run          Preview without sending

# Examples:
cassette play events.wasm --relays wss://relay.damus.io
cassette play *.wasm --relays wss://nos.lol wss://relay.nostr.band
cassette play archive.wasm --relays ws://localhost:7000 --dry-run

# Note: The 'cast' command is deprecated and will show a warning
```

### `listen` - Serve a read-only relay from one or more cassettes

```bash
cassette listen [OPTIONS] <CASSETTES...>

# Options:
#   -p, --port         Port to listen on (auto-selects if not specified)
#   --bind             Bind address (default: 127.0.0.1)
#   --tls              Enable TLS/WSS
#   --tls-cert         Path to TLS certificate
#   --tls-key          Path to TLS key
#   -v, --verbose      Show connection details

# Examples:
cassette listen my-notes.wasm                                    # Auto-select port
cassette listen *.wasm --port 8080                              # Serve all cassettes
cassette listen dir/*.wasm --bind 0.0.0.0 --port 1337          # Listen on all interfaces
cassette listen archive.wasm --verbose                          # Debug mode

# Features:
# - Serves cassettes as a NIP-01 compliant WebSocket relay
# - Supports NIP-11 relay information via HTTP with Accept: application/nostr+json
# - Handles multiple cassettes - aggregates responses from all loaded cassettes
# - Auto-selects available port if not specified
# - Compatible with all Nostr clients (nak, nostcat, web clients, etc.)
# - Each connection gets a fresh state to prevent cross-connection contamination
```

### `deck` - Run a cassette deck relay

```bash
cassette deck [OPTIONS]

# Options:
#   -m, --mode         Operation mode: 'relay' (writable) or 'record' (from relays)
#   -r, --relays       Relay URLs to record from (record mode only)
#   -n, --name         Base name for cassettes (default: deck)
#   -o, --output       Output directory (default: ./deck)
#   -p, --port         Port to serve on (default: 7777)
#   --bind             Bind address (default: 127.0.0.1)
#   -e, --event-limit  Max events per cassette (default: 10000)
#   -s, --size-limit   Max cassette size in MB (default: 100)
#   -d, --duration     Recording duration per cassette in seconds (default: 3600)
#   -f, --filter       Filter JSON for recording
#   -k, --kinds        Event kinds to record
#   --authors          Authors to filter
#   -v, --verbose      Show verbose output
#   --nip-11           Enable NIP-11 support
#   --nip-45           Enable NIP-45 (COUNT) support
#   --nip-50           Enable NIP-50 (search) support

# Examples:
# Relay mode - accept events and compile cassettes
cassette deck                                              # Default relay mode
cassette deck -p 1337 -e 100                              # Custom port and rotation
cassette deck -v --name archive                           # Verbose with custom name

# Record mode - record from other relays
cassette deck --mode record --relays wss://relay.damus.io
cassette deck -m record -r wss://nos.lol --kinds 1 --kinds 30023
cassette deck -m record -r wss://relay.nostr.band --filter '{"#t":["bitcoin"]}'

# Features:
# - Relay mode: Acts as a writable relay, stores events in rotating cassettes
# - Record mode: Continuously records from other relays
# - Auto-rotation based on event count, size, or time
# - Hot-loads compiled cassettes for immediate querying
# - Proper NIP-01 compliance with event deduplication
# - Replaceable event handling (kinds 0, 3, 10000-19999, 30000-39999)
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
Always included in all cassettes. Provides relay metadata and capability discovery.

```bash
# Record with custom relay information
cassette record events.json --name my-relay \
  --relay-name "My Relay" \
  --relay-description "My curated event collection" \
  --relay-contact "contact@example.com" \
  --relay-pubkey "npub1abc..."

# View relay information
cassette scrub my-relay.wasm --info

# Dynamic relay info at runtime
cassette scrub any-cassette.wasm --info \
  --relay-name "Custom Name" \
  --relay-description "Runtime description"
```

> **Note**: NIP-11 is always enabled. Relay info automatically includes `software: "@sandwichfarm/cassette"` and the current CLI version.

#### NIP-45 (Event Counts)
Adds COUNT query support for efficient event counting without retrieving full events.

```bash
# Record with COUNT support
cassette record events.json --name countable --nip-45

# Query event counts
cassette scrub countable.wasm --count --kinds 1        # Count kind 1 events
cassette scrub countable.wasm --count --authors npub1...  # Count by author
cassette scrub countable.wasm --count --since 1700000000 # Count recent events
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
cassette scrub searchable.wasm --search "bitcoin lightning"

# Search with additional filters
cassette scrub searchable.wasm --search "nostr protocol" --kinds 1 --limit 20

# Search supports extensions (advanced)
cassette scrub searchable.wasm --search "bitcoin domain:example.com"
cassette scrub searchable.wasm --search "news language:en"
```

### Combining NIPs

You can combine multiple NIPs for full-featured cassettes:

```bash
# Full-featured cassette
cassette record events.json --name full-relay \
  --nip-42 --nip-45 --nip-50 \
  --relay-description "Full-featured Nostr archive" \
  --relay-contact "contact@example.com" \
  --relay-pubkey "npub1abc..."

# Test all features
cassette scrub full-relay.wasm --info                    # Show relay info
cassette scrub full-relay.wasm --count --kinds 1         # Count events
cassette scrub full-relay.wasm --search "bitcoin"        # Search events
cassette scrub full-relay.wasm --kinds 1 --limit 10      # Get events
```

### Filtering and Querying

Cassettes support comprehensive NIP-01 filtering:

```bash
# By event kind
cassette scrub relay.wasm --kinds 1 --kinds 30023

# By author (accepts npub, hex, or partial)
cassette scrub relay.wasm --authors npub1abc... --authors npub1def...

# Time-based filtering
cassette scrub relay.wasm --since 1700000000 --until 1700100000

# Combination filters
cassette scrub relay.wasm --kinds 1 --authors npub1abc... --limit 50

# Custom JSON filters (advanced)
cassette scrub relay.wasm --filter '{"#t": ["bitcoin"], "#p": ["npub1..."]}'

# Output formats
cassette scrub relay.wasm --kinds 1 --output ndjson | jq .
```

### Performance and Size Optimization

Different NIP combinations affect cassette size and capabilities:

- **Base (NIP-01 + NIP-11)**: Smallest size, basic querying with relay info
- **+ NIP-45**: Adds ~5KB, efficient event counting
- **+ NIP-50**: Adds ~4KB, text search with relevance ranking
- **+ NIP-42**: Adds ~3KB, authentication framework (placeholder)

Choose NIPs based on your use case:
- **Basic Archive**: Default (NIP-01 + NIP-11 included)
- **Analytics**: Add NIP-45 for counting and analysis
- **Search**: Add NIP-50 for text search capabilities
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
│   ├── py/                 # Python loader
│   ├── rust/               # Rust loader
│   ├── go/                 # Go loader
│   ├── cpp/                # C++ loader
│   └── dart/               # Dart loader
├── boombox/               # WebSocket relay server for cassettes
└── gui/                   # Web interface for testing
```

### Components

- **CLI**: Command-line tool for creating and querying cassettes
- **Cassette Tools**: Rust library providing memory management and modular NIP implementations (NIP-01, NIP-11, NIP-42, NIP-45, NIP-50)
- **Loaders**: Language-specific libraries for loading and executing cassettes in JavaScript/TypeScript, Python, Rust, Go, C++, and Dart
- **Boombox**: WebSocket server that serves cassettes as Nostr relays
- **GUI**: Web interface for testing cassettes in the browser

## WebAssembly Interface

Cassettes implement a simplified WebAssembly interface:

```rust
// Core export (v0.5.0+)
fn send(ptr, len) -> ptr       // Handle all NIP-01 messages (REQ, CLOSE, EVENT, COUNT)

// NIP-11 support (always included)
fn info() -> ptr               // Relay information document

// NIP-11 dynamic configuration
fn set_relay_info(ptr, len) -> i32  // Set relay metadata at runtime

// Memory management
fn alloc_buffer(size) -> ptr
fn dealloc_string(ptr, len)
fn get_allocation_size(ptr) -> size
```

The `send` method accepts any NIP-01 protocol message in JSON format, including:
- `["REQ", subscription_id, filters...]` - Query events
- `["CLOSE", subscription_id]` - Close subscription
- `["EVENT", subscription_id, event]` - Submit event (for compatible cassettes)
- `["COUNT", subscription_id, filters...]` - Count events (NIP-45)

This unified interface allows cassettes to be loaded by any compatible runtime without language-specific bindings.

## Language Loaders

Cassette provides official loaders for multiple programming languages, allowing you to integrate cassettes into your applications regardless of your tech stack. All loaders implement the same interface and provide consistent functionality across languages.

### Available Loaders

#### JavaScript/TypeScript
- **Package**: `cassette-loader`
- **Installation**: `npm install cassette-loader`
- **Features**: Browser and Node.js support, TypeScript definitions, event deduplication
- **[Documentation](./loaders/js/README.md)**

```javascript
import { loadCassette } from 'cassette-loader';

const result = await loadCassette('/path/to/cassette.wasm');
if (result.success) {
    const response = result.cassette.methods.send('["REQ", "sub1", {"kinds": [1]}]');
    console.log(response);
}
```

#### Python
- **Package**: `cassette-loader`
- **Installation**: `pip install cassette-loader`
- **Features**: Memory management, event deduplication, debug mode
- **[Documentation](./loaders/py/README.md)**

```python
from cassette_loader import load_cassette

result = load_cassette(wasm_bytes, name='my-cassette')
if result['success']:
    cassette = result['cassette']
    response = cassette.send('["REQ", "sub1", {"kinds": [1]}]')
```

#### Rust
- **Crate**: `cassette-loader`
- **Installation**: Add to `Cargo.toml`
- **Features**: Native performance, thread-safe event tracking, comprehensive error handling
- **[Documentation](./loaders/rust/README.md)**

```rust
use cassette_loader::Cassette;

let mut cassette = Cassette::load("path/to/cassette.wasm", true)?;
let response = cassette.send(r#"["REQ", "sub1", {"kinds": [1]}]"#)?;
```

#### Go
- **Package**: `github.com/cassette/loaders/go`
- **Installation**: `go get github.com/cassette/loaders/go`
- **Features**: Thread-safe operations, debug logging
- **[Documentation](./loaders/go/README.md)**

```go
import cassette "github.com/cassette/loaders/go"

c, err := cassette.LoadCassette("path/to/cassette.wasm", true)
result, err := c.Send(`["REQ", "sub1", {"kinds": [1]}]`)
```

#### C++
- **Library**: `cassette-loader`
- **Installation**: CMake integration
- **Features**: Exception-based error handling, MSGB format support
- **[Documentation](./loaders/cpp/README.md)**

```cpp
#include <cassette_loader.hpp>

cassette::Cassette cassette("path/to/cassette.wasm", true);
std::string response = cassette.send(R"(["REQ", "sub1", {"kinds": [1]}])");
```

#### Dart
- **Package**: `cassette_loader`
- **Installation**: Add to `pubspec.yaml`
- **Features**: Web support, async operations
- **[Documentation](./loaders/dart/README.md)**

```dart
import 'package:cassette_loader/cassette_loader.dart';

final cassette = await Cassette.load('path/to/cassette.wasm');
final response = cassette.send('["REQ", "sub1", {"kinds": [1]}]');
```

### Common Features

All loaders provide:
- **Unified Interface**: Single `send()` method for all NIP-01 messages
- **Event Deduplication**: Automatic filtering of duplicate events
- **Memory Management**: Proper handling of WASM memory allocation/deallocation
- **Debug Support**: Optional verbose logging for troubleshooting
- **Error Handling**: Consistent error reporting across languages

### Creating Your Own Loader

If you need to create a loader for a language not listed above, implement these core functions:

1. **Load WASM module** - Instantiate the WebAssembly module
2. **Memory management** - Handle string passing between host and WASM
3. **Call `send()` function** - Pass messages and retrieve responses
4. **Event tracking** - Implement deduplication for EVENT messages

See the existing loader implementations for reference patterns.

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
pub extern "C" fn send(ptr: *const u8, len: usize) -> *mut u8 {
    let message = ptr_to_string(ptr, len);
    
    // Parse the message to determine type
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&message).unwrap();
    let command = parsed[0].as_str().unwrap();
    
    let response = match command {
        "REQ" => handle_req_command(&parsed),
        "CLOSE" => handle_close_command(&parsed),
        "COUNT" => handle_count_command(&parsed),
        _ => json!(["NOTICE", "Unknown command"]).to_string(),
    };
    
    string_to_ptr(response)
}
```

See `cassette-tools/` for the full API.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

AI Tips:
1. Use BDD! Write tests and Docs First. The default agent on this repo uses tests and docs as its North Star.
2. Use Context Programming for mission critical features. AKA Don't vibe API or business logic, but yes vibe interfaces, SPAs and prototypes.

## Migration Guide

### v0.8.0
- **Deck duplicate event rejection**: The deck relay now properly rejects duplicate events per NIP-01
- **Fixed REQ limit handling**: The deck relay now collects ALL events before applying filters and limits
- **Improved event deduplication**: Proper handling of replaceable and parameterized replaceable events
- **Complete filter implementation**: Full support for ids, kinds, authors, since, until, limit, and tag filters

### v0.7.1
- New `deck` command for running a writable relay that creates cassettes
- NIP-11 is now always enabled in all cassettes (no need for --nip-11 flag)
- Improved NIP-01 compliance with proper replaceable event handling
- The `deck` relay collects all events before applying filters and limits

### v0.6.2
- The `play` command has been renamed to `scrub` to better reflect the analog tape metaphor
- The old `play` command still works but shows a deprecation warning
- Update your scripts to use `cassette scrub` instead of `cassette play`
- NIP-11 relay info now includes `software` and `version` fields automatically

### Note on `cast` vs `play`
- `cast` is the deprecated command that shows a warning
- `play` is the current command for broadcasting events to relays
- `scrub` is for reading/querying cassettes (formerly `play`)

## License

MIT
