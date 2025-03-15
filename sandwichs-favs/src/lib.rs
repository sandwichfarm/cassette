use cassette_tools::{Cassette, CassetteSchema};
use cassette_tools::nip01::{ClientReq, RelayEvent, RelayNotice};
use wasm_bindgen::prelude::*;
use serde_json::{json, Value, from_str, to_string};

// Include the notes.json file at build time
const NOTES_JSON: &str = include_str!("../notes.json");

#[wasm_bindgen]
pub struct SandwichsFavs;

// Adding back the Cassette implementation for SandwichsFavs
impl Cassette for SandwichsFavs {
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

#[wasm_bindgen]
impl SandwichsFavs {
    #[wasm_bindgen]
    pub fn describe() -> String {
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
                            
                            // Implementing "tags" correctly based on NIP-01 with single letter tag filtering
                            // Collect all keys starting with '#' and their corresponding values
                            let tag_filters: Vec<(String, String)> = filter.keys()
                                .filter(|k| k.starts_with('#'))
                                .filter_map(|k| {
                                    let tag_key = k.trim_start_matches('#');
                                    if let Some(values) = filter.get(k).and_then(|v| v.as_array()) {
                                        // Handle array of values for each tag
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
                            
                            // Legacy created_at filter (for backward compatibility)
                            if let Some(created_at) = filter.get("created_at") {
                                if let Some(created_at_obj) = created_at.as_object() {
                                    let min = created_at_obj.get("min").and_then(|min| min.as_i64());
                                    if min.is_some() && (since_filter.is_none() || min > since_filter) {
                                        since_filter = min;
                                    }
                                    
                                    let max = created_at_obj.get("max").and_then(|max| max.as_i64());
                                    if max.is_some() && (until_filter.is_none() || max < until_filter) {
                                        until_filter = max;
                                    }
                                }
                            }
                        }
                    }
                    
                    // Parse the notes embedded at build time
                    let notes: Result<Vec<Value>, _> = from_str(NOTES_JSON);
                    
                    if let Ok(notes) = notes {
                        // Apply all filters in sequence
                        let mut filtered_notes = notes;
                        
                        // Filter by ids if specified
                        if let Some(ids) = ids_filter {
                            filtered_notes = filtered_notes.into_iter()
                                .filter(|note| {
                                    if let Some(id) = note.get("id").and_then(|id| id.as_str()) {
                                        ids.contains(&id.to_string())
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by kinds if specified
                        if let Some(kinds) = kind_filter {
                            filtered_notes = filtered_notes.into_iter()
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
                        if let Some(authors) = author_filter {
                            filtered_notes = filtered_notes.into_iter()
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
                            filtered_notes = filtered_notes.into_iter()
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
                            filtered_notes = filtered_notes.into_iter()
                                .filter(|note| {
                                    if let Some(created_at) = note.get("created_at").and_then(|t| t.as_i64()) {
                                        created_at <= until
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                        }
                        
                        // Filter by tags if specified
                        if let Some(tag_filters) = tags_filter {
                            filtered_notes = filtered_notes.into_iter()
                                .filter(|note| {
                                    if let Some(tags) = note.get("tags").and_then(|t| t.as_array()) {
                                        // For each tag filter, check if note has a matching tag
                                        tag_filters.iter().all(|(key, value)| {
                                            tags.iter().any(|tag| {
                                                if let Some(tag_array) = tag.as_array() {
                                                    // NIP-01 tag format: first element is the single-letter type
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
                        
                        // Apply limit filter if specified
                        if let Some(limit) = limit_filter {
                            if limit < filtered_notes.len() {
                                filtered_notes = filtered_notes.into_iter().take(limit).collect();
                            }
                        }
                        
                        // Convert to EVENT messages
                        let events: Vec<Value> = filtered_notes.into_iter()
                            .map(|note| {
                                json!([
                                    "EVENT",
                                    subscription_id,
                                    note
                                ])
                            })
                            .collect();
                        
                        return json!({
                            "events": events,
                            "eose": ["EOSE", subscription_id]
                        }).to_string();
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