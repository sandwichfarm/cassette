#!/usr/bin/env python3
"""
Cassette Deck Performance Benchmark

This script stress tests the cassette deck with:
- Valid REQ messages with various filters
- Invalid/malformed messages
- EVENT spam (valid and invalid)
- Concurrent connections
- Large result sets
"""

import asyncio
import json
import time
import statistics
import random
import string
import hashlib
import websockets
from datetime import datetime
from collections import defaultdict
from typing import List, Dict, Any, Tuple
import argparse
import sys

# Test configuration
DEFAULT_DECK_URL = "ws://localhost:9999"
DEFAULT_DURATION = 60  # seconds
DEFAULT_CONNECTIONS = 10
DEFAULT_MESSAGES_PER_SECOND = 100

# Performance metrics storage
metrics = defaultdict(list)
errors = defaultdict(int)
filter_metrics = defaultdict(list)  # Track performance by filter type


def generate_random_hex(length: int) -> str:
    """Generate random hex string of specified length."""
    return ''.join(random.choices('0123456789abcdef', k=length))


def generate_valid_event() -> Dict[str, Any]:
    """Generate a valid nostr event."""
    content = f"Benchmark test event {random.randint(1, 1000000)}"
    created_at = int(time.time()) - random.randint(0, 86400)  # Random time in last 24h
    
    event = {
        "id": generate_random_hex(64),
        "pubkey": generate_random_hex(64),
        "created_at": created_at,
        "kind": random.choice([1, 7, 0, 3]),  # Common event kinds
        "tags": [],
        "content": content,
        "sig": generate_random_hex(128)
    }
    
    # Add some random tags
    if random.random() > 0.5:
        event["tags"].append(["e", generate_random_hex(64)])
    if random.random() > 0.7:
        event["tags"].append(["p", generate_random_hex(64)])
    
    return event


def generate_invalid_event() -> Dict[str, Any]:
    """Generate an invalid event (missing fields, wrong types, etc)."""
    base_event = generate_valid_event()
    
    # Choose a random way to make it invalid
    invalid_type = random.choice([
        "missing_id", "missing_pubkey", "missing_sig", 
        "invalid_created_at", "invalid_kind", "wrong_id_length",
        "wrong_pubkey_length", "invalid_json"
    ])
    
    if invalid_type == "missing_id":
        del base_event["id"]
    elif invalid_type == "missing_pubkey":
        del base_event["pubkey"]
    elif invalid_type == "missing_sig":
        del base_event["sig"]
    elif invalid_type == "invalid_created_at":
        base_event["created_at"] = "not a number"
    elif invalid_type == "invalid_kind":
        base_event["kind"] = -1
    elif invalid_type == "wrong_id_length":
        base_event["id"] = generate_random_hex(32)  # Should be 64
    elif invalid_type == "wrong_pubkey_length":
        base_event["pubkey"] = generate_random_hex(32)  # Should be 64
    
    return base_event


def generate_req_filters() -> List[Dict[str, Any]]:
    """Generate various REQ filter combinations."""
    filters = []
    
    # Simple filters
    filters.extend([
        {},  # Get all events
        {"limit": 10},
        {"limit": 100},
        {"limit": 1000},
        {"kinds": [1]},
        {"kinds": [1, 7]},
        {"authors": [generate_random_hex(64)]},
        {"since": int(time.time()) - 3600},  # Last hour
        {"until": int(time.time())},
        {"#e": [generate_random_hex(64)]},  # Tag filter
    ])
    
    # Complex filters
    filters.extend([
        {
            "kinds": [1],
            "limit": 50,
            "since": int(time.time()) - 86400
        },
        {
            "authors": [generate_random_hex(64) for _ in range(5)],
            "kinds": [1, 7],
            "limit": 100
        },
        {
            "#e": [generate_random_hex(64)],
            "#p": [generate_random_hex(64)],
            "kinds": [1]
        }
    ])
    
    return filters


def generate_invalid_messages() -> List[Any]:
    """Generate various invalid messages."""
    return [
        "not even json",
        {"not": "an array"},
        [],  # Empty array
        ["UNKNOWN_COMMAND"],
        ["REQ"],  # Missing subscription ID and filters
        ["REQ", "sub1"],  # Missing filters
        ["REQ", "sub1", "not a filter object"],
        ["EVENT"],  # Missing event
        ["EVENT", "not an event object"],
        ["EVENT", {"id": "too short"}],
        json.dumps(["REQ", "sub1", {}]) + "extra garbage",
        ["REQ", "sub1", {"limit": "not a number"}],
        ["REQ", "sub1", {"kinds": "not an array"}],
        ["REQ", "sub1", {"since": "not a timestamp"}],
        None,  # Will be handled specially
    ]


async def measure_request(websocket, message: Any) -> Tuple[float, bool, str]:
    """Send a message and measure response time."""
    start_time = time.time()
    error = None
    success = True
    
    try:
        if message is None:
            # Send empty message
            await websocket.send("")
        else:
            await websocket.send(json.dumps(message))
        
        # Wait for response with timeout
        response = await asyncio.wait_for(websocket.recv(), timeout=5.0)
        
        # Parse response to check if it's an error
        try:
            resp_data = json.loads(response)
            if isinstance(resp_data, list) and len(resp_data) > 0:
                if resp_data[0] == "NOTICE":
                    error = resp_data[1] if len(resp_data) > 1 else "Unknown error"
                    success = False
                elif resp_data[0] == "OK" and len(resp_data) > 3 and not resp_data[2]:
                    error = resp_data[3]
                    success = False
        except:
            pass
            
    except asyncio.TimeoutError:
        error = "Timeout"
        success = False
    except Exception as e:
        error = str(e)
        success = False
    
    elapsed = time.time() - start_time
    return elapsed, success, error


async def req_benchmark(websocket, duration: int, target_rps: int):
    """Benchmark REQ messages."""
    print("\n📊 REQ Message Benchmark")
    print("=" * 50)
    
    filters = generate_req_filters()
    start_time = time.time()
    request_count = 0
    
    while time.time() - start_time < duration:
        # Pick a random filter
        filter_set = random.choice(filters)
        sub_id = f"bench-{generate_random_hex(8)}"
        req_message = ["REQ", sub_id, filter_set]
        
        # Determine filter type for categorization
        filter_type = "empty"
        if filter_set:
            if "limit" in filter_set and len(filter_set) == 1:
                filter_type = f"limit_{filter_set['limit']}"
            elif "kinds" in filter_set and len(filter_set) == 1:
                filter_type = "kinds_only"
            elif "authors" in filter_set and len(filter_set) == 1:
                filter_type = "authors_only"
            elif len(filter_set) > 2:
                filter_type = "complex"
            elif any(k.startswith("#") for k in filter_set):
                filter_type = "tag_filter"
        
        elapsed, success, error = await measure_request(websocket, req_message)
        
        if success:
            metrics["req_valid"].append(elapsed)
            filter_metrics[filter_type].append(elapsed)
            
            # Read all events until EOSE
            event_count = 0
            try:
                while True:
                    response = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                    resp_data = json.loads(response)
                    if resp_data[0] == "EVENT":
                        event_count += 1
                    elif resp_data[0] == "EOSE":
                        break
            except:
                pass
            
            metrics["req_event_counts"].append(event_count)
            
            # Send CLOSE
            await websocket.send(json.dumps(["CLOSE", sub_id]))
        else:
            errors["req_errors"] += 1
        
        request_count += 1
        
        # Rate limiting
        expected_time = request_count / target_rps
        actual_time = time.time() - start_time
        if actual_time < expected_time:
            await asyncio.sleep(expected_time - actual_time)
    
    print(f"✅ Sent {request_count} REQ messages")
    print(f"   Valid responses: {len(metrics['req_valid'])}")
    print(f"   Errors: {errors['req_errors']}")
    if metrics['req_valid']:
        print(f"   Avg response time: {statistics.mean(metrics['req_valid'])*1000:.2f}ms")
        print(f"   P95 response time: {sorted(metrics['req_valid'])[int(len(metrics['req_valid'])*0.95)]*1000:.2f}ms")
    if metrics['req_event_counts']:
        print(f"   Avg events returned: {statistics.mean(metrics['req_event_counts']):.1f}")


async def event_benchmark(websocket, duration: int, target_rps: int):
    """Benchmark EVENT messages."""
    print("\n📊 EVENT Message Benchmark")
    print("=" * 50)
    
    start_time = time.time()
    request_count = 0
    
    while time.time() - start_time < duration:
        # 70% valid events, 30% invalid
        if random.random() < 0.7:
            event = generate_valid_event()
            message = ["EVENT", event]
            elapsed, success, error = await measure_request(websocket, message)
            
            if success:
                metrics["event_valid"].append(elapsed)
            else:
                errors["event_rejected"] += 1
        else:
            event = generate_invalid_event()
            message = ["EVENT", event]
            elapsed, success, error = await measure_request(websocket, message)
            errors["event_invalid"] += 1
        
        request_count += 1
        
        # Rate limiting
        expected_time = request_count / target_rps
        actual_time = time.time() - start_time
        if actual_time < expected_time:
            await asyncio.sleep(expected_time - actual_time)
    
    print(f"✅ Sent {request_count} EVENT messages")
    print(f"   Valid accepted: {len(metrics['event_valid'])}")
    print(f"   Valid rejected: {errors['event_rejected']}")
    print(f"   Invalid sent: {errors['event_invalid']}")
    if metrics['event_valid']:
        print(f"   Avg response time: {statistics.mean(metrics['event_valid'])*1000:.2f}ms")
        print(f"   P95 response time: {sorted(metrics['event_valid'])[int(len(metrics['event_valid'])*0.95)]*1000:.2f}ms")


async def invalid_message_benchmark(websocket, duration: int):
    """Benchmark invalid/malformed messages."""
    print("\n📊 Invalid Message Benchmark")
    print("=" * 50)
    
    invalid_messages = generate_invalid_messages()
    start_time = time.time()
    request_count = 0
    
    while time.time() - start_time < duration/3:  # Run for 1/3 of total duration
        message = random.choice(invalid_messages)
        
        elapsed, success, error = await measure_request(websocket, message)
        
        if not success:
            metrics["invalid_handled"].append(elapsed)
            errors[f"invalid_{error[:20]}"] += 1
        else:
            errors["invalid_not_caught"] += 1
        
        request_count += 1
        await asyncio.sleep(0.01)  # Don't spam too hard with invalid messages
    
    print(f"✅ Sent {request_count} invalid messages")
    print(f"   Properly rejected: {len(metrics['invalid_handled'])}")
    print(f"   Not caught: {errors['invalid_not_caught']}")
    if metrics['invalid_handled']:
        print(f"   Avg response time: {statistics.mean(metrics['invalid_handled'])*1000:.2f}ms")


async def concurrent_connections_test(deck_url: str, num_connections: int):
    """Test concurrent connections."""
    print(f"\n📊 Concurrent Connections Test ({num_connections} connections)")
    print("=" * 50)
    
    async def connection_worker(conn_id: int):
        try:
            async with websockets.connect(deck_url) as websocket:
                # Each connection sends a few requests
                for i in range(5):
                    sub_id = f"conn{conn_id}-sub{i}"
                    req = ["REQ", sub_id, {"limit": 10}]
                    
                    elapsed, success, error = await measure_request(websocket, req)
                    if success:
                        metrics["concurrent_req"].append(elapsed)
                        
                        # Read events
                        try:
                            while True:
                                response = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                                resp_data = json.loads(response)
                                if resp_data[0] == "EOSE":
                                    break
                        except:
                            pass
                    
                    await asyncio.sleep(0.1)
        except Exception as e:
            errors["connection_failed"] += 1
    
    # Launch all connections concurrently
    start_time = time.time()
    tasks = [connection_worker(i) for i in range(num_connections)]
    await asyncio.gather(*tasks, return_exceptions=True)
    total_time = time.time() - start_time
    
    print(f"✅ Completed in {total_time:.2f}s")
    print(f"   Successful requests: {len(metrics['concurrent_req'])}")
    print(f"   Failed connections: {errors['connection_failed']}")
    if metrics['concurrent_req']:
        print(f"   Avg response time: {statistics.mean(metrics['concurrent_req'])*1000:.2f}ms")
        print(f"   P95 response time: {sorted(metrics['concurrent_req'])[int(len(metrics['concurrent_req'])*0.95)]*1000:.2f}ms")


async def run_benchmark(deck_url: str, duration: int, connections: int, target_rps: int):
    """Run the complete benchmark suite."""
    print("🚀 Starting Cassette Deck Benchmark")
    print(f"   URL: {deck_url}")
    print(f"   Duration: {duration}s")
    print(f"   Target RPS: {target_rps}")
    print(f"   Concurrent connections: {connections}")
    
    # Test single connection first
    try:
        async with websockets.connect(deck_url) as websocket:
            # Run different benchmark types
            await req_benchmark(websocket, duration // 3, target_rps)
            await event_benchmark(websocket, duration // 3, target_rps)
            await invalid_message_benchmark(websocket, duration // 3)
    except Exception as e:
        print(f"❌ Failed to connect to deck: {e}")
        return
    
    # Test concurrent connections
    await concurrent_connections_test(deck_url, connections)
    
    # Print summary table
    print("\n📈 BENCHMARK SUMMARY")
    print("=" * 80)
    
    total_requests = sum(len(v) for v in metrics.values() if isinstance(v, list))
    total_errors = sum(errors.values())
    
    print(f"Total requests: {total_requests + total_errors}")
    print(f"Successful: {total_requests}")
    print(f"Errors: {total_errors}")
    
    # Create performance summary table
    print("\n📊 PERFORMANCE METRICS BY SCENARIO")
    print("=" * 80)
    print(f"{'Scenario':<25} {'Count':>8} {'Avg (ms)':>10} {'P50 (ms)':>10} {'P95 (ms)':>10} {'P99 (ms)':>10}")
    print("-" * 80)
    
    # Define scenarios to report
    scenarios = [
        ("REQ - All Operations", "req_valid"),
        ("EVENT - Valid", "event_valid"),
        ("Invalid Messages", "invalid_handled"),
        ("Concurrent REQ", "concurrent_req"),
    ]
    
    for scenario_name, metric_key in scenarios:
        if metric_key in metrics and metrics[metric_key]:
            times = metrics[metric_key]
            sorted_times = sorted(times)
            print(f"{scenario_name:<25} {len(times):>8} "
                  f"{statistics.mean(times)*1000:>10.2f} "
                  f"{sorted_times[len(times)//2]*1000:>10.2f} "
                  f"{sorted_times[int(len(times)*0.95)]*1000:>10.2f} "
                  f"{sorted_times[int(len(times)*0.99)]*1000:>10.2f}")
    
    # REQ event count statistics
    if "req_event_counts" in metrics and metrics["req_event_counts"]:
        counts = metrics["req_event_counts"]
        print(f"\n{'REQ Event Counts':<25} {len(counts):>8} "
              f"{statistics.mean(counts):>10.1f} "
              f"{sorted(counts)[len(counts)//2]:>10.0f} "
              f"{sorted(counts)[int(len(counts)*0.95)]:>10.0f} "
              f"{sorted(counts)[int(len(counts)*0.99)]:>10.0f}")
    
    # Error breakdown table
    if errors:
        print("\n❌ ERROR BREAKDOWN")
        print("=" * 65)
        print(f"{'Error Type':<53} {'Count':>10}")
        print("-" * 65)
        for error_type, count in sorted(errors.items(), key=lambda x: x[1], reverse=True)[:10]:
            # Don't truncate error messages
            error_display = error_type
            if len(error_display) > 50:
                error_display = error_display[:50] + "..."
            print(f"{error_display:<53} {count:>10}")
    
    # Filter performance breakdown
    if filter_metrics:
        print("\n🔍 REQ FILTER PERFORMANCE")
        print("=" * 80)
        print(f"{'Filter Type':<25} {'Count':>8} {'Avg (ms)':>10} {'P50 (ms)':>10} {'P95 (ms)':>10} {'P99 (ms)':>10}")
        print("-" * 80)
        
        for filter_type, times in sorted(filter_metrics.items()):
            if times:
                sorted_times = sorted(times)
                print(f"{filter_type:<25} {len(times):>8} "
                      f"{statistics.mean(times)*1000:>10.2f} "
                      f"{sorted_times[len(times)//2]*1000:>10.2f} "
                      f"{sorted_times[int(len(times)*0.95)]*1000:>10.2f} "
                      f"{sorted_times[int(len(times)*0.99)]*1000:>10.2f}")
    
    # Overall statistics
    if total_requests > 0:
        all_times = []
        for key, times in metrics.items():
            if isinstance(times, list) and times and key != "req_event_counts":
                all_times.extend(times)
        
        if all_times:
            print(f"\n📈 OVERALL STATISTICS")
            print("=" * 50)
            print(f"Total measurements: {len(all_times)}")
            print(f"Success rate: {(total_requests / (total_requests + total_errors)) * 100:.1f}%")
            print(f"Avg response time: {statistics.mean(all_times)*1000:.2f}ms")
            print(f"Std deviation: {statistics.stdev(all_times)*1000:.2f}ms" if len(all_times) > 1 else "")
            
            # Throughput calculation
            total_duration = sum(all_times)
            if total_duration > 0:
                print(f"Effective throughput: {len(all_times)/total_duration:.1f} req/s")
    
    # Save detailed results
    results = {
        "timestamp": datetime.now().isoformat(),
        "config": {
            "deck_url": deck_url,
            "duration": duration,
            "target_rps": target_rps,
            "connections": connections
        },
        "metrics": {k: v if not isinstance(v, list) else {
            "count": len(v),
            "mean": statistics.mean(v) if v else 0,
            "min": min(v) if v else 0,
            "max": max(v) if v else 0,
            "p50": sorted(v)[len(v)//2] if v else 0,
            "p95": sorted(v)[int(len(v)*0.95)] if v else 0,
            "p99": sorted(v)[int(len(v)*0.99)] if v else 0,
        } for k, v in metrics.items()},
        "errors": dict(errors)
    }
    
    with open(f"results/benchmark_{int(time.time())}.json", "w") as f:
        json.dump(results, f, indent=2)
    
    print(f"\n💾 Detailed results saved to benchmarks/results/")


def main():
    parser = argparse.ArgumentParser(description="Cassette Deck Performance Benchmark")
    parser.add_argument("--url", default=DEFAULT_DECK_URL, help="Deck WebSocket URL")
    parser.add_argument("--duration", type=int, default=DEFAULT_DURATION, help="Test duration in seconds")
    parser.add_argument("--connections", type=int, default=DEFAULT_CONNECTIONS, help="Number of concurrent connections")
    parser.add_argument("--rps", type=int, default=DEFAULT_MESSAGES_PER_SECOND, help="Target requests per second")
    
    args = parser.parse_args()
    
    asyncio.run(run_benchmark(args.url, args.duration, args.connections, args.rps))


if __name__ == "__main__":
    main()