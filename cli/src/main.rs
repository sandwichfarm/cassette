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
            fs::create_dir_all(&self.output_dir)
                .context("Failed to create output directory")?;

            // Copy the WASM file to the output directory
            let dest_path = self.output_dir.join(format!("{}@{}.wasm", self.name, "notes.json"));
            fs::copy(&wasm_path, &dest_path)
                .context("Failed to copy WASM file to output directory")?;

            Ok(dest_path)
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
            
            // Check if input file exists
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
                return Err(anyhow!("No input file provided. Please specify a file."));
            }
            
            Ok(())
        }
    }
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
    let events: Vec<Value> = serde_json::from_reader(reader)?;

    // Count the number of events by kind
    let mut kind_counts = std::collections::HashMap::new();
    for event in &events {
        if let Some(kind) = event.get("kind").and_then(|k| k.as_i64()) {
            *kind_counts.entry(kind).or_insert(0) += 1;
        }
    }
    
    // Display statistics
    println!("=== Cassette CLI - Dub Command ===");
    println!("Processing events for cassette creation...");
    
    println!("\n📊 Event Summary:");
    println!("  Total events: {}", events.len());
    
    // Display kind statistics
    if !kind_counts.is_empty() {
        println!("\n📋 Event Kinds:");
        for (kind, count) in kind_counts.iter() {
            println!("  Kind {}: {} events", kind, count);
        }
    }
    
    // Sample of events
    println!("\n📝 Sample Events:");
    for (i, event) in events.iter().take(2).enumerate() {
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
    
    if events.len() > 2 {
        println!("  ... and {} more events", events.len() - 2);
    }

    // Generate metadata
    let cassette_created = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let event_count = events.len();
    
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
    let events_json_string = serde_json::to_string(&events).unwrap();
    events_file.write_all(events_json_string.as_bytes())?;

    // Initialize generator with output path and name
    let mut generator = generator::CassetteGenerator::new(
        output_dir.clone(),
        name,
        &project_dir
    );
    
    // Set template variables
    generator.set_var("events_json", &events_json_string);
    generator.set_var("cassette_name", name);
    generator.set_var("cassette_description", description);
    generator.set_var("cassette_author", author);
    generator.set_var("cassette_created", &cassette_created);
    generator.set_var("event_count", &event_count.to_string());
    generator.set_var("cassette_version", "0.1.0");

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
