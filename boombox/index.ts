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
const WASM_DIR = join(__dirname, "wasm");

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

// Load cassettes from the WASM directory
async function loadCassettes(): Promise<Cassette[]> {
  console.log(`Loading cassettes from: ${WASM_DIR}`);
  
  const cassettes: Cassette[] = [];
  
  try {
    // First, try to load SandwichsFavs module directly
    try {
      console.log(`Loading SandwichsFavs module directly`);
      const { SandwichsFavs, default: initWasm } = await import('./wasm/sandwichs_favs.js');
      
      // Initialize the WASM module
      await initWasm();
      
      console.log(`SandwichsFavs module loaded, checking for describe method`);
      
      if (typeof SandwichsFavs.describe === 'function') {
        const description = SandwichsFavs.describe();
        console.log(`SandwichsFavs description: ${description}`);
        
        // Create the cassette object
        cassettes.push({
          name: 'sandwichs_favs',
          instance: {
            SandwichsFavs,
            req: SandwichsFavs.req.bind(SandwichsFavs),
            close: SandwichsFavs.close.bind(SandwichsFavs)
          },
          schemas: {
            incoming: [{ type: "array" }],
            outgoing: [{ type: "array" }]
          }
        });
        
        console.log(`Successfully loaded SandwichsFavs module directly`);
      }
    } catch (err) {
      console.error(`Failed to directly load SandwichsFavs:`, err);
      console.log(`Falling back to directory scanning method`);
      
      // Get all files and directories in WASM_DIR
      const dirents = readdirSync(WASM_DIR, { withFileTypes: true });
      
      // Check for direct .js files (not _bg.js files) in the WASM_DIR
      const jsModules = dirents
        .filter(dirent => !dirent.isDirectory() && dirent.name.endsWith('.js') && !dirent.name.endsWith('_bg.js'));
      
      // Load modules from direct .js files
      for (const jsModule of jsModules) {
        const cassetteName = jsModule.name.replace('.js', '');
        try {
          console.log(`Loading cassette from file: ${jsModule.name}`);
          
          // Import the cassette module
          const modulePath = join(WASM_DIR, jsModule.name);
          const module = await import(modulePath);
          
          // Try to get schemas from the module's describe method
          let incomingSchemas: any[] = [];
          let outgoingSchemas: any[] = [];
          
          try {
            if (module && typeof module.default === 'function') {
              // Initialize WASM module if needed
              await module.default();
            }
            
            // Try different possible capitalization of the module name
            const moduleNames = [
              cassetteName,                              // sandwichs_favs
              cassetteName.charAt(0).toUpperCase() + cassetteName.slice(1), // Sandwichs_favs
              'SandwichsFavs',                          // SandwichsFavs
              'sandwichsFavs'                           // sandwichsFavs
            ];
            
            let moduleFound = false;
            
            for (const moduleName of moduleNames) {
              if (!moduleFound && module[moduleName] && typeof module[moduleName].describe === 'function') {
                console.log(`Found module with name: ${moduleName}`);
                console.log(`Getting schemas from ${moduleName}'s describe() method`);
                const description = module[moduleName].describe();
                console.log(`Module description: ${description}`);
                
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
                  
                  moduleFound = true;
                  break; // Exit the loop once we've found and processed a module
                } catch (err) {
                  console.warn(`Failed to parse description for module ${moduleName}:`, err);
                }
              }
            }
            
            // Fallback to checking for a schemas file with the same name
            if (!moduleFound) {
              console.log(`No describe() method found, checking for schema file`);
              const schemaPath = join(WASM_DIR, `${cassetteName}.schemas.json`);
              if (existsSync(schemaPath)) {
                const schemas = JSON.parse(readFileSync(schemaPath, 'utf-8'));
                incomingSchemas = schemas.incoming || [];
                outgoingSchemas = schemas.outgoing || [];
              }
            }
          } catch (err) {
            console.warn(`Failed to load schemas for cassette ${cassetteName}:`, err);
          }
          
          cassettes.push({
            name: cassetteName,
            instance: module,
            schemas: {
              incoming: incomingSchemas,
              outgoing: outgoingSchemas
            }
          });
          
          console.log(`Successfully loaded cassette: ${cassetteName}`);
        } catch (err) {
          console.error(`Failed to load cassette file ${jsModule.name}:`, err);
        }
      }
    }
    
    // Also process directories for backward compatibility
    const dirents = readdirSync(WASM_DIR, { withFileTypes: true });
    for (const dirent of dirents) {
      if (dirent.isDirectory()) {
        const cassetteName = dirent.name;
        const cassettePath = join(WASM_DIR, cassetteName);
        
        try {
          console.log(`Loading cassette from directory: ${cassetteName}`);
          
          // Import the cassette module
          const module = await import(join(cassettePath, "index.js"));
          
          // Try to load schemas if available
          let incomingSchemas: any[] = [];
          let outgoingSchemas: any[] = [];
          
          try {
            const schemaPath = join(cassettePath, "schemas.json");
            if (existsSync(schemaPath)) {
              const schemas = JSON.parse(readFileSync(schemaPath, 'utf-8'));
              incomingSchemas = schemas.incoming || [];
              outgoingSchemas = schemas.outgoing || [];
            }
          } catch (err) {
            console.warn(`Failed to load schemas for cassette ${cassetteName}:`, err);
          }
          
          cassettes.push({
            name: cassetteName,
            instance: module,
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
    }
  } catch (err) {
    console.error("Error loading cassettes:", err);
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
        const reqFunction = cassette.instance.req || 
                          (cassette.instance.SandwichsFavs && cassette.instance.SandwichsFavs.req);
        
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
    const closeFunction = cassette.instance.close || 
                        (cassette.instance.SandwichsFavs && cassette.instance.SandwichsFavs.close);
    
    // For cassettes using the close method
    if (closeFunction) {
      try {
        // Convert the message to a string as required by the close method
        const closeMessage = JSON.stringify(message);
        
        // Call the close method and parse the result
        const responseStr = closeFunction(closeMessage);
        console.log(`Close response from ${cassette.name}:`, responseStr);
        
        // Parse and handle response if needed
        try {
          const response = JSON.parse(responseStr);
          if (response.notice) {
            ws.send(JSON.stringify(response.notice));
          }
        } catch (parseError) {
          console.error(`Error parsing close response: ${parseError}`);
        }
      } catch (error) {
        console.error(`Error in cassette ${cassette.name} close method:`, error);
      }
    } 
    // For cassettes using the closeSubscription method
    else if (cassette.instance.closeSubscription) {
      try {
        cassette.instance.closeSubscription(subscriptionId);
      } catch (error) {
        console.error(`Error closing subscription in cassette ${cassette.name}:`, error);
      }
    }
  }
}