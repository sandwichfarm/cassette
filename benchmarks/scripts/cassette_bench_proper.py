#!/usr/bin/env python3
"""
Direct Cassette WASM Benchmark using official Python bindings

This benchmarks cassette performance using the cassette_loader from bindings/py
"""

import sys
import os
import json
import time
import statistics
import argparse
import signal
from pathlib import Path
from collections import defaultdict
from typing import List, Dict, Any, Tuple

# Timeout handler
class TimeoutError(Exception):
    pass

def timeout_handler(signum, frame):
    raise TimeoutError("Operation timed out")

# Set signal handler
signal.signal(signal.SIGALRM, timeout_handler)

# Add the bindings directory to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "bindings" / "py"))

try:
    from cassette_loader import Cassette
except ImportError:
    print("❌ Failed to import cassette_loader. Make sure you're in the cassette project.")
    exit(1)


def generate_random_hex(length: int) -> str:
    """Generate random hex string."""
    import random
    return ''.join(random.choices('0123456789abcdef', k=length))


def generate_test_filters() -> List[Tuple[str, Dict[str, Any]]]:
    """Generate various filter combinations for testing."""
    filters = [
        ("empty", {}),
        ("limit_1", {"limit": 1}),
        ("limit_10", {"limit": 10}),
        ("limit_100", {"limit": 100}),
        ("limit_1000", {"limit": 1000}),
        ("kinds_1", {"kinds": [1]}),
        ("kinds_multiple", {"kinds": [1, 7, 0]}),
        ("author_single", {"authors": [generate_random_hex(64)]}),
        ("authors_5", {"authors": [generate_random_hex(64) for _ in range(5)]}),
        ("since_recent", {"since": int(time.time()) - 3600}),
        ("until_now", {"until": int(time.time())}),
        ("time_range", {"since": int(time.time()) - 86400, "until": int(time.time())}),
        ("tag_e", {"#e": [generate_random_hex(64)]}),
        ("tag_p", {"#p": [generate_random_hex(64)]}),
        ("complex", {
            "kinds": [1],
            "limit": 50,
            "since": int(time.time()) - 86400,
            "authors": [generate_random_hex(64)]
        }),
    ]
    return filters


def benchmark_cassette(cassette_path: str, iterations: int = 100, debug: bool = False) -> Dict[str, Any]:
    """Benchmark a single cassette."""
    print(f"\n📼 Benchmarking: {os.path.basename(cassette_path)}")
    print("=" * 60)
    
    # Load cassette
    with open(cassette_path, 'rb') as f:
        wasm_bytes = f.read()
    
    cassette = Cassette(wasm_bytes, os.path.basename(cassette_path), debug=debug)
    
    # Get cassette info
    info = cassette.info
    print(f"ℹ️  Cassette: {info.name}")
    print(f"   Events: {info.event_count}")
    print(f"   Size: {len(wasm_bytes) / 1024:.1f} KB")
    
    results = {
        "cassette": os.path.basename(cassette_path),
        "file_size": len(wasm_bytes),
        "event_count": info.event_count,
        "info": {
            "name": info.name,
            "description": info.description,
            "version": info.version
        }
    }
    
    # Test filters
    test_filters = generate_test_filters()
    
    # Warm up
    print("🔥 Warming up...")
    try:
        for i in range(10):
            if debug:
                print(f"  Warmup {i+1}/10...")
            req = json.dumps(["REQ", f"warmup-{i}", {"limit": 1}])
            response = cassette.send(req)
            if debug:
                print(f"  Got response: {type(response)}, length: {len(response) if isinstance(response, list) else 'single'}")
    except Exception as e:
        print(f"❌ Warmup failed: {e}")
        if debug:
            import traceback
            traceback.print_exc()
    
    # Quick test to see what we're dealing with
    print("\n🔍 Quick cassette test...")
    try:
        test_response = cassette.send(json.dumps(["REQ", "test", {"limit": 1}]))
        if isinstance(test_response, list):
            print(f"   Got {len(test_response)} responses")
            for i, resp in enumerate(test_response[:3]):  # Show first 3
                try:
                    parsed = json.loads(resp)
                    print(f"   Response {i+1}: {parsed[0]}")
                except:
                    print(f"   Response {i+1}: (unparseable)")
        else:
            print(f"   Got single response: {test_response[:100]}...")
    except Exception as e:
        print(f"   Test failed: {e}")
    
    print(f"\n🏃 Running {iterations} iterations per filter...")
    
    # Benchmark REQ performance
    filter_results = {}
    
    for idx, (filter_name, filter_obj) in enumerate(test_filters):
        print(f"\n  Testing filter {idx+1}/{len(test_filters)}: {filter_name}", end="", flush=True)
        times = []
        event_counts = []
        
        for i in range(iterations):
            if i % 10 == 0:
                print(".", end="", flush=True)
            
            sub_id = f"bench-{filter_name}-{i}"
            
            # Measure query time
            req_message = json.dumps(["REQ", sub_id, filter_obj])
            
            try:
                start = time.time()
                responses = cassette.send(req_message)
                elapsed = time.time() - start
            except Exception as e:
                print(f"\n    ❌ Error on iteration {i}: {e}")
                continue
            
            times.append(elapsed)
            
            # Count events returned (responses is a list of strings for REQ)
            if isinstance(responses, list):
                event_count = sum(1 for r in responses if json.loads(r)[0] == "EVENT")
                event_counts.append(event_count)
            else:
                event_counts.append(0)
        
        # Calculate statistics
        if times:
            sorted_times = sorted(times)
            avg_ms = statistics.mean(times) * 1000
            avg_events = statistics.mean(event_counts) if event_counts else 0
            print(f" ✓ ({avg_ms:.1f}ms avg, {avg_events:.0f} events)")
            
            filter_results[filter_name] = {
                "count": len(times),
                "avg_ms": avg_ms,
                "min_ms": min(times) * 1000,
                "max_ms": max(times) * 1000,
                "p50_ms": sorted_times[len(times)//2] * 1000,
                "p95_ms": sorted_times[int(len(times)*0.95)] * 1000,
                "p99_ms": sorted_times[int(len(times)*0.99)] * 1000,
                "avg_events": avg_events,
                "max_events": max(event_counts) if event_counts else 0
            }
        else:
            print(" ❌ (no successful iterations)")
            filter_results[filter_name] = {"count": 0, "avg_ms": 0}
    
    results["filters"] = filter_results
    
    # Benchmark COUNT if available
    exports = cassette.instance.exports(cassette.store)
    if 'count' in exports:
        print("\n📊 Benchmarking COUNT queries...")
        count_results = {}
        
        for filter_name, filter_obj in test_filters[:10]:  # Subset for COUNT
            times = []
            
            for i in range(iterations // 2):
                sub_id = f"count-{filter_name}-{i}"
                
                count_message = json.dumps(["COUNT", sub_id, filter_obj])
                
                start = time.time()
                response = cassette.send(count_message)
                elapsed = time.time() - start
                
                times.append(elapsed)
            
            sorted_times = sorted(times)
            count_results[filter_name] = {
                "count": len(times),
                "avg_ms": statistics.mean(times) * 1000,
                "p50_ms": sorted_times[len(times)//2] * 1000,
                "p95_ms": sorted_times[int(len(times)*0.95)] * 1000
            }
        
        results["count"] = count_results
    
    # Get memory stats
    mem_stats = cassette.get_memory_stats()
    results["memory"] = {
        "total_pages": mem_stats.total_pages,
        "total_bytes": mem_stats.total_bytes,
        "allocation_count": mem_stats.allocation_count
    }
    
    return results


def print_comparison_table(results: List[Dict[str, Any]]):
    """Print a comparison table of benchmark results."""
    print("\n📊 CASSETTE PERFORMANCE COMPARISON")
    print("=" * 100)
    
    # REQ Query Performance
    print("\n🔍 REQ QUERY PERFORMANCE (milliseconds)")
    print("=" * 100)
    
    # Collect all filter names
    all_filters = set()
    for result in results:
        all_filters.update(result["filters"].keys())
    
    filter_names = sorted(all_filters)
    
    # Print header
    print(f"{'Filter Type':<20}", end="")
    for result in results:
        cassette_name = os.path.basename(result["cassette"])[:12]
        print(f"{cassette_name:>12}", end=" ")
    print()
    print("-" * (20 + 13 * len(results)))
    
    # Print data
    for filter_name in filter_names:
        print(f"{filter_name:<20}", end="")
        for result in results:
            if filter_name in result["filters"]:
                avg_ms = result["filters"][filter_name]["avg_ms"]
                print(f"{avg_ms:>11.2f}", end="  ")
            else:
                print(f"{'N/A':>11}", end="  ")
        print()
    
    # Summary stats
    print("\n📈 SUMMARY STATISTICS")
    print("=" * 100)
    print(f"{'Cassette':<30} {'Size (KB)':>10} {'Events':>10} {'Avg (ms)':>10} {'P95 (ms)':>10}")
    print("-" * 70)
    
    for result in results:
        all_times = [f["avg_ms"] for f in result["filters"].values()]
        all_p95 = [f["p95_ms"] for f in result["filters"].values()]
        
        avg_time = statistics.mean(all_times) if all_times else 0
        avg_p95 = statistics.mean(all_p95) if all_p95 else 0
        
        print(f"{result['cassette']:<30} "
              f"{result['file_size']/1024:>10.1f} "
              f"{result['event_count']:>10} "
              f"{avg_time:>10.2f} "
              f"{avg_p95:>10.2f}")
    
    # Event retrieval stats
    print("\n📦 EVENT RETRIEVAL STATISTICS")
    print("=" * 100)
    print(f"{'Filter Type':<20} ", end="")
    for result in results:
        cassette_name = os.path.basename(result["cassette"])[:12]
        print(f"{cassette_name + ' (avg)':>12}", end=" ")
    print()
    print("-" * (20 + 13 * len(results)))
    
    for filter_name in ["empty", "limit_10", "limit_100", "kinds_1"]:
        if filter_name in filter_names:
            print(f"{filter_name:<20}", end="")
            for result in results:
                if filter_name in result["filters"]:
                    avg_events = result["filters"][filter_name]["avg_events"]
                    print(f"{avg_events:>11.1f}", end="  ")
                else:
                    print(f"{'N/A':>11}", end="  ")
            print()


def main():
    parser = argparse.ArgumentParser(description="Cassette WASM Benchmark")
    parser.add_argument("cassettes", nargs="+", help="Cassette WASM files to benchmark")
    parser.add_argument("--iterations", "-i", type=int, default=100,
                       help="Iterations per test (default: 100)")
    parser.add_argument("--debug", "-d", action="store_true",
                       help="Enable debug output")
    parser.add_argument("--output", "-o", help="Save results to JSON file")
    
    args = parser.parse_args()
    
    print("🚀 Cassette WASM Benchmark")
    print(f"   Cassettes: {len(args.cassettes)}")
    print(f"   Iterations: {args.iterations}")
    
    results = []
    for cassette_path in args.cassettes:
        if not os.path.exists(cassette_path):
            print(f"❌ Not found: {cassette_path}")
            continue
        
        try:
            result = benchmark_cassette(cassette_path, args.iterations, args.debug)
            results.append(result)
        except Exception as e:
            print(f"❌ Error with {cassette_path}: {e}")
            if args.debug:
                import traceback
                traceback.print_exc()
    
    if results:
        print_comparison_table(results)
        
        if args.output:
            with open(args.output, 'w') as f:
                json.dump({
                    "timestamp": time.time(),
                    "iterations": args.iterations,
                    "results": results
                }, f, indent=2)
            print(f"\n💾 Results saved to: {args.output}")


if __name__ == "__main__":
    main()