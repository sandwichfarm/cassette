// Core unit tests for cassette functionality
use cassette_tools::{string_to_ptr, ptr_to_string, get_string_len};
use serde_json::{json, Value};

// Helper function to read total length from a MSGB pointer
fn get_total_len(ptr: *mut u8) -> usize {
    unsafe {
        let len_bytes = std::slice::from_raw_parts(ptr.add(4), 4);
        u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize + 8
    }
}

#[test]
fn test_string_roundtrip() {
    // Test basic string to pointer conversion
    let test_str = "Hello, world!";
    let ptr = string_to_ptr(test_str.to_string());
    let len = get_total_len(ptr);
    let result = ptr_to_string(ptr, len);
    assert_eq!(result, test_str);
}

#[test]
fn test_json_roundtrip() {
    // Test JSON string to pointer conversion
    let json_str = json!({"test": "value", "number": 42}).to_string();
    let ptr = string_to_ptr(json_str.clone());
    let len = get_total_len(ptr);
    let result = ptr_to_string(ptr, len);
    assert_eq!(result, json_str);
    
    // Verify we can parse the result as JSON
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["test"], "value");
    assert_eq!(parsed["number"], 42);
}

#[test]
fn test_get_string_len() {
    // Test get_string_len functionality
    let test_str = "Hello, world!";
    let ptr = string_to_ptr(test_str.to_string());
    let len = get_string_len(ptr);
    assert_eq!(len, test_str.len());
}

// Test NIP-01 format request construction
#[test]
fn test_nip01_req_format() {
    // Create a valid REQ message
    let req = json!([
        "REQ",
        "test_subscription",
        {
            "kinds": [1],
            "limit": 2
        }
    ]);
    
    // Verify the message follows NIP-01 format
    assert_eq!(req[0], "REQ");
    assert_eq!(req[1], "test_subscription");
    assert_eq!(req[2]["kinds"][0], 1);
    assert_eq!(req[2]["limit"], 2);
}

// Test NIP-01 format close construction
#[test]
fn test_nip01_close_format() {
    // Create a valid CLOSE message
    let close = json!([
        "CLOSE",
        "test_subscription"
    ]);
    
    // Verify the message follows NIP-01 format
    assert_eq!(close[0], "CLOSE");
    assert_eq!(close[1], "test_subscription");
} 