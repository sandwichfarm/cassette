use std::process::Command;
use anyhow::{Result, anyhow};
use std::env;

#[derive(Debug, Clone, Copy)]
pub enum Platform {
    MacOS,
    Linux,
    Windows,
    Other,
}

impl Platform {
    fn detect() -> Self {
        match env::consts::OS {
            "macos" => Platform::MacOS,
            "linux" => Platform::Linux,
            "windows" => Platform::Windows,
            _ => Platform::Other,
        }
    }
    
    fn name(&self) -> &'static str {
        match self {
            Platform::MacOS => "macOS",
            Platform::Linux => "Linux",
            Platform::Windows => "Windows",
            Platform::Other => "Other",
        }
    }
}

pub struct DependencyCheck {
    pub has_rust: bool,
    pub has_wasm_target: bool,
    pub has_wasm_opt: bool,
    pub platform: Platform,
}

impl DependencyCheck {
    pub fn new() -> Self {
        Self {
            has_rust: check_rust_installed(),
            has_wasm_target: check_wasm_target(),
            has_wasm_opt: check_wasm_opt(),
            platform: Platform::detect(),
        }
    }
    
    pub fn check_for_record(&self) -> Result<()> {
        if !self.has_rust {
            let install_cmd = match self.platform {
                Platform::Windows => {
                    "To install Rust on Windows:\n\
                      Download and run: https://win.rustup.rs\n\n\
                    Or use PowerShell:\n\
                      winget install Rustlang.Rustup\n\n\
                    After installation, restart your terminal."
                }
                _ => {
                    "To install Rust:\n\
                      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\n\n\
                    After installation, restart your terminal or run:\n\
                      source $HOME/.cargo/env"
                }
            };
            
            return Err(anyhow!(
                "Rust toolchain not found. The 'record' command requires Rust to compile WASM.\n\n{}",
                install_cmd
            ));
        }
        
        if !self.has_wasm_target {
            return Err(anyhow!(
                "WASM target not found. The 'record' command requires the wasm32-unknown-unknown target.\n\n\
                To install the WASM target:\n\
                  rustup target add wasm32-unknown-unknown"
            ));
        }
        
        if !self.has_wasm_opt {
            let install_cmd = match self.platform {
                Platform::MacOS => "brew install binaryen",
                Platform::Linux => "# Ubuntu/Debian\nsudo apt install binaryen\n\n# Fedora\nsudo dnf install binaryen\n\n# Arch\nsudo pacman -S binaryen",
                Platform::Windows => "# Using Chocolatey\nchoco install binaryen\n\n# Or download from:\nhttps://github.com/WebAssembly/binaryen/releases",
                Platform::Other => "Download from: https://github.com/WebAssembly/binaryen/releases",
            };
            
            eprintln!(
                "⚠️  Warning: wasm-opt not found. Cassettes will not be optimized.\n\
                To install wasm-opt on {}:\n{}\n",
                self.platform.name(),
                install_cmd
            );
        }
        
        Ok(())
    }
    
    pub fn check_for_dub(&self) -> Result<()> {
        // Dub uses the same dependencies as record
        self.check_for_record()
    }
}

fn check_rust_installed() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn check_wasm_target() -> bool {
    Command::new("rustc")
        .args(&["--print", "target-list"])
        .output()
        .map(|output| {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().any(|line| line.trim() == "wasm32-unknown-unknown")
            } else {
                false
            }
        })
        .unwrap_or(false)
}

fn check_wasm_opt() -> bool {
    Command::new("wasm-opt")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Show installation instructions for all missing dependencies
pub fn show_full_installation_guide() {
    let platform = Platform::detect();
    
    println!("\n📚 Cassette Development Dependencies\n");
    println!("The 'record' and 'dub' commands compile events into WASM modules.");
    println!("This requires the Rust toolchain and WASM target.\n");
    println!("Detected platform: {}\n", platform.name());
    
    println!("🔧 Installation Steps:\n");
    
    match platform {
        Platform::Windows => {
            println!("1. Install Rust:");
            println!("   Download and run: https://win.rustup.rs");
            println!("   ");
            println!("   Or using Windows Package Manager:");
            println!("   winget install Rustlang.Rustup\n");
            
            println!("2. Add WASM compilation target (in new terminal):");
            println!("   rustup target add wasm32-unknown-unknown\n");
            
            println!("3. (Optional) Install wasm-opt for smaller cassettes:");
            println!("   # Using Chocolatey");
            println!("   choco install binaryen");
            println!("   ");
            println!("   # Or download from:");
            println!("   https://github.com/WebAssembly/binaryen/releases\n");
        }
        Platform::MacOS => {
            println!("1. Install Rust:");
            println!("   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\n");
            
            println!("2. Configure your shell (if needed):");
            println!("   source $HOME/.cargo/env\n");
            
            println!("3. Add WASM compilation target:");
            println!("   rustup target add wasm32-unknown-unknown\n");
            
            println!("4. (Optional) Install wasm-opt for smaller cassettes:");
            println!("   brew install binaryen\n");
        }
        Platform::Linux => {
            println!("1. Install Rust:");
            println!("   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\n");
            
            println!("2. Configure your shell (if needed):");
            println!("   source $HOME/.cargo/env\n");
            
            println!("3. Add WASM compilation target:");
            println!("   rustup target add wasm32-unknown-unknown\n");
            
            println!("4. (Optional) Install wasm-opt for smaller cassettes:");
            println!("   # Ubuntu/Debian");
            println!("   sudo apt install binaryen");
            println!("   ");
            println!("   # Fedora");
            println!("   sudo dnf install binaryen");
            println!("   ");
            println!("   # Arch Linux");
            println!("   sudo pacman -S binaryen\n");
        }
        Platform::Other => {
            println!("1. Install Rust:");
            println!("   Visit: https://www.rust-lang.org/tools/install\n");
            
            println!("2. Add WASM compilation target:");
            println!("   rustup target add wasm32-unknown-unknown\n");
            
            println!("3. (Optional) Install wasm-opt:");
            println!("   Download from: https://github.com/WebAssembly/binaryen/releases\n");
        }
    }
    
    println!("After installation, run 'cassette record --help' to get started!");
}