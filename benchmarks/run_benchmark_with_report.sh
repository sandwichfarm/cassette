#!/bin/bash

# Run benchmark for a specific language and generate markdown report
# Usage: ./run_benchmark_with_report.sh <language>

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
OUTPUT_DIR="$SCRIPT_DIR/output"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Function to convert language output to markdown
convert_to_markdown() {
    local lang=$1
    local input_file=$2
    local output_file=$3
    
    echo "# Cassette Benchmark Report - $(echo $lang | tr '[:lower:]' '[:upper:]')" > "$output_file"
    echo "" >> "$output_file"
    echo "**Generated:** $(date)" >> "$output_file"
    echo "" >> "$output_file"
    echo "## Results" >> "$output_file"
    echo "" >> "$output_file"
    echo '```' >> "$output_file"
    cat "$input_file" >> "$output_file"
    echo '```' >> "$output_file"
}

# Function to run benchmark and generate report
run_benchmark() {
    local lang=$1
    local dir="$SCRIPT_DIR/$lang"
    
    if [ ! -d "$dir" ]; then
        echo "Error: Directory $dir not found"
        exit 1
    fi
    
    if [ ! -f "$dir/Makefile" ]; then
        echo "Error: No Makefile found in $dir"
        exit 1
    fi
    
    echo "🚀 Running $lang benchmark..."
    
    cd "$dir"
    
    # Run the benchmark and capture output
    TEMP_OUTPUT="/tmp/${lang}_benchmark_${TIMESTAMP}.log"
    
    # Run benchmark with proper output capture
    case "$lang" in
        "rust")
            # Rust benchmark already produces nice formatted output
            cargo build --release >/dev/null 2>&1
            cargo run --release -- ../samples/small.wasm ../samples/medium.wasm ../samples/large.wasm --iterations 100 2>&1 | tee "$TEMP_OUTPUT"
            ;;
        "py")
            # Python benchmark
            make run 2>&1 | tee "$TEMP_OUTPUT"
            ;;
        "go")
            # Go benchmark
            make run 2>&1 | tee "$TEMP_OUTPUT"
            ;;
        "js")
            # JavaScript benchmark
            make run 2>&1 | tee "$TEMP_OUTPUT"
            ;;
        "cpp")
            # C++ benchmark
            make run 2>&1 | tee "$TEMP_OUTPUT"
            ;;
        "dart")
            # Dart benchmark
            make run 2>&1 | tee "$TEMP_OUTPUT"
            ;;
        "deck")
            # Deck benchmark
            make run 2>&1 | tee "$TEMP_OUTPUT"
            ;;
        *)
            echo "Unknown language: $lang"
            exit 1
            ;;
    esac
    
    # Convert to markdown report
    OUTPUT_FILE="$OUTPUT_DIR/${lang}.md"
    convert_to_markdown "$lang" "$TEMP_OUTPUT" "$OUTPUT_FILE"
    
    # Clean up
    rm -f "$TEMP_OUTPUT"
    
    echo "✅ Report saved to: $OUTPUT_FILE"
}

# Main execution
if [ $# -eq 0 ]; then
    echo "Usage: $0 <language>"
    echo "Available languages: py, rust, go, js, cpp, dart, deck"
    exit 1
fi

run_benchmark "$1"