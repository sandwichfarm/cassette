use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write, BufReader, BufRead, Seek};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::Utc;
use std::fs::File;
use tempfile::{tempdir, TempDir};
use std::collections::{HashMap, HashSet};
use wasmtime::{Store, Module, Instance, Memory, TypedFunc, Engine};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, accept_async};
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::net::{TcpListener, TcpStream};
use glob::glob;
use secp256k1::{XOnlyPublicKey, Secp256k1, Message as Secp256k1Message};
use sha2::{Sha256, Digest};

mod ui;
mod deps;
mod embedded_cassette_tools;

/// Sanitize a name for use as a filename
/// Converts to lowercase, replaces spaces with hyphens, removes special characters
fn sanitize_filename(name: &str) -> String {
    let sanitized = name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                // Remove all other characters
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        // Remove consecutive hyphens
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    
    // If the result is empty, use "bruh"
    if sanitized.is_empty() {
        "bruh".to_string()
    } else {
        sanitized
    }
}

/// Validate a Nostr event using rust-nostr
/// Returns true if the event is valid, false otherwise
fn validate_nostr_event(event_json: &Value, verbose: bool) -> bool {
    // Extract required fields from the event JSON
    let id = match event_json.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            if verbose {
                println!("❌ Event missing 'id' field");
            }
            return false;
        }
    };
    
    let pubkey = match event_json.get("pubkey").and_then(|v| v.as_str()) {
        Some(pk) => pk,
        None => {
            if verbose {
                println!("❌ Event {} missing 'pubkey' field", id);
            }
            return false;
        }
    };
    
    let sig = match event_json.get("sig").and_then(|v| v.as_str()) {
        Some(sig) => sig,
        None => {
            if verbose {
                println!("❌ Event {} missing 'sig' field", id);
            }
            return false;
        }
    };
    
    let created_at = match event_json.get("created_at").and_then(|v| v.as_i64()) {
        Some(ts) => ts,
        None => {
            if verbose {
                println!("❌ Event {} missing 'created_at' field", id);
            }
            return false;
        }
    };
    
    let kind = match event_json.get("kind").and_then(|v| v.as_i64()) {
        Some(k) => k,
        None => {
            if verbose {
                println!("❌ Event {} missing 'kind' field", id);
            }
            return false;
        }
    };
    
    let content = event_json.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let tags = event_json.get("tags").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|tag| {
            if let Some(tag_arr) = tag.as_array() {
                let tag_strs: Vec<String> = tag_arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !tag_strs.is_empty() {
                    Some(tag_strs)
                } else {
                    None
                }
            } else {
                None
            }
        }).collect::<Vec<_>>()
    }).unwrap_or_default();
    
    // Recreate the event ID to verify it matches
    // Format: [0, pubkey, created_at, kind, tags, content]
    let mut id_input = format!("[0,\"{}\",{},{},[", pubkey, created_at, kind);
    
    for (i, tag) in tags.iter().enumerate() {
        if i > 0 {
            id_input.push(',');
        }
        id_input.push_str(&serde_json::to_string(tag).unwrap_or_default());
    }
    id_input.push_str("],");
    id_input.push_str(&serde_json::to_string(content).unwrap_or_default());
    id_input.push(']');
    
    // Calculate SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(id_input.as_bytes());
    let computed_id = hex::encode(hasher.finalize());
    
    // Debug output for troubleshooting
    if verbose {
        println!("🔍 Event validation debug:");
        println!("  Input string: {}", id_input);
        println!("  Expected ID:  {}", id);
        println!("  Computed ID:  {}", computed_id);
        println!("  Match: {}", computed_id == id);
    }
    
    // Verify the ID matches
    if computed_id != id {
        if verbose {
            println!("❌ Event {} has invalid ID (computed: {})", id, computed_id);
        }
        return false;
    }
    
    // Verify the signature
    let secp = Secp256k1::new();
    
    // Parse pubkey
    let pubkey_bytes = match hex::decode(pubkey) {
        Ok(bytes) => bytes,
        Err(_) => {
            if verbose {
                println!("❌ Event {} has invalid pubkey format", id);
            }
            return false;
        }
    };
    
    let xonly_pubkey = match XOnlyPublicKey::from_slice(&pubkey_bytes) {
        Ok(pk) => pk,
        Err(_) => {
            if verbose {
                println!("❌ Event {} has invalid pubkey", id);
            }
            return false;
        }
    };
    
    // Parse signature
    let sig_bytes = match hex::decode(sig) {
        Ok(bytes) => bytes,
        Err(_) => {
            if verbose {
                println!("❌ Event {} has invalid signature format", id);
            }
            return false;
        }
    };
    
    if sig_bytes.len() != 64 {
        if verbose {
            println!("❌ Event {} has invalid signature length", id);
        }
        return false;
    }
    
    let signature = match secp256k1::schnorr::Signature::from_slice(&sig_bytes) {
        Ok(sig) => sig,
        Err(_) => {
            if verbose {
                println!("❌ Event {} has invalid signature", id);
            }
            return false;
        }
    };
    
    // Create message hash for verification
    let id_bytes = match hex::decode(id) {
        Ok(bytes) => bytes,
        Err(_) => {
            if verbose {
                println!("❌ Event {} has invalid ID format", id);
            }
            return false;
        }
    };
    
    let message = match Secp256k1Message::from_slice(&id_bytes) {
        Ok(msg) => msg,
        Err(_) => {
            if verbose {
                println!("❌ Event {} cannot create message from ID", id);
            }
            return false;
        }
    };
    
    // Verify signature
    match secp.verify_schnorr(&signature, &message, &xonly_pubkey) {
        Ok(_) => {
            if verbose {
                println!("✅ Event {} is valid", id);
            }
            true
        }
        Err(_) => {
            if verbose {
                println!("❌ Event {} has invalid signature", id);
            }
            false
        }
    }
}

// Macro for debug output that only prints in verbose mode
macro_rules! debugln {
    ($verbose:expr, $($arg:tt)*) => {
        if $verbose {
            println!($($arg)*);
        }
    };
}

/// Escape JSON string for use in a Rust raw string literal with ### delimiters
/// This ensures the JSON doesn't contain "### which would break the raw string
fn escape_json_for_raw_string(json: &str) -> String {
    // Count the maximum consecutive # characters after a quote in the JSON
    let mut max_hashes = 0;
    let mut current_hashes = 0;
    let mut after_quote = false;
    
    for ch in json.chars() {
        match ch {
            '"' => {
                after_quote = true;
                current_hashes = 0;
            }
            '#' if after_quote => {
                current_hashes += 1;
                max_hashes = max_hashes.max(current_hashes);
            }
            _ => {
                after_quote = false;
                current_hashes = 0;
            }
        }
    }
    
    // If we found "### or more, we need to use more # symbols in the raw string
    // For now, we'll use a simpler approach: replace "### with something else
    if max_hashes >= 3 {
        // Replace "### with "##\u{23} (using unicode escape for #)
        json.replace("\"###", "\"##\\u{23}")
    } else {
        json.to_string()
    }
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
    use super::sanitize_filename;
    
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
        
        /// Generate a cassette using embedded cassette-tools library  
        #[cfg(feature = "deck")]
        pub fn generate_with_embedded_tools(&self) -> Result<PathBuf> {
            self.generate_with_tools_dir(None)
        }
        
        /// Generate a cassette using embedded cassette-tools library with optional pre-extracted tools dir
        #[cfg(feature = "deck")]
        pub fn generate_with_tools_dir(&self, existing_tools_dir: Option<&Path>) -> Result<PathBuf> {
            debugln!(self.verbose, "🔧 Generating cassette with embedded tools");
            
            // The project_dir should already be set up by the caller
            // We'll use that instead of creating a new temp directory
            
            // Create src directory
            let src_dir = self.project_dir.join("src");
            fs::create_dir_all(&src_dir)?;
            
            // Determine tools directory
            let tools_dir = if let Some(existing_tools) = existing_tools_dir {
                // Make sure we use the absolute path
                fs::canonicalize(existing_tools)?
            } else {
                // Extract to a local directory within the project
                let tools_dir = self.project_dir.join("cassette-tools");
                fs::create_dir_all(&tools_dir)?;
                self.extract_embedded_tools(&tools_dir)?;
                fs::canonicalize(&tools_dir)?
            };
            
            // Create the wrapper project with local path dependency
            self.create_embedded_project_files(&src_dir, &tools_dir, &self.project_dir)?;
            
            // Build the WASM module
            let output_path = self.build_wasm(&self.project_dir, None::<fn() -> Result<()>>)?;
            
            // Copy to destination
            let dest_path = self.copy_output(output_path)?;
            
            Ok(dest_path)
        }
        
        #[cfg(feature = "deck")]
        fn extract_embedded_tools(&self, tools_dir: &Path) -> Result<()> {
            use crate::embedded_cassette_tools::get_embedded_tools_dir;
            
            // Get the embedded tools directory
            let embedded_dir = get_embedded_tools_dir();
            
            debugln!(self.verbose, "  Extracting embedded cassette-tools from: {}", embedded_dir.display());
            
            // Copy all files from embedded directory to tools_dir
            self.copy_dir_contents(embedded_dir, tools_dir)?;
            
            debugln!(self.verbose, "  Extracted cassette-tools to: {}", tools_dir.display());
            
            Ok(())
        }
        
        #[cfg(feature = "deck")]
        pub fn copy_dir_contents(&self, src: &Path, dst: &Path) -> Result<()> {
            fs::create_dir_all(dst)?;
            
            for entry in fs::read_dir(src)? {
                let entry = entry?;
                let ty = entry.file_type()?;
                let src_path = entry.path();
                let dst_path = dst.join(entry.file_name());
                
                if ty.is_dir() {
                    self.copy_dir_contents(&src_path, &dst_path)?;
                } else {
                    fs::copy(&src_path, &dst_path)?;
                }
            }
            
            Ok(())
        }
        
        #[cfg(feature = "deck")]  
        fn create_embedded_project_files(&self, src_dir: &Path, tools_dir: &Path, project_dir: &Path) -> Result<()> {
            // Create the lib.rs file from template
            let mut lib_rs_file = File::create(src_dir.join("lib.rs"))
                .context("Failed to create lib.rs file")?;
            
            // Use the existing template rendering
            let mut handlebars = Handlebars::new();
            handlebars.set_strict_mode(true);
            // Disable HTML escaping for raw JSON content
            handlebars.register_escape_fn(handlebars::no_escape);
            
            let mut template_data = json!({});
            let obj = template_data.as_object_mut().unwrap();
            
            for (key, value) in &self.template_vars {
                obj.insert(key.clone(), json!(value));
            }
            
            let lib_rs_content = handlebars.render_template(TEMPLATE_RS, &template_data)
                .context("Failed to render lib.rs template")?;
            
            lib_rs_file.write_all(lib_rs_content.as_bytes())
                .context("Failed to write to lib.rs file")?;
            
            // Create Cargo.toml with local path to extracted tools
            let cargo_data = json!({
                "crate_name": self.name,
                "version": "0.1.0",
                "description": "Generated Cassette",
                "cassette_tools_path": tools_dir.display().to_string(),
                "features_array": self.template_vars.get("features_array").unwrap_or(&"[\"default\"]".to_string())
            });
            
            let cargo_content = handlebars.render_template(TEMPLATE_CARGO, &cargo_data)
                .context("Failed to render Cargo.toml template")?;
            
            let cargo_path = project_dir.join("Cargo.toml");
            let mut cargo_file = File::create(&cargo_path)
                .context("Failed to create Cargo.toml file")?;
            cargo_file.write_all(cargo_content.as_bytes())
                .context("Failed to write to Cargo.toml file")?;
            
            Ok(())
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
            // Disable HTML escaping for raw JSON content
            handlebars.register_escape_fn(handlebars::no_escape);

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
            let lib_rs_content = match handlebars.render_template(TEMPLATE_RS, &template_data) {
                Ok(content) => content,
                Err(e) => {
                    let keys: Vec<String> = template_data.as_object()
                        .map(|o| o.keys().cloned().collect())
                        .unwrap_or_default();
                    return Err(anyhow::anyhow!("Failed to render lib.rs template. Error: {}. Available keys: {:?}", e, keys));
                }
            };

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
                "description": "Generated Cassette",
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
            #[cfg(feature = "deck")]
            {
                // When using deck feature, cassette-tools is already embedded
                return Ok("./cassette-tools".to_string());
            }
            
            #[cfg(not(feature = "deck"))]
            {
                // We'll determine this from the current working directory
                let current_dir = std::env::current_dir()?;
                debugln!(self.verbose, "  Current directory: {}", current_dir.display());
            
                // Find the project root by traversing up until we find a marker file
                let mut project_root = current_dir.clone();
                loop {
                if project_root.join("cassette-tools").exists() {
                    // Found the project root - canonicalize to resolve symlinks
                    let tools_path = project_root.join("cassette-tools")
                        .canonicalize()
                        .context("Failed to canonicalize cassette-tools path")?;
                    let tools_path_str = tools_path.display().to_string();
                    debugln!(self.verbose, "  Found cassette-tools at: {}", tools_path_str);
                    return Ok(tools_path_str);
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
            let sanitized_name = sanitize_filename(&self.name);
            let dest_path = self.output_dir.join(format!("{}.wasm", sanitized_name));
            
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
    // Get the send function for sending COUNT (try 'send' then 'req' for backward compatibility)
    let send_func = match instance.get_typed_func::<(i32, i32), i32>(&mut *store, "send")
        .or_else(|_| instance.get_typed_func::<(i32, i32), i32>(&mut *store, "req")) {
        Ok(func) => func,
        Err(_) => return Ok(None), // No send/req function available
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
    _skip_validation: bool,
    nip11_args: &Nip11Args,
    search_query: Option<&str>,
) -> Result<()> {
    // Initialize interactive UI if enabled
    let mut play_ui = if interactive {
        let ui = ui::scrub::ScrubUI::new();
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
    
    // Get the send function (try new 'send' first, then fall back to old 'req')
    let send_func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "send")
        .or_else(|_| instance.get_typed_func::<(i32, i32), i32>(&mut store, "req"))
        .context("Failed to get send/req function")?;
    
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
                            // Validate event if validation is not skipped
                            if !_skip_validation {
                                if !validate_nostr_event(&arr[2], verbose) {
                                    // Skip invalid event
                                    continue;
                                }
                            }
                            
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
            "nip01" => {
                // Output as NIP-01 protocol messages
                for event in &all_events {
                    let event_msg = json!(["EVENT", subscription, event]);
                    println!("{}", serde_json::to_string(&event_msg)?);
                }
                // Send EOSE at the end
                let eose_msg = json!(["EOSE", subscription]);
                println!("{}", serde_json::to_string(&eose_msg)?);
            }
            "ndjson" => {
                for event in &all_events {
                    println!("{}", serde_json::to_string(&event)?);
                }
            }
            "json" => {
                if all_events.len() == 1 {
                    println!("{}", serde_json::to_string_pretty(&all_events[0])?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&all_events)?);
                }
            }
            _ => {
                eprintln!("Unknown output format: {}. Using nip01.", output_format);
                // Default to nip01
                for event in &all_events {
                    let event_msg = json!(["EVENT", subscription, event]);
                    println!("{}", serde_json::to_string(&event_msg)?);
                }
                let eose_msg = json!(["EOSE", subscription]);
                println!("{}", serde_json::to_string(&eose_msg)?);
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
    // Try new 'send' first, then fall back to old 'req' for backward compatibility
    let send_func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "send")
        .or_else(|_| instance.get_typed_func::<(i32, i32), i32>(&mut store, "req"))
        .context("Failed to get send/req function")?;
    
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
        
        // Get the send function (try 'send' then 'req' for backward compatibility)
        let send_func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "send")
            .or_else(|_| instance.get_typed_func::<(i32, i32), i32>(&mut store, "req"))
            .context("Failed to get send/req function")?;
        
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
    let cassette_name = sanitize_filename(name.unwrap_or("dubbed_cassette"));
    
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
        &cassette_name,
        &output_dir,
        false, // no_bindings
        false, // interactive
        false, // verbose
        true, // validate (enabled by default)
        false, // skip_unicode_check
        false, // _nip_11
        false, // nip_42
        false, // nip_45
        false, // nip_50
        nip11_args
    )?;
    
    // Rename the generated file to the specified output name if needed
    let sanitized_name = sanitize_filename(&cassette_name);
    let generated_path = output_dir.join(format!("{}.wasm", sanitized_name));
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
    #[arg(long = "relay-name")]
    relay_name: Option<String>,
    
    /// Description for NIP-11
    #[arg(long = "relay-description")]
    relay_description: Option<String>,
    
    /// Owner pubkey for NIP-11
    #[arg(long = "relay-pubkey")]
    relay_pubkey: Option<String>,
    
    /// Contact for NIP-11
    #[arg(long = "relay-contact")]
    relay_contact: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Record Nostr events from a file or piped input to create a cassette
    Record {
        /// Path to input events.json file (if not provided, reads from stdin)
        input_file: Option<PathBuf>,

        /// Name for the generated cassette (used for filename)
        #[arg(short, long)]
        name: Option<String>,

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
        
        /// Skip validation of Nostr events (validation is enabled by default)
        #[arg(long)]
        _skip_validation: bool,
        
        /// Skip Unicode character checks that might cause Rust compilation issues
        #[arg(long = "skip-unicode-check")]
        skip_unicode_check: bool,
        
        /// Enable NIP-11 (Relay Information Document)
        #[arg(long)]
        _nip_11: bool,
        
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
        output: Option<PathBuf>,
        
        /// Name for the generated cassette (used for filename)
        #[arg(short, long)]
        name: Option<String>,
        
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
    
    /// Scrub through cassette events (send REQ messages and get events)
    Scrub {
        /// Path to the cassette WASM file
        cassette: Option<PathBuf>,
        
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
        
        /// Output format: nip01 (default), json, or ndjson
        #[arg(short, long, default_value = "nip01")]
        output: String,
        /// Enable interactive mode with visual feedback
        #[arg(short = 'i', long)]
        interactive: bool,
        /// Show verbose output including compilation details
        #[arg(short, long)]
        verbose: bool,
        
        /// Skip validation of returned Nostr events (validation is enabled by default)
        #[arg(long)]
        _skip_validation: bool,
        
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
    
    /// [DEPRECATED] Use 'scrub' command instead - Play cassette events
    #[command(name = "deprecated-play", hide = true)]
    DeprecatedPlay {
        /// Path to the cassette WASM file
        cassette: Option<PathBuf>,
        
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
        
        /// Skip validation of returned Nostr events (validation is enabled by default)
        #[arg(long)]
        _skip_validation: bool,
        
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
    
    /// Play events from cassettes to Nostr relays
    Play {
        /// Input cassette files to broadcast
        cassettes: Vec<PathBuf>,
        
        /// Target relay URLs
        #[arg(short, long)]
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
    
    /// [DEPRECATED] Use 'play' command instead - Cast events from cassettes to Nostr relays
    #[command(hide = true)]
    Cast {
        /// Input cassette files to broadcast
        cassettes: Vec<PathBuf>,
        
        /// Target relay URLs
        #[arg(short, long)]
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
    
    /// Start a WebSocket server to serve cassettes as a Nostr relay
    Listen {
        /// Cassette files to serve (supports globs like "*.wasm" or "dir/*.wasm")
        cassettes: Vec<String>,
        
        /// Port to listen on (finds an available port if not specified)
        #[arg(short, long)]
        port: Option<u16>,
        
        /// Bind address
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
        
        /// Enable HTTPS/WSS with auto-generated self-signed certificate
        #[arg(long)]
        tls: bool,
        
        /// Path to TLS certificate file (for custom certificate)
        #[arg(long)]
        tls_cert: Option<PathBuf>,
        
        /// Path to TLS key file (for custom certificate)
        #[arg(long)]
        tls_key: Option<PathBuf>,
        
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Run a cassette deck - continuously record and serve cassettes
    Deck {
        /// Operation mode: 'relay' (writable relay) or 'record' (record from relays)
        #[arg(short, long, default_value = "relay")]
        mode: String,
        
        /// Relay URLs to record from (for record mode)
        #[arg(short, long, num_args = 1.., value_delimiter = ' ')]
        relays: Vec<String>,
        
        /// Base name for cassettes (will append timestamp)
        #[arg(short, long, default_value = "deck")]
        name: String,
        
        /// Output directory for cassettes
        #[arg(short, long, default_value = "./deck")]
        output: PathBuf,
        
        /// Port to serve on
        #[arg(short, long, default_value = "7777")]
        port: u16,
        
        /// Bind address
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
        
        /// Maximum events per cassette (triggers rotation)
        #[arg(short = 'e', long, default_value = "10000")]
        event_limit: usize,
        
        /// Maximum cassette size in MB (triggers rotation)
        #[arg(short = 's', long, default_value = "100")]
        size_limit: usize,
        
        /// Recording duration per cassette in seconds (0 = no time limit)
        #[arg(short = 'd', long, default_value = "3600")]
        duration: u64,
        
        /// Filter JSON for recording
        #[arg(short, long)]
        filter: Option<String>,
        
        /// Event kinds to record
        #[arg(short, long)]
        kinds: Vec<i64>,
        
        /// Authors to filter
        #[arg(short, long)]
        authors: Vec<String>,
        
        /// Enable NIP-11 support
        #[arg(long)]
        _nip_11: bool,
        
        /// Enable NIP-45 (COUNT) support
        #[arg(long)]
        nip_45: bool,
        
        /// Enable NIP-50 (search) support
        #[arg(long)]
        nip_50: bool,
        
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
        
        /// Skip event validation
        #[arg(long)]
        _skip_validation: bool,
        
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


#[cfg(feature = "deck")]
/// Initialize persistent embedded tools directory for deck mode
fn init_embedded_tools_dir(output_dir: &PathBuf) -> Result<PathBuf> {
    let tools_dir = output_dir.join(".cassette-tools-embedded");
    fs::create_dir_all(&tools_dir)?;
    
    // Extract embedded tools once at startup
    if !tools_dir.join("Cargo.toml").exists() {
        use crate::embedded_cassette_tools::get_embedded_tools_dir;
        let embedded_dir = get_embedded_tools_dir();
        
        // Copy all files from embedded directory to tools_dir
        let mut generator = generator::CassetteGenerator::new(
            output_dir.clone(),
            "temp",
            &output_dir,
        );
        generator.copy_dir_contents(embedded_dir, &tools_dir)?;
        println!("📦 Extracted embedded cassette-tools to {}", tools_dir.display());
    }
    
    Ok(tools_dir)
}

/// Process the deck command in relay mode - run a writable relay with cassette backend
async fn process_deck_relay_mode(
    base_name: &str,
    output_dir: &PathBuf,
    port: u16,
    bind_address: &str,
    event_limit: usize,
    size_limit: usize,
    duration: u64,
    _nip_11: bool,
    nip_45: bool,
    nip_50: bool,
    verbose: bool,
    skip_validation: bool,
    nip11_args: &Nip11Args,
) -> Result<()> {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tokio::net::TcpListener;
    use std::time::SystemTime;
    
    println!("🎛️  Starting Cassette Deck in RELAY mode");
    println!("🌐 Accepting events on: {}:{}", bind_address, port);
    println!("📼 Output directory: {}", output_dir.display());
    println!("📊 Rotation: {} events / {} MB / {} seconds", event_limit, size_limit, duration);
    
    // Create output directory
    fs::create_dir_all(output_dir)?;
    
    // Create a persistent directory for embedded cassette-tools
    #[cfg(feature = "deck")]
    let embedded_tools_dir = Arc::new(init_embedded_tools_dir(output_dir)?);
    
    // Shared state for cassettes and current recording
    let active_cassettes: Arc<RwLock<Vec<(PathBuf, Module, Engine)>>> = Arc::new(RwLock::new(Vec::new()));
    let recording_state = Arc::new(RwLock::new(RecordingState {
        current_events: Vec::new(),
        event_count: 0,
        start_time: SystemTime::now(),
        current_size: 0,
        is_compiling: false,
    }));
    let _event_store = Arc::new(RwLock::new(DeckEventStore::new()));
    
    // Load existing cassettes from output directory
    {
        let pattern = output_dir.join("*.wasm");
        let pattern_str = pattern.to_string_lossy();
        let mut loaded_count = 0;
        
        for entry in glob(&pattern_str)? {
            match entry {
                Ok(path) => {
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "wasm") {
                        match fs::read(&path) {
                            Ok(wasm_bytes) => {
                                let engine = wasmtime::Engine::default();
                                match Module::new(&engine, &wasm_bytes) {
                                    Ok(module) => {
                                        // Just load the cassette module - we'll query events at runtime using COUNT
                                        let mut cassettes = active_cassettes.write().await;
                                        cassettes.push((path.clone(), module, engine));
                                        loaded_count += 1;
                                        if verbose {
                                            println!("📼 Loaded cassette module: {}", path.display());
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("⚠️  Failed to load cassette {}: {}", path.display(), e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("⚠️  Failed to read cassette {}: {}", path.display(), e);
                            }
                        }
                    }
                }
                Err(e) => {
                    if verbose {
                        eprintln!("⚠️  Error reading directory: {}", e);
                    }
                }
            }
        }
        
        if loaded_count > 0 {
            println!("📚 Loaded {} existing cassette(s)", loaded_count);
        }
    }
    
    // Start the WebSocket relay server
    let addr = format!("{}:{}", bind_address, port);
    let listener = TcpListener::bind(&addr).await?;
    println!("🌐 Deck relay listening on ws://{}", addr);
    
    // Start rotation monitor
    let mut rotation_handle = {
        let recording_state = recording_state.clone();
        let active_cassettes = active_cassettes.clone();
        let output_dir = output_dir.clone();
        let base_name = base_name.to_string();
        let nip11_args = nip11_args.clone();
        #[cfg(feature = "deck")] 
        let embedded_tools_dir = embedded_tools_dir.clone();
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                
                let should_rotate = {
                    let state = recording_state.read().await;
                    state.event_count >= event_limit ||
                    state.current_size >= size_limit * 1024 * 1024 ||
                    (duration > 0 && state.start_time.elapsed().unwrap().as_secs() >= duration)
                };
                
                if should_rotate {
                    if let Err(e) = rotate_cassette(
                        &recording_state,
                        &active_cassettes,
                        &output_dir,
                        &base_name,
                        _nip_11,
                        nip_45,
                        nip_50,
                        &nip11_args,
                        verbose,
                        #[cfg(feature = "deck")] &embedded_tools_dir,
                    ).await {
                        eprintln!("❌ Failed to rotate cassette: {}", e);
                    }
                }
            }
        })
    };
    
    // Accept connections
    loop {
        tokio::select! {
            Ok((stream, addr)) = listener.accept() => {
                if verbose {
                    println!("📡 New connection from: {}", addr);
                }
                let cassettes = active_cassettes.clone();
                let recording = recording_state.clone();
                let store = _event_store.clone();
                let skip_val = skip_validation;
                tokio::spawn(handle_deck_relay_connection(stream, cassettes, recording, store, skip_val, verbose));
            }
            _ = &mut rotation_handle => {
                eprintln!("⚠️  Rotation handler stopped");
                break;
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n⏹️  Shutting down cassette deck...");
                
                // Final rotation if there are pending events
                let state = recording_state.read().await;
                if state.event_count > 0 {
                    println!("💾 Saving final cassette...");
                    drop(state);
                    rotate_cassette(
                        &recording_state,
                        &active_cassettes,
                        &output_dir,
                        &base_name,
                        _nip_11,
                        nip_45,
                        nip_50,
                        &nip11_args,
                        verbose,
                        #[cfg(feature = "deck")] &embedded_tools_dir,
                    ).await?;
                }
                break;
            }
        }
    }
    
    Ok(())
}


// Helper function to check if an event exists in any loaded cassette using COUNT
async fn check_event_exists_in_cassettes(
    cassettes: &Arc<RwLock<Vec<(PathBuf, Module, Engine)>>>,
    event_id: &str,
) -> bool {
    let cassettes_guard = cassettes.read().await;
    
    for (_path, module, engine) in cassettes_guard.iter() {
        let mut store = Store::new(engine, ());
        
        if let Ok(instance) = Instance::new(&mut store, module, &[]) {
            // Try to query for this specific event ID using COUNT
            if let Ok(send_func) = instance.get_typed_func::<(i32, i32), i32>(&mut store, "send")
                .or_else(|_| instance.get_typed_func::<(i32, i32), i32>(&mut store, "req")) {
                if let Ok(alloc_func) = instance.get_typed_func::<i32, i32>(&mut store, "alloc_buffer") {
                    if let Some(memory) = instance.get_memory(&mut store, "memory") {
                        let dealloc_func = instance.get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string").ok();
                        
                        // Create a COUNT query for this specific event ID
                        let count_msg = json!(["COUNT", "check-event", {"ids": [event_id]}]);
                        let msg_bytes = count_msg.to_string().into_bytes();
                        
                        if let Ok(msg_ptr) = alloc_func.call(&mut store, msg_bytes.len() as i32) {
                            if msg_ptr != 0 {
                                if memory.write(&mut store, msg_ptr as usize, &msg_bytes).is_ok() {
                                    if let Ok(result_ptr) = send_func.call(&mut store, (msg_ptr, msg_bytes.len() as i32)) {
                                        if let Some(dealloc) = &dealloc_func {
                                            let _ = dealloc.call(&mut store, (msg_ptr, msg_bytes.len() as i32));
                                        }
                                        
                                        if result_ptr != 0 {
                                            if let Ok(result) = read_string_from_memory(&mut store, &instance, &memory, result_ptr) {
                                                if let Ok(parsed) = serde_json::from_str::<Vec<Value>>(&result) {
                                                    // Check if we got a COUNT response
                                                    if parsed.len() >= 3 && parsed[0].as_str() == Some("COUNT") {
                                                        if let Some(count_obj) = parsed.get(2) {
                                                            if let Some(count) = count_obj.get("count").and_then(|c| c.as_u64()) {
                                                                if count > 0 {
                                                                    return true;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    false
}

// Helper function to handle relay mode connections (writable relay)
async fn handle_deck_relay_connection(
    stream: TcpStream,
    active_cassettes: Arc<RwLock<Vec<(PathBuf, Module, Engine)>>>,
    recording_state: Arc<RwLock<RecordingState>>,
    _event_store: Arc<RwLock<DeckEventStore>>,
    skip_validation: bool,
    verbose: bool,
) -> Result<()> {
    use tokio_tungstenite::tungstenite::Message;
    
    // Check if this is an HTTP request for NIP-11
    let mut buf = [0u8; 1024];
    let n = stream.peek(&mut buf).await?;
    let peek_data = &buf[..n];
    
    if peek_data.starts_with(b"GET ") {
        // Parse the HTTP request to check headers
        let request = String::from_utf8_lossy(peek_data);
        
        // Check if this is a NIP-11 request by looking for the Accept header
        let has_nip11_header = request.lines().any(|line| {
            line.to_lowercase().starts_with("accept:") && 
            line.contains("application/nostr+json")
        });
        
        if has_nip11_header {
            // Handle HTTP request for NIP-11
            let cassettes = active_cassettes.read().await;
            if let Some((_, module, engine)) = cassettes.first() {
                let mut store = Store::new(engine, ());
                let instance = Instance::new(&mut store, module, &[])?;
                
                if let Ok(info_func) = instance.get_typed_func::<(), i32>(&mut store, "info") {
                    let info_ptr = info_func.call(&mut store, ())?;
                    if info_ptr != 0 {
                        let memory = instance.get_memory(&mut store, "memory").unwrap();
                        let info_str = read_string_from_memory(&mut store, &instance, &memory, info_ptr)?;
                        
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/nostr+json\r\nContent-Length: {}\r\n\r\n{}",
                            info_str.len(),
                            info_str
                        );
                        
                        stream.try_write(response.as_bytes())?;
                        return Ok(());
                    }
                }
            }
            // If we can't provide NIP-11 info, return 404
            let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
            stream.try_write(response.as_bytes())?;
            return Ok(());
        }
        // If it's not a NIP-11 request, fall through to WebSocket handling
    }
    
    // Handle WebSocket connection
    let ws_stream = accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();
    
    // Track subscriptions for this connection
    let mut subscriptions: HashMap<String, Value> = HashMap::new();
    
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if verbose {
                    println!("📨 Received message: {}", text);
                }
                
                // First, try to parse as JSON
                let parsed = match serde_json::from_str::<Value>(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        let notice = json!(["NOTICE", format!("Invalid JSON: {}", e)]);
                        write.send(Message::Text(notice.to_string())).await?;
                        continue;
                    }
                };
                
                // Check if it's an array (required for NIP-01 messages)
                let arr = match parsed.as_array() {
                    Some(a) => a,
                    None => {
                        let notice = json!(["NOTICE", "Invalid message format: expected array"]);
                        write.send(Message::Text(notice.to_string())).await?;
                        continue;
                    }
                };
                
                // Check minimum array length
                if arr.is_empty() {
                    let notice = json!(["NOTICE", "Invalid message: empty array"]);
                    write.send(Message::Text(notice.to_string())).await?;
                    continue;
                }
                
                // Get message type
                let msg_type = match arr[0].as_str() {
                    Some(t) => t,
                    None => {
                        let notice = json!(["NOTICE", "Invalid message: first element must be a string"]);
                        write.send(Message::Text(notice.to_string())).await?;
                        continue;
                    }
                };
                
                if verbose {
                    println!("📨 Message type detected: {}", msg_type);
                }
                
                match msg_type {
                    "EVENT" => {
                        let event_start = std::time::Instant::now();
                        
                        // EVENT messages must have exactly 2 elements: ["EVENT", event_object]
                        if arr.len() != 2 {
                            let notice = json!(["NOTICE", "Invalid EVENT message: expected 2 elements"]);
                            write.send(Message::Text(notice.to_string())).await?;
                            continue;
                        }
                        
                        let event = match arr.get(1) {
                            Some(e) if e.is_object() => e,
                            _ => {
                                let notice = json!(["NOTICE", "Invalid EVENT: second element must be an event object"]);
                                write.send(Message::Text(notice.to_string())).await?;
                                continue;
                            }
                        };
                        
                        // Validate event if needed
                        if !skip_validation {
                            let validation_start = std::time::Instant::now();
                            if let Err(e) = validate_event(event) {
                                let validation_duration = validation_start.elapsed();
                                if verbose {
                                    println!("⏱️  Event validation took: {:?} (failed)", validation_duration);
                                }
                                let notice = json!(["NOTICE", format!("Invalid event: {}", e)]);
                                write.send(Message::Text(notice.to_string())).await?;
                                continue;
                            }
                            let validation_duration = validation_start.elapsed();
                            if verbose {
                                println!("⏱️  Event validation took: {:?}", validation_duration);
                            }
                        }
                        
                        let event_id = event.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        
                        // First check if event exists in cassettes
                        let cassette_check_start = std::time::Instant::now();
                        let exists_in_cassettes = check_event_exists_in_cassettes(&active_cassettes, event_id).await;
                        let cassette_check_duration = cassette_check_start.elapsed();
                        
                        if verbose {
                            println!("⏱️  Cassette duplicate check took: {:?}", cassette_check_duration);
                        }
                        
                        if exists_in_cassettes {
                            if verbose {
                                println!("⚠️  Event {} already exists in cassettes, rejecting", event_id);
                            }
                            
                            // Send OK response with false for duplicate
                            let ok_msg = json!(["OK", event_id, false, "error: duplicate event"]);
                            let ok_msg_str = ok_msg.to_string();
                            
                            if verbose {
                                println!("📤 Sending response: {}", ok_msg_str);
                            }
                            
                            write.send(Message::Text(ok_msg_str)).await?;
                            continue;
                        }
                        
                        // Try to add event to the store
                        let store_start = std::time::Instant::now();
                        let (added, replaced) = {
                            let mut store = _event_store.write().await;
                            store.add_event(event.clone())
                        };
                        let store_duration = store_start.elapsed();
                        
                        if verbose {
                            println!("⏱️  In-memory store operation took: {:?}", store_duration);
                        }
                        
                        if !added {
                            if verbose {
                                println!("⚠️  Event {} already exists, rejecting", event_id);
                            }
                            
                            // Send OK response with false for duplicate
                            let ok_msg = json!(["OK", event_id, false, "error: duplicate event"]);
                            let ok_msg_str = ok_msg.to_string();
                            
                            if verbose {
                                println!("📤 Sending response: {}", ok_msg_str);
                            }
                            
                            write.send(Message::Text(ok_msg_str)).await?;
                        } else {
                            if let Some(old_id) = replaced {
                                if verbose {
                                    println!("♻️  Event replaced older event: {}", old_id);
                                }
                                // Remove the old event from current buffer if it's there
                                let mut state = recording_state.write().await;
                                state.current_events.retain(|e| {
                                    e.get("id").and_then(|i| i.as_str()) != Some(&old_id)
                                });
                                drop(state);
                            }
                            
                            // Add to recording state
                            let mut state = recording_state.write().await;
                            state.current_events.push(event.clone());
                            state.event_count = state.current_events.len();
                            state.current_size = state.current_events.iter()
                                .map(|e| serde_json::to_string(e).unwrap_or_default().len())
                                .sum();
                            
                            if verbose {
                                println!("📥 EVENT received: {} (total: {})", event_id, state.event_count);
                            }
                            
                            // Send OK response with true for successful add
                            let ok_msg = json!(["OK", event_id, true, ""]);
                            let ok_msg_str = ok_msg.to_string();
                        
                            if verbose {
                                println!("📤 Sending response: {}", ok_msg_str);
                            }
                            
                            write.send(Message::Text(ok_msg_str)).await?;
                        }
                        
                        let event_duration = event_start.elapsed();
                        if verbose {
                            println!("⏱️  Total EVENT processing took: {:?}", event_duration);
                        }
                    }
                    "REQ" => {
                        if verbose {
                            println!("📨 Processing REQ message...");
                        }
                        
                        let req_start = std::time::Instant::now();
                        
                        // REQ messages must have at least 3 elements: ["REQ", subscription_id, filter, ...]
                        if arr.len() < 3 {
                            let notice = json!(["NOTICE", "Invalid REQ message: expected at least 3 elements"]);
                            write.send(Message::Text(notice.to_string())).await?;
                            continue;
                        }
                        
                        let sub_id = match arr[1].as_str() {
                            Some(id) => id,
                            None => {
                                let notice = json!(["NOTICE", "Invalid REQ: subscription ID must be a string"]);
                                write.send(Message::Text(notice.to_string())).await?;
                                continue;
                            }
                        };
                        
                        // Validate filters
                        let filters = &arr[2..];
                        for (i, filter) in filters.iter().enumerate() {
                            if !filter.is_object() {
                                let notice = json!(["NOTICE", format!("Invalid REQ: filter {} must be an object", i + 1)]);
                                write.send(Message::Text(notice.to_string())).await?;
                                continue;
                            }
                        }
                        
                        if verbose {
                            println!("📖 REQ received - Subscription ID: {}, Filters: {:?}", sub_id, filters);
                        }
                        
                        // Store subscription
                        subscriptions.insert(sub_id.to_string(), json!(filters));
                        
                        if verbose {
                            println!("📖 Starting event collection...");
                        }
                        
                        // Collect ALL events from all sources first
                        let mut all_collected_events = Vec::new();
                        
                        // 1. Get events from current recording buffer
                        let current_events = {
                            let state = recording_state.read().await;
                            state.current_events.clone()
                        };
                        
                        // Add matching events from buffer
                        for event in &current_events {
                            if event_matches_filters(event, filters) {
                                all_collected_events.push(event.clone());
                            }
                        }
                        
                        if verbose {
                            println!("📊 Found {} matching events in current buffer", all_collected_events.len());
                        }
                        
                        // 2. Query ALL cassettes to get complete state
                        let cassette_query_start = std::time::Instant::now();
                        let cassettes = active_cassettes.read().await;
                        let mut total_cassette_events = 0;
                        
                        if verbose {
                            println!("📖 Found {} cassettes to query", cassettes.len());
                        }
                        
                        for (path_idx, (path, module, engine)) in cassettes.iter().enumerate() {
                            if verbose {
                                println!("📖 Querying cassette {}: {}", path_idx, path.display());
                            }
                            let mut store = Store::new(engine, ());
                            let instance = Instance::new(&mut store, module, &[])?;
                            
                            if let Ok(send_func) = instance.get_typed_func::<(i32, i32), i32>(&mut store, "send")
                .or_else(|_| instance.get_typed_func::<(i32, i32), i32>(&mut store, "req")) {
                                if let Ok(alloc_func) = instance.get_typed_func::<i32, i32>(&mut store, "alloc_buffer") {
                                    let memory = instance.get_memory(&mut store, "memory").unwrap();
                                    let dealloc_func = instance.get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string").ok();
                                    
                                    // Keep querying this cassette until we get EOSE
                                    let mut got_eose = false;
                                    let mut cassette_events = 0;
                                    let mut consecutive_empty_responses = 0;
                                    const MAX_EMPTY_RESPONSES: usize = 2;
                                    
                                    // Send initial REQ to establish subscription
                                    let req_msg = if filters.is_empty() {
                                        json!(["REQ", sub_id, {}])
                                    } else {
                                        let mut msg_arr = vec![json!("REQ"), json!(sub_id)];
                                        msg_arr.extend(filters.iter().cloned());
                                        json!(msg_arr)
                                    };
                                    
                                    while !got_eose && consecutive_empty_responses < MAX_EMPTY_RESPONSES {
                                        let events_before = cassette_events;
                                        // For subsequent calls, just send the same REQ to continue streaming
                                        let msg_bytes = req_msg.to_string().into_bytes();
                                        
                                        // Debug: print what we're sending
                                        if verbose {
                                            println!("  🔍 Sending to cassette: {}", req_msg);
                                        }
                                        let msg_ptr = alloc_func.call(&mut store, msg_bytes.len() as i32)?;
                                        
                                        if msg_ptr == 0 {
                                            break;
                                        }
                                        
                                        memory.write(&mut store, msg_ptr as usize, &msg_bytes)?;
                                        let result_ptr = send_func.call(&mut store, (msg_ptr, msg_bytes.len() as i32))?;
                                        
                                        if let Some(dealloc) = &dealloc_func {
                                            dealloc.call(&mut store, (msg_ptr, msg_bytes.len() as i32))?;
                                        }
                                        
                                        if result_ptr == 0 {
                                            break;
                                        }
                                        
                                        let result = read_string_from_memory(&mut store, &instance, &memory, result_ptr)?;
                                        
                                        // Try to deallocate the result pointer
                                        if let Ok(get_size_func) = instance.get_typed_func::<i32, i32>(&mut store, "get_allocation_size") {
                                            if let Ok(size) = get_size_func.call(&mut store, result_ptr) {
                                                if let Some(dealloc) = &dealloc_func {
                                                    let _ = dealloc.call(&mut store, (result_ptr, size));
                                                }
                                            }
                                        }
                                        
                                        // Parse the response
                                        if verbose {
                                            println!("  🔍 Cassette response: {}", result);
                                        }
                                        
                                        if let Ok(parsed) = serde_json::from_str::<Vec<Value>>(&result) {
                                            if parsed.len() >= 2 {
                                                match parsed[0].as_str() {
                                                    Some("EVENT") => {
                                                        if parsed.len() >= 3 {
                                                            if let Some(event) = parsed.get(2) {
                                                                // Validate event if needed
                                                                if !skip_validation {
                                                                    if let Err(e) = validate_event(event) {
                                                                        if verbose {
                                                                            println!("⚠️  Skipping invalid event from cassette: {}", e);
                                                                        }
                                                                        continue;
                                                                    }
                                                                }
                                                                
                                                                // Collect the event
                                                                all_collected_events.push(event.clone());
                                                                cassette_events += 1;
                                                                
                                                                if verbose {
                                                                    println!("  📥 Collected event: {}", 
                                                                        event.get("id").and_then(|i| i.as_str()).unwrap_or("?"));
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Some("EOSE") => {
                                                        got_eose = true;
                                                    }
                                                    _ => {
                                                        // Ignore other messages during collection
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Check if we got any new events in this iteration
                                        if cassette_events == events_before {
                                            consecutive_empty_responses += 1;
                                        } else {
                                            consecutive_empty_responses = 0;
                                        }
                                    }
                                    
                                    if verbose && cassette_events > 0 {
                                        println!("📊 Found {} events in cassette {}", cassette_events, path_idx);
                                    }
                                    
                                    total_cassette_events += cassette_events;
                                }
                            }
                        }
                        
                        let cassette_query_duration = cassette_query_start.elapsed();
                        if verbose {
                            println!("⏱️  Cassette queries took: {:?} (found {} events across {} cassettes)", 
                                cassette_query_duration, total_cassette_events, cassettes.len());
                        }
                        
                        if verbose {
                            println!("📊 Total collected events before deduplication: {}", all_collected_events.len());
                        }
                        
                        // 3. Apply deduplication for replaceable events
                        let mut events_by_id: HashMap<String, Value> = HashMap::new();
                        let mut replaceable_events: HashMap<(String, i64), String> = HashMap::new();
                        let mut param_replaceable_events: HashMap<(String, i64, String), String> = HashMap::new();
                        
                        for event in all_collected_events {
                            let event_id = event.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                            let event_pubkey = event.get("pubkey").and_then(|p| p.as_str()).unwrap_or("").to_string();
                            let event_kind = event.get("kind").and_then(|k| k.as_i64()).unwrap_or(0);
                            let event_created_at = event.get("created_at").and_then(|t| t.as_i64()).unwrap_or(0);
                            
                            if event_id.is_empty() || event_pubkey.is_empty() {
                                continue; // Skip invalid events
                            }
                            
                            // Check if it's a replaceable event
                            if event_kind == 0 || event_kind == 3 || (event_kind >= 10000 && event_kind <= 19999) {
                                let key = (event_pubkey.clone(), event_kind);
                                if let Some(existing_id) = replaceable_events.get(&key) {
                                    if let Some(existing_event) = events_by_id.get(existing_id) {
                                        if let Some(existing_created_at) = existing_event.get("created_at").and_then(|t| t.as_i64()) {
                                            if event_created_at > existing_created_at {
                                                // Replace with newer event
                                                events_by_id.remove(existing_id);
                                                events_by_id.insert(event_id.clone(), event);
                                                replaceable_events.insert(key, event_id);
                                            }
                                        }
                                    }
                                } else {
                                    events_by_id.insert(event_id.clone(), event);
                                    replaceable_events.insert(key, event_id);
                                }
                            }
                            // Check if it's a parameterized replaceable event
                            else if event_kind >= 30000 && event_kind <= 39999 {
                                let d_tag = event.get("tags")
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
                                    .unwrap_or("")
                                    .to_string();
                                
                                let key = (event_pubkey.clone(), event_kind, d_tag);
                                if let Some(existing_id) = param_replaceable_events.get(&key) {
                                    if let Some(existing_event) = events_by_id.get(existing_id) {
                                        if let Some(existing_created_at) = existing_event.get("created_at").and_then(|t| t.as_i64()) {
                                            if event_created_at > existing_created_at {
                                                // Replace with newer event
                                                events_by_id.remove(existing_id);
                                                events_by_id.insert(event_id.clone(), event);
                                                param_replaceable_events.insert(key, event_id);
                                            }
                                        }
                                    }
                                } else {
                                    events_by_id.insert(event_id.clone(), event);
                                    param_replaceable_events.insert(key, event_id);
                                }
                            }
                            // Regular event - just check for duplicates
                            else {
                                events_by_id.entry(event_id).or_insert(event);
                            }
                        }
                        
                        // Convert back to a vector
                        let mut final_events: Vec<Value> = events_by_id.into_values().collect();
                        
                        if verbose {
                            println!("📊 Events after deduplication: {}", final_events.len());
                        }
                        
                        // 4. Sort events by created_at (newest first) 
                        final_events.sort_by(|a, b| {
                            let a_time = a.get("created_at").and_then(|t| t.as_i64()).unwrap_or(0);
                            let b_time = b.get("created_at").and_then(|t| t.as_i64()).unwrap_or(0);
                            b_time.cmp(&a_time)
                        });
                        
                        // 5. Apply limit if specified in any filter
                        let mut max_limit: Option<u64> = None;
                        for filter in filters {
                            if let Some(limit) = filter.get("limit").and_then(|l| l.as_u64()) {
                                max_limit = Some(max_limit.map_or(limit, |m: u64| m.max(limit)));
                            }
                        }
                        
                        if let Some(limit) = max_limit {
                            final_events.truncate(limit as usize);
                            if verbose {
                                println!("📊 Applied limit of {}, final event count: {}", limit, final_events.len());
                            }
                        }
                        
                        // 6. Send all events
                        for event in final_events {
                            let event_msg = json!(["EVENT", sub_id, event]);
                            let event_msg_str = event_msg.to_string();
                            
                            if verbose {
                                println!("📤 Sending EVENT: {}", 
                                    event.get("id").and_then(|i| i.as_str()).unwrap_or("?"));
                            }
                            
                            write.send(Message::Text(event_msg_str)).await?;
                        }
                        
                        // 7. Send EOSE
                        let eose_msg = json!(["EOSE", sub_id]);
                        let eose_msg_str = eose_msg.to_string();
                        
                        if verbose {
                            println!("📤 Sending EOSE for subscription: {}", sub_id);
                        }
                        
                        write.send(Message::Text(eose_msg_str)).await?;
                        
                        let req_duration = req_start.elapsed();
                        if verbose {
                            println!("⏱️  Total REQ processing took: {:?}", req_duration);
                        }
                    }
                    "CLOSE" => {
                        // CLOSE messages must have exactly 2 elements: ["CLOSE", subscription_id]
                        if arr.len() != 2 {
                            let notice = json!(["NOTICE", "Invalid CLOSE message: expected 2 elements"]);
                            write.send(Message::Text(notice.to_string())).await?;
                            continue;
                        }
                        
                        let sub_id = match arr[1].as_str() {
                            Some(id) => id,
                            None => {
                                let notice = json!(["NOTICE", "Invalid CLOSE: subscription ID must be a string"]);
                                write.send(Message::Text(notice.to_string())).await?;
                                continue;
                            }
                        };
                        
                        if verbose {
                            println!("🔚 CLOSE received for subscription: {}", sub_id);
                        }
                        
                        subscriptions.remove(sub_id);
                    }
                    "COUNT" => {
                        // COUNT messages must have at least 3 elements: ["COUNT", subscription_id, filter, ...]
                        if arr.len() < 3 {
                            let notice = json!(["NOTICE", "Invalid COUNT message: expected at least 3 elements"]);
                            write.send(Message::Text(notice.to_string())).await?;
                            continue;
                        }
                        
                        let sub_id = match arr[1].as_str() {
                            Some(id) => id,
                            None => {
                                let notice = json!(["NOTICE", "Invalid COUNT: subscription ID must be a string"]);
                                write.send(Message::Text(notice.to_string())).await?;
                                continue;
                            }
                        };
                        
                        let filters = &arr[2..];
                        for (i, filter) in filters.iter().enumerate() {
                            if !filter.is_object() {
                                let notice = json!(["NOTICE", format!("Invalid COUNT: filter {} must be an object", i + 1)]);
                                write.send(Message::Text(notice.to_string())).await?;
                                continue;
                            }
                        }
                        
                        if verbose {
                            println!("📊 COUNT received - Subscription ID: {}, Filters: {:?}", sub_id, filters);
                        }
                                    
                                    let mut total_count = 0;
                                    
                                    // Count in current buffer
                                    let current_events = {
                                        let state = recording_state.read().await;
                                        state.current_events.clone()
                                    };
                                    
                                    for event in &current_events {
                                        if event_matches_filters(event, filters) {
                                            total_count += 1;
                                        }
                                    }
                                    
                                    // Count in cassettes
                                    let cassettes = active_cassettes.read().await;
                                    for (_path, module, engine) in cassettes.iter() {
                                        let mut store = Store::new(engine, ());
                                        let instance = Instance::new(&mut store, module, &[])?;
                                        
                                        // Send COUNT to cassette
                                        if let Ok(send_func) = instance.get_typed_func::<(i32, i32), i32>(&mut store, "send")
                .or_else(|_| instance.get_typed_func::<(i32, i32), i32>(&mut store, "req")) {
                                            if let Ok(alloc_func) = instance.get_typed_func::<i32, i32>(&mut store, "alloc_buffer") {
                                                let memory = instance.get_memory(&mut store, "memory").unwrap();
                                                let dealloc_func = instance.get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string").ok();
                                                
                                                let count_msg = if filters.is_empty() {
                                                    json!(["COUNT", sub_id, {}])
                                                } else {
                                                    let mut msg_arr = vec![json!("COUNT"), json!(sub_id)];
                                                    msg_arr.extend(filters.iter().cloned());
                                                    json!(msg_arr)
                                                };
                                                let msg_bytes = count_msg.to_string().into_bytes();
                                                let msg_ptr = alloc_func.call(&mut store, msg_bytes.len() as i32)?;
                                                
                                                if msg_ptr != 0 {
                                                    memory.write(&mut store, msg_ptr as usize, &msg_bytes)?;
                                                    let result_ptr = send_func.call(&mut store, (msg_ptr, msg_bytes.len() as i32))?;
                                                    
                                                    if let Some(dealloc) = &dealloc_func {
                                                        dealloc.call(&mut store, (msg_ptr, msg_bytes.len() as i32))?;
                                                    }
                                                    
                                                    if result_ptr != 0 {
                                                        let result = read_string_from_memory(&mut store, &instance, &memory, result_ptr)?;
                                                        
                                                        // Parse count response
                                                        if let Ok(parsed) = serde_json::from_str::<Vec<Value>>(&result) {
                                                            if parsed.len() >= 3 && parsed[0].as_str() == Some("COUNT") {
                                                                if let Some(count_obj) = parsed[2].as_object() {
                                                                    if let Some(count) = count_obj.get("count").and_then(|c| c.as_u64()) {
                                                                        total_count += count as usize;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    
                                    let count_msg = json!(["COUNT", sub_id, {"count": total_count}]);
                                    let count_msg_str = count_msg.to_string();
                                    
                                    if verbose {
                                        println!("📤 Sending COUNT response: {} events matched", total_count);
                                    }
                                    
                                    write.send(Message::Text(count_msg_str)).await?;
                    }
                    _ => {
                        if verbose {
                            println!("⚠️  Unknown command received: {}", msg_type);
                        }
                        
                        let notice = json!(["NOTICE", format!("Unknown command: {}", msg_type)]);
                        let notice_str = notice.to_string();
                        
                        if verbose {
                            println!("📤 Sending NOTICE: {}", notice_str);
                        }
                        
                        write.send(Message::Text(notice_str)).await?;
                    }
                }
            }
            Ok(Message::Close(_)) => {
                if verbose {
                    println!("🔌 WebSocket connection closed");
                }
                break;
            }
            _ => {}
        }
    }
    
    if verbose {
        println!("👋 Connection handler finished");
    }
    
    Ok(())
}

// Helper function to check if an event matches filters
fn event_matches_filters(event: &Value, filters: &[Value]) -> bool {
    // If no filters provided, match nothing
    if filters.is_empty() {
        return false;
    }
    
    // NIP-01: filters are OR'd together (any filter match = include event)
    for filter in filters {
        if let Some(filter_obj) = filter.as_object() {
            let mut matches = true;
            
            // Check ids (exact match)
            if let Some(ids) = filter_obj.get("ids").and_then(|i| i.as_array()) {
                if let Some(event_id) = event.get("id").and_then(|i| i.as_str()) {
                    let id_matches = ids.iter().any(|id| {
                        if let Some(id_prefix) = id.as_str() {
                            event_id.starts_with(id_prefix)
                        } else {
                            false
                        }
                    });
                    if !id_matches {
                        matches = false;
                    }
                } else {
                    matches = false;
                }
            }
            
            // Check kinds
            if matches && filter_obj.contains_key("kinds") {
                if let Some(kinds) = filter_obj.get("kinds").and_then(|k| k.as_array()) {
                    if let Some(event_kind) = event.get("kind").and_then(|k| k.as_i64()) {
                        if !kinds.iter().any(|k| k.as_i64() == Some(event_kind)) {
                            matches = false;
                        }
                    } else {
                        matches = false;
                    }
                }
            }
            
            // Check authors (prefix match)
            if matches && filter_obj.contains_key("authors") {
                if let Some(authors) = filter_obj.get("authors").and_then(|a| a.as_array()) {
                    if let Some(event_author) = event.get("pubkey").and_then(|p| p.as_str()) {
                        let author_matches = authors.iter().any(|a| {
                            if let Some(author_prefix) = a.as_str() {
                                event_author.starts_with(author_prefix)
                            } else {
                                false
                            }
                        });
                        if !author_matches {
                            matches = false;
                        }
                    } else {
                        matches = false;
                    }
                }
            }
            
            // Check since
            if matches && filter_obj.contains_key("since") {
                if let Some(since) = filter_obj.get("since").and_then(|s| s.as_i64()) {
                    if let Some(created_at) = event.get("created_at").and_then(|t| t.as_i64()) {
                        if created_at < since {
                            matches = false;
                        }
                    } else {
                        matches = false;
                    }
                }
            }
            
            // Check until
            if matches && filter_obj.contains_key("until") {
                if let Some(until) = filter_obj.get("until").and_then(|u| u.as_i64()) {
                    if let Some(created_at) = event.get("created_at").and_then(|t| t.as_i64()) {
                        if created_at > until {
                            matches = false;
                        }
                    } else {
                        matches = false;
                    }
                }
            }
            
            // Check tag filters (e.g., #e, #p, #t)
            if matches {
                for (key, values) in filter_obj {
                    if key.starts_with('#') && key.len() == 2 {
                        let tag_name = &key[1..];
                        if let Some(filter_values) = values.as_array() {
                            if let Some(event_tags) = event.get("tags").and_then(|t| t.as_array()) {
                                let tag_matches = filter_values.iter().any(|filter_value| {
                                    if let Some(filter_str) = filter_value.as_str() {
                                        event_tags.iter().any(|tag| {
                                            if let Some(tag_array) = tag.as_array() {
                                                tag_array.len() >= 2 &&
                                                tag_array[0].as_str() == Some(tag_name) &&
                                                tag_array[1].as_str() == Some(filter_str)
                                            } else {
                                                false
                                            }
                                        })
                                    } else {
                                        false
                                    }
                                });
                                if !tag_matches {
                                    matches = false;
                                }
                            } else {
                                matches = false;
                            }
                        }
                    }
                }
            }
            
            // If all conditions in this filter match, the event matches
            if matches {
                return true;
            }
        }
    }
    
    // No filters matched
    false
}

// Helper function to validate an event
fn validate_event(event: &Value) -> Result<()> {
    // Basic validation
    if !event.is_object() {
        return Err(anyhow!("Event must be an object"));
    }
    
    // Check required fields
    let required_fields = ["id", "pubkey", "created_at", "kind", "tags", "content", "sig"];
    for field in &required_fields {
        if event.get(field).is_none() {
            return Err(anyhow!("Missing required field: {}", field));
        }
    }
    
    // TODO: Add signature verification
    
    Ok(())
}

/// Process the deck command in record mode - continuously record from relays and serve cassettes
async fn process_deck_record_mode(
    relay_urls: &[String],
    base_name: &str,
    output_dir: &PathBuf,
    port: u16,
    bind_address: &str,
    event_limit: usize,
    size_limit: usize,
    duration: u64,
    filter_json: Option<&str>,
    kinds: &[i64],
    authors: &[String],
    _nip_11: bool,
    nip_45: bool,
    nip_50: bool,
    verbose: bool,
    _skip_validation: bool,
    nip11_args: &Nip11Args,
) -> Result<()> {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tokio::net::TcpListener;
    use std::time::SystemTime;
    
    println!("🎛️  Starting Cassette Deck");
    println!("📡 Recording from: {:?}", relay_urls);
    println!("🌐 Serving on: {}:{}", bind_address, port);
    println!("📼 Output directory: {}", output_dir.display());
    
    // Create output directory
    fs::create_dir_all(output_dir)?;
    
    // Create a persistent directory for embedded cassette-tools
    #[cfg(feature = "deck")]
    let embedded_tools_dir = Arc::new(init_embedded_tools_dir(output_dir)?);
    
    // Shared state for hot-loading cassettes
    let active_cassettes: Arc<RwLock<Vec<(PathBuf, Module, Engine)>>> = Arc::new(RwLock::new(Vec::new()));
    let recording_state = Arc::new(RwLock::new(RecordingState {
        current_events: Vec::new(),
        event_count: 0,
        start_time: SystemTime::now(),
        current_size: 0,
        is_compiling: false,
    }));
    let _event_store = Arc::new(RwLock::new(DeckEventStore::new()));
    
    // Start the WebSocket server
    let server_handle = {
        let active_cassettes = active_cassettes.clone();
        let addr = format!("{}:{}", bind_address, port);
        tokio::spawn(async move {
            let listener = TcpListener::bind(&addr).await?;
            println!("🌐 Deck server listening on ws://{}", addr);
            
            while let Ok((stream, _)) = listener.accept().await {
                let cassettes = active_cassettes.clone();
                tokio::spawn(handle_deck_connection(stream, cassettes));
            }
            
            Ok::<(), anyhow::Error>(())
        })
    };
    
    // Start recording from relays
    let recorder_handle = {
        let active_cassettes = active_cassettes.clone();
        let recording_state = recording_state.clone();
        let relay_urls = relay_urls.to_vec();
        let filter_json = filter_json.map(|s| s.to_string());
        let kinds = kinds.to_vec();
        let authors = authors.to_vec();
        let output_dir = output_dir.to_path_buf();
        let base_name = base_name.to_string();
        let nip11_args = nip11_args.clone();
        tokio::spawn(async move {
            let mut relay_since_timestamps: HashMap<String, i64> = HashMap::new();
            
            loop {
                // Connect to relays and start recording
                let mut handles = vec![];
                
                for relay_url in &relay_urls {
                    let state = recording_state.clone();
                    let url = relay_url.clone();
                    let url_for_handle = url.clone(); // Clone for the handle storage
                    let filter_json = filter_json.clone();
                    let kinds = kinds.to_vec();
                    let authors = authors.iter().cloned().collect::<Vec<_>>();
                    let since = relay_since_timestamps.get(&url).copied();
                    
                    let handle = tokio::spawn(async move {
                        record_from_relay(
                            &url,
                            state,
                            filter_json.as_deref(),
                            &kinds,
                            &authors,
                            since,
                        ).await
                    });
                    handles.push((url_for_handle, handle));
                }
                
                // Monitor recording state and trigger rotation
                let rotation_handle = {
                    let recording_state = recording_state.clone();
                    let active_cassettes = active_cassettes.clone();
                    let output_dir = output_dir.clone();
                    let base_name = base_name.clone();
                    let nip11_args = nip11_args.clone();
                    #[cfg(feature = "deck")]
                    let embedded_tools_dir = embedded_tools_dir.clone();
                    
                    tokio::spawn(async move {
                        loop {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            
                            let should_rotate = {
                                let state = recording_state.read().await;
                                state.event_count >= event_limit ||
                                state.current_size >= size_limit * 1024 * 1024 ||
                                (duration > 0 && state.start_time.elapsed().unwrap().as_secs() >= duration)
                            };
                            
                            if should_rotate {
                                // Rotate cassette
                                if let Err(e) = rotate_cassette(
                                    &recording_state,
                                    &active_cassettes,
                                    &output_dir,
                                    &base_name,
                                    _nip_11,
                                    nip_45,
                                    nip_50,
                                    &nip11_args,
                                    verbose,
                                    #[cfg(feature = "deck")] &embedded_tools_dir,
                                ).await {
                                    eprintln!("❌ Failed to rotate cassette: {}", e);
                                }
                            }
                        }
                    })
                };
                
                // Wait for any handle to complete (error condition)
                tokio::select! {
                    _ = async {
                        // Wait for all handles and collect their results
                        for (url, handle) in handles {
                            match handle.await {
                                Ok(Ok(last_timestamp)) => {
                                    // Update the since timestamp for this relay
                                    relay_since_timestamps.insert(url, last_timestamp);
                                }
                                Ok(Err(e)) => {
                                    eprintln!("⚠️  Error from {}: {}", url, e);
                                }
                                Err(e) => {
                                    eprintln!("⚠️  Task error for {}: {}", url, e);
                                }
                            }
                        }
                    } => {
                        eprintln!("⚠️  Relay connections ended, reconnecting in 5 seconds...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                    _ = rotation_handle => {
                        eprintln!("⚠️  Rotation handler stopped");
                    }
                }
            }
        })
    };
    
    // Wait for both tasks
    tokio::select! {
        result = server_handle => {
            eprintln!("❌ Server stopped: {:?}", result);
        }
        result = recorder_handle => {
            eprintln!("❌ Recorder stopped: {:?}", result);
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n⏹️  Shutting down cassette deck...");
        }
    }
    
    Ok(())
}

// Helper struct for recording state
struct RecordingState {
    current_events: Vec<Value>,
    event_count: usize,
    start_time: SystemTime,
    current_size: usize,
    is_compiling: bool,
}

// Global event store for deck mode with deduplication
struct DeckEventStore {
    // All events by ID
    events_by_id: HashMap<String, Value>,
    // Replaceable events indexed by author + kind (for kinds 0, 3, 10000-19999)
    replaceable_events: HashMap<(String, u64), String>, // (author, kind) -> event_id
    // Parameterized replaceable events indexed by author + kind + d-tag (for kinds 30000-39999)
    param_replaceable_events: HashMap<(String, u64, String), String>, // (author, kind, d_tag) -> event_id
}

impl DeckEventStore {
    fn new() -> Self {
        Self {
            events_by_id: HashMap::new(),
            replaceable_events: HashMap::new(),
            param_replaceable_events: HashMap::new(),
        }
    }
    
    // Check if event should replace an existing one
    fn check_replaceable(&self, event: &Value) -> Option<String> {
        let kind = event.get("kind")?.as_u64()?;
        let pubkey = event.get("pubkey")?.as_str()?;
        
        // Check if it's a replaceable event
        if kind == 0 || kind == 3 || (10000..=19999).contains(&kind) {
            // Regular replaceable
            return self.replaceable_events.get(&(pubkey.to_string(), kind)).cloned();
        } else if (30000..=39999).contains(&kind) {
            // Parameterized replaceable - need d-tag
            let d_tag = event.get("tags")?
                .as_array()?
                .iter()
                .find(|tag| {
                    tag.as_array()
                        .and_then(|t| t.get(0))
                        .and_then(|t| t.as_str())
                        .map(|t| t == "d")
                        .unwrap_or(false)
                })?
                .as_array()?
                .get(1)?
                .as_str()?;
            
            return self.param_replaceable_events.get(&(pubkey.to_string(), kind, d_tag.to_string())).cloned();
        }
        
        None
    }
    
    // Add event to store, returns (added, replaced_event_id)
    fn add_event(&mut self, event: Value) -> (bool, Option<String>) {
        let event_id = match event.get("id").and_then(|i| i.as_str()) {
            Some(id) => id.to_string(),
            None => return (false, None),
        };
        
        // Check if we already have this exact event
        if self.events_by_id.contains_key(&event_id) {
            return (false, None);
        }
        
        // Check if this replaces an existing event
        let replaced = self.check_replaceable(&event);
        if let Some(old_id) = &replaced {
            // Remove the old event
            self.events_by_id.remove(old_id);
        }
        
        // Update indices
        if let (Some(kind), Some(pubkey)) = (event.get("kind").and_then(|k| k.as_u64()), 
                                             event.get("pubkey").and_then(|p| p.as_str())) {
            if kind == 0 || kind == 3 || (10000..=19999).contains(&kind) {
                self.replaceable_events.insert((pubkey.to_string(), kind), event_id.clone());
            } else if (30000..=39999).contains(&kind) {
                if let Some(d_tag) = event.get("tags")
                    .and_then(|t| t.as_array())
                    .and_then(|tags| {
                        tags.iter().find(|tag| {
                            tag.as_array()
                                .and_then(|t| t.get(0))
                                .and_then(|t| t.as_str())
                                .map(|t| t == "d")
                                .unwrap_or(false)
                        })
                    })
                    .and_then(|tag| tag.as_array())
                    .and_then(|t| t.get(1))
                    .and_then(|t| t.as_str()) {
                    self.param_replaceable_events.insert((pubkey.to_string(), kind, d_tag.to_string()), event_id.clone());
                }
            }
        }
        
        // Add the event
        self.events_by_id.insert(event_id.clone(), event);
        
        (true, replaced)
    }
}

// Helper function to record from a single relay
async fn record_from_relay(
    relay_url: &str,
    recording_state: Arc<RwLock<RecordingState>>,
    filter_json: Option<&str>,
    kinds: &[i64],
    authors: &[String],
    initial_since: Option<i64>,
) -> Result<i64> {
    use tokio_tungstenite::tungstenite::Message;
    use futures_util::{StreamExt, SinkExt};
    
    println!("📡 Connecting to {}", relay_url);
    let (ws_stream, _) = connect_async(relay_url).await?;
    let (mut write, mut read) = ws_stream.split();
    println!("✅ Connected to {}", relay_url);
    
    // Create subscription filter
    let mut filter = serde_json::Map::new();
    if let Some(filter_str) = filter_json {
        if let Ok(custom_filter) = serde_json::from_str::<serde_json::Map<String, Value>>(filter_str) {
            for (k, v) in custom_filter {
                filter.insert(k, v);
            }
        }
    }
    if !kinds.is_empty() {
        filter.insert("kinds".to_string(), json!(kinds));
    }
    if !authors.is_empty() {
        filter.insert("authors".to_string(), json!(authors));
    }
    
    // Add since timestamp if provided (for reconnections)
    if let Some(since) = initial_since {
        filter.insert("since".to_string(), json!(since));
        println!("🔄 Resuming from timestamp: {}", since);
    }
    
    // Send subscription
    let req_message = json!(["REQ", "deck-sub", filter]);
    write.send(Message::Text(req_message.to_string())).await?;
    
    // Track if we've received EOSE and latest timestamp
    let mut received_eose = false;
    let mut last_event_time = std::time::Instant::now();
    let mut latest_timestamp = initial_since.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    });
    
    // Read events
    loop {
        // Use timeout to periodically check connection health
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(60),
            read.next()
        ).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                    if let Some(arr) = parsed.as_array() {
                        match arr.get(0).and_then(|v| v.as_str()) {
                            Some("EVENT") => {
                                if arr.len() >= 3 {
                                    if let Some(event) = arr.get(2) {
                                        // Extract created_at timestamp
                                        if let Some(created_at) = event.get("created_at").and_then(|v| v.as_i64()) {
                                            if created_at > latest_timestamp {
                                                latest_timestamp = created_at;
                                            }
                                        }
                                        
                                        let event_size = text.len();
                                        let mut state = recording_state.write().await;
                                        state.current_events.push(event.clone());
                                        state.event_count += 1;
                                        state.current_size += event_size;
                                        last_event_time = std::time::Instant::now();
                                    }
                                }
                            }
                            Some("EOSE") => {
                                if !received_eose {
                                    println!("📍 Received EOSE from {} - continuing to listen for new events", relay_url);
                                    received_eose = true;
                                }
                                // Don't break on EOSE - keep the connection alive
                            }
                            Some("NOTICE") => {
                                if arr.len() >= 2 {
                                    println!("📝 Notice from {}: {}", relay_url, arr[1]);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(Some(Ok(Message::Close(_)))) => {
                println!("🔌 {} closed the connection", relay_url);
                break;
            }
            Ok(Some(Ok(Message::Ping(data)))) => {
                // Respond to ping with pong
                write.send(Message::Pong(data)).await?;
            }
            Ok(Some(Ok(Message::Binary(_)))) => {
                // Ignore binary messages
            }
            Ok(Some(Ok(Message::Pong(_)))) => {
                // Pong received, connection is alive
            }
            Ok(Some(Ok(Message::Frame(_)))) => {
                // Ignore frame messages
            }
            Ok(Some(Err(e))) => {
                eprintln!("❌ WebSocket error from {}: {}", relay_url, e);
                break;
            }
            Ok(None) => {
                println!("🔌 Connection to {} ended", relay_url);
                break;
            }
            Err(_) => {
                // Timeout - send a ping to check if connection is alive
                if received_eose && last_event_time.elapsed() > tokio::time::Duration::from_secs(300) {
                    // If we haven't received events for 5 minutes after EOSE, reconnect
                    println!("⏰ No events from {} for 5 minutes, reconnecting...", relay_url);
                    break;
                }
                // Send ping to keep connection alive
                if write.send(Message::Ping(vec![])).await.is_err() {
                    println!("❌ Failed to ping {}, connection lost", relay_url);
                    break;
                }
            }
        }
    }
    
    Ok(latest_timestamp)
}

// Helper function to rotate and compile a new cassette
async fn rotate_cassette(
    recording_state: &Arc<RwLock<RecordingState>>,
    active_cassettes: &Arc<RwLock<Vec<(PathBuf, Module, Engine)>>>,
    output_dir: &PathBuf,
    base_name: &str,
    _nip_11: bool,
    nip_45: bool,
    nip_50: bool,
    nip11_args: &Nip11Args,
    verbose: bool,
    #[cfg(feature = "deck")] embedded_tools_dir: &Arc<PathBuf>,
) -> Result<()> {
    // Check if already compiling
    {
        let mut state = recording_state.write().await;
        if state.is_compiling {
            return Ok(()); // Skip rotation if already compiling
        }
        state.is_compiling = true;
    }
    
    // Get events but DON'T reset state yet - keep events queryable until cassette is compiled
    let events = {
        let state = recording_state.read().await;
        state.current_events.clone()
    };
    
    if events.is_empty() {
        let mut state = recording_state.write().await;
        state.is_compiling = false;
        return Ok(());
    }
    
    // Generate cassette name with timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let cassette_name = format!("{}-{}", base_name, timestamp);
    
    println!("📼 Rotating cassette: {} ({} events)", cassette_name, events.len());
    
    // Spawn background compilation
    let output_dir = output_dir.clone();
    let active_cassettes = active_cassettes.clone();
    let nip11_args = nip11_args.clone();
    let recording_state_clone = recording_state.clone();
    let event_count = events.len();
    
    #[cfg(feature = "deck")]
    let embedded_tools_dir_clone = embedded_tools_dir.clone();
    
    let handle = tokio::task::spawn_blocking(move || -> Result<()> {
        // Create temporary directory for compilation
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path().to_path_buf();
        
        // Build features list
        // Since the Cargo.toml template has default = ["nip11"], we always need nip11
        let mut features = vec!["default", "nip11"];
        if nip_45 { features.push("nip45"); }
        if nip_50 { features.push("nip50"); }
        
        // Create generator
        let mut generator = generator::CassetteGenerator::new(
            output_dir.clone(),
            &cassette_name,
            &project_dir,
        );
        
        let events_json = serde_json::to_string(&events)?;
        println!("🔍 Debug: Serializing {} events for cassette", events.len());
        println!("🔍 Debug: First event sample: {}", 
            events.first()
                .map(|e| serde_json::to_string(e).unwrap_or_default())
                .unwrap_or_default());
        
        generator.set_var("events_json", &events_json);
        generator.set_var("features_array", &serde_json::to_string(&features)?);
        generator.set_var("version", env!("CARGO_PKG_VERSION"));
        
        if let Some(relay_name) = &nip11_args.relay_name {
            generator.set_var("relay_name", relay_name);
        }
        if let Some(desc) = &nip11_args.relay_description {
            generator.set_var("relay_description", desc);
        }
        if let Some(contact) = &nip11_args.relay_contact {
            generator.set_var("relay_contact", contact);
        }
        if let Some(pubkey) = &nip11_args.relay_pubkey {
            generator.set_var("relay_pubkey", pubkey);
        }
        
        generator.set_verbose(verbose);
        
        // Generate cassette using embedded tools
        #[cfg(feature = "deck")]
        let cassette_path = generator.generate_with_tools_dir(Some(&embedded_tools_dir_clone))?;
        
        #[cfg(not(feature = "deck"))]
        let cassette_path = generator.generate()?;
        
        // Hot-load the new cassette
        let engine = Engine::default();
        let module = Module::from_file(&engine, &cassette_path)?;
        
        // Add to active cassettes and clear the buffer
        tokio::runtime::Handle::current().block_on(async {
            let mut cassettes = active_cassettes.write().await;
            cassettes.push((cassette_path.clone(), module, engine));
            println!("✅ Hot-loaded cassette: {}", cassette_path.display());
            
            // NOW we can clear the buffer since the cassette is ready
            let mut state = recording_state_clone.write().await;
            // Remove the events we just compiled (with bounds check)
            let drain_count = event_count.min(state.current_events.len());
            if drain_count > 0 {
                state.current_events.drain(0..drain_count);
            }
            state.event_count = state.current_events.len();
            state.current_size = state.current_events.iter()
                .map(|e| serde_json::to_string(e).unwrap_or_default().len())
                .sum();
            state.start_time = SystemTime::now();
            state.is_compiling = false;
        });
        
        Ok::<(), anyhow::Error>(())
    });
    
    // Wait for the compilation to complete and log any errors
    let recording_state_for_error = recording_state.clone();
    tokio::spawn(async move {
        match handle.await {
            Ok(Ok(())) => {
                // Success - already logged
            }
            Ok(Err(e)) => {
                eprintln!("❌ Cassette compilation failed: {}", e);
                // Reset state on failure to prevent infinite rotation loop
                let mut state = recording_state_for_error.write().await;
                state.current_events.clear();
                state.event_count = 0;
                state.current_size = 0;
                state.start_time = SystemTime::now();
                state.is_compiling = false;
                eprintln!("⚠️  Cleared {} events due to compilation failure", event_count);
            }
            Err(e) => {
                eprintln!("❌ Task join error: {}", e);
                // Reset state on failure to prevent infinite rotation loop
                let mut state = recording_state_for_error.write().await;
                state.current_events.clear();
                state.event_count = 0;
                state.current_size = 0;
                state.start_time = SystemTime::now();
                state.is_compiling = false;
                eprintln!("⚠️  Cleared {} events due to task failure", event_count);
            }
        }
    });
    
    Ok(())
}

// Helper function to handle deck connections
async fn handle_deck_connection(
    stream: TcpStream,
    active_cassettes: Arc<RwLock<Vec<(PathBuf, Module, Engine)>>>,
) -> Result<()> {
    // Check if this is an HTTP request for NIP-11
    let mut buf = [0u8; 1024];
    let n = stream.peek(&mut buf).await?;
    let peek_data = &buf[..n];
    
    if peek_data.starts_with(b"GET ") {
        // Handle HTTP request for NIP-11
        let cassettes = active_cassettes.read().await;
        if let Some((_, module, engine)) = cassettes.first() {
            let mut store = Store::new(engine, ());
            let instance = Instance::new(&mut store, module, &[])?;
            
            if let Ok(info_func) = instance.get_typed_func::<(), i32>(&mut store, "info") {
                let info_ptr = info_func.call(&mut store, ())?;
                if info_ptr != 0 {
                    let memory = instance.get_memory(&mut store, "memory").unwrap();
                    let info_str = read_string_from_memory(&mut store, &instance, &memory, info_ptr)?;
                    
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/nostr+json\r\nContent-Length: {}\r\n\r\n{}",
                        info_str.len(),
                        info_str
                    );
                    
                    stream.try_write(response.as_bytes())?;
                }
            }
        }
        return Ok(());
    }
    
    // Handle WebSocket connection
    let ws_stream = accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();
    
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Process message against all active cassettes
                let cassettes = active_cassettes.read().await;
                let mut all_responses = Vec::new();
                
                for (_path, module, engine) in cassettes.iter() {
                    let mut store = Store::new(engine, ());
                    let instance = Instance::new(&mut store, module, &[])?;
                    
                    // Process the message
                    if let Ok(send_func) = instance.get_typed_func::<(i32, i32), i32>(&mut store, "send")
                .or_else(|_| instance.get_typed_func::<(i32, i32), i32>(&mut store, "req")) {
                        if let Ok(alloc_func) = instance.get_typed_func::<i32, i32>(&mut store, "alloc_buffer") {
                            let memory = instance.get_memory(&mut store, "memory").unwrap();
                            
                            let msg_bytes = text.as_bytes();
                            let msg_ptr = alloc_func.call(&mut store, msg_bytes.len() as i32)?;
                            
                            if msg_ptr != 0 {
                                memory.write(&mut store, msg_ptr as usize, msg_bytes)?;
                                let result_ptr = send_func.call(&mut store, (msg_ptr, msg_bytes.len() as i32))?;
                                
                                if result_ptr != 0 {
                                    let result = read_string_from_memory(&mut store, &instance, &memory, result_ptr)?;
                                    all_responses.push(result);
                                }
                            }
                        }
                    }
                }
                
                // Aggregate and send responses
                for response in all_responses {
                    write.send(Message::Text(response)).await?;
                }
            }
            Ok(Message::Close(_)) => break,
            _ => {}
        }
    }
    
    Ok(())
}

/// Process the listen command - start a WebSocket server for cassettes
async fn process_listen_command(
    cassette_patterns: &[String],
    port: Option<u16>,
    bind_address: &str,
    tls: bool,
    _tls_cert: Option<&std::path::Path>,
    _tls_key: Option<&std::path::Path>,
    verbose: bool,
) -> Result<()> {
    // Expand glob patterns and collect cassette files
    let mut cassette_files = Vec::new();
    for pattern in cassette_patterns {
        for entry in glob(pattern)? {
            match entry {
                Ok(path) => {
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "wasm") {
                        cassette_files.push(path);
                    }
                }
                Err(e) => eprintln!("Warning: Error reading glob pattern: {}", e),
            }
        }
    }
    
    if cassette_files.is_empty() {
        return Err(anyhow!("No cassette files found matching the provided patterns"));
    }
    
    if verbose {
        println!("🎵 Loading {} cassette(s):", cassette_files.len());
        for file in &cassette_files {
            println!("  - {}", file.display());
        }
    }
    
    // Load cassettes into memory
    let mut loaded_cassettes = Vec::new();
    for path in cassette_files {
        let wasm_bytes = fs::read(&path)?;
        let engine = wasmtime::Engine::default();
        let module = Module::new(&engine, &wasm_bytes)?;
        loaded_cassettes.push((path.clone(), Arc::new(module), Arc::new(engine)));
    }
    
    // Find available port if not specified
    let port = if let Some(p) = port {
        p
    } else {
        find_available_port(bind_address).await?
    };
    
    let addr = format!("{}:{}", bind_address, port);
    let listener = TcpListener::bind(&addr).await?;
    
    let protocol = if tls { "wss" } else { "ws" };
    let http_protocol = if tls { "https" } else { "http" };
    
    println!("🚀 Cassette relay server started");
    println!("   WebSocket: {}://{}:{}", protocol, bind_address, port);
    println!("   HTTP (NIP-11): {}://{}:{}", http_protocol, bind_address, port);
    println!("   Press Ctrl+C to stop");
    
    // Create shared state for cassettes
    let cassettes = Arc::new(loaded_cassettes);
    
    // Accept connections
    while let Ok((stream, addr)) = listener.accept().await {
        if verbose {
            println!("New connection from: {}", addr);
        }
        
        let cassettes_clone = cassettes.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, cassettes_clone, verbose).await {
                if verbose {
                    eprintln!("Error handling connection from {}: {}", addr, e);
                }
            }
        });
    }
    
    Ok(())
}

/// Find an available port
async fn find_available_port(bind_address: &str) -> Result<u16> {
    // Try common ports first
    let common_ports = vec![7777, 8080, 8888, 9999, 3333, 4444, 5555];
    
    for port in common_ports {
        let addr = format!("{}:{}", bind_address, port);
        if TcpListener::bind(&addr).await.is_ok() {
            return Ok(port);
        }
    }
    
    // If no common ports available, let OS assign one
    let addr = format!("{}:0", bind_address);
    let listener = TcpListener::bind(&addr).await?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

/// Handle individual connection
async fn handle_connection(
    stream: TcpStream,
    cassettes: Arc<Vec<(PathBuf, Arc<Module>, Arc<wasmtime::Engine>)>>,
    verbose: bool,
) -> Result<()> {
    
    // Peek at the request to determine type
    let mut buffer = vec![0; 1024];
    let stream = stream;
    let n = stream.peek(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);
    
    // Check if it's a NIP-11 request (has application/nostr+json accept header)
    let is_nip11_request = request.lines().any(|line| {
        line.to_lowercase().contains("accept") && 
        line.contains("application/nostr+json")
    });
    
    if is_nip11_request {
        // Serve NIP-11 JSON
        handle_http_request(stream, cassettes, verbose).await
    } else {
        // Everything else is WebSocket upgrade
        handle_websocket_connection(stream, cassettes, verbose).await
    }
}

/// Handle HTTP requests (for NIP-11)
async fn handle_http_request(
    stream: TcpStream,
    cassettes: Arc<Vec<(PathBuf, Arc<Module>, Arc<wasmtime::Engine>)>>,
    verbose: bool,
) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    let mut stream = stream;
    let mut buffer = vec![0; 1024];
    let n = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);
    
    // Parse the request line
    let lines: Vec<&str> = request.lines().collect();
    if lines.is_empty() {
        return Ok(());
    }
    
    // Check if Accept header indicates NIP-11 request
    let has_nip11_header = lines.iter().any(|line| {
        line.to_lowercase().contains("accept") && 
        line.contains("application/nostr+json")
    });
    
    if has_nip11_header {
        // Get NIP-11 info from the first cassette (or merge from all)
        if let Some((_, module, engine)) = cassettes.first() {
            let mut store = Store::new(engine, ());
            let instance = Instance::new(&mut store, module, &[])?;
            
            // Try to get relay info
            if let Ok(info_func) = instance.get_typed_func::<(), i32>(&mut store, "info") {
                let result_ptr = info_func.call(&mut store, ())?;
                
                if result_ptr != 0 {
                    let memory = instance.get_memory(&mut store, "memory")
                        .ok_or_else(|| anyhow!("Memory export not found"))?;
                    
                    let info = read_string_from_memory(&mut store, &instance, &memory, result_ptr)?;
                    
                    // Send HTTP response with NIP-11 info
                    let response = format!(
                        "HTTP/1.1 200 OK\r\n\
                        Content-Type: application/nostr+json\r\n\
                        Access-Control-Allow-Origin: *\r\n\
                        Access-Control-Allow-Headers: *\r\n\
                        Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
                        Content-Length: {}\r\n\
                        \r\n\
                        {}",
                        info.len(),
                        info
                    );
                    
                    stream.write_all(response.as_bytes()).await?;
                    stream.flush().await?;
                    
                    if verbose {
                        println!("Served NIP-11 info via HTTP");
                    }
                }
            }
        }
    } else {
        // Send 404 for non-NIP-11 requests
        let response = "HTTP/1.1 404 Not Found\r\n\
                       Content-Length: 0\r\n\
                       \r\n";
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;
    }
    
    Ok(())
}

/// Handle WebSocket connections
async fn handle_websocket_connection(
    stream: TcpStream,
    cassettes: Arc<Vec<(PathBuf, Arc<Module>, Arc<wasmtime::Engine>)>>,
    verbose: bool,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();
    
    // Handle incoming messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if verbose {
                    println!("Received: {}", text);
                }
                
                // Parse the message to determine type
                if let Ok(parsed) = serde_json::from_str::<Vec<Value>>(&text) {
                    if let Some(cmd) = parsed.get(0).and_then(|v| v.as_str()) {
                        match cmd {
                            "REQ" | "CLOSE" | "EVENT" | "COUNT" => {
                                // Send to all cassettes and collect responses
                                let mut all_events = Vec::new();
                                let mut sent_eose = false;
                                
                                // Create fresh instances for each message to avoid state issues
                                for (_path, module, engine) in cassettes.iter() {
                                    let mut store = Store::new(engine, ());
                                    let instance = Instance::new(&mut store, module, &[])?;
                                    if let Ok(send_func) = instance.get_typed_func::<(i32, i32), i32>(&mut store, "send")
                .or_else(|_| instance.get_typed_func::<(i32, i32), i32>(&mut store, "req")) {
                                        let memory = instance.get_memory(&mut store, "memory")
                                            .ok_or_else(|| anyhow!("Memory export not found"))?;
                                        let alloc_func = instance.get_typed_func::<i32, i32>(&mut store, "alloc_buffer")?;
                                        let dealloc_func = instance.get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string")?;
                                        
                                        // Keep calling send until we get EOSE or no more events
                                        loop {
                                            // Allocate and write request
                                            let bytes = text.as_bytes();
                                            let req_ptr = alloc_func.call(&mut store, bytes.len() as i32)?;
                                            memory.write(&mut store, req_ptr as usize, bytes)?;
                                            
                                            // Call send function
                                            let result_ptr = send_func.call(&mut store, (req_ptr, bytes.len() as i32))?;
                                            
                                            // Deallocate request memory
                                            dealloc_func.call(&mut store, (req_ptr, bytes.len() as i32))?;
                                            
                                            if result_ptr == 0 {
                                                break; // No more events from this cassette
                                            }
                                            
                                            let response = read_string_from_memory(&mut store, &instance, &memory, result_ptr)?;
                                            
                                            // Try to deallocate the result
                                            if let Ok(get_size_func) = instance.get_typed_func::<i32, i32>(&mut store, "get_allocation_size") {
                                                let size = get_size_func.call(&mut store, result_ptr)?;
                                                if size > 0 {
                                                    let _ = dealloc_func.call(&mut store, (result_ptr, size));
                                                }
                                            }
                                            
                                            // Parse response
                                            if let Ok(parsed_response) = serde_json::from_str::<Value>(&response) {
                                                if let Some(arr) = parsed_response.as_array() {
                                                    if arr.len() >= 1 && arr[0].as_str() == Some("EOSE") {
                                                        if !sent_eose {
                                                            all_events.push(parsed_response);
                                                            sent_eose = true;
                                                        }
                                                        break; // End of events from this cassette
                                                    } else if arr.len() >= 1 && arr[0].as_str() == Some("EVENT") {
                                                        all_events.push(parsed_response);
                                                    } else if arr.len() >= 1 && arr[0].as_str() == Some("NOTICE") {
                                                        all_events.push(parsed_response);
                                                        if response.contains("No more events") {
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                // Send all collected events
                                for event in all_events {
                                    let response = serde_json::to_string(&event)?;
                                    write.send(Message::Text(response)).await?;
                                }
                            }
                            _ => {
                                // Unknown command
                                let notice = json!(["NOTICE", "Unknown command"]);
                                write.send(Message::Text(notice.to_string())).await?;
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                if verbose {
                    println!("Client disconnected");
                }
                break;
            }
            _ => {}
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Record { 
            input_file, 
            name, 
            output,
            generate: _,
            no_bindings,
            interactive,
            verbose,
            _skip_validation,
            skip_unicode_check,
            _nip_11,
            nip_42,
            nip_45,
            nip_50,
            nip11
        } => {
            // Check dependencies before proceeding
            let dep_check = deps::DependencyCheck::new();
            dep_check.check_for_record()?;
            
            // Set default values if not provided
            let name_value = name.clone().unwrap_or_else(|| "cassette".to_string());
            let sanitized_name = sanitize_filename(&name_value);
            let output_value = output.clone().unwrap_or_else(|| PathBuf::from("./cassettes"));
            
            // Either process from file or stdin
            if let Some(path) = input_file {
                if !path.exists() {
                    return Err(anyhow!("Input file doesn't exist: {}", path.display()));
                }
                
                process_events(
                    path.to_str().unwrap(),
                    &sanitized_name,
                    &output_value,
                    *no_bindings,
                    *interactive,
                    *verbose,
                    !*_skip_validation,
                    *skip_unicode_check,
                    *_nip_11,
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
                    &sanitized_name,
                    &output_value,
                    *no_bindings,
                    *interactive,
                    *verbose,
                    !*_skip_validation,
                    *skip_unicode_check,
                    *_nip_11,
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
            // Check dependencies before proceeding
            let dep_check = deps::DependencyCheck::new();
            dep_check.check_for_dub()?;
            
            // Check if required parameters are missing
            if cassettes.is_empty() || output.is_none() {
                if cassettes.is_empty() {
                    eprintln!("Error: Missing required cassette input files\n");
                } else {
                    eprintln!("Error: Missing required output file path\n");
                }
                eprintln!("Usage: cassette dub <CASSETTES...> <OUTPUT> [OPTIONS]\n");
                eprintln!("Combine multiple cassettes into a new cassette (dubbing/mixing)\n");
                eprintln!("Arguments:");
                eprintln!("  <CASSETTES...>  Input cassette files to combine");
                eprintln!("  <OUTPUT>        Output cassette file path\n");
                eprintln!("Options:");
                eprintln!("  -n, --name <NAME>           Name for the generated cassette");
                eprintln!("  -f, --filter <JSON>         Filter JSON (can be specified multiple times)");
                eprintln!("  -k, --kinds <KINDS>         Event kinds to filter");
                eprintln!("      --authors <AUTHORS>     Authors to filter");
                eprintln!("  -l, --limit <LIMIT>         Limit number of events");
                eprintln!("      --since <TIMESTAMP>     Events after timestamp");
                eprintln!("      --until <TIMESTAMP>     Events before timestamp");
                eprintln!("  -i, --interactive           Enable interactive mode");
                eprintln!("  -v, --verbose               Show verbose output");
                eprintln!("  -h, --help                  Print help\n");
                eprintln!("Examples:");
                eprintln!("  # Merge multiple cassettes");
                eprintln!("  cassette dub alice.wasm bob.wasm combined.wasm");
                eprintln!("  ");
                eprintln!("  # Merge with filters");
                eprintln!("  cassette dub *.wasm filtered.wasm --kinds 1 --since 1700000000");
                return Ok(());
            }
            
            process_dub_command(
                cassettes,
                output.as_ref().unwrap(),
                name.as_deref(),
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
        Commands::Scrub {
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
            _skip_validation,
            info,
            count,
            search,
            nip11,
        } => {
            // Check if cassette is provided
            if cassette.is_none() {
                eprintln!("Error: Missing required cassette file\n");
                eprintln!("Usage: cassette scrub <CASSETTE> [OPTIONS]\n");
                eprintln!("Scrub through cassette events (send REQ messages and get events)\n");
                eprintln!("Arguments:");
                eprintln!("  <CASSETTE>  Path to the cassette WASM file\n");
                eprintln!("Options:");
                eprintln!("  -s, --subscription <ID>     Subscription ID (default: sub1)");
                eprintln!("  -f, --filter <JSON>         Filter JSON (can be specified multiple times)");
                eprintln!("  -k, --kinds <KINDS>         Event kinds to filter");
                eprintln!("  -a, --authors <AUTHORS>     Authors to filter");
                eprintln!("  -l, --limit <LIMIT>         Limit number of events");
                eprintln!("      --since <TIMESTAMP>     Events after timestamp");
                eprintln!("      --until <TIMESTAMP>     Events before timestamp");
                eprintln!("  -o, --output <FORMAT>       Output format: nip01, json, or ndjson (default: nip01)");
                eprintln!("      --info                  Show NIP-11 relay information");
                eprintln!("      --count                 Perform COUNT query (NIP-45)");
                eprintln!("      --search <QUERY>        Search query for NIP-50");
                eprintln!("  -i, --interactive           Enable interactive mode");
                eprintln!("  -v, --verbose               Show verbose output");
                eprintln!("  -h, --help                  Print help\n");
                eprintln!("Examples:");
                eprintln!("  # Get all events");
                eprintln!("  cassette scrub my-notes.wasm");
                eprintln!("  ");
                eprintln!("  # Filter by kind");
                eprintln!("  cassette scrub my-notes.wasm --kinds 1");
                eprintln!("  ");
                eprintln!("  # Search events");
                eprintln!("  cassette scrub my-notes.wasm --search \"bitcoin\"");
                return Ok(());
            }
            
            let cassette = cassette.as_ref().unwrap();
            
            if *info {
                // Just show NIP-11 info
                process_info_command(cassette, nip11)
            } else if *count {
                // Generate random subscription ID if using default
                let sub_id = if subscription == "sub1" {
                    format!("sub-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("random"))
                } else {
                    subscription.clone()
                };
                
                // Perform COUNT query
                process_count_command(
                    cassette,
                    &sub_id,
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
                // Generate random subscription ID if using default
                let sub_id = if subscription == "sub1" {
                    format!("sub-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("random"))
                } else {
                    subscription.clone()
                };
                
                process_req_command(
                    cassette,
                    &sub_id,
                    filter,
                    kinds,
                    authors,
                    *limit,
                    *since,
                    *until,
                    output,
                    *interactive,
                    *verbose,
                    *_skip_validation,
                    nip11,
                    search.as_deref(),
                )
            }
        }
        Commands::Play {
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
            // Check if required parameters are missing
            if cassettes.is_empty() || relays.is_empty() {
                if cassettes.is_empty() {
                    eprintln!("Error: Missing required cassette files\n");
                } else {
                    eprintln!("Error: Missing required --relays option\n");
                }
                eprintln!("Usage: cassette play <CASSETTES...> --relays <RELAYS...> [OPTIONS]\n");
                eprintln!("Play events from cassettes to Nostr relays\n");
                eprintln!("Arguments:");
                eprintln!("  <CASSETTES...>  Input cassette files to broadcast\n");
                eprintln!("Options:");
                eprintln!("  -r, --relays <RELAYS>       Target relay URLs (required)");
                eprintln!("  -c, --concurrency <N>       Max concurrent connections (default: 5)");
                eprintln!("  -t, --throttle <MS>         Delay between events in ms (default: 100)");
                eprintln!("      --timeout <SECS>        Connection timeout in seconds (default: 30)");
                eprintln!("      --dry-run               Preview without sending");
                eprintln!("  -i, --interactive           Enable interactive mode");
                eprintln!("  -v, --verbose               Show verbose output");
                eprintln!("  -h, --help                  Print help\n");
                eprintln!("Examples:");
                eprintln!("  # Play to single relay");
                eprintln!("  cassette play events.wasm --relays wss://relay.damus.io");
                eprintln!("  ");
                eprintln!("  # Play to multiple relays");
                eprintln!("  cassette play *.wasm --relays wss://nos.lol wss://relay.nostr.band");
                eprintln!("  ");
                eprintln!("  # Test with dry-run");
                eprintln!("  cassette play archive.wasm --relays ws://localhost:7000 --dry-run");
                return Ok(());
            }
            
            process_play_command(
                cassettes,
                relays,
                *concurrency,
                *throttle,
                *timeout,
                *dry_run,
                nip11,
            ).await
        }
        Commands::DeprecatedPlay {
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
            _skip_validation,
            info,
            count,
            search,
            nip11,
        } => {
            // Print deprecation warning
            eprintln!("⚠️  WARNING: The 'play' command is deprecated and will be removed in a future version.");
            eprintln!("⚠️  Please use 'scrub' instead, which better reflects the analog tape metaphor.");
            eprintln!("");
            
            // Check if cassette is provided
            if cassette.is_none() {
                eprintln!("Error: Missing required cassette file\n");
                eprintln!("Usage: cassette scrub <CASSETTE> [OPTIONS]  (use 'scrub' instead of 'play')\n");
                eprintln!("Please use the 'scrub' command instead of the deprecated 'play' command.");
                return Ok(());
            }
            
            let cassette = cassette.as_ref().unwrap();
            
            // Pass through to the same logic as scrub
            if *info {
                // Just show NIP-11 info
                process_info_command(cassette, nip11)
            } else if *count {
                // Generate random subscription ID if using default
                let sub_id = if subscription == "sub1" {
                    format!("sub-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("random"))
                } else {
                    subscription.clone()
                };
                
                // Perform COUNT query
                process_count_command(
                    cassette,
                    &sub_id,
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
                // Generate random subscription ID if using default
                let sub_id = if subscription == "sub1" {
                    format!("sub-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("random"))
                } else {
                    subscription.clone()
                };
                
                process_req_command(
                    cassette,
                    &sub_id,
                    filter,
                    kinds,
                    authors,
                    *limit,
                    *since,
                    *until,
                    output,
                    *interactive,
                    *verbose,
                    *_skip_validation,
                    nip11,
                    search.as_deref(),
                )
            }
        }
        Commands::Listen {
            cassettes,
            port,
            bind,
            tls,
            tls_cert,
            tls_key,
            verbose,
        } => {
            // Check if required parameters are missing
            if cassettes.is_empty() {
                eprintln!("Error: Missing required cassette files\n");
                eprintln!("Usage: cassette listen <CASSETTES...> [OPTIONS]\n");
                eprintln!("Start a WebSocket server to serve cassettes as a Nostr relay\n");
                eprintln!("Arguments:");
                eprintln!("  <CASSETTES...>  Cassette files to serve (supports globs like \"*.wasm\")\n");
                eprintln!("Options:");
                eprintln!("  -p, --port <PORT>           Port to listen on (auto-selects if not specified)");
                eprintln!("      --bind <ADDRESS>        Bind address (default: 127.0.0.1)");
                eprintln!("      --tls                   Enable HTTPS/WSS");
                eprintln!("      --tls-cert <PATH>       Path to TLS certificate");
                eprintln!("      --tls-key <PATH>        Path to TLS key");
                eprintln!("  -v, --verbose               Show verbose output");
                eprintln!("  -h, --help                  Print help\n");
                eprintln!("Examples:");
                eprintln!("  # Serve single cassette");
                eprintln!("  cassette listen my-notes.wasm");
                eprintln!("  ");
                eprintln!("  # Serve on specific port");
                eprintln!("  cassette listen *.wasm --port 8080");
                eprintln!("  ");
                eprintln!("  # Listen on all interfaces");
                eprintln!("  cassette listen cassettes/*.wasm --bind 0.0.0.0 --port 7777");
                return Ok(());
            }
            
            process_listen_command(
                cassettes,
                *port,
                bind,
                *tls,
                tls_cert.as_deref(),
                tls_key.as_deref(),
                *verbose,
            ).await
        }
        Commands::Deck {
            mode,
            relays,
            name,
            output,
            port,
            bind,
            event_limit,
            size_limit,
            duration,
            filter,
            kinds,
            authors,
            _nip_11,
            nip_45,
            nip_50,
            verbose,
            _skip_validation,
            nip11,
        } => {
            match mode.as_str() {
                "relay" => {
                    process_deck_relay_mode(
                        name,
                        output,
                        *port,
                        bind,
                        *event_limit,
                        *size_limit,
                        *duration,
                        *_nip_11,
                        *nip_45,
                        *nip_50,
                        *verbose,
                        *_skip_validation,
                        nip11,
                    ).await
                }
                "record" => {
                    if relays.is_empty() {
                        eprintln!("Error: --relays is required for record mode\n");
                        eprintln!("Usage: cassette deck --mode record --relays <RELAYS...> [OPTIONS]\n");
                        eprintln!("Continuously record events from other relays and serve cassettes\n");
                        eprintln!("Required Options:");
                        eprintln!("  -r, --relays <RELAYS>       Relay URLs to record from\n");
                        eprintln!("Options:");
                        eprintln!("  -n, --name <NAME>           Base name for cassettes (default: deck)");
                        eprintln!("  -o, --output <DIR>          Output directory (default: ./deck)");
                        eprintln!("  -p, --port <PORT>           Port to serve on (default: 7777)");
                        eprintln!("      --bind <ADDRESS>        Bind address (default: 127.0.0.1)");
                        eprintln!("  -e, --event-limit <N>       Max events per cassette (default: 10000)");
                        eprintln!("  -s, --size-limit <MB>       Max cassette size in MB (default: 100)");
                        eprintln!("  -d, --duration <SECS>       Recording duration per cassette (default: 3600)");
                        eprintln!("  -f, --filter <JSON>         Filter JSON for recording");
                        eprintln!("  -k, --kinds <KINDS>         Event kinds to record");
                        eprintln!("      --authors <AUTHORS>     Authors to filter");
                        eprintln!("      --nip-11                Enable NIP-11 support");
                        eprintln!("      --nip-45                Enable NIP-45 (COUNT) support");
                        eprintln!("      --nip-50                Enable NIP-50 (search) support");
                        eprintln!("  -v, --verbose               Show verbose output");
                        eprintln!("  -h, --help                  Print help\n");
                        eprintln!("Examples:");
                        eprintln!("  # Record from single relay");
                        eprintln!("  cassette deck --mode record --relays wss://relay.damus.io");
                        eprintln!("  ");
                        eprintln!("  # Record specific kinds from multiple relays");
                        eprintln!("  cassette deck --mode record --relays wss://nos.lol wss://relay.nostr.band --kinds 1 --kinds 30023");
                        eprintln!("  ");
                        eprintln!("  # Custom rotation settings");
                        eprintln!("  cassette deck --mode record --relays wss://relay.damus.io --event-limit 50000 --duration 7200");
                        return Ok(());
                    }
                    process_deck_record_mode(
                        relays,
                        name,
                        output,
                        *port,
                        bind,
                        *event_limit,
                        *size_limit,
                        *duration,
                        filter.as_deref(),
                        kinds,
                        authors,
                        *_nip_11,
                        *nip_45,
                        *nip_50,
                        *verbose,
                        *_skip_validation,
                        nip11,
                    ).await
                }
                _ => Err(anyhow!("Invalid mode: {}. Use 'relay' or 'record'", mode))
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
            // Print deprecation warning
            eprintln!("⚠️  WARNING: The 'cast' command is deprecated and will be removed in a future version.");
            eprintln!("⚠️  Please use 'play' instead, which better reflects the analog tape metaphor.");
            eprintln!("");
            
            // Pass through to process_cast_command with same parameters
            process_play_command(
                cassettes,
                relays,
                *concurrency,
                *throttle,
                *timeout,
                *dry_run,
                nip11,
            ).await
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

/// Parse events from file, supporting JSON array, NDJSON, and NIP-01 message formats
fn parse_events_from_file(input_file: &str) -> Result<Vec<Value>> {
    let file = File::open(input_file)?;
    let mut reader = BufReader::new(file);
    
    // Read first character to determine format
    let mut first_char = [0u8; 1];
    reader.read_exact(&mut first_char)?;
    
    // Reset reader to beginning
    reader.seek(std::io::SeekFrom::Start(0))?;
    
    if first_char[0] == b'[' {
        // Could be JSON array or NIP-01 message
        let content = std::fs::read_to_string(input_file)?;
        let parsed: Value = serde_json::from_str(&content)?;
        
        if let Some(arr) = parsed.as_array() {
            // Check if it's a NIP-01 message format ["EVENT", {...}]
            if arr.len() == 2 && arr[0].as_str() == Some("EVENT") {
                // Single NIP-01 message, extract the event
                if let Some(event) = arr.get(1) {
                    Ok(vec![event.clone()])
                } else {
                    Err(anyhow!("Invalid NIP-01 message format"))
                }
            } else if arr.len() >= 1 && arr[0].is_object() {
                // Regular JSON array of events
                Ok(arr.clone())
            } else {
                // Could be an array of NIP-01 messages
                let mut events = Vec::new();
                for item in arr {
                    if let Some(msg_arr) = item.as_array() {
                        if msg_arr.len() == 2 && msg_arr[0].as_str() == Some("EVENT") {
                            if let Some(event) = msg_arr.get(1) {
                                events.push(event.clone());
                            }
                        }
                    } else if item.is_object() {
                        // Regular event object
                        events.push(item.clone());
                    }
                }
                Ok(events)
            }
        } else {
            Err(anyhow!("Expected JSON array"))
        }
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
            
            // Try to parse as NIP-01 message first
            if trimmed.starts_with("[") {
                match serde_json::from_str::<Value>(trimmed) {
                    Ok(Value::Array(arr)) if arr.len() == 2 && arr[0].as_str() == Some("EVENT") => {
                        // NIP-01 message format
                        if let Some(event) = arr.get(1) {
                            events.push(event.clone());
                        }
                    }
                    _ => {
                        // Try as regular event
                        match serde_json::from_str::<Value>(trimmed) {
                            Ok(event) if event.is_object() => events.push(event),
                            _ => eprintln!("Warning: Skipping invalid line {}: {}", line_num + 1, trimmed),
                        }
                    }
                }
            } else {
                // Parse as regular JSON object
                match serde_json::from_str::<Value>(trimmed) {
                    Ok(event) if event.is_object() => events.push(event),
                    Ok(_) => eprintln!("Warning: Skipping non-object on line {}", line_num + 1),
                    Err(e) => {
                        // Skip invalid lines with a warning
                        eprintln!("Warning: Skipping invalid JSON on line {}: {}", line_num + 1, e);
                    }
                }
            }
        }
        
        if events.is_empty() {
            return Err(anyhow!("No valid events found in input file"));
        }
        
        Ok(events)
    }
}

/// Filter out events containing problematic Unicode characters
/// Returns (filtered_events, skipped_events) where skipped_events contains (event_id, unicode_char)
fn filter_problematic_unicode_events(events: Vec<Value>, verbose: bool) -> (Vec<Value>, Vec<(String, u32)>) {
    let problematic_chars = [
        '\u{2028}', // Line Separator
        '\u{2029}', // Paragraph Separator
        '\u{202A}', // Left-to-Right Embedding
        '\u{202B}', // Right-to-Left Embedding
        '\u{202C}', // Pop Directional Formatting
        '\u{202D}', // Left-to-Right Override
        '\u{202E}', // Right-to-Left Override
        '\u{2066}', // Left-to-Right Isolate
        '\u{2067}', // Right-to-Left Isolate
        '\u{2068}', // First Strong Isolate
        '\u{2069}', // Pop Directional Isolate
    ];
    
    let mut filtered_events = Vec::new();
    let mut skipped_events = Vec::new();
    
    for event in events {
        let event_json = serde_json::to_string(&event).unwrap_or_default();
        let mut has_problematic_char = false;
        let mut found_char: Option<char> = None;
        
        for &ch in &problematic_chars {
            if event_json.contains(ch) {
                has_problematic_char = true;
                found_char = Some(ch);
                break;
            }
        }
        
        if has_problematic_char {
            // Extract event ID for reporting
            let event_id = event.get("id")
                .and_then(|id| id.as_str())
                .unwrap_or("unknown")
                .to_string();
            
            if let Some(ch) = found_char {
                skipped_events.push((event_id, ch as u32));
            }
            
            debugln!(verbose, "  Skipping event {} due to Unicode character U+{:04X}", 
                     event.get("id").and_then(|id| id.as_str()).unwrap_or("unknown"),
                     found_char.unwrap_or('\0') as u32);
        } else {
            filtered_events.push(event);
        }
    }
    
    (filtered_events, skipped_events)
}

fn process_events(
    input_file: &str,
    name: &str,
    output_dir: &PathBuf,
    _no_bindings: bool,
    interactive: bool,
    verbose: bool,
    validate: bool,
    skip_unicode_check: bool,
    _nip_11: bool,
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
    
    // Filter out events with problematic Unicode unless skip_unicode_check is set
    let (filtered_events, skipped_events) = if skip_unicode_check {
        // Skip the check - include all events
        (original_events, Vec::new())
    } else {
        filter_problematic_unicode_events(original_events, verbose)
    };
    
    // Report skipped events if any
    if !skipped_events.is_empty() {
        eprintln!("\n⚠️  Skipped {} events due to problematic Unicode characters:", skipped_events.len());
        for (event_id, char_code) in skipped_events.iter() {
            eprintln!("   Event ID: {} - Unicode character U+{:04X}", event_id, char_code);
        }
        eprintln!("   To include these events anyway, use the --skip-unicode-check flag.");
    }
    
    // Preprocess events to handle replaceable and addressable events
    debugln!(verbose, "\n🔍 Preprocessing events according to NIP-01...");
    let mut processed_events = preprocess_events(filtered_events);
    
    // Validate events if validation is enabled
    if validate {
        debugln!(verbose, "\n🔍 Validating Nostr events...");
        let original_count = processed_events.len();
        processed_events = processed_events.into_iter()
            .filter(|event| validate_nostr_event(event, verbose))
            .collect();
        let valid_count = processed_events.len();
        let invalid_count = original_count - valid_count;
        
        if verbose {
            println!("✅ Valid events: {}", valid_count);
            if invalid_count > 0 {
                println!("❌ Invalid events filtered out: {}", invalid_count);
            }
        }
        
        if invalid_count > 0 {
            println!("⚠️  Filtered out {} invalid events", invalid_count);
        }
    }
    
    debugln!(verbose, "\n📊 Final Event Summary:");
    debugln!(verbose, "  Total events after preprocessing{}: {}", 
        if validate { " and validation" } else { "" }, 
        processed_events.len()
    );
    if !skipped_events.is_empty() {
        debugln!(verbose, "  Events skipped due to Unicode issues: {}", skipped_events.len());
    }
    
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
    generator.set_var("event_count", &event_count.to_string());
    
    // Properly escape the JSON for template insertion
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
    
    // Add version from Cargo.toml
    generator.set_var("version", env!("CARGO_PKG_VERSION"));

    // Set verbose mode on generator
    generator.set_verbose(verbose);
    
    // Generate the cassette with compilation progress
    let result = if let Some(ref ui) = record_ui {
        // Interactive mode - show compilation progress
        let total_events = processed_events.len() as u64;
        #[cfg(feature = "deck")]
        {
            generator.generate_with_embedded_tools()
        }
        #[cfg(not(feature = "deck"))]
        {
            generator.generate_with_callback(Some(|| {
                ui.show_compilation(total_events)?;
                Ok(())
            }))
        }
    } else {
        // Non-interactive mode
        #[cfg(feature = "deck")]
        {
            generator.generate_with_embedded_tools()
        }
        #[cfg(not(feature = "deck"))]
        {
            generator.generate()
        }
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
// Play command implementation

#[derive(Clone)]
struct RelayStatus {
    url: String,
    connected: bool,
    total: usize,
    successful: usize,
    failed: usize,
}

async fn process_play_command(
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
    
    println!("🎯 Playing events from {} cassette(s) to {} relay(s)", 
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
    
    println!("\n📊 Total unique events to play: {}", all_events.len());
    
    // Initialize relay status tracking
    let relay_statuses = Arc::new(Mutex::new(
        relay_urls.iter().enumerate().map(|(_idx, url)| RelayStatus {
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
            play_to_relay(idx, relay_url, events, statuses, timeout, throttle).await
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
    let send_func = instance.get_typed_func::<(i32, i32), i32>(&mut store, "send")
        .or_else(|_| instance.get_typed_func::<(i32, i32), i32>(&mut store, "req"))?;
    let dealloc_func = instance.get_typed_func::<(i32, i32), ()>(&mut store, "dealloc_string").ok();
    
    // Request all events
    let req_message = json!(["REQ", "play-extract", {}]);
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

/// Play events to a single relay
async fn play_to_relay(
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
    
    println!("\n🎯 Playing Progress:");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        // Basic cases
        assert_eq!(sanitize_filename("Hello World"), "hello-world");
        assert_eq!(sanitize_filename("My Relay!"), "my-relay");
        assert_eq!(sanitize_filename("Test@#$%Name"), "testname");
        
        // With hyphens
        assert_eq!(sanitize_filename("already-hyphenated"), "already-hyphenated");
        assert_eq!(sanitize_filename("multiple   spaces"), "multiple-spaces");
        assert_eq!(sanitize_filename("trailing-spaces  "), "trailing-spaces");
        
        // Special characters
        assert_eq!(sanitize_filename("Special!@#$%^&*()Characters"), "specialcharacters");
        assert_eq!(sanitize_filename("dots.and.periods"), "dotsandperiods");
        assert_eq!(sanitize_filename("under_scores"), "underscores");
        
        // Edge cases
        assert_eq!(sanitize_filename(""), "bruh");
        assert_eq!(sanitize_filename("   "), "bruh");
        assert_eq!(sanitize_filename("---"), "bruh");
        assert_eq!(sanitize_filename("!!!"), "bruh");
        assert_eq!(sanitize_filename("@#$%^&*()"), "bruh");
        assert_eq!(sanitize_filename("a-b-c"), "a-b-c");
        assert_eq!(sanitize_filename("UPPERCASE"), "uppercase");
        
        // Real examples
        assert_eq!(sanitize_filename("Bitcoin & Lightning"), "bitcoin-lightning");
        assert_eq!(sanitize_filename("Nostr (Notes & Other Stuff)"), "nostr-notes-other-stuff");
        assert_eq!(sanitize_filename("My Archive 2024"), "my-archive-2024");
    }
}