//! User-owned playback statistics.
//!
//! Ratings and play/skip counts live beside the library, never inside audio
//! files. A separate file keeps high-frequency playback updates away from the
//! much larger `library.json` write path.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::now_millis;

pub const STATS_FILE: &str = "playback_stats.json";
pub const STATS_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackStats {
    /// 0 means unrated, otherwise 1–5.
    pub rating: u8,
    pub play_count: u32,
    pub skip_count: u32,
    pub first_played_at: u64,
    pub last_played_at: u64,
    pub last_skipped_at: u64,
    /// Per-track user calibration for lyric timestamps, in seconds.
    pub lyrics_offset: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackStats {
    pub schema: u32,
    pub tracks: HashMap<u64, TrackStats>,
}

impl Default for PlaybackStats {
    fn default() -> Self {
        Self {
            schema: STATS_SCHEMA,
            tracks: HashMap::new(),
        }
    }
}

impl PlaybackStats {
    pub fn path() -> PathBuf {
        crate::storage::data_dir().join(STATS_FILE)
    }

    pub fn get(&self, id: u64) -> TrackStats {
        self.tracks.get(&id).cloned().unwrap_or_default()
    }

    pub fn rating(&self, id: u64) -> u8 {
        self.tracks.get(&id).map_or(0, |stats| stats.rating.min(5))
    }

    pub fn rate(&mut self, id: u64, rating: u8) -> bool {
        let rating = rating.min(5);
        let entry = self.tracks.entry(id).or_default();
        if entry.rating == rating {
            return false;
        }
        entry.rating = rating;
        true
    }

    pub fn lyrics_offset(&self, id: u64) -> f32 {
        self.tracks
            .get(&id)
            .map(|stats| stats.lyrics_offset.clamp(-5.0, 5.0))
            .unwrap_or(0.0)
    }

    pub fn set_lyrics_offset(&mut self, id: u64, offset: f32) {
        self.tracks.entry(id).or_default().lyrics_offset = offset.clamp(-5.0, 5.0);
    }

    pub fn mark_play(&mut self, id: u64) {
        let now = now_millis();
        let entry = self.tracks.entry(id).or_default();
        entry.play_count = entry.play_count.saturating_add(1);
        if entry.first_played_at == 0 {
            entry.first_played_at = now;
        }
        entry.last_played_at = now;
    }

    pub fn mark_skip(&mut self, id: u64) {
        let now = now_millis();
        let entry = self.tracks.entry(id).or_default();
        entry.skip_count = entry.skip_count.saturating_add(1);
        entry.last_skipped_at = now;
    }

    pub fn load() -> Self {
        let path = Self::path();
        let Ok(bytes) = fs::read(&path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let dir = crate::storage::data_dir();
        fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
        let path = Self::path();
        let bytes = serde_json::to_vec_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, bytes).with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("replace {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratings_are_clamped_and_clearable() {
        let mut stats = PlaybackStats::default();
        assert_eq!(stats.rating(7), 0);
        assert!(stats.rate(7, 9));
        assert_eq!(stats.rating(7), 5);
        assert!(!stats.rate(7, 5));
        assert!(stats.rate(7, 0));
        assert_eq!(stats.rating(7), 0);
    }

    #[test]
    fn play_and_skip_counts_are_independent() {
        let mut stats = PlaybackStats::default();
        stats.mark_play(3);
        stats.mark_play(3);
        stats.mark_skip(3);
        let entry = stats.get(3);
        assert_eq!(entry.play_count, 2);
        assert_eq!(entry.skip_count, 1);
        assert!(entry.last_played_at > 0);
        assert!(entry.last_skipped_at > 0);
    }

    #[test]
    fn stats_roundtrip_through_json() {
        let mut stats = PlaybackStats::default();
        stats.rate(42, 4);
        stats.mark_play(42);
        let json = serde_json::to_string(&stats).unwrap();
        let decoded: PlaybackStats = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.rating(42), 4);
        assert_eq!(decoded.get(42).play_count, 1);
    }
}
