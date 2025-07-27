//! NIP-45: Event counts
//! 
//! This module implements COUNT support for cassettes

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// COUNT request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountRequest {
    pub subscription_id: String,
    pub filters: Vec<Value>,
}

/// COUNT response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountResponse {
    pub count: u64,
}

/// Event count handler trait
pub trait CountHandler {
    /// Count events matching the given filters
    fn count_events(&self, filters: &[Value]) -> u64;
}

/// Handle COUNT request
#[no_mangle]
pub extern "C" fn handle_count(request_ptr: *const u8, request_len: usize) -> *mut u8 {
    unsafe {
        if request_ptr.is_null() || request_len == 0 {
            return std::ptr::null_mut();
        }
        
        let slice = std::slice::from_raw_parts(request_ptr, request_len);
        let request_str = match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        
        // Parse the COUNT request
        let request: Vec<Value> = match serde_json::from_str(request_str) {
            Ok(r) => r,
            Err(_) => return std::ptr::null_mut(),
        };
        
        // Expecting ["COUNT", subscription_id, ...filters]
        if request.len() < 3 || request[0].as_str() != Some("COUNT") {
            return std::ptr::null_mut();
        }
        
        let subscription_id = match request[1].as_str() {
            Some(id) => id,
            None => return std::ptr::null_mut(),
        };
        
        // Get filters
        let filters: Vec<Value> = request[2..].to_vec();
        
        // For now, return a placeholder count
        // In a real implementation, this would query the cassette's event store
        let count = count_events_placeholder(&filters);
        
        // Create COUNT response
        let response = json!(["COUNT", subscription_id, {
            "count": count
        }]);
        
        let json_str = response.to_string();
        crate::string_to_ptr(json_str)
    }
}

/// Placeholder function to count events
/// In a real implementation, this would query the cassette's event store
fn count_events_placeholder(filters: &[Value]) -> u64 {
    // For demonstration, return a fixed count
    // Real implementation would:
    // 1. Parse each filter
    // 2. Query the cassette's event store
    // 3. Return the actual count
    42
}

/// Count events with specific filtering logic
pub fn count_events_with_filters(events: &[Value], filters: &[Value]) -> u64 {
    let mut count = 0u64;
    
    for event in events {
        for filter in filters {
            if event_matches_filter(event, filter) {
                count += 1;
                break; // Count each event only once even if it matches multiple filters
            }
        }
    }
    
    count
}

/// Check if an event matches a filter
fn event_matches_filter(event: &Value, filter: &Value) -> bool {
    let filter_obj = match filter.as_object() {
        Some(obj) => obj,
        None => return false,
    };
    
    // Check kinds
    if let Some(kinds) = filter_obj.get("kinds").and_then(|k| k.as_array()) {
        if let Some(event_kind) = event.get("kind").and_then(|k| k.as_i64()) {
            if !kinds.iter().any(|k| k.as_i64() == Some(event_kind)) {
                return false;
            }
        }
    }
    
    // Check authors
    if let Some(authors) = filter_obj.get("authors").and_then(|a| a.as_array()) {
        if let Some(event_author) = event.get("pubkey").and_then(|p| p.as_str()) {
            if !authors.iter().any(|a| {
                if let Some(author_prefix) = a.as_str() {
                    event_author.starts_with(author_prefix)
                } else {
                    false
                }
            }) {
                return false;
            }
        }
    }
    
    // Check timestamps
    if let Some(since) = filter_obj.get("since").and_then(|s| s.as_i64()) {
        if let Some(created_at) = event.get("created_at").and_then(|t| t.as_i64()) {
            if created_at < since {
                return false;
            }
        }
    }
    
    if let Some(until) = filter_obj.get("until").and_then(|u| u.as_i64()) {
        if let Some(created_at) = event.get("created_at").and_then(|t| t.as_i64()) {
            if created_at > until {
                return false;
            }
        }
    }
    
    // All checks passed
    true
}