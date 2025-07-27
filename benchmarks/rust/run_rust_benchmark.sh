#!/bin/bash

# Run Rust-based cassette WASM benchmarks

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🚀 Direct Cassette WASM Benchmark (Rust)"
echo "========================================"

# Build Rust benchmark if needed
if [ ! -f "target/release/cassette-bench" ]; then
    echo "📦 Building Rust benchmark..."
    cargo build --release
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
OUTPUT_FILE="results/cassette_rust_benchmark_${TIMESTAMP}.json"

echo "🏃 Running benchmarks..."
./target/release/cassette-bench $CASSETTES --iterations 100 --output "$OUTPUT_FILE"

echo ""
echo "✅ Benchmark complete!"
echo "📊 Results saved to: $OUTPUT_FILE"

# Optional: Run with more iterations for more accurate results
echo ""
echo "💡 For more detailed results, run with more iterations:"
echo "   ./target/release/cassette-bench data/*.wasm --iterations 1000"