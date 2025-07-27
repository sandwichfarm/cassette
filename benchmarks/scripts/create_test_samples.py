#!/usr/bin/env python3
"""
Create test sample cassettes for benchmarking
"""

import json
import time
import hashlib
import random
import subprocess
import os

def generate_event_id(event_data):
    """Generate event ID according to NIP-01"""
    # Remove id and sig fields for hashing
    data = json.dumps([
        0,
        event_data['pubkey'],
        event_data['created_at'],
        event_data['kind'],
        event_data['tags'],
        event_data['content']
    ], separators=(',', ':'), ensure_ascii=False)
    return hashlib.sha256(data.encode()).hexdigest()

def generate_test_event(index, kind=1):
    """Generate a valid test event"""
    created_at = int(time.time()) - random.randint(0, 86400 * 30)  # Random time in last 30 days
    pubkey = hashlib.sha256(f"test_author_{index % 50}".encode()).hexdigest()
    
    content = f"Test event #{index} - This is a benchmark test event with some content. "
    content += "The quick brown fox jumps over the lazy dog. " * random.randint(1, 3)
    
    tags = []
    
    # Add some tags randomly
    if random.random() > 0.5:
        # Reply to another event
        ref_id = hashlib.sha256(f"ref_event_{random.randint(0, index)}".encode()).hexdigest()
        tags.append(["e", ref_id])
    
    if random.random() > 0.7:
        # Mention someone
        ref_pubkey = hashlib.sha256(f"test_author_{random.randint(0, 50)}".encode()).hexdigest()
        tags.append(["p", ref_pubkey])
    
    event = {
        "pubkey": pubkey,
        "created_at": created_at,
        "kind": kind,
        "tags": tags,
        "content": content
    }
    
    # Generate ID
    event["id"] = generate_event_id(event)
    
    # Generate fake signature
    event["sig"] = hashlib.sha256(f"sig_{event['id']}".encode()).hexdigest() * 2
    
    return event

def create_cassette(events, output_dir, name, description):
    """Create a cassette from events using the cassette tool"""
    # Write events to temporary file
    temp_file = f"/tmp/{name}_events.jsonl"
    with open(temp_file, 'w') as f:
        for event in events:
            f.write(json.dumps(event) + '\n')
    
    # Run cassette record command
    cmd = [
        'cassette', 'record',
        '-o', output_dir,
        '-n', name,
        '-d', description,
        temp_file
    ]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"Error creating {name}: {result.stderr}")
            return False
        print(f"✅ Created {name}.wasm")
        return True
    except Exception as e:
        print(f"Error running cassette: {e}")
        return False
    finally:
        # Clean up temp file
        if os.path.exists(temp_file):
            os.remove(temp_file)

def main():
    samples_dir = os.path.join(os.path.dirname(__file__), '..', 'samples')
    os.makedirs(samples_dir, exist_ok=True)
    
    print("🔨 Creating test sample cassettes...")
    
    # Create small cassette (100 events)
    print("\n📦 Creating small.wasm (100 events)...")
    small_events = []
    for i in range(100):
        kind = 1 if i < 80 else (7 if i < 90 else 0)  # 80% text, 10% reactions, 10% metadata
        small_events.append(generate_test_event(i, kind))
    
    create_cassette(small_events, samples_dir, "small", "Small benchmark cassette with 100 test events")
    
    # Create medium cassette (5,000 events)
    print("\n📦 Creating medium.wasm (5,000 events)...")
    medium_events = []
    for i in range(5000):
        # Mix of event kinds
        if i < 3000:
            kind = 1  # Text notes
        elif i < 4000:
            kind = 7  # Reactions
        elif i < 4500:
            kind = 0  # Metadata
        else:
            kind = 6  # Reposts
        medium_events.append(generate_test_event(i, kind))
    
    create_cassette(medium_events, samples_dir, "medium", "Medium benchmark cassette with 5k test events")
    
    # Create large cassette (10,000 events) - reduced size due to disk space
    print("\n📦 Creating benchmark-large.wasm (10,000 events)...")
    print("This will take a while...")
    
    large_events = []
    for i in range(10000):
        if i % 1000 == 0:
            print(f"  Generated {i}/10000 events...")
        
        # Mix of event kinds with realistic distribution
        rand = random.random()
        if rand < 0.6:
            kind = 1  # 60% text notes
        elif rand < 0.8:
            kind = 7  # 20% reactions
        elif rand < 0.9:
            kind = 0  # 10% metadata
        elif rand < 0.95:
            kind = 6  # 5% reposts
        else:
            kind = 3  # 5% follows
        
        large_events.append(generate_test_event(i, kind))
    
    create_cassette(large_events, samples_dir, "benchmark-large", "Large benchmark cassette with 10k test events")
    
    print("\n✅ Sample cassettes created!")
    print("\n📊 Cassette files:")
    os.system(f"ls -lh {samples_dir}/*.wasm 2>/dev/null || echo 'No .wasm files found'")

if __name__ == "__main__":
    main()