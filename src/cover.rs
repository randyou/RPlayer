//! Cover discovery: embedded art, sidecar images and the on-disk cache.

use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{self, Track};
use crate::storage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverSource {
    Embedded,
    Sidecar,
    Cache,
    Online,
}

impl CoverSource {
    pub fn short(self) -> &'static str {
        match self {
            Self::Embedded => "内嵌",
            Self::Sidecar => "本地",
            Self::Cache => "缓存",
            Self::Online => "网络",
        }
    }
}

#[derive(Debug)]
pub struct LocalCover {
    pub bytes: Vec<u8>,
    pub source: CoverSource,
}

/// Album-level key so every track on the same album shares one cover.
pub fn cache_key(track: &Track) -> String {
    let artist = normalize(&track.artist);
    let album = normalize(&track.album);
    let title = normalize(&track.title);
    let key = if !album.is_empty() && album != "未知专辑" {
        format!("{artist}|{album}")
    } else {
        format!("{artist}|{title}")
    };
    format!("{:016x}", fnv1a(key.as_bytes()))
}

/// Embedded art wins, then common sidecar files, then the app cache.
pub fn load_local(track: &Track) -> Option<LocalCover> {
    if let Some(bytes) = model::read_cover(&track.path) {
        if !bytes.is_empty() {
            return Some(LocalCover {
                bytes,
                source: CoverSource::Embedded,
            });
        }
    }
    if let Some(bytes) = read_sidecar(&track.path) {
        return Some(LocalCover {
            bytes,
            source: CoverSource::Sidecar,
        });
    }
    if let Some(bytes) = read_cache(track) {
        return Some(LocalCover {
            bytes,
            source: CoverSource::Cache,
        });
    }
    None
}

pub fn read_cache(track: &Track) -> Option<Vec<u8>> {
    let path = storage::cover_cache_path(&cache_key(track));
    let bytes = fs::read(path).ok()?;
    if bytes.is_empty() {
        None
    } else {
        Some(bytes)
    }
}

pub fn store_cache(track: &Track, bytes: &[u8]) -> std::io::Result<()> {
    let path = storage::cover_cache_path(&cache_key(track));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    let _ = fs::remove_file(storage::cover_miss_path(&cache_key(track)));
    Ok(())
}

pub fn clear_cache() -> std::io::Result<()> {
    let dir = storage::cover_cache_dir();
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

pub fn cache_size() -> u64 {
    let dir = storage::cover_cache_dir();
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| entry.metadata().ok())
        .filter(|meta| meta.is_file())
        .map(|meta| meta.len())
        .sum()
}

pub fn has_recent_miss(track: &Track) -> bool {
    let path = storage::cover_miss_path(&cache_key(track));
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    modified
        .elapsed()
        .map(|elapsed| elapsed.as_secs() < 30 * 24 * 60 * 60)
        .unwrap_or(false)
}

pub fn mark_miss(track: &Track) {
    let path = storage::cover_miss_path(&cache_key(track));
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, crate::model::now_millis().to_string());
}

fn read_sidecar(audio: &Path) -> Option<Vec<u8>> {
    let directory = audio.parent()?;
    let audio_stem = audio
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let mut candidates: Vec<PathBuf> = Vec::new();
    let entries = fs::read_dir(directory).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let image = matches!(
            extension.as_str(),
            "jpg" | "jpeg" | "png" | "webp" | "bmp"
        );
        let common = matches!(
            stem.as_str(),
            "cover" | "folder" | "front" | "album" | "artwork"
        );
        // `<track-name>.jpg` wins over a shared folder image, so a mixed
        // folder can still give each track its own art.
        if image && (common || stem == audio_stem) {
            candidates.push(path);
        }
    }
    // Deterministic preference when several exist.
    candidates.sort_by_key(|path| {
        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if stem == audio_stem {
            return 0;
        }
        match stem.as_str() {
            "cover" => 1,
            "folder" => 2,
            "front" => 3,
            "album" => 4,
            _ => 5,
        }
    });
    for candidate in candidates {
        if let Ok(meta) = fs::metadata(&candidate) {
            if meta.len() > 12 * 1024 * 1024 {
                continue;
            }
        }
        if let Ok(bytes) = fs::read(candidate) {
            if !bytes.is_empty() {
                return Some(bytes);
            }
        }
    }
    None
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['（', '）', '【', '】', '《', '》'], "")
        .replace(['(', ')', '[', ']'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn album_cache_key_is_stable_and_album_scoped() {
        let make = |title: &str, album: &str| Track {
            id: 1,
            path: PathBuf::from("song.flac"),
            title: title.into(),
            artist: "Artist".into(),
            album: album.into(),
            genre: String::new(),
            year: None,
            track_number: None,
            duration: 200.0,
            added_at: 0,
        };
        let first = make("One", "Album");
        let second = make("Two", "Album");
        assert_eq!(cache_key(&first), cache_key(&second));
        assert_eq!(cache_key(&first), cache_key(&first));
        assert_ne!(cache_key(&first), cache_key(&make("One", "Other")));
    }
}
