#!/usr/bin/env python3
"""Test script for Python cassette loader"""

import json
import sys
from cassette_loader import load_cassette


def test_cassette(wasm_path: str):
    """Test cassette loading and operations"""
    print(f"🧪 Testing cassette: {wasm_path}\n")
    
    # Load WASM file
    with open(wasm_path, 'rb') as f:
        wasm_bytes = f.read()
    
    # Test 1: Load cassette
    print("Test 1: Loading cassette...")
    result = load_cassette(wasm_bytes, name=wasm_path.split('/')[-1], debug=False)
    
    if not result['success']:
        print(f"❌ Failed to load: {result['error']}")
        return False
        
    cassette = result['cassette']
    print(f"✅ Loaded successfully")
    print(f"   Name: {cassette.info.name}")
    print(f"   Version: {cassette.info.version}")
    
    # Test 2: Describe function
    print("\nTest 2: Describe function...")
    desc = cassette.describe()
    desc_data = json.loads(desc)
    print(f"✅ Description loaded")
    print(f"   Event count: {desc_data.get('event_count', 0)}")
    
    # Test 3: REQ message
    print("\nTest 3: REQ message...")
    req_msg = json.dumps(["REQ", "test_sub", {"kinds": [1], "limit": 5}])
    response = cassette.req(req_msg)
    
    if response:
        resp_data = json.loads(response)
        if resp_data[0] == "EVENT":
            print(f"✅ Got EVENT response")
            print(f"   Event ID: {resp_data[2]['id'][:8]}...")
        elif resp_data[0] == "NOTICE":
            print(f"⚠️  Got NOTICE: {resp_data[1]}")
        else:
            print(f"✅ Got response: {resp_data[0]}")
    else:
        print("❌ Empty response")
    
    # Test 4: Memory leak check
    print("\nTest 4: Memory leak check...")
    stats = cassette.get_memory_stats()
    if stats.allocation_count == 0:
        print("✅ No memory leaks detected")
    else:
        print(f"⚠️  {stats.allocation_count} allocations remaining")
    
    # Test 5: Multiple requests
    print("\nTest 5: Stress test (100 requests)...")
    leak_count = 0
    for i in range(100):
        req_msg = json.dumps(["REQ", f"sub_{i}", {"kinds": [1], "limit": 1}])
        response = cassette.req(req_msg)
        if i % 20 == 0:
            stats = cassette.get_memory_stats()
            if stats.allocation_count > leak_count:
                leak_count = stats.allocation_count
    
    final_stats = cassette.get_memory_stats()
    if final_stats.allocation_count == 0:
        print("✅ No memory leaks after 100 requests")
    else:
        print(f"⚠️  Memory leak detected: {final_stats.allocation_count} allocations")
    
    # Test 6: CLOSE message
    print("\nTest 6: CLOSE message...")
    close_msg = json.dumps(["CLOSE", "test_sub"])
    close_resp = cassette.close(close_msg)
    close_data = json.loads(close_resp)
    if close_data[0] == "NOTICE":
        print(f"✅ CLOSE processed: {close_data[1]}")
    
    # Clean up
    dispose_result = cassette.dispose()
    print(f"\n🧹 Cleaned up {dispose_result['allocationsCleanedUp']} allocations")
    
    print("\n✅ All tests completed!")
    return True


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python test_cassette.py <wasm_file>")
        sys.exit(1)
    
    success = test_cassette(sys.argv[1])
    sys.exit(0 if success else 1)