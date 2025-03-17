#!/bin/bash

# This script demonstrates both approaches to creating cassettes:
# 1. Using the 'cassette dub' command with JSON events
# 2. Building a custom cassette with the RelayHandler trait

# Set up directories
OUTPUT_DIR="./test-output"
EVENTS_FILE="./test-events.json"
CUSTOM_DIR="./custom-cassette"

echo "Testing both cassette creation approaches..."
mkdir -p $OUTPUT_DIR

# -------- Approach 1: JSON-based cassette with 'cassette dub' --------
echo ""
echo "===== Approach 1: Creating a cassette from JSON events ====="

# Create a sample events file
echo "Creating sample events.json file..."
cat > $EVENTS_FILE << 'EOF'
{
  "events": [
    ["EVENT", "abc123", {
      "id": "abcdef1234567890",
      "pubkey": "e8b487c079b0f67c695ae6c4c2552a47f38adfa2533cc5926bd2c102942fdcb7",
      "created_at": 1683556800,
      "kind": 1,
      "tags": [["t", "test"], ["t", "example"]],
      "content": "This is a test event",
      "sig": "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
    }],
    ["EVENT", "abc123", {
      "id": "0123456789abcdef",
      "pubkey": "e8b487c079b0f67c695ae6c4c2552a47f38adfa2533cc5926bd2c102942fdcb7",
      "created_at": 1683556810,
      "kind": 1,
      "tags": [["t", "sample"]],
      "content": "Another test event",
      "sig": "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
    }]
  ]
}
EOF

# Run the dub command
echo "Running 'cassette dub' command..."
cassette dub -n "Test Cassette" -d "A test cassette from JSON events" -a "Test User" -o "$OUTPUT_DIR/json-based" $EVENTS_FILE

if [ $? -eq 0 ]; then
    echo "✅ JSON-based cassette successfully created!"
    echo "Output files in $OUTPUT_DIR/json-based/"
    ls -la "$OUTPUT_DIR/json-based/"
else
    echo "❌ Failed to create JSON-based cassette"
fi

# -------- Approach 2: Custom cassette with RelayHandler --------
echo ""
echo "===== Approach 2: Building a custom cassette with RelayHandler ====="

# Check if Rust and wasm-bindgen are installed
if ! command -v cargo &> /dev/null || ! command -v wasm-bindgen &> /dev/null; then
    echo "❌ Error: Rust and wasm-bindgen are required for building custom cassettes."
    echo "Please install them and try again."
    exit 1
fi

# Navigate to the custom cassette directory and build it
echo "Building custom cassette..."
cd $CUSTOM_DIR || { echo "❌ Error: Custom cassette directory not found"; exit 1; }

# Build with wasm target
echo "Running cargo build..."
cargo build --target wasm32-unknown-unknown --release

if [ $? -ne 0 ]; then
    echo "❌ Error: Failed to build custom cassette"
    exit 1
fi

# Generate JavaScript bindings with wasm-bindgen
echo "Generating JavaScript bindings..."
mkdir -p "$OUTPUT_DIR/custom"
wasm-bindgen target/wasm32-unknown-unknown/release/custom_cassette.wasm --out-dir "$OUTPUT_DIR/custom" --no-modules

if [ $? -eq 0 ]; then
    echo "✅ Custom cassette successfully built!"
    echo "Output files in $OUTPUT_DIR/custom/"
    ls -la "$OUTPUT_DIR/custom/"
else
    echo "❌ Failed to generate JavaScript bindings for custom cassette"
    exit 1
fi

# -------- Summary --------
echo ""
echo "===== Summary ====="
echo "Both cassette creation approaches have been tested:"
echo "1. JSON-based cassette (via 'cassette dub'): $OUTPUT_DIR/json-based/"
echo "2. Custom cassette (via RelayHandler): $OUTPUT_DIR/custom/"
echo ""
echo "Next steps:"
echo "- Test these cassettes with the Boombox server"
echo "- Connect to the relay with a Nostr client"
echo "- Try the custom cassette with different #custom tags"
echo ""
echo "Done!" 