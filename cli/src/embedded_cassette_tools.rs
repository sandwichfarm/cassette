/// Embedded cassette-tools access
/// Provides a stable on-disk path at runtime by extracting
/// embedded sources from the binary into a temp directory.

#[cfg(feature = "deck")]
mod embedded {
    use include_dir::{include_dir, Dir};
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    // Embed the cassette-tools directory at compile time
    static TOOLS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../cassette-tools");
    static EXTRACTED_PATH: OnceLock<PathBuf> = OnceLock::new();

    pub fn get_embedded_tools_dir() -> &'static Path {
        EXTRACTED_PATH
            .get_or_init(|| {
                // Use a versioned temp folder to avoid collisions
                let base = std::env::temp_dir()
                    .join(format!("cassette-tools-{}", env!("CARGO_PKG_VERSION")));

                // Clean and extract fresh to avoid partial contents
                let _ = std::fs::remove_dir_all(&base);
                std::fs::create_dir_all(&base).ok();
                // Extract embedded files
                // Ignore extraction errors; later file access will surface them explicitly
                let _ = TOOLS_DIR.extract(&base);
                base
            })
            .as_path()
    }

    pub use get_embedded_tools_dir as exported_get_embedded_tools_dir;
}

#[cfg(feature = "deck")]
pub use embedded::exported_get_embedded_tools_dir as get_embedded_tools_dir;

#[cfg(not(feature = "deck"))]
pub fn get_embedded_tools_dir() -> &'static std::path::Path {
    std::path::Path::new("")
}
