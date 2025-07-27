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
    if make run > "$RESULTS_DIR/${lang}_output_$TIMESTAMP.log" 2>&1; then
        print_status "✅ $lang benchmark completed successfully" "$GREEN"
        
        # Move result files to results directory
        find . -name "benchmark_${lang}_*.json" -newer "$RESULTS_DIR/${lang}_output_$TIMESTAMP.log" -exec mv {} "$RESULTS_DIR/" \; 2>/dev/null || true
        
        echo "$lang: SUCCESS" >> "$SUMMARY_FILE"
        return 0
    else
        print_status "❌ $lang benchmark failed" "$RED"
        echo "$lang: FAILED" >> "$SUMMARY_FILE"
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
        if make run > "$RESULTS_DIR/deck_output_$TIMESTAMP.log" 2>&1; then
            print_status "✅ Deck benchmark completed successfully" "$GREEN"
            echo "deck: SUCCESS" >> "$SUMMARY_FILE"
            ((SUCCESSFUL++))
        else
            print_status "❌ Deck benchmark failed" "$RED"
            echo "deck: FAILED" >> "$SUMMARY_FILE"
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