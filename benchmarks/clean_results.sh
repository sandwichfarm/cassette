#!/bin/bash

# Clean benchmark results script
# Extracts only performance metrics from benchmark outputs

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
RESULTS_DIR="$SCRIPT_DIR/results"

# Function to extract Python benchmark results
extract_python_results() {
    local input_file=$1
    local output_file=$2
    
    # Extract only the performance comparison table
    awk '/CASSETTE PERFORMANCE COMPARISON/,/Event retrieval stats/' "$input_file" | \
    grep -E "^(Filter Type|empty|limit_|kinds_|author|since|until|time_range|tag_|complex|SUMMARY STATISTICS|Cassette|small\.wasm|medium\.wasm|large\.wasm|[0-9]+\.|REQ QUERY PERFORMANCE)" | \
    grep -v "Event retrieval stats" > "$output_file"
}

# Function to extract Rust benchmark results
extract_rust_results() {
    local input_file=$1
    local output_file=$2
    
    # Extract only the benchmark results table
    awk '/Benchmark Results/,/^$/' "$input_file" | \
    grep -E "^(Cassette|small\.wasm|medium\.wasm|large\.wasm|empty|limit_|kinds_|author|since|until|time_range|tag_|complex|╔|║|╚|═|─|[0-9]+\.)" > "$output_file"
}

# Function to extract generic benchmark results
extract_generic_results() {
    local input_file=$1
    local output_file=$2
    
    # Extract lines that look like benchmark results (times, throughput, etc)
    grep -E "(ms|milliseconds|seconds|ops/sec|events/sec|throughput|latency|p50|p95|p99|average|median)" "$input_file" | \
    grep -v -E "(Installing|Downloading|Building|Compiling|Warning|Error|mkdir|cd |make)" > "$output_file"
}

# Clean Python results
if ls "$RESULTS_DIR"/py_output_*.log 1> /dev/null 2>&1; then
    echo "Cleaning Python results..."
    latest_py=$(ls -t "$RESULTS_DIR"/py_output_*.log | head -1)
    extract_python_results "$latest_py" "$RESULTS_DIR/py.txt"
    echo "Created: $RESULTS_DIR/py.txt"
fi

# Clean Rust results
if ls "$RESULTS_DIR"/rust_output_*.log 1> /dev/null 2>&1; then
    echo "Cleaning Rust results..."
    latest_rust=$(ls -t "$RESULTS_DIR"/rust_output_*.log | head -1)
    extract_rust_results "$latest_rust" "$RESULTS_DIR/rust.txt"
    echo "Created: $RESULTS_DIR/rust.txt"
fi

# Clean other language results
for lang in go js cpp dart deck; do
    if ls "$RESULTS_DIR"/${lang}_output_*.log 1> /dev/null 2>&1; then
        echo "Cleaning $lang results..."
        latest=$(ls -t "$RESULTS_DIR"/${lang}_output_*.log | head -1)
        extract_generic_results "$latest" "$RESULTS_DIR/${lang}.txt"
        echo "Created: $RESULTS_DIR/${lang}.txt"
    fi
done

echo "Done! Clean results are in $RESULTS_DIR/"