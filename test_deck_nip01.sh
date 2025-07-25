#!/bin/bash
# Test deck relay NIP-01 compliance

echo "Testing deck relay NIP-01 implementation..."

# Start deck in background
echo "Starting deck relay..."
./cli/target/release/cassette deck -p 7778 -v -e 10 &
DECK_PID=$!
sleep 2

# Create some test events
echo "Sending test events..."
echo "First event" | nak event -c - | nak publish ws://127.0.0.1:7778
echo "Second event" | nak event -c - | nak publish ws://127.0.0.1:7778
echo "Third event" | nak event -c - --kind 0 | nak publish ws://127.0.0.1:7778
echo "Fourth event - replaceable" | nak event -c - --kind 0 | nak publish ws://127.0.0.1:7778
sleep 1

# Query with limit
echo -e "\nQuerying with limit 2..."
nak req -l 2 ws://127.0.0.1:7778

# Query kind 0 (should only see the latest)
echo -e "\nQuerying kind 0 events (replaceable)..."
nak req -k 0 ws://127.0.0.1:7778

# Clean up
kill $DECK_PID
wait $DECK_PID 2>/dev/null

echo -e "\nTest complete!"