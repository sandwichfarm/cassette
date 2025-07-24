#!/bin/bash
set -e

echo "🎯 Testing Verbose Flag Behavior"
echo "================================"

# Check if CLI was built
if [ ! -f "cli/target/release/cassette" ]; then
    echo "❌ Error: CLI not built. Building now..."
    cd cli && cargo build --release && cd ..
fi

# Add to PATH
export PATH="$PWD/cli/target/release:$PATH"

# Create a small test events file
cat > test-verbose.json << 'EOF'
[
  {
    "id": "test1",
    "pubkey": "pub1",
    "created_at": 1234567890,
    "kind": 1,
    "tags": [],
    "content": "Test event",
    "sig": "sig1"
  }
]
EOF

echo -e "\n1️⃣ Testing normal mode (no output expected)..."
echo "================================================"
cassette record test-verbose.json \
    --name "test-quiet" \
    --output cassettes-test

echo -e "\n\n2️⃣ Testing verbose mode (detailed output expected)..."
echo "======================================================"
cassette record test-verbose.json \
    --name "test-verbose" \
    --output cassettes-test \
    --verbose

echo -e "\n\n3️⃣ Testing interactive mode (no compilation output)..."
echo "======================================================="
cassette record test-verbose.json \
    --name "test-interactive" \
    --output cassettes-test \
    --interactive

echo -e "\n\n4️⃣ Testing interactive + verbose (should show output)..."
echo "========================================================="
cassette record test-verbose.json \
    --name "test-both" \
    --output cassettes-test \
    --interactive \
    --verbose

# Cleanup
rm -f test-verbose.json
rm -rf cassettes-test

echo -e "\n✅ Test completed!"