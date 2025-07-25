#!/bin/bash

echo "Testing deck NIP-01 message validation..."

# Clean up
rm -rf test_deck
mkdir -p test_deck

# Start deck with verbose output
echo "Starting deck..."
./target/debug/cassette deck -o test_deck -e 10 -v -p 9999 > deck_validation.log 2>&1 &
DECK_PID=$!

sleep 2

echo -e "\n=== Testing malformed messages ==="

echo -e "\n1. Invalid JSON:"
echo 'not valid json' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n2. Not an array:"
echo '{"message": "hello"}' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n3. Empty array:"
echo '[]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n4. First element not a string:"
echo '[123, "data"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n5. Unknown command:"
echo '["UNKNOWN", "test"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n6. Invalid EVENT (wrong number of elements):"
echo '["EVENT"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"
echo '["EVENT", "extra", "element"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n7. Invalid EVENT (second element not object):"
echo '["EVENT", "not an object"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n8. Invalid REQ (too few elements):"
echo '["REQ"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"
echo '["REQ", "sub1"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n9. Invalid REQ (subscription ID not string):"
echo '["REQ", 123, {}]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n10. Invalid REQ (filter not object):"
echo '["REQ", "sub1", "not a filter"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n11. Invalid CLOSE (wrong elements):"
echo '["CLOSE"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"
echo '["CLOSE", "sub1", "extra"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n12. Invalid COUNT (too few elements):"
echo '["COUNT", "sub1"]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(NOTICE|notice)"

echo -e "\n=== Testing valid messages ==="

echo -e "\n13. Valid EVENT:"
nak event -c "Test valid event" -k 1 --envelope | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(OK|ok)"

echo -e "\n14. Valid REQ:"
echo '["REQ", "sub1", {"kinds": [1], "limit": 1}]' | websocat -t -1 ws://localhost:9999 2>&1 | head -5

echo -e "\n15. Valid CLOSE:"
echo '["CLOSE", "sub1"]' | websocat -t -1 ws://localhost:9999 2>&1

echo -e "\n16. Valid COUNT:"
echo '["COUNT", "sub1", {"kinds": [1]}]' | websocat -t -1 ws://localhost:9999 2>&1 | grep -E "(COUNT|count)"

echo -e "\nCheck deck_validation.log for verbose output"

# Clean up
kill $DECK_PID 2>/dev/null