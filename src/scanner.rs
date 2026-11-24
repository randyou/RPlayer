//! Background library scanner.
//!
//! Metadata parsing can be slow on large libraries, so it runs on a worker
//! thread and streams results back to the UI through a channel.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use walkdir::{DirEntry, WalkDir};

use crate::model::{is_audio_file, path_key, Track};

pub enum ScanEvent {
    Started { total: usize },
    Track(Box<Track>),
    Unchanged(u64),
    Progress { scanned: usize, total: usize },
    Finished { errors: Vec<String> },
}

/// Starts scanning all folders. The returned receiver is polled by the UI.
pub fn start_scan(
    folders: Vec<PathBuf>,
    known: HashMap<String, (u64, u64)>,
) -> Receiver<ScanEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("rplayer-scanner".into())
        .spawn(move || scan_worker(folders, known, tx))
        .expect("failed to spawn scanner thread");
    rx
}

fn scan_worker(
    folders: Vec<PathBuf>,
    known: HashMap<String, (u64, u64)>,
    tx: Sender<ScanEvent>,
) {
    let mut audio_files = Vec::new();
    let mut cue_files = Vec::new();
    let mut seen = HashSet::new();

    for folder in &folders {
        if !folder.exists() {
            continue;
        }
        let walker = WalkDir::new(folder)
            .follow_links(false)
            .into_iter()
            .filter_entry(should_enter);
        for entry in walker.filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("cue"))
            {
                let key = path_key(path);
                if seen.insert(format!("cue:{key}")) {
                    cue_files.push(path.to_path_buf());
                }
            } else if is_audio_file(path) {
                let key = path_key(path);
                if seen.insert(key) {
                    audio_files.push(path.to_path_buf());
                }
            }
        }
    }

    audio_files.sort();
    cue_files.sort();
    let mut errors = Vec::new();
    let mut cue_jobs = Vec::new();
    let mut cue_audio = HashSet::new();
    for path in cue_files {
        match crate::cue::load(&path) {
            Ok(sheet) => {
                if !sheet.file.exists() {
                    errors.push(format!("{}: 音频文件不存在", sheet.file.display()));
                    continue;
                }
                let key = path_key(&sheet.file);
                if cue_audio.insert(key) {
                    cue_jobs.push((path, sheet));
                }
            }
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }

    let cue_track_total: usize = cue_jobs.iter().map(|(_, sheet)| sheet.tracks.len()).sum();
    let plain_audio = audio_files
        .iter()
        .filter(|path| !cue_audio.contains(&path_key(path)))
        .count();
    let total = plain_audio + cue_track_total;
    if tx.send(ScanEvent::Started { total }).is_err() {
        return;
    }
    let mut scanned = 0usize;

    for (cue_path, sheet) in cue_jobs {
        let base = match Track::from_path(&sheet.file) {
            Ok(track) => track,
            Err(error) => {
                errors.push(format!("{}: {error}", sheet.file.display()));
                continue;
            }
        };
        let modified = std::fs::metadata(&cue_path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0);
        for track in crate::cue::virtual_tracks(&sheet, &cue_path, &base, modified) {
            scanned += 1;
            if tx.send(ScanEvent::Track(Box::new(track))).is_err() {
                return;
            }
            let _ = tx.send(ScanEvent::Progress { scanned, total });
        }
    }

    for path in audio_files {
        let key = path_key(&path);
        if cue_audio.contains(&key) {
            continue;
        }
        let modified = std::fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0);
        if let Some((id, known_modified)) = known.get(&key) {
            if modified > 0 && *known_modified == modified {
                scanned += 1;
                if tx.send(ScanEvent::Unchanged(*id)).is_err() {
                    return;
                }
                let _ = tx.send(ScanEvent::Progress { scanned, total });
                continue;
            }
        }
        match Track::from_path(&path) {
            Ok(track) => {
                scanned += 1;
                if tx.send(ScanEvent::Track(Box::new(track))).is_err() {
                    return;
                }
            }
            Err(err) => {
                scanned += 1;
                if errors.len() < 20 {
                    errors.push(format!("{}: {err}", path.display()));
                }
            }
        }
        if tx.send(ScanEvent::Progress { scanned, total }).is_err() {
            return;
        }
    }

    let _ = tx.send(ScanEvent::Finished { errors });
}
fn should_enter(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    // Hidden files and macOS resource-fork metadata are not music.
    !(name.starts_with('.') || name.starts_with("._"))
}

/// Scans a folder synchronously. Useful for tests and command-line tooling.
#[allow(dead_code)]
pub fn scan_folder(folder: &Path) -> Vec<Track> {
    let rx = start_scan(vec![folder.to_path_buf()], HashMap::new());
    let mut tracks = Vec::new();
    while let Ok(event) = rx.recv() {
        match event {
            ScanEvent::Track(track) => tracks.push(*track),
            ScanEvent::Finished { .. } => break,
            _ => {}
        }
    }
    tracks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_wav(path: &Path) {
        let samples: Vec<i16> = (0..200).map(|i| ((i * 37) % 1000) as i16).collect();
        let data_len = (samples.len() * 2) as u32;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&44_100u32.to_le_bytes());
        bytes.extend_from_slice(&88_200u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(&bytes).unwrap();
    }

    #[test]
    fn cue_replaces_audio_file_with_virtual_tracks() {
        let dir = std::env::temp_dir().join(format!(
            "rplayer-cue-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        write_wav(&dir.join("album.wav"));
        std::fs::write(
            dir.join("album.cue"),
            r#"PERFORMER "Artist"
TITLE "Album"
FILE "album.wav" WAVE
  TRACK 01 AUDIO
    TITLE "One"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Two"
    INDEX 01 00:00:01
"#,
        )
        .unwrap();
        let tracks = scan_folder(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(tracks.len(), 2, "plain audio must be replaced by CUE tracks");
        assert_eq!(tracks[0].title, "One");
        assert_eq!(tracks[0].cue_start, Some(0.0));
        assert!(tracks[0].cue_source.is_some());
        assert_ne!(tracks[0].id, tracks[1].id);
    }

    #[test]
    fn unchanged_file_is_reused_without_reparsing() {
        let dir = std::env::temp_dir().join(format!("rplayer-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("probe.wav");
        write_wav(&path);
        let track = Track::from_path(&path).unwrap();
        assert!(track.modified > 0);
        let known = HashMap::from([(
            path_key(&path),
            (track.id, track.modified),
        )]);
        let rx = start_scan(vec![dir.clone()], known);
        let mut reused = None;
        while let Ok(event) = rx.recv() {
            match event {
                ScanEvent::Unchanged(id) => reused = Some(id),
                ScanEvent::Finished { .. } => break,
                _ => {}
            }
        }
        assert_eq!(reused, Some(track.id));
        let _ = std::fs::remove_dir_all(dir);
    }
}