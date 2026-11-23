//! Local persistence. RPlayer never uploads the library anywhere.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::Library;

pub const APP_DIR_NAME: &str = "RPlayer";
pub const LIBRARY_FILE: &str = "library.json";

/// Per-user data directory:
/// - Windows: `%APPDATA%\RPlayer`
/// - macOS: `~/Library/Application Support/RPlayer`
/// - Linux: `~/.local/share/RPlayer`
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR_NAME)
}

pub fn library_path() -> PathBuf {
    data_dir().join(LIBRARY_FILE)
}

/// On-disk wrapper with a schema version so future migrations are possible.
#[derive(Debug, Serialize, Deserialize)]
struct LibraryFile {
    schema: u32,
    #[serde(flatten)]
    library: Library,
}

pub fn load_library() -> Library {
    let path = library_path();
    match fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<LibraryFile>(&bytes) {
            Ok(file) => {
                let mut library = file.library;
                library.version = file.schema.max(1);
                library.volume = library.volume.clamp(0.0, 1.0);
                library
            }
            Err(err) => {
                eprintln!("[RPlayer] failed to parse {}: {err}", path.display());
                Library::default()
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Library::default(),
        Err(err) => {
            eprintln!("[RPlayer] failed to read {}: {err}", path.display());
            Library::default()
        }
    }
}

pub fn save_library(library: &Library) -> Result<()> {
    let dir = data_dir();
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(LIBRARY_FILE);
    let payload = LibraryFile {
        schema: 1,
        library: library.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&payload)?;

    // Write-then-rename keeps the previous library intact if the process dies mid-write.
    let tmp = dir.join(format!("{LIBRARY_FILE}.tmp"));
    fs::write(&tmp, bytes).with_context(|| format!("write {}", tmp.display()))?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("replace {}", path.display()))?;
    }
    fs::rename(&tmp, &path).with_context(|| format!("rename {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_file_roundtrips_through_json() {
        let mut lib = Library::default();
        lib.volume = 0.33;
        lib.folders.push(PathBuf::from("D:/Music"));
        let payload = LibraryFile {
            schema: 1,
            library: lib,
        };
        let bytes = serde_json::to_vec(&payload).unwrap();
        let decoded: LibraryFile = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.schema, 1);
        assert_eq!(decoded.library.volume, 0.33);
        assert_eq!(decoded.library.folders.len(), 1);
    }
}
