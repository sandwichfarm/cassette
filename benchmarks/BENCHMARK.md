🚀 Cassette WASM Benchmark
   Cassettes: 1
   Iterations: 20

📼 Benchmarking: small.wasm
============================================================
ℹ️  Cassette: small.wasm
   Events: 0
   Size: 533.2 KB
🔥 Warming up...

🔍 Quick cassette test...
   Got 2 responses
   Response 1: EVENT
   Response 2: EOSE

🏃 Running 20 iterations per filter...

  Testing filter 1/15: empty.. ✓ (40.1ms avg, 100 events)

  Testing filter 2/15: limit_1.. ✓ (0.7ms avg, 1 events)

  Testing filter 3/15: limit_10.. ✓ (4.0ms avg, 10 events)

  Testing filter 4/15: limit_100.. ✓ (39.8ms avg, 100 events)

  Testing filter 5/15: limit_1000.. ✓ (40.6ms avg, 100 events)

  Testing filter 6/15: kinds_1.. ✓ (40.4ms avg, 100 events)

  Testing filter 7/15: kinds_multiple.. ✓ (40.7ms avg, 100 events)

  Testing filter 8/15: author_single.. ✓ (0.3ms avg, 0 events)

  Testing filter 9/15: authors_5.. ✓ (0.3ms avg, 0 events)

  Testing filter 10/15: since_recent.. ✓ (40.6ms avg, 100 events)

  Testing filter 11/15: until_now.. ✓ (40.6ms avg, 100 events)

  Testing filter 12/15: time_range.. ✓ (40.6ms avg, 100 events)

  Testing filter 13/15: tag_e.. ✓ (0.5ms avg, 0 events)

  Testing filter 14/15: tag_p.. ✓ (0.3ms avg, 0 events)

  Testing filter 15/15: complex.. ✓ (0.3ms avg, 0 events)

📊 CASSETTE PERFORMANCE COMPARISON
====================================================================================================

🔍 REQ QUERY PERFORMANCE (milliseconds)
====================================================================================================
Filter Type           small.wasm 
---------------------------------
author_single              0.29  
authors_5                  0.29  
complex                    0.28  
empty                     40.13  
kinds_1                   40.45  
kinds_multiple            40.68  
limit_1                    0.72  
limit_10                   4.04  
limit_100                 39.81  
limit_1000                40.62  
since_recent              40.57  
tag_e                      0.48  
tag_p                      0.35  
time_range                40.58  
until_now                 40.56  

📈 SUMMARY STATISTICS
====================================================================================================
Cassette                        Size (KB)     Events   Avg (ms)   P95 (ms)
----------------------------------------------------------------------
small.wasm                          533.2          0      21.99      22.88

📦 EVENT RETRIEVAL STATISTICS
====================================================================================================
Filter Type          small.wasm (avg) 
---------------------------------
empty                     100.0  
limit_10                   10.0  
limit_100                 100.0  
kinds_1                   100.0  
