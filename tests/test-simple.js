// Simple WebSocket client to test basic request
const WebSocket = require('ws');

// Create a WebSocket connection
const ws = new WebSocket('ws://localhost:3001');

// When the connection is established
ws.on('open', () => {
  console.log('Connected to server');
  
  // Send a simple REQ command
  const request = ["REQ", "simple-test", {"kinds": [1], "limit": 5}];
  console.log('Sending simple request:', JSON.stringify(request));
  ws.send(JSON.stringify(request));
});

// When a message is received
ws.on('message', (data) => {
  console.log('Received:', data.toString());
  
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