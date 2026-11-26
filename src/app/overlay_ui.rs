//! overlay_ui side of the app: split out of `mod.rs` to keep files small.

use super::*;

impl RPlayerApp {
    pub(crate) fn sync_overlay(&mut self) -> bool {
        let track = self.current.and_then(|id| self.library.track(id));
        let position = self.current_position();
        let favorite = self
            .current
            .map(|id| self.library.is_favorite(id))
            .unwrap_or(false);

        let mut state = OverlayState {
            has_track: track.is_some(),
            title: track.map(|track| track.title.clone()).unwrap_or_default(),
            artist: track.map(|track| track.artist.clone()).unwrap_or_default(),
            track_number: track.and_then(|track| track.track_number),
            position,
            duration: track.map(|track| track.duration).unwrap_or_default(),
            playing: self.is_playing(),
            favorite,
            lyrics_opacity: self.library.desktop_lyrics_opacity,
            lyrics_text_opacity: self.library.desktop_lyrics_text_opacity,
            lyrics_font: if self.library.desktop_lyrics_font > 0.0 {
                self.library.desktop_lyrics_font
            } else {
                25.0
            },
            lyrics_locked: {
                crate::platform::set_lyrics_locked(self.library.desktop_lyrics_locked);
                self.library.desktop_lyrics_locked
            },
            mini_style: if self.library.mini_style == "compact" {
                "compact".to_string()
            } else {
                "classic".to_string()
            },
            mini_on_top: self.library.mini_on_top,
            mini_snap: self.library.mini_snap,
            desktop_lyrics_open: self.desktop_lyrics_open,
            mini_open: self.mini_open,
            ..OverlayState::default()
        };

        if let Some(track) = track {
            if self.lyrics_for == Some(track.id) {
                if let Some(lyrics) = &self.lyrics {
                    state.has_lyrics = !lyrics.lines.is_empty();
                    if state.has_lyrics {
                        let lyric_position =
                            (position - self.lyrics_offset() as f64).max(0.0);
                        let active = lyrics.active_index(lyric_position).unwrap_or(0);
                        state.lyric_previous = active
                            .checked_sub(1)
                            .and_then(|index| lyrics.lines.get(index))
                            .map(|line| line.text.clone())
                            .unwrap_or_default();
                        state.lyric_current = lyrics
                            .lines
                            .get(active)
                            .map(|line| line.text.clone())
                            .unwrap_or_default();
                        state.lyric_next = lyrics
                            .lines
                            .get(active + 1)
                            .map(|line| line.text.clone())
                            .unwrap_or_default();
                    }
                }
            }
            if self.cover_for == Some(track.id) {
                state.cover_id = Some(track.id);
                state.cover = self.cover_image.clone();
            }
        }

        if let Ok(mut shared) = self.overlay_state.lock() {
            *shared = state.clone();
        }

        let key = overlay_content_key(&state);
        if key == self.overlay_sync_key {
            false
        } else {
            self.overlay_sync_key = key;
            true
        }
    }

    pub(crate) fn handle_overlay_commands(&mut self, ctx: &egui::Context) {
        let commands = self
            .overlay_commands
            .lock()
            .map(|mut queue| std::mem::take(&mut *queue))
            .unwrap_or_default();

        for command in commands {
            match command {
                OverlayCommand::TogglePlay => self.toggle_play(ctx),
                OverlayCommand::Previous => self.previous_track(ctx),
                OverlayCommand::Next => self.next_track(ctx, false),
                OverlayCommand::ToggleFavorite => {
                    if let Some(id) = self.current {
                        self.toggle_favorite(id);
                    }
                }
                OverlayCommand::Seek(seconds) => self.seek_to(seconds),
                OverlayCommand::SetLyricsOpacity(opacity) => {
                    self.library.desktop_lyrics_opacity = opacity.clamp(0.0, 1.0);
                    self.prefs_dirty = true;
                }
                OverlayCommand::SetLyricsTextOpacity(opacity) => {
                    self.library.desktop_lyrics_text_opacity = opacity.clamp(0.0, 1.0);
                    self.prefs_dirty = true;
                }
                OverlayCommand::SetLyricsLocked(locked) => {
                    self.library.desktop_lyrics_locked = locked;
                    self.prefs_dirty = true;
                }
                OverlayCommand::SetLyricsPosition(x, y) => {
                    self.library.desktop_lyrics_pos = Some((x, y));
                    self.prefs_dirty = true;
                }
                OverlayCommand::ToggleLyrics => {
                    self.toggle_desktop_lyrics(ctx);
                }
                OverlayCommand::SetMiniOnTop(on_top) => {
                    self.library.mini_on_top = on_top;
                    self.prefs_dirty = true;
                    ctx.send_viewport_cmd_to(
                        overlay::mini_viewport(),
                        ViewportCommand::WindowLevel(if on_top {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        }),
                    );
                }
                OverlayCommand::SetMiniPosition(x, y) => {
                    self.library.mini_pos = Some((x, y));
                    self.prefs_dirty = true;
                }
                OverlayCommand::CloseLyrics => {
                    self.desktop_lyrics_open = false;
                    self.library.desktop_lyrics = false;
                    self.lyrics_viewport_active = false;
                    crate::platform::hide_lyrics_panel();
                    ctx.send_viewport_cmd_to(
                        overlay::lyrics_viewport(),
                        ViewportCommand::Close,
                    );
                    self.persist();
                }
                OverlayCommand::CloseMini => {
                    self.mini_open = false;
                    self.mini_viewport_active = false;
                    ctx.send_viewport_cmd_to(overlay::mini_viewport(), ViewportCommand::Close);
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        ViewportCommand::Minimized(false),
                    );
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, ViewportCommand::Focus);
                }
                OverlayCommand::OpenLyricsSettings => {
                    self.settings_open = true;
                    self.settings_tab = 1;
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        ViewportCommand::Minimized(false),
                    );
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, ViewportCommand::Focus);
                }
                OverlayCommand::FetchLyrics => {
                    if let Some(id) = self.current {
                        self.request_lyrics(ctx, id, true);
                    }
                }
                OverlayCommand::OpenMain => {
                    self.mini_open = false;
                    self.mini_viewport_active = false;
                    ctx.send_viewport_cmd_to(overlay::mini_viewport(), ViewportCommand::Close);
                    if self.current.is_some() {
                        self.expanded = true;
                    }
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        ViewportCommand::Minimized(false),
                    );
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, ViewportCommand::Focus);
                }
            }
        }
    }

    pub(crate) fn show_overlays(&mut self, ctx: &egui::Context) {
        let monitor = ctx
            .input(|input| input.viewport().monitor_size)
            .unwrap_or(egui::vec2(1920.0, 1080.0));

        if self.desktop_lyrics_open {
            let position = if self.lyrics_placed {
                None
            } else if let Some((x, y)) = self.library.desktop_lyrics_pos {
                Some(Pos2::new(x, y))
            } else {
                Some(Pos2::new(
                    (monitor.x - 860.0).max(0.0) * 0.5,
                    (monitor.y - 214.0).max(40.0),
                ))
            };
            overlay::show_lyrics(
                ctx,
                self.overlay_state.clone(),
                self.overlay_commands.clone(),
                position,
            );
            self.lyrics_placed = true;
            self.lyrics_viewport_active = true;
            // A window that never activates can still be handed the foreground
            // when it is created; while it holds it, dragging the player needs
            // a throwaway click first (v2.1.0 issue #2).
            crate::platform::release_overlay_foreground("RPlayer 桌面歌词");
            crate::platform::enable_lyrics_native_drag("RPlayer 桌面歌词");
            if !self.lyrics_region {
                // DWM adds a dark 1px frame around borderless overlays on
                // Windows 11; remove it (and the rounded corner preference).
                crate::platform::focus_main_window();
                self.lyrics_region = true;
            }
            // Re-applied every frame: DWM reinstates the frame whenever the
            // overlay is re-composited (focus handback, capsule toggle), which
            // showed up as a title bar flickering over the strip.
            crate::platform::set_window_borderless("RPlayer 桌面歌词");
        } else if self.lyrics_viewport_active {
            self.lyrics_viewport_active = false;
            crate::platform::hide_lyrics_panel();
            ctx.send_viewport_cmd_to(overlay::lyrics_viewport(), ViewportCommand::Close);
        }

        if self.mini_open {
            let position = if self.mini_placed {
                None
            } else if let Some((x, y)) = self.library.mini_pos {
                Some(Pos2::new(
                    x.clamp(0.0, (monitor.x - 410.0).max(0.0)),
                    y.clamp(0.0, (monitor.y - 146.0).max(0.0)),
                ))
            } else {
                Some(Pos2::new(
                    (monitor.x - 446.0).max(0.0),
                    (monitor.y - 190.0).max(40.0),
                ))
            };
            overlay::show_mini(
                ctx,
                self.overlay_state.clone(),
                self.overlay_commands.clone(),
                self.mini_cache.clone(),
                position,
                self.library.mini_on_top,
            );
            self.mini_placed = true;
            self.mini_viewport_active = true;
            if !self.mini_region {
                self.mini_region = crate::platform::apply_round_window(
                    "RPlayer 迷你播放器",
                    20,
                );
            }
        } else if self.mini_viewport_active {
            self.mini_viewport_active = false;
            ctx.send_viewport_cmd_to(overlay::mini_viewport(), ViewportCommand::Close);
        }
    }

    // ---------------------------------------------------------------------
    // Library mutations
    // ---------------------------------------------------------------------

}
