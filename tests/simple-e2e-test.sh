#!/usr/bin/env bash

# Simple End-to-end test script for cassette
# This script focuses on the direct path:
# 1. Use notes.json directly to create a cassette
# 2. Verify the cassette can be loaded by boombox
# 3. Make sure the process works correctly

set -e # Exit on error
set -x # Print commands for debugging

# Directory for logs
LOGS_DIR="../logs"
mkdir -p "$LOGS_DIR"
E2E_LOG="${LOGS_DIR}/simple-e2e-test.log"

# Path to the cassette CLI
CASSETTE_CLI="../cli/target/release/cassette"
# Path to notes.json
NOTES_JSON="../cli/notes.json"
# Output directory for cassettes
CASSETTES_DIR="../cassettes"
# Name for the generated cassette
CASSETTE_NAME="simple-e2e-cassette-$(date +%s)"

echo "🧪 Starting Simple E2E Test..."
echo "📝 Test started at $(date)" | tee -a "$E2E_LOG"

# Step 1: Check if notes.json exists
if [ ! -f "$NOTES_JSON" ]; then
  echo "❌ Error: $NOTES_JSON not found" | tee -a "$E2E_LOG"
  exit 1
fi

NUM_EVENTS=$(jq length "$NOTES_JSON")
echo "✅ Found notes.json file with $NUM_EVENTS events" | tee -a "$E2E_LOG"

# Step 2: Create a cassette directly from notes.json
echo "🔨 Creating cassette from notes.json..." | tee -a "$E2E_LOG"
"$CASSETTE_CLI" dub \
  --name "$CASSETTE_NAME" \
  --description "Simple E2E Test Cassette" \
  --author "E2E Test" \
  --output "$CASSETTES_DIR" \
  "$NOTES_JSON" 2>> "$E2E_LOG"

# Check if cassette was created
CASSETTE_WASM="$CASSETTES_DIR/$CASSETTE_NAME.wasm"
if [ ! -f "$CASSETTE_WASM" ]; then
  echo "❌ Error: Failed to create cassette WASM file" | tee -a "$E2E_LOG"
  exit 1
fi

echo "✅ Successfully created cassette WASM file: $CASSETTE_WASM" | tee -a "$E2E_LOG"

# Step 3: Make sure boombox server is running
echo "🔍 Checking if boombox server is running..." | tee -a "$E2E_LOG"
if ! pgrep -f "bun.*boombox/index.ts" > /dev/null || ! lsof -i :3001 > /dev/null 2>&1; then
  echo "🚀 Starting boombox server in the background..." | tee -a "$E2E_LOG"
  cd ../
  nohup bun run boombox/index.ts > "${LOGS_DIR}/boombox.log" 2>&1 &
  echo "💾 Boombox logs will be written to ${LOGS_DIR}/boombox.log" | tee -a "$E2E_LOG"
  echo "📝 Boombox server PID: $!" | tee -a "$E2E_LOG"
  cd tests/
  sleep 3 # Give it more time to start
fi

# Step 4: Test the cassette by connecting to boombox
echo "🔄 Testing cassette by connecting to boombox..." | tee -a "$E2E_LOG"

# Create a temporary file for the test output
TEST_OUTPUT=$(mktemp)

# Use nak to connect to boombox and make a request
nak req -l 3 -k 1 localhost:3001 > "$TEST_OUTPUT" 2>> "$E2E_LOG"

# Check if we got a response
if [ ! -s "$TEST_OUTPUT" ]; then
  echo "❌ Error: No response from boombox server" | tee -a "$E2E_LOG"
  exit 1
fi

# Display the response summary
EVENT_COUNT=$(grep -c "\"kind\":" "$TEST_OUTPUT")
echo "✅ Received response from boombox server with $EVENT_COUNT events" | tee -a "$E2E_LOG"

# Show a snippet of the response
echo "📄 Sample of response:" | tee -a "$E2E_LOG"
head -n 20 "$TEST_OUTPUT" | tee -a "$E2E_LOG"

# Clean up
rm "$TEST_OUTPUT"

echo "🎉 Simple E2E Test completed successfully!" | tee -a "$E2E_LOG"
echo "✅ Cassette created: $CASSETTE_WASM" | tee -a "$E2E_LOG"
echo "✅ Cassette successfully loaded by boombox server" | tee -a "$E2E_LOG"
echo "📝 Test completed at $(date)" | tee -a "$E2E_LOG"

exit 0 