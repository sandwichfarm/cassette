import { describe, test, expect } from "bun:test";

describe("Schema Validation", () => {
  test("SandwichsFavs exports get_schema function", async () => {
    // Initialize the wasm module
    const { SandwichsFavs } = await import("./wasm/sandwichs_favs.js");
    
    // Check if get_schema exists
    expect(typeof SandwichsFavs.get_schema).toBe("function");
    
    // Get the schema from the cassette
    const schemaStr = SandwichsFavs.get_schema();
    console.log("Schema:", schemaStr);
    
    // Verify it's valid JSON
    const schema = JSON.parse(schemaStr);
    expect(schema).toBeTruthy();
    expect(schema.title).toBe("Sandwich's Favorite Notes");
  });
  
  // These tests require rebuilding the WASM module with `npm run build:wasm`
  // If the functions don't exist, run the binding update tool with `npm run update:bindings`
  test("NIP-01 Client REQ schema is valid", async () => {
    // Initialize the wasm module
    const { SandwichsFavs } = await import("./wasm/sandwichs_favs.js");
    
    // Check if the function exists
    expect(typeof SandwichsFavs.get_client_req_schema).toBe("function");
    
    // Get the schema
    const schemaStr = SandwichsFavs.get_client_req_schema();
    console.log("Client REQ Schema:", schemaStr);
    
    // Verify it's valid JSON
    const schema = JSON.parse(schemaStr);
    expect(schema).toBeTruthy();
    expect(schema.title).toBe("Client Request");
    expect(schema.type).toBe("array");
    
    // Verify the schema has items array with required elements
    expect(schema.items).toBeTruthy();
    expect(Array.isArray(schema.items)).toBe(true);
    expect(schema.items[0].const).toBe("REQ");
    expect(schema.items[1].type).toBe("string");
  });
  
  test("NIP-01 Relay EVENT schema is valid", async () => {
    // Initialize the wasm module
    const { SandwichsFavs } = await import("./wasm/sandwichs_favs.js");
    
    // Check if the function exists
    expect(typeof SandwichsFavs.get_relay_event_schema).toBe("function");
    
    // Get the schema
    const schemaStr = SandwichsFavs.get_relay_event_schema();
    console.log("Relay EVENT Schema:", schemaStr);
    
    // Verify it's valid JSON
    const schema = JSON.parse(schemaStr);
    expect(schema).toBeTruthy();
    expect(schema.title).toBe("Relay Event");
    expect(schema.type).toBe("array");
    
    // Verify the schema has items array with required elements
    expect(schema.items).toBeTruthy();
    expect(Array.isArray(schema.items)).toBe(true);
    expect(schema.items[0].const).toBe("EVENT");
    expect(schema.items[1].type).toBe("string");
    
    // Verify the event object structure
    const eventSchema = schema.items[2];
    expect(eventSchema.type).toBe("object");
    expect(eventSchema.properties).toBeTruthy();
    expect(eventSchema.properties.content).toBeTruthy();
    expect(eventSchema.properties.id).toBeTruthy();
    expect(eventSchema.properties.pubkey).toBeTruthy();
    expect(eventSchema.properties.created_at).toBeTruthy();
    expect(eventSchema.properties.kind).toBeTruthy();
    expect(eventSchema.properties.tags).toBeTruthy();
    expect(eventSchema.properties.sig).toBeTruthy();
  });
  
  test("NIP-01 Relay NOTICE schema is valid", async () => {
    // Initialize the wasm module
    const { SandwichsFavs } = await import("./wasm/sandwichs_favs.js");
    
    // Check if the function exists
    expect(typeof SandwichsFavs.get_relay_notice_schema).toBe("function");
    
    // Get the schema
    const schemaStr = SandwichsFavs.get_relay_notice_schema();
    console.log("Relay NOTICE Schema:", schemaStr);
    
    // Verify it's valid JSON
    const schema = JSON.parse(schemaStr);
    expect(schema).toBeTruthy();
    expect(schema.title).toBe("Relay Notice");
    expect(schema.type).toBe("array");
    
    // Verify the schema has items array with required elements
    expect(schema.items).toBeTruthy();
    expect(Array.isArray(schema.items)).toBe(true);
    expect(schema.items[0].const).toBe("NOTICE");
    expect(schema.items[1].type).toBe("string");
  });
}); 