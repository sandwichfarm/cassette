use cassette_tools::{Cassette, CassetteSchema};
use cassette_tools::nip01::{ClientReq, RelayEvent, RelayNotice};
use wasm_bindgen::prelude::*;
use serde_json::{json, Value, from_str, to_string};
use std::collections::HashMap;

// Include the notes.json file at build time
const NOTES_JSON: &str = include_str!("../notes.json");

#[wasm_bindgen]
pub struct TestCassette;

// Adding back the Cassette implementation for TestCassette
impl Cassette for TestCassette {
    fn describe() -> String {
        "Sandwich's Favorite Notes".to_string()
    }
    
    fn get_schema() -> CassetteSchema {
        CassetteSchema {
            title: "Sandwich's Favorite Notes".to_string(),
            description: "A cassette for managing sandwich favorites".to_string(),
            schema_type: "object".to_string(),
            properties: json!({
                "name": {
                    "type": "string",
                    "description": "The name of the sandwich"
                },
                "ingredients": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "List of ingredients in the sandwich"
                },
                "rating": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 5,
                    "description": "Rating from 1-5 stars"
                }
            }),
            required: vec!["name".to_string(), "ingredients".to_string()],
            items: None,
        }
    }
}

// Implement WASM exports with standardized names
#[wasm_bindgen]
impl TestCassette {
    // Export a standardized describe function
    #[wasm_bindgen(js_name = describe)]
    pub fn describe_impl() -> String {
        // Create the comprehensive API description
        let description = json!({
            "req": {
                "input": from_str(&<ClientReq as Cassette>::get_schema_json()).unwrap_or(json!({})),
                "output": {
                    "oneOf": [
                        from_str(&<RelayEvent as Cassette>::get_schema_json()).unwrap_or(json!({})),
                        from_str(&<RelayNotice as Cassette>::get_schema_json()).unwrap_or(json!({}))
                    ]
                }
            },
            "event": {
                "input": {
                    "type": "array",
                    "items": [
                        {"const": "EVENT"},
                        {"type": "object", "description": "The event object following the NIP-01 format"}
                    ]
                },
                "output": from_str(&<RelayNotice as Cassette>::get_schema_json()).unwrap_or(json!({}))
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
    
    // Export a standardized get_schema function
    #[wasm_bindgen(js_name = get_schema)]
    pub fn get_schema_impl() -> String {
        <Self as Cassette>::get_schema_json()
    }
    
    // Export a standardized req function
    #[wasm_bindgen(js_name = req)]
    pub fn req_impl(request_json: &str) -> String {
        // Parse the incoming request JSON
        let req_value: Result<Value, _> = from_str(request_json);
        
        if let Ok(req) = req_value {
            // Check if this is a valid REQ format according to NIP-01
            if let Some(array) = req.as_array() {
                if array.len() >= 3 && array[0].as_str() == Some("REQ") {
                    let subscription_id = array[1].as_str().unwrap_or("");
                    
                    // Try to get filters from the request
                    let mut kinds_filter = None;
                    let mut authors_filter = None;
                    let mut since_filter = None;
                    let mut until_filter = None;
                    let mut limit_filter = None;
                    let mut ids_filter = None;
                    let mut tag_filters = HashMap::new();
                    let mut and_tag_filters = HashMap::new();
                    
                    // Process filters (starting from index 2)
                    for i in 2..array.len() {
                        if let Some(filter) = array[i].as_object() {
                            // Look for kind filter
                            if let Some(kinds) = filter.get("kinds") {
                                if let Some(kinds_array) = kinds.as_array() {
                                    kinds_filter = Some(
                                        kinds_array.iter()
                                            .filter_map(|k| k.as_i64())
                                            .collect::<Vec<i64>>()
                                    );
                                }
                            }
                            
                            // Look for author filter - correctly use "authors" according to NIP-01
                            if let Some(authors) = filter.get("authors") {
                                if let Some(authors_array) = authors.as_array() {
                                    authors_filter = Some(
                                        authors_array.iter()
                                            .filter_map(|a| a.as_str().map(String::from))
                                            .collect::<Vec<String>>()
                                    );
                                }
                            }
                            
                            // Special handling for the 'nak' tool which uses #p for pubkey filtering
                            if let Some(p_tags) = filter.get("#p") {
                                if let Some(p_array) = p_tags.as_array() {
                                    let p_authors: Vec<String> = p_array.iter()
                                        .filter_map(|p| p.as_str().map(String::from))
                                        .collect();
                                    
                                    if !p_authors.is_empty() {
                                        // For nak tool, we should treat #p as 'authors' filter, not as tags
                                        if let Some(ref mut authors) = authors_filter {
                                            // Append to existing authors
                                            authors.extend(p_authors);
                                        } else {
                                            // Create new authors list
                                            authors_filter = Some(p_authors);
                                        }
                                    }
                                }
                            }
                            
                            // Look for ids filter (event IDs)
                            if let Some(ids) = filter.get("ids") {
                                if let Some(ids_array) = ids.as_array() {
                                    ids_filter = Some(
                                        ids_array.iter()
                                            .filter_map(|id| id.as_str().map(String::from))
                                            .collect::<Vec<String>>()
                                    );
                                }
                            }
                            
                            // Look for since filter
                            if let Some(since_val) = filter.get("since") {
                                since_filter = since_val.as_i64();
                            }
                            
                            // Look for until filter
                            if let Some(until_val) = filter.get("until") {
                                until_filter = until_val.as_i64();
                            }
                            
                            // Look for limit filter
                            if let Some(limit_val) = filter.get("limit") {
                                limit_filter = limit_val.as_u64().map(|l| l as usize);
                            }
                            
                            // Process standard tag filters
                            for (key, value) in filter.iter() {
                                if key.starts_with('#') && key != "#p" { // Skip #p as it's handled specially
                                    let tag_key = key.trim_start_matches('#').to_string();
                                    if let Some(values) = value.as_array() {
                                        let tag_values: Vec<String> = values.iter()
                                            .filter_map(|v| v.as_str().map(String::from))
                                            .collect();
                                        
                                        if !tag_values.is_empty() {
                                            tag_filters.insert(tag_key, tag_values);
                                        }
                                    }
                                }
                            }
                            
                            // Process NIP-119 AND tag filters
                            for (key, value) in filter.iter() {
                                if key.starts_with('&') {
                                    let tag_key = key.trim_start_matches('&').to_string();
                                    if let Some(values) = value.as_array() {
                                        let tag_values: Vec<String> = values.iter()
                                            .filter_map(|v| v.as_str().map(String::from))
                                            .collect();
                                        
                                        if !tag_values.is_empty() {
                                            and_tag_filters.insert(tag_key, tag_values);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Parse the notes embedded at build time
                    let notes: Result<Vec<Value>, _> = from_str(NOTES_JSON);
                    
                    if let Ok(mut notes) = notes {
                        // Filter by ids if specified
                        if let Some(ids) = &ids_filter {
                            notes = notes.into_iter()
                                .filter(|note| {
                                    if let Some(id) = note.get("id").and_then(|id| id.as_str()) {
                                        ids.iter().any(|filter_id| filter_id == id)
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by kinds if specified
                        if let Some(kinds) = &kinds_filter {
                            notes = notes.into_iter()
                                .filter(|note| {
                                    if let Some(kind) = note.get("kind").and_then(|k| k.as_i64()) {
                                        kinds.contains(&kind)
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by authors if specified
                        if let Some(authors) = &authors_filter {
                            notes = notes.into_iter()
                                .filter(|note| {
                                    if let Some(pubkey) = note.get("pubkey").and_then(|p| p.as_str()) {
                                        authors.contains(&pubkey.to_string())
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by since timestamp
                        if let Some(since) = since_filter {
                            notes = notes.into_iter()
                                .filter(|note| {
                                    if let Some(created_at) = note.get("created_at").and_then(|t| t.as_i64()) {
                                        created_at >= since
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by until timestamp
                        if let Some(until) = until_filter {
                            notes = notes.into_iter()
                                .filter(|note| {
                                    if let Some(created_at) = note.get("created_at").and_then(|t| t.as_i64()) {
                                        created_at <= until
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by standard tags
                        for (tag_key, tag_values) in &tag_filters {
                            notes = notes.into_iter()
                                .filter(|note| {
                                    if let Some(tags) = note.get("tags").and_then(|t| t.as_array()) {
                                        tag_values.iter().any(|value| {
                                            tags.iter().any(|tag| {
                                                if let Some(tag_array) = tag.as_array() {
                                                    if tag_array.len() >= 2 {
                                                        let tag_type = tag_array[0].as_str().unwrap_or("");
                                                        let tag_value = tag_array[1].as_str().unwrap_or("");
                                                        tag_type == tag_key && tag_value == value
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
                        
                        // Filter by NIP-119 AND tag filters
                        for (tag_key, tag_values) in &and_tag_filters {
                            notes = notes.into_iter()
                                .filter(|note| {
                                    if let Some(tags) = note.get("tags").and_then(|t| t.as_array()) {
                                        // All values in this group must be found in at least one tag each
                                        tag_values.iter().all(|value| {
                                            // For each value, find at least one matching tag
                                            tags.iter().any(|tag| {
                                                if let Some(tag_array) = tag.as_array() {
                                                    if tag_array.len() >= 2 {
                                                        let tag_type = tag_array[0].as_str().unwrap_or("");
                                                        let tag_value = tag_array[1].as_str().unwrap_or("");
                                                        tag_type == tag_key && tag_value == value
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
                        
                        // Sort by created_at timestamp in descending order before limiting
                        notes.sort_by(|a, b| {
                            let a_time = a.get("created_at").and_then(|t| t.as_i64()).unwrap_or(0);
                            let b_time = b.get("created_at").and_then(|t| t.as_i64()).unwrap_or(0);
                            b_time.cmp(&a_time) // Descending order (newest first)
                        });
                        
                        // Apply limit filter if specified
                        if let Some(limit) = limit_filter {
                            if notes.len() > limit {
                                notes.truncate(limit);
                            }
                        }
                        
                        // Create response with EVENT messages - simplified to work with the test format
                        let events_json = to_string(&notes).unwrap_or_default();
                        return events_json;
                    } else {
                        // If we couldn't parse the embedded notes JSON
                        return json!({
                            "notice": ["NOTICE", "Error parsing embedded notes JSON"]
                        }).to_string();
                    }
                }
            }
            
            // If request doesn't match expected format, return a NOTICE
            return json!({
                "notice": ["NOTICE", "Invalid request format. Expected NIP-01 REQ message."]
            }).to_string();
        } else {
            // If JSON parsing failed, return an error notice
            return json!({
                "notice": ["NOTICE", "Invalid JSON in request"]
            }).to_string();
        }
    }
    
    // Export a standardized event function
    #[wasm_bindgen(js_name = event)]
    pub fn event_impl(event_json: &str) -> String {
        // For now, just acknowledge the event
        let result = json!({
            "success": true,
            "message": "Event received"
        });
        
        to_string(&result).unwrap_or_else(|_| "{}".to_string())
    }
    
    // Export a standardized close function
    #[wasm_bindgen(js_name = close)]
    pub fn close_impl(subscription_id: &str) -> String {
        // For now, just acknowledge the close
        let result = json!({
            "success": true,
            "message": format!("Subscription {} closed", subscription_id)
        });
        
        to_string(&result).unwrap_or_else(|_| "{}".to_string())
    }
}