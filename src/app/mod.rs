//! Application state, playback orchestration and the root layout.

mod dialogs;
mod library_view;
mod now_playing;
mod player_bar;
mod sidebar;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui;
use egui::{
    Align2, Color32, Frame, Id, Key, Margin, RichText, Stroke, TextureHandle, TextureOptions, Vec2,
    ViewportCommand,
};

use crate::audio::AudioEngine;
use crate::model::{self, Library, SortField, SortState, Track};
use crate::scanner::{self, ScanEvent};
use crate::storage;
use crate::theme;
use crate::ui::icons::{self, Icon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum View {
    Library,
    Favorites,
    Recent,
    Folders,
    Playlist(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::One => Icon::RepeatOne,
            _ => Icon::Repeat,
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            Self::Off => "循环关闭",
            Self::All => "列表循环",
            Self::One => "单曲循环",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) enum Dialog {
    #[default]
    None,
    NewPlaylist {
        name: String,
    },
    RenamePlaylist {
        id: u64,
        name: String,
    },
    ConfirmDeletePlaylist {
        id: u64,
    },
    ConfirmRemoveTrack {
        id: u64,
    },
    Folders,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToastKind {
    Info,
    Success,
    Error,
}

pub(super) struct Toast {
    text: String,
    kind: ToastKind,
    created: Instant,
}

pub struct RPlayerApp {
    library: Library,
    audio: Option<AudioEngine>,
    view: View,
    search: String,
    sort: SortState,
    selected: Option<u64>,
    current: Option<u64>,
    queue: Vec<u64>,
    queue_index: usize,
    shuffle: bool,
    repeat: RepeatMode,
    volume: f32,
    volume_before_mute: f32,
    last_saved_volume: f32,
    cover: Option<TextureHandle>,
    cover_for: Option<u64>,
    lyrics: Option<crate::lyrics::Lyrics>,
    lyrics_for: Option<u64>,
    expanded: bool,
    batch_mode: bool,
    checked: HashSet<u64>,
    scanning: bool,
    scan_rx: Option<Receiver<ScanEvent>>,
    scan_total: usize,
    scan_done: usize,
    scan_seen: HashSet<u64>,
    scan_errors: Vec<String>,
    dialog: Dialog,
    focus_pending: bool,
    focus_search: bool,
    toast: Option<Toast>,
    queue_open: bool,
    last_window_title: String,
    closing_saved: bool,
    dark_titlebar_applied: bool,
}

impl RPlayerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        cc.egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));

        let library = storage::load_library();
        let volume = library.volume.clamp(0.0, 1.0);
        let (audio, audio_error) = match AudioEngine::new(volume) {
            Ok(engine) => (Some(engine), None),
            Err(err) => (None, Some(format!("{err:#}"))),
        };

        let mut app = Self {
            volume,
            last_saved_volume: volume,
            volume_before_mute: if volume > 0.01 { volume } else { 0.72 },
            library,
            audio,
            view: View::Library,
            search: String::new(),
            sort: SortState::default(),
            selected: None,
            current: None,
            queue: Vec::new(),
            queue_index: 0,
            shuffle: false,
            repeat: RepeatMode::Off,
            cover: None,
            cover_for: None,
            lyrics: None,
            lyrics_for: None,
            expanded: false,
            batch_mode: false,
            checked: HashSet::new(),
            scanning: false,
            scan_rx: None,
            scan_total: 0,
            scan_done: 0,
            scan_seen: HashSet::new(),
            scan_errors: Vec::new(),
            dialog: Dialog::None,
            focus_pending: false,
            focus_search: false,
            toast: None,
            queue_open: false,
            last_window_title: "RPlayer".to_string(),
            closing_saved: false,
            dark_titlebar_applied: false,
        };

        if let Some(err) = audio_error {
            app.toast(format!("音频输出不可用：{err}"), ToastKind::Error);
        }
        if !app.library.folders.is_empty() && app.library.tracks.is_empty() {
            app.start_scan();
        }
        app
    }

    // ---------------------------------------------------------------------
    // Scan pipeline
    // ---------------------------------------------------------------------

    fn start_scan(&mut self) {
        if self.library.folders.is_empty() {
            self.toast("请先添加一个音乐文件夹", ToastKind::Info);
            return;
        }
        self.scanning = true;
        self.scan_total = 0;
        self.scan_done = 0;
        self.scan_seen.clear();
        self.scan_errors.clear();
        self.scan_rx = Some(scanner::start_scan(self.library.folders.clone()));
    }

    fn poll_scan(&mut self) {
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(rx) = &self.scan_rx {
            loop {
                match rx.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        let mut finished = false;
        for event in events {
            match event {
                ScanEvent::Started { total } => {
                    self.scan_total = total;
                    self.scan_done = 0;
                }
                ScanEvent::Track(track) => {
                    self.scan_seen.insert(track.id);
                    if let Some(existing) =
                        self.library.tracks.iter_mut().find(|t| t.id == track.id)
                    {
                        // Keep the original "added" timestamp across rescans.
                        let added_at = existing.added_at;
                        *existing = *track;
                        existing.added_at = added_at;
                    } else {
                        self.library.tracks.push(*track);
                    }
                }
                ScanEvent::Progress { scanned, total } => {
                    self.scan_done = scanned;
                    self.scan_total = total;
                }
                ScanEvent::Finished { errors } => {
                    self.scan_errors = errors;
                    finished = true;
                }
            }
        }

        if finished || disconnected {
            if !finished {
                self.scan_errors.push("扫描线程意外结束".to_string());
            }
            self.scan_rx = None;
            self.finish_scan();
        }
    }

    fn finish_scan(&mut self) {
        self.scanning = false;
        let seen = &self.scan_seen;
        let before = self.library.tracks.len();
        self.library.tracks.retain(|track| seen.contains(&track.id));
        let removed = before.saturating_sub(self.library.tracks.len());
        self.library.prune_playlists();

        let current_missing = self
            .current
            .map(|id| self.library.track(id).is_none())
            .unwrap_or(false);
        if current_missing {
            if let Some(audio) = &mut self.audio {
                audio.stop();
            }
            self.current = None;
            self.cover = None;
            self.cover_for = None;
        }

        let library = &self.library;
        self.queue.retain(|id| library.track(*id).is_some());
        if self.queue.is_empty() {
            self.queue_index = 0;
        } else if self.queue_index >= self.queue.len() {
            self.queue_index = self.queue.len() - 1;
        }

        let count = self.library.tracks.len();
        let errors = self.scan_errors.len();
        self.persist();

        let mut message = format!("扫描完成 · 共 {count} 首歌曲");
        if removed > 0 {
            message.push_str(&format!("，移除 {removed} 首失效记录"));
        }
        if errors > 0 {
            message.push_str(&format!("，{errors} 个文件无法读取"));
        }
        self.toast(message, ToastKind::Success);
    }

    fn add_folder(&mut self, folder: PathBuf) {
        if self
            .library
            .folders
            .iter()
            .any(|existing| model::same_path(existing, &folder))
        {
            self.toast("这个文件夹已经在音乐库中", ToastKind::Info);
            return;
        }
        self.library.folders.push(folder);
        self.persist();
        self.start_scan();
    }

    fn open_folder_picker(&mut self) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("选择音乐文件夹")
            .pick_folder()
        {
            self.add_folder(folder);
        }
    }

    fn remove_folder(&mut self, folder: &Path) {
        let Some(index) = self
            .library
            .folders
            .iter()
            .position(|f| model::same_path(f, folder))
        else {
            return;
        };
        let removed_folder = self.library.folders.remove(index);

        self.library
            .tracks
            .retain(|track| !path_is_under(&removed_folder, &track.path));
        self.library.prune_playlists();
        self.prune_transport();

        self.persist();
        if !self.library.folders.is_empty() {
            self.start_scan();
        } else {
            self.scanning = false;
            self.scan_rx = None;
            self.toast("已移除全部音乐文件夹", ToastKind::Info);
        }
    }

    fn prune_transport(&mut self) {
        let current_missing = self
            .current
            .map(|id| self.library.track(id).is_none())
            .unwrap_or(false);
        if current_missing {
            if let Some(audio) = &mut self.audio {
                audio.stop();
            }
            self.current = None;
            self.cover = None;
            self.cover_for = None;
        }
        if self
            .selected
            .map(|id| self.library.track(id).is_none())
            .unwrap_or(false)
        {
            self.selected = None;
        }

        let library = &self.library;
        self.queue.retain(|id| library.track(*id).is_some());
        if self.queue.is_empty() {
            self.queue_index = 0;
        } else if self.queue_index >= self.queue.len() {
            self.queue_index = self.queue.len() - 1;
        }
    }

    fn ingest_dropped(&mut self, paths: Vec<PathBuf>) {
        let mut folders = Vec::new();
        for path in paths {
            if path.is_dir() {
                folders.push(path);
            } else if model::is_audio_file(&path) {
                if let Some(parent) = path.parent() {
                    folders.push(parent.to_path_buf());
                }
            }
        }

        let mut added = 0usize;
        for folder in folders {
            if self
                .library
                .folders
                .iter()
                .any(|existing| model::same_path(existing, &folder))
            {
                continue;
            }
            self.library.folders.push(folder);
            added += 1;
        }

        if added == 0 {
            self.toast("没有发现新的音乐文件夹", ToastKind::Info);
            return;
        }
        self.persist();
        self.start_scan();
        self.toast(
            format!("已添加 {added} 个文件夹，正在扫描音乐…"),
            ToastKind::Info,
        );
    }

    // ---------------------------------------------------------------------
    // Playback
    // ---------------------------------------------------------------------

    fn toggle_play(&mut self, ctx: &egui::Context) {
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

    fn play_track(&mut self, ctx: &egui::Context, id: u64) {
        let ids = self.visible_ids();
        if let Some(index) = ids.iter().position(|track| *track == id) {
            self.play_queue(ctx, ids, index);
        }
    }

    fn play_queue(&mut self, ctx: &egui::Context, queue: Vec<u64>, index: usize) {
        if index >= queue.len() {
            return;
        }
        self.queue = queue;
        self.queue_index = index;
        self.start_current(ctx);
    }

    fn start_current(&mut self, ctx: &egui::Context) -> bool {
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

        match audio.play_path(&track.path) {
            Ok(()) => {
                self.current = Some(id);
                self.library.mark_played(id);
                self.lyrics = crate::lyrics::load(&track.path);
                self.lyrics_for = Some(id);
                self.load_cover(ctx, id);
                self.persist();
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

    fn next_track(&mut self, ctx: &egui::Context, automatic: bool) {
        if self.queue.is_empty() {
            return;
        }
        if automatic && self.repeat == RepeatMode::One {
            self.start_current(ctx);
            return;
        }
        if !automatic && self.queue.len() == 1 {
            if let Some(audio) = &mut self.audio {
                audio.seek(0.0);
            }
            return;
        }

        match self.choose_next() {
            Some(index) => {
                self.queue_index = index;
                if !self.start_current(ctx) {
                    if let Some(audio) = &mut self.audio {
                        audio.stop();
                    }
                }
            }
            None => {
                if let Some(audio) = &mut self.audio {
                    audio.stop();
                }
            }
        }
    }

    fn choose_next(&self) -> Option<usize> {
        let len = self.queue.len();
        if len == 0 {
            return None;
        }
        if self.shuffle && len > 1 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let mut index = rng.gen_range(0..len);
            if index == self.queue_index {
                index = (index + 1 + rng.gen_range(0..len - 1)) % len;
            }
            return Some(index);
        }
        if self.queue_index + 1 < len {
            Some(self.queue_index + 1)
        } else if self.repeat == RepeatMode::All {
            Some(0)
        } else {
            None
        }
    }

    fn previous_track(&mut self, ctx: &egui::Context) {
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

        let len = self.queue.len();
        let index = if self.shuffle && len > 1 {
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

    fn seek_to(&mut self, seconds: f64) {
        if let Some(audio) = &mut self.audio {
            audio.seek(seconds);
        }
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.library.volume = self.volume;
        if let Some(audio) = &mut self.audio {
            audio.set_volume(self.volume);
        }
    }

    fn toggle_mute(&mut self) {
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

    fn cycle_repeat(&mut self) {
        self.repeat = self.repeat.next();
    }

    fn load_cover(&mut self, ctx: &egui::Context, id: u64) {
        if self.cover_for == Some(id) {
            return;
        }
        self.cover_for = Some(id);
        self.cover = None;

        let Some(track) = self.library.track(id) else {
            return;
        };
        let Some(bytes) = model::read_cover(&track.path) else {
            return;
        };
        let Ok(image) = image::load_from_memory(&bytes) else {
            return;
        };
        let rgba = image.thumbnail(512, 512).to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        self.cover = Some(ctx.load_texture(
            format!("rplayer-cover-{id}"),
            color_image,
            TextureOptions::LINEAR,
        ));
    }

    fn cover_for_current(&self) -> Option<&TextureHandle> {
        self.cover.as_ref()
    }

    // ---------------------------------------------------------------------
    // Library mutations
    // ---------------------------------------------------------------------

    fn add_track_to_playlist(&mut self, playlist_id: u64, track_id: u64) {
        let name = self
            .library
            .playlist(playlist_id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        if self.library.add_to_playlist(playlist_id, track_id) {
            self.persist();
            self.toast(format!("已添加到「{name}」"), ToastKind::Success);
        } else {
            self.toast("这首歌已经在该播放列表中", ToastKind::Info);
        }
    }

    fn remove_track_from_view_playlist(&mut self, track_id: u64) {
        let View::Playlist(playlist_id) = self.view else {
            return;
        };
        self.library.remove_from_playlist(playlist_id, track_id);
        self.persist();
    }

    fn create_playlist(&mut self, name: String) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let id = self.library.create_playlist(name.to_string());
        self.view = View::Playlist(id);
        self.selected = None;
        self.persist();
        self.toast(format!("已创建播放列表「{name}」"), ToastKind::Success);
    }

    fn rename_playlist(&mut self, id: u64, name: String) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if let Some(playlist) = self.library.playlist_mut(id) {
            playlist.name = name.to_string();
            self.persist();
        }
    }

    fn delete_playlist(&mut self, id: u64) {
        let name = self
            .library
            .playlist(id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        self.library.remove_playlist(id);
        if self.view == View::Playlist(id) {
            self.view = View::Library;
        }
        self.persist();
        self.toast(format!("已删除播放列表「{name}」"), ToastKind::Info);
    }

    fn remove_track(&mut self, id: u64) {
        let removed = self
            .library
            .tracks
            .iter()
            .find(|track| track.id == id)
            .map(|track| track.title.clone());
        self.library.tracks.retain(|track| track.id != id);
        self.library.prune_playlists();
        self.prune_transport();
        if self.selected == Some(id) {
            self.selected = None;
        }
        self.persist();
        if let Some(title) = removed {
            self.toast(format!("已从音乐库移除《{title}》"), ToastKind::Info);
            // Forget cached art if it belonged to this track.
            if self.cover_for == Some(id) {
                self.cover = None;
                self.cover_for = None;
            }
        }
    }

    fn switch_view(&mut self, view: View) {
        if self.view == view {
            return;
        }
        self.view = view;
        self.selected = None;
        self.batch_mode = false;
        self.checked.clear();
    }

    fn toggle_favorite(&mut self, id: u64) {
        let favorite = self.library.toggle_favorite(id);
        self.persist();
        self.toast(
            if favorite {
                "已添加到我的收藏"
            } else {
                "已取消收藏"
            },
            ToastKind::Info,
        );
    }

    fn toggle_checked(&mut self, id: u64) {
        if !self.checked.remove(&id) {
            self.checked.insert(id);
        }
    }

    fn set_all_checked(&mut self, ids: &[u64], checked: bool) {
        if checked {
            self.checked.extend(ids.iter().copied());
        } else {
            for id in ids {
                self.checked.remove(id);
            }
        }
    }

    fn batch_add_to_playlist(&mut self, playlist_id: u64) {
        let ids: Vec<u64> = self.checked.iter().copied().collect();
        let mut added = 0usize;
        for id in ids {
            if self.library.add_to_playlist(playlist_id, id) {
                added += 1;
            }
        }
        let name = self
            .library
            .playlist(playlist_id)
            .map(|playlist| playlist.name.clone())
            .unwrap_or_default();
        self.persist();
        self.toast(
            format!("已添加 {added} 首歌曲到「{name}」"),
            ToastKind::Success,
        );
    }

    fn batch_remove_from_current(&mut self) {
        let ids: Vec<u64> = self.checked.iter().copied().collect();
        match self.view {
            View::Playlist(playlist_id) => {
                for id in &ids {
                    self.library.remove_from_playlist(playlist_id, *id);
                }
                self.persist();
                self.toast(
                    format!("已从当前播放列表移除 {} 首歌曲", ids.len()),
                    ToastKind::Info,
                );
            }
            View::Favorites => {
                let checked = self.checked.clone();
                self.library.favorites.retain(|id| !checked.contains(id));
                self.persist();
                self.toast(
                    format!("已取消收藏 {} 首歌曲", ids.len()),
                    ToastKind::Info,
                );
            }
            _ => {}
        }
        self.checked.clear();
    }

    fn reveal_in_file_manager(&self, path: &Path) {
        #[cfg(target_os = "windows")]
        {
            let arg = format!("/select,{}", path.display());
            let _ = std::process::Command::new("explorer").arg(arg).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-R")
                .arg(path)
                .spawn();
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            if let Some(parent) = path.parent() {
                let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
            }
        }
    }

    fn persist(&mut self) {
        self.library.volume = self.volume;
        if let Err(err) = storage::save_library(&self.library) {
            self.toast(format!("保存音乐库失败：{err:#}"), ToastKind::Error);
        }
        self.last_saved_volume = self.volume;
    }

    fn toast(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast {
            text: text.into(),
            kind,
            created: Instant::now(),
        });
    }

    // ---------------------------------------------------------------------
    // Filtering and sorting
    // ---------------------------------------------------------------------

    fn visible_ids(&self) -> Vec<u64> {
        let terms: Vec<String> = self
            .search
            .split_whitespace()
            .map(|term| term.to_lowercase())
            .collect();

        match self.view {
            View::Playlist(id) => {
                let Some(playlist) = self.library.playlist(id) else {
                    return Vec::new();
                };
                playlist
                    .track_ids
                    .iter()
                    .filter_map(|track_id| self.library.track(*track_id))
                    .filter(|track| track_matches(track, &terms))
                    .map(|track| track.id)
                    .collect()
            }
            View::Favorites => {
                let mut tracks: Vec<&Track> = self
                    .library
                    .favorites
                    .iter()
                    .filter_map(|id| self.library.track(*id))
                    .filter(|track| track_matches(track, &terms))
                    .collect();
                let sort = self.sort;
                tracks.sort_by(|a, b| compare_tracks(a, b, sort));
                tracks.into_iter().map(|track| track.id).collect()
            }
            View::Recent => self
                .library
                .recent
                .iter()
                .filter_map(|id| self.library.track(*id))
                .filter(|track| track_matches(track, &terms))
                .map(|track| track.id)
                .collect(),
            View::Folders => Vec::new(),
            View::Library => {
                let mut tracks: Vec<&Track> = self
                    .library
                    .tracks
                    .iter()
                    .filter(|track| track_matches(track, &terms))
                    .collect();
                let sort = self.sort;
                tracks.sort_by(|a, b| compare_tracks(a, b, sort));
                tracks.into_iter().map(|track| track.id).collect()
            }
        }
    }

    fn view_header(&self, count: usize, total_seconds: f64) -> (String, String) {
        match self.view {
            View::Library => (
                "本地曲库".to_string(),
                model::format_library_summary(count, total_seconds),
            ),
            View::Favorites => (
                "我的收藏".to_string(),
                format!("{count} 首歌曲"),
            ),
            View::Recent => (
                "最近播放".to_string(),
                format!("{count} 首歌曲"),
            ),
            View::Folders => (
                "音乐目录".to_string(),
                format!("{} 个文件夹", self.library.folders.len()),
            ),
            View::Playlist(id) => {
                let name = self
                    .library
                    .playlist(id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "播放列表".to_string());
                let mut subtitle = format!("{count} 首歌曲");
                if total_seconds > 0.0 {
                    subtitle.push_str(&format!(" · {}", model::format_duration(total_seconds)));
                }
                (name, subtitle)
            }
        }
    }

    // ---------------------------------------------------------------------
    // Frame
    // ---------------------------------------------------------------------

    fn poll_audio(&mut self, ctx: &egui::Context) {
        let finished = self
            .audio
            .as_mut()
            .map(AudioEngine::take_finished)
            .unwrap_or(false);
        if finished {
            let can_continue = if self.repeat == RepeatMode::One {
                self.start_current(ctx)
            } else {
                match self.choose_next() {
                    Some(index) => {
                        self.queue_index = index;
                        self.start_current(ctx)
                    }
                    None => false,
                }
            };
            if !can_continue {
                if let Some(audio) = &mut self.audio {
                    audio.stop();
                }
            }
        }

        if self
            .toast
            .as_ref()
            .map(|toast| toast.created.elapsed() > Duration::from_secs_f32(3.6))
            .unwrap_or(false)
        {
            self.toast = None;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            if self.dialog != Dialog::None {
                self.dialog = Dialog::None;
            } else if self.queue_open {
                self.queue_open = false;
            } else if self.expanded {
                self.expanded = false;
            }
        }

        if ctx.wants_keyboard_input() {
            return;
        }

        let (
            space,
            arrow_left,
            arrow_right,
            arrow_up,
            arrow_down,
            delete,
            open_folder,
            new_playlist,
            find,
            modifiers,
        ) = ctx.input(|i| {
            (
                i.key_pressed(Key::Space),
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::ArrowDown),
                i.key_pressed(Key::Delete),
                i.key_pressed(Key::O),
                i.key_pressed(Key::N),
                i.key_pressed(Key::F),
                i.modifiers,
            )
        });

        if modifiers.command && find {
            self.focus_search = true;
        } else if find && self.current.is_some() {
            self.expanded = !self.expanded;
            self.queue_open = false;
        }
        if modifiers.command && open_folder {
            self.open_folder_picker();
        }
        if modifiers.command && new_playlist {
            self.dialog = Dialog::NewPlaylist {
                name: String::new(),
            };
            self.focus_pending = true;
        }
        if delete {
            if let Some(id) = self.selected {
                self.dialog = Dialog::ConfirmRemoveTrack { id };
            }
        }
        if modifiers.command && arrow_right {
            self.next_track(ctx, false);
        } else if modifiers.command && arrow_left {
            self.previous_track(ctx);
        } else if arrow_right {
            let target = self.current_position() + 5.0;
            self.seek_to(target);
        } else if arrow_left {
            let target = (self.current_position() - 5.0).max(0.0);
            self.seek_to(target);
        } else if arrow_up {
            self.set_volume(self.volume + 0.05);
        } else if arrow_down {
            self.set_volume(self.volume - 0.05);
        }

        if space {
            self.toggle_play(ctx);
        }
        if ctx.input(|i| i.pointer.any_released())
            && (self.volume - self.last_saved_volume).abs() > 0.005
        {
            self.persist();
        }
    }

    fn current_position(&self) -> f64 {
        self.audio
            .as_ref()
            .map(AudioEngine::position)
            .unwrap_or(0.0)
    }

    fn current_duration(&self) -> f64 {
        self.current
            .and_then(|id| self.library.track(id))
            .map(|track| track.duration)
            .unwrap_or(0.0)
    }

    fn is_playing(&self) -> bool {
        self.audio
            .as_ref()
            .map(AudioEngine::is_playing)
            .unwrap_or(false)
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = match self.current.and_then(|id| self.library.track(id)) {
            Some(track) => format!("{} — {} · RPlayer", track.title, track.artist),
            None => "RPlayer".to_string(),
        };
        if title != self.last_window_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.last_window_title = title;
        }
    }

    fn request_repaint(&self, ctx: &egui::Context) {
        let playing = self.is_playing();
        let delay = if playing {
            33
        } else if self.scanning || self.toast.is_some() {
            120
        } else {
            500
        };
        ctx.request_repaint_after(Duration::from_millis(delay));
    }

    fn render_toast(&mut self, ctx: &egui::Context) {
        let Some(toast) = &self.toast else {
            return;
        };
        let elapsed = toast.created.elapsed().as_secs_f32();
        let alpha = if elapsed > 3.1 {
            (1.0 - (elapsed - 3.1) / 0.5).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let (color, icon) = match toast.kind {
            ToastKind::Info => (theme::MUTED, Icon::Music),
            ToastKind::Success => (theme::ACCENT, Icon::Check),
            ToastKind::Error => (theme::DANGER, Icon::Close),
        };
        let text = toast.text.clone();

        egui::Area::new(Id::new("rplayer-toast"))
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-24.0, -124.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                Frame::NONE
                    .fill(theme::CARD.gamma_multiply(alpha))
                    .stroke(Stroke::new(1.0_f32, theme::BORDER.gamma_multiply(alpha)))
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(Margin::symmetric(14, 10))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 6],
                        blur: 18,
                        spread: 0,
                        color: Color32::from_black_alpha((90.0 * alpha) as u8),
                    })
                    .show(ui, |ui| {
                        ui.set_max_width(330.0);
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                            icons::paint(ui.painter(), rect, icon, color.gamma_multiply(alpha));
                            ui.add(
                                egui::Label::new(
                                    RichText::new(text)
                                        .size(12.5)
                                        .color(theme::TEXT.gamma_multiply(alpha)),
                                )
                                .wrap(),
                            );
                        });
                    });
            });
    }

    fn render_drop_overlay(&self, ctx: &egui::Context) {
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if !hovering {
            return;
        }
        egui::Area::new(Id::new("rplayer-drop"))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                Frame::NONE
                    .fill(theme::PANEL.gamma_multiply(0.96))
                    .stroke(Stroke::new(1.0_f32, theme::ACCENT))
                    .corner_radius(egui::CornerRadius::same(20))
                    .inner_margin(Margin::same(32))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 18],
                        blur: 44,
                        spread: 0,
                        color: Color32::from_black_alpha(160),
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(320.0);
                        ui.vertical_centered(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(64.0), egui::Sense::hover());
                            ui.painter().circle_filled(
                                rect.center(),
                                32.0,
                                theme::accent_alpha(0.10),
                            );
                            icons::paint(
                                ui.painter(),
                                rect.shrink(16.0),
                                Icon::Folder,
                                theme::ACCENT,
                            );
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("松开以导入音乐")
                                    .size(17.0)
                                    .strong()
                                    .color(theme::TEXT),
                            );
                            ui.label(
                                RichText::new("支持文件夹与音频文件，会自动加入音乐库")
                                    .size(12.0)
                                    .color(theme::MUTED),
                            );
                        });
                    });
            });
    }

    fn handle_close(&mut self, ctx: &egui::Context) {
        if !self.closing_saved && ctx.input(|i| i.viewport().close_requested()) {
            self.persist();
            self.closing_saved = true;
        }
    }

    fn render_dialog(&mut self, ctx: &egui::Context) {
        dialogs::show(self, ctx);
    }
}

impl eframe::App for RPlayerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.dark_titlebar_applied {
            crate::platform::apply_dark_titlebar();
            self.dark_titlebar_applied = true;
        }

        self.poll_scan();
        self.poll_audio(ctx);
        self.handle_shortcuts(ctx);
        self.handle_close(ctx);

        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect()
        });
        if !dropped.is_empty() {
            self.ingest_dropped(dropped);
        }

        let ids = self.visible_ids();
        let total_seconds: f64 = ids
            .iter()
            .filter_map(|id| self.library.track(*id))
            .map(|track| track.duration)
            .sum();

        self.update_title(ctx);

        if self.current.is_none() {
            self.expanded = false;
        }

        if self.expanded && self.current.is_some() {
            self.now_playing(ctx);
            self.render_dialog(ctx);
            self.render_toast(ctx);
            self.render_drop_overlay(ctx);
            self.request_repaint(ctx);
            return;
        }

        self.player_bar(ctx);
        self.sidebar_panel(ctx);
        self.central_panel(ctx, &ids, total_seconds);
        self.queue_window(ctx);
        self.render_dialog(ctx);
        self.render_toast(ctx);
        self.render_drop_overlay(ctx);
        self.request_repaint(ctx);
    }
}

impl Drop for RPlayerApp {
    fn drop(&mut self) {
        self.library.volume = self.volume;
        let _ = storage::save_library(&self.library);
    }
}

fn compare_tracks(a: &Track, b: &Track, sort: SortState) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let ordering = match sort.field {
        SortField::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        SortField::Artist => a
            .artist
            .to_lowercase()
            .cmp(&b.artist.to_lowercase())
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
        SortField::Album => a
            .album
            .to_lowercase()
            .cmp(&b.album.to_lowercase())
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
        SortField::Duration => a
            .duration
            .partial_cmp(&b.duration)
            .unwrap_or(Ordering::Equal),
        SortField::Added => a.added_at.cmp(&b.added_at),
    };
    if sort.ascending {
        ordering
    } else {
        ordering.reverse()
    }
}

fn track_matches(track: &Track, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let haystack = format!(
        "{} {} {} {}",
        track.title, track.artist, track.album, track.genre
    )
    .to_lowercase();
    terms.iter().all(|term| haystack.contains(term))
}

fn path_is_under(folder: &Path, file: &Path) -> bool {
    let folder = model::path_key(folder);
    let file = model::path_key(file);
    if file == folder {
        return true;
    }
    let prefix = format!("{folder}/");
    file.starts_with(&prefix)
}
