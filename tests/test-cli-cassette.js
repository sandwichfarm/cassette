// Test script for CLI-generated cassette
import { loadCassette } from '../cassette-loader/dist/src/index.js';
import fs from 'fs';
import path from 'path';

async function testCassette() {
  console.log('====== CLI Cassette Test ======');
  
  // Load the CLI-generated cassette
  const cassettePath = '../cassettes/test_cassette.wasm';
  console.log(`Loading cassette from ${cassettePath}`);
  
  // Check if the file exists
  if (fs.existsSync(cassettePath)) {
    const stats = fs.statSync(cassettePath);
    console.log(`Cassette file exists, size: ${stats.size} bytes`);
  } else {
    console.error('Cassette file not found!');
    process.exit(1);
  }
  
  console.log('Loading cassette in debug mode...');
  const result = await loadCassette(cassettePath, 'test_cassette.wasm', { debug: true });
  
  if (!result.success) {
    console.error(`Failed to load cassette: ${result.error}`);
    process.exit(1);
  }
  
  const cassette = result.cassette;
  console.log('Cassette loaded successfully!');
  
  // Test a request
  console.log('\n--- Testing request: ["REQ","test1",{"kinds":[1]}] ---');
  const response = cassette.methods.req('["REQ","test1",{"kinds":[1]}]');
  console.log(`Response: ${response.substring(0, 100)}${response.length > 100 ? '...' : ''}`);
  
  try {
    const parsedResponse = JSON.parse(response);
    console.log('Parsed response successfully!');
    if (Array.isArray(parsedResponse) && parsedResponse[0] === 'EVENT') {
      console.log('Valid EVENT response');
    } else {
      console.log('Unexpected response format:', parsedResponse[0]);
    }
  } catch (e) {
    console.log(`Failed to parse response: ${e}`);
  }
  
  // Test a close request
  console.log('\nClosing cassette...');
  const closeResponse = cassette.methods.close('["CLOSE","test1"]');
  console.log(`Close response: ${closeResponse}`);
  
  console.log('\n====== Test Complete ======');
}

testCassette().catch(console.error); 