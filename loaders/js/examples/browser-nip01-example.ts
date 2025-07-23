/**
 * Example demonstrating the improved NIP-01 compliant browser implementation
 * This shows how to load a cassette and subscribe to events with proper NIP-01 handling
 */

import { 
  CassetteManager, 
  loadCassetteFromArrayBuffer,
  subscribeToEvents,
  Cassette
} from '../../loaders/js/src/browser.js';

/**
 * Example 1: Using the CassetteManager class
 * This is the recommended approach for applications that need to manage multiple cassettes
 */
async function example1() {
  console.log('Example 1: Using CassetteManager');
  
  // Create a cassette manager with debugging enabled
  const manager = new CassetteManager();
  
  try {
    // Fetch a cassette file
    const response = await fetch('./test-cassette.wasm');
    const arrayBuffer = await response.arrayBuffer();
    
    // Load the cassette
    const cassette = await manager.loadCassetteFromArrayBuffer(arrayBuffer, 'test-cassette.wasm');
    
    if (!cassette) {
      console.error('Failed to load cassette');
      return;
    }
    
    console.log(`Loaded cassette: ${cassette.name}`);
    
    // Create a subscription request in NIP-01 format
    const subscriptionId = 'sub_' + Math.random().toString(36).substring(2, 15);
    const filters = { kinds: [1], limit: 10 };
    const subscriptionRequest = JSON.stringify(["REQ", subscriptionId, filters]);
    
    // Use the imported subscribeToEvents function instead of directly from manager
    const unsubscribe = subscribeToEvents(
      cassette,
      subscriptionRequest,
      (event: any, subId: string) => {
        console.log(`Received event for ${subId}:`, event);
      },
      {
        onEose: (subId: string) => {
          console.log(`End of stored events for ${subId}`);
        },
        onNotice: (notice: string, subId: string) => {
          console.log(`Notice for ${subId}: ${notice}`);
        }
      }
    );
    
    // Unsubscribe after 5 seconds
    setTimeout(() => {
      unsubscribe();
      console.log('Unsubscribed from events');
    }, 5000);
  } catch (error) {
    console.error('Error in example 1:', error);
  }
}

/**
 * Example 2: Using convenience functions
 * This approach is simpler for applications that only need to work with a single cassette
 */
async function example2() {
  console.log('Example 2: Using convenience functions');
  
  try {
    // Fetch a cassette file
    const response = await fetch('./test-cassette.wasm');
    const arrayBuffer = await response.arrayBuffer();
    
    // Load the cassette
    const cassette = await loadCassetteFromArrayBuffer(arrayBuffer, 'test-cassette.wasm', {
      debug: true,
      deduplicateEvents: true
    });
    
    if (!cassette) {
      console.error('Failed to load cassette');
      return;
    }
    
    console.log(`Loaded cassette: ${cassette.name}`);
    
    // Create a subscription request in NIP-01 format
    const subscriptionId = 'sub_' + Math.random().toString(36).substring(2, 15);
    const subscriptionRequest = ["REQ", subscriptionId, { kinds: [1], limit: 10 }];
    
    // Subscribe to events using the convenience function
    const unsubscribe = subscribeToEvents(
      cassette,
      subscriptionRequest,
      (event: any, subId: string) => {
        console.log(`Received event for ${subId}:`, event);
      },
      {
        onEose: (subId: string) => {
          console.log(`End of stored events for ${subId}`);
        },
        onNotice: (notice: string, subId: string) => {
          console.log(`Notice for ${subId}: ${notice}`);
        },
        debug: true
      }
    );
    
    // Process a direct request (useful for one-off requests)
    const directRequest = JSON.stringify(["REQ", "direct_" + Math.random().toString(36).substring(2, 15), { kinds: [1], limit: 1 }]);
    const directResponse = cassette.methods.req(directRequest);
    
    console.log('Direct request response:', directResponse);
    
    // Unsubscribe after 5 seconds
    setTimeout(() => {
      unsubscribe();
      console.log('Unsubscribed from events');
    }, 5000);
  } catch (error) {
    console.error('Error in example 2:', error);
  }
}

/**
 * Example 3: Error handling and response validation
 */
async function example3() {
  console.log('Example 3: Error handling and response validation');
  
  // Create a cassette manager
  const manager = new CassetteManager();
  
  try {
    // Create a simple validation function to check if a response is valid NIP-01 format
    function isValidNIP01Format(response: string): boolean {
      try {
        const parsed = JSON.parse(response);
        return (
          Array.isArray(parsed) && 
          parsed.length >= 2 && 
          (parsed[0] === "EVENT" || parsed[0] === "NOTICE" || parsed[0] === "EOSE" || parsed[0] === "OK")
        );
      } catch (e) {
        return false;
      }
    }
    
    // Load a cassette with both good and bad responses
    // (This is just a demonstration - normally you'd fetch a real cassette)
    const mockCassette: Cassette = {
      id: 'mock-cassette',
      name: 'Mock Cassette',
      description: 'A mock cassette for testing NIP-01 handling',
      version: '1.0.0',
      fileName: 'mock.wasm',
      methods: {
        describe: () => JSON.stringify({ name: 'Mock Cassette', description: 'A mock cassette for testing', version: '1.0.0' }),
        req: (request: string) => {
          // Parse the request to extract subscription ID
          const parsed = JSON.parse(request);
          const subId = Array.isArray(parsed) && parsed.length >= 2 ? parsed[1] : 'unknown';
          
          // Simulate different response formats based on the filter
          const filters = parsed[2] || {};
          
          if (filters.test === 'valid') {
            // Return a valid NIP-01 EVENT message
            return JSON.stringify(["EVENT", subId, {
              id: "abcdef123456",
              pubkey: "0123456789abcdef",
              created_at: Math.floor(Date.now() / 1000),
              kind: 1,
              tags: [],
              content: "This is a valid NIP-01 event",
              sig: "0123456789abcdef"
            }]);
          } else if (filters.test === 'invalid') {
            // Return an invalid response (not NIP-01 format)
            return "This is not a valid NIP-01 response";
          } else if (filters.test === 'structured') {
            // Return a structured response with events array
            return JSON.stringify({
              events: [
                {
                  id: "event1",
                  pubkey: "pubkey1",
                  created_at: Math.floor(Date.now() / 1000),
                  kind: 1,
                  tags: [],
                  content: "This is a structured event 1",
                  sig: "sig1"
                },
                {
                  id: "event2",
                  pubkey: "pubkey2",
                  created_at: Math.floor(Date.now() / 1000),
                  kind: 1,
                  tags: [],
                  content: "This is a structured event 2",
                  sig: "sig2"
                }
              ],
              eose: true
            });
          } else if (filters.test === 'corrupt') {
            // Return a corrupted NOTICE (missing opening bracket)
            return '"NOTICE", "This is a corrupted NOTICE message"';
          } else {
            // Return an empty array
            return "[]";
          }
        },
        close: (closeStr: string) => JSON.stringify(["NOTICE", "Subscription closed"]),
        getSchema: () => JSON.stringify({})
      },
      // Implement the required memory management methods
      getMemoryStats: () => ({
        allocatedPointers: [],
        allocationCount: 0,
        memory: {
          totalPages: 1,
          totalBytes: 64 * 1024,
          usageEstimate: 'No leaks detected'
        }
      }),
      dispose: () => ({ success: true, allocationsCleanedUp: 0 }),
      eventTracker: undefined
    };
    
    // Add the mock cassette to the manager
    manager.addCassette(mockCassette);
    
    // Test with different request types
    const tests = ['valid', 'invalid', 'structured', 'corrupt', 'empty'];
    
    for (const test of tests) {
      console.log(`\nTesting with ${test} response:`);
      
      const subscriptionId = `sub_${test}_${Math.random().toString(36).substring(2, 15)}`;
      const request = JSON.stringify(["REQ", subscriptionId, { test }]);
      
      const response = manager.processRequest(mockCassette.id, request);
      console.log(`- Response: ${response}`);
      
      // Validate the response
      if (response) {
        const isValid = isValidNIP01Format(response);
        console.log(`- Valid NIP-01 format: ${isValid}`);
        
        if (isValid) {
          console.log(`- Parsed: ${JSON.stringify(JSON.parse(response))}`);
        } else {
          console.log(`- Raw response: ${response}`);
        }
      } else {
        console.log(`- No response received`);
      }
    }
  } catch (error) {
    console.error('Error in example 3:', error);
  }
}

async function runExamples() {
  await example1();
  console.log('\n----------------------------\n');
  await example2();
  console.log('\n----------------------------\n');
  await example3();
}

// Run all examples
runExamples(); 