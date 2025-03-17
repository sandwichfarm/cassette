/**
 * Example showing how to integrate the cassette-loader in a browser environment
 * 
 * This file would be used in a bundled web application such as a Svelte app.
 */
import { loadCassette, Cassette, isWebAssemblySupported, ENV_INFO } from '../src/index.js';

// Define a class for managing cassettes in a browser environment
export class CassetteManager {
  // Registry of loaded cassettes
  private cassettes: Map<string, Cassette> = new Map();
  
  // Event listeners for cassette-related events
  private listeners: Map<string, Array<(data: any) => void>> = new Map();
  
  constructor() {
    // Check if WebAssembly is supported
    if (!isWebAssemblySupported()) {
      console.error("WebAssembly is not supported in this browser");
      throw new Error("WebAssembly is not supported in this browser");
    }
    
    console.log("Environment:", ENV_INFO);
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
      console.log(`Loading cassette from URL: ${url}`);
      
      // Extract file name from URL
      const fileName = url.split('/').pop() || 'unknown_cassette.wasm';
      
      // Load the cassette
      const result = await loadCassette(url, fileName, {
        debug: true
      });
      
      if (result.success && result.cassette) {
        this.cassettes.set(result.cassette.id, result.cassette);
        console.log(`Successfully loaded cassette: ${result.cassette.name} (${result.cassette.id})`);
        
        // Emit event
        this.emit('cassette-loaded', result.cassette);
        
        return result.cassette;
      } else {
        console.error(`Failed to load cassette from ${url}: ${result.error}`);
        
        // Emit error event
        this.emit('cassette-error', { url, error: result.error });
        
        return null;
      }
    } catch (error: any) {
      console.error(`Error loading cassette from ${url}:`, error.message || error);
      
      // Emit error event
      this.emit('cassette-error', { url, error: error.message || error });
      
      return null;
    }
  }
  
  /**
   * Load a cassette from a File object (for drag and drop)
   * @param file File object
   */
  public async loadCassetteFromFile(file: File): Promise<Cassette | null> {
    try {
      console.log(`Loading cassette from file: ${file.name}`);
      
      // Load the cassette
      const result = await loadCassette(file, file.name, {
        debug: true
      });
      
      if (result.success && result.cassette) {
        this.cassettes.set(result.cassette.id, result.cassette);
        console.log(`Successfully loaded cassette: ${result.cassette.name} (${result.cassette.id})`);
        
        // Emit event
        this.emit('cassette-loaded', result.cassette);
        
        return result.cassette;
      } else {
        console.error(`Failed to load cassette from file ${file.name}: ${result.error}`);
        
        // Emit error event
        this.emit('cassette-error', { file: file.name, error: result.error });
        
        return null;
      }
    } catch (error: any) {
      console.error(`Error loading cassette from file ${file.name}:`, error.message || error);
      
      // Emit error event
      this.emit('cassette-error', { file: file.name, error: error.message || error });
      
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
      console.error(`Cassette ${cassetteId} not found`);
      return null;
    }
    
    try {
      console.log(`Processing request with cassette ${cassetteId}: ${request}`);
      const response = cassette.methods.req(request);
      
      // Emit event
      this.emit('cassette-response', { 
        cassetteId, 
        request, 
        response 
      });
      
      return response;
    } catch (error: any) {
      console.error(`Error processing request with cassette ${cassetteId}:`, error.message || error);
      
      // Emit error event
      this.emit('cassette-error', { 
        cassetteId, 
        request, 
        error: error.message || error 
      });
      
      return null;
    }
  }
  
  /**
   * Process a request through all loaded cassettes
   * @param request Request string
   */
  public processRequestAll(request: string): Map<string, string | null> {
    const responses = new Map<string, string | null>();
    
    for (const [id, cassette] of this.cassettes) {
      try {
        const response = cassette.methods.req(request);
        responses.set(id, response);
        
        // Emit event
        this.emit('cassette-response', { 
          cassetteId: id, 
          request, 
          response 
        });
      } catch (error: any) {
        console.error(`Error processing request with cassette ${id}:`, error.message || error);
        responses.set(id, null);
        
        // Emit error event
        this.emit('cassette-error', { 
          cassetteId: id, 
          request, 
          error: error.message || error 
        });
      }
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
      console.error(`Cassette ${cassetteId} not found or does not support close`);
      return false;
    }
    
    try {
      console.log(`Closing subscription with cassette ${cassetteId}: ${closeStr}`);
      cassette.methods.close(closeStr);
      
      // Emit event
      this.emit('cassette-close', { 
        cassetteId, 
        closeStr 
      });
      
      return true;
    } catch (error: any) {
      console.error(`Error closing subscription with cassette ${cassetteId}:`, error.message || error);
      
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
          console.error(`Error closing subscription with cassette ${id}:`, error.message || error);
          
          // Emit error event
          this.emit('cassette-error', { 
            cassetteId: id, 
            closeStr, 
            error: error.message || error 
          });
        }
      }
    }
  }
  
  /**
   * Remove a cassette
   * @param id Cassette ID
   */
  public removeCassette(id: string): boolean {
    const cassette = this.cassettes.get(id);
    if (!cassette) {
      console.error(`Cassette ${id} not found`);
      return false;
    }
    
    this.cassettes.delete(id);
    console.log(`Removed cassette: ${cassette.name} (${id})`);
    
    // Emit event
    this.emit('cassette-removed', cassette);
    
    return true;
  }
  
  /**
   * Check for cassettes in standard locations
   */
  public async loadStandardCassettes(): Promise<Cassette[]> {
    const loaded: Cassette[] = [];
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
              console.log(`Found cassette at ${url}`);
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
} 