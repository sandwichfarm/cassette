use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Only embed cassette-tools if we're building with the deck feature
    if env::var("CARGO_FEATURE_DECK").is_ok() {
        println!("cargo:rerun-if-changed=../cassette-tools/src/");
        println!("cargo:rerun-if-changed=../cassette-tools/Cargo.toml");
        
        let out_dir = env::var("OUT_DIR").unwrap();
        let cassette_tools_dir = Path::new("../cassette-tools");
        
        // Check if cassette-tools exists
        if !cassette_tools_dir.exists() {
            panic!("cassette-tools directory not found at {:?}", cassette_tools_dir);
        }
        
        // Create a directory to store cassette-tools source
        let embedded_tools_dir = Path::new(&out_dir).join("embedded_cassette_tools");
        if embedded_tools_dir.exists() {
            fs::remove_dir_all(&embedded_tools_dir).ok();
        }
        fs::create_dir_all(&embedded_tools_dir).expect("Failed to create embedded tools directory");
        
        // Copy cassette-tools source files
        println!("Embedding cassette-tools source...");
        
        // Copy Cargo.toml
        fs::copy(
            cassette_tools_dir.join("Cargo.toml"),
            embedded_tools_dir.join("Cargo.toml")
        ).expect("Failed to copy Cargo.toml");
        
        // Copy src directory
        let src_dir = cassette_tools_dir.join("src");
        let dest_src_dir = embedded_tools_dir.join("src");
        fs::create_dir_all(&dest_src_dir).expect("Failed to create src directory");
        
        // Recursively copy all source files
        copy_dir_all(&src_dir, &dest_src_dir).expect("Failed to copy source files");
        
        // Set environment variable with the path
        println!("cargo:rustc-env=EMBEDDED_CASSETTE_TOOLS_DIR={}", embedded_tools_dir.display());
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}