pub fn generate_custom_relay_template(name: &str) -> String {
    format!(r#"use cassette_cli::{{RelayImplementation, create_cassette}};
use serde_json::{{json, Value}};
use serde_json::from_str;

// Define your custom relay implementation
#[derive(Default)]
pub struct {}Relay {{
    // Add any state your relay needs here
}}

impl RelayImplementation for {}Relay {{
    fn process_request(&self, request: &str) -> String {{
        // Parse the incoming JSON request
        let req_value: Result<Value, _> = from_str(request);
        
        if let Ok(req) = req_value {{
            // Check if this is a valid REQ format
            if let Some(array) = req.as_array() {{
                if array.len() >= 3 && array[0].as_str() == Some("REQ") {{
                    let subscription_id = array[1].as_str().unwrap_or("");
                    
                    // Process the filters and generate a response
                    // --------------------------------------------
                    // YOUR CUSTOM LOGIC GOES HERE
                    // --------------------------------------------
                    
                    // Example response with a single event
                    return json!({{
                        "events": [
                            ["EVENT", subscription_id, {{
                                "id": "generated_event_id",
                                "pubkey": "your_public_key",
                                "created_at": (std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs() as i64),
                                "kind": 1,
                                "tags": [],
                                "content": "This is a custom relay response!",
                                "sig": "signature_placeholder"
                            }}]
                        ],
                        "eose": ["EOSE", subscription_id]
                    }}).to_string();
                }}
            }}
            
            // Return a notice for invalid requests
            return json!({{
                "notice": ["NOTICE", "Invalid request format"]
            }}).to_string();
        }} else {{
            // Return a notice for unparseable JSON
            return json!({{
                "notice": ["NOTICE", "Invalid JSON in request"]
            }}).to_string();
        }}
    }}
    
    fn get_name(&self) -> String {{
        "{} Relay".to_string()
    }}
    
    fn get_description(&self) -> String {{
        "A custom relay implementation for {}".to_string()
    }}
    
    // Optional: implement connection handlers
    fn on_connect(&self) -> Option<String> {{
        Some(json!({{
            "notice": ["NOTICE", "Welcome to {} Relay!"]
        }}).to_string())
    }}
}}

// Generate the WASM bindings
create_cassette!({}Relay);

// Required for WebAssembly initialization
#[no_mangle]
pub extern "C" fn init() {{
    // Any initialization code can go here
}}
"#, name, name, name, name, name, name)
}

pub fn generate_cargo_toml(name: &str) -> String {
    format!(r#"[package]
name = "{}-relay-cassette"
version = "0.1.0"
edition = "2021"
description = "Custom relay implementation for the Cassette platform"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cassette-cli = {{ path = "../cli" }}
cassette-tools = {{ path = "../cassette-tools" }}
wasm-bindgen = "0.2"
serde_json = "1.0"
serde = {{ version = "1.0", features = ["derive"] }}
"#, name.to_lowercase())
} 