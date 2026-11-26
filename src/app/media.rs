//! media side of the app: split out of `mod.rs` to keep files small.

use super::*;

impl RPlayerApp {
    pub(crate) fn open_lyrics_match(&mut self, id: u64) {
        let Some(track) = self.library.track(id) else { return; };
        self.dialog = Dialog::LyricsMatch {
            id,
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            candidates: Vec::new(),
            searching: false,
            error: None,
        };
        self.focus_pending = true;
    }

    pub(crate) fn start_lyrics_search(&mut self, id: u64, title: String, artist: String, album: String) {
        let duration = self.library.track(id).map(|track| track.duration).unwrap_or(0.0);
        let (tx, rx) = std::sync::mpsc::channel();
        self.lyrics_candidates_rx = Some(rx);
        if let Dialog::LyricsMatch {
            searching,
            error,
            candidates,
            ..
        } = &mut self.dialog
        {
            *searching = true;
            *error = None;
            candidates.clear();
        }
        std::thread::spawn(move || {
            let result = lyrics_fetch::search_candidates(&title, &artist, &album, duration);
            let _ = tx.send(result);
        });
    }

    pub(crate) fn poll_lyrics_candidates(&mut self) {
        let Some(rx) = &self.lyrics_candidates_rx else { return; };
        match rx.try_recv() {
            Ok(result) => {
                self.lyrics_candidates_rx = None;
                if let Dialog::LyricsMatch {
                    searching,
                    error,
                    candidates,
                    ..
                } = &mut self.dialog
                {
                    *searching = false;
                    match result {
                        Ok(list) => {
                            if list.is_empty() {
                                *error = Some("没有找到候选歌词".to_string());
                            }
                            *candidates = list;
                        }
                        Err(err) => *error = Some(err.to_string()),
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.lyrics_candidates_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    pub(crate) fn apply_lyrics_candidate(&mut self, id: u64, text: &str) {
        let lyrics = crate::lyrics::parse_lrc(text);
        if lyrics.is_empty() {
            self.toast("候选歌词内容为空", ToastKind::Error);
            return;
        }
        match crate::lyrics::store_custom(id, &lyrics) {
            Ok(()) => {
                self.lyrics = Some(lyrics);
                self.lyrics_source = Some("匹配".to_string());
                self.dialog = Dialog::None;
                self.toast("歌词已应用", ToastKind::Success);
            }
            Err(err) => self.toast(format!("保存歌词失败：{err}"), ToastKind::Error),
        }
    }

     pub(crate) fn load_lyrics_for(&mut self, track: &Track, id: u64) {
        if let Some(lyrics) = crate::lyrics::load_custom(id) {
            self.lyrics = Some(lyrics);
            self.lyrics_source = Some("编辑".to_string());
        } else if let Some(lyrics) = crate::lyrics::load(&track.path) {
            self.lyrics = Some(lyrics);
            self.lyrics_source = Some("本地".to_string());
        } else if let Some(lyrics) = crate::lyrics::load_cached(id) {
            self.lyrics = Some(lyrics);
            self.lyrics_source = Some("缓存".to_string());
        } else {
            self.lyrics = None;
            self.lyrics_source = None;
        }
        self.lyrics_for = Some(id);
    }

    pub(crate) fn load_cover(&mut self, ctx: &egui::Context, id: u64) {
        if self.cover_for == Some(id) {
            return;
        }
        self.cover_for = Some(id);
        self.cover = None;
        self.cover_image = None;
        self.cover_source = None;
        self.cover_tint = None;

        let Some(track) = self.library.track(id).cloned() else {
            return;
        };
        if let Some(local) = cover::load_local(&track) {
            self.apply_cover_bytes(ctx, id, &local.bytes, local.source);
            return;
        }
        if self.library.auto_cover {
            self.request_cover(ctx, id, false);
        }
    }

    pub(crate) fn apply_cover_bytes(
        &mut self,
        ctx: &egui::Context,
        id: u64,
        bytes: &[u8],
        source: CoverSource,
    ) {
        let Ok(image) = image::load_from_memory(bytes) else {
            return;
        };
        let rgba = image.thumbnail(512, 512).to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        self.cover_tint = Some(average_color(&rgba));
        self.cover_image = Some(Arc::new(color_image.clone()));
        self.cover = Some(ctx.load_texture(
            format!("rplayer-cover-{id}"),
            color_image,
            TextureOptions::LINEAR,
        ));
        self.cover_source = Some(source);
    }

    pub(crate) fn cover_for_current(&self) -> Option<&TextureHandle> {
        self.cover.as_ref()
    }

    pub(crate) fn request_cover(&mut self, ctx: &egui::Context, id: u64, manual: bool) {
        if !manual && self.library.offline_mode {
            return;
        }
        if self.cover_fetching == Some(id) {
            return;
        }
        let Some(track) = self.library.track(id).cloned() else {
            return;
        };
        let key = cover::cache_key(&track);
        if !manual {
            if self.cover_attempted.contains(&key) || cover::has_recent_miss(&track) {
                return;
            }
        }
        self.cover_attempted.insert(key);
        self.cover_fetching = Some(id);
        self.cover_started = Some(std::time::Instant::now());
        if manual {
            self.toast("正在 MusicBrainz 搜索封面…", ToastKind::Info);
        }

        let sender = self.cover_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = crate::cover_fetch::fetch(&track);
            let _ = sender.send(CoverFetchEvent {
                track_id: id,
                manual,
                result,
            });
            ctx.request_repaint();
        });
    }

    pub(crate) fn poll_cover_fetch(&mut self, ctx: &egui::Context) {
        if self
            .cover_started
            .is_some_and(|started| started.elapsed().as_secs() > 40)
            && self.cover_fetching.is_some()
        {
            self.cover_fetching = None;
            self.cover_started = None;
        }
        loop {
            let event = match self.cover_rx.try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            };
            if self.cover_fetching == Some(event.track_id) {
                self.cover_fetching = None;
                self.cover_started = None;
            }
            match event.result {
                Ok(result) => {
                    if let Some(track) = self.library.track(event.track_id).cloned() {
                        let _ = cover::store_cache(&track, &result.bytes);
                    }
                    if self.current == Some(event.track_id) {
                        self.apply_cover_bytes(
                            ctx,
                            event.track_id,
                            &result.bytes,
                            CoverSource::Online,
                        );
                        if event.manual {
                            self.toast(
                                format!(
                                    "已匹配封面（{}）：{}",
                                    result.source, result.matched_title
                                ),
                                ToastKind::Success,
                            );
                        }
                    }
                }
                Err(error) => {
                    if let Some(track) = self.library.track(event.track_id) {
                        if matches!(error, crate::cover_fetch::FetchError::NotFound) {
                            cover::mark_miss(track);
                        }
                    }
                    if event.manual {
                        self.toast(format!("封面匹配失败：{error}"), ToastKind::Error);
                    }
                }
            }
        }
    }

    pub(crate) fn refresh_cover(&mut self, ctx: &egui::Context) {
        let Some(id) = self.current else {
            return;
        };
        let Some(track) = self.library.track(id).cloned() else {
            return;
        };
        let key = cover::cache_key(&track);
        let _ = std::fs::remove_file(storage::cover_cache_path(&key));
        let _ = std::fs::remove_file(storage::cover_miss_path(&key));
        self.cover_attempted.remove(&key);
        self.cover_for = None;
        self.cover = None;
        self.cover_image = None;
        self.cover_source = None;
        self.cover_tint = None;
        self.request_cover(ctx, id, true);
    }

    // ---------------------------------------------------------------------
    // Online lyrics (LRCLIB)
    // ---------------------------------------------------------------------

    pub(crate) fn request_lyrics(&mut self, ctx: &egui::Context, track_id: u64, manual: bool) {
        if !manual && self.library.offline_mode {
            return;
        }
        if self.lyrics_fetching == Some(track_id) {
            return;
        }
        if !manual && self.lyrics_attempted.contains(&track_id) {
            return;
        }
        let Some(track) = self.library.track(track_id).cloned() else {
            return;
        };

        self.lyrics_attempted.insert(track_id);
        self.lyrics_fetching = Some(track_id);
        if manual {
            self.toast("正在从 LRCLIB 搜索歌词…", ToastKind::Info);
        }

        let sender = self.lyrics_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = lyrics_fetch::fetch(&track);
            let _ = sender.send(LyricsFetchEvent {
                track_id,
                manual,
                result,
            });
            ctx.request_repaint();
        });
    }

    pub(crate) fn poll_lyrics_fetch(&mut self) {
        loop {
            let event = match self.lyrics_rx.try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            };
            if self.lyrics_fetching == Some(event.track_id) {
                self.lyrics_fetching = None;
            }

            match event.result {
                Ok(result) => {
                    self.last_net_failure = None;
                    let lyrics = crate::lyrics::parse_lrc(&result.text);
                    if lyrics.is_empty() {
                        if event.manual {
                            self.toast("下载到的歌词内容为空", ToastKind::Error);
                        }
                        continue;
                    }
                    let _ = crate::lyrics::store_cached(event.track_id, &result.text);
                    if self.current == Some(event.track_id) {
                        self.lyrics = Some(lyrics);
                        self.lyrics_for = Some(event.track_id);
                        self.lyrics_source = Some(result.source.to_string());
                        self.toast(
                            format!(
                                "已从 {} 下载{}",
                                result.source,
                                if result.synced { "逐行歌词" } else { "歌词" }
                            ),
                            ToastKind::Success,
                        );
                    }
                }
                Err(error) => {
                    self.last_net_failure = Some(match &error {
                        crate::lyrics_fetch::FetchError::NotFound => NetFailure::NotFound,
                        crate::lyrics_fetch::FetchError::Network(message) => {
                            let lower = message.to_lowercase();
                            if lower.contains("429")
                                || lower.contains("too many")
                                || lower.contains("rate limit")
                            {
                                NetFailure::RateLimited
                            } else {
                                NetFailure::Timeout
                            }
                        }
                        crate::lyrics_fetch::FetchError::Parse(_) => NetFailure::Timeout,
                    });
                    if event.manual {
                        self.toast(format!("歌词下载失败：{error}"), ToastKind::Error);
                    }
                }
            }
        }
    }

    pub(crate) fn download_current_lyrics(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.current {
            self.request_lyrics(ctx, id, true);
        } else {
            self.toast("先播放一首歌曲再下载歌词", ToastKind::Info);
        }
    }

    // ---------------------------------------------------------------------
    // Overlay windows (desktop lyrics + mini player)
    // ---------------------------------------------------------------------

}
