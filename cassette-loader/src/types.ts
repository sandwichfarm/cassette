/**
 * Interface for a Cassette metadata object returned by the describe function
 */
export interface CassetteMetadata {
  name: string;
  description: string;
  version: string;
  author?: string;
  supportedKinds?: number[];
  [key: string]: any;
}

/**
 * Interface for tracking unique events to avoid duplicates
 */
export interface EventTracker {
  /**
   * Set of event IDs to track unique events
   */
  eventIds: Set<string>;
  
  /**
   * Reset the tracker
   */
  reset(): void;
  
  /**
   * Add an event ID and check if it was already seen
   * @param id Event ID to check
   * @returns true if event is new, false if duplicate
   */
  addAndCheck(id: string): boolean;
  
  /**
   * Filter an array of events to remove duplicates
   * @param events Array of events to filter
   * @returns Array with duplicates removed
   */
  filterDuplicates(events: any[]): any[];
}

/**
 * Interface for a loaded Cassette with its methods
 */
export interface Cassette {
  /**
   * Unique identifier for the cassette
   */
  id: string;
  
  /**
   * Original file name of the cassette
   */
  fileName: string;
  
  /**
   * Cassette name from metadata
   */
  name: string;
  
  /**
   * Cassette description from metadata
   */
  description: string;
  
  /**
   * Version string
   */
  version: string;
  
  /**
   * Methods to interact with the cassette
   */
  methods: {
    /**
     * Get cassette metadata
     */
    describe: () => string;
    
    /**
     * Process a request with the cassette
     * @param requestStr Request string in JSON format
     */
    req: (requestStr: string) => string;
    
    /**
     * Close a subscription with the cassette
     * @param closeStr Close string in JSON format
     */
    close?: (closeStr: string) => string;
    
    /**
     * Get JSON schema for the cassette
     */
    getSchema?: () => string;
  };
  
  /**
   * WebAssembly module exports
   */
  exports?: WebAssembly.Exports;
  
  /**
   * Original WebAssembly module instance
   */
  instance?: WebAssembly.Instance;
  
  /**
   * WebAssembly memory
   */
  memory?: WebAssembly.Memory;
  
  /**
   * Event tracker for deduplication
   */
  eventTracker?: EventTracker;
}

/**
 * Options for loading a cassette
 */
export interface CassetteLoaderOptions {
  /**
   * Memory initial size in pages (64KB per page)
   */
  memoryInitialSize?: number;
  
  /**
   * Custom import object to use with the WebAssembly module
   */
  customImports?: WebAssembly.Imports;
  
  /**
   * Whether to expose the WebAssembly exports in the returned cassette
   */
  exposeExports?: boolean;
  
  /**
   * Debug mode (enables extra logging)
   */
  debug?: boolean;
  
  /**
   * Whether to enable event deduplication
   */
  deduplicateEvents?: boolean;
}

/**
 * Result of loading a cassette
 */
export interface CassetteLoadResult {
  /**
   * Whether the cassette was loaded successfully
   */
  success: boolean;
  
  /**
   * The loaded cassette (if success is true)
   */
  cassette?: Cassette;
  
  /**
   * Error message (if success is false)
   */
  error?: string;
}

/**
 * Source of a cassette (file or URL)
 */
export type CassetteSource = File | string | ArrayBuffer | Uint8Array;

/**
 * Error thrown when loading a cassette fails
 */
export class CassetteLoadError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'CassetteLoadError';
  }
}

/**
 * Object containing available methods for interacting with a cassette
 */
export interface CassetteMethods {
  /** Get metadata about the cassette */
  describe: () => string;
  
  /** Process a request and return a response */
  req: (requestStr: string) => string;
  
  /** Close a subscription (optional) */
  close?: (closeStr: string) => string;
  
  /** Get JSON schema for the cassette (optional) */
  getSchema?: () => string;
} 