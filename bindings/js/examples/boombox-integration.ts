/**
 * Example showing how to integrate the cassette-loader with boombox server
 */
import { join, dirname } from 'path';
import { readFileSync, readdirSync, existsSync } from 'fs';
import { fileURLToPath } from 'url';
import { serve } from 'bun';
import type { ServerWebSocket } from 'bun';

// Import the cassette-loader
import { loadCassette, Cassette, isWebAssemblySupported, ENV_INFO } from '../dist/index.js';

// Get the current directory
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Port for WebSocket server
const PORT = parseInt(process.env.PORT || "3001");

// Directory containing WASM modules
const WASM_DIR = join(__dirname, "../../cassettes");

// Check if WebAssembly is supported
if (!isWebAssemblySupported()) {
  console.error("WebAssembly is not supported in this environment");
  process.exit(1);
}

// Log environment info
console.log("Environment:", ENV_INFO);

// Type for subscription data
interface SubscriptionData {
  filters: any[];
}

// Types for WebSocket data
interface WebSocketData {
  subscriptions: Map<string, SubscriptionData>;
}

// Registry of loaded cassettes
const cassettes: Map<string, Cassette> = new Map();

// Load all available cassettes
async function loadCassettes(): Promise<void> {
  console.log(`Loading cassettes from: ${WASM_DIR}`);
  
  // Check if the directory exists
  if (!existsSync(WASM_DIR)) {
    console.error(`Cassette directory ${WASM_DIR} does not exist`);
    return;
  }
  
  // Get all files and directories in WASM_DIR
  const fileEntries = readdirSync(WASM_DIR, { withFileTypes: true });
  
  // Check for .wasm files directly in the WASM_DIR
  const wasmFiles = fileEntries
    .filter(dirent => !dirent.isDirectory() && dirent.name.endsWith('.wasm'));
  
  // Load modules from .wasm files directly
  for (const wasmFile of wasmFiles) {
    try {
      console.log(`Loading cassette from WASM file: ${wasmFile.name}`);
      
      // Path to the WASM file
      const wasmPath = join(WASM_DIR, wasmFile.name);
      
      // Load the cassette using our cassette-loader
      const result = await loadCassette(wasmPath, wasmFile.name, {
        debug: true, // Enable debug logging
        memoryInitialSize: 16
      });
      
      if (result.success && result.cassette) {
        cassettes.set(result.cassette.id, result.cassette);
        console.log(`Successfully loaded cassette: ${result.cassette.name} (${result.cassette.id})`);
      } else {
        console.error(`Failed to load cassette ${wasmFile.name}: ${result.error}`);
      }
    } catch (error: any) {
      console.error(`Error loading cassette ${wasmFile.name}:`, error.message || error);
    }
  }
  
  console.log(`Loaded ${cassettes.size} cassettes`);
}

// Process a request through all cassettes
async function processRequest(request: string): Promise<string[]> {
  const responses: string[] = [];
  
  for (const [id, cassette] of cassettes) {
    try {
      const response = cassette.methods.req(request);
      responses.push(response);
    } catch (error: any) {
      console.error(`Error processing request with cassette ${id}:`, error.message || error);
    }
  }
  
  return responses;
}

// Close a subscription through all cassettes
async function processClose(closeStr: string): Promise<void> {
  for (const [id, cassette] of cassettes) {
    if (cassette.methods.close) {
      try {
        cassette.methods.close(closeStr);
      } catch (error: any) {
        console.error(`Error closing subscription with cassette ${id}:`, error.message || error);
      }
    }
  }
}

// Main server setup
async function startServer() {
  // Load cassettes first
  await loadCassettes();
  
  // Set up WebSocket server
  const server = serve({
    port: PORT,
    fetch(req, server) {
      const url = new URL(req.url);
      
      // Handle HTTP requests
      if (url.pathname === "/api/list-cassettes") {
        // Return list of loaded cassettes
        const cassetteList = Array.from(cassettes.values()).map(c => ({
          id: c.id,
          name: c.name,
          description: c.description,
          version: c.version
        }));
        return new Response(JSON.stringify(cassetteList), {
          headers: { "Content-Type": "application/json" }
        });
      }
      
      // Handle WebSocket upgrade
      if (server.upgrade(req)) {
        return; // Upgraded to WebSocket
      }
      
      // Default response for other requests
      return new Response("Boombox WebSocket Server", {
        headers: { "Content-Type": "text/plain" }
      });
    },
    websocket: {
      open(ws: ServerWebSocket<WebSocketData>) {
        // Initialize WebSocket data
        ws.data = {
          subscriptions: new Map()
        };
        console.log("WebSocket client connected");
      },
      message(ws: ServerWebSocket<WebSocketData>, message: string) {
        try {
          // Parse the message as JSON
          const event = JSON.parse(message);
          
          if (Array.isArray(event) && event.length >= 2) {
            const type = event[0];
            
            if (type === "REQ") {
              // Handle subscription request
              const subId = event[1];
              const filters = event.slice(2);
              
              console.log(`Received REQ for subscription ${subId} with ${filters.length} filters`);
              
              // Store the subscription
              ws.data.subscriptions.set(subId, { filters });
              
              // Process the request through all cassettes
              processRequest(message).then(responses => {
                // Send each response back to the client
                for (const response of responses) {
                  try {
                    const parsedResponse = JSON.parse(response);
                    if (Array.isArray(parsedResponse) && parsedResponse[0] === "EVENT") {
                      ws.send(response);
                    }
                  } catch (error) {
                    console.error("Error parsing cassette response:", error);
                  }
                }
              });
            } else if (type === "CLOSE") {
              // Handle closing a subscription
              const subId = event[1];
              console.log(`Received CLOSE for subscription ${subId}`);
              
              // Remove the subscription
              ws.data.subscriptions.delete(subId);
              
              // Process close through all cassettes
              processClose(message).catch(error => {
                console.error("Error closing subscription:", error);
              });
            }
          }
        } catch (error) {
          console.error("Error processing WebSocket message:", error);
        }
      },
      close(ws: ServerWebSocket<WebSocketData>) {
        console.log("WebSocket client disconnected");
      }
    }
  });
  
  console.log(`Boombox server running on port ${PORT}`);
}

// Start the server
startServer().catch(error => {
  console.error("Failed to start server:", error);
  process.exit(1);
}); 