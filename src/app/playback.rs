//! playback side of the app: split out of `mod.rs` to keep files small.

use super::*;

impl RPlayerApp {
    pub(crate) fn toggle_play(&mut self, ctx: &egui::Context) {
        if let Some(audio) = &mut self.audio {
            if audio.has_source() {
                audio.toggle_pause();
                return;
            }
        }

        if let Some(current) = self.current {
            if let Some(index) = self.queue.iter().position(|id| *id == current) {
                self.queue_index = index;
                if self.start_current(ctx) {
                    return;
                }
            }
        }

        if !self.queue.is_empty() {
            self.queue_index = self.queue_index.min(self.queue.len() - 1);
            self.start_current(ctx);
            return;
        }

        let ids = self.visible_ids();
        if !ids.is_empty() {
            self.play_queue(ctx, ids, 0);
        }
    }

    pub(crate) fn play_track(&mut self, ctx: &egui::Context, id: u64) {
        let ids = self.visible_ids();
        if let Some(index) = ids.iter().position(|track| *track == id) {
            self.play_queue(ctx, ids, index);
        }
    }

    pub(crate) fn play_queue(&mut self, ctx: &egui::Context, queue: Vec<u64>, index: usize) {
        if index >= queue.len() {
            return;
        }
        self.credit_current_track(true);
        self.queue = queue;
        self.queue_index = index;
        self.shuffle_bag.clear();
        self.shuffle_exhausted = false;
        self.start_current(ctx);
    }

    pub(crate) fn start_current(&mut self, ctx: &egui::Context) -> bool {
        self.start_current_at(ctx, None)
    }

    pub(crate) fn start_current_at(&mut self, ctx: &egui::Context, start: Option<f64>) -> bool {
        let Some(id) = self.queue.get(self.queue_index).copied() else {
            return false;
        };
        let Some(track) = self.library.track(id).cloned() else {
            return false;
        };
        let Some(audio) = &mut self.audio else {
            self.toast("音频输出未初始化，无法播放", ToastKind::Error);
            return false;
        };

        let target = crate::playback::start(&track, start);
        let play_result = if target > 0.001 {
            audio.play_path_at(&track.path, target)
        } else {
            audio.play_path(&track.path)
        };
        match play_result {
            Ok(()) => {
                self.current = Some(id);
                self.play_credited = false;
                self.queued_next = None;
                self.boundary_seen = 0;
                self.library.mark_played(id);
                self.load_lyrics_for(&track, id);
                self.load_cover(ctx, id);
                self.persist();
                if self.library.auto_lyrics && self.lyrics.is_none() {
                    self.request_lyrics(ctx, id, false);
                }
                true
            }
            Err(err) => {
                self.toast(
                    format!("无法播放《{}》：{err}", track.title),
                    ToastKind::Error,
                );
                false
            }
        }
    }

    pub(crate) fn next_track(&mut self, ctx: &egui::Context, automatic: bool) {
        if self.queue.is_empty() {
            return;
        }
        self.credit_current_track(!automatic);
        if automatic && self.play_mode.is_repeat_one() {
            self.start_current(ctx);
            return;
        }
        if !automatic && self.queue.len() == 1 {
            self.seek_to(0.0);
            return;
        }

        // A missing or corrupt file must not stop the queue. Try subsequent
        // entries until one opens, or all candidates fail.
        let max_attempts = self.queue.len().max(1);
        let mut attempts = 0;
        loop {
            if attempts >= max_attempts {
                if let Some(audio) = &mut self.audio {
                    audio.stop();
                }
                return;
            }
            let Some(index) = self.choose_next() else {
                if let Some(audio) = &mut self.audio {
                    audio.stop();
                }
                return;
            };
            self.queue_index = index;
            if self.start_current(ctx) {
                return;
            }
            attempts += 1;
        }
    }

    pub(crate) fn choose_next(&mut self) -> Option<usize> {
        let len = self.queue.len();
        if len == 0 {
            return None;
        }
        if self.play_mode.is_shuffle() && len > 1 {
            self.shuffle_bag.retain(|index| *index < len);
            if self.shuffle_bag.is_empty() && self.shuffle_exhausted {
                if !self.play_mode.repeats_all() {
                    return None;
                }
                self.shuffle_exhausted = false;
            }
            if self.shuffle_bag.is_empty() {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                let mut bag: Vec<usize> = (0..len)
                    .filter(|index| *index != self.queue_index)
                    .collect();
                bag.shuffle(&mut rng);
                self.shuffle_bag = bag;
                self.shuffle_exhausted = false;
            }
            let index = self.shuffle_bag.pop()?;
            if self.shuffle_bag.is_empty() {
                self.shuffle_exhausted = true;
            }
            return Some(index);
        }
        if self.queue_index + 1 < len {
            Some(self.queue_index + 1)
        } else if self.play_mode.repeats_all() {
            Some(0)
        } else {
            None
        }
    }

    pub(crate) fn append_to_queue(&mut self, ids: &[u64]) -> usize {
        let mut added = 0;
        for id in ids {
            if !self.queue.contains(id) {
                self.queue.push(*id);
                added += 1;
            }
        }
        if added > 0 {
            self.shuffle_bag.clear();
            self.shuffle_exhausted = false;
            self.last_session_save =
                Instant::now() - Duration::from_secs(6);
        }
        added
    }

    pub(crate) fn reorder_queue(&mut self, from: usize, to: usize) {
        if from == to || from >= self.queue.len() {
            return;
        }
        let item = self.queue.remove(from);
        let target = to.min(self.queue.len());
        self.queue.insert(target, item);
        self.shuffle_bag.clear();
        self.shuffle_exhausted = false;
        if let Some(current) = self.current {
            if let Some(index) = self.queue.iter().position(|id| *id == current) {
                self.queue_index = index;
            }
        }
        if self.queued_next.is_some() {
            if let Some(track) = self.current.and_then(|id| self.library.track(id)).cloned() {
                if let Some(audio) = &mut self.audio {
                    let paused = audio.is_paused();
                    let position = audio.position();
                    if audio.play_path_at(&track.path, position).is_ok() && paused {
                        audio.set_paused(true);
                    }
                }
            }
            self.queued_next = None;
            self.boundary_seen = 0;
        }
        self.last_session_save = std::time::Instant::now() - std::time::Duration::from_secs(6);
    }

    pub(crate) fn clear_queue(&mut self) {
        self.credit_current_track(true);
        if let Some(audio) = &mut self.audio {
            audio.stop();
        }
        self.queue.clear();
        self.queue_index = 0;
        self.shuffle_bag.clear();
        self.shuffle_exhausted = false;
        self.current = None;
        self.queued_next = None;
        self.boundary_seen = 0;
        self.cover = None;
        self.cover_for = None;
        self.cover_image = None;
        self.cover_source = None;
        self.cover_tint = None;
        self.lyrics = None;
        self.lyrics_for = None;
        self.lyrics_source = None;
        self.expanded = false;
        self.queue_open = false;
    }

    pub(crate) fn previous_track(&mut self, ctx: &egui::Context) {
        if self.queue.is_empty() {
            return;
        }
        let position = self
            .audio
            .as_ref()
            .map(AudioEngine::position)
            .unwrap_or(0.0);
        if position > 3.0 {
            if let Some(audio) = &mut self.audio {
                audio.seek(0.0);
            }
            return;
        }

        self.credit_current_track(true);
        let len = self.queue.len();
        let index = if self.play_mode.is_shuffle() && len > 1 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let mut index = rng.gen_range(0..len);
            if index == self.queue_index {
                index = (index + 1) % len;
            }
            index
        } else if self.queue_index > 0 {
            self.queue_index - 1
        } else {
            len - 1
        };
        self.queue_index = index;
        self.start_current(ctx);
    }

    pub(crate) fn seek_to(&mut self, seconds: f64) {
        let Some(track) = self.current.and_then(|id| self.library.track(id)).cloned() else {
            return;
        };
        let target = crate::playback::absolute(&track, seconds);
        if self.queued_next.is_some() {
            // A gapless source is already appended; reopening is the only way
            // to guarantee that a backward seek does not hit the next track.
            if let Some(audio) = &mut self.audio {
                let paused = audio.is_paused();
                if audio.play_path_at(&track.path, target).is_ok() && paused {
                    audio.set_paused(true);
                }
            }
            self.queued_next = None;
            self.boundary_seen = 0;
            return;
        }
        if let Some(audio) = &mut self.audio {
            audio.seek(target);
        }
    }

    pub(crate) fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.library.volume = self.volume;
        if let Some(audio) = &mut self.audio {
            audio.set_volume(self.volume);
        }
    }

    pub(crate) fn toggle_mute(&mut self) {
        if self.volume > 0.005 {
            self.volume_before_mute = self.volume;
            self.set_volume(0.0);
        } else {
            let restore = if self.volume_before_mute > 0.005 {
                self.volume_before_mute
            } else {
                0.72
            };
            self.set_volume(restore);
        }
        self.persist();
    }

    pub(crate) fn cycle_play_mode(&mut self) {
        self.play_mode = self.play_mode.next();
        self.shuffle_bag.clear();
        self.shuffle_exhausted = false;
    }

}
