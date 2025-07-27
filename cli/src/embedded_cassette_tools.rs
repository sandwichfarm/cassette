/// Embedded cassette-tools path for deck functionality
/// This module provides the path to the embedded cassette-tools source
/// that gets copied into the build directory at compile time

use std::path::Path;

#[cfg(feature = "deck")]
pub fn get_embedded_tools_dir() -> &'static Path {
    Path::new(env!("EMBEDDED_CASSETTE_TOOLS_DIR"))
}

#[cfg(not(feature = "deck"))]
pub fn get_embedded_tools_dir() -> &'static Path {
    Path::new("")
}