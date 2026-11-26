//! Layout and typography tokens from the v2 “Listening Room” design.

use egui::FontId;

pub const TOPBAR_HEIGHT: f32 = 54.0;
pub const PLAYER_HEIGHT: f32 = 92.0;



pub const RADIUS_CONTROL: u8 = 8;
pub const RADIUS_CARD: u8 = 10;

pub fn serif(size: f32) -> FontId {
    crate::theme::serif(size)
}

pub fn mono(size: f32) -> FontId {
    crate::theme::mono(size)
}
