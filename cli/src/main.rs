use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write, BufReader, BufRead, Seek};
use std::path::PathBuf;
use chrono::Utc;
use std::fs::File;
use tempfile::tempdir;
use std::collections::HashMap;
use wasmtime::{Store, Module, Instance, Memory};

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

    // Load template files
    const TEMPLATE_RS: &str = include_str!("templates/cassette_template.rs");
    const TEMPLATE_CARGO: &str = include_str!("templates/Cargo.toml");

    pub struct CassetteGenerator {
        output_dir: PathBuf,
        name: String,
        project_dir: PathBuf,
        template_vars: HashMap<String, String>,
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
            }
        }

        pub fn set_var(&mut self, key: &str, value: &str) {
            self.template_vars.insert(key.to_string(), value.to_string());
        }

        pub fn generate(&self) -> Result<PathBuf> {
            // Create src directory if it doesn't exist
            let src_dir = self.project_dir.join("src");
            fs::create_dir_all(&src_dir).context("Failed to create src directory")?;

            // Create the lib.rs file from template
            self.create_project_files(&src_dir)?;

            // Build the WASM module
            let output_path = self.build_wasm(&self.project_dir)?;

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
            println!("Debug: Template data: {}", serde_json::to_string_pretty(&template_data).unwrap_or_default());
            
            // Render the lib.rs template
            let lib_rs_content = handlebars.render_template(TEMPLATE_RS, &template_data)
                .context("Failed to render lib.rs template")?;

            // Debug: Write the rendered template to a log file
            let log_dir = PathBuf::from("../logs");
            fs::create_dir_all(&log_dir).ok(); // Ignore errors
            let log_path = log_dir.join("template_debug.rs");
            let _ = fs::write(&log_path, &lib_rs_content); // Ignore errors
            println!("Debug: Rendered template saved to {:?}", log_path);

            // Write the lib.rs file
            let lib_rs_path = src_dir.join("lib.rs");
            let mut lib_rs_file = File::create(&lib_rs_path)
                .context("Failed to create lib.rs file")?;
            lib_rs_file.write_all(lib_rs_content.as_bytes())
                .context("Failed to write to lib.rs file")?;

            // Get the relative path to cassette-tools
            let cassette_tools_path = self.get_relative_cassette_tools_path()?;

            // Create the Cargo.toml file from template
            let cargo_data = json!({
                "crate_name": self.name,
                "version": "0.1.0",
                "description": self.template_vars.get("cassette_description").unwrap_or(&"Generated Cassette".to_string()),
                "cassette_tools_path": cassette_tools_path
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
            
            // Find the project root by traversing up until we find a marker file
            let mut project_root = current_dir.clone();
            loop {
                if project_root.join("cassette-tools").exists() {
                    // Found the project root
                    let tools_path = project_root.join("cassette-tools").display().to_string();
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

        fn build_wasm(&self, project_dir: &Path) -> Result<PathBuf> {
            // Change to the project directory
            let current_dir = std::env::current_dir()?;
            std::env::set_current_dir(project_dir)?;

            // Print the generated Cargo.toml for debugging
            println!("  Using project directory: {}", project_dir.display());
            if let Ok(cargo_content) = fs::read_to_string(project_dir.join("Cargo.toml")) {
                println!("  Generated Cargo.toml:\n{}", cargo_content);
            }

            // Run cargo build --target wasm32-unknown-unknown
            println!("  Running cargo build...");
            let status = Command::new("cargo")
                .args(&["build", "--target", "wasm32-unknown-unknown", "--release"])
                .status()
                .context("Failed to run cargo build. Make sure Rust and the wasm32-unknown-unknown target are installed.")?;

            // Change back to the original directory
            std::env::set_current_dir(current_dir)?;

            if !status.success() {
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
            println!("  Creating output directory: {:?}", self.output_dir);
            fs::create_dir_all(&self.output_dir)
                .context("Failed to create output directory")?;

            // Copy the WASM file to the output directory with a simple filename
            let dest_path = self.output_dir.join(format!("{}.wasm", self.name));
            
            // Debug output to diagnose any issues
            println!("  Copying from: {:?}", wasm_path);
            println!("  Copying to: {:?}", dest_path);
            
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
                    println!("  ⚠️ Warning: Failed to ensure wasm32-unknown-unknown target is installed");
                }
            }
            
            // Try to copy the file with more robust error handling
            match fs::copy(&wasm_path, &dest_path) {
                Ok(_) => {
                    println!("  ✅ Successfully copied WASM file to {:?}", dest_path);
                    Ok(dest_path)
                },
                Err(e) => {
                    println!("  ❌ Copy failed with error: {:?}", e);
                    
                    // As a fallback, try to use the 'cp' command
                    let status = Command::new("cp")
                        .arg(&wasm_path)
                        .arg(&dest_path)
                        .status();
                        
                    match status {
                        Ok(exit) if exit.success() => {
                            println!("  ✅ Successfully copied WASM file using cp command");
                            Ok(dest_path)
                        },
                        _ => Err(anyhow!("Failed to copy WASM file to output directory: {}", e))
                    }
                }
            }
        }
    }
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
    
    // Get memory export
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow!("Memory export not found"))?;
    
    // Get the req function
    let req_func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "req")
        .context("Failed to get req function")?;
    
    // Get allocation function
    let alloc_func = instance
        .get_typed_func::<i32, i32>(&mut store, "alloc_buffer")
        .or_else(|_| instance.get_typed_func::<i32, i32>(&mut store, "alloc_string"))
        .context("Failed to get allocation function")?;
    
    // Get deallocation function
    let dealloc_func = instance
        .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string")
        .context("Failed to get deallocation function")?;
    
    // Allocate memory for the request string
    let req_bytes = req_string.as_bytes();
    // Collect all events in a loop
    let mut all_events = Vec::new();
    
    loop {
        // Allocate memory for the request string for each call
        let req_ptr = alloc_func.call(&mut store, req_bytes.len() as i32)?;
        
        if req_ptr == 0 {
            return Err(anyhow!("Failed to allocate memory for request"));
        }
        
        // Write request to memory
        memory.write(&mut store, req_ptr as usize, req_bytes)?;
        
        // Call the req function
        let result_ptr = req_func.call(&mut store, (req_ptr, req_bytes.len() as i32))?;
        
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
                            all_events.push(arr[2].clone());
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
    
    // Output all collected events based on format
    match output_format {
        "ndjson" => {
            // For NDJSON, output each event on its own line
            for event in &all_events {
                println!("{}", serde_json::to_string(&event)?);
            }
        }
        _ => {
            // Default JSON output - output as array
            if all_events.len() == 1 {
                println!("{}", serde_json::to_string_pretty(&all_events[0])?);
            } else {
                println!("{}", serde_json::to_string_pretty(&all_events)?);
            }
        }
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
) -> Result<()> {
    if cassette_paths.is_empty() {
        return Err(anyhow!("No input cassettes specified"));
    }
    
    println!("=== Cassette CLI - Dub Command ===");
    println!("Combining {} cassettes...", cassette_paths.len());
    
    // Collect all events from all cassettes
    let mut all_events = Vec::new();
    
    for (idx, cassette_path) in cassette_paths.iter().enumerate() {
        println!("\n📼 Processing cassette {}/{}: {}", 
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
        
        // Get memory export
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("Memory export not found"))?;
        
        // Get the req function
        let req_func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "req")
            .context("Failed to get req function")?;
        
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
            
            // Call the req function
            let result_ptr = req_func.call(&mut store, (req_ptr, req_bytes.len() as i32))?;
            
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
        
        println!("  Found {} events", cassette_events.len());
        all_events.extend(cassette_events);
    }
    
    println!("\n📊 Total events collected: {}", all_events.len());
    
    // Apply filters if specified
    if !kinds.is_empty() || !authors.is_empty() || !filter_args.is_empty() || since.is_some() || until.is_some() {
        println!("\n🔍 Applying filters...");
        
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
        
        println!("  Events after filtering: {}", filtered_events.len());
        all_events = filtered_events;
    }
    
    // Apply limit if specified and not already applied via filter
    if let Some(l) = limit {
        if all_events.len() > l {
            println!("  Applying limit of {} events", l);
            all_events.truncate(l);
        }
    }
    
    // Preprocess events to handle replaceable events
    println!("\n🔍 Preprocessing events according to NIP-01...");
    let processed_events = preprocess_events(all_events);
    println!("  Final event count: {}", processed_events.len());
    
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
        false // no_bindings
    )?;
    
    // Rename the generated file to the specified output name if needed
    let generated_path = output_dir.join(format!("{}.wasm", cassette_name));
    if generated_path != *output_path {
        fs::rename(&generated_path, output_path)
            .context("Failed to rename output file")?;
        println!("\n✅ Dubbed cassette saved to: {}", output_path.display());
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
    },
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
            no_bindings
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
                    *no_bindings
                )?;
            } else {
                // No input file, read from stdin
                println!("Reading events from stdin...");
                
                // Create a temp file to store stdin content
                let temp_dir = tempdir()?;
                let temp_file_path = temp_dir.path().join("stdin_events.json");
                let temp_file_path_str = temp_file_path.to_str().unwrap();
                
                // Read from stdin
                let stdin = std::io::stdin();
                let mut stdin_content = Vec::new();
                stdin.lock().read_to_end(&mut stdin_content)?;
                
                if stdin_content.is_empty() {
                    return Err(anyhow!("No data received from stdin. Please pipe in events or use an input file."));
                }
                
                // Write the stdin content to the temp file
                std::fs::write(&temp_file_path, stdin_content)?;
                
                // Process the temp file
                process_events(
                    temp_file_path_str,
                    &name_value,
                    &desc_value,
                    &author_value,
                    &output_value,
                    *no_bindings
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
        } => {
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
            )
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
    _no_bindings: bool
) -> Result<()> {
    // Parse input file (supports both JSON array and NDJSON)
    let original_events = parse_events_from_file(input_file)?;
    
    // Display statistics
    println!("=== Cassette CLI - Record Command ===");
    println!("Processing events for cassette creation...");
    
    println!("\n📊 Initial Event Summary:");
    println!("  Total events: {}", original_events.len());
    
    // Count the number of events by kind
    let mut kind_counts = std::collections::HashMap::new();
    for event in &original_events {
        if let Some(kind) = event.get("kind").and_then(|k| k.as_i64()) {
            *kind_counts.entry(kind).or_insert(0) += 1;
        }
    }
    
    // Display kind statistics
    if !kind_counts.is_empty() {
        println!("\n📋 Event Kinds:");
        for (kind, count) in kind_counts.iter() {
            println!("  Kind {}: {} events", kind, count);
        }
    }
    
    // Preprocess events to handle replaceable and addressable events
    println!("\n🔍 Preprocessing events according to NIP-01...");
    let processed_events = preprocess_events(original_events);
    
    println!("\n📊 Final Event Summary:");
    println!("  Total events after preprocessing: {}", processed_events.len());
    
    // Sample of events
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

    // Generate metadata
    let cassette_created = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let event_count = processed_events.len();
    
    println!("\n📦 Cassette Information:");
    println!("  Name: {}", name);
    println!("  Description: {}", description);
    println!("  Author: {}", author);
    println!("  Created: {}", cassette_created);

    // Create a temporary directory for building
    let temp_dir = tempdir()?;
    let project_dir = temp_dir.path().to_path_buf();
    println!("Using project directory: {}", project_dir.display());

    println!("\n🔨 Generating WASM Module:");
    println!("  Creating Rust project from template...");
    println!("  Using project directory: {}", project_dir.display());

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

    // Generate the cassette
    match generator.generate() {
        Ok(wasm_path) => {
            println!("  ✅ WASM module generated successfully!");
            println!("  Output: {}", wasm_path.display());
            println!("\n✅ Cassette creation complete!");
            println!("  You can now load this WebAssembly module into the Boombox server.");
            Ok(())
        },
        Err(e) => {
            println!("  ❌ Failed to generate WASM module: {}", e);
            Err(anyhow!("Failed to generate WASM module: {}", e))
        }
    }
}
