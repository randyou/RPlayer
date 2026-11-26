//! Small typographic and surface primitives shared by the v2 shell.

use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Ui};

use crate::theme;
use crate::ui::tokens;

/// A small keycap used by the command palette and shortcut sheet.
pub fn keycap(ui: &mut Ui, text: &str) -> egui::Response {
    Frame::NONE
        .fill(theme::c().card)
        .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .font(tokens::mono(10.0))
                    .color(theme::c().muted),
            )
        })
        .inner
}

