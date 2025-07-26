use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};
use serde_json::{Value, json};
use wasmtime::*;

/// Result type for send method - either single response or multiple responses
#[derive(Debug)]
pub enum SendResult {
    Single(String),
    Multiple(Vec<String>),
}

/// Event tracker for deduplication
#[derive(Debug, Clone)]
pub struct EventTracker {
    event_ids: Arc<Mutex<HashSet<String>>>,
}

impl EventTracker {
    pub fn new() -> Self {
        Self {
            event_ids: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn reset(&self) {
        self.event_ids.lock().unwrap().clear();
    }

    pub fn add_and_check(&self, event_id: &str) -> bool {
        self.event_ids.lock().unwrap().insert(event_id.to_string())
    }
}

/// Memory manager for WASM operations
pub struct MemoryManager {
    memory: Memory,
    alloc_func: TypedFunc<i32, i32>,
}

impl MemoryManager {
    pub fn new(store: &mut Store<()>, instance: &Instance) -> Result<Self> {
        let memory = instance
            .get_memory(&mut *store, "memory")
            .context("memory export not found")?;
        
        let alloc_func = instance
            .get_typed_func::<i32, i32>(&mut *store, "alloc_string")
            .context("alloc_string function not found")?;

        Ok(Self { memory, alloc_func })
    }

    pub fn write_string(&self, store: &mut Store<()>, s: &str) -> Result<i32> {
        let data = s.as_bytes();
        let ptr = self.alloc_func.call(&mut *store, data.len() as i32)?;
        
        if ptr == 0 {
            anyhow::bail!("allocation failed");
        }

        self.memory.write(&mut *store, ptr as usize, data)?;
        Ok(ptr)
    }

    pub fn read_string(&self, store: &mut Store<()>, ptr: i32) -> Result<String> {
        if ptr == 0 {
            anyhow::bail!("null pointer");
        }

        let data = self.memory.data(&store);
        let ptr_usize = ptr as usize;

        // Check for MSGB format
        if ptr_usize + 8 <= data.len() {
            let signature = &data[ptr_usize..ptr_usize + 4];
            if signature == b"MSGB" {
                // Read length (little endian)
                let length_bytes = &data[ptr_usize + 4..ptr_usize + 8];
                let length = u32::from_le_bytes([
                    length_bytes[0],
                    length_bytes[1],
                    length_bytes[2],
                    length_bytes[3],
                ]) as usize;

                if ptr_usize + 8 + length <= data.len() {
                    return String::from_utf8(
                        data[ptr_usize + 8..ptr_usize + 8 + length].to_vec()
                    ).context("invalid UTF-8");
                }
            }
        }

        // Fall back to null-terminated string
        let mut end = ptr_usize;
        while end < data.len() && data[end] != 0 {
            end += 1;
        }

        String::from_utf8(data[ptr_usize..end].to_vec())
            .context("invalid UTF-8")
    }
}

/// Cassette loader
pub struct Cassette {
    store: Store<()>,
    instance: Instance,
    memory_manager: MemoryManager,
    event_tracker: EventTracker,
    send_func: TypedFunc<(i32, i32), i32>,
    info_func: Option<TypedFunc<(), i32>>,
    dealloc_func: Option<TypedFunc<(i32, i32), ()>>,
    get_size_func: Option<TypedFunc<i32, i32>>,
    debug: bool,
}

impl Cassette {
    /// Load a cassette from a WASM file
    pub fn load(path: &str, debug: bool) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, path)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        let memory_manager = MemoryManager::new(&mut store, &instance)?;

        let send_func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "send")
            .context("send function not found")?;

        let info_func = instance
            .get_typed_func::<(), i32>(&mut store, "info")
            .ok();

        let dealloc_func = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string")
            .ok();

        let get_size_func = instance
            .get_typed_func::<i32, i32>(&mut store, "get_allocation_size")
            .ok();

        Ok(Self {
            store,
            instance,
            memory_manager,
            event_tracker: EventTracker::new(),
            send_func,
            info_func,
            dealloc_func,
            get_size_func,
            debug,
        })
    }

    /// Get cassette description by synthesizing from info
    pub fn describe(&mut self) -> Result<String> {
        match self.info() {
            Ok(info_str) => {
                match serde_json::from_str::<Value>(&info_str) {
                    Ok(info) => {
                        let mut parts = Vec::new();
                        
                        if let Some(name) = info.get("name").and_then(|v| v.as_str()) {
                            parts.push(name.to_string());
                        }
                        
                        if let Some(description) = info.get("description").and_then(|v| v.as_str()) {
                            parts.push(description.to_string());
                        }
                        
                        if let Some(nips) = info.get("supported_nips").and_then(|v| v.as_array()) {
                            let nip_numbers: Vec<String> = nips.iter()
                                .filter_map(|v| v.as_i64().map(|n| n.to_string()))
                                .collect();
                            if !nip_numbers.is_empty() {
                                parts.push(format!("Supports NIPs: {}", nip_numbers.join(", ")));
                            }
                        }
                        
                        if parts.is_empty() {
                            Ok("No description available".to_string())
                        } else {
                            Ok(parts.join(" - "))
                        }
                    }
                    Err(_) => Ok("Invalid cassette info format".to_string())
                }
            }
            Err(_) => Ok("No cassette info available".to_string())
        }
    }

    /// Send any NIP-01 message to the cassette
    /// For REQ messages, returns a Vec of responses. For other messages, returns a single response.
    pub fn send(&mut self, message: &str) -> Result<SendResult> {
        // Parse message to determine type
        let (is_req_message, subscription_id) = if let Ok(msg_data) = serde_json::from_str::<Vec<Value>>(message) {
            if msg_data.len() >= 2 {
                let msg_type = msg_data[0].as_str().unwrap_or("");
                
                match msg_type {
                    "REQ" => {
                        // New REQ, reset event tracker
                        self.event_tracker.reset();
                        if self.debug {
                            eprintln!("[Cassette] New REQ, resetting event tracker");
                        }
                        let sub_id = msg_data[1].as_str().unwrap_or("").to_string();
                        (true, sub_id)
                    }
                    "CLOSE" => {
                        // CLOSE message, also reset event tracker
                        self.event_tracker.reset();
                        if self.debug {
                            eprintln!("[Cassette] CLOSE message, resetting event tracker");
                        }
                        (false, String::new())
                    }
                    _ => (false, String::new())
                }
            } else {
                (false, String::new())
            }
        } else {
            (false, String::new())
        };

        // If it's a REQ message, collect all events until EOSE
        if is_req_message {
            let results = self._collect_all_events_for_req(message, &subscription_id)?;
            Ok(SendResult::Multiple(results))
        } else {
            // For non-REQ messages, use single call
            let result = self._send_single(message)?;
            Ok(SendResult::Single(result))
        }
    }

    // Private method for single send call
    fn _send_single(&mut self, message: &str) -> Result<String> {
        // Write message to memory
        let msg_ptr = self.memory_manager.write_string(&mut self.store, message)?;

        // Call send function
        let result_ptr = self.send_func.call(&mut self.store, (msg_ptr, message.len() as i32))?;

        // Deallocate message
        if let Some(dealloc) = &self.dealloc_func {
            let _ = dealloc.call(&mut self.store, (msg_ptr, message.len() as i32));
        }

        if result_ptr == 0 {
            return Ok(json!(["NOTICE", "send() returned null pointer"]).to_string());
        }

        // Read result
        let result_str = self.memory_manager.read_string(&mut self.store, result_ptr)?;

        // Deallocate result
        if let Some(dealloc) = &self.dealloc_func {
            let size = if let Some(get_size) = &self.get_size_func {
                get_size.call(&mut self.store, result_ptr).unwrap_or(result_str.len() as i32)
            } else {
                result_str.len() as i32
            };
            let _ = dealloc.call(&mut self.store, (result_ptr, size));
        }

        // Process results
        self._process_results(&result_str)
    }

    // Process results with event deduplication
    fn _process_results(&mut self, result_str: &str) -> Result<String> {
        // Handle newline-separated messages
        if result_str.contains('\n') {
            let messages: Vec<&str> = result_str.trim().split('\n').collect();
            if self.debug {
                eprintln!("[Cassette] Processing {} newline-separated messages", messages.len());
            }

            let mut filtered_messages = Vec::new();
            for message in messages {
                match serde_json::from_str::<Vec<Value>>(message) {
                    Ok(parsed) => {
                        if parsed.len() < 2 {
                            if self.debug {
                                eprintln!("[Cassette] Invalid message format: {}", message);
                            }
                            continue;
                        }

                        let msg_type = parsed[0].as_str().unwrap_or("");
                        if !["NOTICE", "EVENT", "EOSE", "OK", "CLOSED", "AUTH"].contains(&msg_type) {
                            if self.debug {
                                eprintln!("[Cassette] Unknown message type: {}", msg_type);
                            }
                            continue;
                        }

                        // Filter duplicate events
                        if msg_type == "EVENT" && parsed.len() >= 3 {
                            if let Some(event) = parsed[2].as_object() {
                                if let Some(event_id) = event.get("id").and_then(|v| v.as_str()) {
                                    if !self.event_tracker.add_and_check(event_id) {
                                        if self.debug {
                                            eprintln!("[Cassette] Filtering duplicate event: {}", event_id);
                                        }
                                        continue;
                                    }
                                }
                            }
                        }

                        filtered_messages.push(message);
                    }
                    Err(e) => {
                        if self.debug {
                            eprintln!("[Cassette] Failed to parse message: {}", e);
                        }
                    }
                }
            }

            return Ok(filtered_messages.join("\n"));
        }

        // Single message - check for duplicate
        if let Ok(parsed) = serde_json::from_str::<Vec<Value>>(result_str) {
            if parsed.len() >= 3 && parsed[0] == "EVENT" {
                if let Some(event) = parsed[2].as_object() {
                    if let Some(event_id) = event.get("id").and_then(|v| v.as_str()) {
                        if !self.event_tracker.add_and_check(event_id) {
                            if self.debug {
                                eprintln!("[Cassette] Filtering duplicate event: {}", event_id);
                            }
                            return Ok(String::new());
                        }
                    }
                }
            }
        }

        Ok(result_str.to_string())
    }

    // Private method to collect all events for REQ messages  
    fn _collect_all_events_for_req(&mut self, message: &str, subscription_id: &str) -> Result<Vec<String>> {
        if self.debug {
            eprintln!("[Cassette] Collecting all events for REQ subscription: {}", subscription_id);
        }

        let mut results = Vec::new();

        // Keep calling until we get EOSE or terminating condition
        loop {
            let response = self._send_single(message)?;

            // Empty response means no more events
            if response.is_empty() {
                if self.debug {
                    eprintln!("[Cassette] Received empty response, stopping");
                }
                break;
            }

            // Try to parse the response
            match serde_json::from_str::<Vec<Value>>(&response) {
                Ok(parsed) if parsed.len() >= 1 => {
                    let msg_type = parsed[0].as_str().unwrap_or("");

                    match msg_type {
                        "EOSE" => {
                            if self.debug {
                                eprintln!("[Cassette] Received EOSE for subscription {}", subscription_id);
                            }
                            results.push(response);
                            break;
                        }
                        "CLOSED" => {
                            if self.debug {
                                eprintln!("[Cassette] Received CLOSED for subscription {}", subscription_id);
                            }
                            results.push(response);
                            break;
                        }
                        _ => {
                            // Add the response to results
                            results.push(response);
                        }
                    }
                }
                _ => {
                    if self.debug {
                        eprintln!("[Cassette] Failed to parse response, stopping");
                    }
                    break;
                }
            }
        }

        // Check if we have an EOSE message
        let has_eose = results.iter().any(|r| {
            if let Ok(parsed) = serde_json::from_str::<Vec<Value>>(r) {
                parsed.len() >= 1 && parsed[0].as_str() == Some("EOSE")
            } else {
                false
            }
        });

        // If no EOSE, add one
        if !has_eose {
            results.push(json!(["EOSE", subscription_id]).to_string());
        }

        Ok(results)
    }



    /// Get NIP-11 relay information
    pub fn info(&mut self) -> Result<String> {
        let info_func = self.info_func
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("info function not implemented"))?;

        let ptr = info_func.call(&mut self.store, ())?;
        
        if ptr == 0 {
            return Ok(json!({"supported_nips": []}).to_string());
        }

        let info_str = self.memory_manager.read_string(&mut self.store, ptr)?;
        
        // Try to deallocate
        if let Some(dealloc) = &self.dealloc_func {
            let _ = dealloc.call(&mut self.store, (ptr, info_str.len() as i32));
        }

        Ok(info_str)
    }
}