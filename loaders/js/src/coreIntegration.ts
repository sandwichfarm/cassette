/**
 * Core integration module to patch WebAssembly memory management between browser and Node.js
 */

import { createMemoryManager } from './memory.js';
import { createBrowserMemoryManager } from './browserMemory.js';
import { CassetteLoaderOptions } from './types.js';
import { createLogger, isBrowser, isNode } from './utils.js';

// Function to detect if a WASM module was compiled for a specific environment
export function detectWasmCompilationTarget(
  instance: WebAssembly.Instance,
  logger?: ReturnType<typeof createLogger>
): { 
  isBrowserTargeted: boolean; 
  isNodeTargeted: boolean; 
  targetEnvironment: string;
} {
  const log = logger ? logger.log : (...args: any[]) => {}; // No-op logger if none provided
  const exports = instance.exports;
  
  // Default result
  const result = {
    isBrowserTargeted: false,
    isNodeTargeted: false, 
    targetEnvironment: 'unknown'
  };
  
  // Check for browser-specific exports
  const browserSpecificExports = ['__wbindgen', '__wbg', 'memory'];
  let hasBrowserExports = false;
  
  // Check for Node-specific exports
  const nodeSpecificExports = ['alloc_string', 'dealloc_string', 'get_string_len'];
  let hasNodeExports = false;
  
  // Count occurrences of each type
  for (const name in exports) {
    // Check for browser-specific exports
    if (browserSpecificExports.some(prefix => name.startsWith(prefix))) {
      hasBrowserExports = true;
    }
    
    // Check for Node-specific exports
    if (nodeSpecificExports.includes(name)) {
      hasNodeExports = true;
    }
  }
  
  // Determine the most likely environment
  if (hasBrowserExports && !hasNodeExports) {
    result.isBrowserTargeted = true;
    result.targetEnvironment = 'browser';
    log('WASM module appears to be compiled for browser use');
  } else if (hasNodeExports && !hasBrowserExports) {
    result.isNodeTargeted = true;
    result.targetEnvironment = 'node';
    log('WASM module appears to be compiled for Node.js use');
  } else if (hasBrowserExports && hasNodeExports) {
    result.isBrowserTargeted = true;
    result.isNodeTargeted = true;
    result.targetEnvironment = 'universal';
    log('WASM module appears to be compatible with both browser and Node.js');
  } else {
    log('Could not determine compilation target of WASM module');
  }
  
  return result;
}

/**
 * Create a memory manager for an instance based on environment
 * @param instance WebAssembly instance
 * @param options Options for the memory manager
 */
export function createMemoryManagerForInstance(
  instance: WebAssembly.Instance,
  options: CassetteLoaderOptions
): any {
  const logger = createLogger(options.debug || false, 'MemoryManagerFactory');
  
  // Detect compilation settings by examining exports
  const targetEnv = detectWasmCompilationTarget(instance, logger);
  
  // Determine if we should use browser memory
  let useBrowserMemory = false;
  
  // Explicitly set in options
  if (options.useBrowserMemory !== undefined) {
    useBrowserMemory = options.useBrowserMemory;
    logger.log(`Using memory manager based on explicit option: ${useBrowserMemory ? 'browser' : 'standard'}`);
  } 
  // Force Node environment override
  else if (options.forceNodeEnvironment) {
    useBrowserMemory = false;
    logger.log('Forcing standard memory manager due to forceNodeEnvironment option');
  } 
  // Auto-detect based on current environment and module target
  else {
    const currentEnv = isBrowser() ? 'browser' : 'node';
    logger.log(`Current environment: ${currentEnv}`);
    
    // If we're in a browser, or if the WASM is specifically targeted at browsers
    if (isBrowser() || targetEnv.isBrowserTargeted) {
      useBrowserMemory = true;
      logger.log(`Using browser memory manager based on environment detection`);
    } else {
      useBrowserMemory = false;
      logger.log(`Using standard memory manager based on environment detection`);
    }
  }
  
  if (useBrowserMemory) {
    logger.log('Using browser-specific memory manager');
    return createBrowserMemoryManager(instance, options.debug || false);
  } else {
    logger.log('Using standard memory manager');
    return createMemoryManager(instance, options.debug || false);
  }
}

/**
 * Wrap a request processing function with enhanced error handling and compatibility features
 * @param processFn The function to wrap
 * @returns Enhanced function
 */
export function enhanceRequestProcessing(processFn: (requestStr: string) => string) {
  const logger = createLogger(true, 'EnhancedRequestProcessor');
  
  return (requestStr: string): string => {
    logger.log(`------- New Request Processing -------`);
    logger.log(`Processing in environment: isBrowser=${isBrowser()}, isNode=${isNode()}`);
    
    // Extract and validate subscription ID first
    const subId = extractSubscriptionId(requestStr);
    if (!subId) {
      logger.error('Invalid request: Missing or invalid subscription ID');
      return JSON.stringify(['NOTICE', 'Invalid request: Missing or invalid subscription ID']);
    }
    
    logger.log(`Processing request for subscription: ${subId}`);
    logger.log(`Raw request: ${requestStr.substring(0, 100)}${requestStr.length > 100 ? '...' : ''}`);
    
    // Parse request JSON to extract detailed information
    try {
      const parsedRequest = JSON.parse(requestStr);
      
      if (Array.isArray(parsedRequest) && parsedRequest.length >= 2) {
        const [cmd, ...filters] = parsedRequest;
        
        // Validate command is REQ
        if (cmd === 'REQ') {
          // Process filters
          if (filters.length > 0) {
            const filter = filters[0];
            const filterKeys = Object.keys(filter);
            logger.log(`Filter keys for subscription ${subId}: ${filterKeys.join(', ')}`);
            
            // Analyze different filter types
            if (filter.ids && Array.isArray(filter.ids)) {
              logger.log(`ID filter with ${filter.ids.length} IDs for subscription ${subId}`);
            }
            
            if (filter.authors && Array.isArray(filter.authors)) {
              logger.log(`Author filter with ${filter.authors.length} authors for subscription ${subId}`);
            }
            
            if (filter.kinds && Array.isArray(filter.kinds)) {
              logger.log(`Kind filter for subscription ${subId}: ${filter.kinds.join(', ')}`);
            }
            
            if (filter.limit !== undefined) {
              logger.log(`Limit for subscription ${subId}: ${filter.limit}`);
            }
            
            if (filter.since !== undefined) {
              logger.log(`Since for subscription ${subId}: ${filter.since}`);
            }
            
            if (filter.until !== undefined) {
              logger.log(`Until for subscription ${subId}: ${filter.until}`);
            }
            
            // Analyze tag filters
            const tagFilters = filterKeys.filter(key => key.startsWith('#') || key.startsWith('&'));
            if (tagFilters.length > 0) {
              logger.log(`Tag filters for subscription ${subId}: ${tagFilters.join(', ')}`);
              tagFilters.forEach(tag => {
                logger.log(`  Tag ${tag} for subscription ${subId}: ${JSON.stringify(filter[tag])}`);
              });
            }
          }
        }
      }
    } catch (parseError: Error | unknown) {
      if (parseError instanceof Error) {
        logger.error(`Could not parse request for subscription ${subId}: ${parseError.message}`);
      } else {
        logger.error(`Could not parse request for subscription ${subId}: ${String(parseError)}`);
      }
    }
    
    // Process the request with the underlying function
    let responseStr;
    try {
      responseStr = processFn(requestStr);
      logger.log(`Raw response for subscription ${subId}: ${responseStr.substring(0, 200)}${responseStr.length > 200 ? '...' : ''}`);
      
      // Try to parse and analyze the response
      try {
        const parsedResponse = JSON.parse(responseStr);
        
        if (Array.isArray(parsedResponse)) {
          if (parsedResponse.length > 0) {
            logger.log(`Response message type for subscription ${subId}: ${parsedResponse[0]}`);
            
            // Log details based on message type
            if (parsedResponse[0] === 'EVENT' && parsedResponse.length > 2) {
              const eventSubId = parsedResponse[1];
              const event = parsedResponse[2];
              
              // Verify subscription ID matches
              if (eventSubId !== subId) {
                logger.warn(`Subscription ID mismatch: expected ${subId}, got ${eventSubId}`);
              }
              
              logger.log(`Event for subscription ${subId}:`);
              logger.log(`  Kind: ${event.kind}`);
              logger.log(`  ID: ${event.id?.substring(0, 10)}...`);
              logger.log(`  Pubkey: ${event.pubkey?.substring(0, 10)}...`);
              logger.log(`  Tags count: ${event.tags?.length || 0}`);
              logger.log(`  Content length: ${event.content?.length || 0}`);
            } else if (parsedResponse[0] === 'EOSE' && parsedResponse.length > 1) {
              const eoseSubId = parsedResponse[1];
              if (eoseSubId !== subId) {
                logger.warn(`Subscription ID mismatch in EOSE: expected ${subId}, got ${eoseSubId}`);
              }
              logger.log(`End of stored events for subscription: ${subId}`);
            } else if (parsedResponse[0] === 'NOTICE' && parsedResponse.length > 1) {
              logger.log(`Notice message for subscription ${subId}: ${parsedResponse[1]}`);
            }
          } else {
            logger.warn(`Empty response array for subscription ${subId}`);
          }
        } else if (typeof parsedResponse === 'object') {
          if (parsedResponse.id && parsedResponse.pubkey && typeof parsedResponse.kind === 'number') {
            logger.warn(`Raw event object received for subscription ${subId}, not in NIP-01 format`);
            logger.log(`Raw event: kind=${parsedResponse.kind}, id=${parsedResponse.id.substring(0, 10)}...`);
          } else {
            logger.log(`Response object for subscription ${subId} with keys: ${Object.keys(parsedResponse).join(', ')}`);
          }
        } else {
          logger.log(`Response type for subscription ${subId}: ${typeof parsedResponse}`);
        }
      } catch (parseError: Error | unknown) {
        if (parseError instanceof Error) {
          logger.error(`Response is not valid JSON for subscription ${subId}: ${parseError.message}`);
        } else {
          logger.error(`Response is not valid JSON for subscription ${subId}: ${String(parseError)}`);
        }
      }
    } catch (error: any) {
      logger.error(`Error processing request for subscription ${subId}: ${error.message}`);
      return JSON.stringify(['NOTICE', `Error processing request: ${error.message}`]);
    }
    
    return responseStr;
  };
}

/**
 * Utility to format a response as a NIP-01 EVENT message with subscription ID
 * @param event The event object
 * @param subId Subscription ID
 */
export function formatEventResponse(event: any, subId: string): string {
  if (!event) return JSON.stringify(["NOTICE", "Error: No event to format"]);
  
  try {
    // If it's already a NIP-01 message array, return it as is
    if (Array.isArray(event) && event.length >= 2 && 
        (event[0] === "EVENT" || event[0] === "NOTICE" || event[0] === "EOSE")) {
      return JSON.stringify(event);
    }
    
    // Format as EVENT message
    return JSON.stringify(["EVENT", subId, event]);
  } catch (error: any) {
    return JSON.stringify(["NOTICE", `Error formatting event: ${error.message}`]);
  }
}

/**
 * Extract the subscription ID from a NIP-01 request with improved error handling and logging
 * @param requestStr Request string
 * @returns Subscription ID or null if not found/invalid
 */
export function extractSubscriptionId(requestStr: string): string | null {
  const logger = createLogger(true, 'SubscriptionIdExtractor');
  
  try {
    const parsed = JSON.parse(requestStr);
    
    // Log the parsed request for debugging
    logger.log(`Parsed request: ${JSON.stringify(parsed, null, 2)}`);
    
    if (!Array.isArray(parsed)) {
      logger.warn('Request is not an array');
      return null;
    }
    
    if (parsed.length < 2) {
      logger.warn(`Request array too short: ${parsed.length} elements`);
      return null;
    }
    
    if (parsed[0] !== 'REQ') {
      logger.warn(`Invalid message type: ${parsed[0]}`);
      return null;
    }
    
    const subId = parsed[1];
    if (typeof subId !== 'string') {
      logger.warn(`Subscription ID is not a string: ${typeof subId}`);
      return null;
    }
    
    if (!subId.trim()) {
      logger.warn('Subscription ID is empty or whitespace');
      return null;
    }
    
    logger.log(`Successfully extracted subscription ID: ${subId}`);
    return subId;
  } catch (error) {
    logger.error(`Failed to parse request: ${error instanceof Error ? error.message : String(error)}`);
    return null;
  }
} 