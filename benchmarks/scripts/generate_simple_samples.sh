#!/bin/bash

# Generate simple sample cassette files for benchmarking
# Uses smaller, cleaner datasets to avoid Unicode issues

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SAMPLES_DIR="$SCRIPT_DIR/../samples"

# Create samples directory if it doesn't exist
mkdir -p "$SAMPLES_DIR"

echo "🔨 Generating sample cassettes for benchmarking..."

# Generate small cassette (100 events) - English content only
echo "📦 Creating small.wasm (100 events)..."
nak req -k 1 -l 100 -a 82341f882b6eabcd2ba7f1ef90aad961cf074af15b9ef44a09f9d2a8fbfbe6a2 -a 32e1827635450ebb3c5a7d12c1f8e7b2b514439ac10a67eef3d9fd9c5c68e245 -a 3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459d wss://relay.damus.io | \
    head -n 100 | \
    cassette record -o "$SAMPLES_DIR" -n "small" -d "Small benchmark cassette with 100 events"

# Generate medium cassette (1,000 events) - Popular English-speaking authors
echo "📦 Creating medium.wasm (1,000 events)..."
nak req -k 1,0,7 -l 1000 --since "7 days ago" \
    -a 82341f882b6eabcd2ba7f1ef90aad961cf074af15b9ef44a09f9d2a8fbfbe6a2 \
    -a 32e1827635450ebb3c5a7d12c1f8e7b2b514439ac10a67eef3d9fd9c5c68e245 \
    -a 3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459d \
    -a c48e29f04b482cc01ca1f9ef8c86ef8318c059e0e9353235162f080f26e14c11 \
    -a 83e818dfbeccea56b0f551576b3fd39a7a50e1d8159343500368fa085ccd964b \
    wss://relay.damus.io \
    wss://nos.lol | \
    head -n 1000 | \
    cassette record -o "$SAMPLES_DIR" -n "medium" -d "Medium benchmark cassette with 1k events"

# Generate large cassette (10,000 events) - Mix of different event kinds
echo "📦 Creating benchmark-large.wasm (10,000 events)..."
echo "This will take a while..."

# Create a temporary file to collect events
TEMP_FILE="$SAMPLES_DIR/temp_events.jsonl"
rm -f "$TEMP_FILE"

# Collect text notes (kind 1) - 5000 events
echo "  Collecting text notes..."
nak req -k 1 -l 5000 --since "14 days ago" \
    wss://relay.damus.io \
    wss://nos.lol \
    wss://relay.nostr.band >> "$TEMP_FILE"

# Collect profile metadata (kind 0) - 2000 events  
echo "  Collecting profile metadata..."
nak req -k 0 -l 2000 --since "30 days ago" \
    wss://relay.damus.io \
    wss://purplepag.es >> "$TEMP_FILE"

# Collect reactions (kind 7) - 2000 events
echo "  Collecting reactions..."
nak req -k 7 -l 2000 --since "3 days ago" \
    wss://relay.damus.io \
    wss://nos.lol >> "$TEMP_FILE"

# Collect reposts (kind 6) - 1000 events
echo "  Collecting reposts..."
nak req -k 6 -l 1000 --since "7 days ago" \
    wss://relay.damus.io >> "$TEMP_FILE"

# Filter out any problematic events and create the cassette
echo "  Processing and creating cassette..."
cat "$TEMP_FILE" | \
    grep -E '^{.*}$' | \
    jq -c 'select(.content != null)' | \
    head -n 10000 | \
    cassette record -o "$SAMPLES_DIR" -n "benchmark-large" -d "Large benchmark cassette with 10k mixed events"

# Clean up
rm -f "$TEMP_FILE"

echo ""
echo "✅ Sample cassettes generated successfully!"
echo ""
echo "📊 Cassette sizes:"
ls -lh "$SAMPLES_DIR"/*.wasm 2>/dev/null || echo "  No .wasm files found yet"

# If generation failed, create fallback minimal samples
if [ ! -f "$SAMPLES_DIR/small.wasm" ]; then
    echo ""
    echo "⚠️  Some cassettes failed to generate. Creating minimal fallback samples..."
    
    # Create minimal JSON events
    echo '[{"id":"test1","pubkey":"test","created_at":1234567890,"kind":1,"tags":[],"content":"Test event 1","sig":"test"}]' | \
        cassette record -o "$SAMPLES_DIR" -n "small" -d "Minimal test cassette"
fi