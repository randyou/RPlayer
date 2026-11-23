//! Restore the last playback session without touching the music library.

use eframe::egui;

use super::{RPlayerApp, View};
use crate::session::Session;

impl RPlayerApp {
    pub(super) fn restore_session(&mut self, session: Session) {
        let mut queue: Vec<u64> = session
            .queue
            .into_iter()
            .filter(|id| self.library.track(*id).is_some())
            .collect();
        let current = session
            .current
            .filter(|id| self.library.track(*id).is_some());
        if let Some(id) = current {
            if queue.is_empty() {
                queue.push(id);
            }
            let mut index = session.queue_index.min(queue.len().saturating_sub(1));
            if queue.get(index) != Some(&id) {
                if let Some(found) = queue.iter().position(|candidate| *candidate == id) {
                    index = found;
                } else {
                    queue.insert(0, id);
                    index = 0;
                }
            }
            self.queue_index = index;
            self.current = Some(id);
            self.pending_resume = Some(session.position.max(0.0));
        } else {
            self.queue.clear();
            self.queue_index = 0;
        }
        self.queue = queue;
        self.restore_view(&session.view);
        if let Some(volume) = session.volume {
            if volume.is_finite() {
                self.set_volume(volume.clamp(0.0, 1.0));
            }
        }
    }

    pub(super) fn restore_pending_playback(&mut self, ctx: &egui::Context) {
        let Some(position) = self.pending_resume else {
            return;
        };
        if self.current.is_none() {
            self.pending_resume = None;
            return;
        }
        if self.audio.is_none() {
            return;
        }
        self.pending_resume = None;
        if self.start_current_at(ctx, Some(position)) {
            if let Some(audio) = &mut self.audio {
                audio.set_paused(true);
            }
            self.expanded = false;
        } else {
            self.current = None;
        }
    }

    pub(super) fn tick_session(&mut self) {
        if self.last_session_save.elapsed().as_secs() < 5 {
            return;
        }
        self.last_session_save = std::time::Instant::now();
        self.save_session();
    }

    pub(super) fn save_session(&self) {
        let session = Session {
            schema: 1,
            current: self.current,
            position: self
                .audio
                .as_ref()
                .map(|audio| audio.position())
                .unwrap_or(0.0),
            queue: self.queue.clone(),
            queue_index: self.queue_index,
            view: self.view_key(),
            volume: Some(self.volume),
            saved_at: crate::model::now_millis(),
        };
        let _ = crate::session::save(&session);
    }

    fn view_key(&self) -> String {
        match self.view {
            View::Library => "library".to_string(),
            View::Albums => "albums".to_string(),
            View::Artists => "artists".to_string(),
            View::Favorites => "favorites".to_string(),
            View::Recent => "recent".to_string(),
            View::Folders => "folders".to_string(),
            View::Playlist(id) => format!("playlist:{id}"),
        }
    }

    fn restore_view(&mut self, key: &str) {
        self.view = match key {
            "albums" => View::Albums,
            "artists" => View::Artists,
            "favorites" => View::Favorites,
            "recent" => View::Recent,
            "folders" => View::Folders,
            value if value.starts_with("playlist:") => {
                let id = value["playlist:".len()..].parse::<u64>().ok();
                match id {
                    Some(id) if self.library.playlists.iter().any(|list| list.id == id) => {
                        View::Playlist(id)
                    }
                    _ => View::Library,
                }
            }
            _ => View::Library,
        };
    }
}
