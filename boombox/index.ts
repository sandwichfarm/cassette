import { serve } from "bun";
import type { ServerWebSocket } from "bun";
import { join, dirname } from "path";
import { readFileSync, readdirSync, existsSync } from "fs";
import { fileURLToPath } from "url";
import { validate } from "./schema-validator.js";

// Get the current directory
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Port for WebSocket server
const PORT = parseInt(process.env.PORT || "3001");

// Directory containing WASM modules
const WASM_DIR = join(__dirname, "../cassettes");

// Type for subscription data
interface SubscriptionData {
  filters: any[];
}

// Types for WebSocket data
interface WebSocketData {
  subscriptions: Map<string, SubscriptionData>;
}

// Interface for loaded cassettes
interface Cassette {
  name: string;
  instance: any;
  schemas: {
    incoming: any[];
    outgoing: any[];
  };
}

// Load all available cassettes
async function loadCassettes(): Promise<Cassette[]> {
  const WASM_DIR = '../cassettes';
  const cassettes: Cassette[] = [];
  
  console.log(`Loading cassettes from: ${join(process.cwd(), WASM_DIR)}`);
  
  // Get all files and directories in WASM_DIR
  const fileEntries = readdirSync(WASM_DIR, { withFileTypes: true });
  
  // Check for .wasm files directly in the WASM_DIR
  const wasmFiles = fileEntries
    .filter(dirent => !dirent.isDirectory() && dirent.name.endsWith('.wasm'));
  
  // Load modules from .wasm files directly
  for (const wasmFile of wasmFiles) {
    const cassetteName = wasmFile.name.replace('.wasm', '');
    try {
      console.log(`Loading cassette from WASM file: ${wasmFile.name}`);
      
      // Read the .wasm file
      const wasmPath = join(WASM_DIR, wasmFile.name);
      const wasmBuffer = readFileSync(wasmPath);
      
      // Create a memory instance
      const memory = new WebAssembly.Memory({ initial: 16 });
      
      // Standard imports object for all cassettes
      const importObject = {
        env: {
          memory: memory,
        },
        // Standard wasm-bindgen helpers
        __wbindgen_placeholder__: {
          __wbindgen_string_new: (ptr: number, len: number) => {
            const buf = new Uint8Array(memory.buffer).subarray(ptr, ptr + len);
            return new TextDecoder('utf-8').decode(buf);
          },
          __wbindgen_throw: (ptr: number, len: number) => {
            const buf = new Uint8Array(memory.buffer).subarray(ptr, ptr + len);
            throw new Error(new TextDecoder('utf-8').decode(buf));
          },
          __wbindgen_memory: () => memory
        }
      };
      
      // Compile and instantiate the WebAssembly module
      const wasmModule = await WebAssembly.instantiate(wasmBuffer, importObject);
      const exports = wasmModule.instance.exports;
      
      // Check for both standardized and legacy naming conventions
      const describeFn = exports.describe as Function || exports.describe_wasm as Function || exports.DESCRIBE as Function;
      const reqFn = exports.req as Function || exports.req_wasm as Function || exports.REQ as Function;
      const closeFn = exports.close as Function || exports.close_wasm as Function || exports.CLOSE as Function;
      const allocFn = exports.allocString as Function || exports.alloc_string as Function;
      const deallocFn = exports.deallocString as Function || exports.dealloc_string as Function;
      
      // Verify the module has the required functions
      if (!describeFn) {
        console.warn(`Cassette ${cassetteName} is missing the describe function`);
        continue;
      }
      
      if (!reqFn) {
        console.warn(`Cassette ${cassetteName} is missing the req function`);
        continue;
      }
      
      // Create a standardized wrapper for the module
      const cassetteWrapper = {
        req: function(request: string) {
          if (allocFn && deallocFn) {
            // Using memory management functions
            const encoder = new TextEncoder();
            const requestData = encoder.encode(request);
            
            // Allocate memory for the request
            const requestPtr = allocFn(requestData.length) as number;
            
            // Copy data to the WASM memory
            const memoryView = new Uint8Array(memory.buffer);
            for (let i = 0; i < requestData.length; i++) {
              memoryView[requestPtr + i] = requestData[i];
            }
            
            // Call the request function
            const resultPtr = reqFn(requestPtr, requestData.length) as number;
            
            // Get the result length by finding null terminator
            let resultLength = 0;
            while (memoryView[resultPtr + resultLength] !== 0) {
              resultLength++;
            }
            
            // Extract the result string
            const resultData = memoryView.slice(resultPtr, resultPtr + resultLength);
            const result = new TextDecoder().decode(resultData);
            
            // Free allocated memory
            deallocFn(requestPtr, requestData.length);
            
            return result;
          } else {
            // Direct call approach
            return reqFn(request) as string;
          }
        },
        
        close: function(closeRequest: string) {
          if (closeFn) {
            if (allocFn && deallocFn) {
              // Using memory management functions
              const encoder = new TextEncoder();
              const requestData = encoder.encode(closeRequest);
              
              // Allocate memory for the request
              const requestPtr = allocFn(requestData.length) as number;
              
              // Copy data to the WASM memory
              const memoryView = new Uint8Array(memory.buffer);
              for (let i = 0; i < requestData.length; i++) {
                memoryView[requestPtr + i] = requestData[i];
              }
              
              // Call the close function
              const resultPtr = closeFn(requestPtr, requestData.length) as number;
              
              // Get the result length by finding null terminator
              let resultLength = 0;
              while (memoryView[resultPtr + resultLength] !== 0) {
                resultLength++;
              }
              
              // Extract the result string
              const resultData = memoryView.slice(resultPtr, resultPtr + resultLength);
              const result = new TextDecoder().decode(resultData);
              
              // Free allocated memory
              deallocFn(requestPtr, requestData.length);
              
              return result;
            } else {
              // Direct call approach
              return closeFn(closeRequest) as string;
            }
          } else {
            return JSON.stringify({"notice": ["NOTICE", "Close not implemented"]});
          }
        },
        
        describe: function() {
          return describeFn() as string;
        }
      };
      
      // Get cassette metadata from the describe function
      let description = describeFn() as string;
      console.log(`${cassetteName} description: ${description}`);
      
      // Parse description to get schema information
      let incomingSchemas: any[] = [{ type: "array" }];
      let outgoingSchemas: any[] = [{ type: "array" }];
      
      try {
        const descriptionObj = JSON.parse(description);
        // Here we'll assume incoming schemas are for REQ messages
        // and outgoing schemas are for EVENT messages
        if (descriptionObj && descriptionObj.req) {
          incomingSchemas = [{ type: "array" }]; // Simple schema that accepts any array
        }
        if (descriptionObj) {
          outgoingSchemas = [{ type: "array" }]; // Simple schema that accepts any array
        }
      } catch (err) {
        console.warn(`Failed to parse description for cassette ${cassetteName}:`, err);
      }
      
      // Add the cassette to the list
      cassettes.push({
        name: cassetteName,
        instance: cassetteWrapper,
        schemas: {
          incoming: incomingSchemas,
          outgoing: outgoingSchemas
        }
      });
      
      console.log(`Successfully loaded cassette: ${cassetteName}`);
    } catch (err) {
      console.error(`Failed to load cassette ${cassetteName}:`, err);
    }
  }
  
  return cassettes;
}

// Load all available cassettes
const cassettes = await loadCassettes();
console.log(`Loaded ${cassettes.length} cassettes`);

// Start WebSocket server
serve({
  port: PORT,
  fetch(req: Request, server: any) {
    // Handle WebSocket upgrade
    if (server.upgrade(req)) {
      return; // Upgraded to WebSocket
    }
    
    // Return status for HTTP requests
    return new Response(`Boombox server running - Loaded ${cassettes.length} cassettes`, {
      status: 200,
      headers: { "Content-Type": "text/plain" }
    });
  },
  websocket: {
    open(ws: ServerWebSocket<WebSocketData>) {
      console.log("Connection opened");
      
      // Set a data property to track this connection's subscriptions
      ws.data = {
        subscriptions: new Map<string, SubscriptionData>()
      };
    },
    message(ws: ServerWebSocket<WebSocketData>, message: string | Uint8Array) {
      try {
        // Parse the incoming message
        const msgStr = message.toString();
        const msgData = JSON.parse(msgStr);
        
        // Handle different Nostr message types
        if (Array.isArray(msgData)) {
          const messageType = msgData[0];
          
          if (messageType === "EVENT" && msgData.length >= 2) {
            // Handle EVENT message
            handleEventMessage(ws, msgData);
          } else if (messageType === "REQ" && msgData.length >= 3) {
            // Handle REQ message
            handleReqMessage(ws, msgData);
          } else if (messageType === "CLOSE" && msgData.length >= 2) {
            // Handle CLOSE message
            handleCloseMessage(ws, msgData);
          } else {
            // Unknown message type
            console.warn(`Unknown or invalid message format: ${msgStr}`);
          }
        } else {
          console.warn(`Non-array message received: ${msgStr}`);
        }
      } catch (error) {
        console.error("Error processing message:", error);
      }
    },
    close(ws: ServerWebSocket<WebSocketData>) {
      console.log("Connection closed");
      
      // Clean up any subscriptions or resources
      if (ws.data && ws.data.subscriptions) {
        // Close any active subscriptions
        for (const [subId, sub] of ws.data.subscriptions.entries()) {
          console.log(`Closing subscription ${subId}`);
          // If there's any cleanup needed for subscriptions, do it here
        }
      }
    }
  }
});

console.log(`Boombox server running on port ${PORT}`);

// Handler for EVENT messages
function handleEventMessage(ws: ServerWebSocket<WebSocketData>, message: any[]) {
  const event = message[1];
  console.log(`Received EVENT with kind=${event.kind}`);
  
  // Find cassettes that can handle this event based on schemas
  for (const cassette of cassettes) {
    try {
      // Check if this cassette accepts this event type
      const isValid = cassette.schemas.incoming.some((schema: any) => {
        return validate(message, schema);
      });
      
      if (isValid) {
        console.log(`Processing event with cassette: ${cassette.name}`);
        
        // Call the cassette to process the event
        if (cassette.instance.processEvent) {
          try {
            const responses = cassette.instance.processEvent(message);
            
            // Send responses back to the client
            if (Array.isArray(responses)) {
              for (const response of responses) {
                ws.send(JSON.stringify(response));
              }
            } else if (responses) {
              ws.send(JSON.stringify(responses));
            }
          } catch (error) {
            console.error(`Error in cassette ${cassette.name}:`, error);
          }
        }
      }
    } catch (error) {
      console.error(`Error validating with cassette ${cassette.name}:`, error);
    }
  }
}

// Handler for REQ messages
function handleReqMessage(ws: ServerWebSocket<WebSocketData>, message: any[]) {
  const subscriptionId = message[1];
  const filters = message.slice(2);
  
  console.log(`Received REQ with subscription ID: ${subscriptionId}`);
  console.log(`Filters:`, JSON.stringify(filters));
  
  // Store the subscription
  if (ws.data) {
    ws.data.subscriptions.set(subscriptionId, { filters });
  }
  
  // Find cassettes that can handle this request based on schemas
  for (const cassette of cassettes) {
    try {
      // Check if this cassette can handle this subscription
      const isValid = cassette.schemas.incoming.some((schema: any) => {
        return validate(message, schema);
      });
      
      if (isValid) {
        console.log(`Processing subscription with cassette: ${cassette.name}`);
        
        // Get the appropriate req function depending on the cassette structure
        let reqFunction = cassette.instance.req || 
                          (cassette.instance.SandwichsFavs && cassette.instance.SandwichsFavs.req) ||
                          (cassette.instance.SandwichNotes && cassette.instance.SandwichNotes.req) ||
                          (cassette.instance.CustomCassette && cassette.instance.CustomCassette.req);
        
        if (!reqFunction) {
          // Try to find the req function by checking all exports
          const exportNames = Object.keys(cassette.instance);
          for (const exportName of exportNames) {
            const exportedValue = cassette.instance[exportName];
            if (exportedValue && typeof exportedValue.req === 'function') {
              console.log(`Found req function in export: ${exportName}`);
              reqFunction = exportedValue.req.bind(exportedValue);
              break;
            }
          }
        }
        
        // Handle differently based on the cassette's available methods
        if (reqFunction) {
          // For cassettes like sandwichs_favs that use the req method
          try {
            // Convert the message to a string as required by the req method
            const reqMessage = JSON.stringify(message);
            console.log(`Calling req with:`, reqMessage);
            
            // Call the req method and parse the result
            const responseStr = reqFunction(reqMessage);
            console.log(`Got response:`, responseStr);
            
            // Parse the response
            const response = JSON.parse(responseStr);
            console.log(`Parsed response:`, JSON.stringify(response));
            
            // Send events
            if (response.events && Array.isArray(response.events)) {
              console.log(`Sending ${response.events.length} events`);
              for (const event of response.events) {
                console.log(`Sending event:`, JSON.stringify(event));
                ws.send(JSON.stringify(event));
              }
            } else {
              console.log(`No events found in response`);
            }
            
            // Send EOSE
            if (response.eose) {
              console.log(`Sending EOSE:`, JSON.stringify(response.eose));
              ws.send(JSON.stringify(response.eose));
            } else {
              console.log(`No EOSE found in response`);
            }
          } catch (error) {
            console.error(`Error in cassette ${cassette.name} req method:`, error);
          }
        } else if (cassette.instance.handleSubscription) {
          try {
            const callback = (response: any) => {
              if (ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify(response));
              }
            };
            
            cassette.instance.handleSubscription(subscriptionId, filters, callback);
            
            // Send EOSE (End of Stored Events) if appropriate
            ws.send(JSON.stringify(["EOSE", subscriptionId]));
          } catch (error) {
            console.error(`Error in cassette ${cassette.name}:`, error);
          }
        }
      }
    } catch (error) {
      console.error(`Error validating with cassette ${cassette.name}:`, error);
    }
  }
}

// Handler for CLOSE messages
function handleCloseMessage(ws: ServerWebSocket<WebSocketData>, message: any[]) {
  const subscriptionId = message[1];
  console.log(`Received CLOSE for subscription: ${subscriptionId}`);
  
  // Remove the subscription
  if (ws.data && ws.data.subscriptions.has(subscriptionId)) {
    ws.data.subscriptions.delete(subscriptionId);
  }
  
  // Notify cassettes about the closed subscription
  for (const cassette of cassettes) {
    // Get the appropriate close function depending on the cassette structure
    let closeFunction = cassette.instance.close || 
                      (cassette.instance.SandwichsFavs && cassette.instance.SandwichsFavs.close) ||
                      (cassette.instance.SandwichNotes && cassette.instance.SandwichNotes.close) ||
                      (cassette.instance.CustomCassette && cassette.instance.CustomCassette.close);
    
    if (!closeFunction) {
      // Try to find the close function by checking all exports
      const exportNames = Object.keys(cassette.instance);
      for (const exportName of exportNames) {
        const exportedValue = cassette.instance[exportName];
        if (exportedValue && typeof exportedValue.close === 'function') {
          console.log(`Found close function in export: ${exportName}`);
          closeFunction = exportedValue.close.bind(exportedValue);
          break;
        }
      }
    }
    
    // For cassettes using the close method
    if (closeFunction) {
      try {
        // Convert the message to a string as required by the close method
        const closeMessage = JSON.stringify(message);
        
        // Call the close method and parse the result
        const responseStr = closeFunction(closeMessage);
        console.log(`Close response from ${cassette.name}:`, responseStr);
        
        try {
          // Parse the response and send it if it's not empty
          const response = JSON.parse(responseStr);
          if (response) {
            ws.send(JSON.stringify(response));
          }
        } catch (error) {
          console.warn(`Error parsing close response from ${cassette.name}:`, error);
        }
      } catch (error) {
        console.error(`Error in cassette ${cassette.name} close method:`, error);
      }
    }
  }
}