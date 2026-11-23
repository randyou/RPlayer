//! Lightweight folder watcher.
//!
//! Instead of pulling in a platform watcher we fingerprint audio files on a
//! worker thread every few seconds. The library is small enough for this to be
//! cheap, and it behaves consistently on local disks, network shares and
//! removable media.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use walkdir::WalkDir;

use crate::model::is_audio_file;

pub struct FolderWatcher {
    rx: Receiver<()>,
}

impl FolderWatcher {
    /// Returns true when at least one change signal is waiting.
    pub fn poll_changed(&self) -> bool {
        let mut changed = false;
        while self.rx.try_recv().is_ok() {
            changed = true;
        }
        changed
    }
}

pub fn start(folders: Vec<PathBuf>) -> FolderWatcher {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("rplayer-watcher".into())
        .spawn(move || {
            let mut previous = fingerprint(&folders);
            loop {
                thread::sleep(Duration::from_secs(3));
                let current = fingerprint(&folders);
                if current != previous {
                    previous = current;
                    if tx.send(()).is_err() {
                        break;
                    }
                }
            }
        })
        .expect("failed to spawn folder watcher");
    FolderWatcher { rx }
}

fn fingerprint(folders: &[PathBuf]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for folder in folders {
        folder.hash(&mut hasher);
        if !folder.exists() {
            continue;
        }
        for entry in WalkDir::new(folder)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !entry.file_type().is_file() || !is_audio_file(path) {
                continue;
            }
            path.hash(&mut hasher);
            if let Ok(meta) = entry.metadata() {
                meta.len().hash(&mut hasher);
                if let Ok(modified) = meta.modified() {
                    modified
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|duration| duration.as_secs())
                        .unwrap_or(0)
                        .hash(&mut hasher);
                }
            }
        }
    }
    hasher.finish()
}
