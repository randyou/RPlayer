//! Runtime visual identity for RPlayer.
//!
//! The palette is resolved through [`c`] instead of compile-time constants so
//! the whole UI — including custom-painted rows, icons and overlays — can switch
//! between the midnight and paper themes without rebuilding widgets.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use egui::epaint::Shadow;
use egui::style::{WidgetVisuals, Widgets};
use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin, Stroke,
    Style, TextStyle, Visuals,
};

/// Light or dark appearance. Kept as a tiny enum so it can be persisted and
/// compared cheaply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

impl ThemeMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub sidebar: Color32,
    pub panel: Color32,
    pub card: Color32,
    pub hover: Color32,
    pub selected: Color32,
    pub border: Color32,
    pub border_soft: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_dark: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub shadow: Color32,
}

pub const DARK: Palette = Palette {
    dark: true,
    bg: Color32::from_rgb(0x0B, 0x0D, 0x12),
    sidebar: Color32::from_rgb(0x0F, 0x11, 0x17),
    panel: Color32::from_rgb(0x13, 0x16, 0x1D),
    card: Color32::from_rgb(0x17, 0x1A, 0x22),
    hover: Color32::from_rgb(0x1D, 0x22, 0x2C),
    selected: Color32::from_rgb(0x1D, 0x2B, 0x24),
    border: Color32::from_rgb(0x3A, 0x42, 0x50),
    border_soft: Color32::from_rgb(0x2D, 0x34, 0x42),
    text: Color32::from_rgb(0xF4, 0xF6, 0xFA),
    muted: Color32::from_rgb(0x8B, 0x93, 0xA7),
    faint: Color32::from_rgb(0x5D, 0x65, 0x78),
    accent: Color32::from_rgb(0x8B, 0xFF, 0xB2),
    accent_dark: Color32::from_rgb(0x2E, 0x6B, 0x48),
    on_accent: Color32::from_rgb(0x06, 0x17, 0x0F),
    danger: Color32::from_rgb(0xFF, 0x7A, 0x8A),
    shadow: Color32::from_black_alpha(150),
};

pub const SAND: Palette = Palette {
    dark: false,
    bg: Color32::from_rgb(0xF6, 0xF1, 0xE9),
    sidebar: Color32::from_rgb(0xFC, 0xF8, 0xF1),
    panel: Color32::from_rgb(0xFF, 0xFD, 0xF8),
    card: Color32::from_rgb(0xF1, 0xE8, 0xDA),
    hover: Color32::from_rgb(0xEA, 0xDF, 0xCC),
    selected: Color32::from_rgb(0xF7, 0xE8, 0xC8),
    border: Color32::from_rgb(0xE8, 0xDC, 0xCB),
    border_soft: Color32::from_rgb(0xF2, 0xE9, 0xDB),
    text: Color32::from_rgb(0x24, 0x1C, 0x12),
    muted: Color32::from_rgb(0x5A, 0x4C, 0x3C),
    faint: Color32::from_rgb(0x78, 0x69, 0x57),
    accent: Color32::from_rgb(0xC4, 0x7A, 0x2C),
    accent_dark: Color32::from_rgb(0x8F, 0x54, 0x1A),
    on_accent: Color32::from_rgb(0xFF, 0xF8, 0xED),
    danger: Color32::from_rgb(0xD8, 0x4A, 0x3D),
    shadow: Color32::from_black_alpha(38),
};

pub const MIST: Palette = Palette {
    dark: false,
    bg: Color32::from_rgb(0xEE, 0xF4, 0xF8),
    sidebar: Color32::from_rgb(0xF8, 0xFB, 0xFD),
    panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    card: Color32::from_rgb(0xE6, 0xF0, 0xF6),
    hover: Color32::from_rgb(0xDC, 0xEA, 0xF2),
    selected: Color32::from_rgb(0xD9, 0xED, 0xF5),
    border: Color32::from_rgb(0xD9, 0xE6, 0xEF),
    border_soft: Color32::from_rgb(0xEB, 0xF2, 0xF7),
    text: Color32::from_rgb(0x12, 0x20, 0x29),
    muted: Color32::from_rgb(0x4D, 0x63, 0x70),
    faint: Color32::from_rgb(0x5E, 0x74, 0x82),
    accent: Color32::from_rgb(0x2A, 0x86, 0xB8),
    accent_dark: Color32::from_rgb(0x1B, 0x5F, 0x84),
    on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    danger: Color32::from_rgb(0xD8, 0x4A, 0x5D),
    shadow: Color32::from_black_alpha(38),
};

pub const MIDNIGHT: Palette = Palette {
    dark: true,
    bg: Color32::from_rgb(0x0A, 0x10, 0x1C),
    sidebar: Color32::from_rgb(0x0D, 0x15, 0x24),
    panel: Color32::from_rgb(0x10, 0x1A, 0x2B),
    card: Color32::from_rgb(0x14, 0x21, 0x36),
    hover: Color32::from_rgb(0x1B, 0x2A, 0x43),
    selected: Color32::from_rgb(0x17, 0x32, 0x4A),
    border: Color32::from_rgb(0x3A, 0x4F, 0x6E),
    border_soft: Color32::from_rgb(0x2B, 0x3E, 0x58),
    text: Color32::from_rgb(0xF2, 0xF7, 0xFF),
    muted: Color32::from_rgb(0x8F, 0xA5, 0xC0),
    faint: Color32::from_rgb(0x61, 0x7A, 0x96),
    accent: Color32::from_rgb(0x7C, 0xC7, 0xFF),
    accent_dark: Color32::from_rgb(0x2E, 0x6A, 0x96),
    on_accent: Color32::from_rgb(0x06, 0x12, 0x1C),
    danger: Color32::from_rgb(0xFF, 0x7A, 0x8A),
    shadow: Color32::from_black_alpha(150),
};

pub const CHARCOAL: Palette = Palette {
    dark: true,
    bg: Color32::from_rgb(0x11, 0x11, 0x11),
    sidebar: Color32::from_rgb(0x17, 0x17, 0x17),
    panel: Color32::from_rgb(0x1B, 0x1B, 0x1B),
    card: Color32::from_rgb(0x22, 0x22, 0x22),
    hover: Color32::from_rgb(0x2B, 0x2B, 0x2B),
    selected: Color32::from_rgb(0x26, 0x30, 0x26),
    border: Color32::from_rgb(0x4A, 0x4A, 0x4A),
    border_soft: Color32::from_rgb(0x3C, 0x3C, 0x3C),
    text: Color32::from_rgb(0xF2, 0xF2, 0xF2),
    muted: Color32::from_rgb(0x9A, 0x9A, 0x9A),
    faint: Color32::from_rgb(0x96, 0x96, 0x96),
    accent: Color32::from_rgb(0xA8, 0xC7, 0xA1),
    accent_dark: Color32::from_rgb(0x58, 0x7A, 0x52),
    on_accent: Color32::from_rgb(0x0B, 0x12, 0x0A),
    danger: Color32::from_rgb(0xFF, 0x7A, 0x8A),
    shadow: Color32::from_black_alpha(150),
};

pub const LIGHT: Palette = Palette {
    dark: false,
    bg: Color32::from_rgb(0xF3, 0xF5, 0xF1),
    sidebar: Color32::from_rgb(0xFB, 0xFC, 0xF9),
    panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    card: Color32::from_rgb(0xEE, 0xF2, 0xEB),
    hover: Color32::from_rgb(0xE5, 0xEB, 0xE1),
    selected: Color32::from_rgb(0xDC, 0xF3, 0xE4),
    border: Color32::from_rgb(0xDF, 0xE6, 0xDA),
    border_soft: Color32::from_rgb(0xEE, 0xF2, 0xE9),
    text: Color32::from_rgb(0x13, 0x1C, 0x16),
    muted: Color32::from_rgb(0x4F, 0x5B, 0x53),
    faint: Color32::from_rgb(0x66, 0x73, 0x6A),
    accent: Color32::from_rgb(0x2A, 0xA5, 0x62),
    accent_dark: Color32::from_rgb(0x1B, 0x7A, 0x49),
    on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    danger: Color32::from_rgb(0xD8, 0x4A, 0x5D),
    shadow: Color32::from_black_alpha(38),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreset {
    Paper,
    Sand,
    Mist,
    #[default]
    Forest,
    Midnight,
    Charcoal,
}

impl ThemePreset {
    pub const ALL: [Self; 6] = [
        Self::Paper,
        Self::Sand,
        Self::Mist,
        Self::Forest,
        Self::Midnight,
        Self::Charcoal,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Paper => "paper",
            Self::Sand => "sand",
            Self::Mist => "mist",
            Self::Forest => "forest",
            Self::Midnight => "midnight",
            Self::Charcoal => "charcoal",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Paper => "纸白",
            Self::Sand => "暖沙",
            Self::Mist => "雾蓝",
            Self::Forest => "墨绿",
            Self::Midnight => "午夜",
            Self::Charcoal => "炭黑",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|preset| preset.id() == id)
    }

    pub fn default_for_mode(mode: ThemeMode) -> Self {
        if mode.is_dark() {
            Self::Forest
        } else {
            Self::Paper
        }
    }

    pub fn mode(self) -> ThemeMode {
        if self.is_dark() {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, Self::Forest | Self::Midnight | Self::Charcoal)
    }

    pub fn palette(self) -> &'static Palette {
        match self {
            Self::Paper => &LIGHT,
            Self::Sand => &SAND,
            Self::Mist => &MIST,
            Self::Forest => &DARK,
            Self::Midnight => &MIDNIGHT,
            Self::Charcoal => &CHARCOAL,
        }
    }

    /// `Ctrl+T` keeps the current family: Paper↔Forest, Sand↔Charcoal,
    /// Mist↔Midnight.
    pub fn toggle_mode(self) -> Self {
        match self {
            Self::Paper => Self::Forest,
            Self::Forest => Self::Paper,
            Self::Sand => Self::Charcoal,
            Self::Charcoal => Self::Sand,
            Self::Mist => Self::Midnight,
            Self::Midnight => Self::Mist,
        }
    }
}

static PRESET: AtomicU8 = AtomicU8::new(3);
static FONTS_INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn preset() -> ThemePreset {
    match PRESET.load(Ordering::Relaxed) {
        0 => ThemePreset::Paper,
        1 => ThemePreset::Sand,
        2 => ThemePreset::Mist,
        3 => ThemePreset::Forest,
        4 => ThemePreset::Midnight,
        _ => ThemePreset::Charcoal,
    }
}

pub fn set_preset(preset: ThemePreset) {
    let index = match preset {
        ThemePreset::Paper => 0,
        ThemePreset::Sand => 1,
        ThemePreset::Mist => 2,
        ThemePreset::Forest => 3,
        ThemePreset::Midnight => 4,
        ThemePreset::Charcoal => 5,
    };
    PRESET.store(index, Ordering::Relaxed);
}

pub fn mode() -> ThemeMode {
    preset().mode()
}

pub fn set_mode(mode: ThemeMode) {
    set_preset(ThemePreset::default_for_mode(mode));
}

/// The active palette. Every custom-painted colour goes through this.
pub fn c() -> &'static Palette {
    preset().palette()
}

/// Translucent accent colour. `egui` meshes are composited from
/// premultiplied colours, so compute the premultiplied RGB ourselves instead
/// of handing raw RGB with a low alpha to the painter.
pub fn divider() -> Color32 {
    let palette = c();
    palette.bg.lerp_to_gamma(palette.border_soft, 0.10)
}

pub fn accent_alpha(alpha: f32) -> Color32 {
    let alpha = alpha.clamp(0.0, 1.0);
    let a = (alpha * 255.0).round() as u32;
    let accent = c().accent;
    Color32::from_rgba_premultiplied(
        (accent.r() as u32 * a / 255) as u8,
        (accent.g() as u32 * a / 255) as u8,
        (accent.b() as u32 * a / 255) as u8,
        a as u8,
    )
}

pub fn white_alpha(alpha: f32) -> Color32 {
    let alpha = alpha.clamp(0.0, 1.0);
    let a = (alpha * 255.0).round() as u32;
    Color32::from_rgba_premultiplied(a as u8, a as u8, a as u8, a as u8)
}

pub fn black_alpha(alpha: f32) -> Color32 {
    let alpha = alpha.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(0, 0, 0, (alpha * 255.0).round() as u8)
}

/// Installs fonts once and applies the style for `mode`.
pub fn install(ctx: &egui::Context, mode: ThemeMode) {
    install_preset(ctx, ThemePreset::default_for_mode(mode));
}

/// Installs fonts once and applies a specific preset.
pub fn install_preset(ctx: &egui::Context, preset: ThemePreset) {
    set_preset(preset);
    if !FONTS_INSTALLED.swap(true, Ordering::Relaxed) {
        install_fonts(ctx);
    }
    apply_style(ctx);
}

/// Rebuilds the egui style from the active palette. Call after a live toggle.
pub fn apply_style(ctx: &egui::Context) {
    let palette = *c();
    let mut style = Style::default();
    style.visuals = visuals_for(&palette);
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(16.0, 9.0);
    style.spacing.interact_size = egui::vec2(40.0, 36.0);
    style.spacing.menu_margin = Margin::same(6);
    let mut scroll = egui::style::ScrollStyle::floating();
    scroll.foreground_color = false;
    scroll.bar_width = 9.0;
    scroll.floating_width = 4.0;
    scroll.dormant_background_opacity = 0.0;
    style.spacing.scroll = scroll;
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

fn visuals_for(palette: &Palette) -> Visuals {
    let mut visuals = if palette.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    visuals.dark_mode = palette.dark;
    visuals.panel_fill = palette.bg;
    visuals.window_fill = palette.panel;
    visuals.extreme_bg_color = palette.bg;
    visuals.faint_bg_color = palette.card;
    visuals.code_bg_color = palette.panel;
    visuals.window_corner_radius = CornerRadius::same(16);
    visuals.menu_corner_radius = CornerRadius::same(12);
    visuals.window_stroke = Stroke::new(1.0_f32, palette.border);
    visuals.window_shadow = Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: palette.shadow,
    };
    visuals.popup_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: palette.shadow,
    };
    visuals.selection = egui::style::Selection {
        bg_fill: accent_alpha(0.20),
        stroke: Stroke::new(1.0_f32, palette.accent),
    };
    visuals.hyperlink_color = palette.accent;
    visuals.warn_fg_color = Color32::from_rgb(0xC9, 0x8A, 0x22);
    visuals.error_fg_color = palette.danger;
    visuals.slider_trailing_fill = true;

    let mut widgets = if palette.dark {
        Widgets::dark()
    } else {
        Widgets::light()
    };
    let radius = CornerRadius::same(10);

    widgets.noninteractive = WidgetVisuals {
        bg_fill: palette.panel,
        weak_bg_fill: palette.panel,
        bg_stroke: Stroke::new(
            1.0_f32,
            palette.bg.lerp_to_gamma(palette.border_soft, 0.10),
        ),
        corner_radius: radius,
        fg_stroke: Stroke::new(1.0_f32, palette.text),
        expansion: 0.0,
    };
    widgets.inactive = WidgetVisuals {
        bg_fill: palette.card,
        weak_bg_fill: palette.card,
        bg_stroke: Stroke::new(1.0_f32, palette.border),
        corner_radius: radius,
        fg_stroke: Stroke::new(1.0_f32, palette.text),
        expansion: 0.0,
    };
    widgets.hovered = WidgetVisuals {
        bg_fill: palette.hover,
        weak_bg_fill: palette.hover,
        bg_stroke: Stroke::new(1.0_f32, palette.border),
        corner_radius: radius,
        fg_stroke: Stroke::new(1.0_f32, palette.text),
        expansion: 0.0,
    };
    widgets.active = WidgetVisuals {
        bg_fill: palette.selected,
        weak_bg_fill: palette.selected,
        bg_stroke: Stroke::new(1.0_f32, palette.accent_dark),
        corner_radius: radius,
        fg_stroke: Stroke::new(1.0_f32, palette.text),
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
        .fill(c().panel)
        .stroke(Stroke::new(1.0_f32, c().border))
        .corner_radius(CornerRadius::same(18))
        .inner_margin(Margin::same(18))
}

pub fn card_frame() -> Frame {
    Frame::NONE
        .fill(c().card)
        .stroke(Stroke::new(1.0_f32, c().border_soft))
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
            if selected {
                accent_alpha(0.45)
            } else {
                c().border
            },
        ))
        .corner_radius(CornerRadius::same(99))
        .inner_margin(Margin::symmetric(12, 5))
}
