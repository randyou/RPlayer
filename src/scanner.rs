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
    let mut files = Vec::new();
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
            if !entry.file_type().is_file() || !is_audio_file(entry.path()) {
                continue;
            }
            let key = path_key(entry.path());
            if seen.insert(key) {
                files.push(entry.path().to_path_buf());
            }
        }
    }

    files.sort();
    let total = files.len();
    if tx.send(ScanEvent::Started { total }).is_err() {
        return;
    }

    let mut errors = Vec::new();
    let mut scanned = 0usize;

    for path in files {
        let key = path_key(&path);
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
