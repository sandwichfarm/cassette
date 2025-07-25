#!/bin/bash

echo "Testing deck cassette generation..."

# Clean up
rm -rf test_deck
mkdir -p test_deck

# Start deck with verbose output
echo "Starting deck..."
./cli/target/debug/cassette deck -o test_deck -e 1 -v -p 9999 2>&1 | tee deck_test.log &
DECK_PID=$!

sleep 2

echo ""
echo "Sending a simple test event..."
echo '{"content":"Test event for debugging","created_at":1234567890,"kind":1,"tags":[],"pubkey":"test","id":"test123","sig":"test"}' | \
  websocat -t ws://localhost:9999

sleep 5

echo ""
echo "Checking for cassettes..."
ls -la test_deck/

echo ""
echo "Checking deck log for errors..."
grep -E "(❌|Error|Failed)" deck_test.log || echo "No errors found in log"

# Clean up
kill $DECK_PID 2>/dev/null