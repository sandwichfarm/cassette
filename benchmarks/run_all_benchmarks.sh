#!/bin/bash

# Cassette Benchmarks Runner
# Runs benchmarks for all supported language bindings

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
RESULTS_DIR="$SCRIPT_DIR/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
SUMMARY_FILE="$RESULTS_DIR/benchmark_summary_$TIMESTAMP.txt"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create results directory if it doesn't exist
mkdir -p "$RESULTS_DIR"

# Clean up old log files and summaries
rm -f "$RESULTS_DIR"/*_output_*.log
rm -f "$RESULTS_DIR"/benchmark_summary_*.txt

# Function to print colored output
print_status() {
    echo -e "${2}${1}${NC}"
}

# Function to run a language benchmark
run_benchmark() {
    local lang=$1
    local dir="$SCRIPT_DIR/$lang"
    
    if [ ! -d "$dir" ]; then
        print_status "⚠️  Skipping $lang - directory not found" "$YELLOW"
        return 1
    fi
    
    if [ ! -f "$dir/Makefile" ]; then
        print_status "⚠️  Skipping $lang - no Makefile found" "$YELLOW"
        return 1
    fi
    
    print_status "🚀 Running $lang benchmark..." "$BLUE"
    
    cd "$dir"
    
    # Run the benchmark
    TEMP_LOG="$RESULTS_DIR/${lang}_output_$TIMESTAMP.log"
    if make run > "$TEMP_LOG" 2>&1; then
        print_status "✅ $lang benchmark completed successfully" "$GREEN"
        
        # Extract clean results based on language
        case "$lang" in
            "py")
                # Extract Python benchmark table
                awk '/CASSETTE PERFORMANCE COMPARISON/,/^$/' "$TEMP_LOG" | \
                grep -E "^(Filter Type|empty|limit_|kinds_|author|since|until|time_range|tag_|complex|SUMMARY STATISTICS|Cassette|.*\.wasm|[0-9]+\.|REQ QUERY PERFORMANCE|─|═)" \
                > "$RESULTS_DIR/${lang}.txt"
                ;;
            "rust")
                # Extract Rust benchmark table
                awk '/Benchmark Results/,/^$/' "$TEMP_LOG" | \
                grep -E "^(Cassette|.*\.wasm|empty|limit_|kinds_|author|since|until|time_range|tag_|complex|╔|║|╚|═|─|[0-9]+\.)" \
                > "$RESULTS_DIR/${lang}.txt"
                ;;
            *)
                # For other languages, extract performance metrics
                grep -E "(ms|milliseconds|seconds|ops/sec|events/sec|throughput|latency|p50|p95|p99|average|median|benchmark|performance)" "$TEMP_LOG" | \
                grep -v -E "(Installing|Downloading|Building|Compiling|Warning|Error|mkdir|cd |make)" \
                > "$RESULTS_DIR/${lang}.txt"
                ;;
        esac
        
        # Remove the log file
        rm -f "$TEMP_LOG"
        
        return 0
    else
        print_status "❌ $lang benchmark failed" "$RED"
        # Still save error output
        echo "Benchmark failed. Error output:" > "$RESULTS_DIR/${lang}.txt"
        tail -20 "$TEMP_LOG" >> "$RESULTS_DIR/${lang}.txt"
        rm -f "$TEMP_LOG"
        return 1
    fi
}

# Header
clear
echo "╔══════════════════════════════════════════════╗"
echo "║        Cassette Benchmark Suite              ║"
echo "║                                              ║"
echo "║  Running benchmarks for all language         ║"
echo "║  bindings...                                 ║"
echo "╚══════════════════════════════════════════════╝"
echo ""
echo "Timestamp: $(date)"
echo "Results will be saved to: $RESULTS_DIR"
echo ""

# Initialize summary
echo "Cassette Benchmark Summary - $TIMESTAMP" > "$SUMMARY_FILE"
echo "========================================" >> "$SUMMARY_FILE"
echo "" >> "$SUMMARY_FILE"

# Languages to benchmark
LANGUAGES=(py rust cpp dart go js)
SUCCESSFUL=0
FAILED=0

# Run benchmarks for each language
for lang in "${LANGUAGES[@]}"; do
    echo ""
    if run_benchmark "$lang"; then
        ((SUCCESSFUL++))
    else
        ((FAILED++))
    fi
done

# Run deck benchmark (special case)
echo ""
print_status "🚀 Running deck benchmark..." "$BLUE"
if [ -d "$SCRIPT_DIR/deck" ]; then
    cd "$SCRIPT_DIR/deck"
    if [ -f "Makefile" ]; then
        TEMP_LOG="$RESULTS_DIR/deck_output_$TIMESTAMP.log"
        if make run > "$TEMP_LOG" 2>&1; then
            print_status "✅ Deck benchmark completed successfully" "$GREEN"
            # Extract performance metrics
            grep -E "(ms|milliseconds|seconds|ops/sec|events/sec|throughput|latency|p50|p95|p99|average|median|benchmark|performance)" "$TEMP_LOG" | \
            grep -v -E "(Installing|Downloading|Building|Compiling|Warning|Error|mkdir|cd |make)" \
            > "$RESULTS_DIR/deck.txt"
            rm -f "$TEMP_LOG"
            ((SUCCESSFUL++))
        else
            print_status "❌ Deck benchmark failed" "$RED"
            echo "Benchmark failed. Error output:" > "$RESULTS_DIR/deck.txt"
            tail -20 "$TEMP_LOG" >> "$RESULTS_DIR/deck.txt"
            rm -f "$TEMP_LOG"
            ((FAILED++))
        fi
    fi
fi

# Summary
echo ""
echo "╔══════════════════════════════════════════════╗"
echo "║              Summary                         ║"
echo "╚══════════════════════════════════════════════╝"
echo ""
echo "Total benchmarks run: $((SUCCESSFUL + FAILED))"
print_status "Successful: $SUCCESSFUL" "$GREEN"
print_status "Failed: $FAILED" "$RED"
echo ""
echo "Results saved to: $RESULTS_DIR"
echo "Summary file: $SUMMARY_FILE"

# Generate aggregate report
echo ""
print_status "📊 Generating aggregate report..." "$BLUE"

cd "$SCRIPT_DIR"
if [ -f "generate_report.py" ]; then
    python3 generate_report.py "$RESULTS_DIR" "$TIMESTAMP" || print_status "⚠️  Report generation failed" "$YELLOW"
else
    print_status "⚠️  Report generator not found" "$YELLOW"
fi

echo ""
print_status "✨ Benchmark suite completed!" "$GREEN"