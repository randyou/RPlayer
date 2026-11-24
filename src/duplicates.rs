//! Duplicate detection: same normalized title/artist and close duration.

use std::collections::HashMap;

use crate::model::Track;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateGroup {
    pub track_ids: Vec<u64>,
}

fn key(track: &Track) -> (String, String) {
    (
        track.title.trim().to_lowercase(),
        track.artist.trim().to_lowercase(),
    )
}

/// Groups tracks with the same title/artist and duration within two seconds.
pub fn find(tracks: &[Track]) -> Vec<DuplicateGroup> {
    let mut buckets: HashMap<(String, String), Vec<&Track>> = HashMap::new();
    for track in tracks {
        let (title, artist) = key(track);
        if title.is_empty() {
            continue;
        }
        buckets.entry((title, artist)).or_default().push(track);
    }

    let mut groups = Vec::new();
    for mut bucket in buckets.into_values() {
        if bucket.len() < 2 {
            continue;
        }
        bucket.sort_by(|a, b| a.duration.total_cmp(&b.duration));
        let mut current: Vec<&Track> = Vec::new();
        for track in bucket {
            let fits = current
                .first()
                .is_none_or(|first| track.duration - first.duration <= 2.0);
            if fits {
                current.push(track);
            } else {
                if current.len() >= 2 {
                    groups.push(DuplicateGroup {
                        track_ids: current.iter().map(|track| track.id).collect(),
                    });
                }
                current = vec![track];
            }
        }
        if current.len() >= 2 {
            groups.push(DuplicateGroup {
                track_ids: current.iter().map(|track| track.id).collect(),
            });
        }
    }
    groups.sort_by(|a, b| {
        let a_title = a.track_ids.first().copied().unwrap_or(0);
        let b_title = b.track_ids.first().copied().unwrap_or(0);
        a_title.cmp(&b_title)
    });
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Track;
    use std::path::PathBuf;

    fn track(id: u64, title: &str, artist: &str, duration: f64) -> Track {
        let mut track = Track {
            id,
            path: PathBuf::from(format!("{id}.flac")),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            genre: String::new(),
            year: None,
            track_number: None,
            duration: 0.0,
            added_at: 0,
            modified: 0,
        };
        track.title = title.to_string();
        track.artist = artist.to_string();
        track.duration = duration;
        track
    }

    #[test]
    fn groups_close_duplicates_and_ignores_other_artists() {
        let tracks = vec![
            track(1, "晴天", "周杰伦", 269.0),
            track(2, "晴天", "周杰伦", 270.5),
            track(3, "晴天", "翻唱", 269.0),
            track(4, "晴天", "周杰伦", 280.0),
        ];
        let groups = find(&tracks);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].track_ids, vec![1, 2]);
    }

    #[test]
    fn ignores_unique_tracks() {
        let tracks = vec![
            track(1, "A", "X", 100.0),
            track(2, "B", "X", 100.0),
        ];
        assert!(find(&tracks).is_empty());
    }
}
