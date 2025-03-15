import { serve } from "bun";
import type { ServerWebSocket } from "bun";

const BOOMBOX_WS_URL = "ws://localhost:3001"; // Default target WebSocket URL

// Configuration from environment variables or defaults
const PORT = parseInt(process.env.PORT || "3000");
const TARGET_URL = process.env.TARGET_URL || BOOMBOX_WS_URL;

console.log(`🔌 Nostr Proxy starting on port ${PORT}`);
console.log(`🎯 Forwarding to Boombox at ${TARGET_URL}`);

// Connection map to track client connections and their corresponding boombox connections
const connections = new Map<ServerWebSocket<unknown>, WebSocket>();

serve({
  port: PORT,
  fetch(req, server) {
    // Handle WebSocket upgrade
    if (server.upgrade(req)) {
      return; // Upgraded to WebSocket
    }
    
    // Return simple status for HTTP requests
    return new Response("Nostr Relay Proxy - Connect via WebSocket", {
      status: 200,
      headers: { "Content-Type": "text/plain" }
    });
  },
  websocket: {
    open(ws: ServerWebSocket<unknown>) {
      console.log("Client connected");
      
      // Create a connection to boombox
      const boomboxWs = new WebSocket(TARGET_URL);
      
      // Store the association between client and boombox connections
      connections.set(ws, boomboxWs);
      
      // Forward messages from boombox back to the client
      boomboxWs.onmessage = (event: MessageEvent) => {
        if (ws.readyState === WebSocket.OPEN) {
          try {
            ws.send(event.data);
          } catch (error) {
            console.error("Error forwarding message to client:", error);
          }
        }
      };
      
      boomboxWs.onclose = () => {
        console.log("Boombox connection closed");
        // If boombox connection closes, close client connection too
        if (ws.readyState === WebSocket.OPEN) {
          ws.close();
        }
      };
      
      boomboxWs.onerror = (error: Event) => {
        console.error("Boombox connection error:", error);
      };
    },
    message(ws: ServerWebSocket<unknown>, message: string | Uint8Array) {
      // Get the associated boombox connection
      const boomboxWs = connections.get(ws);
      if (!boomboxWs || boomboxWs.readyState !== WebSocket.OPEN) {
        console.error("No active boombox connection found");
        return;
      }
      
      try {
        // Forward the message to boombox
        boomboxWs.send(message);
      } catch (error) {
        console.error("Error forwarding message to boombox:", error);
      }
    },
    close(ws: ServerWebSocket<unknown>) {
      console.log("Client disconnected");
      
      // Close and cleanup the associated boombox connection
      const boomboxWs = connections.get(ws);
      if (boomboxWs) {
        if (boomboxWs.readyState === WebSocket.OPEN) {
          boomboxWs.close();
        }
        connections.delete(ws);
      }
    },
    drain(ws: ServerWebSocket<unknown>) {
      // Handle backpressure if needed
    }
  }
});

console.log(`🚀 Nostr proxy server running at ws://localhost:${PORT}`);