/**
 * Memory management utilities for WebAssembly cassettes
 */

// Constants
const MAX_STRING_LENGTH = 10_000_000; // 10MB safety limit
const MSGB_SIGNATURE = new Uint8Array([0x4D, 0x53, 0x47, 0x42]); // "MSGB"

/**
 * Handles memory interactions between JavaScript and WebAssembly
 */
export class WasmMemoryManager {
  private memory: WebAssembly.Memory;
  private exports: WebAssembly.Exports;
  private decoder = new TextDecoder('utf-8');
  private encoder = new TextEncoder();
  private debugMode: boolean;
  private allocatedPointers: Set<number>; // Track allocated pointers

  /**
   * Creates a new memory manager
   * @param memory WebAssembly memory
   * @param exports WebAssembly exports
   * @param debug Whether to enable debug logging
   */
  constructor(memory: WebAssembly.Memory, exports: WebAssembly.Exports, debug = false) {
    this.memory = memory;
    this.exports = exports;
    this.debugMode = debug;
    this.allocatedPointers = new Set<number>();
    
    if (debug) {
      this.debug('Memory manager initialized');
    }
  }

  /**
   * Log debug information if debug mode is enabled
   */
  private debug(...args: any[]) {
    if (this.debugMode) {
      console.log('[WasmMemoryManager]', ...args);
    }
  }

  /**
   * Check if a required function exists in the exports
   */
  private hasFunction(name: string): boolean {
    return typeof this.exports[name as keyof typeof this.exports] === 'function';
  }

  /**
   * Call a function in the exports with proper error handling
   */
  private callFunction(name: string, ...args: any[]): any {
    if (!this.hasFunction(name)) {
      throw new Error(`Function ${name} not found in WebAssembly exports`);
    }
    
    try {
      return (this.exports[name as keyof typeof this.exports] as Function)(...args);
    } catch (error) {
      this.debug(`Error calling ${name}:`, error);
      throw error;
    }
  }

  /**
   * Register an allocation in our tracking system
   * @param ptr The pointer to track
   */
  private registerAllocation(ptr: number): void {
    if (ptr === 0) return;
    this.allocatedPointers.add(ptr);
    this.debug(`Registered allocation at pointer ${ptr}, total allocations: ${this.allocatedPointers.size}`);
  }
  
  /**
   * Unregister an allocation from our tracking system
   * @param ptr The pointer to unregister
   */
  private unregisterAllocation(ptr: number): void {
    if (ptr === 0) return;
    const wasPresent = this.allocatedPointers.delete(ptr);
    this.debug(`Unregistered allocation at pointer ${ptr}, was present: ${wasPresent}, remaining allocations: ${this.allocatedPointers.size}`);
  }
  
  /**
   * Get the number of currently tracked allocations
   */
  public getAllocationCount(): number {
    return this.allocatedPointers.size;
  }
  
  /**
   * Get a list of all currently tracked allocations
   */
  public getAllocatedPointers(): number[] {
    return Array.from(this.allocatedPointers);
  }

  /**
   * Check if a buffer has our MSGB signature
   */
  private hasMsgbSignature(ptr: number): boolean {
    if (ptr === 0 || ptr + 4 > this.memory.buffer.byteLength) {
      return false;
    }
    
    const memory = new Uint8Array(this.memory.buffer);
    return (
      memory[ptr] === MSGB_SIGNATURE[0] &&
      memory[ptr + 1] === MSGB_SIGNATURE[1] &&
      memory[ptr + 2] === MSGB_SIGNATURE[2] &&
      memory[ptr + 3] === MSGB_SIGNATURE[3]
    );
  }

  /**
   * Read a string from memory with proper handling for MSGB format
   * 
   * @param ptr Pointer to the memory location
   * @returns The string value
   */
  readString(ptr: number): string {
    if (ptr === 0) {
      this.debug('Received null pointer');
      return '';
    }

    try {
      // Create a view of the memory
      const memory = new Uint8Array(this.memory.buffer);
      
      // Enhanced logging: Show the first 16 bytes at the pointer location
      if (ptr < memory.length) {
        const headerBytes = Array.from(memory.subarray(ptr, Math.min(ptr + 16, memory.length)))
          .map(b => b.toString(16).padStart(2, '0'))
          .join(' ');
        this.debug(`Memory at ptr ${ptr} starts with bytes: ${headerBytes}`);
        
        // Log the beginning as ASCII for easier identification
        const asciiPreview = Array.from(memory.subarray(ptr, Math.min(ptr + 16, memory.length)))
          .map(b => b >= 32 && b <= 126 ? String.fromCharCode(b) : '.')
          .join('');
        this.debug(`ASCII preview: ${asciiPreview}`);
      }
      
      // Check for MSGB signature
      if (this.hasMsgbSignature(ptr)) {
        this.debug('Detected MSGB string format');
        
        // MSGB format: [signature(4 bytes)][length(4 bytes)][data(length bytes)]
        const dataView = new DataView(this.memory.buffer);
        const length = dataView.getUint32(ptr + 4, true); // true = little endian
        
        this.debug(`String length from MSGB format: ${length}`);
        
        if (length > MAX_STRING_LENGTH) {
          throw new Error(`String too large (${length} bytes)`);
        }
        
        const stringData = memory.subarray(ptr + 8, ptr + 8 + length);
        const result = this.decoder.decode(stringData);
        
        // Enhanced debugging for JSON parsing
        this.debug(`Read string: ${result.substring(0, 50)}${result.length > 50 ? '...' : ''}`);
        
        // Validate if it's proper JSON
        try {
          JSON.parse(result);
          this.debug('Successfully validated as valid JSON');
        } catch (jsonError: unknown) {
          if (jsonError instanceof Error) {
            this.debug(`JSON parsing error: ${jsonError.message}`);
          } else {
            this.debug(`JSON parsing error: ${String(jsonError)}`);
          }
          this.debug(`JSON data starts with: ${result.substring(0, 100)}`);
          
          // Detailed character analysis for first 50 chars to find invalid sequences
          const firstChars = result.substring(0, 50);
          const charCodes = Array.from(firstChars).map(c => c.charCodeAt(0).toString(16).padStart(2, '0')).join(' ');
          this.debug(`Character codes: ${charCodes}`);
          
          // Try to identify common JSON parsing issues
          if (result.includes('\0')) {
            this.debug('Warning: String contains null bytes that may cause JSON parsing issues');
          }
          
          // Look for structural issues
          if (result.startsWith('[') && !result.includes(']')) {
            this.debug('JSON structural issue: Array open bracket without matching close bracket');
          } else if (result.startsWith('{') && !result.includes('}')) {
            this.debug('JSON structural issue: Object open bracket without matching close bracket');
          }
        }
        
        return result;
      }
      
      // If we get here, try to use the WebAssembly helper functions
      if (this.hasFunction('get_string_len') && this.hasFunction('get_string_ptr')) {
        try {
          // Get the string length and pointer to data
          const length = this.callFunction('get_string_len', ptr);
          const strPtr = this.callFunction('get_string_ptr', ptr);
          
          this.debug(`Using helper functions: length=${length}, strPtr=${strPtr}`);
          
          // Validate
          if (length <= 0 || length > MAX_STRING_LENGTH || 
              strPtr === 0 || strPtr >= memory.length || strPtr + length > memory.length) {
            this.debug('Invalid length or pointer from helper functions');
            return '';
          }
          
          // Read the string data
          const stringData = new Uint8Array(memory.buffer.slice(strPtr, strPtr + length));
          const result = this.decoder.decode(stringData);
          
          this.debug(`Read string using helper functions: ${result.substring(0, 50)}${result.length > 50 ? '...' : ''}`);
          return result;
        } catch (error) {
          this.debug(`Error using helper functions: ${error}`);
          // Continue with fallback
        }
      }
      
      // Fallback: try to read as a null-terminated string
      this.debug('Using fallback method to read string');
      
      // Find the null terminator
      let end = ptr;
      while (end < memory.length && memory[end] !== 0) {
        end++;
      }
      
      if (end === ptr) {
        this.debug('Empty string (null at start)');
        return '';
      }
      
      // Read the string
      const stringData = new Uint8Array(memory.buffer.slice(ptr, end));
      const result = this.decoder.decode(stringData);
      
      this.debug(`Read string using fallback: ${result.substring(0, 50)}${result.length > 50 ? '...' : ''}`);
      return result;
    } catch (error) {
      this.debug(`Error reading string: ${error}`);
      return '';
    }
  }

  /**
   * Write a string to memory with proper MSGB format
   * 
   * @param str String to write
   * @returns Pointer to the memory location
   */
  writeString(str: string): number {
    if (!str) {
      this.debug('Empty string provided to writeString, returning 0');
      return 0;
    }
    
    this.debug(`Writing string to memory (length ${str.length}): ${str.substring(0, 50)}${str.length > 50 ? '...' : ''}`);
    
    // First try to use the module's custom allocation function
    let ptr = 0;
    
    // Create the UTF-8 encoded string
    const bytes = this.encoder.encode(str);
    
    // Use allocString if available (most common name)
    if (this.hasFunction('allocString')) {
      this.debug('Using allocString function');
      try {
        ptr = this.callFunction('allocString', bytes.length);
        this.registerAllocation(ptr);
      } catch (error) {
        this.debug('Error allocating memory with allocString:', error);
        return 0;
      }
    } 
    // Try alloc_string (Rust-style naming)
    else if (this.hasFunction('alloc_string')) {
      this.debug('Using alloc_string function');
      try {
        ptr = this.callFunction('alloc_string', bytes.length);
        this.registerAllocation(ptr);
      } catch (error) {
        this.debug('Error allocating memory with alloc_string:', error);
        return 0;
      }
    }
    // Try using standard malloc if available
    else if (this.hasFunction('malloc')) {
      this.debug('Using malloc function');
      try {
        // We need an extra byte for the null terminator with malloc
        ptr = this.callFunction('malloc', bytes.length + 1);
        this.registerAllocation(ptr);
      } catch (error) {
        this.debug('Error allocating memory with malloc:', error);
        return 0;
      }
    } 
    // As a fallback, try any other allocation function we can find
    else {
      this.debug('No standard allocation function found, searching for alternatives');
      
      const allocationFunctions = [
        'alloc_buffer', 'allocBuffer', 'alloc', 'create_string',
        'createString', 'string_alloc', 'stringAlloc'
      ];
      
      for (const funcName of allocationFunctions) {
        if (this.hasFunction(funcName)) {
          this.debug(`Found alternative allocation function: ${funcName}`);
          try {
            ptr = this.callFunction(funcName, bytes.length);
            if (ptr !== 0) {
              this.registerAllocation(ptr);
              break;
            }
          } catch (error) {
            this.debug(`Error allocating memory with ${funcName}:`, error);
          }
        }
      }
    }
    
    if (ptr === 0) {
      this.debug('Failed to allocate memory for string');
      return 0;
    }
    
    // Copy the string data to WASM memory
    try {
      const memory = new Uint8Array(this.memory.buffer);
      
      // Check buffer bounds
      if (ptr + bytes.length > memory.length) {
        this.debug(`Memory allocation error: ptr (${ptr}) + length (${bytes.length}) exceeds memory size (${memory.length})`);
        this.deallocateString(ptr);
        return 0;
      }
      
      // Copy the bytes
      for (let i = 0; i < bytes.length; i++) {
        memory[ptr + i] = bytes[i];
      }
      
      // Add a null terminator if we're using malloc-style allocation
      if (this.hasFunction('malloc')) {
        memory[ptr + bytes.length] = 0;
      }
      
      this.debug(`String written to memory at pointer ${ptr}`);
      return ptr;
    } catch (error) {
      this.debug('Error writing string to memory:', error);
      this.deallocateString(ptr);
      return 0;
    }
  }

  /**
   * Deallocate a string from memory with proper handling for MSGB format
   * 
   * @param ptr Pointer to the memory location
   */
  deallocateString(ptr: number): void {
    if (ptr === 0) {
      this.debug('Ignoring request to deallocate null pointer');
      return;
    }
    
    this.debug(`Deallocating string at pointer ${ptr}`);
    
    // Check if this pointer is in our tracking system
    const isTracked = this.allocatedPointers.has(ptr);
    if (!isTracked) {
      this.debug(`Warning: Attempting to deallocate untracked pointer ${ptr}`);
    }
    
    // Perform memory analysis before deallocation
    try {
      // Analyze memory at that pointer to see what we're about to deallocate
      const memory = new Uint8Array(this.memory.buffer);
      if (ptr < memory.length) {
        const headerBytes = Array.from(memory.subarray(ptr, Math.min(ptr + 16, memory.length)))
          .map(b => b.toString(16).padStart(2, '0'))
          .join(' ');
        this.debug(`Memory to deallocate at ptr ${ptr} starts with bytes: ${headerBytes}`);
        
        // Check if it's a MSGB string
        if (this.hasMsgbSignature(ptr)) {
          this.debug('MSGB signature found at deallocation pointer');
          const dataView = new DataView(this.memory.buffer);
          const length = dataView.getUint32(ptr + 4, true); // true = little endian
          this.debug(`String length from MSGB format: ${length}`);
        } else {
          this.debug('Not a MSGB string, trying to estimate length');
          const length = this.estimateStringLength(ptr);
          this.debug(`Estimated string length: ${length}`);
        }
      } else {
        this.debug(`Pointer ${ptr} is outside memory bounds (${memory.length})`);
      }
    } catch (analyzeError) {
      this.debug(`Error analyzing memory before deallocation: ${analyzeError}`);
    }
    
    // First try to use the module's custom deallocation function
    try {
      // Try deallocString (most common name)
      if (this.hasFunction('deallocString')) {
        this.debug('Using deallocString function');
        try {
          this.callFunction('deallocString', ptr);
          this.debug('deallocString call completed successfully');
          this.unregisterAllocation(ptr);
          return;
        } catch (error) {
          this.debug(`Error calling deallocString: ${error}`);
          throw error; // Re-throw to be caught by outer try-catch
        }
      } 
      // Try dealloc_string (Rust-style naming)
      else if (this.hasFunction('dealloc_string')) {
        this.debug('Using dealloc_string function');
        
        // First try with length 0 since this works in cases where string length isn't needed
        try {
          this.debug('Trying dealloc_string with length 0');
          this.callFunction('dealloc_string', ptr, 0);
          this.debug('dealloc_string call with length 0 completed successfully');
          this.unregisterAllocation(ptr);
          return;
        } catch (error) {
          this.debug(`dealloc_string with length 0 failed: ${error}`);
          
          // If that failed, fall back to using estimated length
          try {
            const len = this.estimateStringLength(ptr);
            this.debug(`Retrying with estimated length: ${len}`);
            this.callFunction('dealloc_string', ptr, len);
            this.debug('dealloc_string call with estimated length completed successfully');
            this.unregisterAllocation(ptr);
            return;
          } catch (retryError) {
            this.debug(`Retry with estimated length also failed: ${retryError}`);
            throw error; // Re-throw original error
          }
        }
      }
      // Try standard free if available
      else if (this.hasFunction('free')) {
        this.debug('Using free function');
        try {
          this.callFunction('free', ptr);
          this.debug('free call completed successfully');
          this.unregisterAllocation(ptr);
          return;
        } catch (error) {
          this.debug(`Error calling free: ${error}`);
          throw error; // Re-throw the error
        }
      }
      // As a fallback, try any other deallocation function we can find
      else {
        this.debug('No standard deallocation function found, searching for alternatives');
        
        const deallocationFunctions = [
          'dealloc_buffer', 'deallocBuffer', 'dealloc', 'destroy_string',
          'destroyString', 'string_dealloc', 'stringDealloc', 'free_string', 'freeString'
        ];
        
        for (const funcName of deallocationFunctions) {
          if (this.hasFunction(funcName)) {
            this.debug(`Found alternative deallocation function: ${funcName}`);
            try {
              // Try with and without length parameter
              let len = this.estimateStringLength(ptr);
              if (funcName.includes('buffer') || funcName.includes('Buffer')) {
                this.callFunction(funcName, ptr, len);
              } else {
                this.callFunction(funcName, ptr);
              }
              this.debug(`${funcName} call completed successfully`);
              this.unregisterAllocation(ptr);
              return;
            } catch (error) {
              this.debug(`Error deallocating memory with ${funcName}: ${error}`);
              // Continue to try other functions
            }
          }
        }
        
        this.debug('No deallocation function found or all attempts failed, memory may leak');
        // Still unregister it from our tracking even if we couldn't deallocate
        this.unregisterAllocation(ptr);
      }
    } catch (error) {
      this.debug(`Error deallocating memory: ${error}`);
      // Still unregister it from our tracking even on error
      this.unregisterAllocation(ptr);
    }
  }
  
  /**
   * Estimate the length of a null-terminated string
   * @param ptr Pointer to the string
   * @returns Estimated length
   */
  private estimateStringLength(ptr: number): number {
    if (ptr === 0) return 0;
    
    const memory = new Uint8Array(this.memory.buffer);
    let len = 0;
    
    // Look for null terminator or MSGB signature
    if (this.hasMsgbSignature(ptr)) {
      // Read MSGB format length
      const lengthBuffer = memory.buffer.slice(ptr + 4, ptr + 8);
      len = new DataView(lengthBuffer).getUint32(0, true);
      return len;
    }
    
    // Assume null-terminated string, scan for null byte
    for (let i = 0; ptr + i < memory.length && i < MAX_STRING_LENGTH; i++) {
      if (memory[ptr + i] === 0) {
        return i;
      }
    }
    
    // Default to 100 as a fallback
    return 100;
  }

  /**
   * Call a WebAssembly function that returns a pointer to a string
   * 
   * @param funcName Name of the function to call
   * @param args Arguments to pass to the function
   * @returns The string value
   */
  callStringFunction(funcName: string, ...args: any[]): string {
    try {
      this.debug(`Calling function ${funcName} with args:`, args);
      
      // Call the function
      const ptr = this.callFunction(funcName, ...args);
      this.debug(`Function ${funcName} returned pointer: ${ptr}`);
      
      if (ptr === 0) {
        this.debug(`Function ${funcName} returned null pointer`);
        return '';
      }
      
      // Read the string
      const result = this.readString(ptr);
      
      // Deallocate the string
      this.deallocateString(ptr);
      
      return result;
    } catch (error) {
      this.debug(`Error calling string function ${funcName}:`, error);
      return JSON.stringify({
        error: `Failed to call function ${funcName}: ${error}`
      });
    }
  }
}

/**
 * Create a memory manager for a WebAssembly instance
 * 
 * @param instance WebAssembly instance
 * @param debug Whether to enable debug logging
 * @returns Memory manager instance
 */
export function createMemoryManager(
  instance: WebAssembly.Instance, 
  debug = false
): WasmMemoryManager {
  if (!instance.exports.memory) {
    throw new Error('WebAssembly instance does not export memory');
  }
  
  const memory = instance.exports.memory as WebAssembly.Memory;
  return new WasmMemoryManager(memory, instance.exports, debug);
} 