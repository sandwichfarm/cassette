/**
 * Utility functions for environment detection and handling
 */

/**
 * Check if the code is running in a browser environment
 */
export function isBrowser(): boolean {
  return typeof window !== 'undefined' && typeof document !== 'undefined';
}

/**
 * Check if the code is running in a Node.js environment
 */
export function isNode(): boolean {
  return typeof process !== 'undefined' && 
         process.versions != null && 
         process.versions.node != null;
}

/**
 * Read a file in Node.js environment
 * @param path Path to the file
 * @returns Promise that resolves with the file contents as ArrayBuffer
 */
export async function readFileNode(path: string): Promise<ArrayBuffer> {
  if (!isNode()) {
    throw new Error('readFileNode can only be used in Node.js environment');
  }
  
  try {
    // Dynamic import to avoid issues in browser environment
    const { readFile } = await import('fs/promises');
    const buffer = await readFile(path);
    // Create a fresh ArrayBuffer to comply with type requirements
    return new Uint8Array(buffer).buffer as ArrayBuffer;
  } catch (error: any) {
    throw new Error(`Failed to read file: ${error.message || error}`);
  }
}

/**
 * Fetch a file or URL
 * @param url URL to fetch
 * @returns Promise that resolves with the file contents as ArrayBuffer
 */
export async function fetchFile(url: string): Promise<ArrayBuffer> {
  try {
    // In browser, use fetch API
    if (isBrowser()) {
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Failed to fetch ${url}: ${response.status} ${response.statusText}`);
      }
      const buffer = await response.arrayBuffer();
      // Type assertion to ensure ArrayBuffer type
      return buffer as ArrayBuffer;
    }
    
    // In Node.js, try to use fetch or fallback to fs
    if (isNode()) {
      try {
        // Node.js v18+ has native fetch
        const response = await fetch(url);
        if (!response.ok) {
          throw new Error(`Failed to fetch ${url}: ${response.status} ${response.statusText}`);
        }
        const buffer = await response.arrayBuffer();
        // Type assertion to ensure ArrayBuffer type
        return buffer as ArrayBuffer;
      } catch (error) {
        // Fallback to fs if it's a local file
        if (url.startsWith('file://') || !url.includes('://')) {
          const filePath = url.startsWith('file://') ? url.slice(7) : url;
          return await readFileNode(filePath);
        }
        throw error;
      }
    }
    
    throw new Error('Unsupported environment');
  } catch (error: any) {
    throw new Error(`Failed to fetch file: ${error.message || error}`);
  }
}

/**
 * Generate a unique ID for a cassette
 * @param fileName Original file name of the cassette
 * @returns A unique ID
 */
export function generateCassetteId(fileName: string): string {
  // Extract base name without extension
  const baseName = fileName.replace(/\.[^/.]+$/, "");
  // Convert to a valid identifier
  const validId = baseName.replace(/[^a-zA-Z0-9_]/g, '_').toLowerCase();
  // Add timestamp to ensure uniqueness
  return `${validId}_${Date.now()}`;
}

/**
 * Create a debug logger that only logs when debug mode is enabled
 * @param enabled Whether debug mode is enabled
 * @param prefix Prefix for log messages
 * @returns Object with logging methods
 */
export function createLogger(enabled: boolean = false, prefix: string = 'Cassette') {
  return {
    log: (...args: any[]) => {
      if (enabled) {
        console.log(`[${prefix}]`, ...args);
      }
    },
    error: (...args: any[]) => {
      if (enabled) {
        console.error(`[${prefix} ERROR]`, ...args);
      }
    },
    warn: (...args: any[]) => {
      if (enabled) {
        console.warn(`[${prefix} WARN]`, ...args);
      }
    },
  };
}

/**
 * Convert data to ArrayBuffer regardless of its type
 * @param data Data to convert
 * @returns Promise that resolves with the data as ArrayBuffer
 */
export async function toArrayBuffer(data: File | string | ArrayBuffer | Uint8Array): Promise<ArrayBuffer> {
  // If it's already an ArrayBuffer, return it
  if (data instanceof ArrayBuffer) {
    return data;
  }
  
  // If it's a typed array, get its buffer
  if (ArrayBuffer.isView(data)) {
    const buffer = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength);
    // Type assertion to ensure ArrayBuffer type
    return buffer as ArrayBuffer;
  }
  
  // If it's a File, read it
  if (typeof File !== 'undefined' && data instanceof File) {
    return new Promise<ArrayBuffer>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        if (reader.result instanceof ArrayBuffer) {
          resolve(reader.result);
        } else {
          reject(new Error('Failed to read file as ArrayBuffer'));
        }
      };
      reader.onerror = () => reject(new Error('Failed to read file'));
      reader.readAsArrayBuffer(data);
    });
  }
  
  // If it's a string, it could be a URL or a file path
  if (typeof data === 'string') {
    return await fetchFile(data);
  }
  
  throw new Error('Unsupported data type');
}

import { EventTracker } from './types.js';

/**
 * Creates an event tracker for deduplicating events
 * @returns An EventTracker instance
 */
export function createEventTracker(): EventTracker {
  const eventIds = new Set<string>();
  
  return {
    eventIds,
    
    reset() {
      eventIds.clear();
    },
    
    addAndCheck(id: string): boolean {
      if (eventIds.has(id)) {
        return false; // Already seen this event
      }
      
      eventIds.add(id);
      return true; // New event
    },
    
    filterDuplicates(events: any[]): any[] {
      if (!events || !Array.isArray(events)) {
        return events;
      }
      
      return events.filter(event => {
        if (!event || typeof event !== 'object' || !event.id) {
          return true; // Not a valid event object, keep it
        }
        
        return this.addAndCheck(event.id);
      });
    }
  };
}

/**
 * Process a cassette response according to NIP-01 protocol
 * Handles parsing, validation, and processing responses
 * 
 * @param response Raw response string from a cassette
 * @param subscriptionId The subscription ID to associate with events
 * @param logger Optional logger for debugging
 * @param eventTracker Optional event tracker for deduplication
 * @returns Object with processed events and notices
 */
export function processCassetteResponse(
  response: string, 
  subscriptionId: string,
  logger = createLogger(false),
  eventTracker?: EventTracker
): { 
  events: string[], 
  eose: boolean,
  notices: string[] 
} {
  if (!response) {
    logger.log('Empty response from cassette');
    return { events: [], eose: false, notices: [] };
  }

  logger.log(`Processing cassette response: ${response}`);
  const result = {
    events: [] as string[],
    eose: false,
    notices: [] as string[]
  };

  try {
    // Handle single response
    try {
      const parsedResponse = JSON.parse(response);
      if (Array.isArray(parsedResponse)) {
        if (parsedResponse[0] === "EOSE") {
          result.eose = true;
          // Don't push EOSE to events array, just set the flag
        } else if (parsedResponse[0] === "EVENT" && parsedResponse.length >= 3) {
          // Check for duplicates if we have an event tracker
          if (eventTracker && parsedResponse[2] && parsedResponse[2].id) {
            const eventId = parsedResponse[2].id;
            if (eventTracker.addAndCheck(eventId)) {
              // Only forward if it's not a duplicate
              result.events.push(response);
            } else {
              logger.log(`Filtered duplicate event: ${eventId}`);
              // Don't send NOTICE for deduplication, just log it
            }
          } else {
            // No event tracker, forward all events
            result.events.push(response);
          }
        } else if (parsedResponse[0] === "NOTICE") {
          // Only forward NOTICE messages that came from the cassette
          result.notices.push(response);
        }
      }
    } catch (parseError) {
      logger.log(`Error parsing response: ${parseError}`);
    }
  } catch (error) {
    logger.log(`Error processing response: ${error}`);
  }
  
  return result;
} 