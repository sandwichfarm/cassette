/**
 * Browser-specific exports for cassette-loader
 */
import { Cassette, CassetteLoaderOptions } from './types.js';
import { loadCassette } from './index.js';
import { CassetteManager } from './manager.js';

// Export the CassetteManager from the core library, not the example
export { CassetteManager };

// Re-export the core functionality for convenience
export {
  loadCassette,
  isWebAssemblySupported,
  ENV_INFO,
  VERSION
} from './index.js';

// Re-export the types
export type {
  Cassette,
  CassetteMetadata,
  CassetteLoaderOptions,
  CassetteLoadResult,
  CassetteSource,
  CassetteMemoryStats
} from './types.js';

// Helper functions for browser interfaces
/**
 * Load a cassette from an ArrayBuffer
 * @param arrayBuffer The array buffer containing the WebAssembly binary
 * @param fileName Optional file name for the cassette
 * @param options Optional loader options
 * @returns Promise with the cassette load result
 */
export async function loadCassetteFromArrayBuffer(
  arrayBuffer: ArrayBuffer, 
  fileName?: string, 
  options?: CassetteLoaderOptions
): Promise<Cassette | null> {
  const result = await loadCassette(arrayBuffer, fileName, options);
  return result.success && result.cassette ? result.cassette : null;
}

/**
 * Subscribe to events from a cassette
 * @param cassette The cassette to subscribe to
 * @param request The subscription request
 * @param onEvent Callback for events
 * @param options Additional options
 * @returns Function to unsubscribe
 */
export function subscribeToEvents(
  cassette: Cassette,
  request: any, 
  onEvent: (event: any, subId: string) => void,
  options?: {
    onEose?: (subId: string) => void,
    onNotice?: (notice: string, subId: string) => void,
    debug?: boolean
  }
): () => void {
  // Convert request to string if it's not already
  const requestStr = typeof request === 'string' ? request : JSON.stringify(request);
  
  // Determine subscription ID from request
  let subId = 'unknown';
  try {
    const parsed = JSON.parse(requestStr);
    if (Array.isArray(parsed) && parsed.length >= 2) {
      subId = parsed[1];
    }
  } catch (e) {
    console.error('Invalid subscription request:', e);
  }
  
  // Process the request
  const response = cassette.methods.req(requestStr);
  
  // Process response
  try {
    const parsed = JSON.parse(response);
    
    // Handle different response types
    if (Array.isArray(parsed)) {
      if (parsed[0] === 'EVENT' && parsed.length >= 3) {
        onEvent(parsed[2], subId);
      } else if (parsed[0] === 'EOSE' && options?.onEose) {
        options.onEose(subId);
      } else if (parsed[0] === 'NOTICE' && options?.onNotice) {
        options.onNotice(parsed[1], subId);
      }
    }
  } catch (e) {
    if (options?.debug) {
      console.error('Error processing response:', e);
    }
    
    if (options?.onNotice) {
      options.onNotice(`Error processing response: ${e}`, subId);
    }
  }
  
  // Return unsubscribe function
  return () => {
    if (cassette.methods.close) {
      cassette.methods.close(JSON.stringify(['CLOSE', subId]));
    }
  };
} 