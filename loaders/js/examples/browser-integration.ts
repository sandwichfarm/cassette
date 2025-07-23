/**
 * Example showing how to integrate the cassette-loader in a browser environment
 * 
 * This file demonstrates how to use the CassetteManager exported from the library
 * rather than implementing its own version.
 */
import { loadCassette, isWebAssemblySupported, ENV_INFO } from '../src/index.js';
import { CassetteManager } from '../src/browser.js';

// Example usage of the CassetteManager
async function exampleUsage() {
  // Create a cassette manager instance
  const manager = new CassetteManager({
    debug: true, // Enable debug logs
  });
  
  // Log the environment info
  console.log("Environment:", ENV_INFO);
  
  // Example: Load a cassette from URL
  try {
    const cassette = await manager.loadCassetteFromUrl('path/to/your/cassette.wasm');
    if (cassette) {
      console.log(`Successfully loaded cassette: ${cassette.name} (${cassette.id})`);
      
      // Process a request through the cassette
      const response = manager.processRequest(cassette.id, '["REQ", "sub1", {"kinds": [1]}]');
      console.log('Response:', response);
    }
  } catch (error) {
    console.error('Error loading cassette:', error);
  }
}

// Export the example usage function to make it accessible
export { exampleUsage };

// Export the CassetteManager for backward compatibility with code that might
// have imported it from here directly
export { CassetteManager };

/**
 * This example shows how you would use the CassetteManager in a real application:
 * 
 * ```javascript
 * import { CassetteManager } from 'cassette-loader/browser';
 * 
 * const manager = new CassetteManager({
 *   debug: true
 * });
 * 
 * // Load a cassette
 * async function loadCassette() {
 *   const cassette = await manager.loadCassetteFromUrl('path/to/cassette.wasm');
 *   if (cassette) {
 *     console.log(`Loaded: ${cassette.name}`);
 *     return cassette;
 *   }
 *   return null;
 * }
 * 
 * // Process a request
 * function processRequest(cassetteId, request) {
 *   return manager.processRequest(cassetteId, request);
 * }
 * ```
 */ 