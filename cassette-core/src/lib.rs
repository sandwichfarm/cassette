use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// JSON Schema for a Cassette
#[derive(Serialize, Deserialize)]
pub struct CassetteSchema {
    pub title: String,
    pub description: String,
    pub properties: Value,
    #[serde(rename = "type")]
    pub schema_type: String,
    pub required: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Value>,
}

impl Default for CassetteSchema {
    fn default() -> Self {
        Self {
            title: "Default Schema".to_string(),
            description: "Default schema description".to_string(),
            properties: json!({}),
            schema_type: "object".to_string(),
            required: vec![],
            items: None,
        }
    }
}

/// Trait that all cassettes must implement
pub trait Cassette {
    /// Returns a description of the cassette
    fn describe() -> String;
    
    /// Returns the JSON schema for the cassette
    fn get_schema() -> CassetteSchema;
    
    /// Generates the JSON schema string
    fn get_schema_json() -> String {
        let schema = Self::get_schema();
        serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Memory management utilities - these functions must be exported to WebAssembly
#[no_mangle]
pub extern "C" fn alloc_string(length: usize) -> *mut u8 {
    let mut buffer = Vec::with_capacity(length + 1);  // +1 for null terminator
    buffer.resize(length + 1, 0);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

#[no_mangle]
pub extern "C" fn dealloc_string(ptr: *mut u8, length: usize) {
    if ptr.is_null() {
        return;
    }
    
    unsafe {
        let _ = Vec::from_raw_parts(ptr, length, length + 1);
    }
}

// Helper functions for string operations
pub fn string_to_ptr(s: String) -> *mut u8 {
    let bytes = s.into_bytes();
    let length = bytes.len();
    let ptr = alloc_string(length);
    
    unsafe {
        let buffer = std::slice::from_raw_parts_mut(ptr, length);
        buffer.copy_from_slice(&bytes);
    }
    
    ptr
}

pub fn ptr_to_string(ptr: *const u8, length: usize) -> String {
    if ptr.is_null() || length == 0 {
        return String::new();
    }
    
    unsafe {
        let slice = std::slice::from_raw_parts(ptr, length);
        String::from_utf8_lossy(slice).to_string()
    }
}

/// Macro to implement standard WebAssembly exports for a cassette
#[macro_export]
macro_rules! implement_cassette_exports {
    ($cassette_type:ty) => {
        #[no_mangle]
        pub extern "C" fn describe() -> *mut u8 {
            let description = <$cassette_type>::describe();
            cassette_core::string_to_ptr(description)
        }
        
        #[no_mangle]
        pub extern "C" fn get_schema() -> *mut u8 {
            let schema = <$cassette_type>::get_schema_json();
            cassette_core::string_to_ptr(schema)
        }
        
        #[no_mangle]
        pub extern "C" fn get_description_size() -> usize {
            <$cassette_type>::describe().len()
        }
        
        #[no_mangle]
        pub extern "C" fn get_description_chunk(start: usize, max_length: usize) -> *mut u8 {
            let description = <$cassette_type>::describe();
            let end = std::cmp::min(start + max_length, description.len());
            
            if start >= description.len() {
                return cassette_core::string_to_ptr(String::new());
            }
            
            let chunk = description[start..end].to_string();
            cassette_core::string_to_ptr(chunk)
        }
        
        #[no_mangle]
        pub extern "C" fn get_schema_size() -> usize {
            <$cassette_type>::get_schema_json().len()
        }
        
        #[no_mangle]
        pub extern "C" fn get_schema_chunk(start: usize, max_length: usize) -> *mut u8 {
            let schema = <$cassette_type>::get_schema_json();
            let end = std::cmp::min(start + max_length, schema.len());
            
            if start >= schema.len() {
                return cassette_core::string_to_ptr(String::new());
            }
            
            let chunk = schema[start..end].to_string();
            cassette_core::string_to_ptr(chunk)
        }
        
        // Required functions for memory management
        #[no_mangle]
        pub extern "C" fn alloc_string(length: usize) -> *mut u8 {
            cassette_core::alloc_string(length)
        }
        
        #[no_mangle]
        pub extern "C" fn dealloc_string(ptr: *mut u8, length: usize) {
            cassette_core::dealloc_string(ptr, length);
        }
    }
} 