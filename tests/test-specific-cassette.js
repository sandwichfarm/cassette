// Test script to target a specific cassette
const WebSocket = require('ws');

// Create a WebSocket connection
const ws = new WebSocket('ws://localhost:3001');

// When the connection is established
ws.on('open', () => {
  console.log('Connected to server');
  
  // Send a REQ command with the same parameters as the integration test
  const request = ["REQ", "test-specific", {
    "kinds": [1],
    "limit": 2,
    "cassette": "integration_test_cassette@notes.json" // Specify the cassette to use
  }];
  
  console.log('Sending request to specific cassette:', JSON.stringify(request));
  ws.send(JSON.stringify(request));
});

// When a message is received
ws.on('message', (data) => {
  console.log('Received:', data.toString());
  try {
    const parsed = JSON.parse(data.toString());
    console.log('Parsed JSON:', JSON.stringify(parsed, null, 2));
  } catch (e) {
    console.log('Could not parse as JSON:', e.message);
  }
  
  // After receiving a message, close the connection
  setTimeout(() => {
    console.log('Closing connection...');
    ws.close();
  }, 1000);
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