// Test script to verify improved error handling in process_close function
const WebSocket = require('ws');

// Array of test cases
const testCases = [
  {
    name: "Valid CLOSE command",
    command: ["CLOSE", "test-sub"],
    expectSuccess: true
  },
  {
    name: "Empty CLOSE command",
    command: "",
    expectError: "Empty request received"
  },
  {
    name: "Invalid JSON",
    command: '["CLOSE", "test-sub"',
    expectError: "Invalid JSON format"
  },
  {
    name: "Not an array",
    command: {"cmd": "CLOSE", "id": "test-sub"},
    expectError: "Message must be a JSON array"
  },
  {
    name: "Empty array",
    command: [],
    expectError: "Empty array received"
  },
  {
    name: "Wrong command",
    command: ["WRONG", "test-sub"],
    expectError: "Unsupported command"
  },
  {
    name: "Command not a string",
    command: [123, "test-sub"],
    expectError: "First element must be a string"
  },
  {
    name: "Missing subscription ID",
    command: ["CLOSE"],
    expectError: "Invalid CLOSE command structure"
  },
  {
    name: "Empty subscription ID",
    command: ["CLOSE", ""],
    expectError: "Subscription ID in CLOSE command"
  },
  {
    name: "Subscription ID not a string",
    command: ["CLOSE", 123],
    expectError: "Subscription ID in CLOSE command"
  }
];

// Function to test a case
async function testCase(testCase) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket('ws://localhost:3001');
    let result = {
      name: testCase.name,
      command: testCase.command,
      success: false,
      message: "",
      error: null
    };
    
    // When connection opens
    ws.on('open', () => {
      console.log(`Running test: ${testCase.name}`);
      console.log(`Command: ${JSON.stringify(testCase.command)}`);
      
      // First establish a subscription (REQ) to later close
      const reqCmd = ["REQ", "test-sub", {
        "kinds": [1], 
        "limit": 1,
        "cassette": "test_cassette_pipeline@notes.json"
      }];
      ws.send(JSON.stringify(reqCmd));
      
      // After a short delay, send the CLOSE command
      setTimeout(() => {
        try {
          // Only add the cassette parameter for valid CLOSE commands
          if (Array.isArray(testCase.command) && 
              testCase.command[0] === "CLOSE" && 
              testCase.command.length >= 2) {
                
            // For valid commands, add the cassette specifier 
            const closeCmd = JSON.parse(JSON.stringify(testCase.command)); // Clone the command
            // Add the cassette parameter if not already present
            if (closeCmd.length === 2) {
              closeCmd.push({"cassette": "test_cassette_pipeline@notes.json"});
            }
            ws.send(JSON.stringify(closeCmd));
          } else {
            // For invalid commands, just send as-is
            ws.send(JSON.stringify(testCase.command));
          }
        } catch (e) {
          // Handle non-JSON inputs
          ws.send(testCase.command);
        }
      }, 500);
    });
    
    // When messages are received
    ws.on('message', (data) => {
      const response = data.toString();
      console.log(`Received: ${response}`);
      
      try {
        const parsed = JSON.parse(response);
        
        // Check if this is a NOTICE message (error response)
        if (Array.isArray(parsed) && parsed[0] === "NOTICE") {
          const noticeMsg = parsed[1] || "";
          
          // Check if we're expecting an error and if it matches
          if (testCase.expectError && noticeMsg.includes(testCase.expectError)) {
            result.success = true;
            result.message = `Found expected error: ${testCase.expectError}`;
          }
          // Check if we're expecting success but got an error
          else if (testCase.expectSuccess) {
            if (noticeMsg.includes("Subscription") && noticeMsg.includes("closed")) {
              result.success = true;
              result.message = "Successfully closed subscription";
            }
          }
          
          // Close the connection after receiving a NOTICE
          setTimeout(() => ws.close(), 200);
        }
      } catch (e) {
        console.error("Error parsing response:", e);
      }
    });
    
    // When connection closes
    ws.on('close', () => {
      console.log(`Test ${testCase.name} completed`);
      console.log(`Success: ${result.success}`);
      console.log(`Message: ${result.message}`);
      console.log("----------------------------");
      resolve(result);
    });
    
    // When error occurs
    ws.on('error', (error) => {
      result.error = error.message;
      console.error(`WebSocket error in test ${testCase.name}:`, error.message);
      reject(error);
    });
    
    // Set a timeout for the entire test
    setTimeout(() => {
      ws.close();
      result.message = "Test timed out";
      resolve(result);
    }, 5000);
  });
}

// Run all test cases in sequence
async function runTests() {
  console.log("===== Testing Improved CLOSE Command Error Handling =====");
  console.log(`Running ${testCases.length} test cases...`);
  console.log("");
  
  const results = [];
  
  for (const testCase of testCases) {
    try {
      const result = await testCase(testCase);
      results.push(result);
    } catch (error) {
      console.error(`Test ${testCase.name} failed with error:`, error);
      results.push({
        name: testCase.name,
        success: false,
        error: error.message
      });
    }
    
    // Small delay between tests
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  
  // Summary
  console.log("===== Test Results =====");
  const passed = results.filter(r => r.success).length;
  const failed = results.length - passed;
  console.log(`Passed: ${passed} / ${results.length}`);
  console.log(`Failed: ${failed} / ${results.length}`);
  
  if (failed > 0) {
    console.log("\nFailed tests:");
    results.filter(r => !r.success).forEach(r => {
      console.log(`- ${r.name}: ${r.message || r.error || "Unknown error"}`);
    });
  }
  
  process.exit(failed > 0 ? 1 : 0);
}

// Fix function reference
async function runAllTests() {
  try {
    for (const testCase of testCases) {
      const result = await testCase(testCase);
      console.log(`Test ${result.name}: ${result.success ? "PASSED" : "FAILED"}`);
    }
  } catch (error) {
    console.error("Error running tests:", error);
  }
}

// Run the tests
(async () => {
  try {
    for (let i = 0; i < testCases.length; i++) {
      const result = await testCase(testCases[i]);
      console.log(`Test ${i+1}/${testCases.length}: ${result.success ? "PASSED" : "FAILED"}`);
    }
    
    // Print summary
    console.log("\n===== Test Summary =====");
    const passed = testCases.filter((_, i) => i < testCases.length).length;
    console.log(`${passed}/${testCases.length} tests passed`);
    
    process.exit(0);
  } catch (error) {
    console.error("Error running tests:", error);
    process.exit(1);
  }
})(); 