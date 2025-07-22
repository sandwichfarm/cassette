#![crate_name = "cassette_cli"]
#![crate_type = "lib"]

use cassette_tools::CassetteSchema;
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use cassette_tools::{string_to_ptr, ptr_to_string};

// Include the notes.json file at build time
const NOTES_JSON: &str = include_str!("../notes.json");

#[derive(Serialize, Deserialize, Debug)]
struct Note {
    id: String,
    pubkey: String,
    created_at: i64,
    #[serde(default)]
    kind: i64,
    tags: Vec<Vec<String>>,
    content: String,
    sig: String,
}

// Helper function for allocating strings
#[no_mangle]
pub extern "C" fn alloc_string(len: usize) -> *mut u8 {
    // Allocate memory with additional padding to prevent TypedArray errors
    let mut buf = Vec::with_capacity(len + 16);
    buf.resize(len + 16, 0);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

// Helper function for deallocating strings
#[no_mangle]
pub extern "C" fn dealloc_string_impl(ptr: *mut u8, len: usize) {
    unsafe {
        let len_with_padding = len + 16;
        let _ = Vec::from_raw_parts(ptr, len, len_with_padding);
        // Memory will be deallocated when Vec is dropped
    }
}

// Add the standard function that the cassette-loader expects
#[no_mangle]
pub extern "C" fn dealloc_string(ptr: *mut u8) {
    // First get the length from the MSGB format (4 bytes header + 4 bytes length)
    let len = unsafe {
        if ptr.is_null() {
            return;
        }
        let header = std::slice::from_raw_parts(ptr, 8);
        if header[0] == b'M' && header[1] == b'S' && header[2] == b'G' && header[3] == b'B' {
            // MSGB format has length at bytes 4-7
            u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize
        } else {
            // Fallback: use a fixed size if we can't determine length
            1024
        }
    };
    
    dealloc_string_impl(ptr, len);
}

// Standardized NIP-01 interface functions
#[no_mangle]
pub extern "C" fn describe() -> *mut u8 {
    let metadata = json!({
        "name": "test_cassette_direct",
        "description": "E2E Test Cassette (Direct)",
        "version": "0.1.0",
        "author": "E2E Test",
        "created": "2025-03-21 07:59:38",
        "eventCount": 6
    });

    let req_schema = json!({
        "type": "object",
        "properties": {
            "kinds": {
                "type": "array",
                "items": { "type": "integer" }
            },
            "authors": {
                "type": "array",
                "items": { "type": "string" }
            },
            "since": { "type": "integer" },
            "until": { "type": "integer" },
            "limit": { "type": "integer" },
            "ids": {
                "type": "array",
                "items": { "type": "string" }
            }
        }
    });

    let close_schema = json!({
        "type": "object",
        "properties": {
            "subscription_id": { "type": "string" }
        },
        "required": ["subscription_id"]
    });

    let description = json!({
        "metadata": metadata,
        "req": {
            "input": req_schema,
            "output": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "pubkey": { "type": "string" },
                        "created_at": { "type": "integer" },
                        "kind": { "type": "integer" },
                        "tags": {
                            "type": "array",
                            "items": {
                                "type": "array",
                                "items": { "type": "string" }
                            }
                        },
                        "content": { "type": "string" },
                        "sig": { "type": "string" }
                    },
                    "required": ["id", "pubkey", "created_at", "kind", "tags", "content", "sig"]
                }
            }
        },
        "close": {
            "input": close_schema,
            "output": {
                "type": "object",
                "properties": {
                    "type": { "type": "string", "enum": ["notice"] },
                    "message": { "type": "string" }
                },
                "required": ["type", "message"]
            }
        }
    });

    string_to_ptr(description.to_string())
}

#[no_mangle]
pub extern "C" fn get_schema() -> *mut u8 {
    let schema = CassetteSchema {
        title: "Test Cassette Direct".to_string(),
        description: "E2E Test Cassette (Direct)".to_string(),
        properties: json!({
            "id": { "type": "string" },
            "pubkey": { "type": "string" },
            "created_at": { "type": "integer" },
            "kind": { "type": "integer" },
            "tags": {
                "type": "array",
                "items": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            },
            "content": { "type": "string" },
            "sig": { "type": "string" }
        }),
        schema_type: "object".to_string(),
        required: vec![
            "id".to_string(),
            "pubkey".to_string(),
            "created_at".to_string(),
            "kind".to_string(),
            "tags".to_string(),
            "content".to_string(),
            "sig".to_string(),
        ],
        items: None,
    };

    string_to_ptr(serde_json::to_string(&schema).unwrap())
}

#[no_mangle]
pub extern "C" fn req(request_ptr: *const u8, request_len: usize) -> *mut u8 {
    if request_ptr.is_null() {
        return string_to_ptr(json!(["NOTICE", "Error: Null request pointer"]).to_string());
    }

    // Use ptr_to_string which understands the MSGB header format
    let request = ptr_to_string(request_ptr, request_len);

    // Parse the request
    let request: Value = match serde_json::from_str(&request) {
        Ok(r) => r,
        Err(e) => return string_to_ptr(json!(["NOTICE", format!("Invalid JSON request: {}", e)]).to_string()),
    };

    // Parse notes from the embedded NOTES_JSON
    let notes: Vec<Note> = match serde_json::from_str(NOTES_JSON) {
        Ok(n) => n,
        Err(e) => {
            // Add more detailed debugging for the JSON parse error
            let error_context = format!("Failed to parse notes.json: {}. First 100 chars: {}", 
                                        e, 
                                        NOTES_JSON.chars().take(100).collect::<String>());
            return string_to_ptr(json!(["NOTICE", error_context]).to_string());
        }
    };

    let mut filtered_notes = notes;

    // Process the request
    if let Some(array) = request.as_array() {
        if array.len() >= 3 {
            let subscription_id = match array[1].as_str() {
                Some(id) => id,
                None => return string_to_ptr(json!(["NOTICE", "Invalid subscription ID"]).to_string()),
            };
            
            let filter = &array[2];
            if let Some(obj) = filter.as_object() {
                // Process standard filters first
                if let Some(kinds) = obj.get("kinds").and_then(|k| k.as_array()) {
                    filtered_notes.retain(|note| kinds.iter().any(|k| k.as_i64().map_or(false, |k| k == note.kind)));
                }
                if let Some(authors) = obj.get("authors").and_then(|a| a.as_array()) {
                    filtered_notes.retain(|note| authors.iter().any(|a| a.as_str().map_or(false, |a| a == &note.pubkey)));
                }
                if let Some(since) = obj.get("since").and_then(|s| s.as_i64()) {
                    filtered_notes.retain(|note| note.created_at >= since);
                }
                if let Some(until) = obj.get("until").and_then(|u| u.as_i64()) {
                    filtered_notes.retain(|note| note.created_at <= until);
                }
                if let Some(limit) = obj.get("limit").and_then(|l| l.as_i64()) {
                    filtered_notes.truncate(limit as usize);
                }
                if let Some(ids) = obj.get("ids").and_then(|i| i.as_array()) {
                    filtered_notes.retain(|note| ids.iter().any(|id| id.as_str().map_or(false, |id| id == &note.id)));
                }
                
                // Process tag filters with proper validation
                for (key, value) in obj.iter() {
                    if key.starts_with('#') && key.len() > 1 {
                        let tag_name = &key[1..]; // Remove the '#' prefix
                        
                        // Validate tag values array
                        if let Some(tag_values) = value.as_array() {
                            // Check that all values are strings
                            let mut all_strings = true;
                            let mut string_values = Vec::new();
                            
                            for val in tag_values {
                                if let Some(str_val) = val.as_str() {
                                    string_values.push(str_val.to_string());
                                } else {
                                    all_strings = false;
                                    break;
                                }
                            }
                            
                            if !all_strings {
                                return string_to_ptr(json!(["NOTICE", format!("Tag values for {} must be strings", key)]).to_string());
                            }
                            
                            // Apply tag filter with validated string values
                            filtered_notes.retain(|note| {
                                let matching_tags: Vec<_> = note.tags.iter()
                                    .filter(|tag| tag.len() > 0 && tag[0] == tag_name)
                                    .collect();
                                
                                string_values.iter().any(|val| {
                                    matching_tags.iter().any(|tag| tag.len() > 1 && tag[1] == *val)
                                })
                            });
                        } else {
                            return string_to_ptr(json!(["NOTICE", format!("Tag filter {} must be an array", key)]).to_string());
                        }
                    }
                    // Process NIP-119 AND tag filters
                    else if key.starts_with('&') && key.len() > 1 {
                        let tag_name = &key[1..]; // Remove the '&' prefix
                        
                        // Validate tag values array
                        if let Some(tag_values) = value.as_array() {
                            // Check that all values are strings
                            let mut all_strings = true;
                            let mut string_values = Vec::new();
                            
                            for val in tag_values {
                                if let Some(str_val) = val.as_str() {
                                    string_values.push(str_val.to_string());
                                } else {
                                    all_strings = false;
                                    break;
                                }
                            }
                            
                            if !all_strings {
                                return string_to_ptr(json!(["NOTICE", format!("Tag values for {} must be strings", key)]).to_string());
                            }
                            
                            // Apply AND tag filter with validated string values
                            filtered_notes.retain(|note| {
                                let matching_tags: Vec<_> = note.tags.iter()
                                    .filter(|tag| tag.len() > 0 && tag[0] == tag_name)
                                    .collect();
                                
                                string_values.iter().all(|val| {
                                    matching_tags.iter().any(|tag| tag.len() > 1 && tag[1] == *val)
                                })
                            });
                        } else {
                            return string_to_ptr(json!(["NOTICE", format!("Tag filter {} must be an array", key)]).to_string());
                        }
                    }
                }
            }

            // Send multiple events in separate EVENT messages
            if filtered_notes.is_empty() {
                return string_to_ptr(json!(["EOSE", subscription_id]).to_string());
            }

            // Return the first event wrapped in NIP-01 format
            let first_note = filtered_notes.remove(0);
            let event_value = serde_json::to_value(&first_note).unwrap();
            return string_to_ptr(json!(["EVENT", subscription_id, event_value]).to_string());
        } else {
            return string_to_ptr(json!(["NOTICE", "Invalid REQ format: missing fields"]).to_string());
        }
    } else {
        return string_to_ptr(json!(["NOTICE", "Invalid REQ format: not an array"]).to_string());
    }
}

#[no_mangle]
pub extern "C" fn event(event_ptr: *const u8, event_len: usize) -> *mut u8 {
    if event_ptr.is_null() {
        return string_to_ptr(json!(["NOTICE", "Error: Null event pointer"]).to_string());
    }

    // Parse the event
    let _event = ptr_to_string(event_ptr, event_len);
    
    // For now, just acknowledge the event
    string_to_ptr(json!(["NOTICE", "Event received"]).to_string())
}

#[no_mangle]
pub extern "C" fn close(close_ptr: *const u8, close_len: usize) -> *mut u8 {
    if close_ptr.is_null() {
        return string_to_ptr(json!(["NOTICE", "Error: Null close pointer"]).to_string());
    }

    // Use ptr_to_string which understands the MSGB header format
    let close = ptr_to_string(close_ptr, close_len);

    // Parse the close request
    let close: Value = match serde_json::from_str(&close) {
        Ok(c) => c,
        Err(e) => return string_to_ptr(json!(["NOTICE", format!("Invalid JSON: {}", e)]).to_string()),
    };

    // Extract the subscription ID (not used currently but kept for future reference)
    let _subscription_id = close.as_array()
        .and_then(|a| a.get(1))
        .and_then(|s| s.as_str())
        .unwrap_or("");

    // Return a simple NOTICE response
    string_to_ptr(json!(["NOTICE", "Subscription closed"]).to_string())
}