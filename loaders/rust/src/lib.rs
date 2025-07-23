use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};
use serde_json::{Value, json};
use wasmtime::*;

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
    req_func: TypedFunc<(i32, i32), i32>,
    describe_func: TypedFunc<(), i32>,
    close_func: Option<TypedFunc<(i32, i32), i32>>,
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

        let req_func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "req")
            .context("req function not found")?;

        let describe_func = instance
            .get_typed_func::<(), i32>(&mut store, "describe")
            .context("describe function not found")?;

        let close_func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "close")
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
            req_func,
            describe_func,
            close_func,
            dealloc_func,
            get_size_func,
            debug,
        })
    }

    /// Get cassette description
    pub fn describe(&mut self) -> Result<String> {
        let ptr = self.describe_func.call(&mut self.store, ())?;
        let desc = self.memory_manager.read_string(&mut self.store, ptr)?;
        
        // Try to deallocate
        if let Some(dealloc) = &self.dealloc_func {
            let _ = dealloc.call(&mut self.store, (ptr, desc.len() as i32));
        }

        Ok(desc)
    }

    /// Process a REQ message
    pub fn req(&mut self, request: &str) -> Result<String> {
        // Parse request to check for new REQ
        if let Ok(req_data) = serde_json::from_str::<Vec<Value>>(request) {
            if req_data.len() >= 2 && req_data[0] == "REQ" {
                // New REQ, reset event tracker
                self.event_tracker.reset();
                if self.debug {
                    eprintln!("[Cassette] New REQ, resetting event tracker");
                }
            }
        }

        // Write request to memory
        let req_ptr = self.memory_manager.write_string(&mut self.store, request)?;

        // Call req function
        let result_ptr = self.req_func.call(&mut self.store, (req_ptr, request.len() as i32))?;

        // Deallocate request
        if let Some(dealloc) = &self.dealloc_func {
            let _ = dealloc.call(&mut self.store, (req_ptr, request.len() as i32));
        }

        if result_ptr == 0 {
            return Ok(json!(["NOTICE", "req() returned null pointer"]).to_string());
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
                        if !["NOTICE", "EVENT", "EOSE"].contains(&msg_type) {
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
        if let Ok(parsed) = serde_json::from_str::<Vec<Value>>(&result_str) {
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

        Ok(result_str)
    }

    /// Process a CLOSE message
    pub fn close(&mut self, close_msg: &str) -> Result<String> {
        let close_func = self.close_func
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("close function not implemented"))?;

        // Write close message to memory
        let close_ptr = self.memory_manager.write_string(&mut self.store, close_msg)?;

        // Call close function
        let result_ptr = close_func.call(&mut self.store, (close_ptr, close_msg.len() as i32))?;

        // Deallocate close message
        if let Some(dealloc) = &self.dealloc_func {
            let _ = dealloc.call(&mut self.store, (close_ptr, close_msg.len() as i32));
        }

        if result_ptr == 0 {
            return Ok(json!(["NOTICE", "close() returned null pointer"]).to_string());
        }

        // Read result
        let result_str = self.memory_manager.read_string(&mut self.store, result_ptr)?;

        // Deallocate result
        if let Some(dealloc) = &self.dealloc_func {
            let _ = dealloc.call(&mut self.store, (result_ptr, result_str.len() as i32));
        }

        Ok(result_str)
    }
}