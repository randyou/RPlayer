//! Always-on-top desktop lyrics and mini player viewports.
//!
//! The main app owns playback; these windows are pure views. They publish a
//! snapshot through a mutex and send tiny commands back through a queue.

use std::sync::Arc;

use eframe::egui;
use egui::{Align2, Color32, ColorImage, FontId, Pos2, TextureHandle, Vec2, ViewportId};


pub(super) const LYRICS_SIZE: Vec2 = Vec2::new(860.0, 158.0);

pub(super) const MINI_SIZE: Vec2 = Vec2::new(410.0, 146.0);

#[derive(Clone, Default)]
pub struct OverlayState {
    pub has_track: bool,
    pub title: String,
    pub artist: String,
    /// Album track number, shown as `艺术家 · 编号` on the mini player.
    pub track_number: Option<u32>,
    pub position: f64,
    pub duration: f64,
    pub playing: bool,
    pub favorite: bool,
    pub lyric_previous: String,
    pub lyric_current: String,
    pub lyric_next: String,
    pub has_lyrics: bool,
    pub lyrics_opacity: f32,
    pub lyrics_text_opacity: f32,
    pub lyrics_font: f32,
    pub lyrics_locked: bool,
    pub mini_style: String,
    pub mini_on_top: bool,
    pub mini_snap: bool,
    pub desktop_lyrics_open: bool,
    /// Mini player viewport is up; the tray menu mirrors it as a check mark.
    pub mini_open: bool,
    pub cover_id: Option<u64>,
    pub cover: Option<Arc<ColorImage>>,
}

#[derive(Debug, Clone, Copy)]
pub enum OverlayCommand {
    TogglePlay,
    Previous,
    Next,
    ToggleFavorite,
    Seek(f64),
    SetLyricsOpacity(f32),
    SetLyricsTextOpacity(f32),
    SetLyricsLocked(bool),
    SetLyricsPosition(f32, f32),
    ToggleLyrics,
    SetMiniOnTop(bool),
    SetMiniPosition(f32, f32),
    CloseLyrics,
    CloseMini,
    OpenMain,
    OpenLyricsSettings,
    FetchLyrics,
}

pub fn lyrics_viewport() -> ViewportId {
    ViewportId::from_hash_of("rplayer-desktop-lyrics")
}

pub fn mini_viewport() -> ViewportId {
    ViewportId::from_hash_of("rplayer-mini-player")
}

#[derive(Default)]
pub struct MiniCache {
    cover_id: Option<u64>,
    texture: Option<TextureHandle>,
}


mod lyrics;
mod mini;

pub use lyrics::show_lyrics;
pub use mini::show_mini;

pub(super) fn shadow_text(
    painter: &egui::Painter,
    pos: Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    shadow: Color32,
    alpha: f32,
) {
    // Approximate text-shadow 0 0 1px with an eight-way halo.
    for (dx, dy) in [
        (-0.75, 0.0),
        (0.75, 0.0),
        (0.0, -0.75),
        (0.0, 0.75),
        (-0.55, -0.55),
        (0.55, -0.55),
        (-0.55, 0.55),
        (0.55, 0.55),
    ] {
        painter.text(
            pos + Vec2::new(dx, dy),
            Align2::CENTER_CENTER,
            text,
            font.clone(),
            with_alpha(shadow, alpha),
        );
    }
    painter.text(
        pos,
        Align2::CENTER_CENTER,
        text,
        font,
        with_alpha(color, alpha),
    );
}

pub(super) fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (color.a() as f32 * alpha.clamp(0.0, 1.0)) as u8,
    )
}
