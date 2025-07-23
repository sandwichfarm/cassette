#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { loadCassette } from '../cassette-loader/dist/src/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function main() {
  console.log('Testing generated cassette...');
  
  const cassettePath = path.join(__dirname, '..', 'cassettes', 'test_cassette_direct.wasm');
  
  // Check if cassette exists
  if (!fs.existsSync(cassettePath)) {
    console.error(`Cassette file not found: ${cassettePath}`);
    process.exit(1);
  }
  
  console.log(`Loading cassette from: ${cassettePath}`);
  console.log(`File size: ${fs.statSync(cassettePath).size} bytes`);
  
  // Load the cassette
  const result = await loadCassette(cassettePath, 'test_cassette_direct.wasm', { debug: true });
  
  if (!result.success) {
    console.error(`Failed to load cassette: ${result.error}`);
    process.exit(1);
  }
  
  const cassette = result.cassette;
  console.log('Cassette loaded successfully!');
  console.log('Cassette description:', cassette.description);
  
  // Test various requests
  const testRequests = [
    ['REQ', 'test1', { kinds: [1] }],
    ['REQ', 'test2', { '#e': ['6a95d9db85f2ca6e4d5d15809cd0ab35b21b53c37bf7e487c1cb0a56601d2476'] }],
    ['REQ', 'test3', { since: 1677551273 }],
    ['REQ', 'test4', { authors: ['e771af0b05c8e95fcdf6feb3500544d2fb1ccd384788e9f490bb3ee28e8ed66f'] }],
    ['REQ', 'test5', { '#t': ['value1', 'value2'] }]
  ];
  
  console.log('\n--- Testing REQ requests ---');
  for (const req of testRequests) {
    const reqJson = JSON.stringify(req);
    console.log(`\nSending request: ${reqJson}`);
    
    try {
      const response = cassette.methods.req(reqJson);
      console.log('Response:', response);
      
      // Parse and display responses
      const responses = response.split('\n').filter(Boolean);
      for (const resp of responses) {
        try {
          const parsed = JSON.parse(resp);
          console.log('Parsed response:', parsed);
        } catch (e) {
          console.error('Failed to parse response JSON:', resp);
        }
      }
    } catch (error) {
      console.error('Error during request:', error);
    }
  }
  
  // Test close
  console.log('\n--- Testing CLOSE request ---');
  const closeReq = ['CLOSE', 'test1'];
  const closeReqJson = JSON.stringify(closeReq);
  console.log(`Sending close request: ${closeReqJson}`);
  
  try {
    const closeResponse = cassette.methods.close(closeReqJson);
    console.log('Close response:', closeResponse);
  } catch (error) {
    console.error('Error during close:', error);
  }
  
  console.log('\nTest completed!');
}

main().catch(console.error); 