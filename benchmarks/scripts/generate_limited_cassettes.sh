#!/bin/bash

# Generate sample cassettes with size limits for benchmarking

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SAMPLES_DIR="$SCRIPT_DIR/../samples"

# Create samples directory if it doesn't exist
mkdir -p "$SAMPLES_DIR"

echo "🔨 Generating size-limited sample cassettes for benchmarking..."

# Function to limit file size
limit_size() {
    local max_bytes=$1
    local bytes_read=0
    local line
    
    while IFS= read -r line; do
        bytes_read=$((bytes_read + ${#line} + 1))  # +1 for newline
        if [ $bytes_read -gt $max_bytes ]; then
            echo >&2 "Size limit reached at $bytes_read bytes"
            break
        fi
        echo "$line"
    done
}

# Generate small cassette (up to 100KB)
echo "📦 Creating small.wasm (up to 100KB)..."
nak req -k 1 -l 200 \
    wss://relay.damus.io \
    wss://nos.lol | \
    limit_size 102400 | \
    cassette record -o "$SAMPLES_DIR" -n "small" -d "Small benchmark cassette (~100KB)"

# Generate medium cassette (up to 1MB)
echo "📦 Creating medium.wasm (up to 1MB)..."
nak req -k 1 -l 2000 --since "3 days ago" \
    wss://relay.damus.io \
    wss://nos.lol \
    wss://relay.nostr.band | \
    limit_size 1048576 | \
    cassette record -o "$SAMPLES_DIR" -n "medium" -d "Medium benchmark cassette (~1MB)"

# Generate large cassette (up to 20MB as user requested)
echo "📦 Creating benchmark-large.wasm (up to 20MB)..."
echo "This will take a while..."

# Collect events up to 20MB
TEMP_FILE="$SAMPLES_DIR/temp_events.jsonl"
rm -f "$TEMP_FILE"

# Function to append events up to size limit
append_until_size() {
    local target_file=$1
    local max_size=$2
    local current_size=$(stat -f%z "$target_file" 2>/dev/null || echo 0)
    
    while IFS= read -r line; do
        current_size=$((current_size + ${#line} + 1))
        if [ $current_size -gt $max_size ]; then
            echo >&2 "Target size reached: $current_size bytes"
            break
        fi
        echo "$line" >> "$target_file"
    done
}

# Start with kind 1 (text notes)
echo "  Collecting text notes..."
nak req -k 1 -l 50000 --since "30 days ago" \
    wss://relay.damus.io \
    wss://nos.lol \
    wss://relay.nostr.band | \
    append_until_size "$TEMP_FILE" 20971520  # 20MB limit

# Check if we need more data
CURRENT_SIZE=$(stat -f%z "$TEMP_FILE" 2>/dev/null || echo 0)
if [ $CURRENT_SIZE -lt 20971520 ]; then
    echo "  Adding profile metadata..."
    nak req -k 0 -l 10000 --since "30 days ago" \
        wss://relay.damus.io \
        wss://purplepag.es | \
        append_until_size "$TEMP_FILE" 20971520
fi

# Check again
CURRENT_SIZE=$(stat -f%z "$TEMP_FILE" 2>/dev/null || echo 0)
if [ $CURRENT_SIZE -lt 20971520 ]; then
    echo "  Adding reactions..."
    nak req -k 7 -l 20000 --since "7 days ago" \
        wss://relay.damus.io \
        wss://nos.lol | \
        append_until_size "$TEMP_FILE" 20971520
fi

echo "  Creating cassette from collected events..."
cat "$TEMP_FILE" | cassette record -o "$SAMPLES_DIR" -n "benchmark-large" -d "Large benchmark cassette (~20MB)"

# Clean up
rm -f "$TEMP_FILE"

echo ""
echo "✅ Sample cassettes generated successfully!"
echo ""
echo "📊 Cassette sizes:"
ls -lh "$SAMPLES_DIR"/*.wasm 2>/dev/null || echo "  No .wasm files found yet"

echo ""
echo "📈 Event counts (approximate):"
for f in "$SAMPLES_DIR"/*.wasm; do
    if [ -f "$f" ]; then
        # Count events in the temp file used to create it
        echo "  $(basename "$f"): created with size limits"
    fi
done