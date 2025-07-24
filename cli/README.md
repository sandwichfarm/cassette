# Cassette CLI

Command-line tool for creating, querying, and managing Nostr event cassettes.

## Overview

The Cassette CLI provides tools to:
- **Record** Nostr events from files or streams into portable WebAssembly modules
- **Query** cassettes using NIP-01 filters
- **Combine** multiple cassettes into new ones

## Prerequisites

The Cassette CLI requires Rust to be installed on your system for WASM module generation.

**Install Rust:**
```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown
```

## Installation

Download pre-built binaries from [releases](https://github.com/dskvr/cassette/releases/latest) or build from source:

```bash
# Prerequisites
rustup target add wasm32-unknown-unknown

# Build
cargo build --release

# Install
cargo install --path .
```

## Commands

### `record` - Create cassettes from events

Record Nostr events into a WebAssembly cassette:

```bash
# From file
cassette record events.json --name my-cassette

# From stdin (e.g., piped from nak)
nak req -k 1 -l 100 wss://relay.nostr.band | cassette record --name notes

# With metadata
cassette record events.json \
  --name "my-archive" \
  --description "Personal notes backup" \
  --author "Alice"
```

Options:
- `-n, --name` - Name for the cassette
- `-d, --description` - Description of contents
- `-a, --author` - Author/curator name
- `-o, --output` - Output directory (default: ./cassettes)
- `--no-bindings` - Skip JavaScript bindings generation

### `req` - Query cassettes

Query events from a cassette using NIP-01 filters:

```bash
# Get all events
cassette play my-cassette.wasm

# Filter by event kind
cassette play my-cassette.wasm --kinds 1

# Multiple filters
cassette play my-cassette.wasm \
  --kinds 1 --kinds 7 \
  --authors npub1... \
  --limit 50

# Custom filter JSON
cassette play my-cassette.wasm \
  --filter '{"#t": ["bitcoin", "nostr"]}'

# Output as NDJSON for piping
cassette play my-cassette.wasm --output ndjson | grep "pattern"
```

Options:
- `-s, --subscription` - Subscription ID (default: sub1)
- `-f, --filter` - Custom filter JSON
- `-k, --kinds` - Filter by event kinds
- `-a, --authors` - Filter by author pubkeys
- `-l, --limit` - Maximum events to return
- `--since` - Events after timestamp
- `--until` - Events before timestamp
- `-o, --output` - Output format: json or ndjson

### `dub` - Combine cassettes

Merge multiple cassettes into a new one, optionally applying filters:

```bash
# Simple merge
cassette dub cassette1.wasm cassette2.wasm combined.wasm

# Merge with filters
cassette dub *.wasm filtered.wasm \
  --kinds 1 --kinds 30023 \
  --since 1700000000

# Named output with metadata
cassette dub alice.wasm bob.wasm carol.wasm team.wasm \
  --name "team-notes" \
  --description "Combined team cassettes" \
  --author "Team Lead"
```

Options:
- `-n, --name` - Name for output cassette
- `-d, --description` - Description
- `-a, --author` - Author/curator
- `-f, --filter` - Apply filters when combining
- `-k, --kinds` - Include only these event kinds
- `--authors` - Include only these authors
- `-l, --limit` - Limit total events
- `--since` - Events after timestamp
- `--until` - Events before timestamp

## Output Format

### Cassette Files

The `record` and `dub` commands generate:
- `<name>.wasm` - The WebAssembly module containing events and query logic
- JavaScript bindings (unless `--no-bindings` is used)

### Query Output

The `req` command outputs:
- JSON format (default): Pretty-printed Nostr protocol messages
- NDJSON format: One event per line for piping to other tools

## Examples

### Archive Personal Notes
```bash
# Get your notes from a relay
nak req -k 1 -a <your-pubkey> wss://relay.damus.io | \
  cassette record --name "my-notes-$(date +%Y%m%d)"
```

### Create Test Fixtures
```bash
# Record specific events for testing
cassette record test-events.json --name "test-fixture"

# Query to verify
cassette play test-fixture.wasm --kinds 1
```

### Combine and Filter Archives
```bash
# Merge yearly archives into one, keeping only articles
cassette dub archives-*.wasm all-articles.wasm \
  --kinds 30023 \
  --name "all-articles" \
  --description "All long-form articles"
```

## Integration

Cassettes can be:
- Loaded by the Boombox server to serve as Nostr relays
- Queried directly via CLI for offline access
- Imported by JavaScript/Python applications using the respective loaders
- Tested in the browser using the GUI interface

## Development

The CLI is part of the Cassette monorepo. Key directories:
- `src/main.rs` - CLI implementation
- `src/generator.rs` - Cassette generation logic
- `src/templates/` - Rust code templates for cassettes

To add new commands, extend the `Commands` enum in `main.rs`.