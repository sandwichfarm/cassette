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

echo ""
echo "🧪 Both servers are running. You can now run the test command:"
echo "nak req -l 5 -k 1 localhost:3001"
echo ""
echo "📊 This should display 5 notes from sanwichfavs."
echo "📋 To view logs: tail -f $BOOMBOX_LOG $PROXY_LOG"
echo "⚠️  To stop servers: pkill -f 'bun run'" 