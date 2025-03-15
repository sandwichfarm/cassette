# Custom Cassette Example

This example demonstrates how to create a custom cassette using the `cassette-tools` library. The custom cassette implements the `RelayHandler` trait, allowing it to respond dynamically to relay requests.

## Features

- Implements the `RelayHandler` trait from `cassette-tools`
- Generates dynamic Nostr events based on incoming requests
- Demonstrates how to parse NIP-01 REQ messages
- Shows how to generate custom events based on filters

## How It Works

This cassette generates synthetic events when it receives a REQ message. It supports special functionality through the use of `#custom` tags in filters:

```json
["REQ", "subscription-id", {"#custom": ["echo", "random"]}]
```

Available custom tags:

- `echo`: Echoes back the original request as an event
- `random`: Generates a random event
- Any other tag: Creates a generic event with that tag

## Building the Cassette

To build this cassette into a WASM module:

```bash
# Navigate to the example directory
cd cli/examples/custom-cassette

# Build the WASM module
cargo build --target wasm32-unknown-unknown --release

# Generate JavaScript bindings
wasm-bindgen target/wasm32-unknown-unknown/release/custom_cassette.wasm --out-dir ./pkg --no-modules
```

Alternatively, you can use the `cassette dub` command from the Cassette CLI:

```bash
cassette dub -n "Custom Cassette" -d "A custom cassette that generates dynamic events" -a "Your Name" -o ./output
```

## Using in Boombox

To use this cassette in the Boombox server:

1. Build the WASM module as described above
2. Copy the generated `.wasm` and `.js` files to your Boombox server
3. Configure Boombox to load the cassette
4. Connect to the relay and send requests with `#custom` tags

## Custom Development

To create your own custom cassette:

1. Import `cassette-tools` in your Cargo.toml
2. Implement the `Cassette` and `RelayHandler` traits
3. Use `wasm-bindgen` to expose your implementation to JavaScript
4. Build with `wasm32-unknown-unknown` target

## Testing

You can test your custom cassette by:

1. Running it locally with a WebSocket client
2. Testing with the Boombox server
3. Using browser-based Nostr clients that connect to your relay

## License

This example is provided under the same license as the Cassette project. 