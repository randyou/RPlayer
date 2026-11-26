//! Application state, playback orchestration and the root layout.

mod browse;
mod duplicates;
mod media;
mod overlay_ui;
mod playback;
mod scanning;
mod command_palette;
mod dialogs;
mod equalizer;
mod health;
mod layout;
mod library_view;
mod lyrics_edit;
mod now_playing;
mod now_rail;
mod player_bar;
mod session;
mod settings;
mod smart;
mod stats;
mod sidebar;
mod topbar;
mod tray;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use egui::{
    Align2, Color32, Frame, Id, Key, Margin, Pos2, RichText, Stroke, TextureHandle, TextureOptions,
    Vec2, ViewportCommand,
};

use crate::audio::AudioEngine;
use crate::cover::{self, CoverSource};
use crate::eq::EqShared;
use crate::lyrics_fetch::{self, FetchError, FetchResult};
use crate::model::{self, Library, SortField, SortState, Track};
use crate::overlay::{
    self, OverlayCommand, OverlayState,
};
use crate::scanner::{self, ScanEvent};
use crate::storage;
use crate::theme::{self, ThemeMode, ThemePreset};
use crate::ui::icons::{self, Icon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum View {
    Library,
    Albums,
    Artists,
    Favorites,
    Recent,
    Folders,
    Playlist(u64),
    Smart(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueuedNext {
    index: usize,
    id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlayMode {
    Sequential,
    RepeatAll,
    RepeatOne,
    Shuffle,
    ShuffleAll,
}

impl PlayMode {
    fn next(self) -> Self {
        match self {
            Self::Sequential => Self::RepeatAll,
            Self::RepeatAll => Self::RepeatOne,
            Self::RepeatOne => Self::Shuffle,
            Self::Shuffle => Self::ShuffleAll,
            Self::ShuffleAll => Self::Sequential,
        }
    }

    fn is_shuffle(self) -> bool {
        matches!(self, Self::Shuffle | Self::ShuffleAll)
    }

    fn repeats_all(self) -> bool {
        matches!(self, Self::RepeatAll | Self::ShuffleAll)
    }

    fn is_repeat_one(self) -> bool {
        matches!(self, Self::RepeatOne)
    }

    fn icon(self) -> Icon {
        match self {
            Self::Sequential => Icon::List,
            Self::RepeatAll => Icon::Repeat,
            Self::RepeatOne => Icon::RepeatOne,
            Self::Shuffle => Icon::Shuffle,
            Self::ShuffleAll => Icon::ShuffleRepeat,
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            Self::Sequential => "播放模式：顺序播放",
            Self::RepeatAll => "播放模式：列表循环",
            Self::RepeatOne => "播放模式：单曲循环",
            Self::Shuffle => "播放模式：随机播放",
            Self::ShuffleAll => "播放模式：随机循环",
        }
    }

    fn active(self) -> bool {
        !matches!(self, Self::Sequential)
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
    EditTags {
        edit: crate::tag_edit::TagEdit,
    },
    BatchTags {
        ids: Vec<u64>,
        title: String,
        artist: String,
        album: String,
        genre: String,
        year: String,
    },
    Duplicates {
        groups: Vec<crate::duplicates::DuplicateGroup>,
    },
    LyricsMatch {
        id: u64,
        title: String,
        artist: String,
        album: String,
        candidates: Vec<crate::lyrics_fetch::Candidate>,
        searching: bool,
        error: Option<String>,
    },
    Shortcuts,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToastKind {
    Info,
    Success,
    Error,
}

struct LyricsFetchEvent {
    track_id: u64,
    manual: bool,
    result: Result<FetchResult, FetchError>,
}

struct CoverFetchEvent {
    track_id: u64,
    manual: bool,
    result: Result<crate::cover_fetch::FetchResult, crate::cover_fetch::FetchError>,
}

pub(super) struct Toast {
    text: String,
    kind: ToastKind,
    created: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetFailure {
    NotFound,
    Timeout,
    RateLimited,
}

pub struct RPlayerApp {
    library: Library,
    stats: crate::stats::PlaybackStats,
    stats_dirty: bool,
    last_stats_save: std::time::Instant,
    play_credited: bool,
    browse: browse::BrowseState,
    album_focus: Option<String>,
    artist_focus: Option<String>,
    smart_builder: Option<smart::SmartDraft>,
    health_report: Option<crate::health::HealthReport>,
    health_rx: Option<Receiver<crate::health::HealthReport>>,
    health_checking: bool,
    /// Which health stat card is drilled into, if any.
    health_pane: Option<health::HealthPane>,
    #[cfg(target_os = "windows")]
    tray: Option<crate::platform::tray::Tray>,
    /// True when a background heartbeat already pokes the UI, so the idle path
    /// does not schedule a second timer that would double the idle frame rate.
    tray_heartbeat: bool,
    /// Mirrors the root viewport's minimized state for the heartbeat thread.
    window_minimized: std::sync::Arc<std::sync::atomic::AtomicBool>,
    allow_exit: bool,
    main_window_pos: Option<Pos2>,
    audio: Option<AudioEngine>,
    eq: Arc<EqShared>,
    view: View,
    /// 0..1 fade-in progress after a view switch.
    view_fade: f32,
    /// Viewport whose `update` is running right now. The root viewport clears to
    /// the theme background so a region the panels happen to miss reads as the
    /// window background instead of letting the desktop show through; the
    /// floating overlays keep a transparent framebuffer for their rounded cards.
    active_viewport: egui::ViewportId,
    search: String,
    sort: SortState,
    selected: Option<u64>,
    table_focus: Option<u64>,
    table_scroll_to: Option<usize>,
    table_restore: bool,
    current: Option<u64>,
    queue: Vec<u64>,
    queue_index: usize,
    play_mode: PlayMode,
    shuffle_bag: Vec<usize>,
    shuffle_exhausted: bool,
    volume: f32,
    volume_before_mute: f32,
    last_saved_volume: f32,
    cover: Option<TextureHandle>,
    cover_for: Option<u64>,
    cover_image: Option<Arc<egui::ColorImage>>,
    cover_source: Option<CoverSource>,
    cover_tint: Option<Color32>,
    cover_tx: Sender<CoverFetchEvent>,
    cover_rx: Receiver<CoverFetchEvent>,
    cover_fetching: Option<u64>,
    cover_started: Option<std::time::Instant>,
    cover_attempted: HashSet<String>,
    overlay_state: Arc<Mutex<OverlayState>>,
    overlay_commands: Arc<Mutex<Vec<OverlayCommand>>>,
    /// Hash of the last published overlay content. Used to wake the overlay
    /// viewports explicitly when something visible changes, instead of relying
    /// on their own 33 ms paint timer.
    overlay_sync_key: u64,
    /// Open tray popup, if any. It lives in its own viewport so it can be drawn
    /// with the app's menu rows instead of a native Win32 menu.
    tray_menu: Option<tray::TrayMenu>,
    /// Development aid: `RPLAYER_UI_SCALE=1.5` renders the whole UI at that
    /// scale, which is how the DPI matrix is exercised on a 100% display.
    debug_scale: Option<f32>,
    /// Development aid: `RPLAYER_FRAME_LOG=<path>` appends one line per frame
    /// with the `update` build time in ms plus the repaint causes, used by the
    /// 10k scroll performance baseline.
    frame_log: Option<std::fs::File>,
    /// Development aid: `RPLAYER_SECTION_LOG=<path>` appends one line per
    /// frame with the time spent in each panel group, so a slow frame can be
    /// attributed without a profiler.
    section_log: Option<std::fs::File>,
    /// Last `view_cache_key` plus its sorted ids. `visible_ids` filters and
    /// sorts the whole library, so it is only rebuilt when that key changes.
    visible_ids_cache: Option<(u64, Vec<u64>)>,
    /// Library revision plus the album and artist counts shown in the sidebar.
    /// Grouping 10k tracks every frame used to dominate the frame budget.
    nav_counts_cache: Option<(u64, usize, usize)>,
    mini_cache: Arc<Mutex<overlay::MiniCache>>,
    /// Epoch for the frame log's timestamp column, so frame intervals can be
    /// derived (the log otherwise only records the UI build time).
    started: std::time::Instant,
    lyrics_placed: bool,
    mini_placed: bool,
    lyrics_region: bool,
    mini_region: bool,
    lyrics_viewport_active: bool,
    mini_viewport_active: bool,
    lyrics: Option<crate::lyrics::Lyrics>,
    lyrics_for: Option<u64>,
    lyrics_editor: Option<crate::lyrics::LyricsEditor>,
    lyrics_candidates_rx: Option<Receiver<Result<Vec<crate::lyrics_fetch::Candidate>, FetchError>>>,
    lyrics_source: Option<String>,
    expanded: bool,
    batch_mode: bool,
    checked: HashSet<u64>,
    selection_anchor: Option<u64>,
    scanning: bool,
    scan_rx: Option<Receiver<ScanEvent>>,
    scan_total: usize,
    scan_done: usize,
    scan_seen: HashSet<u64>,
    scan_errors: Vec<String>,
    /// D12 inline error card: expanded state and dismiss flag.
    error_card_open: bool,
    error_card_dismissed: bool,
    scan_finished_at: Option<std::time::Instant>,
    ignored_duplicate_groups: HashSet<String>,
    dialog: Dialog,
    last_dialog: Dialog,
    dialog_anim_start: std::time::Instant,
    dialog_focus_before: Option<egui::Id>,
    focus_pending: bool,
    focus_search: bool,
    command_open: bool,
    command_anim_start: std::time::Instant,
    command_query: String,
    command_index: usize,
    command_focus: bool,
    toast: Option<Toast>,
    queue_open: bool,
    last_window_title: String,
    closing_saved: bool,
    theme_mode: ThemeMode,
    theme_preset: ThemePreset,
    titlebar_dark: Option<bool>,
    eq_open: bool,
    eq_dirty: bool,
    desktop_lyrics_open: bool,
    mini_open: bool,
    settings_open: bool,
    /// When the settings dialog was opened; a click that opens it must not be
    /// read as the Escape that closes it on the very same frame.
    settings_opened: Option<std::time::Instant>,
    settings_tab: usize,
    now_rail_open: bool,
    sidebar_drawer_open: bool,
    /// Playlist row currently under a track drag, for the D11 drop hint.
    playlist_drop_target: Option<(u64, String)>,
    rail_queue: bool,
    pending_resume: Option<f64>,
    last_session_save: std::time::Instant,
    queue_drag: Option<usize>,
    queued_next: Option<QueuedNext>,
    boundary_seen: u64,
    watch: Option<crate::watch::FolderWatcher>,
    watch_folders: Vec<PathBuf>,
    watch_pending: bool,
    media_rx: Option<Receiver<crate::platform::media::MediaCommand>>,
    smtc: Option<crate::platform::smtc_client::SmtcClient>,
    smtc_attempted: bool,
    lyrics_tx: Sender<LyricsFetchEvent>,
    lyrics_rx: Receiver<LyricsFetchEvent>,
    lyrics_fetching: Option<u64>,
    lyrics_attempted: HashSet<u64>,
    last_net_failure: Option<NetFailure>,
    prefs_dirty: bool,
}

impl RPlayerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let library = storage::load_library();
        let theme_preset = ThemePreset::from_id(&library.theme_preset).unwrap_or_else(|| {
            if library.light_theme {
                ThemePreset::Paper
            } else {
                ThemePreset::Forest
            }
        });
        let theme_mode = theme_preset.mode();
        theme::install_preset(&cc.egui_ctx, theme_preset);
        cc.egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::SetTheme(if theme_mode.is_dark() {
                egui::SystemTheme::Dark
            } else {
                egui::SystemTheme::Light
            }));

        let volume = library.volume.clamp(0.0, 1.0);
        let desktop_lyrics_open = library.desktop_lyrics;
        let (lyrics_tx, lyrics_rx) = mpsc::channel();
        let (cover_tx, cover_rx) = mpsc::channel();
        let eq = EqShared::new(library.eq.clone());
        let (mut audio, audio_error) = match AudioEngine::new(volume, eq.clone()) {
            Ok(engine) => (Some(engine), None),
            Err(err) => (None, Some(format!("{err:#}"))),
        };

        if let Some(engine) = &mut audio {
            engine.set_replaygain_mode(&library.replaygain);
        }
        let mut app = Self {
            volume,
            last_saved_volume: volume,
            volume_before_mute: if volume > 0.01 { volume } else { 0.72 },
            library,
            stats: crate::stats::PlaybackStats::load(),
            stats_dirty: false,
            last_stats_save: std::time::Instant::now(),
            play_credited: false,
            browse: browse::BrowseState::new(),
            album_focus: None,
            artist_focus: None,
            smart_builder: None,
            health_report: None,
            health_rx: None,
            health_checking: false,
            health_pane: None,
            #[cfg(target_os = "windows")]
            tray: None,
            tray_heartbeat: false,
            window_minimized: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            allow_exit: false,
            main_window_pos: None,
            tray_menu: None,
            audio,
            eq,
            view: View::Library,
            view_fade: 1.0,
            active_viewport: egui::ViewportId::ROOT,
            search: String::new(),
            sort: SortState::default(),
            selected: None,
            table_focus: None,
            table_scroll_to: None,
            table_restore: false,
            current: None,
            queue: Vec::new(),
            queue_index: 0,
            play_mode: PlayMode::Sequential,
            shuffle_bag: Vec::new(),
            shuffle_exhausted: false,
            cover: None,
            cover_for: None,
            cover_image: None,
            cover_source: None,
            cover_tint: None,
            cover_tx,
            cover_rx,
            cover_fetching: None,
            cover_started: None,
            cover_attempted: HashSet::new(),
            overlay_state: Arc::new(Mutex::new(OverlayState::default())),
            overlay_commands: Arc::new(Mutex::new(Vec::new())),
            overlay_sync_key: 0,
            debug_scale: std::env::var("RPLAYER_UI_SCALE")
                .ok()
                .and_then(|value| value.trim().parse::<f32>().ok())
                .filter(|scale| (0.5..=4.0).contains(scale)),
            frame_log: std::env::var("RPLAYER_FRAME_LOG")
                .ok()
                .and_then(|path| std::fs::File::create(path).ok()),
            section_log: std::env::var("RPLAYER_SECTION_LOG")
                .ok()
                .and_then(|path| std::fs::File::create(path).ok()),
            visible_ids_cache: None,
            nav_counts_cache: None,
            mini_cache: Arc::new(Mutex::new(overlay::MiniCache::default())),
            started: std::time::Instant::now(),
            lyrics_placed: false,
            mini_placed: false,
            lyrics_region: false,
            mini_region: false,
            lyrics_viewport_active: false,
            mini_viewport_active: false,
            lyrics: None,
            lyrics_for: None,
            lyrics_editor: None,
            lyrics_candidates_rx: None,
            lyrics_source: None,
            expanded: false,
            batch_mode: false,
            checked: HashSet::new(),
            selection_anchor: None,
            scanning: false,
            scan_rx: None,
            scan_total: 0,
            scan_done: 0,
            scan_seen: HashSet::new(),
            scan_errors: Vec::new(),
            error_card_open: false,
            error_card_dismissed: false,
            scan_finished_at: None,
            ignored_duplicate_groups: HashSet::new(),
            dialog: Dialog::None,
            last_dialog: Dialog::None,
            dialog_anim_start: std::time::Instant::now(),
            dialog_focus_before: None,
            focus_pending: false,
            focus_search: false,
            command_open: false,
            command_anim_start: std::time::Instant::now(),
            command_query: String::new(),
            command_index: 0,
            command_focus: false,
            toast: None,
            queue_open: false,
            last_window_title: "RPlayer".to_string(),
            closing_saved: false,
            theme_mode,
            theme_preset,
            titlebar_dark: None,
            eq_open: false,
            eq_dirty: false,
            desktop_lyrics_open,
            mini_open: false,
            settings_open: false,
            settings_opened: None,
            settings_tab: 0,
            now_rail_open: true,
            sidebar_drawer_open: false,
            playlist_drop_target: None,
            rail_queue: false,
            pending_resume: None,
            last_session_save: std::time::Instant::now(),
            queue_drag: None,
            queued_next: None,
            boundary_seen: 0,
            watch: None,
            watch_folders: Vec::new(),
            watch_pending: false,
            media_rx: None,
            smtc: None,
            smtc_attempted: false,
            lyrics_tx,
            lyrics_rx,
            lyrics_fetching: None,
            lyrics_attempted: HashSet::new(),
            last_net_failure: None,
            prefs_dirty: false,
        };

        #[cfg(target_os = "windows")]
        {
            app.tray = crate::platform::tray::Tray::create();
            if app.tray.is_some() {
                app.tray_heartbeat = true;
                // A tray click pushes an action and then wakes the UI, so the
                // menu opens on the next frame instead of waiting for a tick.
                {
                    let ctx = cc.egui_ctx.clone();
                    crate::platform::tray::Tray::set_wake(move || {
                        ctx.request_repaint_of(egui::ViewportId::ROOT);
                    });
                }
                // The main viewport is minimized when sent to the tray, so the
                // heartbeat is what keeps menu events flowing while hidden.
                // Visible-but-paused windows do not need that rate: the frame
                // loop is the single idle timer (see `request_repaint`) and one
                // frame costs ~14 ms of paint, so it ticks slowly on purpose.
                let ctx = cc.egui_ctx.clone();
                let minimized = app.window_minimized.clone();
                std::thread::spawn(move || loop {
                    let hidden = minimized.load(std::sync::atomic::Ordering::Relaxed);
                    let ms = if hidden { 300 } else { 3000 };
                    std::thread::sleep(std::time::Duration::from_millis(ms));
                    ctx.request_repaint_of(egui::ViewportId::ROOT);
                });
            }
        }
        if let Some(err) = audio_error {
            app.toast(format!("音频输出不可用：{err}"), ToastKind::Error);
        }
        {
            // Media keys wake the UI directly; the idle heartbeat can then be
            // slow without making the hardware keys feel laggy.
            let ctx = cc.egui_ctx.clone();
            app.media_rx = crate::platform::media::start(move || {
                ctx.request_repaint_of(egui::ViewportId::ROOT);
            });
        }
        if let Some(session) = crate::session::load() {
            app.restore_session(session);
        }
        app.sync_watch();
        if !app.library.folders.is_empty() && app.library.tracks.is_empty() {
            app.start_scan();
        }
        app
    }

    // ---------------------------------------------------------------------
    // Scan pipeline
    // ---------------------------------------------------------------------

    fn undo_last_tag_batch(&mut self) {
        let Some(batch) = self.library.last_tag_batch.take() else {
            self.toast("没有可撤销的批量标签修改", ToastKind::Info);
            return;
        };
        let mut ok = 0usize;
        let mut retry = Vec::new();
        for edit in batch {
            let restore = crate::tag_edit::TagEdit::from_track(&edit.original);
            match crate::tag_edit::save(&restore) {
                Ok(updated) => {
                    self.replace_track(updated);
                    ok += 1;
                }
                Err(_) => retry.push(edit),
            }
        }
        let failed = retry.len();
        self.library.last_tag_batch = if retry.is_empty() {
            None
        } else {
            Some(retry)
        };
        self.persist();
        self.toast(
            format!("已撤销批量标签：成功 {ok} 首，失败 {failed} 首"),
            if failed == 0 { ToastKind::Success } else { ToastKind::Error },
        );
    }

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

    fn replace_track(&mut self, updated: Track) {
        if let Some(index) = self
            .library
            .tracks
            .iter()
            .position(|track| track.id == updated.id)
        {
            self.library.tracks[index] = updated;
            self.persist();
        }
    }

    fn remove_track(&mut self, id: u64) {
        let removed = self
            .library
            .tracks
            .iter()
            .find(|track| track.id == id)
            .map(|track| track.title.clone());
        self.library.tracks.retain(|track| track.id != id);
        self.library.refresh_index();
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
                self.cover_image = None;
            }
        }
    }

    fn switch_view(&mut self, view: View) {
        if self.view == view {
            self.album_focus = None;
            self.artist_focus = None;
            self.selected = None;
            self.view_fade = 0.0;
            return;
        }
        let restore_focus = self.table_focus.or(self.selected);
        let restore_table = self.table_focus.is_some() || self.selected.is_some();
        self.view = view;
        self.selected = None;
        self.table_focus = restore_focus;
        self.table_restore = restore_table;
        // GAP 09: a cross-view selection survives switching views; only the
        // in-table anchor is dropped so a later Shift range starts fresh.
        self.selection_anchor = None;
        self.album_focus = None;
        self.artist_focus = None;
        self.view_fade = 0.0;
    }

    fn set_theme_preset(&mut self, ctx: &egui::Context, preset: ThemePreset) {
        if self.theme_preset == preset {
            return;
        }
        self.theme_preset = preset;
        self.theme_mode = preset.mode();
        self.library.light_theme = !preset.is_dark();
        self.library.theme_preset = preset.id().to_string();
        theme::set_preset(preset);
        theme::apply_style(ctx);
        ctx.send_viewport_cmd(ViewportCommand::SetTheme(if preset.is_dark() {
            egui::SystemTheme::Dark
        } else {
            egui::SystemTheme::Light
        }));
        self.titlebar_dark = None;
        self.persist();
        self.toast(
            format!("已切换到「{}」主题", preset.label()),
            ToastKind::Info,
        );
    }

    fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.set_theme_preset(ctx, self.theme_preset.toggle_mode());
    }

    fn toggle_eq(&mut self) {
        self.eq_open = !self.eq_open;
    }

    fn toggle_desktop_lyrics(&mut self, ctx: &egui::Context) {
        self.desktop_lyrics_open = !self.desktop_lyrics_open;
        self.library.desktop_lyrics = self.desktop_lyrics_open;
        if self.desktop_lyrics_open {
            self.lyrics_placed = false;
            self.lyrics_region = false;
        } else {
            self.lyrics_viewport_active = false;
            self.lyrics_region = false;
            crate::platform::hide_lyrics_panel();
            ctx.send_viewport_cmd_to(overlay::lyrics_viewport(), ViewportCommand::Close);
        }
        self.persist();
    }

    fn toggle_mini(&mut self, ctx: &egui::Context) {
        self.mini_open = !self.mini_open;
        if self.mini_open {
            self.mini_placed = false;
            self.mini_region = false;
            // Switch modes: the main window collapses while the mini player is
            // shown, but keeps its event loop alive to service playback commands.
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                ViewportCommand::Minimized(true),
            );
        } else {
            self.mini_viewport_active = false;
            self.mini_region = false;
            ctx.send_viewport_cmd_to(overlay::mini_viewport(), ViewportCommand::Close);
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                ViewportCommand::Minimized(false),
            );
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, ViewportCommand::Visible(true));
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, ViewportCommand::Focus);
        }
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
        self.selection_anchor = Some(id);
    }

    fn select_range(&mut self, ids: &[u64], target: u64) {
        let Some(anchor) = self.selection_anchor else {
            self.checked.clear();
            self.checked.insert(target);
            self.selection_anchor = Some(target);
            self.batch_mode = true;
            return;
        };
        let Some(from) = ids.iter().position(|id| *id == anchor) else {
            self.checked.clear();
            self.checked.insert(target);
            self.selection_anchor = Some(target);
            self.batch_mode = true;
            return;
        };
        let Some(to) = ids.iter().position(|id| *id == target) else {
            return;
        };
        let (start, end) = if from <= to { (from, to) } else { (to, from) };
        self.checked.clear();
        self.checked.extend(ids[start..=end].iter().copied());
        self.batch_mode = true;
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

    fn add_ids_to_playlist(&mut self, playlist_id: u64, ids: &[u64]) -> usize {
        let mut added = 0usize;
        for id in ids {
            if self.library.add_to_playlist(playlist_id, *id) {
                added += 1;
            }
        }
        if added > 0 {
            self.persist();
        }
        added
    }

    fn batch_add_to_playlist(&mut self, playlist_id: u64) {
        let ids: Vec<u64> = self.checked.iter().copied().collect();
        let added = self.add_ids_to_playlist(playlist_id, &ids);
        let name = self
            .library
            .playlist(playlist_id)
            .map(|playlist| playlist.name.clone())
            .unwrap_or_default();
        let total = ids.len();
        // A silent "0 首" toast reads like a failure; say what actually happened.
        let (message, kind) = if added == 0 {
            (format!("这 {total} 首已在「{name}」中"), ToastKind::Info)
        } else if added < total {
            (
                format!(
                    "已添加 {added} 首到「{name}」，{} 首已存在",
                    total - added
                ),
                ToastKind::Success,
            )
        } else {
            (
                format!("已添加 {added} 首歌曲到「{name}」"),
                ToastKind::Success,
            )
        };
        self.toast(message, kind);
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
        // Anything that reaches a save also changes what the views show
        // (tags, favorites, playlists, removals), so it invalidates the
        // cross-frame caches.
        self.library.touch();
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

    fn visible_ids(&mut self) -> Vec<u64> {
        let key = self.view_cache_key();
        if let Some((cached, ids)) = &self.visible_ids_cache {
            if *cached == key {
                return ids.clone();
            }
        }
        let ids = self.compute_visible_ids();
        self.visible_ids_cache = Some((key, ids.clone()));
        ids
    }

    /// Album and artist counts for the sidebar, recomputed only when the
    /// library content changes.
    fn nav_counts(&mut self) -> (usize, usize) {
        let revision = self.library.revision();
        if let Some((cached, albums, artists)) = self.nav_counts_cache {
            if cached == revision {
                return (albums, artists);
            }
        }
        let ids: Vec<u64> = self.library.tracks.iter().map(|track| track.id).collect();
        let albums = self.album_groups(&ids).len();
        let artists = self.artist_groups(&ids).len();
        self.nav_counts_cache = Some((revision, albums, artists));
        (albums, artists)
    }

    /// Identifies everything `compute_visible_ids` depends on, so the sorted id
    /// list survives across frames until the library or the view state changes.
    fn view_cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.library.revision().hash(&mut hasher);
        match self.view {
            View::Library => 0u8.hash(&mut hasher),
            View::Albums => 1u8.hash(&mut hasher),
            View::Artists => 2u8.hash(&mut hasher),
            View::Favorites => 3u8.hash(&mut hasher),
            View::Recent => 4u8.hash(&mut hasher),
            View::Folders => 5u8.hash(&mut hasher),
            View::Playlist(id) => {
                6u8.hash(&mut hasher);
                id.hash(&mut hasher);
            }
            View::Smart(id) => {
                7u8.hash(&mut hasher);
                id.hash(&mut hasher);
            }
        }
        (self.sort.field as u8).hash(&mut hasher);
        self.sort.ascending.hash(&mut hasher);
        self.search.hash(&mut hasher);
        hasher.finish()
    }

    fn compute_visible_ids(&self) -> Vec<u64> {
        let terms: Vec<String> = self
            .search
            .split_whitespace()
            .map(|term| term.to_lowercase())
            .collect();

        match self.view {
            View::Smart(id) => self
                .library
                .smart_playlist(id)
                .map(|playlist| self.smart_ids(playlist))
                .unwrap_or_default(),
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
            View::Albums => {
                let mut tracks: Vec<&Track> = self
                    .library
                    .tracks
                    .iter()
                    .filter(|track| track_matches(track, &terms))
                    .collect();
                tracks.sort_by(|a, b| {
                    a.album
                        .to_lowercase()
                        .cmp(&b.album.to_lowercase())
                        .then_with(|| a.artist.to_lowercase().cmp(&b.artist.to_lowercase()))
                        .then_with(|| a.track_number.cmp(&b.track_number))
                });
                tracks.into_iter().map(|track| track.id).collect()
            }
            View::Artists => {
                let mut tracks: Vec<&Track> = self
                    .library
                    .tracks
                    .iter()
                    .filter(|track| track_matches(track, &terms))
                    .collect();
                tracks.sort_by(|a, b| {
                    a.artist
                        .to_lowercase()
                        .cmp(&b.artist.to_lowercase())
                        .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase()))
                        .then_with(|| a.track_number.cmp(&b.track_number))
                });
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
            View::Albums => (
                "专辑".to_string(),
                format!("{count} 首歌曲 · 按专辑浏览"),
            ),
            View::Artists => (
                "艺术家".to_string(),
                format!("{count} 首歌曲 · 按艺术家浏览"),
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
            View::Smart(id) => {
                let name = self
                    .library
                    .smart_playlist(id)
                    .map(|list| list.name.clone())
                    .unwrap_or_else(|| "智能歌单".to_string());
                (name, format!("{count} 首歌曲 · 智能规则"))
            }
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

    fn sync_smtc(&mut self) {
        let track = self
            .current
            .and_then(|id| self.library.track(id))
            .cloned();
        let playing = self
            .audio
            .as_ref()
            .is_some_and(|audio| audio.is_playing());
        let position = self.current_position();
        let duration = track.as_ref().map(|track| track.duration).unwrap_or(0.0);
        if let Some(smtc) = &mut self.smtc {
            smtc.sync(
                track.as_ref(),
                playing,
                position,
                duration,
                self.cover_image.as_deref(),
            );
        }
    }

    fn poll_media_keys(&mut self, ctx: &egui::Context) {
        let mut commands = Vec::new();
        if let Some(receiver) = &self.media_rx {
            while let Ok(command) = receiver.try_recv() {
                commands.push(command);
            }
        }
        if let Some(smtc) = &mut self.smtc {
            commands.extend(smtc.poll());
        }
        for command in commands {
            match command {
                crate::platform::media::MediaCommand::PlayPause => {
                    if self.current.is_some() {
                        self.toggle_play(ctx);
                    }
                }
                crate::platform::media::MediaCommand::Next => {
                    if self.current.is_some() {
                        self.next_track(ctx, false);
                    }
                }
                crate::platform::media::MediaCommand::Previous => {
                    if self.current.is_some() {
                        self.previous_track(ctx);
                    }
                }
                crate::platform::media::MediaCommand::Stop => {
                    if let Some(audio) = &mut self.audio {
                        audio.set_paused(true);
                    }
                }
            }
        }
    }

    fn preload_gapless(&mut self) {
        if self.queued_next.is_some() {
            return;
        }
        let Some(current_id) = self.current else {
            return;
        };
        let Some(track) = self.library.track(current_id).cloned() else {
            return;
        };
        if track.cue_end.is_some() {
            return;
        }
        let Some(audio) = self.audio.as_ref() else {
            return;
        };
        let position = audio.position();
        if track.duration <= 0.0 || track.duration - position > 1.4 {
            return;
        }
        let next_index = if self.play_mode.is_repeat_one() {
            Some(self.queue_index)
        } else {
            self.choose_next()
        };
        let Some(next_index) = next_index else {
            return;
        };
        let Some(next_id) = self.queue.get(next_index).copied() else {
            return;
        };
        let Some(next_track) = self.library.track(next_id).cloned() else {
            return;
        };
        if next_id == current_id && self.play_mode == PlayMode::Sequential {
            return;
        }
        if next_track.cue_end.is_some() {
            return;
        }
        if let Some(audio) = &mut self.audio {
            if audio.queue_next(&next_track.path).is_ok() {
                self.queued_next = Some(QueuedNext {
                    index: next_index,
                    id: next_id,
                });
            }
        }
    }

    fn consume_gapless_boundary(&mut self, ctx: &egui::Context) {
        let Some(next) = self.queued_next.take() else {
            return;
        };
        self.credit_completed_track();
        if let Some(audio) = &mut self.audio {
            audio.begin_next_track();
        }
        self.queue_index = next.index;
        let Some(track) = self.library.track(next.id).cloned() else {
            self.current = None;
            return;
        };
        self.current = Some(next.id);
        self.library.mark_played(next.id);
        self.load_lyrics_for(&track, next.id);
        self.load_cover(ctx, next.id);
        self.persist();
        if self.library.auto_lyrics && self.lyrics.is_none() {
            self.request_lyrics(ctx, next.id, false);
        }
    }

    fn cue_boundary_reached(&self) -> bool {
        let Some(track) = self.current.and_then(|id| self.library.track(id)) else {
            return false;
        };
        let Some(audio) = self.audio.as_ref() else {
            return false;
        };
        audio.has_source()
            && audio.is_playing()
            && crate::playback::reached_end(track, audio.position())
    }

    fn poll_audio(&mut self, ctx: &egui::Context) {
        // CUE virtual tracks share one decoder; stop at this track's CUE end
        // instead of waiting for the whole file to finish.
        if self.cue_boundary_reached() {
            self.next_track(ctx, true);
            return;
        }

        let boundary = self
            .audio
            .as_ref()
            .map(AudioEngine::boundary_count)
            .unwrap_or(0);
        if boundary > self.boundary_seen {
            if self.queued_next.is_some() {
                self.consume_gapless_boundary(ctx);
            }
            self.boundary_seen = boundary;
        }
        self.preload_gapless();

        let finished = self
            .audio
            .as_mut()
            .map(AudioEngine::take_finished)
            .unwrap_or(false);
        if finished {
            self.queued_next = None;
            let can_continue = if self.play_mode.is_repeat_one() {
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
        if self.dialog == Dialog::None && ctx.input(|i| i.key_pressed(Key::F1)) {
            self.dialog = Dialog::Shortcuts;
        }

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            if self.dialog != Dialog::None {
                self.dialog = Dialog::None;
                self.restore_dialog_focus(ctx);
            } else if self.settings_open
                && self
                    .settings_opened
                    .is_none_or(|at| at.elapsed().as_millis() >= 250)
            {
                self.settings_open = false;
                self.settings_opened = None;
            } else if self.eq_open {
                self.eq_open = false;
            } else if self.queue_open {
                self.queue_open = false;
            } else if self.batch_mode && !self.checked.is_empty() {
                self.checked.clear();
                self.selection_anchor = None;
                self.batch_mode = false;
            } else if self.expanded {
                self.expanded = false;
            }
        }

        // A modal owns the keyboard. Do not let Space/arrows drive the
        // player or seek the timeline behind the backdrop.
        if self.dialog != Dialog::None {
            return;
        }

        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::K)) {
            self.toggle_command_palette();
            return;
        }

        // These commands stay available even when a custom widget owns focus;
        // they are the entry points a keyboard-only user needs most.
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::F)) {
            self.focus_search = true;
            return;
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::Comma)) {
            self.settings_open = !self.settings_open;
            return;
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::T)) {
            self.toggle_theme(ctx);
            return;
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
            select_all,
            theme_key,
            equalizer_key,
            lyrics_key,
            mini_key,
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
                i.key_pressed(Key::A),
                i.key_pressed(Key::T),
                i.key_pressed(Key::E),
                i.key_pressed(Key::L),
                i.key_pressed(Key::M),
                i.modifiers,
            )
        });

        if modifiers.command && find {
            self.focus_search = true;
        } else if find && self.current.is_some() {
            self.expanded = !self.expanded;
            self.queue_open = false;
        }
        if modifiers.command && select_all {
            let ids = self.visible_ids();
            if !ids.is_empty() {
                self.batch_mode = true;
                self.set_all_checked(&ids, true);
                self.selection_anchor = ids.first().copied();
            }
        }
        if modifiers.command && ctx.input(|input| input.key_pressed(Key::Comma)) {
            self.settings_open = !self.settings_open;
        }
        if modifiers.command && theme_key {
            self.toggle_theme(ctx);
        }
        if modifiers.command && equalizer_key {
            self.toggle_eq();
        }
        if modifiers.command && lyrics_key {
            self.toggle_desktop_lyrics(ctx);
        }
        if modifiers.command && mini_key {
            self.toggle_mini(ctx);
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
        let Some(audio) = self.audio.as_ref() else {
            return 0.0;
        };
        let absolute = audio.position();
        match self.current.and_then(|id| self.library.track(id)) {
            Some(track) => crate::playback::relative(track, absolute),
            None => absolute,
        }
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
        let delay = if self.is_playing() {
            33
        } else if self.scanning
            || self.toast.is_some()
            || self.lyrics_fetching.is_some()
            || self.cover_fetching.is_some()
        {
            120
        } else if self.tray_heartbeat {
            // The tray thread already pokes the root viewport on a slow tick;
            // a second idle timer here would double the idle frame rate.
            return;
        } else {
            // Fully idle: nothing on screen changes on its own, so a slow tick
            // is enough to pick up media-key and filesystem events.
            1000
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
            ToastKind::Info => (theme::c().muted, Icon::Music),
            ToastKind::Success => (theme::c().accent, Icon::Check),
            ToastKind::Error => (theme::c().danger, Icon::Close),
        };
        let text = toast.text.clone();

        egui::Area::new(Id::new("rplayer-toast"))
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-24.0, -124.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                Frame::NONE
                    .fill(theme::c().card.gamma_multiply(alpha))
                    .stroke(Stroke::new(1.0_f32, theme::c().border.gamma_multiply(alpha)))
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(Margin::symmetric(14, 10))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 6],
                        blur: 18,
                        spread: 0,
                        color: crate::theme::scrim((90.0 * alpha) as u8),
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
                                        .color(theme::c().text.gamma_multiply(alpha)),
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
                    .fill(theme::c().panel.gamma_multiply(0.96))
                    .stroke(Stroke::new(1.0_f32, theme::c().accent))
                    .corner_radius(egui::CornerRadius::same(20))
                    .inner_margin(Margin::same(32))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 18],
                        blur: 44,
                        spread: 0,
                        color: crate::theme::scrim(160),
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
                                theme::c().accent,
                            );
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("松开以导入音乐")
                                    .size(17.0)
                                    .strong()
                                    .color(theme::c().text),
                            );
                            ui.label(
                                RichText::new("支持文件夹与音频文件，会自动加入音乐库")
                                    .size(12.0)
                                    .color(theme::c().muted),
                            );
                        });
                    });
            });
    }

    fn flush_prefs(&mut self, ctx: &egui::Context) {
        if self.prefs_dirty && !ctx.input(|input| input.pointer.any_down()) {
            self.persist();
            self.prefs_dirty = false;
        }
    }

    fn handle_close(&mut self, ctx: &egui::Context) {
        if !self.closing_saved
            && !self.allow_exit
            && ctx.input(|i| i.viewport().close_requested())
        {
            #[cfg(target_os = "windows")]
            if self.tray.is_some() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                // Minimize rather than hide: a hidden window stops pumping
                // frames, so the tray menu would never open again. The taskbar
                // button is dropped instead, which is what "closed to tray"
                // should look like (v2 regression A5).
                self.main_window_pos =
                    ctx.input(|input| input.viewport().outer_rect).map(|rect| rect.min);
                crate::platform::set_taskbar_button_visible(false);
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                return;
            }
        }
        if !self.closing_saved && ctx.input(|i| i.viewport().close_requested()) {
            self.credit_current_track(false);
            self.persist();
            self.save_session();
            self.save_stats();
            self.closing_saved = true;
        }
    }

    fn render_dialog(&mut self, ctx: &egui::Context) {
        dialogs::show(self, ctx);
    }

    pub(super) fn restore_dialog_focus(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.dialog_focus_before.take() {
            ctx.memory_mut(|memory| memory.request_focus(id));
        }
    }
}

impl eframe::App for RPlayerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let started = std::time::Instant::now();
        // A theme swap re-styles egui every frame until the blend finishes so
        // built-in widgets (menus, tooltips, text edits) fade with the custom
        // painted surfaces instead of snapping to the new palette.
        if theme::transition_active() {
            theme::apply_style(ctx);
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        self.update_inner(ctx, frame);
        if let Some(file) = &mut self.frame_log {
            use std::io::Write as _;
            let causes: Vec<String> = ctx
                .repaint_causes()
                .iter()
                .map(|cause| format!("{cause:?}"))
                .collect();
            let _ = writeln!(
                file,
                "{:.2}\t{:.2}\t{}",
                started.elapsed().as_secs_f64() * 1000.0,
                self.started.elapsed().as_secs_f64() * 1000.0,
                causes.join(" | ")
            );
        }
    }
}

impl RPlayerApp {
    fn update_inner(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.active_viewport = ctx.viewport_id();
        // Remember the player HWND; the lyrics overlay is created non-activating
        // but can still be handed the foreground, and then every "which window is
        // the player?" lookup would answer with the overlay (v2.1.0 issue #2).
        // This returns the cached handle straight away once it is known, so the
        // enum scan only runs on the first frames where no player window exists
        // yet.
        crate::platform::cache_main_window();
    // Accessibility probe: a manual screen-reader session has to tell "the
    // platform adapter never forwards the action" apart from "egui received it
    // but dropped the id". Setting RPLAYER_AK_LOG appends every action request
    // that actually reaches egui, so the two cases look different on disk.
    if let Ok(path) = std::env::var("RPLAYER_AK_LOG") {
        ctx.input(|input| {
            for event in &input.events {
                if let egui::Event::AccessKitActionRequest(request) = event {
                    use std::io::Write as _;
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&path)
                    {
                        let _ = writeln!(file, "{:?}", request.action);
                    }
                }
            }
        });
    }

        let minimized = ctx
            .input(|input| input.viewport().minimized)
            .unwrap_or(false);
        self.window_minimized
            .store(minimized, std::sync::atomic::Ordering::Relaxed);
        if let Some(scale) = self.debug_scale {
            if (ctx.pixels_per_point() - scale).abs() > 1e-4 {
                ctx.set_pixels_per_point(scale);
            }
        }

        // Only once, while the window is still being set up. Re-applying the
        // dark-mode attribute later makes DWM recompute the non-client frame
        // mid-session, which shifts the client rect and leaves an unpainted
        // strip under the top bar (v2.1.0 issue). The caption itself is never
        // visible here, so a stale attribute costs nothing.
        if self.titlebar_dark.is_none() {
            crate::platform::apply_titlebar(self.theme_mode.is_dark());
            self.titlebar_dark = Some(self.theme_mode.is_dark());
        }

        self.window_resize_edges(ctx);

        self.handle_overlay_commands(ctx);
        self.poll_lyrics_candidates();
        if self.lyrics_candidates_rx.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        self.poll_scan();
        self.sync_watch();
        self.poll_watch();
        self.restore_pending_playback(ctx);
        self.poll_audio(ctx);
        self.poll_stats();
        self.poll_thumbnails(ctx);
        self.poll_health();
        self.poll_lyrics_fetch();
        self.poll_cover_fetch(ctx);
        self.poll_media_keys(ctx);
        self.sync_smtc();
        self.handle_shortcuts(ctx);
        self.poll_tray(ctx);
        self.handle_close(ctx);
        if !self.smtc_attempted {
            self.smtc_attempted = true;
            self.smtc = crate::platform::smtc_client::SmtcClient::new();
        }
        self.tick_session();
        if self.sync_overlay() {
            // The overlay windows paint on their own 33 ms timer, but that timer
            // is not a guarantee: wake both viewports explicitly whenever the
            // published snapshot changes so a new track or lyric line can never
            // be left behind on screen.
            if self.lyrics_viewport_active {
                ctx.request_repaint_of(overlay::lyrics_viewport());
            }
            if self.mini_viewport_active {
                ctx.request_repaint_of(overlay::mini_viewport());
            }
        }

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
            self.topbar_panel(ctx);
            self.now_playing(ctx);
            self.equalizer_window(ctx);
            self.settings_window(ctx);
            self.smart_builder_window(ctx);
            self.health_window(ctx);
            self.render_dialog(ctx);
            self.render_toast(ctx);
            self.render_drop_overlay(ctx);
            self.show_overlays(ctx);
            self.show_tray_menu(ctx);
            lyrics_edit::show(self, ctx);
            self.command_palette(ctx);
            self.flush_prefs(ctx);
            self.request_repaint(ctx);
            return;
        }

        let rail_visible = self.now_rail_open
            && self.current.is_some()
            && Self::rail_available(ctx);
        let mut lap = std::time::Instant::now();
        self.player_bar(ctx);
        self.sidebar_panel(ctx);
        self.topbar_panel(ctx);
        let t_chrome = lap.elapsed().as_secs_f64() * 1000.0;
        lap = std::time::Instant::now();
        self.now_rail_panel(ctx);
        let t_rail = lap.elapsed().as_secs_f64() * 1000.0;
        lap = std::time::Instant::now();
        self.central_panel(ctx, &ids, total_seconds);
        let t_list = lap.elapsed().as_secs_f64() * 1000.0;
        lap = std::time::Instant::now();
        if !rail_visible {
            self.queue_window(ctx);
        }
        self.equalizer_window(ctx);
        self.settings_window(ctx);
        self.smart_builder_window(ctx);
        self.health_window(ctx);
        self.render_dialog(ctx);
        self.render_toast(ctx);
        self.render_drop_overlay(ctx);
        self.show_overlays(ctx);
        self.show_tray_menu(ctx);
        lyrics_edit::show(self, ctx);
        self.command_palette(ctx);
        let t_tail = lap.elapsed().as_secs_f64() * 1000.0;
        if let Some(file) = &mut self.section_log {
            use std::io::Write as _;
            let _ = writeln!(
                file,
                "{t_chrome:.2}\t{t_rail:.2}\t{t_list:.2}\t{t_tail:.2}"
            );
        }
        self.flush_prefs(ctx);
        if let Ok(order) = std::env::var("RPLAYER_PAINT_PROBE") {
            // Development aid: paint a magenta bar over the strip under the top
            // bar so a paint-coverage problem can be attributed to a layer order.
            let order = match order.as_str() {
                "background" => egui::Order::Background,
                "middle" => egui::Order::Middle,
                "foreground" => egui::Order::Foreground,
                _ => egui::Order::Tooltip,
            };
            let painter = ctx.layer_painter(egui::LayerId::new(
                order,
                egui::Id::new("rplayer-paint-probe"),
            ));
            let screen = ctx.screen_rect();
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(screen.left(), 50.0),
                    egui::vec2(screen.width(), 16.0),
                ),
                0.0,
                egui::Color32::from_rgb(255, 0, 255),
            );
        }
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
        SortField::Album => a
            .album
            .to_lowercase()
            .cmp(&b.album.to_lowercase())
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
        SortField::Duration => a
            .duration
            .partial_cmp(&b.duration)
            .unwrap_or(Ordering::Equal),
    };
    if sort.ascending {
        ordering
    } else {
        ordering.reverse()
    }
}

fn average_color(image: &image::RgbaImage) -> Color32 {
    let mut red = 0u64;
    let mut green = 0u64;
    let mut blue = 0u64;
    let mut weight = 0u64;
    for pixel in image.pixels() {
        let alpha = u64::from(pixel[3]);
        if alpha < 16 {
            continue;
        }
        // Bias toward the album's saturated colour rather than its black
        // borders/shadows, otherwise the ambient wash turns muddy.
        let max = u16::from(pixel[0].max(pixel[1]).max(pixel[2])).max(1);
        let min = u16::from(pixel[0].min(pixel[1]).min(pixel[2]));
        let saturation = u64::from((max - min) * 100 / max);
        let pixel_weight = alpha * (20 + saturation * 2);
        red += u64::from(pixel[0]) * pixel_weight;
        green += u64::from(pixel[1]) * pixel_weight;
        blue += u64::from(pixel[2]) * pixel_weight;
        weight += pixel_weight;
    }
    if weight == 0 {
        return theme::c().accent_dark;
    }
    Color32::from_rgb(
        (red / weight) as u8,
        (green / weight) as u8,
        (blue / weight) as u8,
    )
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

/// Hash of everything the overlay windows actually draw.
///
/// Playback position is deliberately excluded: lyric lines already encode it,
/// and position ticks every frame, which would defeat change detection.
fn overlay_content_key(state: &OverlayState) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    state.has_track.hash(&mut hasher);
    state.title.hash(&mut hasher);
    state.artist.hash(&mut hasher);
    state.playing.hash(&mut hasher);
    state.favorite.hash(&mut hasher);
    state.lyric_previous.hash(&mut hasher);
    state.lyric_current.hash(&mut hasher);
    state.lyric_next.hash(&mut hasher);
    state.has_lyrics.hash(&mut hasher);
    state.lyrics_opacity.to_bits().hash(&mut hasher);
    state.lyrics_text_opacity.to_bits().hash(&mut hasher);
    state.lyrics_font.to_bits().hash(&mut hasher);
    state.lyrics_locked.hash(&mut hasher);
    state.mini_style.hash(&mut hasher);
    state.mini_on_top.hash(&mut hasher);
    state.mini_snap.hash(&mut hasher);
    state.desktop_lyrics_open.hash(&mut hasher);
    state.mini_open.hash(&mut hasher);
    state.cover_id.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod play_mode_tests {
    use super::PlayMode;

    #[test]
    fn play_mode_cycles_through_five_states() {
        assert_eq!(PlayMode::Sequential.next(), PlayMode::RepeatAll);
        assert_eq!(PlayMode::RepeatAll.next(), PlayMode::RepeatOne);
        assert_eq!(PlayMode::RepeatOne.next(), PlayMode::Shuffle);
        assert_eq!(PlayMode::Shuffle.next(), PlayMode::ShuffleAll);
        assert_eq!(PlayMode::ShuffleAll.next(), PlayMode::Sequential);
        assert!(PlayMode::ShuffleAll.repeats_all());
        assert!(PlayMode::ShuffleAll.is_shuffle());
        assert!(!PlayMode::Shuffle.repeats_all());
    }
}

