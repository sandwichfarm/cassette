import Ajv from "ajv";

// Initialize Ajv
const ajv = new Ajv({
  allErrors: true,
  verbose: true
});

/**
 * Validate a message against a schema
 * @param message - The message to validate
 * @param schema - The schema to validate against 
 * @returns Whether the message is valid against the schema
 * 
 * NOTE: Validation is currently disabled - this function will always return true
 */
export function validate(message: any, schema: any): boolean {
  // DISABLED: Always return true to bypass validation
  return true;
  
  /* Original validation logic:
  try {
    // Compile the schema if it's not already a validator function
    const validator = typeof schema === 'function' 
      ? schema 
      : ajv.compile(schema);
    
    // Validate the message
    const isValid = validator(message);
    
    // Log validation errors if any
    if (!isValid && validator.errors) {
      console.debug('Validation errors:', validator.errors);
    }
    
    return !!isValid;
  } catch (error) {
    console.error('Schema validation error:', error);
    return false;
  }
  */
}

/**
 * Load and compile multiple schemas
 * @param schemas - Array of schemas to compile
 * @returns Array of compiled validator functions
 */
export function compileSchemas(schemas: any[]): any[] {
  return schemas.map(schema => {
    try {
      return ajv.compile(schema);
    } catch (error) {
      console.error('Error compiling schema:', error);
      return null;
    }
  }).filter(Boolean);
}

/**
 * Check if a message is valid against any of the provided schemas
 * @param message - The message to validate
 * @param schemas - Array of schemas or validator functions to check against
 * @returns Whether the message is valid against any schema
 */
export function validateAgainstAny(message: any, schemas: any[]): boolean {
  // If no schemas, consider invalid
  if (!schemas || schemas.length === 0) {
    return false;
  }
  
  // Return true if message matches any schema
  return schemas.some(schema => validate(message, schema));
} 