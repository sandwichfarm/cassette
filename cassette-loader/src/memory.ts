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
   * Read a string from memory using the enhanced format with signature
   * The enhanced format includes a magic signature to detect truncation:
   * [4-byte signature "MSGB"][4-byte length][string data][null terminator]
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
      const memory = new Uint8Array(this.memory.buffer);
      
      // Enhanced logging: Show the first 16 bytes at the pointer location for debugging
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
      
      // Check for the magic signature "MSGB" (0x4D, 0x53, 0x47, 0x42)
      // This signature is added by the enhanced string_to_ptr function
      if (ptr + 8 <= memory.length && 
          memory[ptr] === 0x4D && memory[ptr + 1] === 0x53 && 
          memory[ptr + 2] === 0x47 && memory[ptr + 3] === 0x42) {
        
        this.debug('Detected enhanced string format with MSGB signature');
        
        // Read the length from bytes 4-7 (after signature)
        const lengthBytes = memory.subarray(ptr + 4, ptr + 8);
        const length = new DataView(lengthBytes.buffer, lengthBytes.byteOffset, 4).getUint32(0, true);
        this.debug(`String length from enhanced format: ${length}`);
        
        if (length <= 0 || length > 10000000) { // Sanity check for length
          this.debug(`Invalid string length: ${length}, returning empty string`);
          return '';
        }
        
        // Calculate the start of the actual string data (after signature and length)
        const strPtr = ptr + 8;
        
        if (strPtr >= memory.length) {
          this.debug(`String pointer ${strPtr} out of bounds for memory length ${memory.length}`);
          return '';
        }
        
        // Log the actual string bytes
        const stringBytes = Array.from(memory.subarray(strPtr, Math.min(strPtr + Math.min(32, length), memory.length)))
          .map(b => b.toString(16).padStart(2, '0'))
          .join(' ');
        this.debug(`First ${Math.min(32, length)} bytes of string data: ${stringBytes}`);
        
        // Read the string data with bounds checking
        const endPtr = Math.min(strPtr + length, memory.length);
        const stringData = memory.subarray(strPtr, endPtr);
        
        try {
          const result = this.decoder.decode(stringData);
          this.debug(`Read string from enhanced format (length ${length}): ${result.substring(0, 50)}${result.length > 50 ? '...' : ''}`);
          return result;
        } catch (decodeError) {
          this.debug(`Error decoding string: ${decodeError}`);
          return '';
        }
      }
      
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
        
        // Log the actual string bytes
        const stringBytes = Array.from(memory.subarray(strPtr, Math.min(strPtr + Math.min(32, length), memory.length)))
          .map(b => b.toString(16).padStart(2, '0'))
          .join(' ');
        this.debug(`First ${Math.min(32, length)} bytes of string data: ${stringBytes}`);
        
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
      
      // Log the raw length bytes
      const rawLengthBytes = Array.from(memory.subarray(ptr, ptr + 4))
        .map(b => b.toString(16).padStart(2, '0'))
        .join(' ');
      this.debug(`Raw length bytes: ${rawLengthBytes}`);
      
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
      
      // Log the actual string bytes
      const stringBytes = Array.from(memory.subarray(strPtr, Math.min(strPtr + Math.min(32, length), memory.length)))
        .map(b => b.toString(16).padStart(2, '0'))
        .join(' ');
      this.debug(`First ${Math.min(32, length)} bytes of string data: ${stringBytes}`);
      
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
   * Deallocate a string from memory, with enhanced handling for different string formats
   * and resilience to failures
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
      
      const memory = new Uint8Array(this.memory.buffer);
      
      // First, detect the string format to determine the correct length
      let length = 0;
      let originalPtr = ptr;
      
      // Check for enhanced string format with MSGB signature
      if (ptr + 8 <= memory.length && 
          memory[ptr] === 0x4D && memory[ptr + 1] === 0x53 && 
          memory[ptr + 2] === 0x47 && memory[ptr + 3] === 0x42) {
        
        this.debug('Detected enhanced string format for deallocation');
        
        // Read length from bytes 4-7 (after signature)
        try {
          const lengthBytes = memory.subarray(ptr + 4, ptr + 8);
          length = new DataView(lengthBytes.buffer, lengthBytes.byteOffset, 4).getUint32(0, true);
          this.debug(`Enhanced format string length for deallocation: ${length}`);
          
          // For deallocation, we need to include the signature and length prefix
          // so the original pointer is already correct
        } catch (enhancedLenError) {
          this.debug(`Error reading enhanced format length: ${enhancedLenError}`);
          // Continue with other methods
        }
      }
      
      // If length is still 0, try using get_string_len function
      if (length === 0 && this.hasFunction('get_string_len')) {
        try {
          length = this.callFunction('get_string_len', ptr);
          this.debug(`Got string length from get_string_len: ${length}`);
        } catch (lenError) {
          this.debug(`Error calling get_string_len: ${lenError}`);
          // Continue with manual method
        }
      } 
      
      // If length is still 0, try manual reading of standard 4-byte prefix
      if (length === 0) {
        // Manually read length prefix
        try {
          // Check if ptr is valid
          if (ptr + 4 > memory.length) {
            this.debug(`Pointer ${ptr} out of bounds for memory length ${memory.length}`);
            return;
          }
          
          const lengthBytes = memory.subarray(ptr, ptr + 4);
          length = new DataView(lengthBytes.buffer, lengthBytes.byteOffset, 4).getUint32(0, true);
          this.debug(`Got string length from manual reading: ${length}`);
        } catch (manualLenError) {
          this.debug(`Error manually reading string length: ${manualLenError}`);
        }
      }
      
      // Sanity check for length
      if (length <= 0 || length > 10000000) {
        this.debug(`Invalid or missing string length (${length}), attempting safe deallocation anyway`);
        // Even with invalid length, try to deallocate with a reasonable default
        // This is better than skipping deallocation entirely
        length = 1; // Use minimal length just to call the function
      }
      
      this.debug(`Deallocating string at pointer ${originalPtr} with length ${length}`);
      
      // Make the actual deallocation call, with try/catch and fallbacks
      try {
        // First verify the function is callable
        if (typeof this.exports.dealloc_string !== 'function') {
          this.debug('dealloc_string is not a function, skipping deallocation');
          return;
        }
        
        // Create a safe wrapper function that won't throw on error
        const safeDealloc = (p: number, l: number) => {
          try {
            return (this.exports.dealloc_string as Function).call(null, p, l);
          } catch (e) {
            this.debug(`Deallocation failed but caught error: ${e}`);
            return undefined;
          }
        };
        
        // Try to deallocate
        safeDealloc(originalPtr, length);
        this.debug('Deallocation attempt completed');
      } catch (deallocError) {
        this.debug(`Error during deallocation process: ${deallocError}`);
        // Continue execution despite errors
      }
    } catch (error) {
      // Log but don't throw - we want to continue even if deallocation fails
      this.debug(`Error in deallocateString process: ${error}`);
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