//! Rating and playback-statistic glue for the UI and playback lifecycle.

use std::time::{Duration, Instant};

use super::{RPlayerApp, ToastKind};

impl RPlayerApp {
    pub(super) fn rating(&self, id: u64) -> u8 {
        self.stats.rating(id)
    }

    pub(super) fn lyrics_offset(&self) -> f32 {
        self.current
            .map(|id| self.stats.lyrics_offset(id))
            .unwrap_or(0.0)
    }

    pub(super) fn adjust_lyrics_offset(&mut self, delta: f32) {
        if let Some(id) = self.current {
            let value = (self.stats.lyrics_offset(id) + delta).clamp(-5.0, 5.0);
            self.stats.set_lyrics_offset(id, value);
            self.stats_dirty = true;
        }
    }

    pub(super) fn set_rating(&mut self, id: u64, rating: u8) {
        if self.stats.rate(id, rating) {
            self.stats_dirty = true;
        }
    }

    /// Credit a track that reached the end of a gapless boundary.
    pub(super) fn credit_completed_track(&mut self) {
        if let Some(id) = self.current {
            self.stats.mark_play(id);
            self.stats_dirty = true;
        }
        self.play_credited = false;
    }

    /// Credit the track that is about to be replaced.
    pub(super) fn credit_current_track(&mut self, skipped: bool) {
        let Some(id) = self.current else {
            self.play_credited = false;
            return;
        };
        if !self.play_credited {
            let position = self.current_position();
            let threshold = self.play_threshold();
            if position >= threshold {
                self.stats.mark_play(id);
                self.stats_dirty = true;
                self.play_credited = true;
            } else if skipped && (self.is_playing() || position > 0.5) {
                self.stats.mark_skip(id);
                self.stats_dirty = true;
            }
        }
        self.play_credited = false;
    }

    pub(super) fn poll_stats(&mut self) {
        if self.is_playing() {
            if let Some(id) = self.current {
                if !self.play_credited && self.current_position() >= self.play_threshold() {
                    self.stats.mark_play(id);
                    self.stats_dirty = true;
                    self.play_credited = true;
                }
            }
        }

        if self.stats_dirty && self.last_stats_save.elapsed() >= Duration::from_secs(5) {
            self.save_stats();
        }
    }

    pub(super) fn save_stats(&mut self) {
        if !self.stats_dirty {
            return;
        }
        match self.stats.save() {
            Ok(()) => {
                self.stats_dirty = false;
                self.last_stats_save = Instant::now();
            }
            Err(err) => {
                self.last_stats_save = Instant::now();
                self.toast(format!("保存评分/播放统计失败：{err:#}"), ToastKind::Error);
            }
        }
    }

    fn play_threshold(&self) -> f64 {
        (self.current_duration() * 0.5).clamp(1.0, 30.0)
    }
}
