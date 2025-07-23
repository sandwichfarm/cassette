/**
 * Simple logger for tests
 */
export function createLogger(debug: boolean = false, prefix: string = '') {
  return {
    log: (...args: any[]) => {
      if (debug) {
        console.log(`[${prefix}]`, ...args);
      }
    },
    warn: (...args: any[]) => {
      console.warn(`[${prefix}]`, ...args);
    },
    error: (...args: any[]) => {
      console.error(`[${prefix}]`, ...args);
    }
  };
} 