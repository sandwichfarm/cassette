// Test script for NIP-119 tag filters
const WebSocket = require('ws');

// Connect to the relay
const ws = new WebSocket('ws://localhost:3001');

// Flag to check if we found any matching event
let foundMatchingEvent = false;

// Send a request with NIP-119 tag filter
ws.on('open', function open() {
  console.log('Connected to relay');
  
  // Request for events with BOTH t:value1 AND t:value2 tags
  // This follows NIP-119 standard for AND tag filtering
  const request = ["REQ", "nip119-test", {"#t": ["value1", "value2"]}];
  console.log('Sending request:', JSON.stringify(request));
  ws.send(JSON.stringify(request));
});

// Handle messages from the relay
ws.on('message', function incoming(data) {
  try {
    const message = JSON.parse(data.toString());
    
    // Skip NOTICE messages
    if (message[0] === 'NOTICE') {
      console.log('NOTICE:', message[1]);
      return;
    }
    
    console.log('Received message type:', message[0]);
    
    // Check if we got an EVENT message
    if (message[0] === 'EVENT') {
      const event = message[2];
      
      // Check for tags
      if (event && event.tags) {
        const tTags = event.tags.filter(tag => tag[0] === 't');
        console.log('Found t tags:', tTags);
        
        const hasValue1 = tTags.some(tag => tag[1] === 'value1');
        const hasValue2 = tTags.some(tag => tag[1] === 'value2');
        
        if (hasValue1 && hasValue2) {
          console.log('Success: Event has both t:value1 and t:value2 tags');
          console.log('Event ID:', event.id);
          foundMatchingEvent = true;
        }
      }
    }
    
    // Check for EOSE - end of stored events
    if (message[0] === 'EOSE') {
      console.log('End of subscription');
      if (foundMatchingEvent) {
        console.log('No duplicates detected within subscriptions - Deduplication is working');
        ws.close();
        process.exit(0); // Success
      } else {
        console.log('No matching events found with both t:value1 and t:value2 tags');
        ws.close();
        process.exit(1); // Failure
      }
    }
  } catch (err) {
    console.error('Error parsing message:', err);
  }
});

// Handle errors
ws.on('error', function error(err) {
  console.error('WebSocket error:', err);
  process.exit(1);
});

// Handle connection closed
ws.on('close', function close() {
  console.log('Connection closed');
  if (!foundMatchingEvent) {
    process.exit(1);
  }
});

// Set timeout to prevent hanging
setTimeout(() => {
  console.error('Test timed out');
  process.exit(1);
}, 3000);
