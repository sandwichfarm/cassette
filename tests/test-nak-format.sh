#!/bin/bash
# Test nak output format

echo "Testing nak output format..."
echo ""
echo "First 3 events from relay:"
nak req --limit 3 wss://relay.damus.io 2>/dev/null | head -3

echo ""
echo "Checking if it's NDJSON (newline-delimited JSON):"
nak req --limit 3 wss://relay.damus.io 2>/dev/null | head -3 | while read line; do
    echo -n "Line is valid JSON: "
    echo "$line" | jq -e . >/dev/null 2>&1 && echo "YES" || echo "NO"
done