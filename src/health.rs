//! Local library health checks. This deliberately avoids decoding audio:
//! it is a fast metadata/path scan, not a full corruption test.

use std::collections::HashMap;

use std::path::{Path, PathBuf};

use crate::model::Track;

#[derive(Debug, Clone, Default)]
pub struct HealthReport {
    pub scanned: usize,
    pub missing: Vec<u64>,
    pub empty_tags: Vec<u64>,
    pub duplicates: Vec<DuplicateGroup>,
}

#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    pub title: String,
    pub artist: String,
    pub ids: Vec<u64>,
}

impl HealthReport {
    pub fn issue_count(&self) -> usize {
        self.missing.len() + self.empty_tags.len() + self.duplicates.len()
    }
}

/// Maps a path from one root directory to another.
pub fn relink_path(path: &Path, old_root: &Path, new_root: &Path) -> Option<PathBuf> {
    let normalize = |value: &Path| {
        value
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_string()
    };
    let path_text = normalize(path);
    let old_text = normalize(old_root);
    if old_text.is_empty() {
        return None;
    }
    let prefix = format!("{}/", old_text.to_lowercase());
    if !path_text.to_lowercase().starts_with(&prefix) {
        return None;
    }
    let rest = path_text.get(old_text.len() + 1..)?;
    Some(new_root.join(rest))
}
/// Repoints every virtual track from one CUE sheet to a new audio file.
pub fn relink_cue_tracks(
    tracks: &mut [Track],
    cue_source: &Path,
    new_audio: &Path,
    new_duration: f64,
) -> usize {
    let last_number = tracks
        .iter()
        .filter(|track| track.cue_source.as_deref() == Some(cue_source))
        .filter_map(|track| track.track_number)
        .max();
    let mut changed = 0;
    for track in tracks.iter_mut() {
        if track.cue_source.as_deref() != Some(cue_source) {
            continue;
        }
        track.path = new_audio.to_path_buf();
        if last_number.is_some() && track.track_number == last_number {
            track.cue_end = Some(new_duration);
            if let Some(start) = track.cue_start {
                track.duration = (new_duration - start).max(0.05);
            }
        }
        changed += 1;
    }
    changed
}

/// Preserves identity and listening history when a track is relinked.
pub fn relinked(old: &Track, mut new: Track) -> Track {
    new.id = old.id;
    new.added_at = old.added_at;
    new.cue_source = None;
    new.cue_start = None;
    new.cue_end = None;
    new
}

/// Fast checks that are safe to run on a background thread.
pub fn scan(tracks: &[Track]) -> HealthReport {
    let mut report = HealthReport {
        scanned: tracks.len(),
        ..HealthReport::default()
    };
    let mut groups: HashMap<String, DuplicateGroup> = HashMap::new();

    for track in tracks {
        if !track.path.exists() {
            report.missing.push(track.id);
        }
        if track.title.trim().is_empty()
            || track.artist.trim().is_empty()
            || track.album.trim().is_empty()
        {
            report.empty_tags.push(track.id);
        }

        let title = track.title.trim().to_lowercase();
        let artist = track.artist.trim().to_lowercase();
        if title.is_empty() || artist.is_empty() {
            continue;
        }
        // Same title + artist and durations within two seconds are candidates.
        let bucket = (track.duration / 2.0).round() as i64;
        let key = format!("{artist}\u{1f}{title}\u{1f}{bucket}");
        groups
            .entry(key)
            .or_insert_with(|| DuplicateGroup {
                title: track.title.clone(),
                artist: track.artist.clone(),
                ids: Vec::new(),
            })
            .ids
            .push(track.id);
    }

    report.duplicates = groups
        .into_values()
        .filter(|group| group.ids.len() > 1)
        .collect();
    report
        .duplicates
        .sort_by(|a, b| a.title.cmp(&b.title));
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn track(id: u64, title: &str, artist: &str, album: &str, duration: f64) -> Track {
        Track {
            id,
            path: PathBuf::from(format!("C:/missing/{id}.mp3")),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            genre: String::new(),
            year: None,
            track_number: None,
            duration,
            added_at: 0,
            modified: 0,
            cue_source: None,
            cue_start: None,
            cue_end: None,
        }
    }

    #[test]
    fn relink_path_maps_under_new_root() {
        let old = Path::new(r"D:\Music\Old");
        let new = Path::new(r"E:\Library");
        let mapped = relink_path(Path::new(r"D:\Music\Old\Album\a.flac"), old, new);
        assert_eq!(mapped, Some(PathBuf::from(r"E:\Library\Album\a.flac")));
        assert!(relink_path(Path::new(r"D:\Other\a.flac"), old, new).is_none());
    }
    #[test]
    fn relink_cue_updates_all_tracks_and_last_end() {
        let mut tracks = vec![
            track(1, "A", "X", "Y", 100.0),
            track(2, "B", "X", "Y", 100.0),
        ];
        let cue = PathBuf::from("album.cue");
        tracks[0].track_number = Some(1);
        tracks[0].cue_source = Some(cue.clone());
        tracks[0].cue_start = Some(0.0);
        tracks[0].cue_end = Some(100.0);
        tracks[1].track_number = Some(2);
        tracks[1].cue_source = Some(cue.clone());
        tracks[1].cue_start = Some(100.0);
        tracks[1].cue_end = Some(200.0);
        let changed = relink_cue_tracks(&mut tracks, &cue, Path::new(r"E:\New\album.flac"), 260.0);
        assert_eq!(changed, 2);
        assert_eq!(tracks[0].path, PathBuf::from(r"E:\New\album.flac"));
        assert_eq!(tracks[1].cue_end, Some(260.0));
        assert_eq!(tracks[1].duration, 160.0);
    }

    #[test]
    fn relink_preserves_identity_and_clears_cue() {
        let old = track(7, "Old", "Artist", "Album", 100.0);
        let mut new = track(99, "New", "Artist", "Album", 120.0);
        new.added_at = 42;
        new.cue_source = Some(PathBuf::from("x.cue"));
        new.cue_start = Some(1.0);
        new.cue_end = Some(2.0);
        let relinked = relinked(&old, new);
        assert_eq!(relinked.id, 7);
        assert_eq!(relinked.added_at, old.added_at);
        assert!(relinked.cue_source.is_none());
        assert!(relinked.cue_start.is_none());
    }

    #[test]
    fn finds_missing_empty_and_duplicates() {
        let tracks = vec![
            track(1, "Song", "Artist", "Album", 180.0),
            track(2, "Song", "Artist", "Album", 180.8),
            track(3, "", "Artist", "Album", 120.0),
        ];
        let report = scan(&tracks);
        assert_eq!(report.scanned, 3);
        assert_eq!(report.missing.len(), 3);
        assert_eq!(report.empty_tags, vec![3]);
        assert_eq!(report.duplicates.len(), 1);
        assert_eq!(report.duplicates[0].ids, vec![1, 2]);
    }
}
