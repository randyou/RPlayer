//! Local persistence. RPlayer never uploads the library anywhere.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::{Library, Track};

pub const APP_DIR_NAME: &str = "RPlayer";
pub const LIBRARY_FILE: &str = "library.json";
pub const SCHEMA_VERSION: u32 = 6;

/// Per-user data directory:
/// - Windows: `%APPDATA%\RPlayer`
/// - macOS: `~/Library/Application Support/RPlayer`
/// - Linux: `~/.local/share/RPlayer`
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR_NAME)
}

pub fn library_db_path() -> PathBuf {
    data_dir().join("library.db")
}

/// Cached lyrics downloaded from LRCLIB live under the app data directory so
/// they survive restarts and never require write access to the music folder.
pub fn lyrics_cache_path(track_id: u64) -> PathBuf {
    data_dir()
        .join("lyrics")
        .join(format!("{track_id:016x}.lrc"))
}

pub fn custom_lyrics_path(track_id: u64) -> PathBuf {
    data_dir()
        .join("lyrics_custom")
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
    let dir = data_dir();
    let path = dir.join(LIBRARY_FILE);
    let db_exists = library_db_path().exists();
    // Recover from an interrupted write or a corrupted main file: the .tmp
    // file is complete before rename, and .bak is the previous good save.
    let candidates = [
        path.clone(),
        dir.join(format!("{LIBRARY_FILE}.tmp")),
        dir.join(format!("{LIBRARY_FILE}.bak")),
    ];
    for candidate in &candidates {
        let Ok(bytes) = fs::read(candidate) else {
            continue;
        };
        match serde_json::from_slice::<LibraryFile>(&bytes) {
            Ok(file) => {
                let from_main = candidate == &path;
                return apply_loaded_library(file, db_exists, from_main, &path);
            }
            Err(err) => {
                eprintln!(
                    "[RPlayer] failed to parse {}: {err}",
                    candidate.display()
                );
            }
        }
    }
    Library::default()
}

fn apply_loaded_library(
    file: LibraryFile,
    db_exists: bool,
    from_main: bool,
    path: &std::path::Path,
) -> Library {
    let mut library = file.library;
    library.version = file.schema.max(1);
    library.volume = library.volume.clamp(0.0, 1.0);
    library.desktop_lyrics_opacity = library.desktop_lyrics_opacity.clamp(0.0, 1.0);
    library.desktop_lyrics_text_opacity =
        library.desktop_lyrics_text_opacity.clamp(0.0, 1.0);
    library.eq = library.eq.sanitized();
    // v0.4 makes the paper theme the default. Older files only had a
    // boolean, so migrate once; the user can switch back instantly.
    if file.schema < 4 {
        library.light_theme = true;
        library.desktop_lyrics_opacity = 0.85;
        library.auto_lyrics = true;
    }
    if file.schema < 5 {
        library.auto_cover = true;
    }
    if db_exists {
        if let Ok(tracks) = load_tracks_from_db() {
            library.tracks = tracks;
            library.refresh_index();
        }
    } else if from_main && !library.tracks.is_empty() {
        // First SQLite launch: keep a JSON backup, then import the tracks.
        let backup = data_dir().join(format!("{LIBRARY_FILE}.bak"));
        let _ = fs::copy(path, backup);
        let _ = save_tracks_to_db(&library.tracks);
    }
    library
}

pub fn save_library(library: &Library) -> Result<()> {
    let dir = data_dir();
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(LIBRARY_FILE);
    // Tracks live in SQLite; JSON keeps settings and playlists. If SQLite
    // fails, fall back to embedding tracks in JSON so nothing is lost.
    let db_ok = save_tracks_to_db(&library.tracks).is_ok();
    let mut settings = library.clone();
    if db_ok {
        settings.tracks.clear();
    }
    let payload = LibraryFile {
        schema: SCHEMA_VERSION,
        library: settings,
    };
    let bytes = serde_json::to_vec_pretty(&payload)?;

    // Write-then-rename keeps the previous library intact if the process dies mid-write.
    let tmp = dir.join(format!("{LIBRARY_FILE}.tmp"));
    fs::write(&tmp, bytes).with_context(|| format!("write {}", tmp.display()))?;
    if path.exists() {
        let backup = dir.join(format!("{LIBRARY_FILE}.bak"));
        let _ = fs::copy(&path, backup);
        fs::remove_file(&path).with_context(|| format!("replace {}", path.display()))?;
    }
    fs::rename(&tmp, &path).with_context(|| format!("rename {}", path.display()))?;
    Ok(())
}

fn open_library_db() -> Result<rusqlite::Connection> {
    let path = library_db_path();
    let conn = rusqlite::Connection::open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            album TEXT NOT NULL,
            genre TEXT NOT NULL,
            year INTEGER,
            track_number INTEGER,
            duration REAL NOT NULL,
            added_at INTEGER NOT NULL,
            modified INTEGER NOT NULL DEFAULT 0,
            cue_source TEXT,
            cue_start REAL,
            cue_end REAL
         );",
    )?;
    // Older databases predate the incremental-scan timestamp.
    let _ = conn.execute(
        "ALTER TABLE tracks ADD COLUMN modified INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute("ALTER TABLE tracks ADD COLUMN cue_source TEXT", []);
    let _ = conn.execute("ALTER TABLE tracks ADD COLUMN cue_start REAL", []);
    let _ = conn.execute("ALTER TABLE tracks ADD COLUMN cue_end REAL", []);
    Ok(conn)
}

fn load_tracks_from_db() -> Result<Vec<Track>> {
    let conn = open_library_db()?;
    let mut stmt = conn.prepare(
        "SELECT id,path,title,artist,album,genre,year,track_number,duration,added_at,modified,cue_source,cue_start,cue_end
         FROM tracks ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Track {
            id: row.get::<_, i64>(0)? as u64,
            path: PathBuf::from(row.get::<_, String>(1)?),
            title: row.get(2)?,
            artist: row.get(3)?,
            album: row.get(4)?,
            genre: row.get(5)?,
            year: row.get::<_, Option<i64>>(6)?.map(|v| v as u32),
            track_number: row.get::<_, Option<i64>>(7)?.map(|v| v as u32),
            duration: row.get(8)?,
            added_at: row.get::<_, i64>(9)? as u64,
            modified: row.get::<_, i64>(10)? as u64,
            cue_source: row.get::<_, Option<String>>(11)?.map(PathBuf::from),
            cue_start: row.get(12)?,
            cue_end: row.get(13)?,
        })
    })?;
    let mut tracks = Vec::new();
    for row in rows {
        tracks.push(row?);
    }
    Ok(tracks)
}

fn save_tracks_to_db(tracks: &[Track]) -> Result<()> {
    let mut conn = open_library_db()?;
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM tracks", [])?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO tracks
             (id,path,title,artist,album,genre,year,track_number,duration,added_at,modified,cue_source,cue_start,cue_end)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        )?;
        for track in tracks {
            stmt.execute(rusqlite::params![
                track.id as i64,
                track.path.to_string_lossy(),
                track.title,
                track.artist,
                track.album,
                track.genre,
                track.year.map(|v| v as i64),
                track.track_number.map(|v| v as i64),
                track.duration,
                track.added_at as i64,
                track.modified as i64,
                track.cue_source.as_ref().map(|path| path.to_string_lossy().to_string()),
                track.cue_start,
                track.cue_end,
            ])?;
        }
    }
    tx.commit()?;
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
