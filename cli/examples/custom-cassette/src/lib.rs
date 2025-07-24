use cassette_tools::{Cassette, CassetteSchema, RelayHandler, RelayResult, implement_info};
use cassette_tools::nip01::{ClientReq, RelayEvent, RelayNotice};
use wasm_bindgen::prelude::*;
use serde_json::{json, Value, from_str, to_string};
use chrono::{Utc, DateTime};

// Implement NIP-11 info function (relay info will be set dynamically by CLI)
implement_info!();

#[wasm_bindgen]
pub struct CustomCassette {
    // You can add custom fields here to store state
}

#[wasm_bindgen]
impl CustomCassette {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[wasm_bindgen]
    pub fn describe() -> String {
        // Create the comprehensive API description
        let description = json!({
            "metadata": {
                "name": "Custom Cassette Example",
                "description": "A custom cassette that generates dynamic events",
                "version": "0.1.0",
                "author": "Cassette CLI User",
                "created": Utc::now().to_rfc3339(),
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
        // Create a new instance and handle the request
        let instance = Self::new();
        match instance.handle_req(request_json) {
            Ok(response) => response,
            Err(err) => json!({
                "notice": ["NOTICE", err]
            }).to_string()
        }
    }
    
    /// Handle NIP-01 CLOSE message
    #[wasm_bindgen]
    pub fn close(close_json: &str) -> String {
        // Create a new instance and handle the close
        let instance = Self::new();
        match instance.handle_close(close_json) {
            Ok(response) => response,
            Err(err) => json!({
                "notice": ["NOTICE", err]
            }).to_string()
        }
    }
}

impl Cassette for CustomCassette {
    fn describe() -> String {
        "Custom Cassette - A dynamic event generator".to_string()
    }
    
    fn get_schema() -> CassetteSchema {
        CassetteSchema {
            title: "Custom Cassette".to_string(),
            description: "A custom cassette that generates dynamic events".to_string(),
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
                }
            }),
            required: vec!["name".to_string(), "version".to_string()],
            items: None,
        }
    }
}

/// Get a hex character representation of the number
fn to_hex(num: u64) -> String {
    format!("{:x}", num)
}

/// Helper function to create a Nostr event
fn create_event(kind: i64, content: &str, tags: Vec<Vec<String>>) -> Value {
    // In a real implementation, we would use proper Nostr libraries to:
    // 1. Create a real ID (SHA256 hash of the canonical event representation)
    // 2. Generate a real signature
    
    // For this example, we're just creating a simple event
    // with a fake ID and signature
    let timestamp = Utc::now().timestamp() as u64;
    let fake_id = format!("{}{}", to_hex(timestamp), "0".repeat(64 - to_hex(timestamp).len()));
    let fake_pubkey = "e8b487c079b0f67c695ae6c4c2552a47f38adfa2533cc5926bd2c102942fdcb7".to_string();
    let fake_sig = "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".to_string();
    
    json!({
        "id": fake_id,
        "pubkey": fake_pubkey,
        "created_at": timestamp,
        "kind": kind,
        "tags": tags,
        "content": content,
        "sig": fake_sig
    })
}

impl RelayHandler for CustomCassette {
    fn handle_req(&self, req_json: &str) -> RelayResult {
        // Parse the incoming request JSON
        let req_value: Result<Value, _> = serde_json::from_str(req_json);
        
        if let Ok(req) = req_value {
            // Check if this is a valid REQ format according to NIP-01
            if let Some(array) = req.as_array() {
                if array.len() >= 3 && array[0].as_str() == Some("REQ") {
                    let subscription_id = array[1].as_str().unwrap_or("");
                    
                    // Extract any #custom tags to provide special responses
                    let mut custom_tags = Vec::new();
                    let mut timestamp = Utc::now().timestamp() as u64;
                    
                    for i in 2..array.len() {
                        if let Some(filter) = array[i].as_object() {
                            // Look for custom tags in the filter
                            for (key, value) in filter.iter() {
                                if key == "#custom" {
                                    if let Some(values) = value.as_array() {
                                        for v in values {
                                            if let Some(tag_value) = v.as_str() {
                                                custom_tags.push(tag_value.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Generate some dynamic events
                    let mut events = Vec::new();
                    
                    // Create a welcome event
                    let welcome_event = create_event(
                        1,
                        "Welcome to the CustomCassette! Use #custom tags in your REQ for special events.",
                        vec![
                            vec!["t".to_string(), "welcome".to_string()],
                            vec!["t".to_string(), "cassette".to_string()]
                        ]
                    );
                    events.push(json!(["EVENT", subscription_id, welcome_event]));
                    
                    // Create a timestamp event
                    let timestamp_event = create_event(
                        1,
                        &format!("Current server time: {}", Utc::now().to_rfc3339()),
                        vec![
                            vec!["t".to_string(), "timestamp".to_string()],
                            vec!["t".to_string(), "cassette".to_string()]
                        ]
                    );
                    events.push(json!(["EVENT", subscription_id, timestamp_event]));
                    
                    // Create custom events based on tags
                    for tag in &custom_tags {
                        match tag.as_str() {
                            "echo" => {
                                // Create an event that echoes back the filter
                                let echo_event = create_event(
                                    1,
                                    &format!("Echo of your request: {}", req_json),
                                    vec![
                                        vec!["t".to_string(), "echo".to_string()],
                                        vec!["t".to_string(), "cassette".to_string()]
                                    ]
                                );
                                events.push(json!(["EVENT", subscription_id, echo_event]));
                            },
                            "random" => {
                                // Create a random event
                                let random_id = (timestamp % 1000).to_string();
                                let random_event = create_event(
                                    1,
                                    &format!("Random event #{}", random_id),
                                    vec![
                                        vec!["t".to_string(), "random".to_string()],
                                        vec!["t".to_string(), "cassette".to_string()]
                                    ]
                                );
                                events.push(json!(["EVENT", subscription_id, random_event]));
                            },
                            _ => {
                                // Create a generic custom event
                                let custom_event = create_event(
                                    1,
                                    &format!("Custom tag: {}", tag),
                                    vec![
                                        vec!["t".to_string(), tag.to_string()],
                                        vec!["t".to_string(), "cassette".to_string()]
                                    ]
                                );
                                events.push(json!(["EVENT", subscription_id, custom_event]));
                            }
                        }
                        
                        // Increment timestamp to ensure unique IDs
                        timestamp += 1;
                    }
                    
                    // Add an EOSE message
                    let eose = json!(["EOSE", subscription_id]);
                    
                    // Return the combined response
                    return Ok(json!({
                        "events": events,
                        "eose": eose
                    }).to_string());
                }
            }
            
            // If request doesn't match expected format, return a NOTICE
            return Err("Invalid request format. Expected NIP-01 REQ message.".to_string());
        } else {
            // If JSON parsing failed, return an error notice
            return Err("Invalid JSON in request".to_string());
        }
    }
    
    // We're using the default implementation for handle_close
} 