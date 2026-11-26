//! Runtime visual identity for RPlayer.
//!
//! The palette is resolved through [`c`] instead of compile-time constants so
//! the whole UI — including custom-painted rows, icons and overlays — can switch
//! between the midnight and paper themes without rebuilding widgets.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::epaint::Shadow;
use egui::style::{WidgetVisuals, Widgets};
use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Visuals,
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
    bg: Color32::from_rgb(0x0F, 0x12, 0x0F),
    sidebar: Color32::from_rgb(0x15, 0x19, 0x14),
    panel: Color32::from_rgb(0x15, 0x19, 0x14),
    card: Color32::from_rgb(0x11, 0x15, 0x11),
    hover: Color32::from_rgb(0x1A, 0x1F, 0x19),
    selected: Color32::from_rgb(0x23, 0x33, 0x28),
    border: Color32::from_rgb(0x38, 0x3B, 0x34),
    border_soft: Color32::from_rgb(0x23, 0x28, 0x22),
    text: Color32::from_rgb(0xF0, 0xE9, 0xDA),
    muted: Color32::from_rgb(0xB0, 0xA5, 0x94),
    faint: Color32::from_rgb(0x7F, 0x76, 0x68),
    accent: Color32::from_rgb(0x8F, 0xB7, 0x9A),
    accent_dark: Color32::from_rgb(0xA6, 0xC8, 0xAE),
    on_accent: Color32::from_rgb(0x0C, 0x17, 0x10),
    danger: Color32::from_rgb(0xE0, 0x7A, 0x5A),
    shadow: Color32::from_black_alpha(180),
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
    bg: Color32::from_rgb(0xF4, 0xEF, 0xE5),
    sidebar: Color32::from_rgb(0xFA, 0xF6, 0xED),
    panel: Color32::from_rgb(0xFA, 0xF6, 0xED),
    card: Color32::from_rgb(0xEF, 0xE8, 0xDB),
    hover: Color32::from_rgb(0xE9, 0xE2, 0xD5),
    selected: Color32::from_rgb(0xE2, 0xEA, 0xDF),
    border: Color32::from_rgb(0xDC, 0xD5, 0xC7),
    border_soft: Color32::from_rgb(0xE8, 0xE1, 0xD4),
    text: Color32::from_rgb(0x1D, 0x1B, 0x16),
    muted: Color32::from_rgb(0x6B, 0x64, 0x57),
    faint: Color32::from_rgb(0x93, 0x89, 0x75),
    accent: Color32::from_rgb(0x2E, 0x5B, 0x44),
    accent_dark: Color32::from_rgb(0x25, 0x4A, 0x37),
    on_accent: Color32::from_rgb(0xF8, 0xF3, 0xE8),
    danger: Color32::from_rgb(0xB6, 0x4B, 0x31),
    shadow: Color32::from_black_alpha(38),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreset {
    #[default]
    Paper,
    Sand,
    Mist,
    Forest,
    Midnight,
    Charcoal,
}

impl ThemePreset {

    /// The only two themes exposed by the v2 design.
    pub const VISIBLE: [Self; 2] = [Self::Paper, Self::Forest];

    pub fn id(self) -> &'static str {
        match self {
            Self::Paper => "paper",
            Self::Sand => "sand",
            Self::Mist => "mist",
            Self::Forest => "nocturne",
            Self::Midnight => "midnight",
            Self::Charcoal => "charcoal",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Paper => "纸张",
            Self::Sand => "暖沙",
            Self::Mist => "雾蓝",
            Self::Forest => "夜幕",
            Self::Midnight => "午夜",
            Self::Charcoal => "炭黑",
        }
    }

    /// Legacy ids are normalized so an existing profile keeps its appearance
    /// family after the v2 palette consolidation.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "paper" | "sand" | "mist" => Some(Self::Paper),
            "nocturne" | "forest" | "midnight" | "charcoal" => Some(Self::Forest),
            _ => None,
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

static PRESET: AtomicU8 = AtomicU8::new(0);
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

fn preset_index(preset: ThemePreset) -> u8 {
    match preset {
        ThemePreset::Paper => 0,
        ThemePreset::Sand => 1,
        ThemePreset::Mist => 2,
        ThemePreset::Forest => 3,
        ThemePreset::Midnight => 4,
        ThemePreset::Charcoal => 5,
    }
}

/// How long a theme swap takes to blend from one palette to the other.
pub const TRANSITION: Duration = Duration::from_millis(320);

#[derive(Clone, Copy)]
struct Transition {
    from: Palette,
    from_preset: ThemePreset,
    to: ThemePreset,
    start: Instant,
}

impl Transition {
    fn elapsed(self) -> f32 {
        (self.start.elapsed().as_secs_f32() / TRANSITION.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Eased 0..1 progress of the swap.
    fn progress(self) -> f32 {
        ease(self.elapsed())
    }
}

thread_local! {
    static TRANSITION_STATE: Cell<Option<Transition>> = const { Cell::new(None) };
}

/// Switches the preset and starts a short colour blend. Callers should keep
/// repainting until [`transition_active`] turns false.
pub fn set_preset(preset: ThemePreset) {
    let next = preset_index(preset);
    if next == PRESET.load(Ordering::Relaxed) && !transition_active() {
        return;
    }
    let from = c();
    let from_preset = self::preset();
    PRESET.store(next, Ordering::Relaxed);
    TRANSITION_STATE.with(|slot| {
        slot.set(Some(Transition {
            from,
            from_preset,
            to: preset,
            start: Instant::now(),
        }))
    });
}

/// Applies a preset without animating. Used while the window is being built.
pub fn set_preset_instant(preset: ThemePreset) {
    PRESET.store(preset_index(preset), Ordering::Relaxed);
    TRANSITION_STATE.with(|slot| slot.set(None));
}

/// True while a theme swap is still blending; the frame loop should request
/// another repaint and refresh the egui style.
pub fn transition_active() -> bool {
    TRANSITION_STATE.with(|slot| match slot.get() {
        Some(t) => t.elapsed() < 1.0,
        None => false,
    })
}

fn current_transition() -> Option<Transition> {
    TRANSITION_STATE.with(|slot| slot.get()).and_then(|t| {
        if t.elapsed() < 1.0 {
            Some(t)
        } else {
            TRANSITION_STATE.with(|slot| slot.set(None));
            None
        }
    })
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn mix_channel(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round().clamp(0.0, 255.0) as u8
}

/// Blends two colours in premultiplied space; used for the theme cross-fade.
pub fn mix_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        mix_channel(a.r(), b.r(), t),
        mix_channel(a.g(), b.g(), t),
        mix_channel(a.b(), b.b(), t),
        mix_channel(a.a(), b.a(), t),
    )
}

fn blend_palette(from: &Palette, to: &Palette, t: f32) -> Palette {
    Palette {
        dark: if t < 0.5 { from.dark } else { to.dark },
        bg: mix_color(from.bg, to.bg, t),
        sidebar: mix_color(from.sidebar, to.sidebar, t),
        panel: mix_color(from.panel, to.panel, t),
        card: mix_color(from.card, to.card, t),
        hover: mix_color(from.hover, to.hover, t),
        selected: mix_color(from.selected, to.selected, t),
        border: mix_color(from.border, to.border, t),
        border_soft: mix_color(from.border_soft, to.border_soft, t),
        text: mix_color(from.text, to.text, t),
        muted: mix_color(from.muted, to.muted, t),
        faint: mix_color(from.faint, to.faint, t),
        accent: mix_color(from.accent, to.accent, t),
        accent_dark: mix_color(from.accent_dark, to.accent_dark, t),
        on_accent: mix_color(from.on_accent, to.on_accent, t),
        danger: mix_color(from.danger, to.danger, t),
        shadow: mix_color(from.shadow, to.shadow, t),
    }
}

/// The active palette. Every custom-painted colour goes through this.
///
/// While a theme swap is running this returns the blended palette, so any
/// surface that repaints during the 320 ms window fades with the rest of the
/// UI instead of snapping.
pub fn c() -> Palette {
    if let Some(t) = current_transition() {
        return blend_palette(&t.from, t.to.palette(), t.progress());
    }
    *preset().palette()
}

/// Text and glyphs drawn straight onto cover art. Media is unpredictable, so
/// this stays light in both themes instead of following the palette.
pub const ON_MEDIA: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);

/// Chroma key painted behind the lyric strip. Never themed: the panel window
/// below it keys this exact colour out.
pub const COLOR_KEY: Color32 = Color32::from_rgb(0xA0, 0xA0, 0xA0);

/// Primary action tints. Hover and press brighten the accent instead of
/// following the palette, and the disabled disc stays neutral.
pub const ACCENT_PULSE: Color32 = Color32::from_rgb(0x7A, 0xF0, 0xA1);
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0x79, 0xEF, 0xA2);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0xA6, 0xFF, 0xC5);
pub const BUTTON_DISABLED: Color32 = Color32::from_rgb(0x22, 0x26, 0x30);

/// Neutral wash layered over dark chrome for hover and pressed states.
pub fn wash(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, alpha)
}

/// Scrim painted behind modal dialogs and toasts.
pub fn scrim(alpha: u8) -> Color32 {
    Color32::from_black_alpha(alpha)
}

/// Sleeve R brand colours, mirroring `design/rplayer-v2/brand/brand.css`.
///
/// The identity is fixed: it must not move when the UI theme changes, so these
/// values are deliberately independent of [`Palette`].
pub mod brand {
    use eframe::egui::Color32;

    /// `--ink`: type on paper.
    pub const INK: Color32 = Color32::from_rgb(0x1D, 0x1B, 0x16);
    /// `--muted`: labels on paper.
    pub const MUTED: Color32 = Color32::from_rgb(0x6B, 0x64, 0x57);
    /// `--paper`: the default sheet.
    pub const PAPER: Color32 = Color32::from_rgb(0xF4, 0xEF, 0xE5);
    /// `--forest`: the mark tile and the only brand fill.
    pub const FOREST: Color32 = Color32::from_rgb(0x2E, 0x5B, 0x44);
    /// `--nocturne`: the dark canvas.
    pub const NOCTURNE: Color32 = Color32::from_rgb(0x0F, 0x12, 0x0F);
    /// Warm paper tint used on the nocturne canvas.
    pub const CREAM: Color32 = Color32::from_rgb(0xF0, 0xE9, 0xDA);
    /// Sage highlight that reads on forest.
    pub const SAGE: Color32 = Color32::from_rgb(0x8F, 0xB7, 0x9A);
    /// Muted type on the nocturne canvas (no brand.css equivalent: it is the
    /// nocturne counterpart of `--muted`).
    pub const ON_DARK_MUTED: Color32 = Color32::from_rgb(0xB0, 0xA5, 0x94);
}

/// Gradient stops and ink used by the expanded now-playing scene. The scene is
/// artwork rather than chrome, so it keeps its own fixed ramp.
pub mod scene {
    use eframe::egui::Color32;

    pub const TOP: Color32 = Color32::from_rgb(0x0C, 0x14, 0x11);
    pub const BOTTOM: Color32 = Color32::from_rgb(0x07, 0x0A, 0x09);
    pub const HEADLINE: Color32 = Color32::from_rgb(0xFA, 0xFC, 0xF8);
    pub const BODY: Color32 = Color32::from_rgb(0xEA, 0xF0, 0xE8);
    pub const SURFACE: Color32 = Color32::from_rgb(0x0E, 0x12, 0x11);
}

/// Resolves a per-preset accent through the running theme blend so the dark
/// chrome fades with the rest of the window instead of snapping at mid-swap.
fn themed(paper: Color32, forest: Color32) -> Color32 {
    let pick = |p: ThemePreset| if p.is_dark() { forest } else { paper };
    match current_transition() {
        Some(t) => mix_color(pick(t.from_preset), pick(t.to), t.progress()),
        None => pick(preset()),
    }
}

/// Deep chrome used by the v2 sidebar and player bar. Kept outside [`Palette`]
/// because the shell is always dark in both themes, even on Paper.
pub fn sidebar_deep() -> Color32 {
    themed(
        Color32::from_rgb(0x1B, 0x20, 0x1B),
        Color32::from_rgb(0x09, 0x0B, 0x09),
    )
}

pub fn sidebar_text() -> Color32 {
    themed(
        Color32::from_rgb(0xEC, 0xE5, 0xD5),
        Color32::from_rgb(0xF0, 0xE9, 0xDA),
    )
}

pub fn sidebar_muted() -> Color32 {
    themed(
        Color32::from_rgb(0xA3, 0x9A, 0x87),
        Color32::from_rgb(0x9C, 0x92, 0x80),
    )
}

pub fn sidebar_faint() -> Color32 {
    themed(
        Color32::from_rgb(0x75, 0x6D, 0x5F),
        Color32::from_rgb(0x6B, 0x63, 0x56),
    )
}

pub fn sidebar_line() -> Color32 {
    themed(
        Color32::from_rgba_unmultiplied(0xEC, 0xE5, 0xD5, 28),
        Color32::from_rgba_unmultiplied(0xF0, 0xE9, 0xDA, 26),
    )
}

pub fn sidebar_selected() -> Color32 {
    themed(
        Color32::from_rgb(0x26, 0x32, 0x29),
        Color32::from_rgb(0x23, 0x33, 0x28),
    )
}

pub fn sidebar_accent() -> Color32 {
    Color32::from_rgb(0x8F, 0xB7, 0x9A)
}

pub fn accent_soft() -> Color32 {
    themed(
        Color32::from_rgb(0xE2, 0xEA, 0xDF),
        Color32::from_rgb(0x23, 0x33, 0x28),
    )
}

pub fn accent_hover() -> Color32 {
    themed(
        Color32::from_rgb(0x25, 0x4A, 0x37),
        Color32::from_rgb(0xA6, 0xC8, 0xAE),
    )
}

pub fn signal() -> Color32 {
    c().danger
}

pub fn signal_soft() -> Color32 {
    themed(
        Color32::from_rgb(0xF3, 0xE1, 0xDA),
        Color32::from_rgb(0x35, 0x20, 0x19),
    )
}

pub fn serif_family() -> FontFamily {
    FontFamily::Name(Arc::from("rplayer-serif"))
}

pub fn serif(size: f32) -> FontId {
    FontId::new(size, serif_family())
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// Translucent accent colour. `egui` meshes are composited from
/// premultiplied colours, so compute the premultiplied RGB ourselves instead
/// of handing raw RGB with a low alpha to the painter.
pub fn divider() -> Color32 {
    let palette = c();
    palette.bg.lerp_to_gamma(palette.border_soft, 0.10)
}

pub fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (f32::from(color.a()) * alpha.clamp(0.0, 1.0)) as u8,
    )
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

/// Installs fonts once and applies a specific preset.
pub fn install_preset(ctx: &egui::Context, preset: ThemePreset) {
    set_preset_instant(preset);
    if !FONTS_INSTALLED.swap(true, Ordering::Relaxed) {
        install_fonts(ctx);
    }
    apply_style(ctx);
}

/// Applies the palette to the current egui style.
///
/// Only the visuals are replaced: spacing, text styles, menu sizes and scroll
/// metrics are left exactly as they were. A live theme swap must not move
/// geometry — an earlier version also reset `item_spacing` / `interact_size` /
/// `text_styles` here, and every TextEdit (the top-bar search pill in
/// particular) grew the moment the theme changed because the running style had
/// been built from egui's defaults (v2.2.x issue: search box height changed
/// after switching themes).
///
/// Cloning the current style also keeps per-window tweaks that panels make with
/// `ui.spacing_mut()` intact.
pub fn apply_style(ctx: &egui::Context) {
    let palette = c();
    let mut style = (*ctx.style()).clone();
    style.visuals = visuals_for(&palette);
    ctx.set_style(style);
}

/// Re-installs the palette when something else (eframe's own theme handling on
/// startup, or the OS theme notification) replaced our visuals behind our back.
///
/// Without this the app boots on egui's default visuals — grey separators,
/// default panel fills — and only looks right after the first manual theme
/// swap, which is exactly how the top-bar hairline went missing.
pub fn ensure_style(ctx: &egui::Context) {
    let palette = c();
    if ctx.style().visuals.panel_fill != palette.bg {
        apply_style(ctx);
    }
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
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_stroke = Stroke::new(1.0_f32, palette.border_soft);
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

    let load = |candidates: &[String]| -> Option<Vec<u8>> {
        candidates
            .iter()
            .find_map(|path| std::fs::read(path).ok())
    };

    let mut sans_candidates: Vec<String> = Vec::new();
    if let Ok(windir) = std::env::var("WINDIR") {
        sans_candidates.push(format!("{windir}\\Fonts\\msyh.ttc"));
        sans_candidates.push(format!("{windir}\\Fonts\\msyhbd.ttc"));
        sans_candidates.push(format!("{windir}\\Fonts\\simhei.ttf"));
    }
    sans_candidates.extend([
        "C:/Windows/Fonts/msyh.ttc".to_string(),
        "C:/Windows/Fonts/msyhbd.ttc".to_string(),
        "/System/Library/Fonts/PingFang.ttc".to_string(),
        "/System/Library/Fonts/Hiragino Sans GB.ttc".to_string(),
        "/Library/Fonts/Arial Unicode.ttf".to_string(),
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".to_string(),
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc".to_string(),
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc".to_string(),
    ]);

    let sans_loaded = if let Some(bytes) = load(&sans_candidates) {
        let mut data = FontData::from_owned(bytes);
        data.index = 0;
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
        true
    } else {
        false
    };

    let mut serif_candidates: Vec<String> = Vec::new();
    if let Ok(windir) = std::env::var("WINDIR") {
        serif_candidates.push(format!("{windir}\\Fonts\\NotoSerifSC-VF.ttf"));
        serif_candidates.push(format!("{windir}\\Fonts\\SourceHanSerifSC-VF.otf"));
        serif_candidates.push(format!("{windir}\\Fonts\\SourceHanSerifSC-Regular.otf"));
        serif_candidates.push(format!("{windir}\\Fonts\\simsun.ttc"));
    }
    serif_candidates.extend([
        "C:/Windows/Fonts/NotoSerifSC-VF.ttf".to_string(),
        "C:/Windows/Fonts/SourceHanSerifSC-VF.otf".to_string(),
        "C:/Windows/Fonts/SourceHanSerifSC-Regular.otf".to_string(),
        "C:/Windows/Fonts/simsun.ttc".to_string(),
        "/System/Library/Fonts/Songti.ttc".to_string(),
        "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc".to_string(),
        "/usr/share/fonts/truetype/noto/NotoSerifCJK-Regular.ttc".to_string(),
    ]);

    let serif_loaded = if let Some(bytes) = load(&serif_candidates) {
        let mut data = FontData::from_owned(bytes);
        data.index = 0;
        fonts
            .font_data
            .insert("rplayer-serif".to_owned(), Arc::new(data));
        true
    } else {
        false
    };

    let mut serif_chain = Vec::new();
    if serif_loaded {
        serif_chain.push("rplayer-serif".to_owned());
    }
    if sans_loaded {
        serif_chain.push("rplayer-cjk".to_owned());
    }
    if serif_chain.is_empty() {
        serif_chain = fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
    }
    fonts.families.insert(serif_family(), serif_chain);

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_color_blends_endpoints_and_midpoint() {
        let a = Color32::from_rgb(10, 20, 30);
        let b = Color32::from_rgb(200, 220, 240);
        assert_eq!(mix_color(a, b, 0.0), a);
        assert_eq!(mix_color(a, b, 1.0), b);
        let mid = mix_color(a, b, 0.5);
        assert_eq!((mid.r(), mid.g(), mid.b()), (105, 120, 135));
    }

    #[test]
    fn ease_is_clamped_and_reaches_both_ends() {
        assert_eq!(ease(-1.0), 0.0);
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(0.5), 0.5);
        assert_eq!(ease(1.0), 1.0);
        assert!(ease(0.25) < 0.25);
        assert!(ease(0.75) > 0.75);
    }

    fn linear(channel: u8) -> f32 {
        let value = f32::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    fn luminance(color: Color32) -> f32 {
        0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (light, dark) = if luminance(a) > luminance(b) {
            (luminance(a), luminance(b))
        } else {
            (luminance(b), luminance(a))
        };
        (light + 0.05) / (dark + 0.05)
    }

    #[test]
    fn legacy_presets_collapse_to_two_visible_themes() {
        assert_eq!(ThemePreset::default(), ThemePreset::Paper);
        assert_eq!(ThemePreset::VISIBLE.len(), 2);
        assert_eq!(ThemePreset::from_id("sand"), Some(ThemePreset::Paper));
        assert_eq!(ThemePreset::from_id("mist"), Some(ThemePreset::Paper));
        assert_eq!(ThemePreset::from_id("forest"), Some(ThemePreset::Forest));
        assert_eq!(ThemePreset::from_id("midnight"), Some(ThemePreset::Forest));
        assert_eq!(ThemePreset::from_id("charcoal"), Some(ThemePreset::Forest));
        assert_eq!(ThemePreset::from_id("nocturne"), Some(ThemePreset::Forest));
    }

    #[test]
    fn paper_and_nocturne_meet_text_contrast() {
        assert!(contrast(LIGHT.text, LIGHT.bg) >= 7.0);
        assert!(contrast(LIGHT.text, LIGHT.panel) >= 7.0);
        assert!(contrast(LIGHT.muted, LIGHT.bg) >= 4.5);
        assert!(contrast(LIGHT.accent, LIGHT.bg) >= 4.5);
        assert!(contrast(LIGHT.on_accent, LIGHT.accent) >= 4.5);

        assert!(contrast(DARK.text, DARK.bg) >= 7.0);
        assert!(contrast(DARK.text, DARK.panel) >= 7.0);
        assert!(contrast(DARK.muted, DARK.bg) >= 4.5);
        assert!(contrast(DARK.accent, DARK.bg) >= 4.5);
        assert!(contrast(DARK.on_accent, DARK.accent) >= 4.5);
    }

}
