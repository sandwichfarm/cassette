use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::fs;
use std::io::{self, Read, Write, BufReader};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use chrono::{Local, Utc};
use std::process::Command;
use std::fs::File;
use tempfile::tempdir;
use std::collections::HashMap;

// Module for cassette generation
mod generator {
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::collections::HashMap;
    use anyhow::{Context, Result, anyhow};
    use handlebars::Handlebars;
    use serde_json::{json, Value};
    use std::process::Command;
    use uuid::Uuid;
    use chrono::Local;

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

#[derive(Parser)]
#[command(author, version, about = "CLI tool for Cassette platform")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a Nostr events file or piped input to create a cassette
    Dub {
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Dub { 
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

pub fn process_events(
    input_file: &str,
    name: &str,
    description: &str,
    author: &str,
    output_dir: &PathBuf,
    no_bindings: bool
) -> Result<()> {
    // Parse input file
    let file = File::open(input_file)?;
    let reader = BufReader::new(file);
    let original_events: Vec<Value> = serde_json::from_reader(reader)?;
    
    // Display statistics
    println!("=== Cassette CLI - Dub Command ===");
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
