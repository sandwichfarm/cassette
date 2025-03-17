// wasm_interface.rs
// Standardized WebAssembly interface for Cassette modules

use wasm_bindgen::prelude::*;
use crate::CassetteSchema;

/// Define the standard interface that all WebAssembly cassettes must implement
/// to ensure compatibility across the entire platform.
pub trait StandardCassetteInterface {
    /// Get a description of the cassette with metadata
    /// Returns: JSON string with cassette metadata and API description
    fn describe() -> String;
    
    /// Get the JSON schema for the cassette
    /// Returns: JSON schema string describing the cassette's data structure
    fn get_schema() -> String;
    
    /// Process a request and return a response
    /// 
    /// Args:
    ///   request_json: A string containing the JSON request
    /// 
    /// Returns: JSON string response
    fn req(request_json: &str) -> String;
    
    /// Close a subscription
    /// 
    /// Args:
    ///   close_json: A string containing the JSON close command
    /// 
    /// Returns: JSON string response confirming closure
    fn close(close_json: &str) -> String;
    
    /// Allocate memory for a string (optional)
    /// 
    /// Args:
    ///   len: The length of the string to allocate
    /// 
    /// Returns: Pointer to the allocated memory
    fn alloc_string(len: usize) -> *mut u8;
    
    /// Deallocate memory for a string (optional)
    /// 
    /// Args:
    ///   ptr: Pointer to the memory to deallocate
    ///   len: The length of the string that was allocated
    fn dealloc_string(ptr: *mut u8, len: usize);
}

/// Helper macro to implement the standard cassette interface
/// 
/// This macro helps cassette implementers ensure they follow the standard
/// interface without having to repeat boilerplate code.
/// 
/// Usage:
/// ```
/// # use cassette_tools::impl_standard_cassette;
/// # use cassette_tools::Cassette;
/// # struct MyCassette;
/// # impl Cassette for MyCassette { /* ... */ }
/// 
/// impl_standard_cassette!(MyCassette);
/// ```
#[macro_export]
macro_rules! impl_standard_cassette {
    ($cassette_type:ty) => {
        #[wasm_bindgen]
        impl $cassette_type {
            #[wasm_bindgen(js_name = describe)]
            pub fn describe_wasm() -> String {
                <Self as $crate::Cassette>::describe()
            }
            
            #[wasm_bindgen(js_name = get_schema)]
            pub fn get_schema_wasm() -> String {
                <Self as $crate::Cassette>::get_schema_json()
            }
            
            #[wasm_bindgen(js_name = req)]
            pub fn req_wasm(request_json: &str) -> String {
                // Implementation should be provided by the user
                "".to_string()
            }
            
            #[wasm_bindgen(js_name = close)]
            pub fn close_wasm(close_json: &str) -> String {
                // Implementation should be provided by the user
                "".to_string()
            }
        }
    };
}

/// Documentation on how to implement the standard interface manually
/// 
/// If you prefer not to use the macro, you can implement the interface
/// manually following these guidelines:
/// 
/// 1. Use the js_name attribute to ensure consistent function names
/// 2. Implement all required functions with the exact signatures
/// 3. Return data in the expected format
/// 
/// Example implementation:
/// ```
/// # use wasm_bindgen::prelude::*;
/// # use cassette_tools::Cassette;
/// # struct MyCassette;
/// # impl Cassette for MyCassette { /* ... */ }
/// 
/// #[wasm_bindgen]
/// impl MyCassette {
///     #[wasm_bindgen(js_name = "describe")]
///     pub fn describe_wasm() -> String {
///         // Implementation
///         "{}".to_string()
///     }
///     
///     // Other methods...
/// }
/// ```
pub mod docs {
    /// Describes the expected format for the describe() function response
    pub const DESCRIBE_FORMAT: &str = r#"{
  "metadata": {
    "name": "Cassette Name",
    "description": "Description of what this cassette does",
    "version": "1.0.0",
    "author": "Author Name",
    "created": "ISO Date",
    "eventCount": 123
  },
  "req": {
    "input": { /* JSON Schema for request */ },
    "output": { /* JSON Schema for response */ }
  },
  "close": {
    "input": { /* JSON Schema for close request */ },
    "output": { /* JSON Schema for close response */ }
  }
}"#;

    /// Describes the expected format for req() function requests
    pub const REQ_REQUEST_FORMAT: &str = r#"["REQ", "subscription_id", {...filters...}]"#;
    
    /// Describes the expected format for req() function responses
    pub const REQ_RESPONSE_FORMAT: &str = r#"{
  "type": "event|notice",
  "message": ["EVENT|NOTICE", "subscription_id", {...event data...}]
}"#;

    /// Describes the expected format for close() function requests
    pub const CLOSE_REQUEST_FORMAT: &str = r#"["CLOSE", "subscription_id"]"#;
    
    /// Describes the expected format for close() function responses
    pub const CLOSE_RESPONSE_FORMAT: &str = r#"{
  "type": "notice",
  "message": ["NOTICE", "subscription_id closed"]
}"#;
} 