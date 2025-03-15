import { describe, test, expect } from "bun:test";

describe("SandwichsFavs Cassette", () => {
  test("SandwichsFavs can be loaded and describe method works", async () => {
    // Initialize the wasm module
    const wasm = await import("./wasm/sandwichs_favs.js");
    await wasm.default();
    
    const description = wasm.SandwichsFavs.describe();
    console.log("Cassette description:", description);
    expect(description).toContain("Sandwich's Favorite Notes");
    expect(description).toContain("Result: 2");
  });

  test("SandwichsFavs calculate method returns 2", async () => {
    // Initialize the wasm module
    const wasm = await import("./wasm/sandwichs_favs.js");
    await wasm.default();
    
    const result = wasm.SandwichsFavs.calculate();
    expect(result).toBe(2n);
  });
}); 