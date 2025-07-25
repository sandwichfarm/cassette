use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write, BufReader, BufRead, Seek};
use std::path::PathBuf;
use chrono::Utc;
use std::fs::File;
use tempfile::tempdir;
use std::collections::{HashMap, HashSet};
use wasmtime::{Store, Module, Instance, Memory, TypedFunc};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use tokio::sync::Mutex;

mod ui;

// Macro for debug output that only prints in verbose mode
macro_rules! debugln {
    ($verbose:expr, $($arg:tt)*) => {
        if $verbose {
            println!($($arg)*);
        }
    };
}

// Module for cassette generation
mod generator {
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::collections::HashMap;
    use anyhow::{Context, Result, anyhow};
    use handlebars::Handlebars;
    use serde_json::json;
    use std::process::Command;
    
    // Local macro for debug output
    macro_rules! debugln {
        ($verbose:expr, $($arg:tt)*) => {
            if $verbose {
                println!($($arg)*);
            }
        };
    }
    

    // Load template files
    const TEMPLATE_RS: &str = include_str!("templates/cassette_template.rs");
    const TEMPLATE_CARGO: &str = include_str!("templates/Cargo.toml");

    pub struct CassetteGenerator {
        output_dir: PathBuf,
        name: String,
        project_dir: PathBuf,
        template_vars: HashMap<String, String>,
        verbose: bool,
    }

    impl CassetteGenerator {
        pub fn new(
            output_dir: PathBuf,
            name: &str,
            project_dir: &Path,
        ) -> Self {
            Self {
                output_dir,
                name: name.to_string(),
                project_dir: project_dir.to_path_buf(),
                template_vars: HashMap::new(),
                verbose: false,
            }
        }

        pub fn set_var(&mut self, key: &str, value: &str) {
            self.template_vars.insert(key.to_string(), value.to_string());
        }
        
        pub fn set_verbose(&mut self, verbose: bool) {
            self.verbose = verbose;
        }

        pub fn generate(&self) -> Result<PathBuf> {
            self.generate_with_callback(None::<fn() -> Result<()>>)
        }
        
        pub fn generate_with_callback<F>(&self, progress_callback: Option<F>) -> Result<PathBuf> 
        where 
            F: FnMut() -> Result<()>
        {
            // Create src directory if it doesn't exist
            let src_dir = self.project_dir.join("src");
            fs::create_dir_all(&src_dir).context("Failed to create src directory")?;

            // Create the lib.rs file from template
            self.create_project_files(&src_dir)?;

            // Build the WASM module with progress callback
            let output_path = self.build_wasm(&self.project_dir, progress_callback)?;

            // Copy the output to the destination
            let dest_path = self.copy_output(output_path)?;
            
            Ok(dest_path)
        }

        fn create_project_files(&self, src_dir: &Path) -> Result<()> {
            // Create Handlebars instance for template rendering
            let mut handlebars = Handlebars::new();
            handlebars.set_strict_mode(true);

            // Convert template vars to JSON
            let mut template_data = json!({});
            let obj = template_data.as_object_mut().unwrap();
            
            for (key, value) in &self.template_vars {
                obj.insert(key.clone(), json!(value));
            }

            // Sanitize the cassette name for use as a Rust struct name
            // Replace hyphens with underscores and ensure it's a valid Rust identifier
            let sanitized_name = self.name.replace("-", "_").replace(" ", "_");
            obj.insert("sanitized_name".to_string(), json!(sanitized_name));
            
            // Debug: Print template data
            debugln!(self.verbose, "Debug: Template data: {}", serde_json::to_string_pretty(&template_data).unwrap_or_default());
            
            // Render the lib.rs template
            let lib_rs_content = handlebars.render_template(TEMPLATE_RS, &template_data)
                .context("Failed to render lib.rs template")?;

            // Debug: Write the rendered template to a log file
            let log_dir = PathBuf::from("../logs");
            fs::create_dir_all(&log_dir).ok(); // Ignore errors
            let log_path = log_dir.join("template_debug.rs");
            let _ = fs::write(&log_path, &lib_rs_content); // Ignore errors
            debugln!(self.verbose, "Debug: Rendered template saved to {:?}", log_path);

            // Write the lib.rs file
            let lib_rs_path = src_dir.join("lib.rs");
            let mut lib_rs_file = File::create(&lib_rs_path)
                .context("Failed to create lib.rs file")?;
            lib_rs_file.write_all(lib_rs_content.as_bytes())
                .context("Failed to write to lib.rs file")?;

            // Get the relative path to cassette-tools
            let cassette_tools_path = self.get_relative_cassette_tools_path()?;
            debugln!(self.verbose, "  Cassette tools path: {}", cassette_tools_path);

            // Create the Cargo.toml file from template
            let cargo_data = json!({
                "crate_name": self.name,
                "version": "0.1.0",
                "description": self.template_vars.get("cassette_description").unwrap_or(&"Generated Cassette".to_string()),
                "cassette_tools_path": cassette_tools_path,
                "features_array": self.template_vars.get("features_array").unwrap_or(&"[\"default\"]".to_string())
            });

            // Render the Cargo.toml template
            let cargo_content = handlebars.render_template(TEMPLATE_CARGO, &cargo_data)
                .context("Failed to render Cargo.toml template")?;

            // Write the Cargo.toml file
            let cargo_path = self.project_dir.join("Cargo.toml");
            let mut cargo_file = File::create(&cargo_path)
                .context("Failed to create Cargo.toml file")?;
            cargo_file.write_all(cargo_content.as_bytes())
                .context("Failed to write to Cargo.toml file")?;

            Ok(())
        }

        fn get_relative_cassette_tools_path(&self) -> Result<String> {
            // Use an absolute path for cassette-tools
            // We'll determine this from the current working directory
            let current_dir = std::env::current_dir()?;
            debugln!(self.verbose, "  Current directory: {}", current_dir.display());
            
            // Find the project root by traversing up until we find a marker file
            let mut project_root = current_dir.clone();
            loop {
                if project_root.join("cassette-tools").exists() {
                    // Found the project root
                    let tools_path = project_root.join("cassette-tools").display().to_string();
                    debugln!(self.verbose, "  Found cassette-tools at: {}", tools_path);
                    return Ok(tools_path);
                }
                
                if !project_root.pop() {
                    // Reached the filesystem root without finding the project root
                    break;
                }
            }
            
            // Fallback: Try a fixed path based on the current directory structure
            let mut path = std::env::current_dir()?;
            path.pop(); // Remove the temp dir name
            path.pop(); // Remove the random temp directory
            path.push("cassette-test"); // Add the project name
            path.push("cassette-tools"); // Add the tools directory
            
            let tools_path = path.display().to_string();
            
            Ok(tools_path)
        }

        fn build_wasm<F>(&self, project_dir: &Path, mut progress_callback: Option<F>) -> Result<PathBuf> 
        where 
            F: FnMut() -> Result<()>
        {
            // Change to the project directory
            let current_dir = std::env::current_dir()?;
            std::env::set_current_dir(project_dir)?;

            // Print the generated Cargo.toml for debugging
            debugln!(self.verbose, "  Using project directory: {}", project_dir.display());
            if let Ok(cargo_content) = fs::read_to_string(project_dir.join("Cargo.toml")) {
                debugln!(self.verbose, "  Generated Cargo.toml:\n{}", cargo_content);
            }

            // Run cargo build --target wasm32-unknown-unknown
            debugln!(self.verbose, "  Running cargo build...");
            
            // Build feature list for this cassette based on template features
            let mut features = vec!["nip11"]; // Always include NIP-11 for info function
            
            // Check the cassette-tools features and enable corresponding cassette features
            if let Some(features_json) = self.template_vars.get("features_array") {
                if let Ok(cassette_tools_features) = serde_json::from_str::<Vec<String>>(features_json) {
                    for feature in cassette_tools_features {
                        match feature.as_str() {
                            "nip42" => features.push("nip42"),
                            "nip45" => features.push("nip45"), 
                            "nip50" => features.push("nip50"),
                            _ => {} // Ignore other features
                        }
                    }
                }
            }
            
            // Build cargo command with features
            let features_str = features.join(",");
            let mut child = Command::new("cargo")
                .args(&["build", "--target", "wasm32-unknown-unknown", "--release", "--features", &features_str])
                .spawn()
                .context("Failed to run cargo build. Make sure Rust and the wasm32-unknown-unknown target are installed.")?;
            
            // Update UI while waiting for compilation
            if let Some(ref mut callback) = progress_callback {
                loop {
                    match child.try_wait()? {
                        Some(_) => break, // Process finished
                        None => {
                            callback()?;
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                }
            }
            
            // Wait for the process to complete
            let output = child.wait_with_output()?;

            // Change back to the original directory
            std::env::set_current_dir(current_dir)?;

            if !output.status.success() {
                debugln!(self.verbose, "Cargo build stderr: {}", String::from_utf8_lossy(&output.stderr));
                return Err(anyhow!("Failed to build WASM module. Cargo build returned error code."));
            }

            // Return the path to the generated WASM file
            let wasm_path = project_dir.join("target/wasm32-unknown-unknown/release")
                .join(format!("{}.wasm", self.name.replace("-", "_")));

            if !wasm_path.exists() {
                return Err(anyhow!("WASM module was not generated at the expected path: {:?}", wasm_path));
            }

            Ok(wasm_path)
        }

        fn copy_output(&self, wasm_path: PathBuf) -> Result<PathBuf> {
            // Create the output directory if it doesn't exist
            debugln!(self.verbose, "  Creating output directory: {:?}", self.output_dir);
            fs::create_dir_all(&self.output_dir)
                .context("Failed to create output directory")?;

            // Copy the WASM file to the output directory with a simple filename
            let dest_path = self.output_dir.join(format!("{}.wasm", self.name));
            
            // Debug output to diagnose any issues
            debugln!(self.verbose, "  Copying from: {:?}", wasm_path);
            debugln!(self.verbose, "  Copying to: {:?}", dest_path);
            
            // Check if source file exists
            if !wasm_path.exists() {
                return Err(anyhow!("Source WASM file does not exist at {:?}", wasm_path));
            }
            
            // Ensure the WASM target is installed
            let status = Command::new("rustup")
                .args(&["target", "add", "wasm32-unknown-unknown"])
                .status();
            
            if let Ok(status) = status {
                if !status.success() {
                    debugln!(self.verbose, "  ⚠️ Warning: Failed to ensure wasm32-unknown-unknown target is installed");
                }
            }
            
            // Try to copy the file with more robust error handling
            match fs::copy(&wasm_path, &dest_path) {
                Ok(_) => {
                    debugln!(self.verbose, "  ✅ Successfully copied WASM file to {:?}", dest_path);
                    Ok(dest_path)
                },
                Err(e) => {
                    debugln!(self.verbose, "  ❌ Copy failed with error: {:?}", e);
                    
                    // As a fallback, try to use the 'cp' command
                    let status = Command::new("cp")
                        .arg(&wasm_path)
                        .arg(&dest_path)
                        .status();
                        
                    match status {
                        Ok(exit) if exit.success() => {
                            debugln!(self.verbose, "  ✅ Successfully copied WASM file using cp command");
                            Ok(dest_path)
                        },
                        _ => Err(anyhow!("Failed to copy WASM file to output directory: {}", e))
                    }
                }
            }
        }
    }
}

/// Process the info command - get NIP-11 relay information
fn process_info_command(
    cassette_path: &PathBuf,
    nip11_args: &Nip11Args,
) -> Result<()> {
    // Read the WASM file
    let wasm_bytes = fs::read(cassette_path)
        .context("Failed to read cassette WASM file")?;
    
    // Initialize wasmtime
    let mut store = Store::default();
    let module = Module::from_binary(store.engine(), &wasm_bytes)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    
    // Set NIP-11 info if provided
    load_cassette_with_nip11(&mut store, &instance, nip11_args)?;
    
    // Check if the cassette exports an info function
    if let Ok(info_func) = instance.get_typed_func::<(), i32>(&mut store, "info") {
        // Call the info function
        let info_ptr = info_func.call(&mut store, ())?;
        
        if info_ptr != 0 {
            // Get memory first
            let memory = instance.get_memory(&mut store, "memory")
                .ok_or_else(|| anyhow!("Memory export not found"))?;
            
            // Read the info string
            let info_str = read_string_from_memory(&mut store, &instance, &memory, info_ptr)?;
            
            // Pretty print the JSON
            if let Ok(info_json) = serde_json::from_str::<Value>(&info_str) {
                println!("{}", serde_json::to_string_pretty(&info_json)?);
            } else {
                println!("{}", info_str);
            }
        } else {
            println!("{{}}");
        }
    } else {
        eprintln!("This cassette does not support NIP-11 (no info function found)");
    }
    
    Ok(())
}

/// Helper function to get event count for a filter using NIP-45 COUNT
fn get_event_count_for_filter(
    store: &mut Store<()>,
    instance: &Instance,
    memory: &Memory,
    alloc_func: &TypedFunc<i32, i32>,
    dealloc_func: &TypedFunc<(i32, i32), ()>,
    filter: &serde_json::Map<String, Value>,
    subscription: &str,
) -> Result<Option<u64>, anyhow::Error> {
    // Get the send function for sending COUNT
    let send_func = match instance.get_typed_func::<(i32, i32), i32>(&mut *store, "send") {
        Ok(func) => func,
        Err(_) => return Ok(None), // No send function available
    };
    
    // Create COUNT message with same filter
    let count_message = json!(["COUNT", subscription, filter]);
    let count_string = count_message.to_string();
    let count_bytes = count_string.as_bytes();
    
    // Allocate memory for COUNT request
    let count_ptr = alloc_func.call(&mut *store, count_bytes.len() as i32)?;
    if count_ptr == 0 {
        return Ok(None);
    }
    
    // Write COUNT request to memory
    memory.write(&mut *store, count_ptr as usize, count_bytes)?;
    
    // Call send function with COUNT
    let result_ptr = send_func.call(&mut *store, (count_ptr, count_bytes.len() as i32))?;
    
    // Deallocate request memory
    dealloc_func.call(&mut *store, (count_ptr, count_bytes.len() as i32))?;
    
    if result_ptr == 0 {
        return Ok(None);
    }
    
    // Read the result
    let result = read_string_from_memory(&mut *store, instance, memory, result_ptr)?;
    
    // Try to deallocate the result
    if let Ok(get_size_func) = instance.get_typed_func::<i32, i32>(&mut *store, "get_allocation_size") {
        let size = get_size_func.call(&mut *store, result_ptr)?;
        if size > 0 {
            let _ = dealloc_func.call(&mut *store, (result_ptr, size));
        }
    }
    
    // Parse COUNT response
    if let Ok(parsed) = serde_json::from_str::<Value>(&result) {
        if let Some(arr) = parsed.as_array() {
            if arr.len() >= 3 && arr[0].as_str() == Some("COUNT") {
                if let Some(count_obj) = arr.get(2) {
                    if let Some(count) = count_obj.get("count") {
                        return Ok(count.as_u64());
                    }
                }
            }
        }
    }
    
    Ok(None)
}

/// Process the REQ command - send requests to a cassette and get events
fn process_req_command(
    cassette_path: &PathBuf,
    subscription: &str,
    filter_args: &[String],
    kinds: &[i64],
    authors: &[String],
    limit: Option<usize>,
    since: Option<i64>,
    until: Option<i64>,
    output_format: &str,
    interactive: bool,
    verbose: bool,
    nip11_args: &Nip11Args,
    search_query: Option<&str>,
) -> Result<()> {
    // Initialize interactive UI if enabled
    let mut play_ui = if interactive {
        let ui = ui::play::PlayUI::new();
        ui.init()?;
        ui.show_loading(&cassette_path.display().to_string())?;
        Some(ui)
    } else {
        None
    };

    // Read the WASM file
    let wasm_bytes = fs::read(cassette_path)
        .context("Failed to read cassette WASM file")?;
    
    // Create a filter object
    let mut filter = serde_json::Map::new();
    
    // Add kinds if specified
    if !kinds.is_empty() {
        filter.insert("kinds".to_string(), json!(kinds));
    }
    
    // Add authors if specified
    if !authors.is_empty() {
        filter.insert("authors".to_string(), json!(authors));
    }
    
    // Add limit if specified
    if let Some(l) = limit {
        filter.insert("limit".to_string(), json!(l));
    }
    
    // Add time filters if specified
    if let Some(s) = since {
        filter.insert("since".to_string(), json!(s));
    }
    if let Some(u) = until {
        filter.insert("until".to_string(), json!(u));
    }
    
    // Add search query if specified (NIP-50)
    if let Some(search) = search_query {
        filter.insert("search".to_string(), json!(search));
    }
    
    // Parse any custom filter JSON arguments
    for filter_json in filter_args {
        let parsed: serde_json::Map<String, Value> = serde_json::from_str(filter_json)
            .context("Failed to parse filter JSON")?;
        filter.extend(parsed);
    }
    
    // Create the REQ message
    let req_message = json!(["REQ", subscription, filter]);
    let req_string = req_message.to_string();
    
    // Initialize wasmtime
    let mut store = Store::default();
    let module = Module::from_binary(store.engine(), &wasm_bytes)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    
    // Set NIP-11 info if provided
    load_cassette_with_nip11(&mut store, &instance, nip11_args)?;
    
    // Get memory export
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow!("Memory export not found"))?;
    
    // Get the send function
    let send_func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "send")
        .context("Failed to get send function")?;
    
    // Get allocation function
    let alloc_func = instance
        .get_typed_func::<i32, i32>(&mut store, "alloc_buffer")
        .or_else(|_| instance.get_typed_func::<i32, i32>(&mut store, "alloc_string"))
        .context("Failed to get allocation function")?;
    
    // Get deallocation function
    let dealloc_func = instance
        .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string")
        .context("Failed to get deallocation function")?;
    
    // Try to get total count first for progress bar (NIP-45)
    let total_count = get_event_count_for_filter(&mut store, &instance, &memory, &alloc_func, &dealloc_func, &filter, subscription).unwrap_or(None);
    
    // Allocate memory for the request string
    let req_bytes = req_string.as_bytes();
    // Collect all events in a loop
    let mut all_events = Vec::new();
    let mut event_count = 0u64;
    
    loop {
        // Allocate memory for the request string for each call
        let req_ptr = alloc_func.call(&mut store, req_bytes.len() as i32)?;
        
        if req_ptr == 0 {
            return Err(anyhow!("Failed to allocate memory for request"));
        }
        
        // Write request to memory
        memory.write(&mut store, req_ptr as usize, req_bytes)?;
        
        // Call the send function
        let result_ptr = send_func.call(&mut store, (req_ptr, req_bytes.len() as i32))?;
        
        // Deallocate request memory immediately after use
        dealloc_func.call(&mut store, (req_ptr, req_bytes.len() as i32))?;
        
        if result_ptr == 0 {
            break; // No more events
        }
        
        // Read the result
        let result = read_string_from_memory(&mut store, &instance, &memory, result_ptr)?;
        
        // Try to deallocate the result pointer
        if let Ok(get_size_func) = instance.get_typed_func::<i32, i32>(&mut store, "get_allocation_size") {
            let size = get_size_func.call(&mut store, result_ptr)?;
            if size > 0 && dealloc_func.call(&mut store, (result_ptr, size)).is_ok() {
                // Successfully deallocated result
            }
        }
        
        // Parse the result
        let parsed_result: Value = serde_json::from_str(&result)?;
        
        if let Some(arr) = parsed_result.as_array() {
            if arr.len() >= 2 {
                match arr[0].as_str() {
                    Some("EVENT") => {
                        if arr.len() >= 3 {
                            event_count += 1;
                            all_events.push(arr[2].clone());
                            
                            // Update interactive UI
                            if let Some(ref mut ui) = play_ui {
                                let total_for_ui = total_count.unwrap_or(all_events.len() as u64);
                                ui.update_playback(total_for_ui, event_count, Some(&arr[2]))?;
                                std::thread::sleep(std::time::Duration::from_millis(50));
                            }
                        }
                    }
                    Some("EOSE") => {
                        break; // End of events
                    }
                    Some("NOTICE") => {
                        // Check for "No more events" message
                        if result.contains("No more events") {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    // Handle completion and output
    if let Some(ui) = play_ui {
        // Interactive mode - show completion screen
        ui.show_completion(all_events.len() as u64)?;
        
        // Wait for user input
        use crossterm::event::{self, Event, KeyCode};
        use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
        
        enable_raw_mode()?;
        loop {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    _ => {}
                }
            }
        }
        disable_raw_mode()?;
        
        ui.cleanup()?;
    } else {
        // Non-interactive mode - just output results
        match output_format {
            "ndjson" => {
                for event in &all_events {
                    println!("{}", serde_json::to_string(&event)?);
                }
            }
            _ => {
                if all_events.len() == 1 {
                    println!("{}", serde_json::to_string_pretty(&all_events[0])?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&all_events)?);
                }
            }
        }
    }
    
    Ok(())
}

/// Process the COUNT command - send COUNT requests to a cassette and get event counts
fn process_count_command(
    cassette_path: &PathBuf,
    subscription: &str,
    filter_args: &[String],
    kinds: &[i64],
    authors: &[String],
    limit: Option<usize>,
    since: Option<i64>,
    until: Option<i64>,
    verbose: bool,
    nip11_args: &Nip11Args,
    search_query: Option<&str>,
) -> Result<()> {
    // Read the WASM file
    let wasm_bytes = fs::read(cassette_path)
        .context("Failed to read cassette WASM file")?;
    
    // Create a filter object
    let mut filter = serde_json::Map::new();
    
    // Add kinds if specified
    if !kinds.is_empty() {
        filter.insert("kinds".to_string(), json!(kinds));
    }
    
    // Add authors if specified
    if !authors.is_empty() {
        filter.insert("authors".to_string(), json!(authors));
    }
    
    // Add limit if specified
    if let Some(l) = limit {
        filter.insert("limit".to_string(), json!(l));
    }
    
    // Add time filters if specified
    if let Some(s) = since {
        filter.insert("since".to_string(), json!(s));
    }
    if let Some(u) = until {
        filter.insert("until".to_string(), json!(u));
    }
    
    // Add search query if specified (NIP-50)
    if let Some(search) = search_query {
        filter.insert("search".to_string(), json!(search));
    }
    
    // Parse any custom filter JSON arguments
    for filter_json in filter_args {
        let parsed: serde_json::Map<String, Value> = serde_json::from_str(filter_json)
            .context("Failed to parse filter JSON")?;
        filter.extend(parsed);
    }
    
    // Create the COUNT message
    let count_message = json!(["COUNT", subscription, filter]);
    let count_string = count_message.to_string();
    
    debugln!(verbose, "Sending COUNT request: {}", count_string);
    
    // Initialize wasmtime
    let mut store = Store::default();
    let module = Module::from_binary(store.engine(), &wasm_bytes)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    
    // Set NIP-11 info if provided
    load_cassette_with_nip11(&mut store, &instance, nip11_args)?;
    
    // Get memory export
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow!("Memory export not found"))?;
    
    // Get the send function (COUNT also uses the send function)
    let send_func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "send")
        .context("Failed to get send function")?;
    
    // Get allocation function
    let alloc_func = instance
        .get_typed_func::<i32, i32>(&mut store, "alloc_buffer")
        .or_else(|_| instance.get_typed_func::<i32, i32>(&mut store, "alloc_string"))
        .context("Failed to get allocation function")?;
    
    // Get deallocation function
    let dealloc_func = instance
        .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string")
        .context("Failed to get deallocation function")?;
    
    // Allocate memory for the COUNT request
    let count_bytes = count_string.as_bytes();
    let count_ptr = alloc_func.call(&mut store, count_bytes.len() as i32)?;
    
    if count_ptr == 0 {
        return Err(anyhow!("Failed to allocate memory for COUNT request"));
    }
    
    // Write COUNT request to memory
    memory.write(&mut store, count_ptr as usize, count_bytes)?;
    
    // Call the send function with COUNT message
    let result_ptr = send_func.call(&mut store, (count_ptr, count_bytes.len() as i32))?;
    
    // Deallocate request memory
    dealloc_func.call(&mut store, (count_ptr, count_bytes.len() as i32))?;
    
    if result_ptr == 0 {
        return Err(anyhow!("No response from COUNT request"));
    }
    
    // Read the result
    let result = read_string_from_memory(&mut store, &instance, &memory, result_ptr)?;
    
    // Try to deallocate the result pointer
    if let Ok(get_size_func) = instance.get_typed_func::<i32, i32>(&mut store, "get_allocation_size") {
        let size = get_size_func.call(&mut store, result_ptr)?;
        if size > 0 && dealloc_func.call(&mut store, (result_ptr, size)).is_ok() {
            // Successfully deallocated result
        }
    }
    
    debugln!(verbose, "COUNT response: {}", result);
    
    // Parse and pretty print the result
    let parsed_result: Value = serde_json::from_str(&result)
        .context("Failed to parse COUNT response")?;
    
    // Verify it's a COUNT response
    if let Some(arr) = parsed_result.as_array() {
        if arr.len() >= 3 && arr[0].as_str() == Some("COUNT") {
            // Print the count object (third element)
            if let Some(count_obj) = arr.get(2) {
                println!("{}", serde_json::to_string_pretty(count_obj)?);
            } else {
                println!("{{\"count\": 0}}");
            }
        } else {
            // Unexpected response format, print as-is
            println!("{}", serde_json::to_string_pretty(&parsed_result)?);
        }
    } else {
        return Err(anyhow!("Invalid COUNT response format"));
    }
    
    Ok(())
}

/// Process the DUB command - combine multiple cassettes into a new one
fn process_dub_command(
    cassette_paths: &[PathBuf],
    output_path: &PathBuf,
    name: Option<&str>,
    description: Option<&str>,
    author: Option<&str>,
    filter_args: &[String],
    kinds: &[i64],
    authors: &[String],
    limit: Option<usize>,
    since: Option<i64>,
    until: Option<i64>,
    interactive: bool,
    verbose: bool,
    nip11_args: &Nip11Args,
) -> Result<()> {
    if cassette_paths.is_empty() {
        return Err(anyhow!("No input cassettes specified"));
    }
    
    // Initialize interactive UI if enabled
    let mut dub_ui = if interactive {
        let cassette_names: Vec<String> = cassette_paths.iter()
            .map(|p| p.file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string())
            .collect();
        let ui = ui::dub::DubUI::new(cassette_paths.len(), cassette_names);
        ui.init()?;
        ui.show_loading()?;
        std::thread::sleep(std::time::Duration::from_secs(2));
        Some(ui)
    } else {
        None
    };
    
    debugln!(verbose, "=== Cassette CLI - Dub Command ===");
    debugln!(verbose, "Combining {} cassettes...", cassette_paths.len());
    
    // Collect all events from all cassettes
    let mut all_events = Vec::new();
    
    for (idx, cassette_path) in cassette_paths.iter().enumerate() {
        debugln!(verbose, "\n📼 Processing cassette {}/{}: {}", 
            idx + 1, 
            cassette_paths.len(), 
            cassette_path.display()
        );
        
        if !cassette_path.exists() {
            return Err(anyhow!("Cassette file not found: {}", cassette_path.display()));
        }
        
        // Read the WASM file
        let wasm_bytes = fs::read(cassette_path)
            .context("Failed to read cassette WASM file")?;
        
        // Initialize wasmtime
        let mut store = Store::default();
        let module = Module::from_binary(store.engine(), &wasm_bytes)?;
        let instance = Instance::new(&mut store, &module, &[])?;
        
        // Set NIP-11 info if provided
        load_cassette_with_nip11(&mut store, &instance, nip11_args)?;
        
        // Get memory export
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("Memory export not found"))?;
        
        // Get the send function
        let send_func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "send")
            .context("Failed to get send function")?;
        
        // Get allocation function
        let alloc_func = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc_buffer")
            .or_else(|_| instance.get_typed_func::<i32, i32>(&mut store, "alloc_string"))
            .context("Failed to get allocation function")?;
        
        // Get deallocation function
        let dealloc_func = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string")
            .context("Failed to get deallocation function")?;
        
        // Create a REQ message to get all events from this cassette
        let req_message = json!(["REQ", "dub_extract", {}]);
        let req_string = req_message.to_string();
        let req_bytes = req_string.as_bytes();
        
        // Keep calling req until we get EOSE
        let mut cassette_events = Vec::new();
        let mut first_call = true;
        
        loop {
            // Allocate memory for the request string for each call
            let req_ptr = alloc_func.call(&mut store, req_bytes.len() as i32)?;
            
            if req_ptr == 0 {
                return Err(anyhow!("Failed to allocate memory for request"));
            }
            
            // Write request to memory
            memory.write(&mut store, req_ptr as usize, req_bytes)?;
            
            // Call the send function
            let result_ptr = send_func.call(&mut store, (req_ptr, req_bytes.len() as i32))?;
            
            // Deallocate request memory immediately after use
            dealloc_func.call(&mut store, (req_ptr, req_bytes.len() as i32))?;
            
            if result_ptr == 0 {
                if first_call {
                    println!("  No events found in cassette");
                }
                break; // No more events
            }
            
            first_call = false;
            
            // Read the result
            let result = read_string_from_memory(&mut store, &instance, &memory, result_ptr)?;
            
            // We might need to deallocate the result pointer too
            // Check if there's a get_allocation_size function
            if let Ok(get_size_func) = instance.get_typed_func::<i32, i32>(&mut store, "get_allocation_size") {
                let size = get_size_func.call(&mut store, result_ptr)?;
                if size > 0 && dealloc_func.call(&mut store, (result_ptr, size)).is_ok() {
                    // Successfully deallocated result
                }
            }
            
            // Parse the result
            let parsed_result: Value = serde_json::from_str(&result)?;
            
            if let Some(arr) = parsed_result.as_array() {
                if arr.len() >= 2 {
                    match arr[0].as_str() {
                        Some("EVENT") => {
                            if arr.len() >= 3 {
                                cassette_events.push(arr[2].clone());
                                
                                // Update interactive UI
                                if let Some(ref mut ui) = dub_ui {
                                    ui.update_processing(idx, cassette_events.len() as u64, all_events.len() as u64 + cassette_events.len() as u64)?;
                                    std::thread::sleep(std::time::Duration::from_millis(30));
                                }
                            }
                        }
                        Some("EOSE") => {
                            break; // End of events
                        }
                        Some("NOTICE") => {
                            // Just continue, might be "No more events"
                            if result.contains("No more events") {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        debugln!(verbose, "  Found {} events", cassette_events.len());
        all_events.extend(cassette_events);
    }
    
    debugln!(verbose, "\n📊 Total events collected: {}", all_events.len());
    
    // Show mixing phase in interactive mode
    if let Some(ref ui) = dub_ui {
        ui.show_mixing(all_events.len() as u64)?;
    }
    
    // Apply filters if specified
    if !kinds.is_empty() || !authors.is_empty() || !filter_args.is_empty() || since.is_some() || until.is_some() {
        debugln!(verbose, "\n🔍 Applying filters...");
        
        let mut filtered_events = Vec::new();
        
        // Create a filter object
        let mut filter = serde_json::Map::new();
        
        if !kinds.is_empty() {
            filter.insert("kinds".to_string(), json!(kinds));
        }
        
        if !authors.is_empty() {
            filter.insert("authors".to_string(), json!(authors));
        }
        
        if let Some(l) = limit {
            filter.insert("limit".to_string(), json!(l));
        }
        
        if let Some(s) = since {
            filter.insert("since".to_string(), json!(s));
        }
        
        if let Some(u) = until {
            filter.insert("until".to_string(), json!(u));
        }
        
        // Parse any custom filter JSON arguments
        for filter_json in filter_args {
            let parsed: serde_json::Map<String, Value> = serde_json::from_str(filter_json)
                .context("Failed to parse filter JSON")?;
            filter.extend(parsed);
        }
        
        // Apply the filter to each event
        for event in all_events {
            if event_matches_filter(&event, &filter) {
                filtered_events.push(event);
            }
        }
        
        debugln!(verbose, "  Events after filtering: {}", filtered_events.len());
        all_events = filtered_events;
    }
    
    // Apply limit if specified and not already applied via filter
    if let Some(l) = limit {
        if all_events.len() > l {
            debugln!(verbose, "  Applying limit of {} events", l);
            all_events.truncate(l);
        }
    }
    
    // Preprocess events to handle replaceable events
    debugln!(verbose, "\n🔍 Preprocessing events according to NIP-01...");
    let processed_events = preprocess_events(all_events);
    debugln!(verbose, "  Final event count: {}", processed_events.len());
    
    // Generate the new cassette
    let cassette_name = name.unwrap_or("dubbed_cassette");
    let cassette_desc = description.unwrap_or("Combined cassette created by dubbing multiple cassettes");
    let cassette_author = author.unwrap_or("Cassette CLI Dub");
    
    // Create a temporary directory for building
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("dubbed_events.json");
    
    // Write events to temp file
    let events_json = serde_json::to_string(&processed_events)?;
    fs::write(&temp_file, &events_json)?;
    
    // Get the output directory from the output path
    let output_dir = output_path.parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    
    // Process events to create the new cassette
    process_events(
        temp_file.to_str().unwrap(),
        cassette_name,
        cassette_desc,
        cassette_author,
        &output_dir,
        false, // no_bindings
        false, // interactive
        false, // verbose
        false, // nip_11
        false, // nip_42
        false, // nip_45
        false, // nip_50
        nip11_args
    )?;
    
    // Rename the generated file to the specified output name if needed
    let generated_path = output_dir.join(format!("{}.wasm", cassette_name));
    if generated_path != *output_path {
        fs::rename(&generated_path, output_path)
            .context("Failed to rename output file")?;
    }
    
    // Handle completion
    if let Some(ui) = dub_ui {
        // Interactive mode - show completion screen
        ui.show_completion(&output_path.display().to_string(), processed_events.len() as u64)?;
        
        // Wait for user input
        use crossterm::event::{self, Event, KeyCode};
        use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
        
        enable_raw_mode()?;
        loop {
            if let Ok(Event::Key(key)) = event::read() {
                if matches!(key.code, KeyCode::Char(_) | KeyCode::Enter | KeyCode::Esc) {
                    break;
                }
            }
        }
        disable_raw_mode()?;
        
        ui.cleanup()?;
    } else {
        debugln!(verbose, "\n✅ Dubbed cassette saved to: {}", output_path.display());
    }
    
    Ok(())
}

/// Helper function to check if an event matches a filter
fn event_matches_filter(event: &Value, filter: &serde_json::Map<String, Value>) -> bool {
    // Check kinds
    if let Some(kinds) = filter.get("kinds").and_then(|k| k.as_array()) {
        if let Some(event_kind) = event.get("kind").and_then(|k| k.as_i64()) {
            let kind_match = kinds.iter().any(|k| k.as_i64() == Some(event_kind));
            if !kind_match {
                return false;
            }
        }
    }
    
    // Check authors
    if let Some(authors) = filter.get("authors").and_then(|a| a.as_array()) {
        if let Some(event_author) = event.get("pubkey").and_then(|p| p.as_str()) {
            let author_match = authors.iter().any(|a| a.as_str() == Some(event_author));
            if !author_match {
                return false;
            }
        }
    }
    
    // Check timestamps
    if let Some(since) = filter.get("since").and_then(|s| s.as_i64()) {
        if let Some(created_at) = event.get("created_at").and_then(|t| t.as_i64()) {
            if created_at < since {
                return false;
            }
        }
    }
    
    if let Some(until) = filter.get("until").and_then(|u| u.as_i64()) {
        if let Some(created_at) = event.get("created_at").and_then(|t| t.as_i64()) {
            if created_at > until {
                return false;
            }
        }
    }
    
    // Check tag filters
    for (key, values) in filter {
        if key.starts_with('#') && key.len() == 2 {
            let tag_name = &key[1..];
            if let Some(tags) = event.get("tags").and_then(|t| t.as_array()) {
                let has_matching_tag = tags.iter().any(|tag| {
                    if let Some(tag_arr) = tag.as_array() {
                        if tag_arr.len() >= 2 {
                            if let (Some(t_name), Some(t_value)) = (tag_arr[0].as_str(), tag_arr[1].as_str()) {
                                if t_name == tag_name {
                                    if let Some(filter_values) = values.as_array() {
                                        return filter_values.iter().any(|v| v.as_str() == Some(t_value));
                                    }
                                }
                            }
                        }
                    }
                    false
                });
                
                if !has_matching_tag {
                    return false;
                }
            }
        }
    }
    
    true
}

/// Read a string from WASM memory using MSGB format
fn read_string_from_memory(
    store: &mut Store<()>,
    _instance: &Instance,
    memory: &Memory,
    ptr: i32,
) -> Result<String> {
    // Check for MSGB signature
    let mut sig_bytes = [0u8; 4];
    memory.read(&mut *store, ptr as usize, &mut sig_bytes)?;
    
    if &sig_bytes == b"MSGB" {
        // Read length
        let mut len_bytes = [0u8; 4];
        memory.read(&mut *store, (ptr + 4) as usize, &mut len_bytes)?;
        let length = u32::from_le_bytes(len_bytes) as usize;
        
        // Read string data
        let mut string_data = vec![0u8; length];
        memory.read(&mut *store, (ptr + 8) as usize, &mut string_data)?;
        
        String::from_utf8(string_data).context("Invalid UTF-8 in response")
    } else {
        // Fallback: read null-terminated string
        let mem_data = memory.data(&*store);
        let start = ptr as usize;
        let mut end = start;
        
        while end < mem_data.len() && mem_data[end] != 0 {
            end += 1;
        }
        
        String::from_utf8(mem_data[start..end].to_vec()).context("Invalid UTF-8 in response")
    }
}

#[derive(Parser)]
#[command(author, version, about = "CLI tool for Cassette platform")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Common NIP-11 arguments for commands that load cassettes
#[derive(clap::Args, Clone)]
struct Nip11Args {
    /// Name for NIP-11
    #[arg(long = "name")]
    relay_name: Option<String>,
    
    /// Description for NIP-11
    #[arg(long = "description")]
    relay_description: Option<String>,
    
    /// Owner pubkey for NIP-11
    #[arg(long = "pubkey")]
    relay_pubkey: Option<String>,
    
    /// Contact for NIP-11
    #[arg(long = "contact")]
    relay_contact: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Record Nostr events from a file or piped input to create a cassette
    Record {
        /// Path to input events.json file (if not provided, reads from stdin)
        input_file: Option<PathBuf>,

        /// Name for the generated cassette
        #[arg(short, long)]
        name: Option<String>,

        /// Description for the generated cassette
        #[arg(short, long)]
        description: Option<String>,

        /// Author of the cassette
        #[arg(short, long)]
        author: Option<String>,

        /// Output directory for the generated WASM module
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Whether to actually generate the WASM module (default: true)
        #[arg(long, default_value = "true")]
        generate: bool,

        /// Skip JavaScript bindings generation, distribute only the .wasm file
        #[arg(
            long = "no-bindings",
            help = "Skip JavaScript bindings generation, distribute only the .wasm file",
            action = clap::ArgAction::SetTrue
        )]
        no_bindings: bool,
        /// Enable interactive mode with visual feedback
        #[arg(short = 'i', long)]
        interactive: bool,
        /// Show verbose output including compilation details
        #[arg(short, long)]
        verbose: bool,
        
        /// Enable NIP-11 (Relay Information Document)
        #[arg(long)]
        nip_11: bool,
        
        /// Enable NIP-42 (Authentication)
        #[arg(long)]
        nip_42: bool,
        
        /// Enable NIP-45 (Event Counts)
        #[arg(long)]
        nip_45: bool,
        
        /// Enable NIP-50 (Search Capability)
        #[arg(long)]
        nip_50: bool,
        
        #[command(flatten)]
        nip11: Nip11Args,
    },
    
    /// Combine multiple cassettes into a new cassette (dubbing/mixing)
    Dub {
        /// Input cassette files to combine
        cassettes: Vec<PathBuf>,
        
        /// Output cassette file path
        output: PathBuf,
        
        /// Name for the generated cassette
        #[arg(short, long)]
        name: Option<String>,
        
        /// Description for the generated cassette
        #[arg(short, long)]
        description: Option<String>,
        
        /// Author of the cassette
        #[arg(short, long)]
        author: Option<String>,
        
        /// Filter JSON (can be specified multiple times)
        #[arg(short, long, value_name = "JSON")]
        filter: Vec<String>,
        
        /// Kinds to filter (can be specified multiple times)
        #[arg(short, long)]
        kinds: Vec<i64>,
        
        /// Authors to filter (can be specified multiple times)
        #[arg(long)]
        authors: Vec<String>,
        
        /// Limit number of events
        #[arg(short, long)]
        limit: Option<usize>,
        
        /// Since timestamp
        #[arg(long)]
        since: Option<i64>,
        
        /// Until timestamp
        #[arg(long)]
        until: Option<i64>,
        /// Enable interactive mode with visual feedback
        #[arg(short = 'i', long)]
        interactive: bool,
        /// Show verbose output including compilation details
        #[arg(short, long)]
        verbose: bool,
        
        #[command(flatten)]
        nip11: Nip11Args,
    },
    
    /// Send a REQ message to a cassette and get events
    Play {
        /// Path to the cassette WASM file
        cassette: PathBuf,
        
        /// Subscription ID
        #[arg(short, long, default_value = "sub1")]
        subscription: String,
        
        /// Filter JSON (can be specified multiple times)
        #[arg(short, long, value_name = "JSON")]
        filter: Vec<String>,
        
        /// Kinds to filter (can be specified multiple times)
        #[arg(short, long)]
        kinds: Vec<i64>,
        
        /// Authors to filter (can be specified multiple times)
        #[arg(short, long)]
        authors: Vec<String>,
        
        /// Limit number of events
        #[arg(short, long)]
        limit: Option<usize>,
        
        /// Since timestamp
        #[arg(long)]
        since: Option<i64>,
        
        /// Until timestamp
        #[arg(long)]
        until: Option<i64>,
        
        /// Output format: json (default) or ndjson
        #[arg(short, long, default_value = "json")]
        output: String,
        /// Enable interactive mode with visual feedback
        #[arg(short = 'i', long)]
        interactive: bool,
        /// Show verbose output including compilation details
        #[arg(short, long)]
        verbose: bool,
        
        /// Show NIP-11 relay information instead of playing events
        #[arg(long)]
        info: bool,
        
        /// Perform COUNT query instead of REQ (NIP-45)
        #[arg(long)]
        count: bool,
        
        /// Search query for NIP-50 text search
        #[arg(long)]
        search: Option<String>,
        
        #[command(flatten)]
        nip11: Nip11Args,
    },
    
    /// Cast events from cassettes to Nostr relays
    Cast {
        /// Input cassette files to broadcast
        cassettes: Vec<PathBuf>,
        
        /// Target relay URLs
        #[arg(short, long, required = true)]
        relays: Vec<String>,
        
        /// Maximum concurrent relay connections
        #[arg(short, long, default_value = "5")]
        concurrency: usize,
        
        /// Delay between event publishes in milliseconds (per relay)
        #[arg(short, long, default_value = "100")]
        throttle: u64,
        
        /// Timeout for relay connections in seconds
        #[arg(long, default_value = "30")]
        timeout: u64,
        
        /// Dry run - show what would be sent without actually sending
        #[arg(long)]
        dry_run: bool,
        /// Enable interactive mode with visual feedback
        #[arg(short = 'i', long)]
        interactive: bool,
        /// Show verbose output including compilation details
        #[arg(short, long)]
        verbose: bool,
        
        #[command(flatten)]
        nip11: Nip11Args,
    },
}

/// Helper function to load a cassette and set its NIP-11 info if available
fn load_cassette_with_nip11(
    store: &mut Store<()>,
    instance: &Instance,
    nip11_args: &Nip11Args,
) -> Result<()> {
    // Check if the cassette exports set_relay_info function
    if let Ok(set_relay_info) = instance.get_typed_func::<(i32, i32), i32>(&mut *store, "set_relay_info") {
        // Build RelayInfo from CLI arguments
        let mut relay_info = serde_json::Map::new();
        
        // Add required fields 
        relay_info.insert("supported_nips".to_string(), json!(Vec::<u32>::new()));
        
        // Add optional fields if provided
        if let Some(name) = &nip11_args.relay_name {
            relay_info.insert("name".to_string(), json!(name));
        }
        if let Some(desc) = &nip11_args.relay_description {
            relay_info.insert("description".to_string(), json!(desc));
        }
        if let Some(pubkey) = &nip11_args.relay_pubkey {
            relay_info.insert("pubkey".to_string(), json!(pubkey));
        }
        if let Some(contact) = &nip11_args.relay_contact {
            relay_info.insert("contact".to_string(), json!(contact));
        }
        
        // Serialize to JSON
        let json_str = serde_json::to_string(&relay_info)?;
        let json_bytes = json_str.as_bytes();
        
        // Get memory and allocation function
        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| anyhow!("Memory export not found"))?;
        
        let alloc_func = instance
            .get_typed_func::<i32, i32>(&mut *store, "alloc_buffer")
            .or_else(|_| instance.get_typed_func::<i32, i32>(&mut *store, "alloc_string"))?;
        
        // Allocate memory for the JSON string
        let json_ptr = alloc_func.call(&mut *store, json_bytes.len() as i32)?;
        if json_ptr == 0 {
            return Err(anyhow!("Failed to allocate memory for NIP-11 info"));
        }
        
        // Write JSON to memory
        memory.write(&mut *store, json_ptr as usize, json_bytes)?;
        
        // Call set_relay_info
        let result = set_relay_info.call(&mut *store, (json_ptr, json_bytes.len() as i32))?;
        
        // Clean up allocated memory
        if let Ok(dealloc_func) = instance.get_typed_func::<(i32, i32), ()>(&mut *store, "dealloc_string") {
            dealloc_func.call(&mut *store, (json_ptr, json_bytes.len() as i32))?;
        }
        
        if result != 0 {
            eprintln!("Warning: Failed to set NIP-11 info (error code: {})", result);
        }
    }
    
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Record { 
            input_file, 
            name, 
            description, 
            author, 
            output,
            generate: _,
            no_bindings,
            interactive,
            verbose,
            nip_11,
            nip_42,
            nip_45,
            nip_50,
            nip11
        } => {
            // Set default values if not provided
            let name_value = name.clone().unwrap_or_else(|| "cassette".to_string());
            let desc_value = description.clone().unwrap_or_else(|| "Generated cassette".to_string());
            let author_value = author.clone().unwrap_or_else(|| "Cassette CLI".to_string());
            let output_value = output.clone().unwrap_or_else(|| PathBuf::from("./cassettes"));
            
            // Either process from file or stdin
            if let Some(path) = input_file {
                if !path.exists() {
                    return Err(anyhow!("Input file doesn't exist: {}", path.display()));
                }
                
                process_events(
                    path.to_str().unwrap(),
                    &name_value,
                    &desc_value,
                    &author_value,
                    &output_value,
                    *no_bindings,
                    *interactive,
                    *verbose,
                    *nip_11,
                    *nip_42,
                    *nip_45,
                    *nip_50,
                    nip11
                )?;
            } else {
                // No input file, read from stdin
                println!("Reading events from stdin...");
                
                // Create a temp file to store stdin content
                let temp_dir = tempdir()?;
                let temp_file_path = temp_dir.path().join("stdin_events.json");
                let temp_file_path_str = temp_file_path.to_str().unwrap();
                
                // Read from stdin line by line (for NDJSON from nak)
                let stdin = std::io::stdin();
                let mut temp_file = File::create(&temp_file_path)?;
                
                for line in stdin.lock().lines() {
                    let line = line?;
                    if !line.trim().is_empty() {
                        writeln!(temp_file, "{}", line)?;
                    }
                }
                
                // Ensure file has content
                temp_file.flush()?;
                let metadata = std::fs::metadata(&temp_file_path)?;
                if metadata.len() == 0 {
                    return Err(anyhow!("No data received from stdin. Please pipe in events or use an input file."));
                }
                
                // Process the temp file
                process_events(
                    temp_file_path_str,
                    &name_value,
                    &desc_value,
                    &author_value,
                    &output_value,
                    *no_bindings,
                    *interactive,
                    *verbose,
                    *nip_11,
                    *nip_42,
                    *nip_45,
                    *nip_50,
                    nip11
                )?;
                
                // The temp directory will be cleaned up when it goes out of scope
            }
            
            Ok(())
        }
        Commands::Dub {
            cassettes,
            output,
            name,
            description,
            author,
            filter,
            kinds,
            authors,
            limit,
            since,
            until,
            interactive,
            verbose,
            nip11,
        } => {
            process_dub_command(
                cassettes,
                output,
                name.as_deref(),
                description.as_deref(),
                author.as_deref(),
                filter,
                kinds,
                authors,
                *limit,
                *since,
                *until,
                *interactive,
                *verbose,
                nip11,
            )
        }
        Commands::Play {
            cassette,
            subscription,
            filter,
            kinds,
            authors,
            limit,
            since,
            until,
            output,
            interactive,
            verbose,
            info,
            count,
            search,
            nip11,
        } => {
            if *info {
                // Just show NIP-11 info
                process_info_command(cassette, nip11)
            } else if *count {
                // Perform COUNT query
                process_count_command(
                    cassette,
                    subscription,
                    filter,
                    kinds,
                    authors,
                    *limit,
                    *since,
                    *until,
                    *verbose,
                    nip11,
                    search.as_deref(),
                )
            } else {
                process_req_command(
                    cassette,
                    subscription,
                    filter,
                    kinds,
                    authors,
                    *limit,
                    *since,
                    *until,
                    output,
                    *interactive,
                    *verbose,
                    nip11,
                    search.as_deref(),
                )
            }
        }
        Commands::Cast {
            cassettes,
            relays,
            concurrency,
            throttle,
            timeout,
            dry_run,
            interactive: _,
            verbose: _,
            nip11,
        } => {
            // Use tokio runtime for async operations
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(process_cast_command(
                cassettes,
                relays,
                *concurrency,
                *throttle,
                *timeout,
                *dry_run,
                nip11,
            ))
        }
    }
}

/// Preprocess events to handle replaceable and addressable replaceable events according to NIP-01
/// Returns a filtered list of events with only the latest version of each replaceable event
fn preprocess_events(events: Vec<Value>) -> Vec<Value> {
    // Maps to track the latest versions of replaceable events
    // For replaceable events (10000-19999): Key format: "{pubkey}:{kind}"
    // For addressable replaceable events (30000-39999): Key format: "{pubkey}:{kind}:{d_tag_value}"
    let mut replaceable_events: HashMap<String, (usize, i64)> = HashMap::new();
    let mut duplicates_removed = 0;
    let mut total_replaceable = 0;
    let mut total_addressable = 0;
    
    // First pass: identify all replaceable events and track their indices
    for (i, event) in events.iter().enumerate() {
        if let (Some(kind), Some(pubkey), Some(created_at)) = (
            event.get("kind").and_then(|k| k.as_i64()),
            event.get("pubkey").and_then(|p| p.as_str()),
            event.get("created_at").and_then(|t| t.as_i64())
        ) {
            // Handle replaceable events (kinds 10000-19999)
            if kind >= 10000 && kind <= 19999 {
                total_replaceable += 1;
                let key = format!("{}:{}", pubkey, kind);
                
                // Check if we already have this event type
                if let Some((_, existing_ts)) = replaceable_events.get(&key) {
                    if created_at > *existing_ts {
                        // This is newer, replace the existing one
                        replaceable_events.insert(key, (i, created_at));
                    }
                    duplicates_removed += 1;
                } else {
                    // First time seeing this event type
                    replaceable_events.insert(key, (i, created_at));
                }
            }
            // Handle addressable replaceable events (kinds 30000-39999)
            else if kind >= 30000 && kind <= 39999 {
                total_addressable += 1;
                // Find the 'd' tag value
                let d_tag_value = event.get("tags")
                    .and_then(|tags| tags.as_array())
                    .and_then(|tags_array| {
                        tags_array.iter()
                            .find(|tag| tag.as_array()
                                .and_then(|t| t.get(0).and_then(|t0| t0.as_str()))
                                .unwrap_or("") == "d"
                            )
                            .and_then(|tag| tag.as_array())
                            .and_then(|tag_array| tag_array.get(1).and_then(|t1| t1.as_str()))
                    })
                    .unwrap_or("");
                
                let key = format!("{}:{}:{}", pubkey, kind, d_tag_value);
                
                // Check if we already have this event type
                if let Some((_, existing_ts)) = replaceable_events.get(&key) {
                    if created_at > *existing_ts {
                        // This is newer, replace the existing one
                        replaceable_events.insert(key, (i, created_at));
                    }
                    duplicates_removed += 1;
                } else {
                    // First time seeing this event type
                    replaceable_events.insert(key, (i, created_at));
                }
            }
        }
    }
    
    // If no replaceable events were found, return the original list
    if replaceable_events.is_empty() {
        return events;
    }
    
    // Second pass: build a new list with only the latest events
    let mut filtered_events = Vec::with_capacity(events.len() - duplicates_removed);
    let indices_to_keep: Vec<usize> = replaceable_events.values()
        .map(|(index, _)| *index)
        .collect();
    
    for (i, event) in events.into_iter().enumerate() {
        // For regular events or the latest version of replaceable events
        if let Some(kind) = event.get("kind").and_then(|k| k.as_i64()) {
            if (kind >= 10000 && kind <= 19999) || (kind >= 30000 && kind <= 39999) {
                // Only keep this replaceable event if it's in our indices_to_keep list
                if indices_to_keep.contains(&i) {
                    filtered_events.push(event);
                }
            } else {
                // Non-replaceable event, keep it
                filtered_events.push(event);
            }
        } else {
            // If kind is missing, keep the event
            filtered_events.push(event);
        }
    }
    
    println!("  Found {} replaceable events (kinds 10000-19999)", total_replaceable);
    println!("  Found {} addressable replaceable events (kinds 30000-39999)", total_addressable);
    println!("  Removed {} older versions of replaceable events", duplicates_removed);
    
    filtered_events
}

/// Parse events from file, supporting both JSON array and NDJSON formats
fn parse_events_from_file(input_file: &str) -> Result<Vec<Value>> {
    let file = File::open(input_file)?;
    let mut reader = BufReader::new(file);
    
    // Read first character to determine format
    let mut first_char = [0u8; 1];
    reader.read_exact(&mut first_char)?;
    
    // Reset reader to beginning
    reader.seek(std::io::SeekFrom::Start(0))?;
    
    if first_char[0] == b'[' {
        // JSON array format
        serde_json::from_reader(reader).context("Failed to parse JSON array")
    } else {
        // Assume NDJSON format (newline-delimited JSON)
        let mut events = Vec::new();
        let lines = reader.lines();
        
        for (line_num, line_result) in lines.enumerate() {
            let line = line_result.context("Failed to read line")?;
            let trimmed = line.trim();
            
            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }
            
            // Parse each line as a JSON object
            match serde_json::from_str::<Value>(trimmed) {
                Ok(event) => events.push(event),
                Err(e) => {
                    // Skip invalid lines with a warning
                    eprintln!("Warning: Skipping invalid JSON on line {}: {}", line_num + 1, e);
                }
            }
        }
        
        if events.is_empty() {
            return Err(anyhow!("No valid events found in input file"));
        }
        
        Ok(events)
    }
}

pub fn process_events(
    input_file: &str,
    name: &str,
    description: &str,
    author: &str,
    output_dir: &PathBuf,
    _no_bindings: bool,
    interactive: bool,
    verbose: bool,
    nip_11: bool,
    nip_42: bool,
    nip_45: bool,
    nip_50: bool,
    nip11_args: &Nip11Args,
) -> Result<()> {
    // Initialize interactive UI if enabled
    let mut record_ui = if interactive {
        let ui = ui::record::RecordUI::new();
        ui.init()?;
        Some(ui)
    } else {
        None
    };

    // Parse input file (supports both JSON array and NDJSON)
    let original_events = parse_events_from_file(input_file)?;
    
    // Display statistics (only in verbose mode)
    debugln!(verbose, "=== Cassette CLI - Record Command ===");
    debugln!(verbose, "Processing events for cassette creation...");
    
    debugln!(verbose, "\n📊 Initial Event Summary:");
    debugln!(verbose, "  Total events: {}", original_events.len());
    
    // Count the number of events by kind
    let mut kind_counts = std::collections::HashMap::new();
    for event in &original_events {
        if let Some(kind) = event.get("kind").and_then(|k| k.as_i64()) {
            *kind_counts.entry(kind).or_insert(0) += 1;
        }
    }
    
    // Display kind statistics
    if verbose && !kind_counts.is_empty() {
        println!("\n📋 Event Kinds:");
        for (kind, count) in kind_counts.iter() {
            println!("  Kind {}: {} events", kind, count);
        }
    }
    
    // Update interactive UI with initial stats
    if let Some(ref mut ui) = record_ui {
        for (i, event) in original_events.iter().enumerate() {
            if let Some(kind) = event.get("kind").and_then(|k| k.as_u64()) {
                ui.update(i as u64 + 1, Some(kind))?;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
    
    // Preprocess events to handle replaceable and addressable events
    debugln!(verbose, "\n🔍 Preprocessing events according to NIP-01...");
    let processed_events = preprocess_events(original_events);
    
    debugln!(verbose, "\n📊 Final Event Summary:");
    debugln!(verbose, "  Total events after preprocessing: {}", processed_events.len());
    
    // Sample of events
    if verbose {
        println!("\n📝 Sample Events:");
        for (i, event) in processed_events.iter().take(2).enumerate() {
        if let (Some(id), Some(kind), Some(pubkey)) = (
            event.get("id").and_then(|id| id.as_str()),
            event.get("kind").and_then(|k| k.as_i64()),
            event.get("pubkey").and_then(|p| p.as_str()),
        ) {
            println!("  Event {}: ID={}, Kind={}, Pubkey={}", 
                i + 1, 
                id.chars().take(8).collect::<String>() + "...",
                kind,
                pubkey.chars().take(8).collect::<String>() + "..."
            );
        }
    }
    
        if processed_events.len() > 2 {
            println!("  ... and {} more events", processed_events.len() - 2);
        }
    }

    // Generate metadata
    let cassette_created = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let event_count = processed_events.len();
    
    debugln!(verbose, "\n📦 Cassette Information:");
    debugln!(verbose, "  Name: {}", name);
    debugln!(verbose, "  Description: {}", description);
    debugln!(verbose, "  Author: {}", author);
    debugln!(verbose, "  Created: {}", cassette_created);

    // Create a temporary directory for building
    let temp_dir = tempdir()?;
    let project_dir = temp_dir.path().to_path_buf();

    debugln!(verbose, "\n🔨 Generating WASM Module:");
    debugln!(verbose, "  Creating Rust project from template...");
    debugln!(verbose, "  Using project directory: {}", project_dir.display());

    // Write events to the src directory
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    let events_json_path = src_dir.join("events.json");
    let mut events_file = File::create(&events_json_path)?;
    let events_json_string = serde_json::to_string(&processed_events).unwrap();
    events_file.write_all(events_json_string.as_bytes())?;

    // Initialize generator with output path and name
    let mut generator = generator::CassetteGenerator::new(
        output_dir.clone(),
        name,
        &project_dir
    );
    
    // Set template variables
    generator.set_var("cassette_name", name);
    generator.set_var("cassette_description", description);
    generator.set_var("cassette_author", author);
    generator.set_var("cassette_created", &cassette_created);
    generator.set_var("event_count", &event_count.to_string());
    generator.set_var("cassette_version", "0.1.0");
    
    // Properly escape the JSON for template insertion
    // Note: We're not double-escaping anymore, just using the raw JSON
    generator.set_var("events_json", &events_json_string);
    
    // Build features array based on NIP flags
    // Always include nip11 since info function should always be available
    let mut features = vec!["default".to_string(), "nip11".to_string()];
    if nip_42 {
        features.push("nip42".to_string());
    }
    if nip_45 {
        features.push("nip45".to_string());
    }
    if nip_50 {
        features.push("nip50".to_string());
    }
    
    // Convert features vector to JSON array format for template
    let features_json = serde_json::to_string(&features)?;
    generator.set_var("features_array", &features_json);
    
    // Set relay metadata for NIP-11 embedding
    if let Some(relay_name) = &nip11_args.relay_name {
        generator.set_var("relay_name", relay_name);
    }
    if let Some(relay_description) = &nip11_args.relay_description {
        generator.set_var("relay_description", relay_description);
    }
    if let Some(relay_pubkey) = &nip11_args.relay_pubkey {
        generator.set_var("relay_pubkey", relay_pubkey);
    }
    if let Some(relay_contact) = &nip11_args.relay_contact {
        generator.set_var("relay_contact", relay_contact);
    }

    // Set verbose mode on generator
    generator.set_verbose(verbose);
    
    // Generate the cassette with compilation progress
    let result = if let Some(ref ui) = record_ui {
        // Interactive mode - show compilation progress
        let total_events = processed_events.len() as u64;
        generator.generate_with_callback(Some(|| {
            ui.show_compilation(total_events)?;
            Ok(())
        }))
    } else {
        // Non-interactive mode
        generator.generate()
    };
    
    match result {
        Ok(wasm_path) => {
            if let Some(ui) = record_ui {
                // Show completion screen in interactive mode
                ui.show_completion(processed_events.len() as u64, &wasm_path.display().to_string())?;
                std::thread::sleep(std::time::Duration::from_secs(3));
                ui.cleanup()?;
            } else {
                debugln!(verbose, "  ✅ WASM module generated successfully!");
                debugln!(verbose, "  Output: {}", wasm_path.display());
                debugln!(verbose, "\n✅ Cassette creation complete!");
                debugln!(verbose, "  You can now load this WebAssembly module into the Boombox server.");
            }
            Ok(())
        },
        Err(e) => {
            if let Some(ui) = record_ui {
                ui.cleanup()?;
            }
            println!("  ❌ Failed to generate WASM module: {}", e);
            Err(anyhow!("Failed to generate WASM module: {}", e))
        }
    }
}
// Cast command implementation

#[derive(Clone)]
struct RelayStatus {
    url: String,
    connected: bool,
    total: usize,
    successful: usize,
    failed: usize,
}

async fn process_cast_command(
    cassette_paths: &[std::path::PathBuf],
    relay_urls: &[String],
    concurrency: usize,
    throttle_ms: u64,
    timeout_secs: u64,
    dry_run: bool,
    nip11_args: &Nip11Args,
) -> Result<()> {
    if cassette_paths.is_empty() {
        return Err(anyhow!("No cassettes specified"));
    }
    
    if relay_urls.is_empty() {
        return Err(anyhow!("No relays specified"));
    }
    
    println!("🎯 Casting events from {} cassette(s) to {} relay(s)", 
        cassette_paths.len(), relay_urls.len());
    
    if dry_run {
        println!("🏃 DRY RUN MODE - No events will actually be sent");
    }
    
    // Extract all unique events from cassettes
    let mut all_events = Vec::new();
    let mut event_ids = HashSet::new();
    
    for cassette_path in cassette_paths {
        println!("\n📼 Loading cassette: {}", cassette_path.display());
        
        if !cassette_path.exists() {
            eprintln!("  ⚠️  Warning: Cassette file not found, skipping");
            continue;
        }
        
        let events = extract_all_events_from_cassette(cassette_path, nip11_args)?;
        let initial_count = events.len();
        let mut added = 0;
        
        for event in events {
            if let Some(id) = event.get("id").and_then(|v| v.as_str()) {
                if event_ids.insert(id.to_string()) {
                    all_events.push(event);
                    added += 1;
                }
            }
        }
        
        println!("  ✓ Loaded {} events ({} unique)", initial_count, added);
    }
    
    if all_events.is_empty() {
        return Err(anyhow!("No events found in cassettes"));
    }
    
    println!("\n📊 Total unique events to cast: {}", all_events.len());
    
    // Initialize relay status tracking
    let relay_statuses = Arc::new(Mutex::new(
        relay_urls.iter().enumerate().map(|(idx, url)| RelayStatus {
            url: url.clone(),
            connected: false,
            total: all_events.len(),
            successful: 0,
            failed: 0,
        }).collect::<Vec<_>>()
    ));
    
    if dry_run {
        println!("\n🔍 Events that would be sent:");
        for (i, event) in all_events.iter().take(5).enumerate() {
            if let Some(id) = event.get("id").and_then(|v| v.as_str()) {
                println!("  Event {}: {}", i + 1, id);
            }
        }
        if all_events.len() > 5 {
            println!("  ... and {} more events", all_events.len() - 5);
        }
        return Ok(());
    }
    
    // Broadcast to all relays concurrently
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let timeout = tokio::time::Duration::from_secs(timeout_secs);
    let throttle = tokio::time::Duration::from_millis(throttle_ms);
    
    let tasks: Vec<_> = relay_urls.iter().enumerate().map(|(idx, relay_url)| {
        let events = all_events.clone();
        let relay_url = relay_url.clone();
        let statuses = relay_statuses.clone();
        let semaphore = semaphore.clone();
        
        tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            broadcast_to_relay(idx, relay_url, events, statuses, timeout, throttle).await
        })
    }).collect();
    
    // Start progress display
    let display_handle = {
        let statuses = relay_statuses.clone();
        tokio::spawn(async move {
            loop {
                display_relay_status(&statuses).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                // Check if all relays are done
                let all_done = {
                    let statuses = statuses.lock().await;
                    statuses.iter().all(|s| s.successful + s.failed >= s.total)
                };
                
                if all_done {
                    break;
                }
            }
            // Final display
            display_relay_status(&statuses).await;
        })
    };
    
    // Wait for all broadcasts to complete
    let results = futures_util::future::join_all(tasks).await;
    
    // Stop display task
    display_handle.abort();
    
    // Print final results
    println!("\n\n📊 Final Results:");
    let statuses = relay_statuses.lock().await;
    for status in statuses.iter() {
        let success_rate = if status.total > 0 {
            status.successful as f64 / status.total as f64 * 100.0
        } else {
            0.0
        };
        println!("  {} - {}/{} events ({:.1}% success rate)", 
            status.url, status.successful, status.total, success_rate);
    }
    
    // Check for errors
    for result in results {
        if let Err(e) = result {
            eprintln!("\n⚠️  Task error: {}", e);
        }
    }
    
    Ok(())
}

/// Extract all events from a cassette
fn extract_all_events_from_cassette(cassette_path: &std::path::PathBuf, nip11_args: &Nip11Args) -> Result<Vec<Value>> {
    use wasmtime::{Store, Module, Instance};
    
    let engine = wasmtime::Engine::default();
    let module = Module::from_file(&engine, cassette_path)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    
    // Set NIP-11 info if provided
    load_cassette_with_nip11(&mut store, &instance, nip11_args)?;
    
    let memory = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow!("No memory export found"))?;
    
    let alloc_func = instance
        .get_typed_func::<i32, i32>(&mut store, "alloc_buffer")
        .or_else(|_| instance.get_typed_func::<i32, i32>(&mut store, "alloc_string"))?;
    let send_func = instance.get_typed_func::<(i32, i32), i32>(&mut store, "send")?;
    let dealloc_func = instance.get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string").ok();
    
    // Request all events
    let req_message = json!(["REQ", "cast-extract", {}]);
    let req_string = serde_json::to_string(&req_message)?;
    let req_bytes = req_string.as_bytes();
    
    let mut events = Vec::new();
    
    loop {
        let req_ptr = alloc_func.call(&mut store, req_bytes.len() as i32)?;
        if req_ptr == 0 {
            break;
        }
        
        memory.write(&mut store, req_ptr as usize, req_bytes)?;
        let result_ptr = send_func.call(&mut store, (req_ptr, req_bytes.len() as i32))?;
        
        if let Some(dealloc) = &dealloc_func {
            dealloc.call(&mut store, (req_ptr, req_bytes.len() as i32))?;
        }
        
        if result_ptr == 0 {
            break;
        }
        
        let result = read_string_from_memory(&mut store, &instance, &memory, result_ptr)?;
        
        if let Ok(get_size_func) = instance.get_typed_func::<i32, i32>(&mut store, "get_allocation_size") {
            if let Ok(size) = get_size_func.call(&mut store, result_ptr) {
                if let Some(dealloc) = &dealloc_func {
                    let _ = dealloc.call(&mut store, (result_ptr, size));
                }
            }
        }
        
        let parsed: Value = serde_json::from_str(&result)?;
        if let Some(arr) = parsed.as_array() {
            if arr.len() >= 2 {
                match arr[0].as_str() {
                    Some("EVENT") => {
                        if let Some(event) = arr.get(2) {
                            events.push(event.clone());
                        }
                    }
                    Some("EOSE") => break,
                    _ => {}
                }
            }
        }
    }
    
    Ok(events)
}

/// Broadcast events to a single relay
async fn broadcast_to_relay(
    idx: usize,
    relay_url: String,
    events: Vec<Value>,
    statuses: Arc<Mutex<Vec<RelayStatus>>>,
    timeout: tokio::time::Duration,
    throttle: tokio::time::Duration,
) -> Result<()> {
    // Connect to relay with timeout
    let ws_stream = tokio::time::timeout(
        timeout,
        connect_async(&relay_url)
    ).await
        .map_err(|_| anyhow!("Connection timeout"))?
        .map_err(|e| anyhow!("Connection failed: {}", e))?;
    
    // Update connection status
    {
        let mut statuses = statuses.lock().await;
        statuses[idx].connected = true;
    }
    
    let (mut write, mut read) = ws_stream.0.split();
    
    // Send events
    for event in events {
        let event_msg = json!(["EVENT", event]);
        let msg_text = serde_json::to_string(&event_msg)?;
        
        // Send event
        write.send(Message::Text(msg_text)).await?;
        
        // Wait for OK response
        let success = wait_for_ok(&mut read).await?;
        
        // Update status
        {
            let mut statuses = statuses.lock().await;
            if success {
                statuses[idx].successful += 1;
            } else {
                statuses[idx].failed += 1;
            }
        }
        
        // Throttle between sends
        if throttle.as_millis() > 0 {
            tokio::time::sleep(throttle).await;
        }
    }
    
    // Close connection
    let _ = write.close().await;
    
    Ok(())
}

/// Wait for OK response from relay
async fn wait_for_ok(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
        >
    >
) -> Result<bool> {
    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                if let Ok(parsed) = serde_json::from_str::<Vec<Value>>(&text) {
                    if parsed.len() >= 3 && parsed[0] == "OK" {
                        // Check if success (third element is true)
                        return Ok(parsed[2].as_bool().unwrap_or(false));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(false)
}

/// Display relay status with ANSI escape codes
async fn display_relay_status(statuses: &Arc<Mutex<Vec<RelayStatus>>>) {
    let statuses = statuses.lock().await;
    
    // Move cursor up and clear lines
    let lines = statuses.len() + 2;
    print!("\x1b[{}A", lines);
    
    println!("\n🎯 Broadcasting Progress:");
    for status in statuses.iter() {
        let progress = if status.total > 0 {
            (status.successful + status.failed) as f64 / status.total as f64 * 100.0
        } else {
            0.0
        };
        
        let connection_status = if status.connected { "🟢" } else { "🔴" };
        println!("{} {} - {}/{} ({:.1}%)", 
            connection_status, status.url, status.successful, status.total, progress);
    }
}