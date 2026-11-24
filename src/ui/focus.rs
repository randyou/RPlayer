//! Keyboard focus paint helpers.
//!
//! egui already owns the actual focus state and tab order. Custom-painted
//! controls only need to show that state, because unlike a stock `Button`
//! they do not get a focus outline from the theme.

use egui::{Response, Stroke, StrokeKind, Ui};

use crate::theme;

/// Draw a 2 px accent ring inside `response.rect` while it owns keyboard focus.
pub fn ring(ui: &Ui, response: &Response) {
    ring_with_radius(ui, response, 9);
}

/// Focus ring for circular controls such as the transport play buttons.
pub fn ring_round(ui: &Ui, response: &Response) {
    let radius = (response.rect.height() * 0.5).round() as u8;
    ring_with_radius(ui, response, radius.max(9));
}

/// Move keyboard focus to a control when the pointer activates it, so a
/// mouse click can be continued with Space/Enter or the arrow keys.
pub fn focus_on_click(response: &Response) {
    if response.clicked() {
        response.request_focus();
    }
}

fn ring_with_radius(ui: &Ui, response: &Response, radius: u8) {
    if !response.has_focus() {
        return;
    }
    ui.painter().rect_stroke(
        response.rect.shrink(1.0),
        egui::CornerRadius::same(radius),
        Stroke::new(2.0_f32, theme::c().accent),
        StrokeKind::Inside,
    );
}

/// Focus ring for controls whose hit area is larger or differently shaped
/// than the visible content (sliders, table rows).
pub fn ring_rect(ui: &Ui, rect: egui::Rect, radius: u8) {
    ui.painter().rect_stroke(
        rect.shrink(1.0),
        egui::CornerRadius::same(radius),
        Stroke::new(2.0_f32, theme::c().accent),
        StrokeKind::Inside,
    );
}
