use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// Constants for string handling
const MSGB_SIGNATURE: [u8; 4] = [0x4D, 0x53, 0x47, 0x42]; // "MSGB"
const MAX_STRING_LENGTH: usize = 10_000_000; // 10MB safety limit

/// Allocate a buffer of specified size
/// This function is called from JavaScript to allocate memory in the WebAssembly module
/// for storing strings or other data.
#[no_mangle]
pub fn alloc_buffer(size: usize) -> *mut u8 {
    // Safety check
    if size > MAX_STRING_LENGTH {
        return std::ptr::null_mut();
    }
    
    // Create a new buffer with exact capacity
    let mut buffer = Vec::with_capacity(size);
    // Set the length to initialize memory
    unsafe { buffer.set_len(size); }
    
    // Get raw pointer
    let ptr = buffer.as_mut_ptr();
    
    // Prevent Rust from freeing the memory when this function returns
    std::mem::forget(buffer);
    
    ptr
}

/// Convert a Rust string to a pointer that can be returned to WebAssembly
/// This function converts a string to a pointer that can be returned to JavaScript.
/// Format: [4-byte signature][4-byte length][string bytes]
/// 
/// The signature helps identify this string format for debugging and validation.
#[no_mangle]
pub fn string_to_ptr(s: String) -> *mut u8 {
    // Safety check for maximum size
    if s.len() > MAX_STRING_LENGTH {
        return std::ptr::null_mut();
    }
    
    // Get the bytes from the string
    let bytes = s.into_bytes();
    let bytes_len = bytes.len();
    
    // Calculate total size needed: signature + length + data
    let total_size = 4 + 4 + bytes_len;
    
    // Allocate a buffer with exact capacity
    let mut buffer = Vec::with_capacity(total_size);
    
    // Initialize memory to zeros
    buffer.resize(total_size, 0);
    
    // Write the "MSGB" signature
    buffer[0..4].copy_from_slice(&MSGB_SIGNATURE);
    
    // Write the length as little-endian bytes
    let len_bytes = (bytes_len as u32).to_le_bytes();
    buffer[4..8].copy_from_slice(&len_bytes);
    
    // Copy the string bytes
    buffer[8..8+bytes_len].copy_from_slice(&bytes);
    
    // Get raw pointer to the buffer
    let ptr = buffer.as_mut_ptr();
    
    // Prevent Rust from freeing the memory
    std::mem::forget(buffer);
    
    ptr
}

/// Convert a buffer in WebAssembly memory to a Rust string
/// 
/// Args:
///   ptr: Pointer to the memory location
///   len: Length of the data (including any prefix if present)
///
/// Returns: The string value
#[no_mangle]
pub fn ptr_to_string(ptr: *const u8, len: usize) -> String {
    // Safety checks
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    
    // Create a safe slice from the pointer
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    
    // Check if this has an MSGB signature
    if len >= 8 && 
       slice[0] == MSGB_SIGNATURE[0] &&
       slice[1] == MSGB_SIGNATURE[1] &&
       slice[2] == MSGB_SIGNATURE[2] &&
       slice[3] == MSGB_SIGNATURE[3] {
        // Read the embedded length
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&slice[4..8]);
        let string_len = u32::from_le_bytes(length_bytes) as usize;
        
        // Safety check for reasonable string length
        if string_len > MAX_STRING_LENGTH || string_len + 8 > len {
            return String::new();
        }
        
        // Extract just the string bytes (after signature and length)
        return String::from_utf8_lossy(&slice[8..8+string_len]).to_string();
    }
    
    // No signature, treat the entire buffer as a string
    String::from_utf8_lossy(slice).to_string()
}

/// Deallocate memory previously allocated with string_to_ptr or alloc_buffer
/// 
/// Args:
///   ptr: Pointer to the memory location
///   len: Length of the allocation
#[no_mangle]
pub fn dealloc_buffer(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    
    unsafe {
        // Recreate the Vec and let it drop
        Vec::from_raw_parts(ptr, len, len);
    }
}

/// Deallocate a string that was allocated with string_to_ptr
/// This function is called from JavaScript to free memory allocated by string_to_ptr
/// 
/// Args:
///   ptr: Pointer to the memory location
///   len: Length hint (can be 0, in which case we try to determine it)
#[no_mangle]
pub extern "C" fn dealloc_string(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    
    // If len is 0, try to determine the actual allocation size
    let actual_len = if len == 0 {
        get_allocation_size(ptr)
    } else {
        len
    };
    
    // If we still don't have a valid length, we can't safely deallocate
    if actual_len == 0 {
        return;
    }
    
    unsafe {
        // Deallocate the entire buffer
        let _ = Vec::from_raw_parts(ptr, actual_len, actual_len);
    }
}

/// Get the length of a string at a given pointer
/// This works with both MSGB format and raw strings.
/// 
/// Args:
///   ptr: Pointer to the memory location
///
/// Returns: The length of the string data (not including any prefix)
#[no_mangle]
pub fn get_string_len(ptr: *const u8) -> usize {
    if ptr.is_null() {
        return 0;
    }
    
    // Check if this has our signature
    let has_signature = unsafe {
        let bytes = std::slice::from_raw_parts(ptr, 4);
        bytes.len() == 4 && 
        bytes[0] == MSGB_SIGNATURE[0] && 
        bytes[1] == MSGB_SIGNATURE[1] && 
        bytes[2] == MSGB_SIGNATURE[2] && 
        bytes[3] == MSGB_SIGNATURE[3]
    };
    
    if has_signature {
        // Read the embedded length
        unsafe {
            let mut length_bytes = [0u8; 4];
            let bytes = std::slice::from_raw_parts(ptr.add(4), 4);
            length_bytes.copy_from_slice(bytes);
            let length = u32::from_le_bytes(length_bytes) as usize;
            
            // Validate the length
            if length > MAX_STRING_LENGTH {
                return 0;
            }
            length
        }
    } else {
        // If no signature, we can't determine length safely
        0
    }
}

/// Get a pointer to the actual string data, skipping any prefix
/// 
/// Args:
///   ptr: Pointer to the memory location
///
/// Returns: Pointer to the start of the string data
#[no_mangle]
pub fn get_string_ptr(ptr: *const u8) -> *const u8 {
    if ptr.is_null() {
        return ptr;
    }
    
    // Check if this has our signature
    let has_signature = unsafe {
        let bytes = std::slice::from_raw_parts(ptr, 4);
        bytes.len() == 4 && 
        bytes[0] == MSGB_SIGNATURE[0] && 
        bytes[1] == MSGB_SIGNATURE[1] && 
        bytes[2] == MSGB_SIGNATURE[2] && 
        bytes[3] == MSGB_SIGNATURE[3]
    };
    
    if has_signature {
        // Skip signature and length to get to the string data
        unsafe { ptr.add(8) }
    } else {
        // Return the pointer as is
        ptr
    }
}

/// Get the total allocation size for a string pointer
/// This includes the MSGB header (8 bytes) plus the string data length
/// 
/// Args:
///   ptr: Pointer to the memory location
///
/// Returns: Total allocation size, or 0 if unknown
#[no_mangle]
pub extern "C" fn get_allocation_size(ptr: *const u8) -> usize {
    if ptr.is_null() {
        return 0;
    }
    
    // Check if this has our MSGB signature
    let has_signature = unsafe {
        if ptr as usize + 4 > usize::MAX {
            return 0;
        }
        let bytes = std::slice::from_raw_parts(ptr, 4);
        bytes.len() == 4 && 
        bytes[0] == MSGB_SIGNATURE[0] && 
        bytes[1] == MSGB_SIGNATURE[1] && 
        bytes[2] == MSGB_SIGNATURE[2] && 
        bytes[3] == MSGB_SIGNATURE[3]
    };
    
    if has_signature {
        // Read the embedded length from MSGB header
        unsafe {
            let mut length_bytes = [0u8; 4];
            let bytes = std::slice::from_raw_parts(ptr.add(4), 4);
            length_bytes.copy_from_slice(bytes);
            let string_length = u32::from_le_bytes(length_bytes) as usize;
            
            // Validate the length
            if string_length > MAX_STRING_LENGTH {
                return 0;
            }
            
            // Return total size: signature (4) + length (4) + data
            8 + string_length
        }
    } else {
        // Without MSGB signature, we can't determine the allocation size
        0
    }
}

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

/// Include the standardized WebAssembly interface module
pub mod wasm_interface;

/// Modular NIP support
pub mod nips;

// The macro is automatically exported at crate root due to #[macro_export]
// No need to re-export it

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

/// Result type for relay operations
pub type RelayResult = Result<String, String>;

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

/// Trait for handling relay operations
pub trait RelayHandler {
    /// Handle a JSON-formatted relay message
    fn handle_message(&self, message: &str) -> RelayResult {
        let parsed: Result<Value, _> = serde_json::from_str(message);
        
        match parsed {
            Ok(value) => {
                if let Some(arr) = value.as_array() {
                    if arr.len() > 0 {
                        if let Some(cmd) = arr[0].as_str() {
                            match cmd {
                                "REQ" => self.handle_req(message),
                                "CLOSE" => self.handle_close(message),
                                _ => Err(format!("Unsupported command: {}", cmd)),
                            }
                        } else {
                            Err("Invalid message format: first element must be a string".to_string())
                        }
                    } else {
                        Err("Empty message array".to_string())
                    }
                } else {
                    Err("Message must be a JSON array".to_string())
                }
            },
            Err(e) => Err(format!("Failed to parse JSON: {}", e)),
        }
    }
    
    /// Handle a REQ command
    fn handle_req(&self, req_json: &str) -> RelayResult;
    
    /// Handle a CLOSE command
    fn handle_close(&self, close_json: &str) -> RelayResult {
        // Default implementation for CLOSE
        let parsed: Result<Value, _> = serde_json::from_str(close_json);
        
        match parsed {
            Ok(value) => {
                if let Some(arr) = value.as_array() {
                    if arr.len() >= 2 && arr[0].as_str() == Some("CLOSE") {
                        let subscription_id = arr[1].as_str().unwrap_or("");
                        Ok(json!({
                            "notice": ["NOTICE", format!("Subscription {} closed", subscription_id)]
                        }).to_string())
                    } else {
                        Err("Invalid CLOSE message format".to_string())
                    }
                } else {
                    Err("CLOSE message must be a JSON array".to_string())
                }
            },
            Err(e) => Err(format!("Failed to parse CLOSE JSON: {}", e)),
        }
    }
}

/// EventBasedHandler implements RelayHandler using a static list of events
pub struct EventBasedHandler {
    events_json: String,
}

impl EventBasedHandler {
    /// Create a new EventBasedHandler with JSON events
    pub fn new(events_json: &str) -> Self {
        Self {
            events_json: events_json.to_string(),
        }
    }
}

impl RelayHandler for EventBasedHandler {
    fn handle_req(&self, req_json: &str) -> RelayResult {
        // Validate that the request isn't empty
        if req_json.trim().is_empty() {
            return Err("Empty request received".to_string());
        }

        // Parse the incoming request JSON with detailed error handling
        let req_value: Result<Value, serde_json::Error> = serde_json::from_str(req_json);
        
        match req_value {
            Ok(req) => {
                // Check if this is a valid REQ format according to NIP-01
                if let Some(array) = req.as_array() {
                    // Validate REQ message structure
                    if array.len() < 2 {
                        return Err(format!("REQ message too short. Expected at least 2 elements, got {}", array.len()).to_string());
                    }
                    
                    // Check if first element is "REQ"
                    if array[0].as_str() != Some("REQ") {
                        return Err(format!("Invalid message type. Expected 'REQ', got '{}'", 
                            array[0].as_str().unwrap_or("non-string value")).to_string());
                    }
                    
                    // Get subscription ID (second element) with validation
                    let subscription_id = match array[1].as_str() {
                        Some(id) if !id.trim().is_empty() => id,
                        Some(_) => return Err("Subscription ID cannot be empty or whitespace".to_string()),
                        None => return Err("Subscription ID must be a string".to_string())
                    };
                    
                    // Log the subscription ID for debugging
                    println!("Processing request for subscription: {}", subscription_id);
                    
                    // Validate there's at least one filter if filters are expected
                    if array.len() < 3 {
                        // No filters provided, which is valid - we'll return all events
                        // Process without filters
                        let events: Result<Vec<Value>, _> = serde_json::from_str(&self.events_json);
                    
                        if let Ok(events) = events {
                            // Convert to EVENT messages without filtering
                            let events: Vec<Value> = events.into_iter()
                                .map(|event| {
                                    json!([
                                        "EVENT",
                                        subscription_id,
                                        event
                                    ])
                                })
                                .collect();
                            
                            // Log the number of events being returned
                            println!("Returning {} events for subscription {}", events.len(), subscription_id);
                            
                            return Ok(json!({
                                "events": events,
                                "eose": ["EOSE", subscription_id]
                            }).to_string());
                        } else {
                            return Err("Error parsing embedded events JSON".to_string());
                        }
                    }
                    
                    // Try to get filters from the request
                    let mut kind_filter: Option<Vec<i64>> = None;
                    let mut author_filter: Option<Vec<String>> = None;
                    let mut tags_filter: Option<Vec<(String, String)>> = None;
                    let mut and_tags_filter: Option<Vec<(String, Vec<String>)>> = None;
                    let mut since_filter: Option<i64> = None;
                    let mut until_filter: Option<i64> = None;
                    let mut limit_filter: Option<usize> = None;
                    let mut ids_filter: Option<Vec<String>> = None;
                    
                    // Process filters (starting from index 2)
                    for i in 2..array.len() {
                        if let Some(filter) = array[i].as_object() {
                            // Look for kind filter
                            if let Some(kinds) = filter.get("kinds") {
                                if let Some(kinds_array) = kinds.as_array() {
                                    kind_filter = Some(
                                        kinds_array.iter()
                                            .filter_map(|k| k.as_i64())
                                            .collect()
                                    );
                                } else {
                                    return Err("'kinds' filter must be an array".to_string());
                                }
                            }
                            
                            // Look for author filter
                            if let Some(authors) = filter.get("authors") {
                                if let Some(authors_array) = authors.as_array() {
                                    author_filter = Some(
                                        authors_array.iter()
                                            .filter_map(|a| a.as_str().map(String::from))
                                            .collect()
                                    );
                                } else {
                                    return Err("'authors' filter must be an array".to_string());
                                }
                            }
                            
                            // Look for ids filter (event IDs)
                            if let Some(ids) = filter.get("ids") {
                                if let Some(ids_array) = ids.as_array() {
                                    ids_filter = Some(
                                        ids_array.iter()
                                            .filter_map(|id| id.as_str().map(String::from))
                                            .collect()
                                    );
                                } else {
                                    return Err("'ids' filter must be an array".to_string());
                                }
                            }
                            
                            // Look for since filter
                            if let Some(since_val) = filter.get("since") {
                                since_filter = since_val.as_i64();
                                if since_filter.is_none() {
                                    return Err("'since' filter must be an integer timestamp".to_string());
                                }
                            }
                            
                            // Look for until filter
                            if let Some(until_val) = filter.get("until") {
                                until_filter = until_val.as_i64();
                                if until_filter.is_none() {
                                    return Err("'until' filter must be an integer timestamp".to_string());
                                }
                            }
                            
                            // Look for limit filter
                            if let Some(limit_val) = filter.get("limit") {
                                limit_filter = limit_val.as_u64().map(|l| l as usize);
                                if limit_filter.is_none() {
                                    return Err("'limit' filter must be a positive integer".to_string());
                                }
                            }
                            
                            // Implementing "tags" correctly based on NIP-01
                            let tag_filters: Vec<(String, String)> = filter.keys()
                                .filter(|k| k.starts_with('#'))
                                .filter_map(|k| {
                                    let tag_key = k.trim_start_matches('#');
                                    if let Some(values) = filter.get(k).and_then(|v| v.as_array()) {
                                        Some(values.iter()
                                            .filter_map(|v| v.as_str().map(|value| (tag_key.to_string(), value.to_string())))
                                            .collect::<Vec<_>>())
                                    } else {
                                        None
                                    }
                                })
                                .flatten()
                                .collect();
                            
                            if !tag_filters.is_empty() {
                                tags_filter = Some(tag_filters);
                            }
                            
                            // AND tag filtering (NIP-119)
                            let and_tag_filters: Vec<(String, Vec<String>)> = filter.keys()
                                .filter(|k| k.starts_with('&'))
                                .filter_map(|k| {
                                    let tag_key = k.trim_start_matches('&');
                                    if let Some(values) = filter.get(k).and_then(|v| v.as_array()) {
                                        let tag_values: Vec<String> = values.iter()
                                            .filter_map(|v| v.as_str().map(String::from))
                                            .collect();
                                        
                                        if !tag_values.is_empty() {
                                            Some((tag_key.to_string(), tag_values))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            
                            if !and_tag_filters.is_empty() {
                                and_tags_filter = Some(and_tag_filters);
                            }
                        } else {
                            return Err(format!("Filter at position {} must be an object", i).to_string());
                        }
                    }
                    
                    // Parse the events embedded at build time
                    let events: Result<Vec<Value>, _> = serde_json::from_str(&self.events_json);
                    
                    if let Ok(events) = events {
                        // Apply all filters in sequence
                        let mut filtered_events = events;
                        
                        // Filter by ids if specified
                        if let Some(ids) = ids_filter {
                            filtered_events = filtered_events.into_iter()
                                .filter(|event| {
                                    if let Some(id) = event.get("id").and_then(|id| id.as_str()) {
                                        ids.contains(&id.to_string())
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by kinds if specified
                        if let Some(kinds) = kind_filter {
                            filtered_events = filtered_events.into_iter()
                                .filter(|event| {
                                    if let Some(kind) = event.get("kind").and_then(|k| k.as_i64()) {
                                        kinds.contains(&kind)
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by authors if specified
                        if let Some(authors) = author_filter {
                            filtered_events = filtered_events.into_iter()
                                .filter(|event| {
                                    if let Some(pubkey) = event.get("pubkey").and_then(|p| p.as_str()) {
                                        authors.contains(&pubkey.to_string())
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by since timestamp
                        if let Some(since) = since_filter {
                            filtered_events = filtered_events.into_iter()
                                .filter(|event| {
                                    if let Some(created_at) = event.get("created_at").and_then(|t| t.as_i64()) {
                                        created_at >= since
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by until timestamp
                        if let Some(until) = until_filter {
                            filtered_events = filtered_events.into_iter()
                                .filter(|event| {
                                    if let Some(created_at) = event.get("created_at").and_then(|t| t.as_i64()) {
                                        created_at <= until
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by tags if specified
                        if let Some(tag_filters) = tags_filter {
                            filtered_events = filtered_events.into_iter()
                                .filter(|event| {
                                    if let Some(tags) = event.get("tags").and_then(|t| t.as_array()) {
                                        tag_filters.iter().all(|(key, value)| {
                                            tags.iter().any(|tag| {
                                                if let Some(tag_array) = tag.as_array() {
                                                    if tag_array.len() >= 2 {
                                                        let tag_type = tag_array[0].as_str().unwrap_or("");
                                                        let tag_value = tag_array[1].as_str().unwrap_or("");
                                                        tag_type == key && tag_value == value
                                                    } else {
                                                        false
                                                    }
                                                } else {
                                                    false
                                                }
                                            })
                                        })
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by AND tags if specified
                        if let Some(and_tag_filters) = and_tags_filter {
                            filtered_events = filtered_events.into_iter()
                                .filter(|event| {
                                    if let Some(tags) = event.get("tags").and_then(|t| t.as_array()) {
                                        and_tag_filters.iter().all(|(key, values)| {
                                            values.iter().all(|value| {
                                                tags.iter().any(|tag| {
                                                    if let Some(tag_array) = tag.as_array() {
                                                        if tag_array.len() >= 2 {
                                                            let tag_type = tag_array[0].as_str().unwrap_or("");
                                                            let tag_value = tag_array[1].as_str().unwrap_or("");
                                                            tag_type == key && tag_value == value
                                                        } else {
                                                            false
                                                        }
                                                    } else {
                                                        false
                                                    }
                                                })
                                            })
                                        })
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Apply limit filter if specified
                        if let Some(limit) = limit_filter {
                            if limit < filtered_events.len() {
                                filtered_events = filtered_events.into_iter().take(limit).collect();
                            }
                        }
                        
                        // Convert filtered events to EVENT messages
                        let events: Vec<Value> = filtered_events.into_iter()
                            .map(|event| {
                                json!([
                                    "EVENT",
                                    subscription_id,
                                    event
                                ])
                            })
                            .collect();
                        
                        // Log the number of filtered events
                        println!("Returning {} filtered events for subscription {}", events.len(), subscription_id);
                        
                        return Ok(json!({
                            "events": events,
                            "eose": ["EOSE", subscription_id]
                        }).to_string());
                    } else {
                        return Err("Error parsing embedded events JSON".to_string());
                    }
                } else {
                    return Err("Request must be a JSON array".to_string());
                }
            }
            Err(e) => {
                return Err(format!("Invalid JSON in request: {}", e).to_string());
            }
        }
    }

    fn handle_close(&self, close_json: &str) -> RelayResult {
        // Validate that the request isn't empty
        if close_json.trim().is_empty() {
            return Err("Empty close request received".to_string());
        }

        // Parse the incoming close JSON with detailed error handling
        let close_value: Result<Value, serde_json::Error> = serde_json::from_str(close_json);
        
        match close_value {
            Ok(msg) => {
                // Check if this is a valid CLOSE format according to NIP-01
                if let Some(array) = msg.as_array() {
                    if array.len() >= 2 && array[0].as_str() == Some("CLOSE") {
                        let subscription_id = array[1].as_str().unwrap_or("");
                        
                        // Return a successful close message
                        return Ok(json!({
                            "notice": ["NOTICE", format!("Closed subscription {}", subscription_id)]
                        }).to_string());
                    } else {
                        // If CLOSE message doesn't match expected format, return a NOTICE
                        return Err("Invalid close message format. Expected ['CLOSE', subscription_id]".to_string());
                    }
                } else {
                    // The request is valid JSON but not an array
                    return Err("Invalid close message format. Expected a JSON array".to_string());
                }
            },
            Err(e) => {
                // If JSON parsing failed, return a detailed error message
                let error_msg = format!("Invalid JSON in close request: {}", e.to_string());
                return Err(error_msg);
            }
        }
    }
}

/// Macro to re-export NIP-11 info function from cassette-tools
/// This is now just a simple re-export since relay info is set dynamically
#[macro_export]
macro_rules! implement_info {
    () => {
        // Deprecated: This macro is no longer needed.
        // The info function is now provided directly by cassette-tools
    };
}

// Always provide info function - basic info with supported_nips is always available
#[cfg(not(feature = "nip11"))]
#[no_mangle]
pub extern "C" fn info() -> *mut u8 {
    let nips = crate::nips::build_supported_nips();
    let info_obj = serde_json::json!({"supported_nips": nips});
    crate::string_to_ptr(info_obj.to_string())
}

/// CassetteMacros provides macros to make implementation easier
#[macro_export]
macro_rules! cassette_module {
    ($struct_name:ident, $title:expr, $description:expr) => {
        use cassette_tools::{Cassette, CassetteSchema, RelayHandler, RelayResult};
        use cassette_tools::nip01::{ClientReq, RelayEvent, RelayNotice, RelayEose};
        use serde_json::{json, Value, from_str, to_string};
        use wasm_bindgen::prelude::*;

        #[wasm_bindgen]
        pub struct $struct_name {
            // Internal state can go here
        }

        impl $struct_name {
            #[wasm_bindgen(constructor)]
            pub fn new() -> Self {
                Self {
                    // Initialize internal state
                }
            }
        }

        impl Cassette for $struct_name {
            fn describe() -> String {
                $title.to_string()
            }

            fn get_schema() -> CassetteSchema {
                CassetteSchema {
                    title: $title.to_string(),
                    description: $description.to_string(),
                    schema_type: "object".to_string(),
                    properties: json!({
                        "name": {
                            "type": "string",
                            "description": "The name of the cassette"
                        },
                        "version": {
                            "type": "string",
                            "description": "Version information"
                        }
                    }),
                    required: vec!["name".to_string(), "version".to_string()],
                    items: None,
                }
            }
        }

        #[wasm_bindgen]
        impl $struct_name {
            #[wasm_bindgen]
            pub fn describe() -> String {
                let description = json!({
                    "metadata": {
                        "name": $title,
                        "description": $description,
                    },
                    "req": {
                        "input": from_str(&<ClientReq as Cassette>::get_schema_json()).unwrap_or(json!({})),
                        "output": {
                            "oneOf": [
                                from_str(&<RelayEvent as Cassette>::get_schema_json()).unwrap_or(json!({})),
                                from_str(&<RelayNotice as Cassette>::get_schema_json()).unwrap_or(json!({})),
                                from_str(&<RelayEose as Cassette>::get_schema_json()).unwrap_or(json!({}))
                            ]
                        }
                    },
                    "close": {
                        "input": {
                            "type": "array",
                            "items": [
                                {"const": "CLOSE"},
                                {"type": "string", "description": "Subscription ID to close"}
                            ]
                        },
                        "output": from_str(&<RelayNotice as Cassette>::get_schema_json()).unwrap_or(json!({}))
                    }
                });
                
                to_string(&description).unwrap_or_else(|_| "{}".to_string())
            }

            #[wasm_bindgen]
            pub fn get_schema() -> String {
                <Self as Cassette>::get_schema_json()
            }

            #[wasm_bindgen]
            pub fn req(request_json: &str) -> String {
                // Create an instance and handle the request
                let instance = Self::new();
                match instance.handle_req(request_json) {
                    Ok(response) => response,
                    Err(err) => json!({
                        "notice": ["NOTICE", err]
                    }).to_string()
                }
            }

            #[wasm_bindgen]
            pub fn close(close_json: &str) -> String {
                // Create an instance and handle the close
                let instance = Self::new();
                match instance.handle_close(close_json) {
                    Ok(response) => response,
                    Err(err) => json!({
                        "notice": ["NOTICE", err]
                    }).to_string()
                }
            }
        }
    };
}

/// NIP-01 Implementation
pub mod nip01 {
    use super::*;

    /// Client Request message (REQ)
    pub struct ClientReq;
    
    impl Cassette for ClientReq {
        fn describe() -> String {
            "NIP-01 Client Request (REQ) message".to_string()
        }
        
        fn get_schema() -> CassetteSchema {
            CassetteSchema {
                title: "Client Request".to_string(),
                description: "A message sent by clients to request events from relays".to_string(),
                schema_type: "array".to_string(),
                properties: json!({}),
                required: vec![],
                items: Some(json!([
                    {
                        "const": "REQ"
                    },
                    {
                        "type": "string",
                        "description": "An identifier for the subscription, this should be unique."
                    },
                    {
                        "type": "object",
                        "properties": {
                            "ids": {
                                "type": "array",
                                "items": {
                                    "type": "string",
                                    "pattern": "^[a-f0-9]{64}$"
                                },
                                "description": "A list of event ids in lowercase hex"
                            },
                            "authors": {
                                "type": "array",
                                "items": {
                                    "type": "string",
                                    "pattern": "^[a-f0-9]{64}$"
                                },
                                "description": "A list of lowercase pubkeys, matching the pubkey of an event"
                            },
                            "kinds": {
                                "type": "array",
                                "items": {
                                    "type": "integer",
                                    "minimum": 0
                                },
                                "description": "A list of kind numbers"
                            },
                            "since": {
                                "type": "integer",
                                "minimum": 0,
                                "description": "An integer Unix timestamp in seconds, where events must have created_at >= since"
                            },
                            "until": {
                                "type": "integer",
                                "minimum": 0,
                                "description": "An integer Unix timestamp in seconds, where events must have created_at <= until"
                            },
                            "limit": {
                                "type": "integer",
                                "minimum": 1,
                                "description": "The maximum number of events relays SHOULD return in the initial query"
                            }
                        },
                        "patternProperties": {
                            "^#[a-zA-Z]$": {
                                "type": "array",
                                "items": {
                                    "type": "string"
                                },
                                "description": "A list of tag values, where specific tags (#e, #p) have designated meanings"
                            },
                            "^&[a-zA-Z]$": {
                                "type": "array",
                                "items": {
                                    "type": "string"
                                },
                                "description": "A list of tag values that must ALL be present (NIP-119)"
                            }
                        },
                        "additionalProperties": false
                    }
                ])),
            }
        }
    }
    
    /// Relay Event message (EVENT)
    pub struct RelayEvent;
    
    impl Cassette for RelayEvent {
        fn describe() -> String {
            "NIP-01 Relay Event (EVENT) message".to_string()
        }
        
        fn get_schema() -> CassetteSchema {
            CassetteSchema {
                title: "Relay Event".to_string(),
                description: "A message sent by relays to clients in response to a client request.".to_string(),
                schema_type: "array".to_string(),
                properties: json!({}),
                required: vec![],
                items: Some(json!([
                    {
                        "const": "EVENT"
                    },
                    {
                        "type": "string",
                        "description": "The id of the subscription that the note is being sent in response to"
                    },
                    {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The content of the note"
                            },
                            "created_at": {
                                "type": "integer",
                                "description": "The timestamp of the note creation"
                            },
                            "id": {
                                "type": "string",
                                "pattern": "^[a-f0-9]{64}$",
                                "description": "The id is a hash derived as specified in NIP-01"
                            },
                            "kind": {
                                "type": "integer"
                            },
                            "pubkey": {
                                "type": "string",
                                "pattern": "^[a-f0-9]{64}$",
                                "description": "The public key of the note's author"
                            },
                            "sig": {
                                "type": "string",
                                "description": "The cryptographic signature of the note"
                            },
                            "tags": {
                                "type": "array",
                                "description": "The tags of the note",
                                "items": {
                                    "type": "array",
                                    "items": {
                                        "type": "string"
                                    }
                                }
                            }
                        },
                        "required": [
                            "content",
                            "created_at",
                            "id",
                            "kind",
                            "pubkey",
                            "sig",
                            "tags"
                        ]
                    }
                ])),
            }
        }
    }
    
    /// Relay Notice message (NOTICE)
    pub struct RelayNotice;
    
    impl Cassette for RelayNotice {
        fn describe() -> String {
            "NIP-01 Relay Notice (NOTICE) message".to_string()
        }
        
        fn get_schema() -> CassetteSchema {
            CassetteSchema {
                title: "Relay Notice".to_string(),
                description: "A message sent by relays to clients, usually to inform them of an issue.".to_string(),
                schema_type: "array".to_string(),
                properties: json!({}),
                required: vec![],
                items: Some(json!([
                    {
                        "const": "NOTICE"
                    },
                    {
                        "type": "string"
                    }
                ])),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_roundtrip() {
        let test_str = "Hello, World!";
        let ptr = string_to_ptr(test_str.to_string());
        assert!(!ptr.is_null());

        // Get total length including header
        let total_len = unsafe { std::slice::from_raw_parts(ptr.add(4), 4) };
        let str_len = u32::from_le_bytes(total_len.try_into().unwrap()) as usize;
        assert_eq!(str_len, test_str.len());

        // Read back the string
        let result = ptr_to_string(ptr, str_len + 8); // Add 8 for header
        assert_eq!(result, test_str);

        // Clean up
        unsafe { dealloc_string(ptr, str_len + 8) };
    }

    #[test]
    fn test_empty_string() {
        let test_str = "";
        let ptr = string_to_ptr(test_str.to_string());
        assert!(!ptr.is_null());

        // Get total length including header
        let total_len = unsafe { std::slice::from_raw_parts(ptr.add(4), 4) };
        let str_len = u32::from_le_bytes(total_len.try_into().unwrap()) as usize;
        assert_eq!(str_len, 0);

        // Read back the string
        let result = ptr_to_string(ptr, 8); // Just header for empty string
        assert_eq!(result, test_str);

        // Clean up
        unsafe { dealloc_string(ptr, 8) };
    }

    #[test]
    fn test_unicode_string() {
        let test_str = "Hello, 世界! 🌍";
        let ptr = string_to_ptr(test_str.to_string());
        assert!(!ptr.is_null());

        // Get total length including header
        let total_len = unsafe { std::slice::from_raw_parts(ptr.add(4), 4) };
        let str_len = u32::from_le_bytes(total_len.try_into().unwrap()) as usize;
        assert_eq!(str_len, test_str.len());

        // Read back the string
        let result = ptr_to_string(ptr, str_len + 8);
        assert_eq!(result, test_str);

        // Clean up
        unsafe { dealloc_string(ptr, str_len + 8) };
    }

    #[test]
    fn test_msgb_signature() {
        let test_str = "Test";
        let ptr = string_to_ptr(test_str.to_string());
        
        // Check MSGB signature
        let signature = unsafe { std::slice::from_raw_parts(ptr, 4) };
        assert_eq!(signature, &MSGB_SIGNATURE);

        // Clean up
        unsafe { dealloc_string(ptr, test_str.len() + 8) };
    }

    #[test]
    fn test_large_string() {
        let test_str = "a".repeat(1_000_000); // 1MB string
        let ptr = string_to_ptr(test_str.to_string());
        assert!(!ptr.is_null());

        // Get total length including header
        let total_len = unsafe { std::slice::from_raw_parts(ptr.add(4), 4) };
        let str_len = u32::from_le_bytes(total_len.try_into().unwrap()) as usize;
        assert_eq!(str_len, test_str.len());

        // Read back the string
        let result = ptr_to_string(ptr, str_len + 8);
        assert_eq!(result, test_str);

        // Clean up
        unsafe { dealloc_string(ptr, str_len + 8) };
    }

    #[test]
    fn test_too_large_string() {
        let test_str = "a".repeat(MAX_STRING_LENGTH + 1);
        let ptr = string_to_ptr(test_str);
        assert!(ptr.is_null());
    }

    #[test]
    fn test_null_pointer_handling() {
        let result = ptr_to_string(std::ptr::null(), 0);
        assert_eq!(result, "");
    }

    #[test]
    fn test_buffer_allocation() {
        let size = 1024;
        let ptr = alloc_buffer(size);
        assert!(!ptr.is_null());

        // Write some data
        unsafe {
            let slice = std::slice::from_raw_parts_mut(ptr, size);
            for i in 0..size {
                slice[i] = (i % 256) as u8;
            }
        }

        // Read it back
        unsafe {
            let slice = std::slice::from_raw_parts(ptr, size);
            for i in 0..size {
                assert_eq!(slice[i], (i % 256) as u8);
            }
        }

        // Clean up
        unsafe { dealloc_buffer(ptr, size) };
    }
}
