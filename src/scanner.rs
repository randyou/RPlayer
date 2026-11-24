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
        let file_time = |path: &std::path::Path| {
            std::fs::metadata(path)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0)
        };
        let modified = file_time(&cue_path).max(file_time(&sheet.file));
        for track in crate::cue::virtual_tracks(&sheet, &cue_path, &base, modified) {
            let key = crate::model::scan_key(&track);
            if let Some((id, known_modified)) = known.get(&key) {
                scanned += 1;
                if *known_modified == modified {
                    if tx.send(ScanEvent::Unchanged(*id)).is_err() {
                        return;
                    }
                    let _ = tx.send(ScanEvent::Progress { scanned, total });
                    continue;
                }
            }
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

    fn write_wav(path: &Path, seconds: u32) {
        let samples: Vec<i16> = (0..44_100 * seconds)
            .map(|i| ((i * 37) % 1000) as i16)
            .collect();
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
        let audio = dir.join("专辑 音轨.wav");
        write_wav(&audio, 12);
        std::fs::write(
            dir.join("album.cue"),
            r#"PERFORMER "Artist"
TITLE "Album"
FILE "专辑 音轨.wav" WAVE
  TRACK 01 AUDIO
    TITLE "One"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Two"
    INDEX 01 00:05:00
  TRACK 03 AUDIO
    TITLE "Three"
    INDEX 01 00:10:00
"#,
        )
        .unwrap();
        let tracks = scan_folder(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(tracks.len(), 3, "plain audio must be replaced by CUE tracks");
        assert_eq!(tracks[0].title, "One");
        assert_eq!(tracks[0].cue_start, Some(0.0));
        assert!(tracks[0].cue_source.is_some());
        assert_ne!(tracks[0].id, tracks[1].id);

        for track in &tracks {
            assert_eq!(track.path, audio, "CUE FILE with spaces/unicode must resolve");
            assert_eq!(track.artist, "Artist");
            assert_eq!(track.album, "Album");
        }
        assert!((tracks[0].duration - 5.0).abs() < 0.001);
        assert_eq!(tracks[0].cue_end, Some(5.0));
        assert!((tracks[1].duration - 5.0).abs() < 0.001);
        assert_eq!(tracks[1].cue_start, Some(5.0));
        assert_eq!(tracks[1].cue_end, Some(10.0));
        assert!((tracks[2].duration - 2.0).abs() < 0.001);
        assert_eq!(tracks[2].cue_start, Some(10.0));
        assert_eq!(tracks[2].cue_end, Some(12.0));

        // The same mapping the player uses: UI time is relative to the segment.
        assert_eq!(crate::playback::start(&tracks[1], None), 5.0);
        assert_eq!(crate::playback::absolute(&tracks[1], 1.5), 6.5);
        assert_eq!(crate::playback::relative(&tracks[1], 6.5), 1.5);
        assert!(crate::playback::reached_end(&tracks[1], 9.99));
        assert!(!crate::playback::reached_end(&tracks[1], 9.0));
    }

    #[test]
    fn cue_tracks_are_reused_on_rescan() {
        let dir = std::env::temp_dir().join(format!(
            "rplayer-cue-rescan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        write_wav(&dir.join("album.wav"), 2);
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
        let first = scan_folder(&dir);
        let known = first
            .iter()
            .map(|track| (crate::model::scan_key(track), (track.id, track.modified)))
            .collect();
        let rx = start_scan(vec![dir.clone()], known);
        let mut unchanged = 0;
        let mut tracks = 0;
        while let Ok(event) = rx.recv() {
            match event {
                ScanEvent::Track(_) => tracks += 1,
                ScanEvent::Unchanged(_) => unchanged += 1,
                ScanEvent::Finished { .. } => break,
                _ => {}
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(tracks, 0, "second scan should reuse CUE tracks");
        assert_eq!(unchanged, 2);
    }

    #[test]
    fn unchanged_file_is_reused_without_reparsing() {
        let dir = std::env::temp_dir().join(format!("rplayer-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("probe.wav");
        write_wav(&path, 1);
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