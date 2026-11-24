//! Layout and typography tokens from the v2 “Listening Room” design.

use egui::{FontId, Vec2};

pub const SIDEBAR_WIDTH: f32 = 236.0;
pub const TOPBAR_HEIGHT: f32 = 54.0;
pub const PLAYER_HEIGHT: f32 = 92.0;
pub const RAIL_WIDTH: f32 = 300.0;
pub const RAIL_BREAKPOINT: f32 = 1280.0;

pub const WINDOW_DEFAULT: Vec2 = Vec2::new(1440.0, 900.0);
pub const WINDOW_MIN: Vec2 = Vec2::new(1120.0, 720.0);

pub const PAGE_TITLE: f32 = 42.0;
pub const NOW_TITLE: f32 = 60.0;
pub const TRACK_TITLE: f32 = 13.0;
pub const TRACK_ARTIST: f32 = 9.5;
pub const META: f32 = 9.0;
pub const BODY: f32 = 13.0;
pub const LYRIC: f32 = 32.0;
pub const LYRIC_ACTIVE: f32 = 14.0;

pub const RADIUS_CONTROL: u8 = 8;
pub const RADIUS_CARD: u8 = 10;
pub const RADIUS_COVER: u8 = 2;
pub const ROW_HEIGHT: f32 = 56.0;
pub const COVER_ROW: f32 = 38.0;

pub fn serif(size: f32) -> FontId {
    crate::theme::serif(size)
}

pub fn mono(size: f32) -> FontId {
    crate::theme::mono(size)
}
