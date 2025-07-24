#!/bin/bash
set -e

echo "🎯 Testing Interactive UI for Cassette CLI"
echo "=========================================="

# Check if CLI was built
if [ ! -f "cli/target/release/cassette" ]; then
    echo "❌ Error: CLI not built. Building now..."
    cd cli && cargo build --release && cd ..
fi

# Add to PATH
export PATH="$PWD/cli/target/release:$PATH"

# Create a test events file
echo "📝 Creating test events file..."
cat > test-events.json << 'EOF'
[
  {
    "id": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "pubkey": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    "created_at": 1234567890,
    "kind": 1,
    "tags": [["t", "bitcoin"], ["t", "nostr"]],
    "content": "Hello Nostr! This is a test note about Bitcoin.",
    "sig": "sig1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
  },
  {
    "id": "2234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "pubkey": "bbcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    "created_at": 1234567891,
    "kind": 7,
    "tags": [["e", "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"], ["p", "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"]],
    "content": "⚡",
    "sig": "sig2234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
  },
  {
    "id": "3234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "pubkey": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    "created_at": 1234567892,
    "kind": 30023,
    "tags": [["d", "bitcoin-whitepaper"], ["title", "Bitcoin: A Peer-to-Peer Electronic Cash System"]],
    "content": "A purely peer-to-peer version of electronic cash...",
    "sig": "sig3234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
  },
  {
    "id": "4234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "pubkey": "cbcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    "created_at": 1234567893,
    "kind": 3,
    "tags": [["p", "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"], ["p", "bbcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"]],
    "content": "{\"name\":\"Test User\",\"about\":\"Testing Nostr\"}",
    "sig": "sig4234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
  }
]
EOF

echo -e "\n1️⃣ Testing normal record mode..."
cassette record test-events.json \
    --name "test-normal" \
    --description "Test cassette normal mode" \
    --author "Test Author" \
    --output cassettes

echo -e "\n2️⃣ Testing interactive record mode..."
echo "Press Ctrl+C if the UI appears frozen"
cassette record test-events.json \
    --name "test-interactive" \
    --description "Test cassette interactive mode" \
    --author "Test Author" \
    --output cassettes \
    --interactive

echo -e "\n3️⃣ Testing with piped input (normal)..."
cat test-events.json | cassette record \
    --name "test-piped" \
    --description "Test cassette piped mode" \
    --output cassettes

echo -e "\n4️⃣ Testing with piped input (interactive)..."
cat test-events.json | cassette record \
    --name "test-piped-interactive" \
    --description "Test cassette piped interactive" \
    --output cassettes \
    --interactive

echo -e "\n✅ All tests completed!"
echo "Generated cassettes:"
ls -la cassettes/*.wasm 2>/dev/null || echo "No cassettes found"

# Cleanup
rm -f test-events.json