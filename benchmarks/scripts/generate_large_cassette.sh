#!/bin/bash

# Generate a large cassette with 10k+ events for benchmarking

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="$SCRIPT_DIR/../data"
CASSETTE_BIN="$SCRIPT_DIR/../../cli/target/release/cassette"

mkdir -p "$DATA_DIR"

echo "🎬 Generating large cassette for benchmarking..."

# Create a temporary directory for event batches
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Relay list for fetching events
RELAYS=(
    "wss://relay.damus.io"
    "wss://relay.primal.net"
    "wss://purplepag.es"
    "wss://relay.nostr.band"
)

# Fetch events in batches
echo "📥 Fetching events from multiple relays..."

TOTAL_EVENTS=0
BATCH_NUM=0

# Fetch kind 1 events (text notes)
for relay in "${RELAYS[@]}"; do
    echo "  Fetching from $relay..."
    
    # Fetch recent events
    timeout 10 nak req -k 1 -l 2000 "$relay" > "$TEMP_DIR/batch_${BATCH_NUM}_recent.jsonl" 2>/dev/null || true
    COUNT=$(wc -l < "$TEMP_DIR/batch_${BATCH_NUM}_recent.jsonl")
    echo "    Got $COUNT recent events"
    TOTAL_EVENTS=$((TOTAL_EVENTS + COUNT))
    BATCH_NUM=$((BATCH_NUM + 1))
    
    # Fetch older events with until filter
    UNTIL=$(($(date +%s) - 86400)) # 24 hours ago
    timeout 10 nak req -k 1 -l 2000 --until "$UNTIL" "$relay" > "$TEMP_DIR/batch_${BATCH_NUM}_older.jsonl" 2>/dev/null || true
    COUNT=$(wc -l < "$TEMP_DIR/batch_${BATCH_NUM}_older.jsonl")
    echo "    Got $COUNT older events"
    TOTAL_EVENTS=$((TOTAL_EVENTS + COUNT))
    BATCH_NUM=$((BATCH_NUM + 1))
    
    # Stop if we have enough events
    if [ $TOTAL_EVENTS -ge 10000 ]; then
        echo "✅ Collected enough events ($TOTAL_EVENTS)"
        break
    fi
done

# Also fetch some other kinds for variety
echo "  Fetching reaction events..."
timeout 10 nak req -k 7 -l 1000 wss://relay.damus.io > "$TEMP_DIR/batch_reactions.jsonl" 2>/dev/null || true
COUNT=$(wc -l < "$TEMP_DIR/batch_reactions.jsonl")
TOTAL_EVENTS=$((TOTAL_EVENTS + COUNT))

echo "  Fetching profile events..."
timeout 10 nak req -k 0 -l 500 wss://relay.damus.io > "$TEMP_DIR/batch_profiles.jsonl" 2>/dev/null || true
COUNT=$(wc -l < "$TEMP_DIR/batch_profiles.jsonl")
TOTAL_EVENTS=$((TOTAL_EVENTS + COUNT))

# Combine all batches
echo "📦 Combining $TOTAL_EVENTS events..."
cat "$TEMP_DIR"/*.jsonl > "$DATA_DIR/all_events.jsonl"

# Remove duplicates
echo "🧹 Removing duplicates..."
sort -u "$DATA_DIR/all_events.jsonl" > "$DATA_DIR/unique_events.jsonl"
UNIQUE_COUNT=$(wc -l < "$DATA_DIR/unique_events.jsonl")
echo "  Unique events: $UNIQUE_COUNT"

# Record into a cassette
echo "📼 Recording cassette..."
"$CASSETTE_BIN" record "$DATA_DIR/unique_events.jsonl" -n benchmark-large -o "$DATA_DIR"

# Clean up temporary files
rm -f "$DATA_DIR/all_events.jsonl" "$DATA_DIR/unique_events.jsonl"

echo "✅ Large cassette created: $DATA_DIR/benchmark-large.wasm"
echo "   Total events: $UNIQUE_COUNT"

# Verify the cassette
echo "🔍 Verifying cassette..."
EVENT_COUNT=$("$CASSETTE_BIN" scrub "$DATA_DIR/benchmark-large.wasm" 2>/dev/null | grep -c '"EVENT"' || echo "0")
echo "   Verified events in cassette: $EVENT_COUNT"