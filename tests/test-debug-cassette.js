// Test all cassettes with direct loading

import { loadCassette } from '../loaders/js/dist/src/index.js';
import path from 'path';
import fs from 'fs';

console.log("====== Cassette Testing Suite ======");

async function testCassette(cassettePath) {
  console.log(`\n--- Testing Cassette: ${cassettePath} ---`);
  
  // Make sure it exists
  if (!fs.existsSync(cassettePath)) {
    console.error(`Error: Cassette file not found at ${cassettePath}`);
    return false;
  }
  
  console.log(`Cassette file exists, size: ${fs.statSync(cassettePath).size} bytes`);
  
  try {
    // Load the cassette
    console.log("Loading cassette in debug mode...");
    const result = await loadCassette(cassettePath, path.basename(cassettePath), {
      debug: true,
    });
    
    if (!result.success) {
      console.error(`Error loading cassette: ${result.error}`);
      return false;
    }
    
    const cassette = result.cassette;
    console.log("Cassette loaded successfully!");

    // Get description
    try {
      const describeResponse = cassette.methods.describe();
      console.log("Describe response:", describeResponse.substring(0, 250) + (describeResponse.length > 250 ? '...' : ''));
      
      try {
        const describeJson = JSON.parse(describeResponse);
        console.log("Parsed description:", {
          name: describeJson.name,
          description: describeJson.description,
          version: describeJson.version,
          author: describeJson.author,
          event_count: describeJson.event_count
        });
      } catch (e) {
        console.error("Failed to parse describe JSON:", e.message);
      }
    } catch (e) {
      console.error("Error calling describe:", e.message);
    }

    // Test sending various request formats
    const testRequests = [
      ["REQ", "test1", { kinds: [1] }],
      ["REQ", "test2", { "#e": ["6a95d9db85f2ca6e4d5d15809cd0ab35b21b53c37bf7e487c1cb0a56601d2476"] }],
      ["REQ", "test3", { since: 1677551273 }],
      ["REQ", "test4", { authors: ["e771af0b05c8e95fcdf6feb3500544d2fb1ccd384788e9f490bb3ee28e8ed66f"] }],
      ["REQ", "test5", { "#t": ["value1", "value2"] }]
    ];
    
    let success = true;
    
    for (const request of testRequests) {
      console.log(`\n--- Testing request: ${JSON.stringify(request)} ---`);
      const requestStr = JSON.stringify(request);
      
      try {
        const response = cassette.methods.req(requestStr);
        console.log(`Response: ${response.substring(0, 250)}${response.length > 250 ? '...' : ''}`);
        
        try {
          const parsed = JSON.parse(response);
          if (parsed[0] === "NOTICE") {
            console.error(`Error response: ${parsed[1]}`);
            success = false;
          } else {
            console.log(`Parsed response type: ${parsed[0]}`);
          }
        } catch (e) {
          console.error(`Failed to parse response: ${e.message}`);
          success = false;
        }
      } catch (e) {
        console.error(`Error calling req: ${e.message}`);
        success = false;
      }
    }
    
    // Close the cassette
    console.log("\nClosing cassette...");
    try {
      if (cassette.methods.close) {
        const closeResult = cassette.methods.close(JSON.stringify(["CLOSE", "test1"]));
        console.log(`Close result: ${closeResult}`);
      } else {
        console.log("Close method not available");
      }
    } catch (e) {
      console.error("Error closing cassette:", e.message);
    }
    
    return success;
    
  } catch (error) {
    console.error("Error loading cassette:", error);
    return false;
  }
}

async function main() {
  // Find all cassettes
  const cassettesDir = '../cassettes';
  const cassettes = fs.readdirSync(cassettesDir)
    .filter(file => file.endsWith('.wasm'))
    .map(file => path.join(cassettesDir, file));
  
  console.log(`Found ${cassettes.length} cassettes to test:`);
  cassettes.forEach(c => console.log(`- ${c}`));
  
  let totalSuccess = 0;
  let totalFailure = 0;
  
  for (const cassette of cassettes) {
    const success = await testCassette(cassette);
    if (success) {
      totalSuccess++;
    } else {
      totalFailure++;
    }
  }
  
  console.log("\n====== Test Results ======");
  console.log(`Total cassettes: ${cassettes.length}`);
  console.log(`Successful: ${totalSuccess}`);
  console.log(`Failed: ${totalFailure}`);
}

main().catch(err => {
  console.error("Unhandled error:", err);
  process.exit(1);
}); 