/**
 * Example showing how to integrate the cassette-loader with a Node.js WebSocket server
 * This version uses standard Node.js libraries instead of Bun-specific imports.
 */
import { join, dirname } from 'path';
import { readFileSync, readdirSync, existsSync } from 'fs';
import { fileURLToPath } from 'url';
import http from 'http';
import { WebSocketServer, WebSocket } from 'ws';
import { URL } from 'url';

// Import the cassette-loader
import { loadCassette, Cassette, isWebAssemblySupported, ENV_INFO } from '../src/index.js';

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

// Registry of loaded cassettes
const cassettes: Map<string, Cassette> = new Map();

// Map to store client subscriptions
const clientSubscriptions = new Map<WebSocket, Map<string, SubscriptionData>>();

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
  
  // Create HTTP server
  const server = http.createServer((req, res) => {
    const url = new URL(req.url || '/', `http://${req.headers.host}`);
    
    // Handle HTTP requests
    if (url.pathname === "/api/list-cassettes") {
      // Return list of loaded cassettes
      const cassetteList = Array.from(cassettes.values()).map(c => ({
        id: c.id,
        name: c.name,
        description: c.description,
        version: c.version
      }));
      
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(cassetteList));
      return;
    }
    
    // Default response for other requests
    res.writeHead(200, { 'Content-Type': 'text/plain' });
    res.end("Boombox WebSocket Server");
  });
  
  // Create WebSocket server
  const wss = new WebSocketServer({ server });
  
  // Handle WebSocket connections
  wss.on('connection', (ws: WebSocket) => {
    console.log("WebSocket client connected");
    
    // Initialize client subscriptions
    clientSubscriptions.set(ws, new Map());
    
    // Handle messages
    ws.addEventListener('message', async (event) => {
      try {
        const message = event.data.toString();
        // Parse the message as JSON
        const parsedEvent = JSON.parse(message);
        
        if (Array.isArray(parsedEvent) && parsedEvent.length >= 2) {
          const type = parsedEvent[0];
          
          if (type === "REQ") {
            // Handle subscription request
            const subId = parsedEvent[1];
            const filters = parsedEvent.slice(2);
            
            console.log(`Received REQ for subscription ${subId} with ${filters.length} filters`);
            
            // Store the subscription
            const subscriptions = clientSubscriptions.get(ws);
            if (subscriptions) {
              subscriptions.set(subId, { filters });
            }
            
            // Process the request through all cassettes
            const responses = await processRequest(message);
            
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
          } else if (type === "CLOSE") {
            // Handle closing a subscription
            const subId = parsedEvent[1];
            console.log(`Received CLOSE for subscription ${subId}`);
            
            // Remove the subscription
            const subscriptions = clientSubscriptions.get(ws);
            if (subscriptions) {
              subscriptions.delete(subId);
            }
            
            // Process close through all cassettes
            await processClose(message);
          }
        }
      } catch (error) {
        console.error("Error processing WebSocket message:", error);
      }
    });
    
    // Handle disconnections
    ws.addEventListener('close', () => {
      console.log("WebSocket client disconnected");
      // Clean up client subscriptions
      clientSubscriptions.delete(ws);
    });
  });
  
  // Start the server
  server.listen(PORT, () => {
    console.log(`Boombox server running on port ${PORT}`);
  });
}

// Start the server
startServer().catch(error => {
  console.error("Failed to start server:", error);
  process.exit(1);
}); 