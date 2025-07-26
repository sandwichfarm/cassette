/**
 * Cassette Loader - A cross-platform library for loading Nostr cassettes
 * 
 * This library provides a standardized way to load and interact with
 * WebAssembly-based Nostr cassettes in both Node.js and browser environments.
 */

// Import types
import {
  Cassette,
  CassetteLoaderOptions,
  CassetteLoadError,
  CassetteLoadResult,
  CassetteMetadata,
  CassetteSource,
  EventTracker
} from './types.js';

// Export error classes
export { CassetteLoadError } from './types.js';

// Export the main functionality
export { loadCassette } from './loader.js';

// Export utility functions
export {
  isBrowser,
  isNode,
  generateCassetteId,
  createLogger,
  createEventTracker
} from './utils.js';

// Re-export types for consumers
export type {
  Cassette,
  CassetteLoaderOptions,
  CassetteLoadResult,
  CassetteMetadata,
  CassetteSource,
  EventTracker
};

/**
 * Version of the cassette-loader package
 */
export const VERSION = '1.1.0';

/**
 * Check if WebAssembly is supported in the current environment
 */
export function isWebAssemblySupported(): boolean {
  return typeof WebAssembly === 'object' && 
         typeof WebAssembly.compile === 'function' && 
         typeof WebAssembly.instantiate === 'function';
}

/**
 * Information about the environment
 */
export const ENV_INFO = {
  isNode: typeof process !== 'undefined' && 
          process.versions != null && 
          process.versions.node != null,
  isBrowser: typeof window !== 'undefined' && typeof document !== 'undefined',
  webAssembly: isWebAssemblySupported(),
  version: VERSION
}; 