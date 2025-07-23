// Simple WebSocket client to test a direct CLOSE command
const WebSocket = require('ws');

// Create a WebSocket connection
const ws = new WebSocket('ws://localhost:3001');

// When the connection is established
ws.on('open', () => {
  console.log('Connected to server');
  
  // First establish a subscription (REQ) to later close
  const reqCmd = ["REQ", "test-close-sub", {"kinds": [1], "limit": 1}];
  console.log('Sending REQ:', JSON.stringify(reqCmd));
  ws.send(JSON.stringify(reqCmd));
  
  // Wait a bit then send a CLOSE
  setTimeout(() => {
    const closeCmd = ["CLOSE", "test-close-sub"];
    console.log('Sending CLOSE:', JSON.stringify(closeCmd));
    ws.send(JSON.stringify(closeCmd));
  }, 1000);
});

// When a message is received
ws.on('message', (data) => {
  const response = data.toString();
  console.log('Received:', response);
  
  try {
    const parsed = JSON.parse(response);
    console.log('Parsed JSON:', JSON.stringify(parsed, null, 2));
  } catch (e) {
    console.log('Could not parse as JSON:', e.message);
  }
});

// When an error occurs
ws.on('error', (error) => {
  console.error('WebSocket error:', error.message);
  process.exit(1);
});

// When the connection is closed
ws.on('close', () => {
  console.log('Connection closed');
  process.exit(0);
});

// Close the connection after a timeout
setTimeout(() => {
  console.log('Closing connection due to timeout...');
  ws.close();
}, 5000); 