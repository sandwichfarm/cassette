use cassette_tools::{Cassette, CassetteSchema};
use cassette_tools::nip01::{ClientReq, RelayEvent, RelayNotice};
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

#[wasm_bindgen]
pub struct {{cassette_struct}};

// Cassette implementation for our generated cassette
impl Cassette for {{cassette_struct}} {
    fn describe() -> String {
        format!("{} - {}", CASSETTE_NAME, CASSETTE_DESC)
    }
    
    fn get_schema() -> CassetteSchema {
        CassetteSchema {
            title: CASSETTE_NAME.to_string(),
            description: CASSETTE_DESC.to_string(),
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
                },
                "eventCount": {
                    "type": "integer",
                    "description": "Number of events in the cassette"
                }
            }),
            required: vec!["name".to_string(), "version".to_string()],
            items: None,
        }
    }
}

#[wasm_bindgen]
impl {{cassette_struct}} {
    #[wasm_bindgen]
    pub fn describe() -> String {
        // Create the comprehensive API description
        let description = json!({
            "metadata": {
                "name": CASSETTE_NAME,
                "description": CASSETTE_DESC,
                "version": CASSETTE_VERSION,
                "author": CASSETTE_AUTHOR,
                "created": CASSETTE_CREATED,
                "eventCount": {{event_count}}
            },
            "req": {
                "input": from_str(&<ClientReq as Cassette>::get_schema_json()).unwrap_or(json!({})),
                "output": {
                    "oneOf": [
                        from_str(&<RelayEvent as Cassette>::get_schema_json()).unwrap_or(json!({})),
                        from_str(&<RelayNotice as Cassette>::get_schema_json()).unwrap_or(json!({}))
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
    
    /// Process a NIP-01 REQ message and return either an EVENT or NOTICE response
    #[wasm_bindgen]
    pub fn req(request_json: &str) -> String {
        // Parse the incoming request JSON
        let req_value: Result<Value, _> = from_str(request_json);
        
        if let Ok(req) = req_value {
            // Check if this is a valid REQ format according to NIP-01
            if let Some(array) = req.as_array() {
                if array.len() >= 3 && array[0].as_str() == Some("REQ") {
                    let subscription_id = array[1].as_str().unwrap_or("");
                    
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
                        }
                    }
                    
                    // Parse the events embedded at build time
                    let events: Result<Vec<Value>, _> = from_str(EVENTS_JSON);
                    
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
                        
                        // Convert to EVENT messages
                        let events: Vec<Value> = filtered_events.into_iter()
                            .map(|event| {
                                json!([
                                    "EVENT",
                                    subscription_id,
                                    event
                                ])
                            })
                            .collect();
                        
                        return json!({
                            "events": events,
                            "eose": ["EOSE", subscription_id]
                        }).to_string();
                    } else {
                        // If we couldn't parse the embedded events JSON
                        return json!({
                            "notice": ["NOTICE", "Error parsing embedded events JSON"]
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
    
    /// Handle NIP-01 CLOSE message
    #[wasm_bindgen]
    pub fn close(close_json: &str) -> String {
        // Parse the incoming close JSON
        let close_value: Result<Value, _> = from_str(close_json);
        
        if let Ok(close) = close_value {
            // Check if this is a valid CLOSE format according to NIP-01
            if let Some(array) = close.as_array() {
                if array.len() >= 2 && array[0].as_str() == Some("CLOSE") {
                    let subscription_id = array[1].as_str().unwrap_or("");
                    
                    // Return a notice acknowledging the closure
                    return json!({
                        "notice": ["NOTICE", format!("Subscription {} closed", subscription_id)]
                    }).to_string();
                }
            }
            
            // If request doesn't match expected format, return a NOTICE
            return json!({
                "notice": ["NOTICE", "Invalid request format. Expected NIP-01 CLOSE message."]
            }).to_string();
        } else {
            // If JSON parsing failed, return an error notice
            return json!({
                "notice": ["NOTICE", "Invalid JSON in request"]
            }).to_string();
        }
    }
} 