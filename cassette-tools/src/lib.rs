use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub fn one_plus_one() -> u64 {
    1 + 1
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

// Re-export the macro for convenience
// pub use crate::impl_standard_cassette;

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

/// Relay operation types
pub enum RelayOperation {
    /// Client made a REQ request
    Request,
    /// Client closed a subscription
    Close,
    /// Other operation
    Other(String),
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
        // Parse the incoming request JSON
        let req_value: Result<Value, _> = serde_json::from_str(req_json);
        
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
                        
                        return Ok(json!({
                            "events": events,
                            "eose": ["EOSE", subscription_id]
                        }).to_string());
                    } else {
                        // If we couldn't parse the embedded events JSON
                        return Err("Error parsing embedded events JSON".to_string());
                    }
                }
            }
            
            // If request doesn't match expected format, return a NOTICE
            return Err("Invalid request format. Expected NIP-01 REQ message.".to_string());
        } else {
            // If JSON parsing failed, return an error notice
            return Err("Invalid JSON in request".to_string());
        }
    }
}

/// CassetteMacros provides macros to make implementation easier
#[macro_export]
macro_rules! cassette_module {
    ($struct_name:ident, $title:expr, $description:expr) => {
        use cassette_tools::{Cassette, CassetteSchema, RelayHandler, RelayResult};
        use cassette_tools::nip01::{ClientReq, RelayEvent, RelayNotice};
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
                                    "minimum": "0"
                                },
                                "description": "A list of kind numbers"
                            },
                            "since": {
                                "type": "integer",
                                "minimum": "0",
                                "description": "An integer Unix timestamp in seconds, where events must have created_at >= since"
                            },
                            "until": {
                                "type": "integer",
                                "minimum": "0",
                                "description": "An integer Unix timestamp in seconds, where events must have created_at <= until"
                            },
                            "limit": {
                                "type": "integer",
                                "minimum": "1",
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
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
