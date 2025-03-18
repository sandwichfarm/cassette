/**
 * Memory management utilities for WebAssembly cassettes
 */

/**
 * Handles memory interactions between JavaScript and WebAssembly
 */
export class WasmMemoryManager {
  private memory: WebAssembly.Memory;
  private exports: WebAssembly.Exports;
  private decoder = new TextDecoder('utf-8');
  private encoder = new TextEncoder();
  private debugMode: boolean;

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
   * Call a function in the exports
   */
  private callFunction(name: string, ...args: any[]): any {
    if (!this.hasFunction(name)) {
      throw new Error(`Function ${name} not found in WebAssembly exports`);
    }
    return (this.exports[name as keyof typeof this.exports] as Function)(...args);
  }

  /**
   * Read a string from memory using the new 4-byte length prefix format
   * @param ptr Pointer to the memory location
   * @returns The string value
   */
  readString(ptr: number): string {
    if (ptr === 0) {
      this.debug('Received null pointer');
      return '';
    }

    try {
      const memory = new Uint8Array(this.memory.buffer);
      
      // First check if we can use the get_string_len and get_string_ptr functions
      if (this.hasFunction('get_string_len') && this.hasFunction('get_string_ptr')) {
        // Get the string length using the helper function
        const length = this.callFunction('get_string_len', ptr);
        this.debug(`String length from get_string_len: ${length}`);
        
        if (length <= 0 || length > 10000000) { // Sanity check for length
          this.debug(`Invalid string length: ${length}, returning empty string`);
          return '';
        }
        
        // Get pointer to the actual string data (after length prefix)
        const strPtr = this.callFunction('get_string_ptr', ptr);
        this.debug(`String pointer from get_string_ptr: ${strPtr}`);
        
        if (strPtr <= 0 || strPtr >= memory.length) {
          this.debug(`Invalid string pointer: ${strPtr}, returning empty string`);
          return '';
        }
        
        // Read the string data with bounds checking
        const endPtr = Math.min(strPtr + length, memory.length);
        const stringData = memory.subarray(strPtr, endPtr);
        const result = this.decoder.decode(stringData);
        this.debug(`Read string (length ${length}): ${result.substring(0, 50)}${result.length > 50 ? '...' : ''}`);
        
        return result;
      }
      
      // Fallback: manually parse the 4-byte length prefix
      if (ptr + 4 > memory.length) {
        this.debug(`Pointer ${ptr} out of bounds for memory length ${memory.length}`);
        return '';
      }
      
      // Read the length prefix (4 bytes in little-endian)
      const lengthBytes = memory.subarray(ptr, ptr + 4);
      const length = new DataView(lengthBytes.buffer, lengthBytes.byteOffset, 4).getUint32(0, true);
      this.debug(`String length from manual parsing: ${length}`);
      
      if (length <= 0 || length > 10000000) { // Sanity check for length
        this.debug(`Invalid string length: ${length}, returning empty string`);
        return '';
      }
      
      // Calculate the start of the actual string data (after the 4-byte prefix)
      const strPtr = ptr + 4;
      
      if (strPtr >= memory.length) {
        this.debug(`String pointer ${strPtr} out of bounds for memory length ${memory.length}`);
        return '';
      }
      
      // Read the string data with bounds checking
      const endPtr = Math.min(strPtr + length, memory.length);
      const stringData = memory.subarray(strPtr, endPtr);
      
      try {
        const result = this.decoder.decode(stringData);
        this.debug(`Read string (length ${length}): ${result.substring(0, 50)}${result.length > 50 ? '...' : ''}`);
        return result;
      } catch (decodeError) {
        this.debug(`Error decoding string: ${decodeError}`);
        return '';
      }
    } catch (error) {
      console.error('Error reading string from WebAssembly memory:', error);
      return '';
    }
  }

  /**
   * Write a string to memory
   * @param str String to write
   * @returns Pointer to the memory location
   */
  writeString(str: string): number {
    try {
      const bytes = this.encoder.encode(str);
      this.debug(`Writing string (length ${bytes.length}): ${str.substring(0, 50)}${str.length > 50 ? '...' : ''}`);
      
      // Check if we can use string_to_ptr function directly
      if (this.hasFunction('string_to_ptr')) {
        this.debug('Using string_to_ptr function');
        return this.callFunction('string_to_ptr', str);
      }
      
      // Fallback: use alloc_string and manual memory writing
      if (this.hasFunction('alloc_string')) {
        // Allocate memory (include space for length prefix and null terminator)
        const totalLength = 4 + bytes.length + 1;
        const ptr = this.callFunction('alloc_string', totalLength);
        this.debug(`Allocated memory at pointer: ${ptr}, total length: ${totalLength}`);
        
        // Write to memory
        const memory = new Uint8Array(this.memory.buffer);
        
        // Write length prefix (4 bytes, little-endian)
        const view = new DataView(memory.buffer);
        view.setUint32(ptr, bytes.length, true);
        
        // Write string data
        for (let i = 0; i < bytes.length; i++) {
          memory[ptr + 4 + i] = bytes[i];
        }
        
        // Add null terminator
        memory[ptr + 4 + bytes.length] = 0;
        
        return ptr;
      }
      
      throw new Error('No string allocation functions found in WebAssembly exports');
    } catch (error) {
      console.error('Error writing string to WebAssembly memory:', error);
      throw error;
    }
  }

  /**
   * Deallocate a string from memory
   * @param ptr Pointer to the memory location
   */
  deallocateString(ptr: number): void {
    if (ptr === 0) {
      return;
    }

    try {
      // Check if dealloc_string exists and is actually callable
      if (!this.hasFunction('dealloc_string')) {
        this.debug('No dealloc_string function available, skipping deallocation');
        return;
      }
      
      // Get string length first
      let length = 0;
      
      if (this.hasFunction('get_string_len')) {
        try {
          length = this.callFunction('get_string_len', ptr);
          this.debug(`Got string length from get_string_len: ${length}`);
        } catch (lenError) {
          console.error(`Error calling get_string_len: ${lenError}`);
          // Continue with manual method
        }
      } 
      
      if (length === 0) {
        // Manually read length prefix
        try {
          const memory = new Uint8Array(this.memory.buffer);
          // Check if ptr is valid
          if (ptr + 4 > memory.length) {
            this.debug(`Pointer ${ptr} out of bounds for memory length ${memory.length}`);
            return;
          }
          
          const lengthBytes = memory.subarray(ptr, ptr + 4);
          length = new DataView(lengthBytes.buffer, lengthBytes.byteOffset, 4).getUint32(0, true);
          this.debug(`Got string length from manual reading: ${length}`);
          
          // Sanity check for length
          if (length <= 0 || length > 10000000) {
            this.debug(`Invalid string length: ${length}, skipping deallocation`);
            return;
          }
        } catch (manualLenError) {
          console.error(`Error manually reading string length: ${manualLenError}`);
          // If we can't determine length, skip deallocation
          console.warn('Unable to determine string length for deallocation, skipping');
          return;
        }
      }
      
      this.debug(`Deallocating string at pointer ${ptr} with length ${length}`);
      
      // Verify that the function is callable with the expected signature
      try {
        // Try to call with safe arguments first as a test
        const testResult = typeof this.exports.dealloc_string === 'function' ? 
          (this.exports.dealloc_string as Function).call(null, 0, 0) : undefined;
        
        // If we get here, then the function is callable
        this.callFunction('dealloc_string', ptr, length);
        this.debug('Successfully deallocated string');
      } catch (deallocError) {
        // Don't try to deallocate if we got an error with the test call
        console.error('Error deallocating string from WebAssembly memory:', deallocError);
        console.warn('Memory may not be properly deallocated, but continuing execution');
      }
    } catch (error) {
      console.error('Error in deallocateString process:', error);
      console.warn('Continuing execution despite deallocation failure');
    }
  }

  /**
   * Call a WebAssembly function that returns a pointer to a string
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
      console.error(`Error calling WebAssembly function ${funcName}:`, error);
      return JSON.stringify({
        error: `Failed to call function ${funcName}: ${error}`
      });
    }
  }
}

/**
 * Create a memory manager for a WebAssembly instance
 * @param instance WebAssembly instance
 * @param debug Whether to enable debug logging
 * @returns Memory manager instance
 */
export function createMemoryManager(
  instance: WebAssembly.Instance, 
  debug = false
): WasmMemoryManager {
  const memory = instance.exports.memory as WebAssembly.Memory;
  return new WasmMemoryManager(memory, instance.exports, debug);
} 