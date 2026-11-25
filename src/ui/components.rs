//! Small typographic and surface primitives shared by the v2 shell.

use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, Stroke, Ui};

use crate::theme;
use crate::ui::tokens;

/// Monospaced micro-label used above page titles and section headings.
pub fn eyebrow(ui: &mut Ui, text: &str) -> egui::Response {
    ui.label(
        RichText::new(text)
            .font(tokens::mono(tokens::META))
            .color(theme::c().faint),
    )
}

pub fn page_title(ui: &mut Ui, text: &str) -> egui::Response {
    // D13: below the rail breakpoint the page title drops from 42 to 32 so the
    // header stops eating the first content row on short windows.
    let size = if ui.ctx().screen_rect().width() < 1120.0 {
        32.0
    } else {
        tokens::PAGE_TITLE
    };
    ui.label(
        RichText::new(text)
            .font(tokens::serif(size))
            .color(theme::c().text),
    )
}

pub fn page_header(ui: &mut Ui, kicker: &str, title: &str, meta: &str) {
    eyebrow(ui, kicker);
    ui.add_space(4.0);
    page_title(ui, title);
    if !meta.is_empty() {
        ui.add_space(5.0);
        ui.label(
            RichText::new(meta)
                .size(11.5)
                .color(theme::c().muted),
        );
    }
}

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

pub fn chip(ui: &mut Ui, text: &str) -> egui::Response {
    Frame::NONE
        .fill(theme::accent_soft())
        .stroke(Stroke::new(1.0_f32, theme::accent_alpha(0.22)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(10.0)
                    .color(theme::c().accent),
            )
        })
        .inner
}

pub fn surface(ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> egui::InnerResponse<()> {
    Frame::NONE
        .fill(theme::c().panel)
        .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
        .corner_radius(CornerRadius::same(tokens::RADIUS_CARD))
        .inner_margin(Margin::same(16))
        .show(ui, add)
}

pub fn empty_state(ui: &mut Ui, title: &str, body: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(36.0);
        ui.label(RichText::new(title).font(tokens::serif(24.0)));
        ui.add_space(6.0);
        ui.label(
            RichText::new(body)
                .size(12.0)
                .color(theme::c().muted),
        );
        ui.add_space(18.0);
    });
}

pub fn toolbar(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.with_layout(Layout::left_to_right(Align::Center), add);
}
