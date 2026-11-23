//! RPlayer — a fast, local-first desktop music player.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod eq;
mod lyrics;
mod lyrics_fetch;
mod model;
mod overlay;
mod platform;
mod scanner;
mod storage;
mod theme;
mod ui;

use std::sync::Arc;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).ok();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("RPlayer")
        .with_inner_size([1200.0, 780.0])
        .with_min_inner_size([940.0, 640.0])
        // The GL context needs alpha support for the transparent desktop
        // lyrics overlay, even though the main panels paint every pixel.
        .with_transparent(true);

    if let Some(icon) = icon {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "RPlayer",
        options,
        Box::new(|cc| Ok(Box::new(app::RPlayerApp::new(cc)))),
    )
}
