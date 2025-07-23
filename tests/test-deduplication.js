// Test script for event deduplication (ensuring the same event isn't sent twice)
const WebSocket = require('ws');

// Track the events we've seen
const seenEvents = new Map();
let duplicateFound = false;
let subscriptionCount = 0;
const maxSubscriptions = 2;
let currentSubscription = 1;

// Connect to the relay
const ws = new WebSocket('ws://localhost:3001');

function sendSubscription(id) {
  // Create a subscription for kind 1 events with a limit of 5
  const request = ["REQ", `sub${id}`, {"kinds": [1], "limit": 5}];
  console.log(`Sending subscription ${id}:`, JSON.stringify(request));
  ws.send(JSON.stringify(request));
  subscriptionCount++;
}

// On connection, send first subscription
ws.on('open', function open() {
  console.log('Connected to relay');
  sendSubscription(currentSubscription);
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
    
    // Track EOSE messages to know when we're done
    if (message[0] === 'EOSE') {
      const subId = message[1];
      console.log(`EOSE received for subscription: ${subId}`);
      
      // Check if we've received all EOSEs
      if (--subscriptionCount <= 0) {
        // All subscriptions are done
        checkResults();
      } else if (currentSubscription < maxSubscriptions) {
        // Send next subscription
        currentSubscription++;
        sendSubscription(currentSubscription);
      }
      return;
    }
    
    // Track events by ID and subscription
    if (message[0] === 'EVENT') {
      const subId = message[1];
      const event = message[2];
      
      if (event && event.id) {
        const eventId = event.id;
        console.log(`Received event ${eventId} for sub ${subId}`);
        
        // Check if we've seen this event on this subscription before
        const key = `${eventId}:${subId}`;
        if (seenEvents.has(key)) {
          console.log(`DUPLICATE: Event ${eventId} seen twice on subscription ${subId}`);
          duplicateFound = true;
        } else {
          seenEvents.set(key, true);
        }
      }
    }
  } catch (err) {
    console.error('Error parsing message:', err);
  }
});

function checkResults() {
  if (duplicateFound) {
    console.log('❌ Deduplication test failed: Duplicates found within subscriptions');
    ws.close();
    process.exit(1);
  } else {
    console.log('✅ No duplicates detected within subscriptions - Deduplication is working');
    ws.close();
    process.exit(0);
  }
}

// Handle errors
ws.on('error', function error(err) {
  console.error('WebSocket error:', err);
  process.exit(1);
});

// Set timeout to prevent hanging
setTimeout(() => {
  console.error('Test timed out');
  process.exit(1);
}, 10000); // Increased timeout to 10 seconds 