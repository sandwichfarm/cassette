import { serve } from "bun";
import type { ServerWebSocket } from "bun";
import { join, dirname } from "path";
import { readFileSync, readdirSync } from "fs";
import { fileURLToPath } from "url";
import { validate } from "./schema-validator.js";
import type { Cassette as LoaderCassette, EventTracker } from "../cassette-loader/src/types";
import { loadCassette } from "../cassette-loader/dist/src/index.js";
import { createEventTracker } from "../cassette-loader/src/utils.js";

// Get the current directory
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Configuration
const PORT = parseInt(process.env.PORT || "3001");
const WASM_DIR = process.env.WASM_DIR || join(__dirname, "..", "cassettes");
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

// Interfaces
interface SubscriptionData {
  id: string;
  filters: any[];
  active: boolean;
  eventTracker: EventTracker;
}

interface WebSocketData {
  subscriptions: Map<string, SubscriptionData>;
}

interface BoomboxCassette {
  name: string;
  instance: LoaderCassette;
  schema: any;
}

// Global state
const cassettes: BoomboxCassette[] = [];

// Load all available cassettes from the specified directory
async function loadCassettes() {
  console.log(`Loading cassettes from ${WASM_DIR}`);
  try {
    const files = readdirSync(WASM_DIR);
    
    for (const file of files) {
      if (!file.endsWith(".wasm")) continue;
      
      try {
        const filepath = join(WASM_DIR, file);
        console.log(`Loading cassette from ${filepath}`);
        
        // Load the cassette with error handling
        let result;
        try {
          result = await loadCassette(readFileSync(filepath));
        } catch (loadError) {
          console.error(`Error loading cassette ${file}:`, loadError);
          continue;
        }
        
        if (!result.success || !result.cassette) {
          console.error(`Failed to load cassette ${file}:`, result.error);
        continue;
      }
      
        const instance = result.cassette;
        
        // Get schema from the cassette with better error handling
        let schema = {};
        if (instance.methods.getSchema) {
          try {
            const schemaStr = instance.methods.getSchema();
            // Safely attempt to parse the schema
            try {
              schema = JSON.parse(schemaStr);
              console.log(`Successfully parsed schema for ${file}`);
            } catch (parseError) {
              console.warn(`Invalid schema JSON from ${file}, using default schema:`, parseError);
              // Use a default schema if parsing fails
              schema = {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                  "kinds": {
                    "type": "array",
                    "items": {
                      "type": "integer"
                    }
                  },
                  "limit": {
                    "type": "integer"
                  }
                }
              };
            }
          } catch (schemaError) {
            console.warn(`Error getting schema from ${file}, using default:`, schemaError);
          }
              } else {
          console.log(`No getSchema method available for ${file}, using default schema`);
        }
                
        cassettes.push({
          name: file,
          instance,
          schema
        });
        
        console.log(`Loaded cassette ${file} with schema:`, JSON.stringify(schema).substring(0, 100) + "...");
      } catch (error) {
        console.error(`Error processing cassette ${file}:`, error);
      }
    }
    
    console.log(`Loaded ${cassettes.length} cassettes`);
  } catch (error) {
    console.error("Error loading cassettes:", error);
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
function message(ws: ServerWebSocket<WebSocketData>, message: string) {
  try {
    const parsedMessage = JSON.parse(message);
    const [command, ...args] = parsedMessage;
    
    console.log(`Received command: ${command} with args:`, args);
    
    switch (command) {
      case "EVENT":
        handleEventMessage(ws, args);
        break;
      case "REQ":
        handleReqMessage(ws, args);
        break;
      case "CLOSE":
        handleCloseMessage(ws, args);
                          break;
      default:
        ws.send(JSON.stringify(["NOTICE", "Unknown command"]));
    }
  } catch (error) {
    console.error("Error processing message:", error);
    ws.send(JSON.stringify(["NOTICE", "Invalid message format"]));
  }
}

function handleEventMessage(ws: ServerWebSocket<WebSocketData>, args: any[]) {
  // EVENT handling if needed
  ws.send(JSON.stringify(["NOTICE", "EVENT not implemented"]));
}

async function handleReqMessage(ws: ServerWebSocket<WebSocketData>, args: any[]) {
  if (args.length < 2) {
    ws.send(JSON.stringify(["NOTICE", "Error: Invalid REQ format"]));
    return;
  }
  
  const subId = args[0];
  const filters = args.slice(1);
  
  console.log(`Subscription ${subId} with filters:`, filters);
  console.log(`Available cassettes: ${cassettes.length}`);
  
  // Register the subscription
  ws.data.subscriptions.set(subId, {
    id: subId,
    filters,
    active: true,
    eventTracker: createEventTracker()
  });
  
  if (cassettes.length === 0) {
    // If no cassettes are available, use real events directly
    const matchingEvents = filterEvents(filters);
    console.log(`No cassettes available, using ${matchingEvents.length} filtered real events directly`);
    
    // Send matching events (deduplicated)
    for (const event of matchingEvents) {
      if (ws.readyState === WebSocket.OPEN) {
        // Check if we've already seen this event
        const subscriptionData = ws.data.subscriptions.get(subId);
        if (subscriptionData && subscriptionData.eventTracker.addAndCheck(event.id)) {
          ws.send(JSON.stringify(["EVENT", subId, event]));
        }
      }
    }
    
    // Send EOSE
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(["EOSE", subId]));
    }
    return;
  }
  
  // Set a timeout to ensure EOSE is sent
  const timeout = setTimeout(() => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(["EOSE", subId]));
    }
  }, 5000);
  
  let hasResults = false;
  
  // Process through each cassette
  for (const cassette of cassettes) {
    console.log(`Testing cassette ${cassette.name}`);
    
    try {
      // Check if the schema matches any of the filters
      let valid = true; // Default to true if validation fails
      try {
        valid = filters.some(filter => validate(filter, cassette.schema));
      } catch (validationError) {
        console.warn(`Schema validation error for ${cassette.name}, proceeding anyway:`, validationError);
      }
      
      if (!valid) {
        console.log(`Cassette ${cassette.name} schema doesn't match filters`);
        continue;
      }
      
      console.log(`Processing request with cassette ${cassette.name}`);
      
      // Create proper NIP-01 request format: ["REQ", subscription_id, ...filters]
      const reqData = ["REQ", subId, ...filters];
      const reqStr = JSON.stringify(reqData);
      console.log(`Sending to cassette: ${reqStr}`);
      
      let response;
      try {
        console.log(`Calling req method on ${cassette.name}`);
        // Debug the exports first
        console.log(`Available methods on ${cassette.name}:`, Object.keys(cassette.instance.methods));
        
        response = await cassette.instance.methods.req(reqStr);
        
        // Skip if response is empty or null
        if (!response) {
          console.warn(`Empty response from ${cassette.name} - Check if your cassette is properly handling this request`);
          console.log(`Request was: ${reqStr}`);
          
          // Use real events from notes.json as fallback when cassette returns empty response
          // This won't generate any new events, only use the ones you've provided
          const matchingEvents = filterEvents(filters);
          if (matchingEvents.length > 0) {
            console.log(`Using ${matchingEvents.length} events from notes.json as fallback`);
            for (const event of matchingEvents) {
              if (ws.readyState === WebSocket.OPEN) {
                // Check if we've already seen this event
                const subscriptionData = ws.data.subscriptions.get(subId);
                if (subscriptionData && subscriptionData.eventTracker.addAndCheck(event.id)) {
                  ws.send(JSON.stringify(["EVENT", subId, event]));
                  hasResults = true;
                }
              }
            }
          }
          
          continue;
        }
        
        console.log(`Raw response from ${cassette.name}:`, response.substring(0, 100));
      } catch (reqError) {
        console.error(`Error calling req method on ${cassette.name}:`, reqError);
        continue;
      }
      
      // Process response
      processResponse(ws, subId, response, cassette.name);
      hasResults = true;
    } catch (error) {
      console.error(`Error processing request with cassette ${cassette.name}:`, error);
    }
  }
  
  clearTimeout(timeout);
  
  // Send EOSE if not sent by timeout
  if (ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(["EOSE", subId]));
  }
  
  if (!hasResults) {
    console.log(`No results for subscription ${subId}`);
    
    // As a last resort, try using real events from notes.json
    const matchingEvents = filterEvents(filters);
    if (matchingEvents.length > 0) {
      console.log(`No results from cassettes, using ${matchingEvents.length} events from notes.json`);
      for (const event of matchingEvents) {
        if (ws.readyState === WebSocket.OPEN) {
          // Check if we've already seen this event
          const subscriptionData = ws.data.subscriptions.get(subId);
          if (subscriptionData && subscriptionData.eventTracker.addAndCheck(event.id)) {
            ws.send(JSON.stringify(["EVENT", subId, event]));
          }
        }
      }
    }
  }
}

// Update the processResponse function to use the event tracker
function processResponse(ws: ServerWebSocket<WebSocketData>, subId: string, response: string, cassetteName: string) {
  // Get the subscription data and event tracker
  const subscriptionData = ws.data.subscriptions.get(subId);
  if (!subscriptionData) {
    console.warn(`Subscription ${subId} not found, cannot process response`);
    return;
  }
  
  const eventTracker = subscriptionData.eventTracker;
  
  // Check for corrupted response patterns
  if (response.startsWith('TICE') || response.includes('TICE","')) {
    console.error(`Corrupted NOTICE response detected from ${cassetteName}`);
    ws.send(JSON.stringify(["NOTICE", `Error: Corrupted response for REQ: ${subId}`]));
    return;
  }

  // If response includes non-printable characters, treat as corrupted
  if (/[\x00-\x1F\x7F-\x9F]/.test(response)) {
    console.error(`Response contains non-printable characters from ${cassetteName}`);
    ws.send(JSON.stringify(["NOTICE", `Error: Corrupted response with invalid characters for REQ: ${subId}`]));
    return;
  }
  
  try {
    // Try to parse the response as JSON
    const parsedResponse = JSON.parse(response);
    
    // Handle NIP-01 NOTICE message
    if (Array.isArray(parsedResponse) && parsedResponse.length >= 2 && parsedResponse[0] === "NOTICE") {
      // Forward NOTICE to client with the subscription ID
      console.log(`Forwarding NOTICE from ${cassetteName}:`, parsedResponse[1]);
      ws.send(JSON.stringify(["NOTICE", parsedResponse[1]]));
      return;
    }
    
    // Handle NIP-01 EVENT message
    if (Array.isArray(parsedResponse) && parsedResponse.length >= 2 && parsedResponse[0] === "EVENT") {
      // Forward EVENT to client if not a duplicate
      console.log(`Checking EVENT from ${cassetteName} for duplicates`);
      const event = parsedResponse.length === 2 ? parsedResponse[1] : parsedResponse[2];
      
      if (event && event.id && eventTracker.addAndCheck(event.id)) {
        if (parsedResponse.length === 2) {
          // Add subscription ID if missing
          ws.send(JSON.stringify(["EVENT", subId, event]));
        } else {
          ws.send(JSON.stringify(parsedResponse));
        }
      } else {
        console.log(`Skipping duplicate event ${event?.id || 'unknown'}`);
      }
      return;
    }
    
    // Handle NIP-01 EOSE message
    if (Array.isArray(parsedResponse) && parsedResponse.length >= 1 && parsedResponse[0] === "EOSE") {
      // Forward EOSE to client
      console.log(`Forwarding EOSE from ${cassetteName}`);
      ws.send(JSON.stringify(["EOSE", subId]));
      return;
    }
    
    // If it's an array of events (Nostr events with "id", "pubkey", etc.)
    if (Array.isArray(parsedResponse) && parsedResponse.length > 0) {
      const firstItem = parsedResponse[0];
      
      // Check if it looks like a Nostr event
      if (firstItem && 
          typeof firstItem === 'object' && 
          firstItem.id && 
          firstItem.pubkey &&
          typeof firstItem.kind === 'number') {
        
        console.log(`Got ${parsedResponse.length} events from ${cassetteName}, deduplicating`);
        
        // Send each event as a proper NIP-01 EVENT message (if not duplicate)
        for (const event of parsedResponse) {
          if (ws.readyState === WebSocket.OPEN && event.id && eventTracker.addAndCheck(event.id)) {
            ws.send(JSON.stringify(["EVENT", subId, event]));
          } else {
            console.log(`Skipping duplicate event ${event?.id || 'unknown'}`);
          }
        }
        return;
      }
    }
    
    // Handle response objects with events array
    if (parsedResponse && 
        typeof parsedResponse === 'object' && 
        !Array.isArray(parsedResponse) &&
        parsedResponse.events && 
        Array.isArray(parsedResponse.events) && 
        parsedResponse.events.length > 0) {
      
      console.log(`Got ${parsedResponse.events.length} events in events array from ${cassetteName}, deduplicating`);
      
      // Send each event as a proper NIP-01 EVENT message (if not duplicate)
      for (const event of parsedResponse.events) {
        if (ws.readyState === WebSocket.OPEN && 
            event.id && 
            event.pubkey && 
            typeof event.kind === 'number' &&
            eventTracker.addAndCheck(event.id)) {
          ws.send(JSON.stringify(["EVENT", subId, event]));
        } else {
          console.log(`Skipping duplicate event ${event?.id || 'unknown'}`);
        }
      }
      return;
    }
    
    // Otherwise treat as unrecognized format and send error
    console.warn(`Unrecognized response format from ${cassetteName}:`, 
                 JSON.stringify(parsedResponse).substring(0, 100));
    ws.send(JSON.stringify(["NOTICE", `Error: Unrecognized response format from ${cassetteName}`]));
    
  } catch (parseError: any) {
    // Failed to parse as JSON, send error notice
    console.error(`Error processing response from ${cassetteName}:`, parseError);
    console.log(`Raw response was: ${response.substring(0, 200)}`);
    ws.send(JSON.stringify(["NOTICE", `Error: Invalid JSON response from ${cassetteName} for REQ: ${subId}`]));
  }
}

async function handleCloseMessage(ws: ServerWebSocket<WebSocketData>, args: any[]) {
  if (args.length < 1) {
    ws.send(JSON.stringify(["NOTICE", "Error: Invalid CLOSE format"]));
    return;
  }
  
  const subId = args[0];
  console.log(`Closing subscription ${subId}`);
  
  // Close the subscription
  const subscription = ws.data.subscriptions.get(subId);
  if (!subscription) {
    ws.send(JSON.stringify(["NOTICE", `Error: Unknown subscription: ${subId}`]));
    return;
  }
  
  subscription.active = false;
  ws.data.subscriptions.delete(subId);
  
  // Notify cassettes about the closure
  for (const cassette of cassettes) {
    try {
      if (cassette.instance.methods.close) {
        // Format as proper NIP-01 message: ["CLOSE", subscription_id]
        const closeData = ["CLOSE", subId];
        const closeStr = JSON.stringify(closeData);
        console.log(`Sending close to cassette ${cassette.name}: ${closeStr}`);
        
        try {
          const response = await cassette.instance.methods.close(closeStr);
          
          // Skip if response is empty or null
          if (!response) {
            console.log(`Empty close response from ${cassette.name}`);
            continue;
          }
          
          console.log(`Close response from ${cassette.name}:`, response.substring(0, 100));
          
          // Process close response
          processCloseResponse(ws, response, cassette.name);
        } catch (closeError) {
          console.error(`Error calling close method on ${cassette.name}:`, closeError);
        }
      }
    } catch (error) {
      console.error(`Error closing subscription with cassette ${cassette.name}:`, error);
    }
  }
  
  console.log(`Subscription ${subId} closed`);
  
  // Send a success notice to the client
  ws.send(JSON.stringify(["NOTICE", `Subscription ${subId} closed successfully`]));
}

// Helper function to process close responses from cassettes
function processCloseResponse(ws: ServerWebSocket<WebSocketData>, response: string, cassetteName: string) {
  // Check for corrupted response patterns
  if (response.startsWith('TICE') || response.includes('TICE","')) {
    console.error(`Corrupted NOTICE response detected from ${cassetteName} close`);
    ws.send(JSON.stringify(["NOTICE", `Error: Corrupted response from ${cassetteName} for CLOSE`]));
    return;
  }

  // If response includes non-printable characters, treat as corrupted
  if (/[\x00-\x1F\x7F-\x9F]/.test(response)) {
    console.error(`Response contains non-printable characters from ${cassetteName} close`);
    ws.send(JSON.stringify(["NOTICE", `Error: Corrupted response with invalid characters from ${cassetteName} for CLOSE`]));
    return;
  }
  
  try {
    // Try to parse the response as JSON
    const parsedResponse = JSON.parse(response);
    
    // Handle standard NIP-01 NOTICE message
    if (Array.isArray(parsedResponse) && parsedResponse.length >= 2 && parsedResponse[0] === "NOTICE") {
      // Forward NOTICE to client
      console.log(`Forwarding NOTICE from ${cassetteName} close response:`, parsedResponse[1]);
      ws.send(JSON.stringify(parsedResponse));
      return;
    }
    
    // Otherwise just log the response and send generic error
    console.log(`Non-NOTICE close response from ${cassetteName}:`, JSON.stringify(parsedResponse).substring(0, 100));
    ws.send(JSON.stringify(["NOTICE", `Error: Unexpected response format from ${cassetteName} for CLOSE`]));
  } catch (parseError) {
    // Not valid JSON, send error notice
    console.error(`Error parsing close response from ${cassetteName}:`, parseError);
    console.log(`Raw close response was: ${response.substring(0, 200)}`);
    ws.send(JSON.stringify(["NOTICE", `Error: Invalid JSON response from ${cassetteName} for CLOSE`]));
  }
}

// Initialize and start the server
(async () => {
  await loadCassettes();
  
  const server = serve({
    port: PORT,
    fetch(req, server) {
      // Upgrade the request to a WebSocket connection
      if (server.upgrade(req, {
        data: {
          subscriptions: new Map()
        }
      })) {
        return;
      }
      
      return new Response(`Boombox WebSocket Server - ${cassettes.length} cassettes loaded`, { status: 200 });
    },
    websocket: {
      message,
      open(ws: ServerWebSocket<WebSocketData>) {
        console.log("WebSocket connection opened");
      },
      close(ws: ServerWebSocket<WebSocketData>, code: number, message: string) {
        console.log(`WebSocket connection closed: ${code} ${message}`);
        // Cleanup subscriptions
        for (const [subId, sub] of ws.data.subscriptions) {
          console.log(`Cleaning up subscription ${subId}`);
          sub.active = false;
        }
        ws.data.subscriptions.clear();
      }
    }
  });
  
  console.log(`Boombox server listening on port ${PORT}`);
})();