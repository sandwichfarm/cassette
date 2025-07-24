use cassette_tools::{string_to_ptr, ptr_to_string, implement_info};
use serde_json::{json, Value};

// Implement NIP-11 info function (relay info will be set dynamically by CLI)
implement_info!();

// Also re-export the NIP-42 and NIP-45 handlers from cassette-tools
pub use cassette_tools::nips::nip42::{enable_auth, handle_auth};
pub use cassette_tools::nips::nip45::handle_count;

// Test events stored in the cassette
static EVENTS: &str = r#"[
    {
        "id": "test1",
        "pubkey": "test_pubkey_1", 
        "created_at": 1234567890,
        "kind": 1,
        "tags": [],
        "content": "Test event 1",
        "sig": "test_sig_1"
    },
    {
        "id": "test2",
        "pubkey": "test_pubkey_2",
        "created_at": 1234567891,
        "kind": 3,
        "tags": [],
        "content": "Test event 2", 
        "sig": "test_sig_2"
    }
]"#;

#[no_mangle]
pub extern "C" fn describe() -> *mut u8 {
    let metadata = json!({
        "name": "test-nips",
        "description": "Test cassette demonstrating NIP-11, NIP-42, and NIP-45 support",
        "version": "0.1.0",
        "supported_nips": vec![1, 11, 42, 45],
        "event_count": 2,
        "features": ["authentication", "event_counts", "relay_info"]
    });
    
    string_to_ptr(metadata.to_string())
}

#[no_mangle]
pub extern "C" fn req(ptr: *const u8, len: usize) -> *mut u8 {
    let request_str = ptr_to_string(ptr, len);
    
    // Parse the REQ message
    let parsed: Value = match serde_json::from_str(&request_str) {
        Ok(v) => v,
        Err(e) => {
            let error = json!(["NOTICE", format!("JSON parse error: {}", e)]);
            return string_to_ptr(error.to_string());
        }
    };
    
    let arr = match parsed.as_array() {
        Some(a) => a,
        None => {
            let error = json!(["NOTICE", "Invalid REQ format"]);
            return string_to_ptr(error.to_string());
        }
    };
    
    if arr.len() < 3 {
        let error = json!(["NOTICE", "REQ must have at least 3 elements"]);
        return string_to_ptr(error.to_string());
    }
    
    let subscription_id = match arr[1].as_str() {
        Some(id) => id,
        None => {
            let error = json!(["NOTICE", "Invalid subscription ID"]);
            return string_to_ptr(error.to_string());
        }
    };
    
    // Parse our test events
    let events: Vec<Value> = serde_json::from_str(EVENTS).unwrap();
    
    // Build response with EVENT messages
    let mut response_events = Vec::new();
    
    for event in events {
        response_events.push(json!(["EVENT", subscription_id, event]));
    }
    
    // Add EOSE
    response_events.push(json!(["EOSE", subscription_id]));
    
    // Convert to NDJSON format
    let response = response_events.iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    
    string_to_ptr(response)
}

#[no_mangle]
pub extern "C" fn close(ptr: *const u8, len: usize) -> *mut u8 {
    let close_str = ptr_to_string(ptr, len);
    
    // Parse the CLOSE message
    let parsed: Value = match serde_json::from_str(&close_str) {
        Ok(v) => v,
        Err(e) => {
            let error = json!(["NOTICE", format!("JSON parse error: {}", e)]);
            return string_to_ptr(error.to_string());
        }
    };
    
    let arr = match parsed.as_array() {
        Some(a) => a,
        None => {
            let error = json!(["NOTICE", "Invalid CLOSE format"]);
            return string_to_ptr(error.to_string());
        }
    };
    
    if arr.len() >= 2 {
        let subscription_id = arr[1].as_str().unwrap_or("unknown");
        let notice = json!(["NOTICE", format!("Subscription {} closed", subscription_id)]);
        string_to_ptr(notice.to_string())
    } else {
        let error = json!(["NOTICE", "Invalid CLOSE format"]);
        string_to_ptr(error.to_string())
    }
}

// Memory allocation functions are already provided by cassette-tools