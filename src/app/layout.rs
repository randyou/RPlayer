//! Responsive breakpoints for the shell (D13).
//!
//! Four tiers share one logical minimum width of 720 px:
//! - `Wide`  >= 1280: 236 sidebar + 300 right rail
//! - `Medium` 1120..1279: right rail hidden
//! - `Compact` 840..1119: 72 icon sidebar, album column hidden, 76 player bar
//! - `Narrow` < 840: drawer sidebar, two-line 96 player bar
//!
//! Breakpoints are evaluated per frame from the logical window width; no
//! animation on tier changes so resizing never flickers.

/// Layout tier for the current window width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tier {
    Wide,
    Medium,
    Compact,
    Narrow,
}

/// Logical minimum width we still promise a complete layout for.
pub(super) const MIN_WIDTH: f32 = 720.0;
/// Logical minimum height: 720×720 is the 200% DPI floor on a 1440 screen.
pub(super) const MIN_HEIGHT: f32 = 600.0;

impl Tier {
    pub(super) fn of(width: f32) -> Self {
        if width >= 1280.0 {
            Self::Wide
        } else if width >= 1120.0 {
            Self::Medium
        } else if width >= 840.0 {
            Self::Compact
        } else {
            Self::Narrow
        }
    }

    pub(super) fn of_ctx(ctx: &eframe::egui::Context) -> Self {
        Self::of(ctx.screen_rect().width())
    }

    /// Sidebar width in logical pixels, or 0 when the sidebar is a drawer.
    pub(super) fn sidebar_width(self) -> f32 {
        match self {
            Self::Wide | Self::Medium => 236.0,
            Self::Compact => 72.0,
            Self::Narrow => 0.0,
        }
    }

    pub(super) fn player_height(self) -> f32 {
        match self {
            Self::Wide | Self::Medium => 92.0,
            Self::Compact => 76.0,
            Self::Narrow => 96.0,
        }
    }

    pub(super) fn shows_rail(self) -> bool {
        self == Self::Wide
    }

    /// Album column only fits from the medium tier upward.
    pub(super) fn shows_album_column(self) -> bool {
        matches!(self, Self::Wide | Self::Medium)
    }

    pub(super) fn icons_only_sidebar(self) -> bool {
        self == Self::Compact
    }

    pub(super) fn drawer(self) -> bool {
        self == Self::Narrow
    }

    /// Dialog width: `min(92vw, 720px)` on narrow windows.
    pub(super) fn dialog_width(self, viewport: f32, preferred: f32) -> f32 {
        if self == Self::Narrow {
            (viewport * 0.92).min(720.0).min(preferred)
        } else {
            preferred
        }
    }
}

/// Shrink a preferred dialog width to `min(92vw, 720px)` on the narrow tier.
pub(super) fn fit_width(ctx: &eframe::egui::Context, preferred: f32) -> f32 {
    Tier::of_ctx(ctx).dialog_width(ctx.screen_rect().width(), preferred)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_follow_the_four_breakpoints() {
        assert_eq!(Tier::of(1440.0), Tier::Wide);
        assert_eq!(Tier::of(1280.0), Tier::Wide);
        assert_eq!(Tier::of(1279.0), Tier::Medium);
        assert_eq!(Tier::of(1120.0), Tier::Medium);
        assert_eq!(Tier::of(1119.0), Tier::Compact);
        assert_eq!(Tier::of(840.0), Tier::Compact);
        assert_eq!(Tier::of(839.0), Tier::Narrow);
        assert_eq!(Tier::of(720.0), Tier::Narrow);
    }

    #[test]
    fn tiers_pick_shell_dimensions() {
        assert_eq!(Tier::Wide.sidebar_width(), 236.0);
        assert_eq!(Tier::Compact.sidebar_width(), 72.0);
        assert_eq!(Tier::Narrow.sidebar_width(), 0.0);
        assert_eq!(Tier::Wide.player_height(), 92.0);
        assert_eq!(Tier::Compact.player_height(), 76.0);
        assert_eq!(Tier::Narrow.player_height(), 96.0);
        assert!(Tier::Wide.shows_rail());
        assert!(!Tier::Medium.shows_rail());
        assert!(!Tier::Compact.shows_album_column());
        assert!(Tier::Medium.shows_album_column());
    }

    #[test]
    fn narrow_dialogs_cap_at_ninety_two_percent() {
        assert!((Tier::Narrow.dialog_width(720.0, 820.0) - 662.4).abs() < 0.01);
        assert_eq!(Tier::Narrow.dialog_width(1920.0, 620.0), 620.0);
        assert_eq!(Tier::Compact.dialog_width(1024.0, 820.0), 820.0);
    }
}
