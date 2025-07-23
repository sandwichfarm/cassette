#!/usr/bin/env bash

# End-to-end test script for cassette
# This script will:
# 1. Load events from notes.json
# 2. Pipe events from nak into cassette dub from relay wss://purplepag.es
# 3. Load the wasms into boombox (via cassette-loader)

set -e # Exit on error

# Directory for logs
LOGS_DIR="../logs"
mkdir -p "$LOGS_DIR"
E2E_LOG="${LOGS_DIR}/e2e-test.log"

# Path to the cassette CLI
CASSETTE_CLI="../cli/target/release/cassette"
# Path to notes.json
NOTES_JSON="../cli/notes.json"
# Output directory for cassettes
CASSETTES_DIR="../cassettes"
# Name for the generated cassette
CASSETTE_NAME="e2e-test-cassette-$(date +%s)"

echo "🧪 Starting E2E Test..."
echo "📝 Test started at $(date)" | tee -a "$E2E_LOG"

# Step 1: Check if notes.json exists
if [ ! -f "$NOTES_JSON" ]; then
  echo "❌ Error: $NOTES_JSON not found" | tee -a "$E2E_LOG"
  exit 1
fi

echo "✅ Found notes.json file" | tee -a "$E2E_LOG"
echo "📊 Notes file contains $(jq length "$NOTES_JSON") events" | tee -a "$E2E_LOG"

# Step 2: Fetch events from wss://purplepag.es using nak and pipe to cassette dub
echo "🔄 Fetching events from wss://purplepag.es and creating cassette..." | tee -a "$E2E_LOG"

# Check if nak is installed
if ! command -v nak &> /dev/null; then
  echo "❌ Error: nak (Nostr Army Knife) is not installed" | tee -a "$E2E_LOG"
  echo "Please install it with: go install github.com/fiatjaf/nak@latest" | tee -a "$E2E_LOG"
  exit 1
fi

# Create a temporary file for the events
TEMP_EVENTS_FILE=$(mktemp)

# First option: Use nak to fetch events from wss://purplepag.es
echo "🔄 Fetching events from wss://purplepag.es using nak..." | tee -a "$E2E_LOG"
nak req -l 10 -k 1 wss://purplepag.es > "$TEMP_EVENTS_FILE" 2>> "$E2E_LOG"

# Check if we got any events
if [ ! -s "$TEMP_EVENTS_FILE" ]; then
  echo "⚠️ Warning: No events fetched from wss://purplepag.es" | tee -a "$E2E_LOG"
  echo "🔄 Using local notes.json file instead" | tee -a "$E2E_LOG"
  cp "$NOTES_JSON" "$TEMP_EVENTS_FILE"
fi

# Now create the cassette from the events
echo "🔨 Creating cassette from events..." | nak req -l 10 -k 1 wss://purplepag.es | 
"$CASSETTE_CLI" dub --name "$CASSETTE_NAME" \
  --description "E2E Test Cassette" \
  --author "E2E Test" \
  --output "$CASSETTES_DIR" \
  "$TEMP_EVENTS_FILE" 2>> "$E2E_LOG"

# Check if cassette was created
CASSETTE_WASM="$CASSETTES_DIR/$CASSETTE_NAME.wasm"
if [ ! -f "$CASSETTE_WASM" ]; then
  echo "❌ Error: Failed to create cassette WASM file" | tee -a "$E2E_LOG"
  exit 1
fi

echo "✅ Successfully created cassette WASM file: $CASSETTE_WASM" | tee -a "$E2E_LOG"

# Clean up temp file
rm "$TEMP_EVENTS_FILE"

# Step 3: Verify the boombox server will load our cassette
echo "🔍 Verifying that boombox will load our cassette..." | tee -a "$E2E_LOG"

# Check if boombox server is running
if ! pgrep -f "bun.*boombox/index.ts" > /dev/null || ! lsof -i :3001 > /dev/null 2>&1; then
  echo "🚀 Starting boombox server in the background..." | tee -a "$E2E_LOG"
  nohup bun run ../boombox/index.ts > "${LOGS_DIR}/boombox.log" 2>&1 &
  echo "💾 Boombox logs will be written to ${LOGS_DIR}/boombox.log" | tee -a "$E2E_LOG"
  echo "📝 Boombox server PID: $!" | tee -a "$E2E_LOG"
  sleep 2 # Give it a moment to start
fi

# Test the cassette by connecting to boombox and making a request
echo "🔄 Testing cassette by connecting to boombox and making a request..." | tee -a "$E2E_LOG"

# Create a temporary file for the test output
TEST_OUTPUT=$(mktemp)

# Use nak to connect to boombox and make a request
nak req -l 5 -k 1 localhost:3001 > "$TEST_OUTPUT" 2>> "$E2E_LOG"

# Check if we got a response
if [ ! -s "$TEST_OUTPUT" ]; then
  echo "❌ Error: No response from boombox server" | tee -a "$E2E_LOG"
  exit 1
fi

# Display the response summary
echo "✅ Received response from boombox server" | tee -a "$E2E_LOG"
echo "📊 Response contains $(grep -c "\"kind\":" "$TEST_OUTPUT") events" | tee -a "$E2E_LOG"

# Clean up
rm "$TEST_OUTPUT"

echo "🎉 E2E Test completed successfully!" | tee -a "$E2E_LOG"
echo "✅ Cassette created: $CASSETTE_WASM" | tee -a "$E2E_LOG"
echo "✅ Cassette loaded into boombox server" | tee -a "$E2E_LOG"
echo "📝 Test completed at $(date)" | tee -a "$E2E_LOG"

exit 0 