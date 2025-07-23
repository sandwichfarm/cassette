// Test file for cassette functionality
use cassette_tools::{string_to_ptr, ptr_to_string, get_string_len};
use serde_json::{json, Value};

// Import functions directly
use cassette_cli::{req, close};

// First test the core string handling functions
#[test]
fn test_string_roundtrip() {
    // Test basic string to pointer conversion
    let test_str = "Hello, world!";
    let ptr = string_to_ptr(test_str.to_string());
    let len = unsafe {
        let len_bytes = std::slice::from_raw_parts(ptr.add(4), 4);
        u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize + 8
    };
    let result = ptr_to_string(ptr, len);
    assert_eq!(result, test_str);
}

#[test]
fn test_json_roundtrip() {
    // Test JSON string to pointer conversion
    let json_str = json!({"test": "value", "number": 42}).to_string();
    let ptr = string_to_ptr(json_str.clone());
    let len = unsafe {
        let len_bytes = std::slice::from_raw_parts(ptr.add(4), 4);
        u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize + 8
    };
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

#[test]
fn test_req_function() {
    // Create a valid REQ message
    let req_msg = json!([
        "REQ",
        "test_subscription",
        {
            "kinds": [1],
            "limit": 1
        }
    ]).to_string();

    // Convert to pointer with MSGB format
    let ptr = string_to_ptr(req_msg);
    let len = unsafe {
        let len_bytes = std::slice::from_raw_parts(ptr.add(4), 4);
        u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize + 8
    };

    // Call the req function
    let result_ptr = req(ptr, len);
    
    // Read the response length
    let result_len = unsafe {
        let len_bytes = std::slice::from_raw_parts(result_ptr.add(4), 4);
        u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize + 8
    };
    
    // Convert back to string
    let result = ptr_to_string(result_ptr, result_len);
    
    // Parse the response and verify the format
    let parsed: Value = serde_json::from_str(&result).unwrap();
    println!("Parsed REQ response: {:#?}", parsed);
    
    // It should be either an EVENT or a NOTICE
    assert!(parsed[0] == "EVENT" || parsed[0] == "NOTICE");
}

#[test]
fn test_close_function() {
    // Create a valid CLOSE message
    let close_msg = json!([
        "CLOSE",
        "test_subscription"
    ]).to_string();

    // Convert to pointer with MSGB format
    let ptr = string_to_ptr(close_msg);
    let len = unsafe {
        let len_bytes = std::slice::from_raw_parts(ptr.add(4), 4);
        u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize + 8
    };

    // Call the close function
    let result_ptr = close(ptr, len);
    
    // Read the response length
    let result_len = unsafe {
        let len_bytes = std::slice::from_raw_parts(result_ptr.add(4), 4);
        u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize + 8
    };
    
    // Convert back to string
    let result = ptr_to_string(result_ptr, result_len);
    
    // Parse the response and verify the format
    let parsed: Value = serde_json::from_str(&result).unwrap();
    println!("Parsed CLOSE response: {:#?}", parsed);
    
    // It should be a NOTICE
    assert_eq!(parsed[0], "NOTICE");
}

// Add more tests as needed to verify core functionality without
// relying on the main module implementation 