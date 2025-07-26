# Cassette Benchmarks

Performance benchmarking suite for both:
1. **Direct Cassette WASM modules** - Testing raw cassette performance
2. **Cassette Deck relay** - Testing the deck server implementation

## Direct Cassette Benchmarks

Test the raw performance of cassette WASM modules:

```bash
# Run cassette benchmarks (compares multiple cassettes)
./run_cassette_benchmark.sh

# Run with custom parameters
python scripts/cassette_benchmark.py data/*.wasm --iterations 1000
```

### What it measures:
- **REQ Performance**: Response time for 17+ different filter types
- **COUNT Performance**: NIP-45 COUNT query speed (if supported)
- **Memory Operations**: Allocation/deallocation performance
- **Filter Efficiency**: Comparison across different query patterns

### Output:
- Comparative performance table across cassettes
- Filter-specific timing breakdowns
- Memory operation benchmarks
- P50, P95, P99 latency percentiles

## Deck Relay Benchmarks

This benchmark suite stress tests the deck with:
- ✅ Valid REQ messages with various filter combinations
- ❌ Invalid/malformed messages
- 📤 EVENT spam (both valid and invalid events)
- 🔥 Concurrent connections
- 📊 Large result sets (10k+ events)

## Quick Start

```bash
# Run the complete benchmark suite
./run_benchmark.sh
```

This will:
1. Generate a large cassette with 10k+ real events (if needed)
2. Start a deck instance with verbose logging
3. Run the benchmark suite
4. Save results and performance metrics

## Benchmark Components

### 1. REQ Message Tests
- Various filter combinations (empty, limits, kinds, authors, tags)
- Measures response time and events returned
- Tests subscription lifecycle (REQ → EVENT* → EOSE → CLOSE)

### 2. EVENT Message Tests
- 70% valid events, 30% invalid events
- Tests event validation and storage
- Measures acceptance/rejection rates

### 3. Invalid Message Tests
- Malformed JSON
- Missing required fields
- Wrong data types
- Protocol violations

### 4. Concurrent Connection Tests
- Multiple simultaneous WebSocket connections
- Parallel request handling
- Connection limit testing

## Custom Benchmarks

Run with custom parameters:

```bash
# Long duration test (5 minutes)
python3 scripts/deck_benchmark.py --duration 300

# High throughput test
python3 scripts/deck_benchmark.py --rps 500 --connections 50

# Custom deck URL
python3 scripts/deck_benchmark.py --url ws://remote-deck:9999
```

## Results

Results are saved in `results/` directory:
- `benchmark_<timestamp>.json` - Detailed metrics in JSON format
- `deck_benchmark_<timestamp>.log` - Deck verbose logs with performance traces

## Metrics Collected

- **Response Times**: Min, Max, Average, P50, P95, P99
- **Throughput**: Requests per second achieved
- **Error Rates**: By error type
- **Event Counts**: Events returned per query
- **Deck Performance**: Internal timing metrics from deck logs

## Generating Larger Cassettes

To create even larger test datasets:

```bash
# Edit the relay list and limits in generate_large_cassette.sh
vim scripts/generate_large_cassette.sh

# Regenerate
rm data/benchmark-large.wasm
./scripts/generate_large_cassette.sh
```

## Interpreting Results

Good performance indicators:
- P95 response time < 10ms for simple queries
- P95 response time < 100ms for complex queries returning many events
- Error rate < 0.1% for valid requests
- Can handle 100+ concurrent connections

Performance issues to watch for:
- Response times increasing over time (memory leak)
- High error rates under load
- Crashes or hangs with concurrent connections
- Slow cassette queries (check deck logs for timing)