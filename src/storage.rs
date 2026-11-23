//! Local persistence. RPlayer never uploads the library anywhere.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::Library;

pub const APP_DIR_NAME: &str = "RPlayer";
pub const LIBRARY_FILE: &str = "library.json";
pub const SCHEMA_VERSION: u32 = 5;

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

/// Cached lyrics downloaded from LRCLIB live under the app data directory so
/// they survive restarts and never require write access to the music folder.
pub fn lyrics_cache_path(track_id: u64) -> PathBuf {
    data_dir()
        .join("lyrics")
        .join(format!("{track_id:016x}.lrc"))
}

pub fn cover_cache_dir() -> PathBuf {
    data_dir().join("covers")
}

/// Downloaded album art. The cache key is an album-level hash.
pub fn cover_cache_path(key: &str) -> PathBuf {
    cover_cache_dir().join(format!("{key}.img"))
}

/// Negative cache marker for albums MusicBrainz could not match.
pub fn cover_miss_path(key: &str) -> PathBuf {
    cover_cache_dir().join(format!("{key}.miss"))
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
                library.desktop_lyrics_opacity =
                    library.desktop_lyrics_opacity.clamp(0.0, 1.0);
                library.desktop_lyrics_text_opacity =
                    library.desktop_lyrics_text_opacity.clamp(0.0, 1.0);
                library.eq = library.eq.sanitized();
                // v0.4 makes the paper theme the default. Older files only had
                // a boolean, so migrate once; the user can switch back instantly.
                if file.schema < 4 {
                    library.light_theme = true;
                    library.desktop_lyrics_opacity = 0.85;
                    library.auto_lyrics = true;
                }
                if file.schema < 5 {
                    library.auto_cover = true;
                }
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
        schema: SCHEMA_VERSION,
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
        lib.eq = crate::eq::EqSettings::from_preset(4);
        lib.light_theme = true;
        lib.desktop_lyrics_opacity = 0.6;
        lib.auto_lyrics = false;
        let payload = LibraryFile {
            schema: SCHEMA_VERSION,
            library: lib,
        };
        let bytes = serde_json::to_vec(&payload).unwrap();
        let decoded: LibraryFile = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.schema, SCHEMA_VERSION);
        assert_eq!(decoded.library.volume, 0.33);
        assert_eq!(decoded.library.folders.len(), 1);
        assert!(decoded.library.light_theme);
        assert!((decoded.library.desktop_lyrics_opacity - 0.6).abs() < 0.01);
        assert!(!decoded.library.auto_lyrics);
        assert_eq!(decoded.library.eq.preset, Some(4));
        assert!(decoded.library.eq.enabled);
    }
}
