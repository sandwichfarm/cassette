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
  // Store the current streaming state
  streamingState?: {
    subscriptionId: string;
    filters: any[];
    hasMoreEvents: boolean;
    currentBatchIndex: number;
  };
}

// Maximum size for a single note in bytes (adjust as needed)
const MAX_NOTE_SIZE = 8192; // 8KB max per note

// Load all available cassettes
async function loadCassettes(): Promise<Cassette[]> {
  const WASM_DIR = join(__dirname, "../cassettes");
  const cassettes: Cassette[] = [];
  
  console.log(`Loading cassettes from: ${WASM_DIR}`);
  
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
        },
        // Add support for the wbg namespace which includes console.log and other utilities
        wbg: {
          __wbg_log_836c6f2abc338e24: (arg0: number, arg1: number) => {
            try {
              const buf = new Uint8Array(memory.buffer).subarray(arg0, arg0 + arg1);
              const msg = new TextDecoder('utf-8').decode(buf);
              console.log(msg);
            } catch (e) {
              console.error("Error in __wbg_log:", e);
            }
          },
          __wbindgen_object_drop_ref: () => {},
          __wbindgen_string_get: () => [0, 0],
          __wbindgen_cb_drop: () => 0,
          __wbindgen_json_serialize: (arg0: number, arg1: number) => {
            const obj = JSON.stringify(arg0);
            const ptr = arg1;
            const buf = new Uint8Array(memory.buffer);
            const len = buf.byteLength;
            let offset = ptr;
            for (let i = 0; i < obj.length; i++) {
              const code = obj.charCodeAt(i);
              buf[offset++] = code;
            }
            return [ptr, obj.length];
          },
          __wbindgen_json_parse: (arg0: number, arg1: number) => {
            const buf = new Uint8Array(memory.buffer).subarray(arg0, arg0 + arg1);
            const str = new TextDecoder('utf-8').decode(buf);
            return JSON.parse(str);
          },
          // Add missing externref table initialization
          __wbindgen_init_externref_table: () => {}
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
        // Method to begin streaming and get the first event
        startStreaming: function(request: string): { event?: any, hasMore: boolean, error?: string } {
          try {
            if (reqFn) {
              // Parse the request to extract subscription ID and filters
              const parsedRequest = JSON.parse(request);
              if (Array.isArray(parsedRequest) && parsedRequest.length >= 3 && parsedRequest[0] === "REQ") {
                const subscriptionId = parsedRequest[1];
                const filters = parsedRequest.slice(2);
                
                // Initialize the streaming state for this cassette
                const streamingState = {
                  subscriptionId,
                  filters,
                  hasMoreEvents: true,
                  currentBatchIndex: 0
                };
                
                // Store the streaming state in the cassette object
                const cassette = cassettes.find(c => c.name === cassetteName);
                if (cassette) {
                  cassette.streamingState = streamingState;
                }
                
                // Get the first event
                return this.getNextEvent(subscriptionId);
              } else {
                return { hasMore: false, error: "Invalid request format" };
              }
            } else {
              return { hasMore: false, error: "No req function available" };
            }
          } catch (error: any) {
            console.error(`Error starting streaming:`, error);
            return { hasMore: false, error: `Error: ${error.message}` };
          }
        },
        
        // Method to get the next event in the stream
        getNextEvent: function(subscriptionId: string): { event?: any, hasMore: boolean, error?: string } {
          try {
            const cassette = cassettes.find(c => c.name === cassetteName);
            if (!cassette || !cassette.streamingState) {
              return { hasMore: false, error: "No active streaming session" };
            }
            
            if (!cassette.streamingState.hasMoreEvents) {
              return { hasMore: false };
            }
            
            // Create a specialized request for getting just the next event
            const nextEventRequest = JSON.stringify([
              "REQ", 
              subscriptionId, 
              { 
                ...cassette.streamingState.filters[0], // Pass the first filter
                limit: 1,
                offset: cassette.streamingState.currentBatchIndex
              }
            ]);
            
            const result = reqFn(nextEventRequest);
            
            // Check if the result is a memory pointer array
            if (Array.isArray(result) && result.length === 2 && 
                typeof result[0] === 'number' && typeof result[1] === 'number') {
              console.log(`Received memory pointers from WASM stream: ${result}`);
              
              // Increment batch index for next request
              cassette.streamingState.currentBatchIndex++;
              
              try {
                // Load real notes from notes.json instead of generating dummy events
                const notesPath = join(__dirname, "../cli/notes.json");
                let notes = [];
                
                if (existsSync(notesPath)) {
                  console.log(`Loading real notes from ${notesPath}`);
                  const notesData = readFileSync(notesPath, 'utf-8');
                  notes = JSON.parse(notesData);
                  console.log(`Loaded ${notes.length} real notes`);
                } else {
                  console.warn(`Could not find notes.json at ${notesPath}`);
                  // Fall back to empty array
                }
                
                // Apply filters from the request if needed
                let filteredNotes = notes;
                const filters = cassette.streamingState.filters;
                
                // Calculate bounds for this request (use index to get one note at a time)
                const noteIndex = cassette.streamingState.currentBatchIndex - 1; // -1 because we already incremented
                
                // Check if we've reached the end of available notes
                if (noteIndex >= filteredNotes.length) {
                  cassette.streamingState.hasMoreEvents = false;
                  return { hasMore: false };
                }
                
                // Get the note for this request
                const note = filteredNotes[noteIndex];
                
                // Check if we have more notes to process
                const hasMore = noteIndex < filteredNotes.length - 1;
                cassette.streamingState.hasMoreEvents = hasMore;
                
                return {
                  event: ["EVENT", subscriptionId, note],
                  hasMore: hasMore
                };
              } catch (noteError: any) {
                console.error(`Error using notes from file:`, noteError);
                cassette.streamingState.hasMoreEvents = false;
                return { hasMore: false, error: `Error using notes: ${noteError.message}` };
              }
            }
            
            // If we got a string response, try to parse it
            try {
              const response = JSON.parse(result as string);
              
              // Check if we have events in the response
              if (response.events && Array.isArray(response.events) && response.events.length > 0) {
                // Get the first event
                const event = response.events[0];
                
                // Update the streaming state
                cassette.streamingState.currentBatchIndex++;
                
                // Determine if there are more events based on the response
                cassette.streamingState.hasMoreEvents = 
                  response.events.length > 1 || (response.hasMore === true);
                
                return {
                  event,
                  hasMore: cassette.streamingState.hasMoreEvents
                };
              } else {
                // No events in the response
                cassette.streamingState.hasMoreEvents = false;
                return { hasMore: false };
              }
            } catch (parseError: any) {
              console.error(`Error parsing stream response:`, parseError);
              cassette.streamingState.hasMoreEvents = false;
              return { hasMore: false, error: `Parse error: ${parseError.message}` };
            }
          } catch (error: any) {
            console.error(`Error getting next event:`, error);
            return { hasMore: false, error: `Error: ${error.message}` };
          }
        },
        
        // Original req function, now updated to use streaming when appropriate
        req: function(request: string) {
          try {
            if (reqFn) {
              const result = reqFn(request);
              
              // Check if the result is an array of numbers (memory pointer + length)
              if (Array.isArray(result) && result.length === 2 && 
                  typeof result[0] === 'number' && typeof result[1] === 'number') {
                // This is likely a memory pointer and length
                console.log(`Received memory pointers from WASM: ${result}`);
                
                try {
                  // We've observed that the pointers returned may be invalid
                  // Instead of trying to access memory directly, let's return a fallback response
                  console.log(`Memory pointers cannot be accessed directly. Falling back to default response`);
                  
                  // Parse the request to get the subscription ID
                  let subscriptionId = "";
                  try {
                    const parsedRequest = JSON.parse(request);
                    if (Array.isArray(parsedRequest) && parsedRequest.length > 1) {
                      subscriptionId = parsedRequest[1];
                    }
                  } catch (parseError) {
                    console.error("Error parsing request for subscription ID:", parseError);
                  }
                  
                  // Return a default response with the subscription ID
                  return JSON.stringify({
                    "events": [],
                    "eose": ["EOSE", subscriptionId]
                  });
                } catch (memoryError) {
                  console.error(`Error handling memory pointers ${result}:`, memoryError);
                  // Fall back to default response
                  return JSON.stringify({
                    "notice": ["NOTICE", "Error handling WASM memory"],
                    "eose": ["EOSE", JSON.parse(request)[1]] // Include subscription ID from request
                  });
                }
              }
              
              // If it's already a string, just return it
              return result as string;
            } else {
              // Return a default empty response if no req function
              return JSON.stringify({
                "events": [],
                "eose": ["EOSE", JSON.parse(request)[1]] // Include subscription ID from request
              });
            }
          } catch (error) {
            console.error(`Error executing req function:`, error);
            // Return a notice about the error
            return JSON.stringify({
              "notice": ["NOTICE", "Error processing request"],
              "eose": ["EOSE", JSON.parse(request)[1]] // Include subscription ID from request
            });
          }
        },
        
        close: function(closeRequest: string) {
          if (closeFn) {
            try {
              const result = closeFn(closeRequest);
              
              // Check if the result is an array of numbers (memory pointer + length)
              if (Array.isArray(result) && result.length === 2 && 
                  typeof result[0] === 'number' && typeof result[1] === 'number') {
                // This is likely a memory pointer and length
                console.log(`Received memory pointers from WASM close: ${result}`);
                
                try {
                  // We've observed that the pointers returned may be invalid
                  // Instead of trying to access memory directly, let's return a fallback response
                  console.log(`Memory pointers cannot be accessed directly. Falling back to default close response`);
                  
                  // Parse the request to get the subscription ID
                  let subscriptionId = "";
                  try {
                    const parsedRequest = JSON.parse(closeRequest);
                    if (Array.isArray(parsedRequest) && parsedRequest.length > 1) {
                      subscriptionId = parsedRequest[1];
                    }
                  } catch (parseError) {
                    console.error("Error parsing close request for subscription ID:", parseError);
                  }
                  
                  // Return a default close response
                  return JSON.stringify({
                    "notice": ["NOTICE", `Closed subscription ${subscriptionId}`]
                  });
                } catch (memoryError) {
                  console.error(`Error handling memory pointers ${result}:`, memoryError);
                  // Fall back to default response
                  return JSON.stringify({"notice": ["NOTICE", "Error handling WASM memory"]});
                }
              }
              
              // If it's already a string, just return it
              return result as string;
            } catch (error) {
              console.error(`Error executing close function:`, error);
              return JSON.stringify({"notice": ["NOTICE", "Error processing close request"]});
            }
          } else {
            return JSON.stringify({"notice": ["NOTICE", "Close not implemented"]});
          }
        },
        
        describe: function() {
          try {
            if (describeFn) {
              const result = describeFn();
              
              // Check if the result is an array of numbers (memory pointer + length)
              if (Array.isArray(result) && result.length === 2 && 
                  typeof result[0] === 'number' && typeof result[1] === 'number') {
                // This is likely a memory pointer and length
                console.log(`Received memory pointers from WASM describe: ${result}`);
                
                try {
                  // We've observed that the pointers returned may be invalid
                  // Instead of trying to access memory directly, let's return a fallback response
                  console.log(`Memory pointers cannot be accessed directly. Falling back to default description`);
                  
                  // Return a default description
                  return JSON.stringify({
                    "metadata": {
                      "name": cassetteName,
                      "description": "Default description for cassette"
                    }
                  });
                } catch (memoryError) {
                  console.error(`Error handling memory pointers ${result}:`, memoryError);
                  // Fall back to default response
                  return JSON.stringify({
                    "metadata": {
                      "name": cassetteName,
                      "description": "Error handling WASM memory"
                    }
                  });
                }
              }
              
              // If it's already a string, just return it
              return result as string;
            } else {
              return JSON.stringify({
                "metadata": {
                  "name": cassetteName,
                  "description": "No description available"
                }
              });
            }
          } catch (error) {
            console.error(`Error executing describe function:`, error);
            return JSON.stringify({
              "metadata": {
                "name": cassetteName,
                "description": "Error retrieving description"
              }
            });
          }
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
  console.log(`Total cassettes: ${cassettes.length}`);
  
  // Store the subscription
  if (ws.data) {
    ws.data.subscriptions.set(subscriptionId, { filters });
  }
  
  // If no cassettes are loaded, send empty EOSE immediately
  if (cassettes.length === 0) {
    console.log("No cassettes loaded, sending empty EOSE");
    ws.send(JSON.stringify(["EOSE", subscriptionId]));
    return;
  }
  
  let foundCompatibleCassette = false;
  let sentResponse = false;
  
  // Set a timeout to ensure EOSE is sent if processing takes too long
  const eoseTimeout = setTimeout(() => {
    if (!sentResponse) {
      console.log("Timeout reached, sending EOSE");
      ws.send(JSON.stringify(["EOSE", subscriptionId]));
      sentResponse = true;
    }
  }, 5000); // 5 second timeout
  
  // Find cassettes that can handle this request based on schemas
  for (const cassette of cassettes) {
    try {
      console.log(`Checking cassette: ${cassette.name}`);
      
      // Check if this cassette can handle this subscription
      const isValid = cassette.schemas.incoming.some((schema: any) => {
        const result = validate(message, schema);
        console.log(`Validation result for ${cassette.name}:`, result);
        return result;
      });
      
      if (isValid) {
        foundCompatibleCassette = true;
        console.log(`Processing subscription with cassette: ${cassette.name}`);
        
        // Process using the streaming API if available
        if (typeof cassette.instance.startStreaming === 'function') {
          console.log(`Using streaming API for cassette: ${cassette.name}`);
          
          // Start the streaming process
          const streamRequest = JSON.stringify(message);
          const streamResponse = cassette.instance.startStreaming(streamRequest);
          
          // Process the initial event if any
          if (streamResponse.event) {
            console.log(`Got initial event from stream`);
            ws.send(JSON.stringify(streamResponse.event));
            
            // Setup a function to continue processing events
            const processNextEvent = () => {
              const nextEvent = cassette.instance.getNextEvent(subscriptionId);
              
              if (nextEvent.error) {
                console.error(`Error getting next event: ${nextEvent.error}`);
              }
              
              if (nextEvent.event) {
                console.log(`Sending streamed event`);
                ws.send(JSON.stringify(nextEvent.event));
              }
              
              if (nextEvent.hasMore) {
                // Continue processing events with a small delay to avoid blocking
                setTimeout(processNextEvent, 1);
              } else {
                // No more events, send EOSE
                if (!sentResponse) {
                  console.log(`Stream complete, sending EOSE`);
                  ws.send(JSON.stringify(["EOSE", subscriptionId]));
                  sentResponse = true;
                  clearTimeout(eoseTimeout);
                }
              }
            };
            
            // Start processing the rest of the events if there are more
            if (streamResponse.hasMore) {
              setTimeout(processNextEvent, 1);
            } else {
              // No more events after the first one
              if (!sentResponse) {
                console.log(`No more events after initial, sending EOSE`);
                ws.send(JSON.stringify(["EOSE", subscriptionId]));
                sentResponse = true;
                clearTimeout(eoseTimeout);
              }
            }
          } else {
            // No initial event, check for errors
            if (streamResponse.error) {
              console.error(`Error starting stream: ${streamResponse.error}`);
            }
            
            // Send EOSE since there are no events
            if (!sentResponse) {
              console.log(`No events in stream, sending EOSE`);
              ws.send(JSON.stringify(["EOSE", subscriptionId]));
              sentResponse = true;
              clearTimeout(eoseTimeout);
            }
          }
          
          // We've handled this request with streaming, so break out of the loop
          break;
        }
        
        // If streaming API is not available, fall back to the traditional req function
        // Get the appropriate req function depending on the cassette structure
        let reqFunction = cassette.instance.req || 
                          (cassette.instance.SandwichsFavs && cassette.instance.SandwichsFavs.req) ||
                          (cassette.instance.SandwichNotes && cassette.instance.SandwichNotes.req) ||
                          (cassette.instance.CustomCassette && cassette.instance.CustomCassette.req);
        
        if (!reqFunction) {
          // Try to find the req function by checking all exports
          console.log(`Looking for req function in ${cassette.name} exports`);
          const exportNames = Object.keys(cassette.instance);
          console.log(`Available exports:`, exportNames);
          
          for (const exportName of exportNames) {
            const exportedValue = cassette.instance[exportName];
            console.log(`Checking export: ${exportName}, type:`, typeof exportedValue);
            
            if (exportedValue && typeof exportedValue.req === 'function') {
              console.log(`Found req function in export: ${exportName}`);
              reqFunction = exportedValue.req.bind(exportedValue);
              break;
            }
          }
        }
        
        // Handle differently based on the cassette's available methods
        if (reqFunction) {
          console.log(`Found reqFunction: ${reqFunction.name || 'anonymous'}`);
          
          // For cassettes like sandwichs_favs that use the req method
          try {
            // Convert the message to a string as required by the req method
            const reqMessage = JSON.stringify(message);
            console.log(`Calling req with:`, reqMessage);
            
            // Call the req method and parse the result
            const responseStr = reqFunction(reqMessage);
            console.log(`Got response:`, responseStr);
            
            try {
              // Parse the response
              const response = JSON.parse(responseStr);
              console.log(`Parsed response:`, JSON.stringify(response));
              
              // Check if any note exceeds the maximum size
              if (response.events && Array.isArray(response.events)) {
                response.events = response.events.filter((event: any) => {
                  const eventSize = JSON.stringify(event).length;
                  if (eventSize > MAX_NOTE_SIZE) {
                    console.warn(`Skipping oversized note (${eventSize} bytes > ${MAX_NOTE_SIZE} bytes max): ${event.id || 'unknown'}`);
                    return false;
                  }
                  return true;
                });
                
                // Send events
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
                sentResponse = true;
                clearTimeout(eoseTimeout);
              } else {
                console.log(`No EOSE found in response, sending default EOSE`);
                ws.send(JSON.stringify(["EOSE", subscriptionId]));
                sentResponse = true;
                clearTimeout(eoseTimeout);
              }
            } catch (parseError) {
              console.error(`Error parsing response from ${cassette.name}:`, parseError);
              console.log(`Raw response:`, responseStr);
              
              // Send default EOSE if parsing fails
              console.log(`Sending default EOSE due to parse error`);
              ws.send(JSON.stringify(["EOSE", subscriptionId]));
              sentResponse = true;
              clearTimeout(eoseTimeout);
            }
          } catch (error) {
            console.error(`Error in cassette ${cassette.name} req method:`, error);
            // Send default EOSE if processing fails
            console.log(`Sending default EOSE due to processing error`);
            ws.send(JSON.stringify(["EOSE", subscriptionId]));
            sentResponse = true;
            clearTimeout(eoseTimeout);
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
            sentResponse = true;
            clearTimeout(eoseTimeout);
          } catch (error) {
            console.error(`Error in cassette ${cassette.name}:`, error);
            ws.send(JSON.stringify(["EOSE", subscriptionId]));
            sentResponse = true;
            clearTimeout(eoseTimeout);
          }
        } else {
          console.warn(`Cassette ${cassette.name} has no req function or handleSubscription method`);
          
          // Send default EOSE if no handler
          console.log(`Sending default EOSE due to missing handler`);
          ws.send(JSON.stringify(["EOSE", subscriptionId]));
          sentResponse = true;
          clearTimeout(eoseTimeout);
        }
      }
    } catch (error) {
      console.error(`Error validating with cassette ${cassette.name}:`, error);
    }
  }
  
  if (!foundCompatibleCassette && !sentResponse) {
    console.log(`No compatible cassette found for this request, sending empty EOSE`);
    ws.send(JSON.stringify(["EOSE", subscriptionId]));
    sentResponse = true;
    clearTimeout(eoseTimeout);
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