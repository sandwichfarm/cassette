// wasm_interface.rs
// Standardized WebAssembly interface for Cassette modules

// Define the standard interface that all WebAssembly cassettes must implement
// to ensure compatibility across the entire platform.
pub trait StandardCassetteInterface {
    /// Get a description of the cassette with metadata
    /// Returns: JSON string with cassette metadata and API description
    fn describe() -> *mut u8;
    
    /// Get the JSON schema for the cassette
    /// Returns: JSON schema string describing the cassette's data structure
    fn get_schema() -> *mut u8;
    
    /// Process a request and return a response
    /// 
    /// Args:
    ///   request_ptr: A pointer to the request string in WebAssembly memory
    ///   request_len: The length of the request string
    /// 
    /// Returns: Pointer to the response string in WebAssembly memory
    fn req(request_ptr: *const u8, request_len: usize) -> *mut u8;
    
    /// Close a subscription
    /// 
    /// Args:
    ///   close_ptr: A pointer to the close command string in WebAssembly memory
    ///   close_len: The length of the close command string
    /// 
    /// Returns: Pointer to the response string in WebAssembly memory
    fn close(close_ptr: *const u8, close_len: usize) -> *mut u8;
    
    /// Allocate memory for a string
    /// 
    /// Args:
    ///   len: The length of the string to allocate
    /// 
    /// Returns: Pointer to the allocated memory
    fn alloc_string(len: usize) -> *mut u8;
    
    /// Deallocate memory for a string
    /// 
    /// Args:
    ///   ptr: Pointer to the memory to deallocate
    ///   len: The length of the string that was allocated
    fn dealloc_string(ptr: *mut u8, len: usize);
}

/// Documentation on how to implement the standard interface
/// 
/// Guidelines for implementing the interface:
/// 
/// 1. Mark functions with #[no_mangle] to ensure they're exported correctly
/// 2. Use extern "C" to ensure the correct calling convention
/// 3. Implement all required functions with the exact signatures
/// 4. Use string_to_ptr and ptr_to_string for string conversion
/// 
/// Example implementation:
/// ```
/// use cassette_tools::{string_to_ptr, ptr_to_string, Cassette};
/// 
/// struct MyCassette;
/// impl Cassette for MyCassette { /* ... */ }
/// 
/// #[no_mangle]
/// pub extern "C" fn describe() -> *mut u8 {
///     string_to_ptr(MyCassette::describe())
/// }
/// 
/// // Other methods...
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