import { loadCassette } from './loader.js';
import { createMemoryManager } from './memory.js';
import { createLogger, generateCassetteId, isNode, isBrowser } from './utils.js';
import { WasmMemoryManager } from './memory.js';
import { Cassette, CassetteLoaderOptions } from './types.js';
import { isWebAssemblySupported, ENV_INFO } from './index.js';
import { processCassetteResponse } from './utils.js';

/**
 * Manager class for loading and working with WebAssembly cassettes
 */
export class CassetteManager {
  // Registry of loaded cassettes
  private cassettes: Map<string, Cassette> = new Map();
  
  // Event listeners for cassette-related events
  private listeners: Map<string, Array<(data: any) => void>> = new Map();
  
  // Logger for debugging
  private logger: ReturnType<typeof createLogger>;
  
  // Options
  private options: CassetteLoaderOptions;
  
  // Track subscription state
  private subscriptionState = new Map<string, Set<string>>();
  
  /**
   * Create a new cassette manager
   * @param options Options for the cassette manager
   */
  constructor(options: CassetteLoaderOptions = {}) {
    // Check if WebAssembly is supported
    if (!isWebAssemblySupported()) {
      console.error("WebAssembly is not supported in this environment");
      throw new Error("WebAssembly is not supported in this environment");
    }
    
    this.options = {
      debug: false,
      deduplicateEvents: true,
      ...options
    };
    
    this.logger = createLogger(this.options.debug || false, 'CassetteManager');
    this.logger.log('CassetteManager initialized');
    
    // Apply environment override if requested
    if (this.options.forceNodeEnvironment) {
      this.logger.log('Forcing Node.js environment mode');
    }
    
    this.logger.log("Environment:", ENV_INFO);
  }
  
  /**
   * Add an event listener
   * @param event Event name
   * @param callback Callback function
   */
  public addEventListener(event: string, callback: (data: any) => void): void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, []);
    }
    this.listeners.get(event)?.push(callback);
  }
  
  /**
   * Remove an event listener
   * @param event Event name
   * @param callback Callback function
   */
  public removeEventListener(event: string, callback: (data: any) => void): void {
    if (!this.listeners.has(event)) return;
    
    const eventListeners = this.listeners.get(event);
    if (!eventListeners) return;
    
    const index = eventListeners.indexOf(callback);
    if (index !== -1) {
      eventListeners.splice(index, 1);
    }
  }
  
  /**
   * Emit an event
   * @param event Event name
   * @param data Event data
   */
  private emit(event: string, data: any): void {
    if (!this.listeners.has(event)) return;
    
    const eventListeners = this.listeners.get(event);
    if (!eventListeners) return;
    
    for (const listener of eventListeners) {
      try {
        listener(data);
      } catch (error) {
        console.error(`Error in event listener for ${event}:`, error);
      }
    }
  }
  
  /**
   * Get all loaded cassettes
   */
  public getCassettes(): Cassette[] {
    return Array.from(this.cassettes.values());
  }
  
  /**
   * Get a cassette by ID
   * @param id Cassette ID
   */
  public getCassette(id: string): Cassette | undefined {
    return this.cassettes.get(id);
  }
  
  /**
   * Check if a cassette is loaded
   * @param id Cassette ID
   */
  public hasCassette(id: string): boolean {
    return this.cassettes.has(id);
  }
  
  /**
   * Load a cassette from a URL
   * @param url URL to the cassette file
   */
  public async loadCassetteFromUrl(url: string): Promise<Cassette | null> {
    try {
      this.logger.log(`Loading cassette from URL: ${url}`);
      
      // Extract file name from URL
      const fileName = url.split('/').pop() || 'unknown_cassette.wasm';
      
      // Load the cassette
      const result = await loadCassette(url, fileName, {
        debug: this.options.debug,
        ...this.options
      });
      
      if (result.success && result.cassette) {
        this.cassettes.set(result.cassette.id, result.cassette);
        this.logger.log(`Successfully loaded cassette: ${result.cassette.name} (${result.cassette.id})`);
        
        // Emit event
        this.emit('cassette-loaded', result.cassette);
        
        return result.cassette;
      } else {
        this.logger.error(`Failed to load cassette from ${url}: ${result.error}`);
        
        // Emit error event
        this.emit('cassette-error', { url, error: result.error });
        
        return null;
      }
    } catch (error: any) {
      this.logger.error(`Error loading cassette from ${url}:`, error.message || error);
      
      // Emit error event
      this.emit('cassette-error', { url, error: error.message || error });
      
      return null;
    }
  }
  
  /**
   * Load a cassette from a File object (for drag and drop in browser environments)
   * @param file File object
   */
  public async loadCassetteFromFile(file: File): Promise<Cassette | null> {
    try {
      this.logger.log(`Loading cassette from file: ${file.name}`);
      
      // Load the cassette
      const result = await loadCassette(file, file.name, {
        debug: this.options.debug,
        ...this.options
      });
      
      if (result.success && result.cassette) {
        this.cassettes.set(result.cassette.id, result.cassette);
        this.logger.log(`Successfully loaded cassette: ${result.cassette.name} (${result.cassette.id})`);
        
        // Emit event
        this.emit('cassette-loaded', result.cassette);
        
        return result.cassette;
      } else {
        this.logger.error(`Failed to load cassette from file ${file.name}: ${result.error}`);
        
        // Emit error event
        this.emit('cassette-error', { file: file.name, error: result.error });
        
        return null;
      }
    } catch (error: any) {
      this.logger.error(`Error loading cassette from file ${file.name}:`, error.message || error);
      
      // Emit error event
      this.emit('cassette-error', { file: file.name, error: error.message || error });
      
      return null;
    }
  }
  
  /**
   * Load a cassette from an ArrayBuffer
   * @param arrayBuffer The array buffer containing the WebAssembly binary
   * @param fileName Optional file name for the cassette
   * @param options Optional loader options
   * @returns The loaded cassette or null if loading failed
   */
  public async loadCassetteFromArrayBuffer(arrayBuffer: ArrayBuffer, fileName: string, options: CassetteLoaderOptions = {}): Promise<Cassette | null> {
    try {
      this.logger.log(`Loading cassette from ArrayBuffer, size: ${arrayBuffer.byteLength} bytes, name: ${fileName}`);
      
      // Merge options with defaults
      const mergedOptions = {
        debug: this.options.debug,
        ...this.options,
        ...options
      };
      
      // Load the cassette using the core loadCassette function
      const result = await loadCassette(arrayBuffer, fileName, mergedOptions);
      
      if (result.success && result.cassette) {
        this.cassettes.set(result.cassette.id, result.cassette);
        this.logger.log(`Successfully loaded cassette from ArrayBuffer: ${result.cassette.name} (${result.cassette.id})`);
        
        // Emit event
        this.emit('cassette-loaded', result.cassette);
        
        return result.cassette;
      } else {
        this.logger.error(`Failed to load cassette from ArrayBuffer ${fileName}: ${result.error}`);
        
        // Emit error event
        this.emit('cassette-error', { file: fileName, error: result.error });
        
        return null;
      }
    } catch (error: any) {
      this.logger.error(`Error loading cassette from ArrayBuffer ${fileName}:`, error.message || error);
      
      // Emit error event
      this.emit('cassette-error', { file: fileName, error: error.message || error });
      
      return null;
    }
  }
  
  /**
   * Process a request through a specific cassette
   * @param cassetteId Cassette ID
   * @param request Request string
   */
  public processRequest(cassetteId: string, request: string): string | null {
    const cassette = this.cassettes.get(cassetteId);
    if (!cassette) {
      this.logger.error(`Cassette ${cassetteId} not found`);
      return null;
    }
    
    try {
      this.logger.log(`Processing request with cassette ${cassetteId}: ${request}`);
      
      // Verify the cassette has a req method
      if (!cassette.methods.req) {
        this.logger.error(`Cassette ${cassetteId} does not have a req method`);
        return JSON.stringify(["NOTICE", "Cassette does not have a req method"]);
      }
      
      // Get the function and log exports for debugging
      this.logger.log(`Available methods:`, Object.keys(cassette.methods));
      
      // Try to call the req function
      let response = cassette.methods.req(request);
      this.logger.log(`Raw response from cassette:`, response);
      
      // Handle undefined/null response
      if (response === undefined || response === null) {
        this.logger.warn(`Cassette ${cassetteId} returned undefined/null for request: ${request}`);
        
        // If no response, create a proper NIP-01 notice
        response = JSON.stringify(["NOTICE", "Function returned undefined or null"]);
      }
      
      // Ensure response is a string
      if (typeof response !== 'string') {
        this.logger.log(`Converting non-string response to string:`, response);
        try {
          response = JSON.stringify(response);
        } catch (err) {
          this.logger.error(`Failed to stringify response:`, err);
          response = JSON.stringify(["NOTICE", "Failed to stringify response"]);
        }
      }
      
      // Ensure response is valid NIP-01 format
      try {
        const parsed = JSON.parse(response);
        if (!Array.isArray(parsed) || parsed.length < 2 || !["EVENT", "NOTICE", "EOSE", "OK"].includes(parsed[0])) {
          this.logger.warn(`Response is not in NIP-01 format, wrapping: ${response}`);
          response = JSON.stringify(["NOTICE", response]);
        }
      } catch (e) {
        // If it's not valid JSON, wrap in a NOTICE
        this.logger.warn(`Response is not valid JSON, wrapping: ${response}`);
        response = JSON.stringify(["NOTICE", response]);
      }
      
      // Emit event
      this.emit('cassette-response', { 
        cassetteId, 
        request, 
        response 
      });
      
      return response;
    } catch (error: any) {
      this.logger.error(`Error processing request with cassette ${cassetteId}:`, error.message || error);
      
      // Create a proper NIP-01 error notice
      const errorResponse = JSON.stringify(["NOTICE", `Error: ${error.message || "Unknown error"}`]);
      
      // Emit error event
      this.emit('cassette-error', { 
        cassetteId, 
        request, 
        error: error.message || error 
      });
      
      return errorResponse;
    }
  }
  
  /**
   * Process a request through all loaded cassettes
   * @param request Request string
   */
  public processRequestAll(request: string): Map<string, string | null> {
    const responses = new Map<string, string | null>();
    
    try {
      // Parse request to get subscription ID
      const parsedRequest = JSON.parse(request);
      if (!Array.isArray(parsedRequest) || parsedRequest[0] !== "REQ" || parsedRequest.length < 2) {
        throw new Error("Invalid REQ message format");
      }
      
      const subscriptionId = parsedRequest[1];
      const seenEventIds = new Set<string>();
      const allResponses: string[] = [];
      
      // Process each cassette
      for (const [id, cassette] of this.cassettes) {
        let response = cassette.methods.req(request);
        while (response) {
          try {
            const parsed = JSON.parse(response);
            if (Array.isArray(parsed)) {
              if (parsed[0] === "EVENT" && parsed.length >= 3 && parsed[2]?.id) {
                const eventId = parsed[2].id;
                if (!seenEventIds.has(eventId)) {
                  seenEventIds.add(eventId);
                  responses.set(`${id}:event:${eventId}`, response);
                }
              } else if (parsed[0] === "EOSE") {
                break; // This cassette is done
              }
            }
          } catch (e) {
            break; // Can't parse, move to next cassette
          }
          
          // Get next response from this cassette
          response = cassette.methods.req(request);
        }
      }
      
      // Always add an EOSE at the end
      responses.set(`eose:${subscriptionId}`, JSON.stringify(["EOSE", subscriptionId]));
      
    } catch (error) {
      this.logger.error(`Error processing request: ${error}`);
    }
    
    return responses;
  }
  
  /**
   * Close a subscription for a specific cassette
   * @param cassetteId Cassette ID
   * @param closeStr Close string
   */
  public closeSubscription(cassetteId: string, closeStr: string): boolean {
    const cassette = this.cassettes.get(cassetteId);
    if (!cassette || !cassette.methods.close) {
      this.logger.error(`Cassette ${cassetteId} not found or does not support close`);
      return false;
    }
    
    try {
      this.logger.log(`Closing subscription with cassette ${cassetteId}: ${closeStr}`);
      cassette.methods.close(closeStr);
      
      // Emit event
      this.emit('cassette-close', { 
        cassetteId, 
        closeStr 
      });
      
      return true;
    } catch (error: any) {
      this.logger.error(`Error closing subscription with cassette ${cassetteId}:`, error.message || error);
      
      // Emit error event
      this.emit('cassette-error', { 
        cassetteId, 
        closeStr, 
        error: error.message || error 
      });
      
      return false;
    }
  }
  
  /**
   * Close a subscription for all cassettes
   * @param closeStr Close string
   */
  public closeSubscriptionAll(closeStr: string): void {
    try {
      // Parse close message to get subscription ID
      const parsedClose = JSON.parse(closeStr);
      if (!Array.isArray(parsedClose) || parsedClose[0] !== "CLOSE" || parsedClose.length < 2) {
        throw new Error("Invalid CLOSE message format");
      }
      const subscriptionId = parsedClose[1];
      
      // Remove subscription from state for all cassettes
      for (const [id, subscriptions] of this.subscriptionState) {
        subscriptions.delete(subscriptionId);
      }
      
      // Forward close to cassettes
      for (const [id, cassette] of this.cassettes) {
        if (cassette.methods.close) {
          try {
            cassette.methods.close(closeStr);
            
            // Emit event
            this.emit('cassette-close', { 
              cassetteId: id, 
              closeStr 
            });
          } catch (error: any) {
            this.logger.error(`Error closing subscription with cassette ${id}:`, error.message || error);
            
            // Emit error event
            this.emit('cassette-error', { 
              cassetteId: id, 
              closeStr, 
              error: error.message || error 
            });
          }
        }
      }
    } catch (error: any) {
      this.logger.error(`Error processing close message:`, error.message || error);
    }
  }
  
  /**
   * Remove a cassette
   * @param id Cassette ID
   */
  public removeCassette(id: string): boolean {
    const cassette = this.cassettes.get(id);
    if (!cassette) {
      this.logger.error(`Cassette ${id} not found`);
      return false;
    }
    
    this.cassettes.delete(id);
    this.logger.log(`Removed cassette: ${cassette.name} (${id})`);
    
    // Emit event
    this.emit('cassette-removed', cassette);
    
    return true;
  }
  
  /**
   * Check for cassettes in standard locations
   */
  public async loadStandardCassettes(): Promise<Cassette[]> {
    const loaded: Cassette[] = [];
    
    // Skip this in node.js environments where fetch might not be available
    if (typeof window === 'undefined' && typeof fetch === 'undefined') {
      this.logger.log('Skipping standard cassettes in non-browser environment');
      return loaded;
    }
    
    const locations = [
      '/cassettes/',
      '/cli/',
      './',
    ];
    
    const knownCassettes = [
      'minimal_cassette.wasm',
      'test_standardized_interface_bg.wasm',
      'test_cassette.wasm',
      'test-cassette.wasm'
    ];
    
    for (const location of locations) {
      for (const cassette of knownCassettes) {
        try {
          // Check if the file exists using HEAD request
          const url = `${location}${cassette}`;
          
          try {
            const response = await fetch(url, { method: 'HEAD' });
            
            if (response.ok) {
              this.logger.log(`Found cassette at ${url}`);
              const result = await this.loadCassetteFromUrl(url);
              if (result) {
                loaded.push(result);
              }
            }
          } catch (error) {
            // Silently ignore fetch errors for standard locations
          }
        } catch (error) {
          // Silently ignore errors
        }
      }
    }
    
    return loaded;
  }
  
  /**
   * Add a cassette to the manager
   * @param cassette Cassette to add
   */
  public addCassette(cassette: Cassette): void {
    if (!cassette || !cassette.id) {
      this.logger.error('Invalid cassette object');
      return;
    }
    
    this.cassettes.set(cassette.id, cassette);
    this.logger.log(`Added cassette to manager: ${cassette.name} (${cassette.id})`);
    
    // Emit event
    this.emit('cassette-loaded', cassette);
  }
  
  /**
   * Dispose of all cassettes and clean up resources
   */
  public dispose(): void {
    this.logger.log('Disposing CassetteManager');
    
    // Dispose of all cassettes
    for (const [id, cassette] of this.cassettes) {
      try {
        // If the cassette has a dispose method, call it
        if (cassette.dispose) {
          this.logger.log(`Disposing cassette: ${cassette.name} (${id})`);
          cassette.dispose();
        }
      } catch (error) {
        this.logger.error(`Error disposing cassette ${id}:`, error);
      }
    }
    
    // Clear the cassettes map
    this.cassettes.clear();
    
    // Clear event listeners
    this.listeners.clear();
  }
  
  /**
   * Get memory statistics for all loaded cassettes
   */
  public getMemoryStats(): Record<string, any> {
    const stats: Record<string, any> = {
      totalCassettes: this.cassettes.size,
      cassetteStats: {}
    };
    
    for (const [id, cassette] of this.cassettes) {
      if (cassette.getMemoryStats) {
        stats.cassetteStats[id] = cassette.getMemoryStats();
      } else {
        stats.cassetteStats[id] = { memoryStatsNotAvailable: true };
      }
    }
    
    return stats;
  }
  
  /**
   * Process a request and collect all events and an EOSE message
   * Simple helper for clients that just want the results
   * 
   * @param subscriptionId Subscription ID
   * @param filters Array of filters
   * @returns Promise resolving to an array of response strings (EVENT messages followed by one EOSE)
   */
  public async getEvents(subscriptionId: string, filters: any[]): Promise<string[]> {
    const request = JSON.stringify(["REQ", subscriptionId, ...filters]);
    const allResponses: string[] = [];
    
    // We use a Set for deduplication
    const seenEventIds = new Set<string>();
    
    // Process all cassettes
    for (const [id, cassette] of this.cassettes) {
      let gotEOSE = false;
      
      // Process each cassette until it sends EOSE
      while (!gotEOSE) {
        const response = cassette.methods.req(request);
        
        // If no response, move to next cassette
        if (!response) break;
        
        try {
          const parsed = JSON.parse(response);
          
          if (Array.isArray(parsed)) {
            if (parsed[0] === "EVENT") {
              // Only add if not seen before (deduplication)
              if (parsed.length >= 3 && parsed[2]?.id && !seenEventIds.has(parsed[2].id)) {
                seenEventIds.add(parsed[2].id);
                allResponses.push(response);
              }
            } else if (parsed[0] === "EOSE") {
              gotEOSE = true;
            } else if (parsed[0] === "NOTICE") {
              // If we get a notice, add it but don't deduplicate
              allResponses.push(response);
              
              // If it says no more events, we're done with this cassette
              if (parsed[1] === "No more events") {
                gotEOSE = true;
              }
            }
          }
        } catch (e) {
          // If we can't parse the response, move to the next cassette
          break;
        }
      }
    }
    
    // Add final EOSE message
    allResponses.push(JSON.stringify(["EOSE", subscriptionId]));
    
    return allResponses;
  }
} 