use cassette_tools::{Cassette, CassetteSchema, RelayHandler, RelayResult, EventBasedHandler};
use wasm_bindgen::prelude::*;
use serde_json::{json, Value, from_str, to_string};

// Include the events JSON at build time
// The actual JSON content will be substituted during generation
const EVENTS_JSON: &str = r#"{{events_json}}"#;

// Metadata for this cassette
const CASSETTE_NAME: &str = "{{cassette_name}}";
const CASSETTE_DESC: &str = "{{cassette_description}}";
const CASSETTE_VERSION: &str = "{{cassette_version}}";
const CASSETTE_AUTHOR: &str = "{{cassette_author}}";
const CASSETTE_CREATED: &str = "{{cassette_created}}";

// Define our cassette struct
#[wasm_bindgen]
pub struct {{cassette_name}} {
    handler: EventBasedHandler,
}

// Implement constructor and core methods
#[wasm_bindgen]
impl {{cassette_name}} {
    // Constructor
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Initialize with the embedded events
        Self {
            handler: EventBasedHandler::new(include_str!("events.json")),
        }
    }

    // Implementation of standardized interface:

    /// Describe this cassette - returns metadata and schema information
    #[wasm_bindgen(js_name = "describe")]
    pub fn describe_wasm() -> String {
        // Create a comprehensive API description with metadata
        let description = json!({
            "metadata": {
                "name": "{{cassette_name}}",
                "description": "{{cassette_description}}",
                "version": "0.1.0",
                "author": "{{cassette_author}}",
                "created": "{{created_at}}",
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
    
    /// Get JSON schema for this cassette
    #[wasm_bindgen(js_name = "getSchema")]
    pub fn get_schema_wasm() -> String {
        <Self as Cassette>::get_schema_json()
    }
    
    /// Process a REQ request
    #[wasm_bindgen(js_name = "req")]
    pub fn req_wasm(request_json: &str) -> String {
        // Create an instance and handle the request
        let instance = Self::new();
        match instance.handler.handle_req(request_json) {
            Ok(response) => response,
            Err(err) => json!({
                "notice": ["NOTICE", err]
            }).to_string()
        }
    }
    
    /// Handle a CLOSE request
    #[wasm_bindgen(js_name = "close")]
    pub fn close_wasm(close_json: &str) -> String {
        // Create an instance and handle the close
        let instance = Self::new();
        match instance.handler.handle_close(close_json) {
            Ok(response) => response,
            Err(err) => json!({
                "notice": ["NOTICE", err]
            }).to_string()
        }
    }
    
    /// Memory management: Allocate a string
    #[wasm_bindgen(js_name = "allocString")]
    pub fn alloc_string(len: usize) -> *mut u8 {
        let mut buf = Vec::with_capacity(len);
        let ptr = buf.as_mut_ptr();
        std::mem::forget(buf);
        ptr
    }
    
    /// Memory management: Deallocate a string
    #[wasm_bindgen(js_name = "deallocString")]
    pub fn dealloc_string(ptr: *mut u8, len: usize) {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, 0, len);
        }
    }
}

// Implement Cassette trait
impl Cassette for {{cassette_name}} {
    fn describe() -> String {
        "{{cassette_description}}".to_string()
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