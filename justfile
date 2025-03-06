bridge:
    cd counter-mcp && bunx @dvmcp/bridge

test:
    cd counter-mcp && bun run testdvm.ts 

build:
    cd counter-wasm && zig build