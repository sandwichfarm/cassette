#!/usr/bin/env node
/**
 * Cross-language test to verify JS and Python loaders produce same results
 */

import { loadCassette } from '../cassette-loader/dist/src/index.js';
import { spawn } from 'child_process';
import fs from 'fs/promises';

async function testJSLoader(wasmPath) {
    console.log('🟦 Testing JavaScript loader...');
    
    const wasmBuffer = await fs.readFile(wasmPath);
    const result = await loadCassette(wasmBuffer, 'test-cassette');
    
    if (!result.success) {
        throw new Error(`JS loader failed: ${result.error}`);
    }
    
    const cassette = result.cassette;
    console.log(`   Name: ${cassette.name}`);
    console.log(`   Version: ${cassette.version}`);
    
    // Test REQ
    const req = JSON.stringify(["REQ", "sub1", {"kinds": [1], "limit": 5}]);
    const response = cassette.methods.req(req);
    console.log(`   REQ response: ${response.substring(0, 100)}...`);
    
    // Get memory stats
    const stats = cassette.getMemoryStats();
    console.log(`   Memory allocations: ${stats.allocationCount}`);
    
    cassette.dispose();
    
    return response;
}

async function testPythonLoader(wasmPath) {
    console.log('\n🟨 Testing Python loader...');
    
    return new Promise((resolve, reject) => {
        const pythonScript = `
import sys
sys.path.insert(0, '../cassette-loader-py')
from cassette_loader import load_cassette
import json

with open('${wasmPath}', 'rb') as f:
    wasm_bytes = f.read()

result = load_cassette(wasm_bytes, 'test-cassette')
if result['success']:
    cassette = result['cassette']
    print(f"   Name: {cassette.info.name}")
    print(f"   Version: {cassette.info.version}")
    
    req = json.dumps(["REQ", "sub1", {"kinds": [1], "limit": 5}])
    response = cassette.req(req)
    print(f"   REQ response: {response[:100]}...")
    
    stats = cassette.get_memory_stats()
    print(f"   Memory allocations: {stats.allocation_count}")
    
    cassette.dispose()
    print("RESPONSE:" + response)
else:
    print(f"Failed: {result['error']}")
    sys.exit(1)
`;
        
        const proc = spawn('../cassette-loader-py/venv/bin/python', ['-c', pythonScript], {
            cwd: process.cwd()
        });
        
        let output = '';
        let response = '';
        
        proc.stdout.on('data', (data) => {
            const str = data.toString();
            if (str.includes('RESPONSE:')) {
                response = str.split('RESPONSE:')[1].trim();
            } else {
                output += str;
            }
        });
        
        proc.stderr.on('data', (data) => {
            console.error(`Python error: ${data}`);
        });
        
        proc.on('close', (code) => {
            if (code !== 0) {
                reject(new Error(`Python process exited with code ${code}`));
            } else {
                console.log(output);
                resolve(response);
            }
        });
    });
}

async function main() {
    const wasmPath = process.argv[2];
    if (!wasmPath) {
        console.error('Usage: node test-cross-language.js <wasm-file>');
        process.exit(1);
    }
    
    console.log(`📊 Cross-language cassette loader test`);
    console.log(`   WASM file: ${wasmPath}\n`);
    
    try {
        const jsResponse = await testJSLoader(wasmPath);
        const pyResponse = await testPythonLoader(wasmPath);
        
        console.log('\n📋 Comparison:');
        
        // Parse responses
        const jsData = JSON.parse(jsResponse);
        const pyData = JSON.parse(pyResponse);
        
        // Compare type
        if (jsData[0] === pyData[0]) {
            console.log(`   ✅ Response type matches: ${jsData[0]}`);
        } else {
            console.log(`   ❌ Response type mismatch: JS=${jsData[0]}, Python=${pyData[0]}`);
        }
        
        // Compare content (for EVENT responses)
        if (jsData[0] === 'EVENT' && pyData[0] === 'EVENT') {
            const jsEvent = jsData[2];
            const pyEvent = pyData[2];
            
            if (jsEvent.id === pyEvent.id) {
                console.log(`   ✅ Event ID matches: ${jsEvent.id}`);
            } else {
                console.log(`   ❌ Event ID mismatch`);
            }
            
            if (jsEvent.content === pyEvent.content) {
                console.log(`   ✅ Event content matches`);
            } else {
                console.log(`   ❌ Event content mismatch`);
            }
        }
        
        console.log('\n✅ Cross-language test completed!');
        
    } catch (error) {
        console.error(`\n❌ Test failed: ${error.message}`);
        process.exit(1);
    }
}

main();