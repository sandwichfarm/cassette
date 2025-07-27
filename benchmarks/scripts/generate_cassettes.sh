#!/bin/bash

# Generate sample cassette files for benchmarking

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SAMPLES_DIR="$SCRIPT_DIR/../samples"

# Create samples directory if it doesn't exist
mkdir -p "$SAMPLES_DIR"

echo "🔨 Generating sample cassettes for benchmarking..."

# Generate small cassette (100 events)
echo "📦 Creating small.wasm (100 events)..."
nak req -k 1 -l 100 wss://relay.damus.io wss://nos.lol | \
    cassette -o "$SAMPLES_DIR/small.wasm" -n "small" -d "Small benchmark cassette with 100 events"

# Generate medium cassette (1,000 events)
echo "📦 Creating medium.wasm (1,000 events)..."
nak req -k 1 -k 0 -k 7 -l 1000 --since 3d wss://relay.damus.io wss://nos.lol wss://relay.nostr.band | \
    cassette -o "$SAMPLES_DIR/medium.wasm" -n "medium" -d "Medium benchmark cassette with 1k events"

# Generate large cassette (10,000 events)
echo "📦 Creating large.wasm (10,000 events)..."
echo "This will take a while..."
{
    # Text notes (kind 1) - 5000 events
    echo "  Collecting text notes..." >&2
    nak req -k 1 -l 5000 --since 7d wss://relay.damus.io wss://nos.lol wss://relay.nostr.band
    
    # Profile metadata (kind 0) - 2000 events  
    echo "  Collecting profile metadata..." >&2
    nak req -k 0 -l 2000 --since 14d wss://relay.damus.io wss://purplepag.es
    
    # Reactions (kind 7) - 2000 events
    echo "  Collecting reactions..." >&2
    nak req -k 7 -l 2000 --since 3d wss://relay.damus.io wss://nos.lol
    
    # Reposts (kind 6) - 1000 events
    echo "  Collecting reposts..." >&2
    nak req -k 6 -l 1000 --since 7d wss://relay.damus.io
} | cassette -o "$SAMPLES_DIR/large.wasm" -n "large" -d "Large benchmark cassette with 10k mixed events"

echo ""
echo "✅ Sample cassettes generated successfully!"
echo ""
echo "📊 Cassette sizes:"
ls -lh "$SAMPLES_DIR"/*.wasm 2>/dev/null || echo "  No .wasm files found yet"