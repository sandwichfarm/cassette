#!/bin/bash

# Generate small sample cassettes for benchmarking
# Start with very small samples to ensure the process works

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SAMPLES_DIR="$SCRIPT_DIR/../samples"

# Create samples directory if it doesn't exist
mkdir -p "$SAMPLES_DIR"

echo "🔨 Generating small sample cassettes for benchmarking..."

# Generate tiny cassette (10 events) to test the process
echo "📦 Creating tiny.wasm (10 events)..."
nak req -k 1 -l 10 \
    wss://relay.damus.io | \
    cassette record -o "$SAMPLES_DIR" -n "tiny" -d "Tiny test cassette with 10 events"

# If tiny works, generate small cassette (100 events)
if [ -f "$SAMPLES_DIR/tiny.wasm" ]; then
    echo "✅ Tiny cassette created successfully!"
    echo "📦 Creating small.wasm (100 events)..."
    nak req -k 1 -l 100 \
        wss://relay.damus.io \
        wss://nos.lol | \
        cassette record -o "$SAMPLES_DIR" -n "small" -d "Small benchmark cassette with 100 events"
fi

# If small works, generate medium cassette (1000 events)
if [ -f "$SAMPLES_DIR/small.wasm" ]; then
    echo "✅ Small cassette created successfully!"
    echo "📦 Creating medium.wasm (1000 events)..."
    nak req -k 1 -l 1000 --since "3 days ago" \
        wss://relay.damus.io \
        wss://nos.lol | \
        cassette record -o "$SAMPLES_DIR" -n "medium" -d "Medium benchmark cassette with 1000 events"
fi

# If medium works, generate benchmark-large cassette (5000 events)
if [ -f "$SAMPLES_DIR/medium.wasm" ]; then
    echo "✅ Medium cassette created successfully!"
    echo "📦 Creating benchmark-large.wasm (5000 events)..."
    nak req -k 1 -l 5000 --since "7 days ago" \
        wss://relay.damus.io \
        wss://nos.lol \
        wss://relay.nostr.band | \
        cassette record -o "$SAMPLES_DIR" -n "benchmark-large" -d "Large benchmark cassette with 5000 events"
fi

echo ""
echo "📊 Cassette files created:"
ls -lh "$SAMPLES_DIR"/*.wasm 2>/dev/null || echo "  ❌ No .wasm files created"

echo ""
echo "📝 Checking cassette contents:"
for f in "$SAMPLES_DIR"/*.wasm; do
    if [ -f "$f" ]; then
        echo "  $(basename "$f"):"
        # Try to get basic info about the cassette
        cassette req "$f" '{"limit":1}' 2>&1 | head -5 || echo "    Could not read cassette"
    fi
done