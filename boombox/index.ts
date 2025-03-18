import { serve } from "bun";
import type { ServerWebSocket } from "bun";
import { join, dirname } from "path";
import { readFileSync, readdirSync } from "fs";
import { fileURLToPath } from "url";
import { validate } from "./schema-validator.js";
import type { Cassette as LoaderCassette } from "../cassette-loader/src/types";
import { loadCassette } from "../cassette-loader/dist/src/index.js";

// Get the current directory
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Configuration
const PORT = parseInt(process.env.PORT || "3001");
const WASM_DIR = process.env.WASM_DIR || join(__dirname, "..", "cassettes");
const LOG_FILE = process.env.LOG_FILE || join(__dirname, "..", "logs", "boombox-new.log");

// Interfaces
interface SubscriptionData {
  id: string;
  filters: any[];
  active: boolean;
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
    active: true
  });
  
  if (cassettes.length === 0) {
    ws.send(JSON.stringify(["EOSE", subId]));
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
        response = await cassette.instance.methods.req(reqStr);
        
        // Skip if response is empty or null
        if (!response) {
          console.warn(`Empty response from ${cassette.name}`);
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
  }
}

// Helper function to process responses from cassettes
function processResponse(ws: ServerWebSocket<WebSocketData>, subId: string, response: string, cassetteName: string) {
  // Sanitize the response to ensure it's valid JSON
  let sanitizedResponse = response;
  
  // Handle responses that might be corrupted or malformed
  if (response.startsWith('TICE","')) {
    // Fix for corrupted ["NOTICE",...] format
    sanitizedResponse = '["NO' + response;
    console.log(`Fixed corrupted NOTICE response from ${cassetteName}`);
  }
  
  try {
    // Try to parse the response as JSON
    const parsedResponse = JSON.parse(sanitizedResponse);
    
    // Handle standard NIP-01 message types
    if (Array.isArray(parsedResponse) && parsedResponse.length >= 2) {
      const [messageType, ...messageArgs] = parsedResponse;
      
      switch(messageType) {
        case "NOTICE":
          // Forward NOTICE messages to the client
          console.log(`Forwarding NOTICE from ${cassetteName}:`, messageArgs[0]);
          ws.send(sanitizedResponse);
          return;
          
        case "EVENT":
          // Handle EVENT message format ["EVENT", subscription_id, event]
          if (messageArgs.length >= 2) {
            // Check if this EVENT is for the correct subscription
            const eventSubId = messageArgs[0];
            if (eventSubId === subId) {
              console.log(`Got EVENT for subscription ${subId} from ${cassetteName}`);
              ws.send(sanitizedResponse);
            } else {
              console.warn(`Cassette ${cassetteName} returned EVENT for wrong subscription: ${eventSubId}`);
            }
          }
          return;
          
        case "EOSE":
          // Ignore EOSE messages, we'll send our own
          console.log(`Ignoring EOSE from ${cassetteName}`);
          return;
      }
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
        
        console.log(`Got ${parsedResponse.length} events from ${cassetteName}`);
        
        // Send each event as a proper NIP-01 EVENT message
        for (const event of parsedResponse) {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify(["EVENT", subId, event]));
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
      
      console.log(`Got ${parsedResponse.events.length} events in events array from ${cassetteName}`);
      
      // Send each event as a proper NIP-01 EVENT message
      for (const event of parsedResponse.events) {
        if (ws.readyState === WebSocket.OPEN && 
            event.id && 
            event.pubkey && 
            typeof event.kind === 'number') {
          ws.send(JSON.stringify(["EVENT", subId, event]));
        }
      }
      return;
    }
    
    // Otherwise treat as unrecognized format and log
    console.warn(`Unrecognized response format from ${cassetteName}:`, 
                 JSON.stringify(parsedResponse).substring(0, 100));
    
  } catch (parseError) {
    // Couldn't parse as JSON, try to extract events from HTML-encoded JSON
    if (response.includes('&quot;id&quot;') && 
        response.includes('&quot;pubkey&quot;') && 
        response.includes('&quot;kind&quot;')) {
      
      console.log(`Response from ${cassetteName} appears to be HTML-encoded JSON, attempting to decode`);
      
      try {
        // Decode HTML entities and try again
        const decoded = response.replace(/&quot;/g, '"')
                               .replace(/&lt;/g, '<')
                               .replace(/&gt;/g, '>')
                               .replace(/&amp;/g, '&');
        
        // Try to find JSON array pattern in the decoded string
        const jsonStart = decoded.indexOf('[{');
        const jsonEnd = decoded.lastIndexOf('}]') + 2;
        
        if (jsonStart >= 0 && jsonEnd > jsonStart) {
          const jsonStr = decoded.substring(jsonStart, jsonEnd);
          const events = JSON.parse(jsonStr);
          
          console.log(`Successfully extracted ${events.length} events from encoded response`);
          
          // Send each event
          for (const event of events) {
            if (ws.readyState === WebSocket.OPEN && 
                event.id && 
                event.pubkey && 
                typeof event.kind === 'number') {
              ws.send(JSON.stringify(["EVENT", subId, event]));
            }
          }
          return;
        }
      } catch (decodeError) {
        console.error(`Error decoding HTML-encoded JSON from ${cassetteName}:`, decodeError);
      }
    }
    
    // All attempts failed
    console.error(`Error processing response from ${cassetteName}:`, parseError);
    console.log(`Raw response was: ${response.substring(0, 200)}`);
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
  // Sanitize the response to ensure it's valid JSON
  let sanitizedResponse = response;
  
  // Handle responses that might be corrupted or malformed
  if (response.startsWith('TICE","')) {
    // Fix for corrupted ["NOTICE",...] format
    sanitizedResponse = '["NO' + response;
    console.log(`Fixed corrupted NOTICE response from ${cassetteName} close`);
  }
  
  try {
    // Try to parse the response as JSON
    const parsedResponse = JSON.parse(sanitizedResponse);
    
    // Handle standard NIP-01 NOTICE message
    if (Array.isArray(parsedResponse) && parsedResponse.length >= 2 && parsedResponse[0] === "NOTICE") {
      // Forward NOTICE to client
      console.log(`Forwarding NOTICE from ${cassetteName} close response:`, parsedResponse[1]);
      ws.send(sanitizedResponse);
      return;
    }
    
    // Otherwise just log the response
    console.log(`Non-NOTICE close response from ${cassetteName}:`, JSON.stringify(parsedResponse).substring(0, 100));
  } catch (parseError) {
    // Not valid JSON, log and continue
    console.error(`Error parsing close response from ${cassetteName}:`, parseError);
    console.log(`Raw close response was: ${response.substring(0, 200)}`);
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