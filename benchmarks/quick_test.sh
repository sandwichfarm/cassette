#!/bin/bash

# Quick benchmark test (10 seconds, for rapid iteration)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🚀 Quick Benchmark Test (10 seconds)"
echo "===================================="

# Use existing venv if available
if [ -d "venv" ]; then
    source venv/bin/activate
else
    echo "Run ./run_benchmark.sh first to set up the environment"
    exit 1
fi

# Check if test cassette exists, if not create a small one
if [ ! -f "data/test-small.wasm" ]; then
    echo "📼 Creating small test cassette..."
    mkdir -p data
    timeout 5 nak req -k 1 -l 100 wss://relay.damus.io > data/test_events.jsonl || true
    ../cli/target/release/cassette record data/test_events.jsonl -n test-small -o data
    rm data/test_events.jsonl
fi

# Kill any existing deck
pkill -f "cassette deck" || true
sleep 1

# Start deck with small cassette
echo "🚀 Starting deck..."
../cli/target/release/cassette deck -o data -p 9999 > /dev/null 2>&1 &
DECK_PID=$!
sleep 2

# Run quick benchmark
echo "🏃 Running quick test..."
python scripts/deck_benchmark.py --duration 10 --rps 50 --connections 5

# Cleanup
kill $DECK_PID 2>/dev/null || true

echo "✅ Quick test complete!"