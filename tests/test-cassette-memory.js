/**
 * Debug script to test cassette memory management and NIP-01 interface
 * 
 * This script loads a test cassette and calls its methods to check for memory issues.
 * It uses verbose logging to trace memory allocations and deallocations.
 */

import { loadCassette } from '../cassette-loader/dist/src/index.js';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// Get current directory
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Configuration
const CASSETTE_PATH = path.join(__dirname, '..', 'cassettes', 'test-cassette.wasm');
console.log(`Using cassette at path: ${CASSETTE_PATH}`);

// Check if the cassette exists
if (!fs.existsSync(CASSETTE_PATH)) {
  console.error(`❌ Cassette not found: ${CASSETTE_PATH}`);
  console.error('Please create a test cassette first using the CLI:');
  console.error('cd cli && cargo run dub notes.json --name test-cassette');
  process.exit(1);
}

// NIP-01 test data
const TEST_DATA = {
  // A basic REQ command with a simple filter
  basicReq: JSON.stringify(['REQ', 'test-sub-1', { kinds: [1], limit: 3 }]),
  
  // A CLOSE command
  closeCommand: JSON.stringify(['CLOSE', 'test-sub-1']),
  
  // Invalid REQ - malformed JSON
  invalidReq: '["REQ", "sub3", { "kinds": [1], ',
};

/**
 * Main test function
 */
async function testCassette() {
  console.log('🔍 Starting memory management test...');
  
  try {
    // Load the cassette with debug logging enabled
    console.log('📂 Loading cassette...');
    const result = await loadCassette(CASSETTE_PATH, undefined, { 
      debug: true,
      memoryInitialSize: 16,
      exposeExports: true
    });
    
    if (!result.success) {
      console.error(`❌ Failed to load cassette: ${result.error}`);
      process.exit(1);
    }
    
    const cassette = result.cassette;
    console.log(`✅ Successfully loaded cassette: ${cassette.name}`);
    
    // Test 1: Call describe()
    console.log('\n📋 TEST 1: Calling describe() method...');
    try {
      const description = cassette.methods.describe();
      console.log('✅ describe() returned successfully');
      console.log(`📄 Description (first 100 chars): ${description.substring(0, 100)}...`);
      
      // Parse the description to check if it's valid JSON
      try {
        const parsedDesc = JSON.parse(description);
        console.log('✅ Description is valid JSON');
        console.log('📄 Metadata:', JSON.stringify(parsedDesc, null, 2));
      } catch (parseError) {
        console.error('❌ Failed to parse description as JSON:', parseError);
      }
    } catch (describeError) {
      console.error('❌ Error calling describe():', describeError);
    }
    
    // Test 2: Call req() with valid input
    console.log('\n📋 TEST 2: Calling req() method with valid input...');
    try {
      console.log('📄 Request:', TEST_DATA.basicReq);
      const reqResult = cassette.methods.req(TEST_DATA.basicReq);
      console.log('✅ req() returned successfully');
      console.log(`📄 Response (first 100 chars): ${reqResult.substring(0, 100)}...`);
      
      // Parse the response to check if it's valid JSON
      try {
        const parsedReq = JSON.parse(reqResult);
        console.log('✅ Response is valid JSON');
        console.log('📄 Response structure:', JSON.stringify(parsedReq, null, 2));
        
        // Check if response follows NIP-01 format
        if (Array.isArray(parsedReq)) {
          console.log('✅ Response is an array (NIP-01 format)');
          if (['EVENT', 'NOTICE', 'EOSE'].includes(parsedReq[0])) {
            console.log(`✅ First element is a valid NIP-01 message type: ${parsedReq[0]}`);
          } else {
            console.error(`❌ First element is not a valid NIP-01 message type: ${parsedReq[0]}`);
          }
        } else if (parsedReq.type && parsedReq.message) {
          console.log('✅ Response has type/message properties');
          console.log(`📄 Type: ${parsedReq.type}, Message: ${JSON.stringify(parsedReq.message)}`);
        } else {
          console.error('❌ Response does not follow NIP-01 format');
        }
      } catch (parseError) {
        console.error('❌ Failed to parse req response as JSON:', parseError);
      }
    } catch (reqError) {
      console.error('❌ Error calling req():', reqError);
    }
    
    // Test 3: Call req() with invalid input
    console.log('\n📋 TEST 3: Calling req() method with invalid input...');
    try {
      console.log('📄 Invalid Request:', TEST_DATA.invalidReq);
      const invalidResult = cassette.methods.req(TEST_DATA.invalidReq);
      console.log('✅ req() with invalid input returned successfully');
      console.log(`📄 Response: ${invalidResult}`);
      
      // Parse the response to check if it's valid JSON
      try {
        const parsedInvalid = JSON.parse(invalidResult);
        console.log('✅ Response is valid JSON');
        console.log('📄 Response structure:', JSON.stringify(parsedInvalid, null, 2));
      } catch (parseError) {
        console.error('❌ Failed to parse invalid req response as JSON:', parseError);
      }
    } catch (reqError) {
      console.error('❌ Error calling req() with invalid input:', reqError);
    }
    
    // Test 4: Call close() if available
    if (cassette.methods.close) {
      console.log('\n📋 TEST 4: Calling close() method...');
      try {
        console.log('📄 Close Request:', TEST_DATA.closeCommand);
        const closeResult = cassette.methods.close(TEST_DATA.closeCommand);
        console.log('✅ close() returned successfully');
        console.log(`📄 Response: ${closeResult}`);
        
        // Parse the response to check if it's valid JSON
        try {
          const parsedClose = JSON.parse(closeResult);
          console.log('✅ Response is valid JSON');
          console.log('📄 Response structure:', JSON.stringify(parsedClose, null, 2));
        } catch (parseError) {
          console.error('❌ Failed to parse close response as JSON:', parseError);
        }
      } catch (closeError) {
        console.error('❌ Error calling close():', closeError);
      }
    } else {
      console.log('\n📋 TEST 4: close() method not available');
    }
    
    // Test 5: Memory leak detection
    console.log('\n📋 TEST 5: Testing memory leak detection...');
    
    // Get current memory statistics
    const memStats = cassette.getMemoryStats();
    console.log('📊 Current memory statistics:');
    console.log(`   - Allocation count: ${memStats.allocationCount}`);
    console.log(`   - Memory pages: ${memStats.memory.totalPages}`);
    console.log(`   - Memory size: ${(memStats.memory.totalBytes / 1024 / 1024).toFixed(2)} MB`);
    console.log(`   - Status: ${memStats.memory.usageEstimate}`);
    
    // Create a controlled memory leak by making multiple requests without disposing
    console.log('\n📋 Creating artificial memory pressure with multiple requests...');
    const requestCount = 10;
    for (let i = 0; i < requestCount; i++) {
      // Modify the subscription ID to make each request unique
      const reqData = JSON.stringify(['REQ', `test-sub-${i}`, { kinds: [1], limit: 3 }]);
      cassette.methods.req(reqData);
      
      // Log memory stats every few requests
      if (i % 3 === 0 || i === requestCount - 1) {
        const currentStats = cassette.getMemoryStats();
        console.log(`   - After ${i+1} requests: ${currentStats.allocationCount} allocations`);
      }
    }
    
    // Display full memory stats after creating pressure
    const afterStats = cassette.getMemoryStats();
    console.log('\n📊 Memory statistics after test requests:');
    console.log(`   - Allocation count: ${afterStats.allocationCount}`);
    if (afterStats.allocationCount > 0) {
      console.log(`   - Allocated pointers: ${afterStats.allocatedPointers.slice(0, 5).join(', ')}${afterStats.allocatedPointers.length > 5 ? '...' : ''}`);
    }
    
    // Test cleanup
    console.log('\n📋 TEST 6: Testing memory cleanup...');
    const disposeResult = cassette.dispose();
    console.log(`✅ Disposed cassette, cleaned up ${disposeResult.allocationsCleanedUp} allocations`);
    
    // Check if cleanup was successful
    const finalStats = cassette.getMemoryStats();
    console.log('\n📊 Final memory statistics:');
    console.log(`   - Remaining allocations: ${finalStats.allocationCount}`);
    
    if (finalStats.allocationCount === 0) {
      console.log('✅ Memory cleanup successful, no leaks detected');
    } else {
      console.warn(`⚠️ Some allocations could not be cleaned up: ${finalStats.allocationCount} remaining`);
    }
    
    console.log('\n✅ Memory management test completed!');
  } catch (error) {
    console.error('❌ Unexpected error during test:', error);
    process.exit(1);
  }
}

// Run the test
testCassette(); 