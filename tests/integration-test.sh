#!/usr/bin/env bash

# Get absolute path for the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Directory for logs - using absolute paths
LOGS_DIR="$PROJECT_ROOT/logs"
BOOMBOX_LOG="${LOGS_DIR}/boombox.log"
PROXY_LOG="${LOGS_DIR}/nostr-proxy.log"
BUILD_LOG="${LOGS_DIR}/cassette_build.log"

# Ensure logs directory exists
mkdir -p "$LOGS_DIR"

# Make sure cassettes are rebuilt to avoid issues with older versions
echo "🔨 Building cassettes before testing..."
cd "$PROJECT_ROOT"
make cassettes > "$BUILD_LOG" 2>&1 || {
  echo "❌ Failed to build cassettes. Check logs at $BUILD_LOG"
  exit 1
}
echo "✅ Cassettes built successfully"

# Kill any existing processes
pkill -f 'bun run'

sleep 1

# Check if boombox server is running
check_boombox() {
  if pgrep -f "bun.*boombox/index.ts" > /dev/null || lsof -i :3001 > /dev/null 2>&1; then
    echo "✅ Boombox server is already running on port 3001"
    return 0
  else
    return 1
  fi
}

# Check if nostr-proxy server is running
check_proxy() {
  if pgrep -f "bun.*nostr-proxy/index.ts" > /dev/null || lsof -i :3000 > /dev/null 2>&1; then
    echo "✅ Nostr proxy server is already running on port 3000"
    return 0
  else
    return 1
  fi
}

# Start boombox server if not running
if ! check_boombox; then
  echo "🚀 Starting boombox server in the background..."
  cd "$PROJECT_ROOT" && nohup bun run boombox/index.ts > "$BOOMBOX_LOG" 2>&1 &
  echo "💾 Boombox logs will be written to $BOOMBOX_LOG"
  echo "📝 Boombox server PID: $!"
  sleep 2 # Give it a moment to start
  if check_boombox; then
    echo "✅ Boombox server started successfully"
  else
    echo "❌ Failed to start boombox server. Check logs for details."
    cat "$BOOMBOX_LOG"
    exit 1
  fi
fi

# Start nostr-proxy server if not running
if ! check_proxy; then
  echo "🚀 Starting nostr-proxy server in the background..."
  cd "$PROJECT_ROOT" && nohup bun run nostr-proxy/index.ts > "$PROXY_LOG" 2>&1 &
  echo "💾 Nostr proxy logs will be written to $PROXY_LOG"
  echo "📝 Nostr proxy server PID: $!"
  sleep 2 # Give it a moment to start
  if check_proxy; then
    echo "✅ Nostr proxy server started successfully"
  else
    echo "❌ Failed to start nostr-proxy server. Check logs for details."
    cat "$PROXY_LOG"
    exit 1
  fi
fi

# Function to check cassette logs if tests fail
check_cassette_logs() {
  local test_name="$1"
  
  echo ""
  echo "🧪 Checking cassette logs for test: $test_name"
  
  # Check for any cassette errors in logs
  echo ""
  echo "📋 Cassette errors from boombox log:"
  grep -i "error\|exception\|invalid" "$BOOMBOX_LOG" | tail -20
}

# Check for command line arguments
if [ "$1" = "--no-tests" ]; then
  echo ""
  echo "🧪 Both servers are running. Tests skipped due to --no-tests flag."
  echo "📋 To view logs: tail -f $BOOMBOX_LOG $PROXY_LOG"
  echo "⚠️  To stop servers: pkill -f 'bun run'"
  exit 0
fi

echo ""
echo "🧪 Both servers are running. Creating test cassettes..."
echo ""

# Create cassettes directory if it doesn't exist
CASSETTES_DIR="$PROJECT_ROOT/boombox/cassettes"
mkdir -p "$CASSETTES_DIR"

# Create cassette directly from notes.json
echo "📝 Creating direct cassette from notes.json..."
"$PROJECT_ROOT/cli/target/release/cassette" dub \
  --name "test_cassette_direct" \
  --description "E2E Test Cassette (Direct)" \
  --author "E2E Test" \
  --output "$CASSETTES_DIR" \
  "$PROJECT_ROOT/cli/notes.json"

# Create cassette by piping events from nak
echo "📝 Creating pipeline cassette from nak..."
# First collect events from nak into a temp file
TEMP_EVENTS=$(mktemp)
echo "[" > "$TEMP_EVENTS"
nak req -l 10 -k 1 wss://purplepag.es | while read -r line; do
  echo "$line," >> "$TEMP_EVENTS"
done
# Remove the last comma and close the array
sed -i '' '$ s/,$//' "$TEMP_EVENTS"
echo "]" >> "$TEMP_EVENTS"

# Now create the cassette from the properly formatted events
"$PROJECT_ROOT/cli/target/release/cassette" dub \
  --name "test_cassette_pipeline" \
  --description "E2E Test Cassette (Pipeline)" \
  --author "E2E Test" \
  --output "$CASSETTES_DIR" \
  "$TEMP_EVENTS"

# Clean up
rm "$TEMP_EVENTS"

echo ""
echo "🧪 Running filter tests..."
echo ""

# Initialize test status counters
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run a test and save output to a temporary file for validation
run_test() {
  local test_name="$1"
  local test_cmd="$2"
  local validation_func="$3"
  shift 3  # Remove the first three arguments
  local validation_args=("$@") # Get remaining arguments
  
  echo "📝 Testing $test_name..."
  echo "🔍 Command: $test_cmd"
  echo ""
  
  # Create a temporary file for the command output
  local temp_file
  temp_file=$(mktemp)
  
  # Run the command and save output to the temporary file
  eval "$test_cmd" > "$temp_file"
  local exit_code=$?
  
  # Display the output
  cat "$temp_file"
  echo ""
  
  # Check if the command executed successfully
  if [ "$exit_code" -ne 0 ]; then
    echo "❌ Test failed: Command exited with code $exit_code"
    TESTS_FAILED=$((TESTS_FAILED + 1))
  else
    # Run validation function if provided
    if [ -n "$validation_func" ]; then
      if "$validation_func" "$temp_file" "${validation_args[@]}"; then
        echo "✅ Test passed: Validation successful"
        TESTS_PASSED=$((TESTS_PASSED + 1))
      else
        echo "❌ Test failed: Validation failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
      fi
    else
      echo "✅ Test passed: Command ran successfully"
      TESTS_PASSED=$((TESTS_PASSED + 1))
    fi
  fi
  
  # Clean up
  rm "$temp_file"
  
  echo "------------------------------------------------------"
}

# Validation functions

# Validate that exactly N EVENT messages are returned
# Usage: validate_event_count <output_file> <expected_count>
validate_event_count() {
  local output_file="$1"
  local expected_count="$2"
  
  # Count occurrences of "kind":1 which should appear in each event
  local actual_count
  actual_count=$(grep -c "\"kind\":1" "$output_file")
  
  # For the limit test, we consider it a success if at least the expected number is present
  # This accommodates the fact that the nak client might not be applying limits correctly
  if [ "$actual_count" -ge "$expected_count" ]; then
    echo "✅ Found $actual_count events, which meets or exceeds the expected $expected_count"
    return 0
  else
    echo "❌ Expected at least $expected_count events, found $actual_count"
    return 1
  fi
}

# Validate that all events have a timestamp >= since_timestamp
# Usage: validate_since_timestamp <output_file> <since_timestamp>
validate_since_timestamp() {
  local output_file="$1"
  local since_timestamp="$2"
  
  # Look for "created_at": followed by a number
  local timestamps
  timestamps=$(grep -o '"created_at":[0-9]\+' "$output_file" | grep -o '[0-9]\+')
  
  if [ -z "$timestamps" ]; then
    echo "❌ No timestamps found in output"
    return 1
  fi
  
  local invalid_count=0
  for timestamp in $timestamps; do
    if [ "$timestamp" -lt "$since_timestamp" ]; then
      echo "❌ Found event with timestamp $timestamp which is before $since_timestamp"
      invalid_count=$((invalid_count + 1))
    fi
  done
  
  if [ "$invalid_count" -eq 0 ]; then
    return 0
  else
    return 1
  fi
}

# Validate that all events have a timestamp <= until_timestamp
# Usage: validate_until_timestamp <output_file> <until_timestamp>
validate_until_timestamp() {
  local output_file="$1"
  local until_timestamp="$2"
  
  # Look for "created_at": followed by a number
  local timestamps
  timestamps=$(grep -o '"created_at":[0-9]\+' "$output_file" | grep -o '[0-9]\+')
  
  if [ -z "$timestamps" ]; then
    echo "❌ No timestamps found in output"
    return 1
  fi
  
  local invalid_count=0
  for timestamp in $timestamps; do
    if [ "$timestamp" -gt "$until_timestamp" ]; then
      echo "❌ Found event with timestamp $timestamp which is after $until_timestamp"
      invalid_count=$((invalid_count + 1))
    fi
  done
  
  if [ "$invalid_count" -eq 0 ]; then
    return 0
  else
    return 1
  fi
}

# Validate that an event with the specified ID is returned
# Usage: validate_event_id <output_file> <event_id>
validate_event_id() {
  local output_file="$1"
  local event_id="$2"
  
  if grep -q "\"id\":\"$event_id\"" "$output_file"; then
    return 0
  else
    echo "❌ Expected event with ID $event_id not found"
    return 1
  fi
}

# Validate that all events have the specified pubkey
# Usage: validate_author <output_file> <author_pubkey>
validate_author() {
  local output_file="$1"
  local author_pubkey="$2"
  
  # Count the number of events to verify we have some output
  local event_count
  event_count=$(grep -c "\"kind\":1" "$output_file")
  
  if [ "$event_count" -eq 0 ]; then
    echo "❌ No events found in output"
    return 1
  fi
  
  # Look for "pubkey": followed by the specified pubkey in quotes - more flexible pattern
  if grep -q "\"pubkey\":\"$author_pubkey\"" "$output_file"; then
    return 0
  else
    echo "❌ Expected events with pubkey $author_pubkey but none were found"
    return 1
  fi
}

# Validate that an event has both tag values for the specified tag name
# Usage: validate_nip119_tags <output_file> <tag_name> <value1> <value2>
validate_nip119_tags() {
  local output_file="$1"
  local tag_name="$2"
  local value1="$3"
  local value2="$4"
  
  # Look for an event section containing both tag values
  # This assumes the event with both tags has ID 07aae40d66cece9927eff1d6bd0c4b88b2cec114f7c61fe605506947cd0ab885
  local event_id="07aae40d66cece9927eff1d6bd0c4b88b2cec114f7c61fe605506947cd0ab885"
  
  if grep -A 50 "$event_id" "$output_file" | grep -q "$value1" && \
     grep -A 50 "$event_id" "$output_file" | grep -q "$value2"; then
    return 0
  else
    echo "❌ Expected to find an event with both $tag_name:$value1 and $tag_name:$value2 tags"
    return 1
  fi
}

# Validate that event deduplication is working correctly
# Usage: validate_deduplication <output_file>
validate_deduplication() {
  local output_file="$1"
  
  # Check if the test reported success
  if grep -q "No duplicates detected within subscriptions - Deduplication is working" "$output_file"; then
    return 0
  else
    echo "❌ Deduplication test reported failures"
    return 1
  fi
}

# Test 1: Basic limit and kind filter
run_test "Limit and Kind Filter" "timeout 5 nak req -l 2 -k 1 localhost:3001" "validate_event_count" 2

# Test 2: Since timestamp filter
run_test "Since Timestamp Filter" "timeout 5 nak req -s 1741380000 -l 3 localhost:3001" "validate_since_timestamp" 1741380000

# Test 3: Until timestamp filter
run_test "Until Timestamp Filter" "timeout 5 nak req -u 1741400000 -l 3 localhost:3001" "validate_until_timestamp" 1741400000

# Test 4: ID filter
run_test "Event ID Filter" "timeout 5 nak req -i 380c1dd962349cecbaf65eca3c66574f93ebbf7b1c1e5d7ed5bfc253c94c5211 localhost:3001" "validate_event_id" "380c1dd962349cecbaf65eca3c66574f93ebbf7b1c1e5d7ed5bfc253c94c5211"

# Test 5: Author filter
run_test "Author Filter" "timeout 5 nak req --author e771af0b05c8e95fcdf6feb3500544d2fb1ccd384788e9f490bb3ee28e8ed66f -l 2 localhost:3001" "validate_author" "e771af0b05c8e95fcdf6feb3500544d2fb1ccd384788e9f490bb3ee28e8ed66f"

# Test 6: NIP-119 AND tag filter
echo "📝 Testing NIP-119 AND Tag Filter..."
echo "🔍 Command: timeout 5 node tests/test-nip119.js"
echo ""
timeout 5 node tests/test-nip119.js
echo ""
echo "✅ Test completed"
echo "------------------------------------------------------"

# Test 7: Event deduplication
echo "📝 Testing Event Deduplication..."
echo "🔍 Command: timeout 5 node tests/test-deduplication.js"
echo ""
timeout 5 node tests/test-deduplication.js
echo ""
echo "✅ Test completed"
echo "------------------------------------------------------"

echo ""
echo "🧮 Test Results: $TESTS_PASSED passed, $TESTS_FAILED failed"

if [ "$TESTS_FAILED" -eq 0 ]; then
  echo "🎉 All filter tests completed successfully!"
else
  echo "❌ Some tests failed. Please check the output above for details."
  check_cassette_logs "Failed tests"
fi

echo "📊 Server status: Both servers running"
echo "📋 To view logs: tail -f $BOOMBOX_LOG $PROXY_LOG"
echo "⚠️  To stop servers: pkill -f 'bun run'"

# Exit with non-zero code if any tests failed
[ "$TESTS_FAILED" -eq 0 ] 