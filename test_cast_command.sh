#!/bin/bash
set -e

echo "🧪 Testing Cassette Cast Command"
echo "================================"

# Check if cassette CLI exists
if ! command -v cassette &> /dev/null; then
    echo "❌ Error: cassette command not found in PATH"
    echo "Building cassette CLI..."
    cd cli
    cargo build --release
    export PATH="$PWD/target/release:$PATH"
    cd ..
fi

# Test dry run with example cassettes
echo -e "\n1️⃣ Testing dry run mode..."
cassette cast \
    cassettes/test_cassette_direct.wasm \
    --relays wss://relay.example.com \
    --relays wss://relay2.example.com \
    --dry-run \
    --timeout 5

echo -e "\n2️⃣ Testing with multiple cassettes (dry run)..."
cassette cast \
    cassettes/test_cassette_direct.wasm \
    cassettes/test_cassette_pipeline.wasm \
    --relays wss://relay.example.com \
    --dry-run \
    --concurrency 2 \
    --throttle 100

echo -e "\n3️⃣ Testing help output..."
cassette cast --help

echo -e "\n✅ Cast command tests completed!"