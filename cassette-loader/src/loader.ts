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
  isNode
} from './utils.js';

/**
 * Default options for loading cassettes
 */
const DEFAULT_OPTIONS: CassetteLoaderOptions = {
  memoryInitialSize: 16,
  exposeExports: false,
  debug: false
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
    const arrayBuffer = await toArrayBuffer(source);
    
    // Create a memory instance
    const memory = new WebAssembly.Memory({ initial: opts.memoryInitialSize || 16 });
    
    // Standard import object that works with wasm-bindgen generated modules
    const defaultImports: WebAssembly.Imports = {
      env: {
        memory
      },
      // Add a standardized namespace for wasm-bindgen functions
      __wbindgen_placeholder__: {
        __wbindgen_string_new: function(ptr: number, len: number) {
          const buf = new Uint8Array(memory.buffer).subarray(ptr, ptr + len);
          return buf;
        },
        __wbindgen_throw: function(ptr: number, len: number) {
          const buf = new Uint8Array(memory.buffer).subarray(ptr, ptr + len);
          throw new Error(new TextDecoder('utf-8').decode(buf));
        }
      },
      // Add standardized console bindings that many cassettes use
      console: {
        log: function(...args: any[]) {
          console.log('[Cassette]', ...args);
          return 0;
        },
        error: function(...args: any[]) {
          console.error('[Cassette Error]', ...args);
          return 0;
        }
      }
    };
    
    // Merge default imports with custom imports if provided
    const importObject = opts.customImports 
      ? mergeImports(defaultImports, opts.customImports) 
      : defaultImports;
    
    // Compile the WebAssembly module
    logger.log('Compiling WebAssembly module...');
    const module = await WebAssembly.compile(arrayBuffer);
    
    // Add dynamic import stubs for any missing imports the module requires
    const requiredImports = WebAssembly.Module.imports(module);
    addDynamicImports(importObject, requiredImports, logger);
    
    // Instantiate the WebAssembly module
    logger.log('Instantiating WebAssembly module...');
    const instance = await WebAssembly.instantiate(module, importObject);
    const exports = instance.exports;
    
    logger.log('WebAssembly module instantiated successfully');
    logger.log('Exports:', Object.keys(exports));
    
    // Look for the required functions
    const describeFn = findExportFunction(exports, [
      'describe', 'describe_wasm', 'DESCRIBE', 'getInfo', 'get_info', 
      'getMetadata', 'get_metadata', 'metadata', 'info', 'getDetails', 
      'get_details', 'getDescription', 'get_description', 'details',
      'about', 'getAbout', 'get_about', 'manifest', 'getManifest'
    ]);
    
    if (!describeFn) {
      throw new CassetteLoadError('WebAssembly module missing required "describe" function');
    }
    
    const reqFn = findExportFunction(exports, [
      'req', 'req_wasm', 'REQ', 'request', 'process_request', 'processRequest',
      'handleRequest', 'handle_request', 'handleReq', 'handle_req', 
      'process', 'processReq', 'process_req', 'call', 'invoke', 
      'execute', 'run', 'handle', 'event', 'processEvent', 'process_event',
      'handleEvent', 'handle_event', 'emit', 'submit', 'send'
    ]);
    
    if (!reqFn) {
      throw new CassetteLoadError('WebAssembly module missing required "req" function');
    }
    
    const closeFn = findExportFunction(exports, [
      'close', 'close_wasm', 'CLOSE', 'closeSubscription', 'close_subscription', 
      'unsubscribe', 'disconnect', 'end', 'finish', 'complete', 'terminate',
      'destroy', 'cleanup', 'dispose'
    ]);
    
    // Optional memory management functions
    const allocFn = findExportFunction(exports, ['allocString', 'alloc_string', 'alloc', 'malloc', 'allocate']);
    const deallocFn = findExportFunction(exports, ['deallocString', 'dealloc_string', 'dealloc', 'free', 'deallocate']);
    
    // Create wrapper methods for the cassette
    logger.log('Creating cassette wrapper...');
    
    // Extract metadata from describe function
    let metadata: any;
    try {
      const describeResult = extractWasmResult(describeFn.call(null), memory, deallocFn);
      logger.log('Describe result:', describeResult);
      
      // Parse the metadata JSON
      metadata = JSON.parse(describeResult);
    } catch (error) {
      logger.error('Failed to parse metadata:', error);
      metadata = {
        name: fileName.replace('.wasm', ''),
        description: 'Failed to parse metadata',
        version: 'unknown'
      };
    }
    
    // Create the cassette object
    const cassette: Cassette = {
      id: cassetteId,
      fileName,
      name: metadata.name || metadata.metadata?.name || fileName.replace('.wasm', ''),
      description: metadata.description || metadata.metadata?.description || 'No description available',
      version: metadata.version || metadata.metadata?.version || 'unknown',
      methods: {
        describe: () => {
          try {
            const result = describeFn.call(null);
            return extractWasmResult(result, memory, deallocFn);
          } catch (error: any) {
            logger.error('Error in describe method:', error);
            return JSON.stringify({
              error: error.message || 'Unknown error',
              name: cassette.name,
              description: 'Error occurred during describe call',
              version: cassette.version
            });
          }
        },
        req: (requestStr: string) => {
          try {
            logger.log('Processing request:', requestStr);
            
            // If we have alloc/dealloc functions, use them for passing strings
            if (allocFn && deallocFn) {
              logger.log('Using memory management functions for request');
              
              // Convert request string to bytes
              const bytes = new TextEncoder().encode(requestStr);
              // Allocate memory for the request
              const ptr = allocFn.call(null, bytes.length);
              
              // Write the request to memory
              const mem = new Uint8Array(memory.buffer);
              for (let i = 0; i < bytes.length; i++) {
                mem[ptr + i] = bytes[i];
              }
              mem[ptr + bytes.length] = 0; // Null terminator
              
              // Call the req function with the pointer
              const result = reqFn.call(null, ptr);
              
              // Free the allocated memory
              deallocFn.call(null, ptr, bytes.length);
              
              // Extract the result
              return extractWasmResult(result, memory, deallocFn);
            }
            
            // Otherwise, try passing the string directly
            const result = reqFn.call(null, requestStr);
            return extractWasmResult(result, memory, deallocFn);
          } catch (error: any) {
            logger.error('Error in req method:', error);
            return JSON.stringify({
              error: error.message || 'Unknown error',
              notice: ["NOTICE", error.message || 'Unknown error']
            });
          }
        }
      },
      memory
    };
    
    // Add close method if available
    if (closeFn) {
      cassette.methods.close = (closeStr: string) => {
        try {
          logger.log('Processing close:', closeStr);
          
          // Similar approach as req method
          if (allocFn && deallocFn) {
            const bytes = new TextEncoder().encode(closeStr);
            const ptr = allocFn.call(null, bytes.length);
            
            const mem = new Uint8Array(memory.buffer);
            for (let i = 0; i < bytes.length; i++) {
              mem[ptr + i] = bytes[i];
            }
            mem[ptr + bytes.length] = 0; // Null terminator
            
            const result = closeFn.call(null, ptr);
            
            deallocFn.call(null, ptr, bytes.length);
            
            return extractWasmResult(result, memory, deallocFn);
          }
          
          const result = closeFn.call(null, closeStr);
          return extractWasmResult(result, memory, deallocFn);
        } catch (error: any) {
          logger.error('Error in close method:', error);
          return JSON.stringify({
            error: error.message || 'Unknown error',
            notice: ["NOTICE", error.message || 'Unknown error']
          });
        }
      };
    }
    
    // Optionally expose WebAssembly exports
    if (opts.exposeExports) {
      cassette.exports = exports;
      cassette.instance = instance;
    }
    
    logger.log('Cassette loaded successfully:', cassette.name);
    
    return {
      success: true,
      cassette
    };
  } catch (error: any) {
    logger.error('Failed to load cassette:', error);
    return {
      success: false,
      error: error.message || 'Unknown error loading cassette'
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