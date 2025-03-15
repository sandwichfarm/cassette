#!/usr/bin/env bun

/**
 * This script manually updates the WASM bindings to include new methods
 * from sandwichs-favs if wasm-bindgen isn't available.
 * 
 * This is a fallback solution and not as complete as using wasm-bindgen.
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// Get the directory where this script is located
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const boomboxDir = path.resolve(__dirname, '..');
const wasmDir = path.resolve(boomboxDir, 'wasm');

console.log('Manually updating WASM bindings...');

// Path to the TypeScript definition file
const dtsFilePath = path.resolve(wasmDir, 'sandwichs_favs.d.ts');

// Read the methods from the Rust implementation
// This requires the user to input the method names
console.log('Please enter the names of the methods to add to the bindings (comma-separated):');
process.stdin.once('data', (data) => {
  const methodNames = data.toString().trim().split(',').map(m => m.trim());
  
  if (methodNames.length === 0 || methodNames[0] === '') {
    console.log('No methods specified. Exiting.');
    process.exit(0);
  }
  
  console.log(`Adding methods: ${methodNames.join(', ')}`);
  
  // Update the TypeScript definition file
  updateTypeScriptDefinitions(dtsFilePath, methodNames);
  
  // Update the JavaScript files
  updateJavaScriptBindings(
    path.resolve(wasmDir, 'sandwichs_favs.js'),
    methodNames
  );
  
  updateBackgroundJavaScript(
    path.resolve(wasmDir, 'sandwichs_favs_bg.js'),
    methodNames
  );
  
  updateWasmTypeScriptDefinitions(
    path.resolve(wasmDir, 'sandwichs_favs_bg.wasm.d.ts'),
    methodNames
  );
  
  console.log('WASM bindings updated successfully!');
  process.exit(0);
});

/**
 * Updates the TypeScript definition file to include new methods
 */
function updateTypeScriptDefinitions(filePath, methodNames) {
  try {
    let content = fs.readFileSync(filePath, 'utf8');
    
    // Find the class definition
    const classRegex = /export class SandwichsFavs \{[\s\S]*?\}/;
    const classMatch = content.match(classRegex);
    
    if (classMatch) {
      let classContent = classMatch[0];
      const closingBraceIndex = classContent.lastIndexOf('}');
      
      // Add new methods to the class
      let newMethods = '';
      for (const methodName of methodNames) {
        if (!classContent.includes(`static ${methodName}()`)) {
          newMethods += `  static ${methodName}(): string;\n  `;
        }
      }
      
      // Insert new methods before the closing brace
      if (newMethods) {
        const updatedClassContent = 
          classContent.substring(0, closingBraceIndex) + 
          newMethods + 
          classContent.substring(closingBraceIndex);
        
        content = content.replace(classRegex, updatedClassContent);
      }
      
      // Find the InitOutput interface
      const interfaceRegex = /export interface InitOutput \{[\s\S]*?\}/;
      const interfaceMatch = content.match(interfaceRegex);
      
      if (interfaceMatch) {
        let interfaceContent = interfaceMatch[0];
        const closingBraceIndex = interfaceContent.lastIndexOf('}');
        
        // Add new methods to the interface
        let newMethods = '';
        for (const methodName of methodNames) {
          const wasmMethodName = `sandwichsfavs_${methodName}`;
          if (!interfaceContent.includes(wasmMethodName)) {
            newMethods += `  readonly ${wasmMethodName}: () => [number, number];\n  `;
          }
        }
        
        // Insert new methods before the closing brace
        if (newMethods) {
          const updatedInterfaceContent = 
            interfaceContent.substring(0, closingBraceIndex) + 
            newMethods + 
            interfaceContent.substring(closingBraceIndex);
          
          content = content.replace(interfaceRegex, updatedInterfaceContent);
        }
      }
      
      fs.writeFileSync(filePath, content);
      console.log(`Updated TypeScript definitions in ${filePath}`);
    } else {
      console.error('Could not find the SandwichsFavs class in the TypeScript definitions');
    }
  } catch (error) {
    console.error(`Error updating TypeScript definitions: ${error.message}`);
  }
}

/**
 * Updates the JavaScript file to include new methods
 */
function updateJavaScriptBindings(filePath, methodNames) {
  try {
    let content = fs.readFileSync(filePath, 'utf8');
    
    // Find the class definition
    const classRegex = /export class SandwichsFavs \{[\s\S]*?\}/;
    const classMatch = content.match(classRegex);
    
    if (classMatch) {
      let classContent = classMatch[0];
      const closingBraceIndex = classContent.lastIndexOf('}');
      
      // Add new methods to the class
      let newMethods = '';
      for (const methodName of methodNames) {
        if (!classContent.includes(`static ${methodName}()`)) {
          newMethods += `
    /**
     * @returns {string}
     */
    static ${methodName}() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.sandwichsfavs_${methodName}();
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
`;
        }
      }
      
      // Insert new methods before the closing brace
      if (newMethods) {
        const updatedClassContent = 
          classContent.substring(0, closingBraceIndex) + 
          newMethods + 
          classContent.substring(closingBraceIndex);
        
        content = content.replace(classRegex, updatedClassContent);
      }
      
      fs.writeFileSync(filePath, content);
      console.log(`Updated JavaScript bindings in ${filePath}`);
    } else {
      console.error('Could not find the SandwichsFavs class in the JavaScript bindings');
    }
  } catch (error) {
    console.error(`Error updating JavaScript bindings: ${error.message}`);
  }
}

/**
 * Updates the background JavaScript file to include new methods
 */
function updateBackgroundJavaScript(filePath, methodNames) {
  try {
    let content = fs.readFileSync(filePath, 'utf8');
    
    // Find the class definition
    const classRegex = /export class SandwichsFavs \{[\s\S]*?\}/;
    const classMatch = content.match(classRegex);
    
    if (classMatch) {
      let classContent = classMatch[0];
      const closingBraceIndex = classContent.lastIndexOf('}');
      
      // Add new methods to the class
      let newMethods = '';
      for (const methodName of methodNames) {
        if (!classContent.includes(`static ${methodName}()`)) {
          newMethods += `
    /**
     * @returns {string}
     */
    static ${methodName}() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.sandwichsfavs_${methodName}();
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
`;
        }
      }
      
      // Insert new methods before the closing brace
      if (newMethods) {
        const updatedClassContent = 
          classContent.substring(0, closingBraceIndex) + 
          newMethods + 
          classContent.substring(closingBraceIndex);
        
        content = content.replace(classRegex, updatedClassContent);
      }
      
      fs.writeFileSync(filePath, content);
      console.log(`Updated background JavaScript in ${filePath}`);
    } else {
      console.error('Could not find the SandwichsFavs class in the background JavaScript');
    }
  } catch (error) {
    console.error(`Error updating background JavaScript: ${error.message}`);
  }
}

/**
 * Updates the WASM TypeScript definition file to include new exported functions
 */
function updateWasmTypeScriptDefinitions(filePath, methodNames) {
  try {
    let content = fs.readFileSync(filePath, 'utf8');
    let lines = content.split('\n');
    
    // Find the last export line
    let lastExportIndex = -1;
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].trim().startsWith('export const')) {
        lastExportIndex = i;
      }
    }
    
    if (lastExportIndex !== -1) {
      let newExports = [];
      
      for (const methodName of methodNames) {
        const wasmMethodName = `sandwichsfavs_${methodName}`;
        if (!content.includes(wasmMethodName)) {
          newExports.push(`export const ${wasmMethodName}: () => [number, number];`);
        }
      }
      
      if (newExports.length > 0) {
        // Insert new exports after the last export line
        lines.splice(lastExportIndex + 1, 0, ...newExports);
        fs.writeFileSync(filePath, lines.join('\n'));
        console.log(`Updated WASM TypeScript definitions in ${filePath}`);
      }
    } else {
      console.error('Could not find export lines in the WASM TypeScript definitions');
    }
  } catch (error) {
    console.error(`Error updating WASM TypeScript definitions: ${error.message}`);
  }
} 