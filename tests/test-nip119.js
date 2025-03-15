const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:3001');

ws.on('open', function() {
  console.log('Connected to server');
  
  // Create a request with NIP-119 &t filter (AND operator)
  const reqMsg = ['REQ', '1:', {'&t': ['value1', 'value2']}];
  
  console.log('Sending REQ with NIP-119 &t filter:', JSON.stringify(reqMsg));
  ws.send(JSON.stringify(reqMsg));
});

ws.on('message', function(data) {
  console.log('\nReceived:');
  const msg = JSON.parse(data.toString());
  console.log(JSON.stringify(msg, null, 2));
  
  if (msg[0] === 'EOSE') {
    console.log('\nClosing connection...');
    ws.close();
  }
});

ws.on('error', function(error) {
  console.error('WebSocket error:', error);
});

ws.on('close', function() {
  console.log('Connection closed');
  process.exit(0);
});
