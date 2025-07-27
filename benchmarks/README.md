# Cassette Benchmarks

Comprehensive benchmarking suite for testing Cassette performance across all language bindings and the Cassette Deck relay.

## Directory Structure

```
benchmarks/
├── samples/          # WASM cassette samples used for benchmarking
├── results/          # Benchmark results (JSON and logs)
├── py/              # Python binding benchmarks
├── rust/            # Rust binding benchmarks
├── cpp/             # C++ binding benchmarks
├── dart/            # Dart binding benchmarks
├── go/              # Go binding benchmarks
├── js/              # JavaScript/Node.js binding benchmarks
├── deck/            # Cassette Deck benchmarks
└── run_all_benchmarks.sh  # Master benchmark runner
```

## Quick Start

### Run All Language Benchmarks

```bash
# Run benchmarks for all language bindings
./run_all_benchmarks.sh
```

This will:
- Run benchmarks for each language binding (Python, Rust, C++, Dart, Go, JS)
- Save results to the `results/` directory
- Generate a summary report

### Run Individual Language Benchmarks

Each language has its own directory with a Makefile:

```bash
# Python
cd py && make run

# Rust
cd rust && make run

# C++
cd cpp && make run

# Dart
cd dart && make run

# Go
cd go && make run

# JavaScript/Node.js
cd js && make run

# Deck benchmark
cd deck && make run
```

## Benchmark Types

### 1. Language Binding Benchmarks

Tests the raw performance of cassette WASM modules through each language binding:

- **REQ Performance**: Response time for various filter types
- **Event Retrieval**: Number of events returned per query
- **Memory Usage**: WASM memory allocation statistics
- **Percentiles**: P50, P95, P99 response times

### 2. Deck Relay Benchmarks

Stress tests the Cassette Deck server with:
- ✅ Valid REQ messages with various filter combinations
- ❌ Invalid/malformed messages
- 📤 EVENT spam (both valid and invalid events)
- 🔥 Concurrent connections
- 📊 Large result sets

## Sample Cassettes

The `samples/` directory contains test cassettes of various sizes:
- `small.wasm` - Small cassette for quick tests
- `medium.wasm` - Medium-sized cassette
- `benchmark-large.wasm` - Large cassette for stress testing

## Filter Types Tested

All benchmarks test the same set of filters for consistency:
- Empty filter (all events)
- Limit filters (1, 10, 100, 1000)
- Kind filters (single and multiple)
- Author filters (single and multiple)
- Time-based filters (since, until, range)
- Tag filters (#e, #p)
- Complex multi-condition filters

## Custom Parameters

### Language Benchmarks

Most benchmarks support custom iterations:

```bash
# Python
cd py && make run ITERATIONS=200

# Rust
cd rust && cargo run --release -- --iterations 200 ../samples/*.wasm

# Go
cd go && ./benchmark --iterations 200 ../samples/*.wasm

# JavaScript
cd js && node benchmark.js --iterations 200 ../samples/*.wasm
```

### Deck Benchmark

```bash
# Long duration test (5 minutes)
cd deck && make run DURATION=300

# High throughput test
cd deck && make run RPS=500 CONNECTIONS=50

# Custom deck URL
cd deck && make run DECK_URL=ws://remote-deck:9999
```

## Results

Results are saved in the `results/` directory:

### Language Benchmark Results
- `benchmark_py_[timestamp].json` - Python results
- `benchmark_rust_[timestamp].json` - Rust results
- `benchmark_cpp_[timestamp].json` - C++ results
- `benchmark_dart_[timestamp].json` - Dart results
- `benchmark_go_[timestamp].json` - Go results
- `benchmark_js_[timestamp].json` - JavaScript results

### Deck Results
- `benchmark_[timestamp].json` - Deck performance metrics
- `deck_benchmark_[timestamp].log` - Detailed deck logs

### Metrics Collected

Each benchmark captures:
- **Response Times**: Min, Max, Average, P50, P95, P99
- **Throughput**: Operations per second
- **Event Counts**: Events returned per query type
- **Memory Stats**: Allocation counts and sizes
- **Error Rates**: By error type (deck only)

## Dependencies

### Python
- Python 3.8+
- cassette_loader from bindings/py

### Rust
- Rust 1.70+
- Cargo

### C++
- C++17 compiler
- wasmtime C++ bindings
- nlohmann/json

### Dart
- Dart SDK 2.17+
- wasm package

### Go
- Go 1.21+
- wasmtime-go

### JavaScript
- Node.js 16+
- npm

### Deck Benchmark
- Python 3.8+
- websockets
- Running Cassette Deck instance

## Installing Dependencies

```bash
# Install all language dependencies
cd py && make install-deps
cd ../rust && make install-deps
cd ../cpp && make install-deps
cd ../dart && make install-deps
cd ../go && make install-deps
cd ../js && make install-deps
cd ../deck && make install-deps
```

## Performance Targets

Good performance indicators:
- P95 response time < 5ms for simple queries
- P95 response time < 50ms for complex queries
- Consistent performance across language bindings (within 2x)
- Linear scaling with result set size

## Generating Test Data

To create larger test cassettes:

```bash
# Generate from real relay data
cd scripts
./generate_large_cassette.sh
```

## Contributing

When adding benchmarks:
1. Follow the existing structure and patterns
2. Use the same test filters for consistency
3. Output results in the standard JSON format
4. Include a Makefile with standard targets (run, install-deps, clean)
5. Update this README with any new requirements