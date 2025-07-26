import { 
  Cassette, 
  CassetteLoaderOptions, 
  CassetteLoadResult, 
  CassetteSource,
  CassetteLoadError
} from './types.js';
import {
  toArrayBuffer,
  generateCassetteId,
  createLogger,
  isBrowser,
  isNode,
  createEventTracker
} from './utils.js';
import { WasmMemoryManager, createMemoryManager } from './memory.js';

/**
 * Default options for loading cassettes
 */
const DEFAULT_OPTIONS: CassetteLoaderOptions = {
  memoryInitialSize: 16,
  exposeExports: false,
  debug: false,
  deduplicateEvents: true
};

/**
 * Function to find exported functions by various possible names
 * @param exports WebAssembly exports
 * @param possibleNames Array of possible function names
 * @returns The function if found, null otherwise
 */
function findExportFunction(exports: WebAssembly.Exports, possibleNames: string[]): Function | null {
  // First try exact matches from possible names
  for (const name of possibleNames) {
    if (name in exports && typeof exports[name as keyof typeof exports] === 'function') {
      return exports[name as keyof typeof exports] as Function;
    }
  }
  
  // Try different naming conventions
  for (const exportName of Object.keys(exports)) {
    if (typeof exports[exportName as keyof typeof exports] !== 'function') continue;
    
    // Check if the export name matches any of the possible names (case-insensitive)
    for (const name of possibleNames) {
      const lowerExportName = exportName.toLowerCase();
      const lowerName = name.toLowerCase();
      
      // Check for various patterns
      if (lowerExportName === lowerName ||
          lowerExportName.endsWith(`_${lowerName}`) ||
          lowerExportName.endsWith(lowerName) ||
          lowerExportName.startsWith(lowerName)) {
        return exports[exportName as keyof typeof exports] as Function;
      }
    }
  }
  
  return null;
}

/**
 * Core interface for direct WebAssembly interactions without wasm-bindgen
 */
class CoreCassetteInterface {
  private exports: WebAssembly.Exports;
  private memory: WebAssembly.Memory;
  private memoryManager: WasmMemoryManager;
  private logger: ReturnType<typeof createLogger>;
  private eventTracker = createEventTracker();
  private currentSubscriptionId: string = '';
  
  constructor(instance: WebAssembly.Instance, debug = false) {
    this.exports = instance.exports;
    this.memory = this.exports.memory as WebAssembly.Memory;
    this.memoryManager = createMemoryManager(instance, debug);
    this.logger = createLogger(debug, 'CoreCassetteInterface');
  }
  
  /**
   * Helper to validate JSON structure before sending to WASM
   * Returns either the original string if valid or a fixed version if possible
   */
  private validateJsonFormat(jsonStr: string): string {
    try {
      // Try to parse as JSON to check validity
      const parsed = JSON.parse(jsonStr);
      
      // Handle the specific edge case for REQ format with invalid filter
      if (Array.isArray(parsed) && parsed[0] === "REQ" && parsed.length >= 3) {
        const filter = parsed[2];
        
        // Check if filter is misformatted (e.g., using numeric keys)
        if (typeof filter === 'object' && filter !== null) {
          // Create a new filter object with all keys properly stringified
          const fixedFilter: Record<string, any> = {};
          
          // Copy all entries with string keys
          for (const key in filter) {
            fixedFilter[String(key)] = filter[key];
          }
          
          // Replace the filter in the array and re-stringify
          parsed[2] = fixedFilter;
          
          const fixed = JSON.stringify(parsed);
          if (fixed !== jsonStr) {
            this.logger.log(`Fixed JSON format issues: ${jsonStr} -> ${fixed}`);
            return fixed;
          }
        }
      }
      
      // No issues or fixes needed
      return jsonStr;
    } catch (error) {
      // If parsing fails, return the original string
      // The error will be properly reported elsewhere
      this.logger.warn(`JSON validation failed: ${error}`);
      return jsonStr;
    }
  }
  
  // Universal send method for all NIP-01 messages
  send(messageStr: string): string | string[] {
    this.logger.log(`Processing message: ${messageStr.substring(0, 100)}${messageStr.length > 100 ? '...' : ''}`);
    
    if (typeof this.exports.send !== 'function') {
      throw new Error('send function not implemented by cassette');
    }
    
    // Parse the message to determine type
    let isReqMessage = false;
    let subscriptionId = '';
    
    try {
      this.logger.log(`Validating message JSON: ${messageStr}`);
      const parsedMsg = JSON.parse(messageStr);
      
      if (Array.isArray(parsedMsg) && parsedMsg.length >= 1) {
        const messageType = parsedMsg[0];
        
        // Handle REQ messages specially - they need to loop
        if (messageType === "REQ" && parsedMsg.length >= 2) {
          subscriptionId = parsedMsg[1];
          isReqMessage = true;
          
          // Reset event tracker for new REQ
          if (this.eventTracker) {
            this.logger.log(`New REQ call received, resetting event tracker`);
            this.eventTracker.reset();
            this.currentSubscriptionId = subscriptionId;
          }
        }
        
        // Handle CLOSE messages
        if (messageType === "CLOSE" && parsedMsg.length >= 2) {
          const closeSubscriptionId = parsedMsg[1];
          
          // Reset event tracker and subscription ID on CLOSE
          if (closeSubscriptionId === this.currentSubscriptionId) {
            this.logger.log(`Closing subscription ${closeSubscriptionId}, resetting event tracker`);
            if (this.eventTracker) {
              this.eventTracker.reset();
            }
            this.currentSubscriptionId = '';
          }
        }
      }
    } catch (parseError) {
      this.logger.warn(`Failed to parse message string: ${parseError}`);
    }
    
    // If it's a REQ message, collect all events until EOSE
    if (isReqMessage) {
      return this._collectAllEventsForReq(messageStr, subscriptionId);
    }
    
    // For non-REQ messages, use single call
    return this._sendSingle(messageStr);
  }
  
  // Process results with event deduplication
  private processResults(result: string): string {
    // Split result into individual messages if it contains newlines
    if (result.includes('\n')) {
      const messages = result.trim().split('\n');
      this.logger.log(`Processing ${messages.length} newline-separated messages`);
      
      // Validate and filter messages
      const filteredMessages = messages.filter(message => {
        try {
          const parsed = JSON.parse(message);
          if (!Array.isArray(parsed) || parsed.length < 2) {
            this.logger.warn(`Invalid message format: ${message.substring(0, 100)}`);
            return false;
          }
          if (!["NOTICE", "EVENT", "EOSE", "OK", "COUNT"].includes(parsed[0])) {
            this.logger.warn(`Unknown message type: ${parsed[0]}`);
            return false;
          }
          
          // For EVENT messages, check for duplicates
          if (parsed[0] === "EVENT" && parsed.length >= 3 && this.eventTracker && 
              typeof parsed[2] === 'object' && parsed[2].id) {
            const eventId = parsed[2].id;
            if (!this.eventTracker.addAndCheck(eventId)) {
              this.logger.log(`Filtered duplicate event: ${eventId}`);
              return false;
            }
          }
          return true;
        } catch (parseError) {
          this.logger.warn(`Failed to parse message: ${parseError}`);
          return false;
        }
      });
      
      // Return filtered messages
      return filteredMessages.join('\n');
    }
    
    // If we get here, the result doesn't contain newlines
    try {
      const parsed = JSON.parse(result);
      if (Array.isArray(parsed)) {
        if (parsed[0] === "EVENT" && parsed.length >= 3 && 
            this.eventTracker && typeof parsed[2] === 'object' && parsed[2].id) {
          const eventId = parsed[2].id;
          if (!this.eventTracker.addAndCheck(eventId)) {
            this.logger.log(`Filtered duplicate event: ${eventId}`);
            return ''; // Return empty string instead of NOTICE message
          }
        }
        // Always return EOSE messages
        if (parsed[0] === "EOSE") {
          this.logger.log(`Received EOSE message for subscription ${parsed[1]}`);
          return result;
        }
      }
      return result;
    } catch (parseError: any) {
      this.logger.warn(`Failed to parse result: ${parseError.message}`);
      return JSON.stringify(["NOTICE", `Error: ${parseError.message}`]);
    }
  }
  
  getSchema(): string {
    this.logger.log('Getting cassette schema');
    
    // Try to use chunked method first for schema
    if (typeof this.exports.get_schema_size === 'function' && 
        typeof this.exports.get_schema_chunk === 'function') {
      this.logger.log('Using chunked schema method');
      const size = (this.exports.get_schema_size as Function)();
      let schema = '';
      
      // Load in chunks of 1000 bytes
      const chunkSize = 1000;
      for (let i = 0; i < size; i += chunkSize) {
        const ptr = (this.exports.get_schema_chunk as Function)(i, chunkSize);
        const chunkStr = this.memoryManager.readString(ptr);
        schema += chunkStr;
        this.memoryManager.deallocateString(ptr);
      }
      
      return schema;
    }
    
    // Fall back to direct get_schema method
    this.logger.log('Using direct get_schema method');
    if (typeof this.exports.get_schema === 'function') {
      return this.memoryManager.callStringFunction('get_schema');
    }
    
    // Last resort - return empty schema
    this.logger.log('No schema method found, returning empty schema');
    return '{}';
  }
  
  info(): string {
    this.logger.log('Getting cassette NIP-11 relay information');
    
    // Try to use the info method if available
    if (typeof this.exports.info === 'function') {
      return this.memoryManager.callStringFunction('info');
    }
    
    // Fallback to minimal info if info method not found
    this.logger.log('No info method found, returning minimal info');
    return JSON.stringify({ supported_nips: [] });
  }
  
  describe(): string {
    this.logger.log('Getting cassette description');
    // Since we're removing the describe function from cassettes,
    // return a minimal description based on available info
    try {
      const info = this.info();
      const infoObj = JSON.parse(info);
      return JSON.stringify({
        name: infoObj.name || 'Unknown Cassette',
        description: infoObj.description || 'No description available',
        version: '1.0.0'
      });
    } catch (e) {
      return JSON.stringify({
        name: 'Unknown Cassette',
        description: 'No description available',
        version: '1.0.0'
      });
    }
  }
  
  // Private method to collect all events for REQ messages
  private _collectAllEventsForReq(messageStr: string, subscriptionId: string): string[] {
    this.logger.log(`Collecting all events for REQ subscription: ${subscriptionId}`);
    const results: string[] = [];
    
    // Keep calling until we get EOSE or terminating condition
    while (true) {
      const response = this._sendSingle(messageStr);
      
      // Empty response means no more events
      if (!response || response.length === 0) {
        this.logger.log('Received empty response, stopping');
        break;
      }
      
      try {
        const parsed = JSON.parse(response);
        
        // Check for terminating messages
        if (Array.isArray(parsed)) {
          if (parsed[0] === "EOSE") {
            this.logger.log(`Received EOSE for subscription ${subscriptionId}`);
            results.push(response);
            break;
          } else if (parsed[0] === "CLOSED") {
            this.logger.log(`Received CLOSED for subscription ${subscriptionId}`);
            results.push(response);
            break;
          }
        }
        
        // Add the response to results
        results.push(response);
        
      } catch (e) {
        this.logger.warn(`Failed to parse response: ${e}`);
        // If we can't parse the response, stop
        break;
      }
    }
    
    // If we didn't get an explicit EOSE, add one
    const hasEOSE = results.some(r => {
      try {
        const p = JSON.parse(r);
        return Array.isArray(p) && p[0] === "EOSE";
      } catch {
        return false;
      }
    });
    
    if (!hasEOSE) {
      results.push(JSON.stringify(["EOSE", subscriptionId]));
    }
    
    return results;
  }
  
  // Private method for single send call
  private _sendSingle(messageStr: string): string {
    // Validate and potentially fix JSON format issues before sending to WASM
    const validatedMessageStr = this.validateJsonFormat(messageStr);
    
    // Write message string to memory
    let messagePtr = 0;
    let resultPtr = 0;
    let result = '';
    
    try {
      // First allocate and write the message string to memory
      try {
        messagePtr = this.memoryManager.writeString(validatedMessageStr);
        if (messagePtr === 0) {
          this.logger.error("Failed to allocate memory for message string");
          return JSON.stringify(["NOTICE", "Error: Failed to allocate memory for message"]);
        }
      } catch (allocError: any) {
        this.logger.error(`Error allocating memory for message: ${allocError}`);
        return JSON.stringify(["NOTICE", `Error: ${allocError.message}`]);
      }
    
      // Call send function (which should return a pointer to the result)
      try {
        this.logger.log('Calling send function');
        resultPtr = (this.exports.send as Function)(messagePtr, messageStr.length);
        
        if (resultPtr === 0) {
          this.logger.warn('send function returned null pointer');
          return JSON.stringify(["NOTICE", "Error: Empty response from cassette"]);
        }
      } catch (callError: any) {
        this.logger.error(`Error calling send function: ${callError}`);
        return JSON.stringify(["NOTICE", `Error: ${callError.message}`]);
      }
      
      // Read result from memory
      try {
        result = this.memoryManager.readString(resultPtr);
        if (!result || result.length === 0) {
          this.logger.warn('Empty result from send function');
          return JSON.stringify(["NOTICE", "Error: Empty response from cassette"]);
        }
        
        this.logger.log(`Raw result from send: ${result.substring(0, 100)}${result.length > 100 ? '...' : ''}`);
        
        // Process results just like before for event deduplication
        return this.processResults(result);
      } catch (readError: any) {
        this.logger.error(`Error reading result from memory: ${readError}`);
        return JSON.stringify(["NOTICE", `Error: ${readError.message}`]);
      }
    } catch (error: any) {
      this.logger.error(`Error in send method: ${error}`);
      return JSON.stringify(["NOTICE", `Error: ${error.message}`]);
    } finally {
      // Clean up memory (with proper error handling)
      if (messagePtr) {
        try {
          this.memoryManager.deallocateString(messagePtr);
        } catch (cleanupError) {
          this.logger.error(`Error cleaning up message memory: ${cleanupError}`);
        }
      }
      
      if (resultPtr) {
        try {
          this.memoryManager.deallocateString(resultPtr);
        } catch (cleanupError) {
          this.logger.error(`Error cleaning up result memory: ${cleanupError}`);
        }
      }
    }
  }
}

/**
 * Load a cassette from various sources
 * @param source Source of the cassette (file, URL, or ArrayBuffer)
 * @param fileName Original file name of the cassette (optional, used for ID generation)
 * @param options Options for loading the cassette
 * @returns Promise that resolves with the result of loading the cassette
 */
export async function loadCassette(
  source: CassetteSource,
  fileName?: string,
  options: CassetteLoaderOptions = {}
): Promise<CassetteLoadResult> {
  // Merge options with defaults
  const opts = { ...DEFAULT_OPTIONS, ...options };
  const logger = createLogger(opts.debug);
  
  try {
    // Get the original file name if it's a File object
    if (typeof File !== 'undefined' && source instanceof File) {
      fileName = fileName || source.name;
    }
    
    // Default file name if none provided
    fileName = fileName || 'unknown_cassette.wasm';
    
    // Generate a unique ID for this cassette
    const cassetteId = generateCassetteId(fileName);
    
    logger.log(`Loading cassette ${fileName} (ID: ${cassetteId})`);
    
    // Convert source to ArrayBuffer
    const buffer = await toArrayBuffer(source);
    
    // Create memory for the WebAssembly module
    const memory = new WebAssembly.Memory({ 
      initial: opts.memoryInitialSize || 16, 
      // maximum: 1024 // Uncomment to set a maximum memory size
    });
    
    // Create import object with memory
    const baseImports: WebAssembly.Imports = {
      env: {
        memory,
        log: (...args: any[]) => {
          logger.log('WASM log:', ...args);
        },
        error: (...args: any[]) => {
          logger.error('WASM error:', ...args);
        },
        warn: (...args: any[]) => {
          logger.warn('WASM warn:', ...args);
        },
        abort: (...args: any[]) => {
          logger.error('WASM abort:', ...args);
          throw new Error('WASM aborted: ' + args.join(' '));
        }
      },
      // Include wbindgen helpers for compatibility
      __wbindgen_placeholder__: {
        __wbindgen_string_new: (ptr: number, len: number) => {
          const memory = new Uint8Array((baseImports.env.memory as WebAssembly.Memory).buffer);
          const slice = memory.slice(ptr, ptr + len);
          const text = new TextDecoder().decode(slice);
          return text;
        },
        __wbindgen_throw: (ptr: number, len: number) => {
          const memory = new Uint8Array((baseImports.env.memory as WebAssembly.Memory).buffer);
          const slice = memory.slice(ptr, ptr + len);
          const text = new TextDecoder().decode(slice);
          throw new Error(text);
        }
      }
    };
    
    // Merge custom imports with base imports if provided
    const importObject = opts.customImports 
      ? mergeImports(baseImports, opts.customImports)
      : baseImports;
    
    // Compile and instantiate the WebAssembly module
    logger.log('Compiling WebAssembly module...');
    const { instance, module } = await WebAssembly.instantiate(buffer, importObject);
    logger.log('WebAssembly module compiled and instantiated');
    
    // Get the module's imports to detect missing imports
    const requiredImports = WebAssembly.Module.imports(module);

    // Check for any missing imports and add dynamic stubs
    addDynamicImports(importObject, requiredImports, logger);
    
    const exports = instance.exports;
    
    // Create a memory manager for this instance
    const memoryManager = createMemoryManager(instance, opts.debug);
    
    // Create an instance of the core interface
    const coreInterface = new CoreCassetteInterface(instance, opts.debug);
    
    // Get description from the cassette
    let description: string;
    try {
      description = coreInterface.describe();
      logger.log(`Cassette description: ${description}`);
    } catch (error) {
      logger.error('Failed to get description:', error);
      description = JSON.stringify({
        name: fileName.replace(/\.[^/.]+$/, ""),
        description: "No description available",
        version: "unknown"
      });
    }
    
    // Parse the description as JSON
    let metadata;
    try {
      metadata = JSON.parse(description);
    } catch (error) {
      logger.error('Failed to parse description JSON:', error);
      metadata = {
        name: fileName.replace(/\.[^/.]+$/, ""),
        description: "Error parsing description",
        version: "unknown"
      };
    }
    
    // Extract metadata fields
    const name = metadata.name || metadata.metadata?.name || fileName.replace(/\.[^/.]+$/, "");
    const desc = metadata.description || metadata.metadata?.description || "No description available";
    const version = metadata.version || metadata.metadata?.version || "unknown";
    
    // Create a cassette object with methods
    const cassette: Cassette = {
      id: cassetteId,
      fileName,
      name,
      description: desc,
      version,
      methods: {
        describe: () => coreInterface.describe(),
        send: (messageStr: string) => coreInterface.send(messageStr),
        getSchema: () => coreInterface.getSchema(),
        info: () => coreInterface.info()
      },
      eventTracker: opts.deduplicateEvents !== false ? createEventTracker() : undefined,
      // Add memory stats method
      getMemoryStats: () => {
        return {
          allocatedPointers: memoryManager.getAllocatedPointers(),
          allocationCount: memoryManager.getAllocationCount(),
          memory: {
            totalPages: memory.buffer.byteLength / (64 * 1024),
            totalBytes: memory.buffer.byteLength,
            usageEstimate: memoryManager.getAllocationCount() > 0 ? 'Potential memory leak detected' : 'No leaks detected'
          }
        };
      },
      // Add dispose method to clean up resources
      dispose: () => {
        logger.log(`Disposing cassette ${cassetteId}`);
        
        // Get all allocated pointers that haven't been freed
        const allocatedPointers = memoryManager.getAllocatedPointers();
        
        if (allocatedPointers.length > 0) {
          logger.warn(`Found ${allocatedPointers.length} leaked memory allocations, attempting cleanup`);
          
          // Attempt to free each pointer
          for (const ptr of allocatedPointers) {
            try {
              memoryManager.deallocateString(ptr);
            } catch (error) {
              logger.error(`Failed to clean up memory at pointer ${ptr}:`, error);
            }
          }
          
          // Check if we've successfully cleaned up
          const remainingAllocations = memoryManager.getAllocationCount();
          if (remainingAllocations > 0) {
            logger.error(`Failed to clean up ${remainingAllocations} memory allocations`);
          } else {
            logger.log('Successfully cleaned up all memory allocations');
          }
        } else {
          logger.log('No memory leaks detected');
        }
        
        // Additional cleanup could be added here (e.g., closing WebAssembly instance)
        return { success: true, allocationsCleanedUp: allocatedPointers.length };
      }
    };
    
    // If debug mode is enabled, periodically check for memory leaks
    if (opts.debug) {
      logger.log('Setting up memory leak detection');
      
      // Check memory status after a delay
      setTimeout(() => {
        const memoryStats = cassette.getMemoryStats();
        if (memoryStats.allocationCount > 0) {
          logger.warn(`⚠️ Potential memory leak detected: ${memoryStats.allocationCount} allocations have not been freed`);
          logger.warn('To inspect memory stats, call cassette.getMemoryStats()');
          logger.warn('To clean up resources, call cassette.dispose()');
        }
      }, 10000); // Check after 10 seconds
    }

    // Expose exports if requested
    if (opts.exposeExports) {
      (cassette as any).exports = exports;
      (cassette as any).instance = instance;
      (cassette as any).memory = memory;
    }
    
    logger.log(`Cassette loaded successfully: ${name} (v${version})`);
    
    return {
      success: true,
      cassette,
      fileName,
      memory,
      instance
    };
  } catch (error: any) {
    logger.error('Failed to load cassette:', error);
    return {
      success: false,
      error: `Failed to load cassette: ${error.message || error}`
    };
  }
}

/**
 * Add any missing imports that the module requires
 * @param importObject Import object to modify
 * @param requiredImports Required imports from the module
 * @param logger Logger instance
 */
function addDynamicImports(
  importObject: WebAssembly.Imports, 
  requiredImports: WebAssembly.ModuleImportDescriptor[],
  logger: ReturnType<typeof createLogger>
): void {
  // Check for each required import
  for (const imp of requiredImports) {
    // Create the module namespace if it doesn't exist
    if (!importObject[imp.module]) {
      importObject[imp.module] = {};
    }
    
    // Skip if the import already exists
    if (imp.name in (importObject[imp.module] as object)) {
      continue;
    }
    
    // Add a stub function or global based on the kind
    if (imp.kind === 'function') {
      // Create a stub function based on name patterns
      logger.log(`Creating stub function for ${imp.module}.${imp.name}`);
      (importObject[imp.module] as any)[imp.name] = createStubFunction(imp.module, imp.name, logger);
    } else if (imp.kind === 'global') {
      logger.log(`Creating stub global for ${imp.module}.${imp.name}`);
      (importObject[imp.module] as any)[imp.name] = 0;
    }
  }
}

/**
 * Create a stub function for an import based on its name
 * @param module Module name
 * @param name Function name
 * @param logger Logger instance
 * @returns Stub function
 */
function createStubFunction(module: string, name: string, logger: ReturnType<typeof createLogger>): Function {
  const lowerName = name.toLowerCase();
  
  // Log functions
  if (lowerName.includes('log') || lowerName.includes('print')) {
    return function(...args: any[]) {
      console.log(`[WASM ${module}.${name}]`, ...args);
      return 0;
    };
  }
  
  // Error functions
  if (lowerName.includes('error') || lowerName.includes('panic')) {
    return function(...args: any[]) {
      console.error(`[WASM ${module}.${name}]`, ...args);
      return 0;
    };
  }
  
  // Default stub
  return function(...args: any[]) {
    logger.log(`Called ${module}.${name} with args:`, args);
    return 0;
  };
}

/**
 * Merge two import objects
 * @param base Base import object
 * @param custom Custom import object to merge
 * @returns Merged import object
 */
function mergeImports(base: WebAssembly.Imports, custom: WebAssembly.Imports): WebAssembly.Imports {
  const result: WebAssembly.Imports = { ...base };
  
  // Merge each module from custom imports
  for (const module of Object.keys(custom)) {
    if (!result[module]) {
      result[module] = {};
    }
    
    // Merge the module
    result[module] = { ...result[module], ...custom[module] };
  }
  
  return result;
}

/**
 * Extract a result from a WebAssembly function
 * @param result Result from a WebAssembly function
 * @param memory WebAssembly memory
 * @param deallocFn Deallocation function (optional)
 * @returns Extracted string value
 */
function extractWasmResult(result: any, memory: WebAssembly.Memory, deallocFn: Function | null | undefined): string {
  // If the result is a string, return it directly
  if (typeof result === 'string') {
    return result;
  }
  
  // If the result is an array with two elements [ptr, len], handle it
  if (Array.isArray(result) && result.length === 2) {
    const ptr = result[0];
    const len = result[1];
    
    // Read the string from memory using TextDecoder
    const memoryArray = new Uint8Array(memory.buffer, ptr, len);
    const response = new TextDecoder('utf-8').decode(memoryArray);
    
    // If there's a dealloc function, free the memory
    if (deallocFn) {
      deallocFn.call(null, ptr, len);
    }
    
    return response;
  }
  
  // If the result is a number, assume it's a pointer to a null-terminated string
  if (typeof result === 'number') {
    const mem = new Uint8Array(memory.buffer);
    let response = '';
    let i = result;
    const maxLen = 10000; // Safety limit to prevent infinite loops
    let count = 0;
    
    while (i < mem.length && mem[i] !== 0 && count < maxLen) {
      response += String.fromCharCode(mem[i]);
      i++;
      count++;
    }
    
    if (count === 0) {
      console.error('Empty string at pointer:', result);
      return JSON.stringify({
        error: 'Empty response from function'
      });
    }
    
    return response;
  }
  
  // Handle other types of results
  if (result === undefined || result === null) {
    return JSON.stringify({
      error: 'Function returned undefined or null'
    });
  }
  
  // Try to stringify the result
  try {
    return JSON.stringify(result);
  } catch (error) {
    return JSON.stringify({
      error: `Could not stringify result: ${error}`
    });
  }
} 