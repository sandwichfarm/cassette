use cassette_tools::{Cassette, CassetteSchema, RelayHandler, EventBasedHandler, string_to_ptr, ptr_to_string, dealloc_string};
use serde_json::{json, to_string};

// Import embedded events
const EMBEDDED_EVENTS: &str = r#"{{events_json}}"#;

// Define cassette constants
const CASSETTE_NAME: &str = "{{cassette_name}}";
const CASSETTE_DESC: &str = "{{cassette_description}}";
const CASSETTE_VERSION: &str = "{{cassette_version}}";
const CASSETTE_AUTHOR: &str = "{{cassette_author}}";
const CASSETTE_CREATED: &str = "{{cassette_created}}";

// Define cassette struct
pub struct {{sanitized_name}} {
    handler: EventBasedHandler,
}

impl {{sanitized_name}} {
    pub fn new() -> Self {
        let handler = EventBasedHandler::new(EMBEDDED_EVENTS);
        Self { handler }
    }
}

impl Cassette for {{sanitized_name}} {
    fn describe() -> String {
        // Create a comprehensive API description with metadata
        let description = json!({
            "metadata": {
                "name": "{{cassette_name}}",
                "description": "{{cassette_description}}",
                "version": "0.1.0",
                "author": "{{cassette_author}}",
                "created": "{{cassette_created}}",
                "eventCount": {{event_count}} // Number of events in this cassette
            },
            "req": {
                // Request format schema
                "input": json!({
                    "type": "array",
                    "description": "NIP-01 REQ request"
                }),
                // Response format schema
                "output": json!({
                    "type": "object", 
                    "description": "Response containing events and EOSE message"
                })
            },
            "close": {
                // Close format schema
                "input": json!({
                    "type": "array",
                    "description": "NIP-01 CLOSE command"
                }),
                // Close response format schema
                "output": json!({
                    "type": "object",
                    "description": "Response confirming subscription closure"
                })
            }
        });
        
        to_string(&description).unwrap_or_else(|_| "{}".to_string())
    }
    
    fn get_schema() -> CassetteSchema {
        CassetteSchema {
            title: "{{cassette_name}}".to_string(),
            description: "{{cassette_description}}".to_string(),
            schema_type: "object".to_string(),
            properties: json!({
                "name": {
                    "type": "string",
                    "description": "The name of the cassette"
                },
                "version": {
                    "type": "string",
                    "description": "Version information"
                },
                "author": {
                    "type": "string",
                    "description": "Author of the cassette"
                },
                "created": {
                    "type": "string",
                    "description": "Creation timestamp"
                }
            }),
            required: vec!["name".to_string(), "version".to_string()],
            items: None,
        }
    }
}

// Export WebAssembly functions
#[no_mangle]
pub extern "C" fn describe() -> *mut u8 {
    let description = {{sanitized_name}}::describe();
    string_to_ptr(description)
}

#[no_mangle]
pub extern "C" fn get_schema() -> *mut u8 {
    let schema = {{sanitized_name}}::get_schema_json();
    string_to_ptr(schema)
}

#[no_mangle]
pub extern "C" fn req(ptr: *const u8, length: usize) -> *mut u8 {
    // Handle null pointer or invalid length case
    if ptr.is_null() || length == 0 {
        return string_to_ptr(json!(["NOTICE", "Error: Empty request received"]).to_string());
    }

    // Convert request from WebAssembly memory to a Rust string
    let request_str = ptr_to_string(ptr, length);
    
    // Add simple validation to ensure we got a request string
    if request_str.is_empty() {
        return string_to_ptr(json!(["NOTICE", "Error: Failed to read request from memory"]).to_string());
    }
    
    // Validate JSON structure before processing
    match serde_json::from_str::<serde_json::Value>(&request_str) {
        Ok(json_value) => {
            // Validate it's an array with at least 2 elements (REQ + subscription ID)
            if !json_value.is_array() {
                return string_to_ptr(json!(["NOTICE", "Error: Invalid request format: expected JSON array"]).to_string());
            }
            
            let json_array = json_value.as_array().unwrap();
            if json_array.len() < 2 {
                return string_to_ptr(json!(["NOTICE", "Error: Invalid REQ message: expected at least command and subscription ID"]).to_string());
            }
            
            // Check if it's a REQ command
            if let Some(cmd) = json_array[0].as_str() {
                if cmd != "REQ" {
                    return string_to_ptr(json!(["NOTICE", format!("Error: Unsupported command: {}", cmd)]).to_string());
                }
            } else {
                return string_to_ptr(json!(["NOTICE", "Error: Invalid command format: expected string"]).to_string());
            }
            
            // Check subscription ID
            if let Some(sub_id) = json_array[1].as_str() {
                if sub_id.trim().is_empty() {
                    return string_to_ptr(json!(["NOTICE", "Error: Invalid subscription ID: cannot be empty"]).to_string());
                }
            } else {
                return string_to_ptr(json!(["NOTICE", "Error: Invalid subscription ID: expected string"]).to_string());
            }
            
            // Check filters (if provided)
            if json_array.len() > 2 {
                for (i, filter) in json_array.iter().skip(2).enumerate() {
                    if !filter.is_object() {
                        return string_to_ptr(json!(["NOTICE", format!("Error: Invalid filter at position {}: expected object", i+2)]).to_string());
                    }
                    
                    // Check common filter fields
                    if let Some(obj) = filter.as_object() {
                        // Validate kinds is an array if present
                        if let Some(kinds) = obj.get("kinds") {
                            if !kinds.is_array() {
                                return string_to_ptr(json!(["NOTICE", "Error: Invalid filter: 'kinds' must be an array"]).to_string());
                            }
                        }
                        
                        // Validate authors is an array if present
                        if let Some(authors) = obj.get("authors") {
                            if !authors.is_array() {
                                return string_to_ptr(json!(["NOTICE", "Error: Invalid filter: 'authors' must be an array"]).to_string());
                            }
                        }
                        
                        // Validate other common filter fields as needed
                        if let Some(limit) = obj.get("limit") {
                            if !limit.is_number() {
                                return string_to_ptr(json!(["NOTICE", "Error: Invalid filter: 'limit' must be a number"]).to_string());
                            }
                        }
                    }
                }
            }
        },
        Err(e) => {
            return string_to_ptr(json!(["NOTICE", format!("Error: Invalid JSON: {}", e)]).to_string());
        }
    }
    
    // Process the request with improved error handling
    let instance = {{sanitized_name}}::new();
    let response = match instance.handler.handle_req(&request_str) {
        Ok(response) => response,
        Err(err) => {
            // Format error as a proper NOTICE message
            json!(["NOTICE", format!("Error: Request processing failed: {}", err)]).to_string()
        }
    };
    
    // Ensure we have a valid response
    if response.is_empty() {
        return string_to_ptr(json!(["NOTICE", "Error: Empty response from handler"]).to_string());
    }
    
    // Convert response to a pointer to be returned to WebAssembly
    string_to_ptr(response)
}

#[no_mangle]
pub extern "C" fn close(ptr: *const u8, length: usize) -> *mut u8 {
    // Handle null pointer or invalid length case
    if ptr.is_null() || length == 0 {
        return string_to_ptr(json!(["NOTICE", "Error: Empty close request received"]).to_string());
    }
    
    // Convert close command from WebAssembly memory to a Rust string
    let close_str = ptr_to_string(ptr, length);
    
    // Add simple validation to ensure we got a close string
    if close_str.is_empty() {
        return string_to_ptr(json!(["NOTICE", "Error: Failed to read close request from memory"]).to_string());
    }
    
    // Validate JSON structure before processing
    match serde_json::from_str::<serde_json::Value>(&close_str) {
        Ok(json_value) => {
            // Validate it's an array with at least 2 elements (CLOSE + subscription ID)
            if !json_value.is_array() {
                return string_to_ptr(json!(["NOTICE", "Error: Invalid close request format: expected JSON array"]).to_string());
            }
            
            let json_array = json_value.as_array().unwrap();
            if json_array.len() < 2 {
                return string_to_ptr(json!(["NOTICE", "Error: Invalid CLOSE message: expected command and subscription ID"]).to_string());
            }
            
            // Check if it's a CLOSE command
            if let Some(cmd) = json_array[0].as_str() {
                if cmd != "CLOSE" {
                    return string_to_ptr(json!(["NOTICE", format!("Error: Unsupported command: {}", cmd)]).to_string());
                }
            } else {
                return string_to_ptr(json!(["NOTICE", "Error: Invalid command format: expected string"]).to_string());
            }
            
            // Check subscription ID
            if let Some(sub_id) = json_array[1].as_str() {
                if sub_id.trim().is_empty() {
                    return string_to_ptr(json!(["NOTICE", "Error: Invalid subscription ID: cannot be empty"]).to_string());
                }
            } else {
                return string_to_ptr(json!(["NOTICE", "Error: Invalid subscription ID: expected string"]).to_string());
            }
            
            // Process the close command with proper error handling
            let instance = {{sanitized_name}}::new();
            let response = match instance.handler.handle_close(&close_str) {
                Ok(response) => response,
                Err(err) => {
                    // Format error as a proper NOTICE message
                    json!(["NOTICE", format!("Error: Close processing failed: {}", err)]).to_string()
                }
            };
            
            // Ensure we have a valid response
            if response.is_empty() {
                return string_to_ptr(json!(["NOTICE", "Error: Empty response from close handler"]).to_string());
            }
            
            // Convert response to a pointer to be returned to WebAssembly
            string_to_ptr(response)
        },
        Err(e) => {
            return string_to_ptr(json!(["NOTICE", format!("Error: Invalid JSON in close request: {}", e)]).to_string());
        }
    }
}

#[no_mangle]
pub extern "C" fn alloc_string(len: usize) -> *mut u8 {
    let mut buffer = Vec::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
} 