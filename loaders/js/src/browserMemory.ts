/**
 * Browser-specific memory management utilities for WebAssembly cassettes
 */

import { createLogger } from './utils.js';

/**
 * Browser-optimized memory manager for WebAssembly modules
 * Includes enhanced error handling and format detection for string operations
 */
export interface BrowserWasmMemoryManager {
  readString(ptr: number): string;
  writeString(str: string): number;
  deallocateString(ptr: number): void;
  callStringFunction(funcName: string, ...args: any[]): string;
}

/**
 * Creates a memory manager optimized for browser environments
 * This uses a simplified approach to memory management with more
 * robust error handling for WebAssembly interaction
 */
export function createBrowserMemoryManager(instance: WebAssembly.Instance, debug = false): BrowserWasmMemoryManager {
  const memory = instance.exports.memory as WebAssembly.Memory;
  const exports = instance.exports;
  const logger = createLogger(debug, 'BrowserMemoryManager');
  
  // Find the best deallocation function available
  const findDeallocFunction = () => {
    const deallocFnNames = ['dealloc_string', 'free', 'dealloc'];
    for (const name of deallocFnNames) {
      if (typeof exports[name] === 'function') {
        logger.log(`Using ${name} function for deallocation`);
        return exports[name] as Function;
      }
    }
    return null;
  };
  
  const deallocFn = findDeallocFunction();
  
  return {
    /**
     * Read a string from WebAssembly memory
     * @param ptr Pointer to the string in memory
     * @returns The string value
     */
    readString(ptr: number): string {
      if (ptr === 0) {
        return '';
      }
      
      try {
        // Get the memory buffer
        const view = new Uint8Array(memory.buffer);
        
        // Detect string format
        // First check if it's a 4-byte length prefixed string (common format)
        if (ptr + 4 <= view.length) {
          try {
            const lengthBytes = new Uint8Array(memory.buffer, ptr, 4);
            const length = new DataView(lengthBytes.buffer, lengthBytes.byteOffset, 4).getUint32(0, true);
            
            // Sanity check on length
            if (length > 0 && length < 100000000 && ptr + 4 + length <= view.length) {
              const bytes = new Uint8Array(memory.buffer, ptr + 4, length);
              return new TextDecoder('utf-8').decode(bytes);
            }
          } catch (e) {
            logger.error(`Error reading length-prefixed string: ${e}`);
            // Fall through to null-terminated approach
          }
        }
        
        // Fall back to null-terminated string (common for C-style strings)
        let end = ptr;
        while (end < view.length && view[end] !== 0) {
          end++;
        }
        
        if (end > ptr) {
          const bytes = new Uint8Array(memory.buffer, ptr, end - ptr);
          return new TextDecoder('utf-8').decode(bytes);
        }
        
        logger.error(`Could not determine string format at pointer ${ptr}`);
        return '';
      } catch (error) {
        logger.error(`Error reading string: ${error}`);
        return '';
      }
    },
    
    /**
     * Write a string to WebAssembly memory
     * This is a simplified version that just returns the pointer
     * and doesn't handle complex allocation
     */
    writeString(str: string): number {
      // For browser implementations, we use a simpler model:
      // Just call the alloc_string function if available
      try {
        if (typeof exports.alloc_string === 'function') {
          const encoder = new TextEncoder();
          const bytes = encoder.encode(str);
          const ptr = (exports.alloc_string as Function)(bytes.length);
          
          // Write the string to memory
          const view = new Uint8Array(memory.buffer);
          for (let i = 0; i < bytes.length; i++) {
            view[ptr + i] = bytes[i];
          }
          
          return ptr;
        }
        
        logger.error('No alloc_string function found');
        return 0;
      } catch (error) {
        logger.error(`Error writing string: ${error}`);
        return 0;
      }
    },
    
    /**
     * Deallocate a string from WebAssembly memory
     * Enhanced for browser environments with better error handling
     */
    deallocateString(ptr: number): void {
      if (ptr === 0 || !deallocFn) {
        return;
      }
      
      try {
        // Use a safe wrapper that won't throw on errors
        const safeDeallocate = (p: number, len: number = 0) => {
          try {
            // If we don't have a length, just pass the pointer
            if (len === 0) {
              deallocFn(p);
            } else {
              deallocFn(p, len);
            }
            return true;
          } catch (e) {
            logger.log(`[CoreCassetteInterface] [req] Deallocation failed: ${e}`);
            return false;
          }
        };
        
        // Try to deallocate with a reasonable length if needed
        // The length isn't critical for most deallocation implementations
        safeDeallocate(ptr, 1);
      } catch (e) {
        logger.log(`Safe deallocation wrapper failed: ${e}`);
        // Continue execution despite errors
      }
    },
    
    /**
     * Call a function that returns a string, handling memory management
     */
    callStringFunction(funcName: string, ...args: any[]): string {
      try {
        if (typeof exports[funcName] !== 'function') {
          logger.error(`Function ${funcName} not found`);
          return '';
        }
        
        const result = (exports[funcName] as Function)(...args);
        if (result === 0) {
          return '';
        }
        
        const str = this.readString(result);
        this.deallocateString(result);
        return str;
      } catch (error) {
        logger.error(`Error calling string function ${funcName}: ${error}`);
        return '';
      }
    }
  };
} 