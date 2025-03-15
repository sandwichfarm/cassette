# boombox

A test and validation suite for sandwichs-favs and cassette-tools.

## Installation

To install dependencies:

```bash
bun install
```

## Running

To run tests:

```bash
bun test
```

To run the application:

```bash
bun run index.ts
```

## WASM Builds

This project depends on WASM modules generated from the sandwichs-favs crate. We've added tools to help with building and updating the WASM bindings.

### Building WASM

To build the WASM module from sandwichs-favs:

```bash
bun run build:wasm
```

This script will:
1. Build the sandwichs-favs crate with the wasm32-unknown-unknown target
2. Copy the WASM file to the boombox/wasm directory
3. Attempt to generate JavaScript and TypeScript bindings using wasm-bindgen (if installed)

### Updating WASM Bindings Manually

If wasm-bindgen is not available or if you need to manually update the bindings:

```bash
bun run update:bindings
```

When prompted, enter the names of the methods to add, separated by commas. For example:

```
get_client_req_schema,get_relay_event_schema,get_relay_notice_schema
```

See [scripts/README.md](./scripts/README.md) for more details.

---

This project was created using `bun init` in bun v1.2.4. [Bun](https://bun.sh) is a fast all-in-one JavaScript runtime.
