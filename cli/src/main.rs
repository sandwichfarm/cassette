use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use uuid::Uuid;
use chrono::Local;

// Module for cassette generation
mod generator {
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use anyhow::{Context, Result, anyhow};
    use handlebars::Handlebars;
    use serde_json::{json, Value};
    use std::process::Command;
    use tempfile::tempdir;
    use uuid::Uuid;
    use chrono::Local;

    // Load template files
    const TEMPLATE_RS: &str = include_str!("templates/cassette_template.rs");
    const TEMPLATE_CARGO: &str = include_str!("templates/Cargo.toml");

    pub struct CassetteGenerator {
        events: Value,
        name: String,
        description: String,
        author: String,
        output_dir: PathBuf,
    }

    impl CassetteGenerator {
        pub fn new(
            events: Value,
            name: Option<String>,
            description: Option<String>,
            author: Option<String>,
            output_dir: Option<PathBuf>,
        ) -> Self {
            // Generate default values if not provided
            let name = name.unwrap_or_else(|| format!("cassette-{}", Uuid::new_v4().to_string().split('-').next().unwrap()));
            let description = description.unwrap_or_else(|| "Generated Nostr events cassette".to_string());
            let author = author.unwrap_or_else(|| "Cassette CLI".to_string());
            let output_dir = output_dir.unwrap_or_else(|| PathBuf::from("."));

            Self {
                events,
                name,
                description,
                author,
                output_dir,
            }
        }

        pub fn generate(&self) -> Result<PathBuf> {
            // Create a temporary directory for building
            let temp_dir = tempdir().context("Failed to create temporary directory")?;
            let temp_path = temp_dir.path();

            // Create a project structure
            self.create_project_structure(temp_path)?;

            // Build the WASM module
            let output_path = self.build_wasm(temp_path)?;

            // Copy the output to the destination
            self.copy_output(output_path)?;

            // Return the path to the generated WASM file
            let wasm_path = self.output_dir.join(format!("{}.wasm", self.name));
            
            Ok(wasm_path)
        }

        fn create_project_structure(&self, base_dir: &Path) -> Result<()> {
            // Create src directory
            let src_dir = base_dir.join("src");
            fs::create_dir_all(&src_dir).context("Failed to create src directory")?;

            // Create the lib.rs file from template
            let struct_name = self.name.replace("-", "_").replace(" ", "_");
            let struct_name = struct_name.split('_')
                .map(|s| s.chars().next().unwrap().to_uppercase().to_string() + &s[1..])
                .collect::<String>();

            // Create Handlebars instance for template rendering
            let mut handlebars = Handlebars::new();
            handlebars.set_strict_mode(true);

            // Get the JSON string and event count
            let events_json = serde_json::to_string(&self.events).unwrap_or_else(|_| "[]".to_string());
            let event_count = match &self.events {
                Value::Array(events) => events.len(),
                _ => 0
            };

            // Prepare the template data
            let template_data = json!({
                "events_json": events_json,
                "cassette_name": self.name,
                "cassette_description": self.description,
                "cassette_version": "0.1.0",
                "cassette_author": self.author,
                "cassette_created": Local::now().to_rfc3339(),
                "cassette_struct": struct_name,
                "event_count": event_count
            });

            // Render the lib.rs template
            let lib_rs_content = handlebars.render_template(TEMPLATE_RS, &template_data)
                .context("Failed to render lib.rs template")?;

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
                "description": self.description,
                "cassette_tools_path": cassette_tools_path
            });

            // Render the Cargo.toml template
            let cargo_content = handlebars.render_template(TEMPLATE_CARGO, &cargo_data)
                .context("Failed to render Cargo.toml template")?;

            // Write the Cargo.toml file
            let cargo_path = base_dir.join("Cargo.toml");
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

        fn copy_output(&self, wasm_path: PathBuf) -> Result<()> {
            // Create the output directory if it doesn't exist
            fs::create_dir_all(&self.output_dir)
                .context("Failed to create output directory")?;

            // Copy the WASM file to the output directory
            let dest_path = self.output_dir.join(format!("{}.wasm", self.name));
            fs::copy(&wasm_path, &dest_path)
                .context("Failed to copy WASM file to output directory")?;

            // Generate JavaScript bindings using wasm-bindgen
            self.generate_js_bindings(&dest_path)?;

            Ok(())
        }

        fn generate_js_bindings(&self, wasm_path: &Path) -> Result<()> {
            // Check if wasm-bindgen-cli is installed
            let wasm_bindgen_version = Command::new("wasm-bindgen")
                .arg("--version")
                .output();

            if wasm_bindgen_version.is_err() {
                println!("Note: wasm-bindgen-cli is not installed. JavaScript bindings will not be generated.");
                println!("To install wasm-bindgen-cli, run: cargo install wasm-bindgen-cli");
                return Ok(());
            }

            // Run wasm-bindgen to generate JavaScript bindings
            let status = Command::new("wasm-bindgen")
                .args(&[
                    wasm_path.to_str().unwrap(),
                    "--out-dir", self.output_dir.to_str().unwrap(),
                    "--target", "web"
                ])
                .status();

            match status {
                Ok(status) if status.success() => {
                    println!("JavaScript bindings generated successfully");
                    Ok(())
                },
                Ok(_) => {
                    println!("Warning: wasm-bindgen returned an error. JavaScript bindings may not have been generated correctly.");
                    Ok(())
                },
                Err(e) => {
                    println!("Warning: Failed to run wasm-bindgen: {}", e);
                    Ok(())
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
            generate
        } => {
            // Either read the file or stdin
            let input = match input_file {
                Some(path) => {
                    fs::read_to_string(path)
                        .with_context(|| format!("Failed to read input file: {}", path.display()))?
                }
                None => {
                    // Read from stdin
                    let mut buffer = String::new();
                    io::stdin()
                        .read_to_string(&mut buffer)
                        .context("Failed to read from stdin")?;
                    buffer
                }
            };

            // Process the input events
            process_events(&input, name.clone(), description.clone(), author.clone(), output.clone(), *generate)?;
            
            Ok(())
        }
    }
}

fn process_events(
    input: &str, 
    name: Option<String>, 
    description: Option<String>, 
    author: Option<String>,
    output_dir: Option<PathBuf>,
    should_generate: bool
) -> Result<()> {
    // Parse the input as JSON
    let events: Value = serde_json::from_str(input)
        .context("Failed to parse input as JSON")?;
    
    // For now, just display information about what we would process
    println!("=== Cassette CLI - Dub Command ===");
    println!("Processing events for cassette creation...");
    
    if let Value::Array(events_array) = &events {
        println!("\n📊 Event Summary:");
        println!("  Total events: {}", events_array.len());
        
        // Count the number of events by kind
        let mut kind_counts = std::collections::HashMap::new();
        for event in events_array {
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
        
        // Sample of events
        println!("\n📝 Sample Events:");
        for (i, event) in events_array.iter().take(2).enumerate() {
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
        
        if events_array.len() > 2 {
            println!("  ... and {} more events", events_array.len() - 2);
        }
    } else {
        println!("⚠️ Warning: Input is not an array of events");
        println!("  Please provide a valid array of Nostr events");
        return Err(anyhow!("Input is not an array of events"));
    }
    
    // Show cassette metadata
    let cassette_name = name.clone().unwrap_or_else(
        || format!("cassette-{}", Uuid::new_v4().to_string().split('-').next().unwrap())
    );
    let cassette_description = description.clone().unwrap_or_else(
        || "Generated Nostr events cassette".to_string()
    );
    let cassette_author = author.clone().unwrap_or_else(
        || "Cassette CLI".to_string()
    );
    
    println!("\n📦 Cassette Information:");
    println!("  Name: {}", cassette_name);
    println!("  Description: {}", cassette_description);
    println!("  Author: {}", cassette_author);
    println!("  Created: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    
    // Generate the WASM module if requested
    if should_generate {
        println!("\n🔨 Generating WASM Module:");
        println!("  Creating Rust project from template...");
        
        // Create a new cassette generator
        let generator = generator::CassetteGenerator::new(
            events,
            name,
            description,
            author,
            output_dir,
        );
        
        // Generate the WASM module
        match generator.generate() {
            Ok(wasm_path) => {
                println!("  WASM module generated successfully!");
                println!("  Output: {}", wasm_path.display());
                println!("\n✅ Cassette creation complete!");
                println!("  You can load this WebAssembly module into the Boombox server.");
            },
            Err(e) => {
                println!("  ❌ Failed to generate WASM module: {}", e);
                return Err(anyhow!("Failed to generate WASM module: {}", e));
            }
        }
    } else {
        println!("\n📝 Generation Skipped:");
        println!("  WASM module generation was disabled.");
        println!("  Run with --generate=true to create the actual WebAssembly module.");
    }
    
    Ok(())
}
