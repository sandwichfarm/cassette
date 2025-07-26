#!/bin/bash

# Run direct cassette WASM benchmarks

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🚀 Direct Cassette WASM Benchmark"
echo "================================="

# Set up virtual environment if needed
VENV_DIR="venv"
if [ ! -d "$VENV_DIR" ]; then
    echo "📦 Creating Python virtual environment..."
    python3 -m venv "$VENV_DIR"
fi

# Activate virtual environment
source "$VENV_DIR/bin/activate"

# Check if wasmtime is installed
if ! python -c "import wasmtime" &> /dev/null; then
    echo "📦 Installing wasmtime module..."
    pip install wasmtime
fi

# Create some test cassettes if they don't exist
echo "🔍 Checking for test cassettes..."

# Small cassette (100 events)
if [ ! -f "data/small.wasm" ]; then
    echo "📼 Creating small cassette (100 events)..."
    timeout 5 nak req -k 1 -l 100 wss://relay.damus.io > data/small_events.jsonl 2>/dev/null || true
    ../cli/target/release/cassette record data/small_events.jsonl -n small -o data
    rm -f data/small_events.jsonl
fi

# Medium cassette (1000 events)
if [ ! -f "data/medium.wasm" ]; then
    echo "📼 Creating medium cassette (1000 events)..."
    timeout 10 nak req -k 1 -l 1000 wss://relay.damus.io > data/medium_events.jsonl 2>/dev/null || true
    ../cli/target/release/cassette record data/medium_events.jsonl -n medium -o data
    rm -f data/medium_events.jsonl
fi

# Check if large cassette exists
if [ ! -f "data/benchmark-large.wasm" ]; then
    echo "📼 Large cassette not found. Run ./scripts/generate_large_cassette.sh to create it."
fi

# Collect all cassettes
CASSETTES=""
for cassette in data/*.wasm; do
    if [ -f "$cassette" ]; then
        CASSETTES="$CASSETTES $cassette"
    fi
done

if [ -z "$CASSETTES" ]; then
    echo "❌ No cassettes found in data/ directory"
    exit 1
fi

echo "📊 Found cassettes:$CASSETTES"
echo ""

# Run the benchmark
TIMESTAMP=$(date +%s)
OUTPUT_FILE="results/cassette_benchmark_${TIMESTAMP}.json"

echo "🏃 Running benchmarks..."
python scripts/cassette_bench_proper.py $CASSETTES --iterations 100 --output "$OUTPUT_FILE"

echo ""
echo "✅ Benchmark complete!"
echo "📊 Results saved to: $OUTPUT_FILE"

# Optional: Run with more iterations for more accurate results
echo ""
echo "💡 For more detailed results, run with more iterations:"
echo "   python scripts/cassette_bench_proper.py data/*.wasm --iterations 1000"