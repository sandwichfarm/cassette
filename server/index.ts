import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { readFile } from 'fs/promises';
import { join } from 'path';

const server = new McpServer({
  name: "example-server",
  version: "1.0.0"
});

interface WasmExports {
  increment: (value: number) => number;
}

// Load and instantiate the WASM module
async function loadWasmModule(): Promise<WasmExports> {
  const wasmPath = join(process.cwd(), 'plusone-wasm', 'incrementer.wasm');
  const wasmBuffer = await readFile(wasmPath);
  const wasmModule = await WebAssembly.instantiate(wasmBuffer);
  return wasmModule.instance.exports as unknown as WasmExports;
}

// Initialize WASM and start the server
async function main() {
  const wasmExports = await loadWasmModule();
  
  // Example usage of the increment function
  const result = wasmExports.increment(5); // Replace 5 with your input value
  console.log('WASM increment result:', result);

  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch(console.error);