//! Visual identity for RPlayer: deep midnight surfaces, a single mint accent
//! and a bundled-CJK-friendly font stack.

use std::sync::Arc;

use egui::epaint::Shadow;
use egui::style::{WidgetVisuals, Widgets};
use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin, Stroke,
    Style, TextStyle, Visuals,
};

pub const BG: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x12);
pub const SIDEBAR: Color32 = Color32::from_rgb(0x0F, 0x11, 0x17);
pub const PANEL: Color32 = Color32::from_rgb(0x13, 0x16, 0x1D);
pub const CARD: Color32 = Color32::from_rgb(0x17, 0x1A, 0x22);
pub const HOVER: Color32 = Color32::from_rgb(0x1D, 0x22, 0x2C);
pub const SELECTED: Color32 = Color32::from_rgb(0x20, 0x2A, 0x2A);
pub const BORDER: Color32 = Color32::from_rgb(0x26, 0x2B, 0x36);
pub const BORDER_SOFT: Color32 = Color32::from_rgb(0x1E, 0x23, 0x2E);
pub const TEXT: Color32 = Color32::from_rgb(0xF4, 0xF6, 0xFA);
pub const MUTED: Color32 = Color32::from_rgb(0x8B, 0x93, 0xA7);
pub const FAINT: Color32 = Color32::from_rgb(0x5D, 0x65, 0x78);
pub const ACCENT: Color32 = Color32::from_rgb(0x8B, 0xFF, 0xB2);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0x2E, 0x6B, 0x48);
pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x7A, 0x8A);

/// Translucent accent colour. `egui` meshes are composited from
/// premultiplied colours, so compute the premultiplied RGB ourselves instead
/// of handing raw RGB with a low alpha to the painter (which would otherwise
/// make every "12% accent" surface look almost solid).
pub fn accent_alpha(alpha: f32) -> Color32 {
    let alpha = alpha.clamp(0.0, 1.0);
    let a = (alpha * 255.0).round() as u32;
    Color32::from_rgba_premultiplied(
        (ACCENT.r() as u32 * a / 255) as u8,
        (ACCENT.g() as u32 * a / 255) as u8,
        (ACCENT.b() as u32 * a / 255) as u8,
        a as u8,
    )
}

pub fn white_alpha(alpha: f32) -> Color32 {
    let alpha = alpha.clamp(0.0, 1.0);
    let a = (alpha * 255.0).round() as u32;
    let value = (255 * a / 255) as u8;
    Color32::from_rgba_premultiplied(value, value, value, a as u8)
}

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);

    let mut style = Style::default();
    style.visuals = dark_visuals();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.interact_size = egui::vec2(38.0, 34.0);
    style.spacing.menu_margin = Margin::same(6);
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(24.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        ),
    ]
    .into();

    ctx.set_style(style);
}

fn dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = Color32::from_rgb(0x0A, 0x0C, 0x11);
    visuals.faint_bg_color = CARD;
    visuals.code_bg_color = PANEL;
    visuals.window_corner_radius = CornerRadius::same(16);
    visuals.menu_corner_radius = CornerRadius::same(12);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.window_shadow = Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    visuals.popup_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(120),
    };
    visuals.selection = egui::style::Selection {
        bg_fill: accent_alpha(0.20),
        stroke: Stroke::new(1.0_f32, ACCENT),
    };
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = Color32::from_rgb(0xFF, 0xC7, 0x6E);
    visuals.error_fg_color = DANGER;
    visuals.slider_trailing_fill = true;

    let mut widgets = Widgets::dark();
    let radius = CornerRadius::same(10);

    widgets.noninteractive = WidgetVisuals {
        bg_fill: PANEL,
        weak_bg_fill: PANEL,
        bg_stroke: Stroke::new(1.0_f32, BORDER_SOFT),
        corner_radius: radius,
        fg_stroke: Stroke::new(1.0_f32, TEXT),
        expansion: 0.0,
    };
    widgets.inactive = WidgetVisuals {
        bg_fill: CARD,
        weak_bg_fill: CARD,
        bg_stroke: Stroke::new(1.0_f32, BORDER),
        corner_radius: radius,
        fg_stroke: Stroke::new(1.0_f32, TEXT),
        expansion: 0.0,
    };
    widgets.hovered = WidgetVisuals {
        bg_fill: HOVER,
        weak_bg_fill: HOVER,
        bg_stroke: Stroke::new(1.0_f32, Color32::from_rgb(0x33, 0x3B, 0x49)),
        corner_radius: radius,
        fg_stroke: Stroke::new(1.0_f32, Color32::WHITE),
        expansion: 0.0,
    };
    widgets.active = WidgetVisuals {
        bg_fill: SELECTED,
        weak_bg_fill: SELECTED,
        bg_stroke: Stroke::new(1.0_f32, ACCENT_DARK),
        corner_radius: radius,
        fg_stroke: Stroke::new(1.0_f32, TEXT),
        expansion: 0.0,
    };
    widgets.open = widgets.active;
    visuals.widgets = widgets;
    visuals
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let mut candidates: Vec<(String, u32)> = Vec::new();
    if let Ok(windir) = std::env::var("WINDIR") {
        candidates.push((format!("{windir}\\Fonts\\msyh.ttc"), 0));
        candidates.push((format!("{windir}\\Fonts\\msyhbd.ttc"), 0));
        candidates.push((format!("{windir}\\Fonts\\simhei.ttf"), 0));
    }
    candidates.extend([
        ("C:/Windows/Fonts/msyh.ttc".to_string(), 0),
        ("C:/Windows/Fonts/msyhbd.ttc".to_string(), 0),
        ("/System/Library/Fonts/PingFang.ttc".to_string(), 0),
        ("/System/Library/Fonts/Hiragino Sans GB.ttc".to_string(), 0),
        ("/Library/Fonts/Arial Unicode.ttf".to_string(), 0),
        (
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".to_string(),
            0,
        ),
        (
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc".to_string(),
            0,
        ),
        (
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc".to_string(),
            0,
        ),
    ]);

    for (path, index) in candidates {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let mut data = FontData::from_owned(bytes);
        data.index = index;
        fonts
            .font_data
            .insert("rplayer-cjk".to_owned(), Arc::new(data));
        // Append as a fallback so Latin text keeps egui's crisp default face.
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push("rplayer-cjk".to_owned());
        }
        break;
    }

    ctx.set_fonts(fonts);
}

/// Floating panel used for dialogs and the player bar.
pub fn panel_frame() -> Frame {
    Frame::NONE
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(18))
        .inner_margin(Margin::same(18))
}

pub fn card_frame() -> Frame {
    Frame::NONE
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, BORDER_SOFT))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(14))
}

pub fn pill_frame(selected: bool) -> Frame {
    Frame::NONE
        .fill(if selected {
            accent_alpha(0.12)
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::new(
            1.0_f32,
            if selected { accent_alpha(0.45) } else { BORDER },
        ))
        .corner_radius(CornerRadius::same(99))
        .inner_margin(Margin::symmetric(12, 5))
}
