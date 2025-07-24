// Import consistent memory management functions from cassette_tools
extern crate cassette_tools;
use cassette_tools::{string_to_ptr, ptr_to_string, implement_info};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::cell::RefCell;

// Embed relay metadata at compile time
const EMBEDDED_RELAY_INFO: &str = r#"{
{{#if relay_name}}"name": "{{relay_name}}",{{/if}}
{{#if relay_description}}"description": "{{relay_description}}",{{/if}}
{{#if relay_pubkey}}"pubkey": "{{relay_pubkey}}",{{/if}}
{{#if relay_contact}}"contact": "{{relay_contact}}",{{/if}}
"supported_nips": []
}"#;

// Custom info function that includes embedded relay metadata
#[cfg(feature = "nip11")]
#[no_mangle]
pub extern "C" fn info() -> *mut u8 {
    // Parse the embedded relay info and ensure supported_nips is populated
    let mut relay_info: serde_json::Map<String, serde_json::Value> = serde_json::from_str(EMBEDDED_RELAY_INFO)
        .unwrap_or_else(|_| serde_json::Map::new());
    
    // Always update supported_nips with current build features
    relay_info.insert(
        "supported_nips".to_string(), 
        serde_json::json!(cassette_tools::nips::build_supported_nips())
    );
    
    let json_str = serde_json::to_string(&relay_info).unwrap_or_else(|_| "{}".to_string());
    string_to_ptr(json_str)
}

// NIP-11 info function is provided by cassette-tools
// When NIP-11 is enabled, it provides the full implementation
// When NIP-11 is not enabled, it provides a stub implementation

// Import the req and close functions from the lib crate
// extern crate cassette_cli;
// use cassette_cli::{req as req_impl, close as close_impl};

// Core types that match NIP-01 exactly
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Filter {
    ids: Option<Vec<String>>,
    authors: Option<Vec<String>>,
    kinds: Option<Vec<i64>>,
    #[serde(flatten)]
    tag_filters: std::collections::HashMap<String, Vec<String>>,
    since: Option<i64>,
    until: Option<i64>,
    limit: Option<usize>,
    // NIP-50: Search field
    search: Option<String>,
}

// Custom deserialization helpers to ensure NIP-119 tag filters are correctly parsed
impl Filter {
    // Helper method to check if a key is a NIP-119 AND filter
    fn is_nip119_and_filter(key: &str) -> bool {
        key.starts_with('&') && key.len() > 1
    }

    // Helper method to check if a key is a regular tag filter
    fn is_regular_tag_filter(key: &str) -> bool {
        key.starts_with('#') && key.len() == 2
    }
}

#[derive(Serialize, Deserialize, Clone)]
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

// Events embedded by CLI during build
#[cfg(not(test))]
const EVENTS: &str = r###"{{{ events_json }}}"###;

#[cfg(test)]
const EVENTS: &str = r#"[{
    "id": "test_id",
    "pubkey": "test_pubkey",
    "created_at": 1234567890,
    "kind": 1,
    "tags": [
        ["t", "tag1"],
        ["t", "tag2"],
        ["p", "pubkey1"]
    ],
    "content": "test content",
    "sig": "test_sig"
}]"#;

// Streaming state management
thread_local! {
    static SUBSCRIPTION_EVENTS: RefCell<Vec<Note>> = RefCell::new(Vec::new());
    static CURRENT_INDEX: RefCell<usize> = RefCell::new(0);
    static CURRENT_SUBSCRIPTION: RefCell<String> = RefCell::new(String::new());
    static EOSE_SENT: RefCell<bool> = RefCell::new(false);
    static DEBUG_MSGS: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

// Core interface functions
#[no_mangle]
pub extern "C" fn describe() -> *mut u8 {
    // Collect any debug messages
    let debug_messages = DEBUG_MSGS.with(|msgs| {
        let msgs = msgs.borrow();
        msgs.join("\n")
    });

    let metadata = json!({
        "name": "{{ cassette_name }}",
        "description": "{{ cassette_description }}",
        "version": "{{ cassette_version }}",
        "author": "{{ cassette_author }}",
        "created": "{{ cassette_created }}",
        "event_count": {{ event_count }},
        "debug_log": debug_messages
    });

    string_to_ptr(metadata.to_string())
}

#[no_mangle]
pub extern "C" fn req(ptr: *const u8, len: usize) -> *mut u8 {
    if ptr.is_null() {
        return string_to_ptr(json!(["NOTICE", "Error: Null request pointer"]).to_string());
    }

    // Get the request string from the pointer
    let request_str = ptr_to_string(ptr, len);
    
    // Add debug log
    DEBUG_MSGS.with(|msgs| {
        let mut msgs = msgs.borrow_mut();
        msgs.push(format!("Request received: {}", request_str));
    });
    
    // Parse the message to check if it's COUNT or REQ
    let msg = match serde_json::from_str::<Value>(&request_str) {
        Ok(v) => v,
        Err(e) => {
            // Log parsing error
            DEBUG_MSGS.with(|msgs| {
                let mut msgs = msgs.borrow_mut();
                msgs.push(format!("JSON parse error: {} in: {}", e, request_str));
            });
            return string_to_ptr(json!(["NOTICE", format!("Invalid JSON: {}", e)]).to_string());
        }
    };

    // Validate message format
    if !msg.is_array() {
        DEBUG_MSGS.with(|msgs| {
            let mut msgs = msgs.borrow_mut();
            msgs.push(format!("Message is not an array: {}", msg));
        });
        return string_to_ptr(json!(["NOTICE", "Message must be an array"]).to_string());
    }
    
    let arr = msg.as_array().unwrap();
    if arr.is_empty() {
        return string_to_ptr(json!(["NOTICE", "Empty message array"]).to_string());
    }
    
    // Check command type
    let command = arr[0].as_str().unwrap_or("");
    match command {
        "COUNT" => handle_count_command(&arr),
        "REQ" => handle_req_command(&arr),
        _ => {
            DEBUG_MSGS.with(|msgs| {
                let mut msgs = msgs.borrow_mut();
                msgs.push(format!("Unknown command: {}", command));
            });
            string_to_ptr(json!(["NOTICE", format!("Unknown command: {}", command)]).to_string())
        }
    }
}

// Handle COUNT command
fn handle_count_command(arr: &[Value]) -> *mut u8 {
    if arr.len() < 3 {
        return string_to_ptr(json!(["NOTICE", "COUNT must contain at least command, id, and filter"]).to_string());
    }
    
    let subscription_id = arr[1].as_str().unwrap_or("").to_string();
    if subscription_id.is_empty() {
        return string_to_ptr(json!(["NOTICE", "Invalid subscription ID"]).to_string());
    }
    
    // Parse filters
    let mut filters = Vec::new();
    for f in &arr[2..] {
        match serde_json::from_value::<Filter>(f.clone()) {
            Ok(filter) => filters.push(filter),
            Err(e) => {
                DEBUG_MSGS.with(|msgs| {
                    let mut msgs = msgs.borrow_mut();
                    msgs.push(format!("Filter parse error in COUNT: {} in: {}", e, f));
                });
            }
        }
    }
    
    // Load and parse events
    let events: Vec<Note> = match serde_json::from_str(EVENTS) {
        Ok(notes) => notes,
        Err(_) => Vec::new(),
    };
    
    // Count matching events
    let mut count = 0usize;
    'count_loop: for event in &events {
        for filter in &filters {
            if matches_filter(event, filter) {
                count += 1;
                continue 'count_loop;
            }
        }
    }
    
    // Return COUNT response according to NIP-45
    string_to_ptr(json!(["COUNT", subscription_id, {
        "count": count
    }]).to_string())
}

// Handle REQ command  
fn handle_req_command(arr: &[Value]) -> *mut u8 {
    if arr.len() < 3 {
        return string_to_ptr(json!(["NOTICE", "REQ must contain at least command, id, and filter"]).to_string());
    }
    
    let subscription_id = arr[1].as_str().unwrap_or("").to_string();
    if subscription_id.is_empty() {
        DEBUG_MSGS.with(|msgs| {
            let mut msgs = msgs.borrow_mut();
            msgs.push("Empty subscription ID".to_string());
        });
        return string_to_ptr(json!(["NOTICE", "Invalid subscription ID"]).to_string());
    }
    
    // Parse filters
    let mut filters = Vec::new();
    for f in &arr[2..] {
        match serde_json::from_value::<Filter>(f.clone()) {
            Ok(filter) => filters.push(filter),
            Err(e) => {
                DEBUG_MSGS.with(|msgs| {
                    let mut msgs = msgs.borrow_mut();
                    msgs.push(format!("Filter parse error: {} in: {}", e, f));
                });
                // Continue with any valid filters
            }
        }
    }

    // Check if this is a new subscription
    CURRENT_SUBSCRIPTION.with(|current| {
        let mut current = current.borrow_mut();
        if *current != subscription_id {
            *current = subscription_id.clone();
            CURRENT_INDEX.with(|idx| *idx.borrow_mut() = 0);
            EOSE_SENT.with(|sent| *sent.borrow_mut() = false);
        }
    });

    // Load and parse events if needed
    let mut should_load_events = false;
    SUBSCRIPTION_EVENTS.with(|events| {
        should_load_events = events.borrow().is_empty();
    });

    if should_load_events {
        // Load and parse events
        let events: Vec<Note> = match serde_json::from_str(EVENTS) {
            Ok(notes) => notes,
            Err(e) => {
                DEBUG_MSGS.with(|msgs| {
                    let mut msgs = msgs.borrow_mut();
                    msgs.push(format!("Failed to parse embedded events: {}", e));
                });
                
                // Print the problematic JSON for debugging
                DEBUG_MSGS.with(|msgs| {
                    let mut msgs = msgs.borrow_mut();
                    // Only print the first 200 chars to avoid overflowing logs
                    let preview = if EVENTS.len() > 200 {
                        format!("{}...(truncated)", &EVENTS[0..200])
                    } else {
                        EVENTS.to_string()
                    };
                    msgs.push(format!("Events JSON: {}", preview));
                    
                    // Attempt to analyze the first few characters
                    let first_few = EVENTS.chars().take(20).collect::<String>();
                    msgs.push(format!("First 20 chars: {:?}", first_few));
                    
                    // Check if it appears to be a JSON array
                    if !EVENTS.trim().starts_with('[') {
                        msgs.push("Error: Events JSON doesn't start with '[' character".to_string());
                    }
                });
                
                // Return a notice with more detailed error information including the exact error position
                let error_msg = format!("Failed to load events: {} at position {}", e, e.column());
                return string_to_ptr(json!(["NOTICE", error_msg]).to_string())
            }
        };

        // Apply filters (NIP-01: filters are OR'd together, conditions within a filter are AND'd)
        let mut matching_events = Vec::new();
        
        'event_loop: for event in events {
            for filter in &filters {
                if matches_filter(&event, filter) {
                    matching_events.push(event.clone());
                    continue 'event_loop;
                }
            }
        }

        // Check if any filter has a search query (NIP-50)
        let has_search_query = filters.iter().any(|f| f.search.is_some());
        
        if has_search_query {
            // NIP-50: Sort by search relevance (highest score first)
            #[cfg(feature = "nip50")]
            {
                // Get the first search query for scoring
                let search_query = filters.iter()
                    .find_map(|f| f.search.as_ref())
                    .cloned()
                    .unwrap_or_default();
                    
                matching_events.sort_by(|a, b| {
                    let score_a = score_event_for_search(a, &search_query);
                    let score_b = score_event_for_search(b, &search_query);
                    score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        } else {
            // Default: Sort by created_at in reverse order (newest first)
            matching_events.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        }
        
        // Apply limit if specified - find the highest limit across all filters
        let max_limit = filters.iter()
            .filter_map(|f| f.limit)
            .max();
            
        if let Some(limit) = max_limit {
            matching_events.truncate(limit);
        }

        // Store events for this subscription
        SUBSCRIPTION_EVENTS.with(|subs| {
            subs.borrow_mut().clear();
            subs.borrow_mut().extend_from_slice(&matching_events);
        });
    }

    // Return next event or EOSE
    SUBSCRIPTION_EVENTS.with(|events| {
        CURRENT_INDEX.with(|idx| {
            let events = events.borrow();
            let mut idx = idx.borrow_mut();
            
            if *idx < events.len() {
                // Stream one event at a time
                let response = json!(["EVENT", subscription_id.clone(), &events[*idx]]);
                *idx += 1;
                string_to_ptr(response.to_string())
            } else if !EOSE_SENT.with(|sent| *sent.borrow()) {
                // All events have been sent, now send EOSE
                EOSE_SENT.with(|sent| *sent.borrow_mut() = true);
                string_to_ptr(json!(["EOSE", subscription_id.clone()]).to_string())
            } else {
                // No more events and EOSE already sent
                string_to_ptr(json!(["NOTICE", "No more events"]).to_string())
            }
        })
    })
}

#[no_mangle]
pub extern "C" fn close(close_ptr: *const u8, close_len: usize) -> *mut u8 {
    // Safety check for null pointer
    if close_ptr.is_null() {
        return string_to_ptr(json!(["NOTICE", "Error: Null close pointer"]).to_string());
    }
    
    // Get the close request string from the pointer and validate it
    let close_str = ptr_to_string(close_ptr, close_len);
    
    // Add debug log
    DEBUG_MSGS.with(|msgs| {
        let mut msgs = msgs.borrow_mut();
        msgs.push(format!("CLOSE received: {}", close_str));
    });
    
    // Parse the close request
    let close_value = match serde_json::from_str::<Value>(&close_str) {
        Ok(val) => val,
        Err(e) => {
            DEBUG_MSGS.with(|msgs| {
                let mut msgs = msgs.borrow_mut();
                msgs.push(format!("CLOSE JSON parse error: {} in: {}", e, close_str));
            });
            return string_to_ptr(json!(["NOTICE", format!("Invalid JSON in close request: {}", e)]).to_string());
        }
    };
    
    // Validate the close request format
    if !close_value.is_array() {
        DEBUG_MSGS.with(|msgs| {
            let mut msgs = msgs.borrow_mut();
            msgs.push(format!("CLOSE is not an array: {}", close_value));
        });
        return string_to_ptr(json!(["NOTICE", "CLOSE message must be an array"]).to_string());
    }
    
    let arr = close_value.as_array().unwrap();
    if arr.len() < 2 {
        DEBUG_MSGS.with(|msgs| {
            let mut msgs = msgs.borrow_mut();
            msgs.push("CLOSE array too short".to_string());
        });
        return string_to_ptr(json!(["NOTICE", "CLOSE must contain command and subscription ID"]).to_string());
    }
    
    if arr[0].as_str().unwrap_or("") != "CLOSE" {
        DEBUG_MSGS.with(|msgs| {
            let mut msgs = msgs.borrow_mut();
            msgs.push(format!("First element is not 'CLOSE': {}", arr[0]));
        });
        return string_to_ptr(json!(["NOTICE", "First element must be CLOSE"]).to_string());
    }
    
    let subscription_id = arr[1].as_str().unwrap_or("").to_string();
    if subscription_id.is_empty() {
        DEBUG_MSGS.with(|msgs| {
            let mut msgs = msgs.borrow_mut();
            msgs.push("Empty subscription ID in CLOSE".to_string());
        });
        return string_to_ptr(json!(["NOTICE", "Invalid subscription ID"]).to_string());
    }
    
    // Reset the subscription state
    CURRENT_SUBSCRIPTION.with(|sub| *sub.borrow_mut() = String::new());
    CURRENT_INDEX.with(|idx| *idx.borrow_mut() = 0);
    SUBSCRIPTION_EVENTS.with(|events| events.borrow_mut().clear());
    EOSE_SENT.with(|sent| *sent.borrow_mut() = false);
    
    // Respond with a simple notice
    string_to_ptr(json!(["NOTICE", "Subscription closed"]).to_string())
}

// Helper function to check if an event matches a filter according to NIP-01
fn matches_filter(event: &Note, filter: &Filter) -> bool {
    // Check IDs
    if let Some(ids) = &filter.ids {
        if !ids.contains(&event.id) {
            return false;
        }
    }

    // Check authors
    if let Some(authors) = &filter.authors {
        if !authors.contains(&event.pubkey) {
            return false;
        }
    }

    // Check kinds
    if let Some(kinds) = &filter.kinds {
        if !kinds.contains(&event.kind) {
            return false;
        }
    }

    // Check timestamps
    if let Some(since) = filter.since {
        if event.created_at < since {
            return false;
        }
    }
    if let Some(until) = filter.until {
        if event.created_at > until {
            return false;
        }
    }

    // Check tag filters
    for (key, values) in &filter.tag_filters {
        // Log the tag filter for debugging
        DEBUG_MSGS.with(|msgs| {
            let mut msgs = msgs.borrow_mut();
            msgs.push(format!("Checking tag filter: {} with values: {:?}", key, values));
        });
        
        if key.starts_with('&') {
            // NIP-119: All tag values must be present
            let tag_name = &key[1..];
            
            DEBUG_MSGS.with(|msgs| {
                let mut msgs = msgs.borrow_mut();
                msgs.push(format!("NIP-119 AND filter for tag '{}' with values: {:?}", tag_name, values));
            });
            
            let tag_values: Vec<String> = event.tags.iter()
                .filter(|t| t.get(0).map_or(false, |n| n == tag_name))
                .filter_map(|t| t.get(1).cloned())
                .collect();
                
            DEBUG_MSGS.with(|msgs| {
                let mut msgs = msgs.borrow_mut();
                msgs.push(format!("Event has tag values: {:?}", tag_values));
            });

            // For NIP-119 AND semantics, all requested values must be present
            if !values.iter().all(|v| tag_values.contains(v)) {
                DEBUG_MSGS.with(|msgs| {
                    let mut msgs = msgs.borrow_mut();
                    msgs.push(format!("NIP-119 filter failed - not all values present"));
                });
                return false;
            }
            
            DEBUG_MSGS.with(|msgs| {
                let mut msgs = msgs.borrow_mut();
                msgs.push(format!("NIP-119 filter matched"));
            });
        } else if key.starts_with('#') {
            // Regular tag filter: Any value must match
            let tag_name = &key[1..];
            
            DEBUG_MSGS.with(|msgs| {
                let mut msgs = msgs.borrow_mut();
                msgs.push(format!("Regular tag filter for tag '{}' with values: {:?}", tag_name, values));
            });
            
            let tag_values: Vec<String> = event.tags.iter()
                .filter(|t| t.get(0).map_or(false, |n| n == tag_name))
                .filter_map(|t| t.get(1).cloned())
                .collect();
                
            DEBUG_MSGS.with(|msgs| {
                let mut msgs = msgs.borrow_mut();
                msgs.push(format!("Event has tag values: {:?}", tag_values));
            });

            // For regular OR semantics, at least one value must be present
            if !values.iter().any(|v| tag_values.contains(v)) {
                DEBUG_MSGS.with(|msgs| {
                    let mut msgs = msgs.borrow_mut();
                    msgs.push(format!("Regular tag filter failed - no matching values"));
                });
                return false;
            }
            
            DEBUG_MSGS.with(|msgs| {
                let mut msgs = msgs.borrow_mut();
                msgs.push(format!("Regular tag filter matched"));
            });
        }
    }

    // Check search query (NIP-50)
    #[cfg(feature = "nip50")]
    if let Some(search_query) = &filter.search {
        if !matches_search_query(event, search_query) {
            return false;
        }
    }

    // If we've passed all conditions, the event matches the filter
    true
}

// NIP-50 search functionality
#[cfg(feature = "nip50")]
fn matches_search_query(event: &Note, search_query: &str) -> bool {
    // Convert to serde_json::Value for compatibility with nip50 module
    let event_json = serde_json::json!({
        "id": event.id,
        "pubkey": event.pubkey,
        "created_at": event.created_at,
        "kind": event.kind,
        "tags": event.tags,
        "content": event.content,
        "sig": event.sig
    });
    
    // Use the nip50 module's scoring function
    cassette_tools::nips::nip50::score_event(&event_json, 
        &cassette_tools::nips::nip50::parse_search_query(search_query)) > 0.0
}

#[cfg(feature = "nip50")]
fn score_event_for_search(event: &Note, search_query: &str) -> f32 {
    // Convert to serde_json::Value for compatibility with nip50 module
    let event_json = serde_json::json!({
        "id": event.id,
        "pubkey": event.pubkey,
        "created_at": event.created_at,
        "kind": event.kind,
        "tags": event.tags,
        "content": event.content,
        "sig": event.sig
    });
    
    // Use the nip50 module's scoring function
    cassette_tools::nips::nip50::score_event(&event_json, 
        &cassette_tools::nips::nip50::parse_search_query(search_query))
}

// Note: Memory management functions are already exported by cassette_tools
// We don't need to re-export them here to avoid duplicate symbols

