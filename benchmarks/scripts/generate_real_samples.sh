#!/bin/bash

# Generate real cassette samples from live relays

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SAMPLES_DIR="$SCRIPT_DIR/../samples"
TEMP_DIR="$SAMPLES_DIR/temp"

# Create directories
mkdir -p "$SAMPLES_DIR" "$TEMP_DIR"

echo "🔨 Generating real cassette samples from live relay data..."

# Small cassette (100 recent text notes)
echo "📦 Creating small.wasm (100 events)..."
nak req -k 1 -l 100 \
    wss://relay.damus.io \
    wss://nos.lol > "$TEMP_DIR/small_events.jsonl"

cassette record -o "$SAMPLES_DIR" -n "small" -d "Small benchmark cassette with 100 real events" "$TEMP_DIR/small_events.jsonl"

# Medium cassette (1000 mixed events)
echo "📦 Creating medium.wasm (1000 events)..."
{
    # Text notes
    nak req -k 1 -l 600 --since "3 days ago" \
        wss://relay.damus.io \
        wss://nos.lol \
        wss://relay.nostr.band
    
    # Reactions
    nak req -k 7 -l 200 --since "1 day ago" \
        wss://relay.damus.io
    
    # Metadata
    nak req -k 0 -l 100 --since "7 days ago" \
        wss://purplepag.es
    
    # Reposts
    nak req -k 6 -l 100 --since "2 days ago" \
        wss://relay.damus.io
} > "$TEMP_DIR/medium_events.jsonl"

cassette record -n "medium" -d "Medium benchmark cassette with 1000 real events" "$TEMP_DIR/medium_events.jsonl"

# Large cassette (5000 mixed events) 
echo "📦 Creating benchmark-large.wasm (5000 events)..."
echo "This will take a while..."

{
    # Text notes from multiple relays
    echo "  Collecting text notes..."
    nak req -k 1 -l 3000 --since "7 days ago" \
        wss://relay.damus.io \
        wss://nos.lol \
        wss://relay.nostr.band \
        wss://relay.snort.social
    
    # Reactions
    echo "  Collecting reactions..."
    nak req -k 7 -l 1000 --since "2 days ago" \
        wss://relay.damus.io \
        wss://nos.lol
    
    # Metadata updates
    echo "  Collecting metadata..."
    nak req -k 0 -l 500 --since "14 days ago" \
        wss://purplepag.es \
        wss://relay.damus.io
    
    # Reposts and other kinds
    echo "  Collecting reposts..."
    nak req -k 6 -l 300 --since "3 days ago" \
        wss://relay.damus.io
    
    # Follow lists
    echo "  Collecting follow lists..."
    nak req -k 3 -l 200 --since "7 days ago" \
        wss://relay.damus.io
} > "$TEMP_DIR/large_events.jsonl"

cassette record -n "benchmark-large" -d "Large benchmark cassette with 5000 real events" "$TEMP_DIR/large_events.jsonl"

# Clean up temp files
rm -rf "$TEMP_DIR"

echo ""
echo "✅ Real cassette samples generated!"
echo ""
echo "📊 Cassette files:"
ls -lh "$SAMPLES_DIR"/*.wasm

echo ""
echo "📈 Testing cassettes..."
for f in "$SAMPLES_DIR"/*.wasm; do
    if [ -f "$f" ]; then
        echo -n "  $(basename "$f"): "
        # Send a simple REQ to check if it works
        if cassette req "$f" '{"limit":1}' > /dev/null 2>&1; then
            echo "✅ Valid"
        else
            echo "❌ Invalid"
        fi
    fi
done