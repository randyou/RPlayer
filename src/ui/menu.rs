//! Shared context-menu rows.
//!
//! egui's stock menu buttons are dense and mostly text-only. The redesign
//! uses 30px rows, a small leading glyph, optional shortcut hint and a single
//! dangerous action at the end of the menu.

use eframe::egui::{
    self, Align2, CornerRadius, CursorIcon, FontId, Pos2, Response, RichText, Sense,
    Stroke, Ui, Vec2, WidgetInfo, WidgetType,
};

use crate::theme;

use super::focus;
use super::icons::{self, Icon};

pub const ROW_HEIGHT: f32 = 30.0;
pub const MAX_WIDTH: f32 = 300.0;

/// Prepare a context-menu body.
pub fn begin(ui: &mut Ui) {
    // egui can keep the popup frame on the previous/fallback theme while the
    // rows use the current palette. Paint the panel explicitly so text and
    // background always come from the same theme.
    let panel = theme::c().panel;
    ui.painter().rect_filled(
        ui.max_rect().expand(6.0),
        CornerRadius::same(8),
        panel,
    );
    ui.visuals_mut().window_fill = panel;
    ui.visuals_mut().panel_fill = panel;
    ui.visuals_mut().window_stroke =
        Stroke::new(1.0_f32, theme::c().border_soft);
    ui.set_width(MAX_WIDTH);
    ui.spacing_mut().item_spacing.y = 2.0;
    ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
}

/// A menu section label in mono caps.
pub fn heading(ui: &mut Ui, text: &str) {
    if ui.available_height() > 0.0 {
        ui.add_space(2.0);
    }
    ui.label(
        RichText::new(text)
            .font(FontId::monospace(8.2))
            .color(theme::c().faint),
    );
}

/// A single actionable row.
pub fn item(
    ui: &mut Ui,
    icon: Option<Icon>,
    label: &str,
    shortcut: Option<&str>,
    danger: bool,
) -> Response {
    row(ui, MAX_WIDTH, icon, label, shortcut, danger, false, true)
}

/// A menu row with an explicit width and an optional checked state.
///
/// Popups that are not the default 300px context menu (the tray menu viewport,
/// for example) need to size their rows to the window instead of a constant.
/// `enabled: false` keeps the row in place but greys it out and swallows clicks,
/// which is what a tray menu needs when nothing is playing.
#[allow(clippy::too_many_arguments)]
pub fn row(
    ui: &mut Ui,
    width: f32,
    icon: Option<Icon>,
    label: &str,
    shortcut: Option<&str>,
    danger: bool,
    checked: bool,
    enabled: bool,
) -> Response {
    item_impl(ui, width, icon, label, shortcut, danger, checked, enabled)
}

#[allow(clippy::too_many_arguments)]
fn item_impl(
    ui: &mut Ui,
    width: f32,
    icon: Option<Icon>,
    label: &str,
    shortcut: Option<&str>,
    danger: bool,
    checked: bool,
    enabled: bool,
) -> Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::hover());
    let id = ui.id().with(("menu-item", label));
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, id, sense);

    if enabled && response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(6),
            if danger {
                theme::signal_soft()
            } else {
                theme::accent_soft()
            },
        );
    }

    // Disabled rows keep their place but drop to 35% opacity, so the popup
    // geometry never shifts when playback stops (design gap D1).
    let fg = if !enabled {
        theme::c().text.gamma_multiply(0.35)
    } else if danger {
        theme::signal()
    } else {
        theme::c().text
    };
    let left = rect.left() + if icon.is_some() { 34.0 } else { 12.0 };
    if let Some(icon) = icon {
        icons::paint(
            ui.painter(),
            egui::Rect::from_center_size(
                Pos2::new(rect.left() + 19.0, rect.center().y),
                Vec2::splat(14.0),
            ),
            icon,
            if !enabled {
                theme::c().muted.gamma_multiply(0.35)
            } else if danger {
                theme::signal()
            } else {
                theme::c().muted
            },
        );
    }
    ui.painter().text(
        Pos2::new(left, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.0),
        fg,
    );
    if let Some(shortcut) = shortcut {
        // Leave room for the check mark so the two never overlap.
        let right = if checked { 34.0 } else { 12.0 };
        ui.painter().text(
            Pos2::new(rect.right() - right, rect.center().y),
            Align2::RIGHT_CENTER,
            shortcut,
            FontId::monospace(9.0),
            theme::c().faint,
        );
    }
    if checked {
        let mark = Pos2::new(rect.right() - 16.0, rect.center().y);
        let stroke = Stroke::new(
            1.6_f32,
            if enabled {
                theme::c().accent
            } else {
                theme::c().faint
            },
        );
        ui.painter().line_segment(
            [Pos2::new(mark.x - 3.5, mark.y), Pos2::new(mark.x - 1.0, mark.y + 2.5)],
            stroke,
        );
        ui.painter().line_segment(
            [Pos2::new(mark.x - 1.0, mark.y + 2.5), Pos2::new(mark.x + 4.0, mark.y - 2.5)],
            stroke,
        );
    }

    response.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::Button,
            enabled,
            match (danger, checked) {
                (true, _) => format!("{label}（危险操作）"),
                (false, true) => format!("{label}（已开启）"),
                (false, false) => label.to_owned(),
            },
        )
    });
    if response.has_focus() {
        focus::ring_rect(ui, rect, 6);
    }
    if enabled {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

/// A submenu row. The nested menu keeps egui's navigation model, but the
/// trigger row uses the same spacing and typography as other menu actions.
pub fn submenu<R>(
    ui: &mut Ui,
    _icon: Option<Icon>,
    label: &str,
    add_contents: impl FnOnce(&mut Ui) -> R,
) {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
        ui.set_width(230.0);
        ui.menu_button(
            RichText::new(label).size(12.0).color(theme::c().text),
            add_contents,
        );
    });
}

/// A hairline divider with breathing room.
pub fn separator(ui: &mut Ui) {
    separator_sized(ui, MAX_WIDTH);
}

/// A hairline divider sized to `width`, for popups that are not the default
/// context-menu width.
pub fn separator_sized(ui: &mut Ui, width: f32) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0_f32, theme::c().border_soft),
    );
    ui.add_space(4.0);
}

/// Muted helper text inside a menu.
pub fn hint(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(10.5)
            .color(theme::c().faint),
    );
}

