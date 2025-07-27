#!/bin/bash

# Generate sample cassette files for benchmarking
# Creates multiple cassettes of various sizes with UNIQUE events

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SAMPLES_DIR="$SCRIPT_DIR/../samples"

# Create samples directory if it doesn't exist
mkdir -p "$SAMPLES_DIR"

echo "🔨 Generating sample cassettes for benchmarking..."
echo ""

# Track timestamps to avoid duplicates using a file
TIMESTAMP_FILE="$SAMPLES_DIR/.last_timestamp"
CURRENT_TIMESTAMP=$(date +%s)
echo "$CURRENT_TIMESTAMP" > "$TIMESTAMP_FILE"

# Function to get the last timestamp
get_last_timestamp() {
    cat "$TIMESTAMP_FILE" 2>/dev/null || echo "$CURRENT_TIMESTAMP"
}

# Function to process events and update timestamp
process_and_record() {
    local name="$1"
    local description="$2"
    local flags="$3"
    
    # Save events to temp file
    local temp_file=$(mktemp)
    cat > "$temp_file"
    
    # Extract oldest timestamp from events
    local oldest=$(cat "$temp_file" | jq -r '.created_at' 2>/dev/null | sort -n | head -1)
    
    # Update timestamp file if we found an older one
    if [ -n "$oldest" ]; then
        local current=$(get_last_timestamp)
        if [ "$oldest" -lt "$current" ]; then
            echo "$((oldest - 1))" > "$TIMESTAMP_FILE"
            echo "  Next query will use --until $((oldest - 1))" >&2
        fi
    fi
    
    # Pass events to cassette record
    cat "$temp_file" | cassette record -o "$SAMPLES_DIR" -n "$name" --relay-description "$description" $flags
    rm -f "$temp_file"
}

# Generate base cassettes for benchmarks
echo "📦 Creating base cassettes for benchmarks..."

# Small cassette (100 events)
UNTIL_TS=$(get_last_timestamp)
echo "  Creating small.wasm (using --until $UNTIL_TS)"
nak req -k 1 -l 100 --until $UNTIL_TS \
    wss://relay.damus.io \
    wss://nos.lol | \
    process_and_record "small" "Small benchmark cassette" ""
sleep 0.5

# Medium cassette (1,000 events)
UNTIL_TS=$(get_last_timestamp)
echo "  Creating medium.wasm (using --until $UNTIL_TS)"
nak req -k 1 -l 1000 --until $UNTIL_TS \
    wss://relay.damus.io \
    wss://nos.lol \
    wss://relay.nostr.band | \
    process_and_record "medium" "Medium benchmark cassette" ""
sleep 0.5

# Large cassette (5,000 events)
UNTIL_TS=$(get_last_timestamp)
echo "  Creating large.wasm (using --until $UNTIL_TS)"
nak req -k 1 -l 5000 --until $UNTIL_TS \
    wss://relay.damus.io \
    wss://nos.lol \
    wss://relay.nostr.band \
    wss://relay.snort.social | \
    process_and_record "large" "Large benchmark cassette" ""

# Generate additional numbered cassettes for variety
echo ""
echo "📦 Creating additional cassettes for testing variety..."

# Generate small cassettes (100 events each) - 3 additional files
for i in {1..3}; do
    UNTIL_TS=$(get_last_timestamp)
    echo "  Creating small_${i}.wasm (using --until $UNTIL_TS)"
    nak req -k 1 -l 100 --until $UNTIL_TS \
        wss://relay.damus.io \
        wss://nos.lol | \
        process_and_record "small_${i}" "Small benchmark cassette #${i}" ""
    sleep 0.5
done

# Generate medium cassettes (1,000 events each) - 3 additional files
for i in {1..3}; do
    UNTIL_TS=$(get_last_timestamp)
    echo "  Creating medium_${i}.wasm (using --until $UNTIL_TS)"
    nak req -k 1 -l 1000 --until $UNTIL_TS \
        wss://relay.damus.io \
        wss://nos.lol \
        wss://relay.nostr.band | \
        process_and_record "medium_${i}" "Medium benchmark cassette #${i}" ""
    sleep 0.5
done

# Generate large cassettes (5,000 events each) - 3 additional files
for i in {1..3}; do
    UNTIL_TS=$(get_last_timestamp)
    echo "  Creating large_${i}.wasm (using --until $UNTIL_TS)"
    nak req -k 1 -l 5000 --until $UNTIL_TS \
        wss://relay.damus.io \
        wss://nos.lol \
        wss://relay.nostr.band \
        wss://relay.snort.social | \
        process_and_record "large_${i}" "Large benchmark cassette #${i}" ""
    sleep 0.5
done

echo ""
echo "✅ Sample cassettes generated successfully!"
echo ""
echo "📊 Cassette files:"
ls -lh "$SAMPLES_DIR"/*.wasm 2>/dev/null || echo "No cassettes found"

echo ""
echo "📈 Event counts:"
for f in "$SAMPLES_DIR"/*.wasm; do
    if [ -f "$f" ]; then
        # Using cassette req to get a count
        count=$(cassette req "$f" '{"limit":0}' 2>/dev/null | grep -c '"EVENT"' || echo "?")
        echo "  $(basename "$f"): ~$count events"
    fi
done

echo ""
echo "💡 All cassettes contain unique events!"
echo "   Each cassette queries progressively older events using --until"

# Clean up timestamp file
rm -f "$TIMESTAMP_FILE"