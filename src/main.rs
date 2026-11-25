//! RPlayer — a fast, local-first desktop music player.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod backup;
mod cover;
mod cover_fetch;
mod cue;
mod duplicates;
mod eq;
mod health;
mod lyrics;
mod lyrics_fetch;
mod m3u;
mod model;
mod overlay;
mod platform;
mod playback;
mod scanner;
mod session;
mod stats;
mod storage;
mod tag_edit;
mod theme;
mod ui;
mod watch;

use std::sync::Arc;

use eframe::egui;

fn main() -> eframe::Result<()> {
    // One player per session: a second launch asks the running copy to come
    // forward and exits, so there is never a second tray icon or a second
    // always-on-top lyrics window.
    #[cfg(target_os = "windows")]
    if !platform::tray::single_instance_guard() {
        platform::tray::request_show_from_other_instance();
        return Ok(());
    }

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).ok();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("RPlayer")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([1120.0, 720.0])
        .with_decorations(false)
        .with_window_level(egui::WindowLevel::Normal)
        // The player window itself is opaque: a per-pixel-alpha swap chain here
        // meant any strip the panels did not paint let the desktop through (the
        // "透明的缝" under the top bar). The floating overlays (lyrics strip,
        // mini player, tray menu) each ask for transparency on their own
        // viewport, so they keep their rounded cards.
        .with_transparent(false);

    if let Some(icon) = icon {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    #[cfg(target_os = "windows")]
    platform::ensure_aumid_shortcut();

    eframe::run_native(
        "RPlayer",
        options,
        Box::new(|cc| Ok(Box::new(app::RPlayerApp::new(cc)))),
    )
}
