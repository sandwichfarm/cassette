#!/bin/bash

# Run the complete cassette deck benchmark suite

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🎯 Cassette Deck Benchmark Suite"
echo "================================"

# Check if Python 3 is available
if ! command -v python3 &> /dev/null; then
    echo "❌ Python 3 is required but not installed"
    exit 1
fi

# Set up virtual environment if needed
VENV_DIR="venv"
if [ ! -d "$VENV_DIR" ]; then
    echo "📦 Creating Python virtual environment..."
    python3 -m venv "$VENV_DIR"
fi

# Activate virtual environment
source "$VENV_DIR/bin/activate"

# Check if websockets module is installed
if ! python -c "import websockets" &> /dev/null; then
    echo "📦 Installing websockets module..."
    pip install websockets
fi

# Check if cassette binary exists
CASSETTE_BIN="../cli/target/release/cassette"
if [ ! -f "$CASSETTE_BIN" ]; then
    echo "🔨 Building cassette..."
    (cd ../cli && cargo build --release)
fi

# Create results directory
mkdir -p results

# Check if large cassette exists
if [ ! -f "data/benchmark-large.wasm" ]; then
    echo "📼 Generating large cassette (this may take a few minutes)..."
    ./scripts/generate_large_cassette.sh
    
    if [ ! -f "data/benchmark-large.wasm" ]; then
        echo "❌ Failed to generate large cassette"
        exit 1
    fi
fi

# Kill any existing deck process
echo "🧹 Cleaning up existing deck processes..."
pkill -f "cassette deck" || true
sleep 1

# Start the deck with the large cassette
echo "🚀 Starting cassette deck with large dataset..."
DECK_LOG="results/deck_benchmark_$(date +%s).log"
"$CASSETTE_BIN" deck -o data -p 9999 -v > "$DECK_LOG" 2>&1 &
DECK_PID=$!
echo "   Deck PID: $DECK_PID"

# Wait for deck to start
echo "⏳ Waiting for deck to start..."
sleep 3

# Verify deck is running
if ! kill -0 $DECK_PID 2>/dev/null; then
    echo "❌ Deck failed to start. Check log: $DECK_LOG"
    tail -20 "$DECK_LOG"
    exit 1
fi

# Run the benchmark
echo "🏃 Running benchmark..."
echo ""

# Default benchmark (1 minute, moderate load)
python scripts/deck_benchmark.py

# You can also run with custom parameters:
# python scripts/deck_benchmark.py --duration 300 --rps 500 --connections 50

# Kill the deck
echo ""
echo "🛑 Stopping deck..."
kill $DECK_PID 2>/dev/null || true

echo "✅ Benchmark complete!"
echo ""
echo "📊 Results saved in: results/"
echo "📜 Deck log saved in: $DECK_LOG"

# Show performance metrics from deck log
echo ""
echo "📈 Deck Performance Metrics:"
grep -E "(⏱️|Performance)" "$DECK_LOG" | tail -20 || echo "No performance metrics found"