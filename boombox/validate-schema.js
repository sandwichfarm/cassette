// Import needed modules
import { SandwichsFavs } from "./wasm/sandwichs_favs.js";
import Ajv from "ajv";

// Get the schema from the cassette
try {
  console.log("Getting schema from SandwichsFavs...");
  const schemaStr = SandwichsFavs.get_schema();
  console.log("Schema:", schemaStr);
  
  // Parse the schema
  const schema = JSON.parse(schemaStr);
  console.log("Parsed schema:", schema);
  
  // Create an Ajv instance
  const ajv = new Ajv();
  
  // Compile the schema
  const validate = ajv.compile(schema);
  
  // Valid data
  const validData = {
    name: "BLT",
    ingredients: ["Bacon", "Lettuce", "Tomato", "Mayo"],
    rating: 5
  };
  
  // Invalid data (missing required field)
  const invalidData = {
    name: "Grilled Cheese"
    // Missing ingredients
  };
  
  // Test valid data
  const isValid = validate(validData);
  console.log("Valid data is valid:", isValid);
  
  // Test invalid data
  const isInvalid = validate(invalidData);
  console.log("Invalid data is valid:", isInvalid);
  if (!isInvalid) {
    console.log("Validation errors:", validate.errors);
  }
  
} catch (error) {
  console.error("Error:", error);
} 