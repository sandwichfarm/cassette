/**
 * Browser-specific exports for cassette-loader
 */
export { CassetteManager } from '../examples/browser-integration.js';

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
  CassetteSource
} from './types.js'; 