// Integration tests for NIP-01 interface functions
use cassette_tools::{string_to_ptr, ptr_to_string};
use serde_json::{json, Value};

// Now we can import directly
use cassette_cli::{req, close};

// Helper function to read total length from a MSGB pointer
fn get_total_len(ptr: *mut u8) -> usize {
    unsafe {
        let len_bytes = std::slice::from_raw_parts(ptr.add(4), 4);
        u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize + 8
    }
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

    // Convert to pointer
    let ptr = string_to_ptr(req_msg);
    let len = get_total_len(ptr);

    // Call the req function
    let result_ptr = req(ptr, len);
    
    // Get response and parse it
    let result_len = get_total_len(result_ptr);
    let result = ptr_to_string(result_ptr, result_len);
    println!("REQ response: {}", result);
    
    // Response should be a valid JSON
    let parsed: Value = serde_json::from_str(&result).unwrap();
    
    // It should be either an EVENT or a NOTICE
    assert!(
        parsed[0] == "EVENT" || parsed[0] == "EOSE" || parsed[0] == "NOTICE", 
        "Expected EVENT, EOSE or NOTICE, got: {:?}", parsed[0]
    );
}

#[test]
fn test_req_invalid_json() {
    // Test with invalid JSON
    let ptr = string_to_ptr("not a valid JSON".to_string());
    let len = get_total_len(ptr);

    // Call the req function
    let result_ptr = req(ptr, len);
    
    // Get response and parse it
    let result_len = get_total_len(result_ptr);
    let result = ptr_to_string(result_ptr, result_len);
    println!("Invalid REQ response: {}", result);
    
    // Response should be a valid JSON
    let parsed: Value = serde_json::from_str(&result).unwrap();
    
    // Should be a NOTICE for invalid input
    assert_eq!(parsed[0], "NOTICE");
}

#[test]
fn test_close_function() {
    // Create a valid CLOSE message
    let close_msg = json!([
        "CLOSE",
        "test_subscription"
    ]).to_string();

    // Convert to pointer
    let ptr = string_to_ptr(close_msg);
    let len = get_total_len(ptr);

    // Call the close function
    let result_ptr = close(ptr, len);
    
    // Get response and parse it
    let result_len = get_total_len(result_ptr);
    let result = ptr_to_string(result_ptr, result_len);
    println!("CLOSE response: {}", result);
    
    // Response should be a valid JSON
    let parsed: Value = serde_json::from_str(&result).unwrap();
    
    // It should be a NOTICE
    assert_eq!(parsed[0], "NOTICE");
}

#[test]
fn test_close_invalid_json() {
    // Test with invalid JSON
    let ptr = string_to_ptr("not a valid JSON".to_string());
    let len = get_total_len(ptr);

    // Call the close function
    let result_ptr = close(ptr, len);
    
    // Get response and parse it
    let result_len = get_total_len(result_ptr);
    let result = ptr_to_string(result_ptr, result_len);
    println!("Invalid CLOSE response: {}", result);
    
    // Response should be a valid JSON
    let parsed: Value = serde_json::from_str(&result).unwrap();
    
    // Should be a NOTICE for invalid input
    assert_eq!(parsed[0], "NOTICE");
} 