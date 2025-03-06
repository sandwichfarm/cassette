import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { readFile } from 'fs/promises';
import { join } from 'path';
import { z } from 'zod';

const server = new McpServer({
  name: "example-server",
  version: "1.0.0"
});

interface WasmExports {
  increment: (value: number) => number;
}

async function loadWasmModule(): Promise<WasmExports> {
  const wasmPath = join(process.cwd(), '../plusone-wasm', 'incrementer.wasm');
  const wasmBuffer = await readFile(wasmPath);
  const wasmModule = await WebAssembly.instantiate(wasmBuffer);
  return wasmModule.instance.exports as unknown as WasmExports;
}

async function main() {
  const wasmExports = await loadWasmModule();
  const transport = new StdioServerTransport();
  server.tool("plusone",
    { a: z.number() },
    async ({ a }) => {
      const text = String(wasmExports.increment(a));
      return {
        content: [{ type: "text", text }]
      }
    }
  );
  await server.connect(transport);
}

main().catch(console.error);