# Cassette Benchmark Report - RUST

**Generated:** Sat Jul 26 21:55:44 PDT 2025

## Results

```
    Finished `release` profile [optimized] target(s) in 0.04s
     Running `target/release/cassette-bench ../samples/small.wasm ../samples/medium.wasm ../samples/large.wasm --iterations 100`
🚀 Cassette WASM Benchmark (Rust)
   Cassettes: 3
   Iterations: 100

📼 Benchmarking: small.wasm
============================================================
ℹ️  Cassette: Unknown
   Events: 0
   Size: 634.8 KB
🔥 Warming up...

🏃 Running 100 iterations per filter...
❌ Error with ../samples/small.wasm: Failed to get send function

📼 Benchmarking: medium.wasm
============================================================
ℹ️  Cassette: Unknown
   Events: 0
   Size: 11833.7 KB
🔥 Warming up...

🏃 Running 100 iterations per filter...
❌ Error with ../samples/medium.wasm: Failed to get send function
❌ Not found: ../samples/large.wasm
```
