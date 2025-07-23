const fs = require('fs');
const path = require('path');
const { loadCassette } = require('../cassette-loader/dist/src/loader.js');

const CASSETTE_PATH = path.join(__dirname, '../boombox/cassettes/test_cassette_direct@notes.json.wasm');

// Test with different filter formats
async function testCassettes() {
  try {
    console.log(`Loading cassette from ${CASSETTE_PATH}`);
    
    // Check if the cassette file exists
    if (!fs.existsSync(CASSETTE_PATH)) {
      console.error(`Cassette file does not exist: ${CASSETTE_PATH}`);
      return;
    }
    
    // Load the cassette file
    const buffer = fs.readFileSync(CASSETTE_PATH);
    console.log(`Read ${buffer.byteLength} bytes from cassette file`);
    
    // Load the cassette
    const result = await loadCassette(new Uint8Array(buffer));
    console.log(`Cassette loaded: ${JSON.stringify(result.metadata)}`);
    
    const cassette = result.cassette;
    
    // Get the cassette description
    console.log(`\n--- Getting cassette description ---`);
    const description = cassette.methods.describe();
    console.log(`Cassette description: ${description}`);
    
    // Test NIP-01 request
    console.log(`\n--- Testing standard NIP-01 request ---`);
    const nip01Req = JSON.stringify(["REQ", "test1", {"kinds": [1], "limit": 5}]);
    console.log(`Sending request: ${nip01Req}`);
    const nip01Resp = cassette.methods.req(nip01Req);
    console.log(`Response: ${nip01Resp.slice(0, 200)}...`);
    console.log(`Response length: ${nip01Resp.length} characters`);
    
    // Test NIP-119 request with &t filter
    console.log(`\n--- Testing NIP-119 request with &t filter ---`);
    const nip119Req = JSON.stringify(["REQ", "test2", {"&t": ["value1", "value2"]}]);
    console.log(`Sending request: ${nip119Req}`);
    const nip119Resp = cassette.methods.req(nip119Req);
    console.log(`Response: ${nip119Resp.slice(0, 200)}...`);
    console.log(`Response length: ${nip119Resp.length} characters`);
    
    // Parse the response
    console.log(`\n--- Parsing NIP-119 response ---`);
    const responseLines = nip119Resp.trim().split('\n');
    for (const line of responseLines) {
      console.log(`Line: ${line}`);
      try {
        const parsed = JSON.parse(line);
        console.log(`Parsed: ${JSON.stringify(parsed)}`);
      } catch (error) {
        console.error(`Failed to parse line: ${error.message}`);
      }
    }
    
    // Close the cassette
    console.log(`\n--- Closing cassette ---`);
    const closeReq = JSON.stringify(["CLOSE", "test1"]);
    console.log(`Sending close: ${closeReq}`);
    const closeResp = cassette.methods.close(closeReq);
    console.log(`Close response: ${closeResp}`);
    
  } catch (error) {
    console.error('Error testing cassette:', error);
  }
}

// Run the test
testCassettes().catch(console.error); 