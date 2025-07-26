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
 * Memory statistics for a cassette
 */
export interface CassetteMemoryStats {
  /**
   * Array of currently allocated memory pointers
   */
  allocatedPointers: number[];
  
  /**
   * Number of currently allocated memory blocks
   */
  allocationCount: number;
  
  /**
   * Memory information
   */
  memory: {
    /**
     * Total number of WebAssembly memory pages
     */
    totalPages: number;
    
    /**
     * Total bytes in WebAssembly memory
     */
    totalBytes: number;
    
    /**
     * Estimate of memory usage status
     */
    usageEstimate: string;
  };
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
  fileName?: string;
  
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
     * Universal send method for all NIP-01 messages
     * Returns array of strings for REQ messages, single string for others
     */
    send: (messageStr: string) => string | string[];
    
    /**
     * Get description and metadata for the cassette
     */
    describe: () => string;
    
    /**
     * Get JSON schema for the cassette (optional)
     */
    getSchema?: () => string;
    
    /**
     * Get NIP-11 relay information (optional)
     */
    info?: () => string;
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
  
  /**
   * Get memory statistics for the cassette
   * Used to detect memory leaks
   */
  getMemoryStats: () => CassetteMemoryStats;
  
  /**
   * Dispose of the cassette and clean up resources
   * Returns information about the cleanup operation
   */
  dispose: () => { success: boolean; allocationsCleanedUp: number };
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

  /**
   * Explicitly use browser memory management
   */
  useBrowserMemory?: boolean;

  /**
   * Force Node.js environment for memory management
   */
  forceNodeEnvironment?: boolean;
}

/**
 * Result of loading a cassette
 */
export interface CassetteLoadResult {
  /**
   * Whether the load was successful
   */
  success: boolean;
  
  /**
   * Error message if load failed
   */
  error?: string;
  
  /**
   * The loaded cassette, if successful
   */
  cassette?: Cassette;
  
  /**
   * Original filename of the cassette
   */
  fileName?: string;
  
  /**
   * WebAssembly memory
   */
  memory?: WebAssembly.Memory;
  
  /**
   * WebAssembly instance
   */
  instance?: WebAssembly.Instance;
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
  /** Universal send method for all NIP-01 messages - returns array for REQ, string for others */
  send: (messageStr: string) => string | string[];
  
  /** Get metadata about the cassette */
  describe: () => string;
  
  /** Get JSON schema for the cassette (optional) */
  getSchema?: () => string;
  
  /** Get NIP-11 relay information (optional) */
  info?: () => string;
} 