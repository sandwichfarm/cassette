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
}); 