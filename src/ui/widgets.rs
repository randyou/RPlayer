//! Reusable RPlayer controls: icon buttons, sidebar rows, search field,
//! the mint seek bar and custom volume control.

use egui::{
    Align2, Button, Color32, CornerRadius, CursorIcon, FontId, Frame, Margin, Pos2, Rect, Response,
    RichText, Sense, Stroke, TextEdit, Ui, Vec2,
};

use crate::theme;
use crate::ui::icons::{self, Icon};

pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hovered = response.hovered();
    if hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(9), theme::c().hover);
    }
    icons::paint(
        ui.painter(),
        rect.shrink(size * 0.27),
        icon,
        if hovered { theme::c().text } else { theme::c().muted },
    );
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

pub fn icon_button_tinted(
    ui: &mut Ui,
    icon: Icon,
    size: f32,
    tint: Color32,
    active: bool,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if active || response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(9), theme::accent_alpha(0.12));
    }
    let color = if active {
        theme::c().accent
    } else if response.hovered() {
        theme::c().text
    } else {
        tint
    };
    icons::paint(ui.painter(), rect.shrink(size * 0.27), icon, color);
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

pub fn circle_button(
    ui: &mut Ui,
    icon: Icon,
    size: f32,
    fill: Color32,
    tint: Color32,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hovered = response.hovered();
    let center = rect.center();
    let radius = size * 0.5;
    if response.is_pointer_button_down_on() {
        ui.painter()
            .circle_filled(center, radius, Color32::from_rgb(0x7A, 0xF0, 0xA1));
    } else {
        ui.painter()
            .circle_filled(center, radius * if hovered { 1.03 } else { 1.0 }, fill);
    }
    icons::paint(ui.painter(), rect.shrink(size * 0.30), icon, tint);
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

pub fn play_button(ui: &mut Ui, playing: bool, enabled: bool) -> Response {
    let size = 50.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let center = rect.center();
    let radius = size * 0.5;

    if enabled {
        // Soft halo behind the primary action.
        ui.painter()
            .circle_filled(center, radius + 5.0, theme::accent_alpha(0.10));
        let fill = if response.is_pointer_button_down_on() {
            Color32::from_rgb(0x79, 0xEF, 0xA2)
        } else if response.hovered() {
            Color32::from_rgb(0xA6, 0xFF, 0xC5)
        } else {
            theme::c().accent
        };
        ui.painter().circle_filled(center, radius, fill);
        let icon = if playing { Icon::Pause } else { Icon::Play };
        icons::paint(
            ui.painter(),
            rect.shrink(size * 0.31).translate(if playing {
                Vec2::ZERO
            } else {
                Vec2::new(size * 0.015, 0.0)
            }),
            icon,
            theme::c().on_accent,
        );
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        ui.painter()
            .circle_filled(center, radius, Color32::from_rgb(0x22, 0x26, 0x30));
        icons::paint(
            ui.painter(),
            rect.shrink(size * 0.31),
            Icon::Play,
            theme::c().faint,
        );
        response
    }
}

pub fn nav_item(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    count: Option<usize>,
    selected: bool,
) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, CornerRadius::same(10), theme::accent_alpha(0.10));
        let bar = Rect::from_center_size(
            Pos2::new(rect.left() + 2.0, rect.center().y),
            Vec2::new(3.0, 18.0),
        );
        painter.rect_filled(bar, CornerRadius::same(2), theme::c().accent);
    } else if response.hovered() {
        painter.rect_filled(rect, CornerRadius::same(10), theme::c().hover);
    }

    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 25.0, rect.center().y),
        Vec2::splat(19.0),
    );
    icons::paint(
        painter,
        icon_rect,
        icon,
        if selected {
            theme::c().accent
        } else {
            theme::c().muted
        },
    );

    painter.text(
        Pos2::new(rect.left() + 44.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        if selected { theme::c().text } else { theme::c().muted },
    );

    if let Some(count) = count {
        painter.text(
            Pos2::new(rect.right() - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            count.to_string(),
            FontId::proportional(11.0),
            theme::c().faint,
        );
    }

    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn playlist_item(ui: &mut Ui, name: &str, selected: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, CornerRadius::same(9), theme::accent_alpha(0.10));
    } else if response.hovered() {
        painter.rect_filled(rect, CornerRadius::same(9), theme::c().hover);
    }
    let dot = Rect::from_center_size(
        Pos2::new(rect.left() + 25.0, rect.center().y),
        Vec2::splat(13.0),
    );
    icons::paint(
        painter,
        dot,
        Icon::Music,
        if selected {
            theme::c().accent
        } else {
            theme::c().faint
        },
    );
    painter.text(
        Pos2::new(rect.left() + 44.0, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(13.0),
        if selected { theme::c().text } else { theme::c().muted },
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn section_label(ui: &mut Ui, text: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(text).color(theme::c().faint).size(11.0).strong());
    ui.add_space(2.0);
}

pub fn search_box(ui: &mut Ui, query: &mut String, hint: &str) -> Response {
    let mut response = None;
    Frame::NONE
        .fill(theme::c().card)
        .stroke(Stroke::new(1.0_f32, theme::c().border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(12, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                icons::paint(ui.painter(), rect, Icon::Search, theme::c().muted);
                let edit = TextEdit::singleline(query)
                    .hint_text(RichText::new(hint).color(theme::c().faint))
                    .frame(false)
                    .margin(Margin::symmetric(2, 2))
                    .desired_width(ui.available_width());
                response = Some(ui.add(edit));
            });
        });
    response.expect("search box always adds a text edit")
}

pub fn seek_bar(ui: &mut Ui, position: f64, duration: f64) -> Option<f64> {
    let width = ui.available_width().max(80.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::click_and_drag());
    let hovered = response.hovered() || response.dragged();
    let fraction = if duration > 0.0 {
        (position / duration).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };

    let rail = Rect::from_center_size(rect.center(), Vec2::new(width, 4.0));
    ui.painter()
        .rect_filled(rail, CornerRadius::same(2), theme::c().border);
    let filled_width = width * fraction;
    if filled_width > 0.5 {
        let filled = Rect::from_min_size(rail.min, Vec2::new(filled_width, 4.0));
        ui.painter()
            .rect_filled(filled, CornerRadius::same(2), theme::c().accent);
    }

    let handle_center = Pos2::new(rail.left() + filled_width, rail.center().y);
    if hovered {
        ui.painter()
            .circle_filled(handle_center, 9.0, theme::accent_alpha(0.18));
    }
    ui.painter().circle_filled(
        handle_center,
        if hovered { 5.5 } else { 4.0 },
        theme::c().accent,
    );

    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let pointer = response.interact_pointer_pos()?;
    if response.clicked() || response.dragged() {
        let t = ((pointer.x - rail.left()) / rail.width()).clamp(0.0, 1.0);
        return Some(t as f64 * duration);
    }
    None
}

/// Draws a compact volume bar. Returns the new volume while dragging.
pub fn volume_bar(ui: &mut Ui, volume: f32) -> Option<f32> {
    let width = 84.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::click_and_drag());
    let rail = Rect::from_center_size(rect.center(), Vec2::new(width, 4.0));
    ui.painter()
        .rect_filled(rail, CornerRadius::same(2), theme::c().border);
    let fraction = volume.clamp(0.0, 1.0);
    let filled = Rect::from_min_size(rail.min, Vec2::new(width * fraction, 4.0));
    if fraction > 0.01 {
        ui.painter()
            .rect_filled(filled, CornerRadius::same(2), theme::c().muted);
    }
    let handle = Pos2::new(rail.left() + width * fraction, rail.center().y);
    ui.painter().circle_filled(
        handle,
        if response.hovered() { 5.0 } else { 3.5 },
        if response.hovered() {
            theme::c().accent
        } else {
            theme::c().text
        },
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if let Some(pointer) = response.interact_pointer_pos() {
        if response.clicked() || response.dragged() {
            let t = ((pointer.x - rail.left()) / rail.width()).clamp(0.0, 1.0);
            return Some(t);
        }
    }
    None
}

pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let button = Button::new(RichText::new(text).color(theme::c().on_accent).strong().size(14.0))
        .fill(theme::c().accent)
        .corner_radius(10)
        .min_size(Vec2::new(96.0, 40.0));
    ui.add(button)
}

pub fn primary_button_full(ui: &mut Ui, text: &str) -> Response {
    let width = ui.available_width().max(120.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 42.0), Sense::click());
    let fill = if response.hovered() {
        theme::c().accent.gamma_multiply(1.06)
    } else {
        theme::c().accent
    };
    ui.painter().rect_filled(rect, CornerRadius::same(10), fill);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(14.0),
        theme::c().on_accent,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn secondary_button(ui: &mut Ui, text: &str) -> Response {
    let button = Button::new(RichText::new(text).color(theme::c().text).size(13.0))
        .fill(theme::c().card)
        .stroke(Stroke::new(1.0_f32, theme::c().border))
        .corner_radius(10)
        .min_size(Vec2::new(72.0, 38.0));
    ui.add(button)
}

pub fn danger_button(ui: &mut Ui, text: &str) -> Response {
    let button = Button::new(RichText::new(text).color(theme::c().danger).size(13.0))
        .fill(theme::c().danger.gamma_multiply(0.12))
        .stroke(Stroke::new(1.0_f32, theme::c().danger.gamma_multiply(0.45)))
        .corner_radius(10)
        .min_size(Vec2::new(72.0, 38.0));
    ui.add(button)
}

pub fn chip(ui: &mut Ui, text: &str, selected: bool) -> Response {
    let button = Button::new(RichText::new(text).size(12.0).color(if selected {
        theme::c().accent
    } else {
        theme::c().muted
    }))
    .fill(if selected {
        theme::accent_alpha(0.12)
    } else {
        Color32::TRANSPARENT
    })
    .stroke(Stroke::new(
        1.0_f32,
        if selected {
            theme::accent_alpha(0.38)
        } else {
            theme::c().border
        },
    ))
    .corner_radius(CornerRadius::same(99))
    .min_size(Vec2::new(64.0, 30.0));
    ui.add(button).on_hover_cursor(CursorIcon::PointingHand)
}

pub fn empty_illustration(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let center = rect.center();
    let painter = ui.painter();
    painter.circle_filled(center, size * 0.5, theme::accent_alpha(0.04));
    painter.circle_filled(center, size * 0.37, theme::c().card);
    painter.circle_stroke(center, size * 0.37, Stroke::new(1.0_f32, theme::c().border));
    icons::paint(
        painter,
        rect.shrink(size * 0.34),
        Icon::Music,
        theme::c().accent,
    );
}

/// Small animated switch used by the equalizer and overlay settings.
pub fn toggle_switch(ui: &mut Ui, on: &mut bool) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(42.0, 23.0), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool(response.id, *on);
    let track = if t > 0.5 {
        theme::c().accent
    } else {
        theme::c().border
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(99), track);
    let knob_x = egui::lerp((rect.left() + 11.5)..=(rect.right() - 11.5), t);
    ui.painter().circle_filled(
        Pos2::new(knob_x, rect.center().y),
        8.5,
        theme::c().panel,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn divider(ui: &mut Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0, theme::c().border_soft);
}
