import { serve } from "bun";
import type { ServerWebSocket } from "bun";
import { join, dirname } from "path";
import { readFileSync, readdirSync } from "fs";
import { fileURLToPath } from "url";
// Schema validation is temporarily disabled
// import { validate } from "./schema-validator.js";
import type { Cassette as LoaderCassette, EventTracker } from "../cassette-loader/src/types";
import { loadCassette, createEventTracker } from '../cassette-loader/src';
import { CassetteManager } from '../cassette-loader/src/manager';


// NOTE: Schema validation has been temporarily disabled to bypass validation errors
// while debugging other issues with the cassette implementation.

// Get the current directory
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Configuration
const PORT = parseInt(process.env.PORT || "3001");
// const WASM_DIR = process.env.WASM_DIR || join(__dirname, "..", "cassettes");
const WASM_DIR = join(__dirname, "cassettes");
// Make sure we're using the correct directory
console.log(`Configured to load cassettes from: ${WASM_DIR}`);
const LOG_FILE = process.env.LOG_FILE || join(__dirname, "..", "logs", "boombox-new.log");

// Load real events from notes.json for testing
const NOTES_PATH = join(__dirname, "..", "cli", "notes.json");
let realEvents: any[] = [];
try {
  const notesContent = readFileSync(NOTES_PATH, 'utf-8');
  realEvents = JSON.parse(notesContent);
  console.log(`Loaded ${realEvents.length} real events from ${NOTES_PATH}`);
} catch (err) {
  console.error(`Failed to load notes from ${NOTES_PATH}:`, err);
}

// Global state
const cassettes: LoaderCassette[] = [];

// Create a cassette manager instance
const cassetteManager = new CassetteManager();

// Load all available cassettes from the specified directory
async function loadCassettes() {
  console.log(`Loading cassettes from ${WASM_DIR}`);
  try {
    console.log(`Checking if directory exists`);
    let files;
    try {
      files = readdirSync(WASM_DIR);
      console.log(`Found ${files.length} files in ${WASM_DIR}`);
    } catch (err) {
      console.error(`Failed to read directory ${WASM_DIR}:`, err);
      return;
    }
    
    console.log(`Files in ${WASM_DIR}:`, files);
    
    for (const file of files) {
      if (!file.endsWith(".wasm")) {
        console.log(`Skipping non-WASM file: ${file}`);
        continue;
      }
      
      try {
        const filepath = join(WASM_DIR, file);
        console.log(`Loading cassette from ${filepath}`);
        
        // Load the cassette with error handling
        let result;
        try {
          const fileBuffer = readFileSync(filepath);
          console.log(`Read file ${filepath}, size: ${fileBuffer.length} bytes`);
          result = await loadCassette(fileBuffer, file, {
            debug: true,
            deduplicateEvents: false
          });
          console.log(`Cassette load result:`, result);
        } catch (loadError) {
          console.error(`Error loading cassette ${file}:`, loadError);
          continue;
        }
        
        if (!result.success || !result.cassette) {
          console.error(`Failed to load cassette ${file}:`, result.error);
          continue;
        }
        
        // Add the cassette to the manager
        cassetteManager.addCassette(result.cassette);
        console.log(`Successfully loaded cassette: ${result.cassette.name} (${result.cassette.id})`);
        console.log(`Cassette methods:`, Object.keys(result.cassette.methods));
      } catch (error) {
        console.error(`Error processing cassette ${file}:`, error);
      }
    }
    
    const loadedCassettes = cassetteManager.getCassettes();
    console.log(`Loaded ${loadedCassettes.length} cassettes:`);
    loadedCassettes.forEach(cassette => {
      console.log(`- ${cassette.name} (${cassette.id})`);
    });
  } catch (error) {
    console.error('Failed to load cassettes:', error);
  }
}

// Helper function to filter real events from notes.json according to NIP-01 filters
function filterEvents(filters: any[]): any[] {
  if (!realEvents || realEvents.length === 0) {
    console.log('No real events available to filter');
    return [];
  }

  // Implementation of basic NIP-01 filtering logic
  return realEvents.filter(event => {
    for (const filter of filters) {
      let matchesFilter = true;
      
      // Check ids filter
      if (filter.ids && filter.ids.length > 0) {
        if (!filter.ids.includes(event.id)) {
          matchesFilter = false;
        }
      }
      
      // Check authors filter
      if (filter.authors && filter.authors.length > 0) {
        if (!filter.authors.includes(event.pubkey)) {
          matchesFilter = false;
        }
      }
      
      // Check kinds filter
      if (filter.kinds && filter.kinds.length > 0) {
        if (!filter.kinds.includes(event.kind)) {
          matchesFilter = false;
        }
      }
      
      // Check since filter
      if (filter.since !== undefined) {
        if (event.created_at < filter.since) {
          matchesFilter = false;
        }
      }
      
      // Check until filter
      if (filter.until !== undefined) {
        if (event.created_at > filter.until) {
          matchesFilter = false;
        }
      }
      
      // Handle NIP-119 "&" tag filters
      for (const key in filter) {
        if (key.startsWith('&')) {
          const tagName = key.substring(1);
          const tagValues = filter[key];
          
          if (tagValues && tagValues.length > 0) {
            // Find all tags of the specified type
            const matchingTags = event.tags.filter((tag: string[]) => tag[0] === tagName);
            
            // Check if ALL of the values in the filter are present in the tags
            const allValuesMatch = tagValues.every((value: string) => 
              matchingTags.some((tag: string[]) => tag[1] === value)
            );
            
            if (!allValuesMatch) {
              matchesFilter = false;
            }
          }
        }
      }
      
      // Handle tag filters (exact match)
      for (const key in filter) {
        if (key.startsWith('#')) {
          const tagName = key.substring(1);
          const tagValues = filter[key];
          
          if (tagValues && tagValues.length > 0) {
            // Find any tag that matches one of the values
            const hasMatchingTag = event.tags.some((tag: string[]) => 
              tag[0] === tagName && tagValues.includes(tag[1])
            );
            
            if (!hasMatchingTag) {
              matchesFilter = false;
            }
          }
        }
      }
      
      // If this filter matches, we don't need to check the others
      if (matchesFilter) {
        return true;
      }
    }
    
    // None of the filters matched
    return false;
  });
}

// Process incoming WebSocket messages
async function handleReqMessage(ws: ServerWebSocket, args: any[]) {
  const subscriptionId = args[0];
  const filters = args.slice(1);

  console.log(`Processing REQ for subscription ${subscriptionId}`);
  
  try {
    // Create the request string exactly as received from the client
    const request = JSON.stringify(["REQ", subscriptionId, ...filters]);
    
    // Use processRequestAll which handles everything properly
    const responses = cassetteManager.processRequestAll(request);
    
    // Send each response back to the client
    let eventCount = 0;
    let eoseCount = 0;
    
    for (const [_, response] of responses) {
      if (!response) continue;
      
      try {
        const parsed = JSON.parse(response);
        if (Array.isArray(parsed)) {
          if (parsed[0] === "EVENT") eventCount++;
          else if (parsed[0] === "EOSE") eoseCount++;
        }
        
        // Send to the WebSocket client
        ws.send(response);
      } catch (e) {
        console.error(`Error parsing response: ${e}`);
      }
    }
    
    console.log(`Completed subscription ${subscriptionId}: ${eventCount} events, ${eoseCount} EOSE messages`);
  } catch (error) {
    console.error(`Error processing subscription ${subscriptionId}:`, error);
    // Ensure we send an EOSE in case of error
    ws.send(JSON.stringify(["EOSE", subscriptionId]));
  }
}

async function handleCloseMessage(ws: ServerWebSocket, args: any[]) {
  const subscriptionId = args[0];
  const fullMessage = ["CLOSE", subscriptionId];
  const messageStr = JSON.stringify(fullMessage);

  for (const cassette of cassetteManager.getCassettes()) {
    try {
      if (cassette.methods.close) {
        const response = await cassette.methods.close(messageStr);
        if (response) {
          ws.send(response);
        }
      }
    } catch (error) {
      console.error(`Error from cassette ${cassette.name}:`, error);
    }
  }
}

// Create the server
const server = serve({
  port: PORT,
  fetch(req, server) {
    // Handle WebSocket upgrade
    if (server.upgrade(req)) {
      return; // Upgraded to WebSocket
    }
    
    // Default response for other requests
    return new Response("Boombox WebSocket server is running", { 
      status: 200, 
      headers: { "Content-Type": "text/plain" } 
    });
  },
  websocket: {
    async message(ws: ServerWebSocket, message: string) {
      try {
        const data = JSON.parse(message);
        if (!Array.isArray(data)) {
          return;
        }

        const [type, ...args] = data;
        switch (type) {
          case "REQ":
            await handleReqMessage(ws, args);
            break;
          case "CLOSE":
            await handleCloseMessage(ws, args);
            break;
          default:
            // Forward unknown message types to cassettes
            const messageStr = JSON.stringify([type, ...args]);
            for (const cassette of cassetteManager.getCassettes()) {
              try {
                const method = cassette.methods[type as keyof typeof cassette.methods];
                if (method) {
                  const response = await method(messageStr);
                  if (response) {
                    ws.send(response);
                  }
                }
              } catch (error) {
                console.error(`Error from cassette ${cassette.name}:`, error);
              }
            }
            break;
        }
      } catch (error) {
        console.error("Error processing message:", error);
      }
    }
  }
});

console.log(`WebAssembly directory: ${WASM_DIR}`);
console.log(`Log file: ${LOG_FILE}`);

// Load cassettes when the server starts
loadCassettes().catch(error => {
  console.error("Error loading cassettes:", error);
});

// Export the handleReqMessage function and cassetteManager
export { handleReqMessage, cassetteManager };