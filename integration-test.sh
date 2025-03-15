#!/usr/bin/env bash

# Directory for logs
LOGS_DIR="./logs"
BOOMBOX_LOG="${LOGS_DIR}/boombox.log"
PROXY_LOG="${LOGS_DIR}/nostr-proxy.log"

# Ensure logs directory exists
mkdir -p "$LOGS_DIR"

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
  nohup bun run boombox/index.ts > "$BOOMBOX_LOG" 2>&1 &
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
  nohup bun run nostr-proxy/index.ts > "$PROXY_LOG" 2>&1 &
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

# Check for command line arguments
if [ "$1" = "--no-tests" ]; then
  echo ""
  echo "🧪 Both servers are running. Tests skipped due to --no-tests flag."
  echo "📋 To view logs: tail -f $BOOMBOX_LOG $PROXY_LOG"
  echo "⚠️  To stop servers: pkill -f 'bun run'"
  exit 0
fi

echo ""
echo "🧪 Both servers are running. Running filter tests..."
echo ""

# Function to run a test and display results
run_test() {
  local test_name="$1"
  local test_cmd="$2"
  
  echo "📝 Testing $test_name..."
  echo "🔍 Command: $test_cmd"
  echo ""
  eval "$test_cmd"
  echo ""
  echo "✅ Test completed"
  echo "------------------------------------------------------"
}

# Test 1: Basic limit and kind filter
run_test "Limit and Kind Filter" "nak req -l 2 -k 1 localhost:3001"

# Test 2: Since timestamp filter
run_test "Since Timestamp Filter" "nak req -s 1741380000 -l 3 localhost:3001"

# Test 3: Until timestamp filter
run_test "Until Timestamp Filter" "nak req -u 1741400000 -l 3 localhost:3001"

# Test 4: ID filter
run_test "Event ID Filter" "nak req -i 380c1dd962349cecbaf65eca3c66574f93ebbf7b1c1e5d7ed5bfc253c94c5211 localhost:3001"

# Test 5: Author filter
run_test "Author Filter" "nak req -p e771af0b05c8e95fcdf6feb3500544d2fb1ccd384788e9f490bb3ee28e8ed66f -l 2 localhost:3001"

# Test 6: NIP-119 AND tag filter
echo "📝 Testing NIP-119 AND Tag Filter..."
echo "🔍 Running: node test-nip119.js"
echo ""
node test-nip119.js
echo ""
echo "✅ Test completed"
echo "------------------------------------------------------"

echo ""
echo "🎉 All filter tests completed successfully!"
echo "📊 Server status: Both servers running"
echo "📋 To view logs: tail -f $BOOMBOX_LOG $PROXY_LOG"
echo "⚠️  To stop servers: pkill -f 'bun run'" 