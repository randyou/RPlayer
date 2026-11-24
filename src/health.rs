//! Local library health checks. This deliberately avoids decoding audio:
//! it is a fast metadata/path scan, not a full corruption test.

use std::collections::HashMap;

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
