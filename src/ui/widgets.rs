//! Reusable RPlayer controls: icon buttons, sidebar rows, search field,
//! the mint seek bar and custom volume control.

use egui::{
    Align2, Button, Color32, CornerRadius, CursorIcon, FontId, Frame, Margin, Pos2, Rect, Response,
    RichText, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2, WidgetInfo, WidgetType,
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
    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
    mark(&response, WidgetType::Button, tooltip, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

/// Small icon button for the dark sidebar, using sidebar-specific colors.
pub fn sidebar_icon_button(ui: &mut Ui, icon: Icon, size: f32, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(6),
            crate::theme::wash(14),
        );
    }
    icons::paint(
        ui.painter(),
        rect.shrink(size * 0.27),
        icon,
        if response.hovered() {
            theme::sidebar_text()
        } else {
            theme::sidebar_muted()
        },
    );
    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
    mark(&response, WidgetType::Button, tooltip, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

/// Tinted toggle button for the dark player bar / sidebar.
pub fn sidebar_icon_button_tinted(
    ui: &mut Ui,
    icon: Icon,
    size: f32,
    active: bool,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if active || response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(7),
            crate::theme::wash(12),
        );
    }
    let color = if active {
        theme::c().accent
    } else if response.hovered() {
        theme::sidebar_text()
    } else {
        theme::sidebar_muted()
    };
    icons::paint(ui.painter(), rect.shrink(size * 0.27), icon, color);
    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
    // S9: these are actions, not switches; narration must not append 关闭.
    mark(&response, WidgetType::Button, tooltip, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
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
    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
    // S9: these are actions, not switches; narration must not append 关闭.
    mark(&response, WidgetType::Button, tooltip, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

/// Compact glyph toggle for the dark player bar.
pub fn sidebar_glyph_button(
    ui: &mut Ui,
    glyph: &str,
    size: f32,
    active: bool,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if active || response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(7),
            crate::theme::wash(12),
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(size * 0.46),
        if active {
            theme::c().accent
        } else if response.hovered() {
            theme::sidebar_text()
        } else {
            theme::sidebar_muted()
        },
    );
    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
    // S9: these are actions, not switches; narration must not append 关闭.
    mark(&response, WidgetType::Button, tooltip, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

/// Compact text glyph button used for the desktop lyric "词" shortcut.
pub fn glyph_button_tinted(
    ui: &mut Ui,
    glyph: &str,
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
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(size * 0.46),
        color,
    );
    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
    // S9: these are actions, not switches; narration must not append 关闭.
    mark(&response, WidgetType::Button, tooltip, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
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
    crate::ui::focus::focus_on_click(&response);
    let hovered = response.hovered();
    let center = rect.center();
    let radius = size * 0.5;
    if response.is_pointer_button_down_on() {
        ui.painter()
            .circle_filled(center, radius, crate::theme::ACCENT_PULSE);
    } else {
        ui.painter()
            .circle_filled(center, radius * if hovered { 1.03 } else { 1.0 }, fill);
    }
    icons::paint(ui.painter(), rect.shrink(size * 0.30), icon, tint);
    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
    crate::ui::focus::ring_round(ui, &response);
    response
}

pub fn play_button(ui: &mut Ui, playing: bool, enabled: bool) -> Response {
    play_button_sized(ui, playing, enabled, 50.0)
}

/// Same control at a caller-chosen diameter (D13 compact player bar uses 40).
pub fn play_button_sized(ui: &mut Ui, playing: bool, enabled: bool, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    crate::ui::focus::focus_on_click(&response);
    let center = rect.center();
    let radius = size * 0.5;

    let response = if enabled {
        // Soft halo behind the primary action.
        ui.painter()
            .circle_filled(center, radius + 5.0, theme::accent_alpha(0.10));
        let fill = if response.is_pointer_button_down_on() {
            crate::theme::ACCENT_PRESSED
        } else if response.hovered() {
            crate::theme::ACCENT_HOVER
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
            .circle_filled(center, radius, crate::theme::BUTTON_DISABLED);
        icons::paint(
            ui.painter(),
            rect.shrink(size * 0.31),
            Icon::Play,
            theme::c().faint,
        );
        response
    };
    mark(
        &response,
        WidgetType::Button,
        if playing { "暂停" } else { "播放" },
        None,
    );
    crate::ui::focus::ring_round(ui, &response);
    response
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
    let hover_t = ui.ctx().animate_bool_with_time(
        ui.id().with(("nav-hover", label)),
        response.hovered() && !selected,
        crate::ui::motion::duration(crate::ui::motion::FAST),
    );
    if hover_t > 0.001 {
        painter.rect_filled(
            rect,
            CornerRadius::same(8),
            crate::theme::wash((14.0 * hover_t) as u8),
        );
    }
    if selected {
        painter.rect_filled(rect, CornerRadius::same(8), theme::sidebar_selected());
        let bar = Rect::from_center_size(
            Pos2::new(rect.left() + 1.5, rect.center().y),
            Vec2::new(2.0, 18.0),
        );
        painter.rect_filled(bar, CornerRadius::same(1), theme::sidebar_accent());
    }

    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 22.0, rect.center().y),
        Vec2::splat(17.0),
    );
    icons::paint(
        painter,
        icon_rect,
        icon,
        if selected {
            theme::c().accent
        } else {
            theme::sidebar_muted()
        },
    );

    painter.text(
        Pos2::new(rect.left() + 39.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.5),
        if selected {
            theme::sidebar_text()
        } else {
            theme::sidebar_muted()
        },
    );

    if let Some(count) = count {
        painter.text(
            Pos2::new(rect.right() - 10.0, rect.center().y),
            Align2::RIGHT_CENTER,
            count.to_string(),
            FontId::proportional(10.0),
            theme::sidebar_faint(),
        );
    }

    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    mark(&response, WidgetType::Button, label, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

/// Icon-only navigation entry for the 72px compact sidebar (D13).
///
/// Keeps the same selected-hover language as [`nav_item`] but centers a 17px
/// glyph in a 48×40 hit area; the label moves into the tooltip/AccessKit name.
pub fn nav_icon_item(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 40.0), Sense::click());
    let painter = ui.painter();
    let hover_t = ui.ctx().animate_bool_with_time(
        ui.id().with(("nav-icon-hover", label)),
        response.hovered() && !selected,
        crate::ui::motion::duration(crate::ui::motion::FAST),
    );
    if hover_t > 0.001 {
        painter.rect_filled(
            rect,
            CornerRadius::same(8),
            crate::theme::wash((14.0 * hover_t) as u8),
        );
    }
    if selected {
        painter.rect_filled(rect, CornerRadius::same(8), theme::sidebar_selected());
        let bar = Rect::from_center_size(
            Pos2::new(rect.left() + 1.5, rect.center().y),
            Vec2::new(2.0, 18.0),
        );
        painter.rect_filled(bar, CornerRadius::same(1), theme::sidebar_accent());
    }
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(17.0));
    icons::paint(
        painter,
        icon_rect,
        icon,
        if selected {
            theme::c().accent
        } else {
            theme::sidebar_muted()
        },
    );

    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(label);
    mark(&response, WidgetType::Button, label, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

pub fn playlist_item(ui: &mut Ui, name: &str, selected: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    let painter = ui.painter();
    let hover_t = ui.ctx().animate_bool_with_time(
        ui.id().with(("playlist-hover", name)),
        response.hovered() && !selected,
        crate::ui::motion::duration(crate::ui::motion::FAST),
    );
    if hover_t > 0.001 {
        painter.rect_filled(
            rect,
            CornerRadius::same(7),
            crate::theme::wash((12.0 * hover_t) as u8),
        );
    }
    if selected {
        painter.rect_filled(rect, CornerRadius::same(7), theme::sidebar_selected());
    }
    let dot = Rect::from_center_size(
        Pos2::new(rect.left() + 22.0, rect.center().y),
        Vec2::splat(13.0),
    );
    icons::paint(
        painter,
        dot,
        Icon::Music,
        if selected {
            theme::sidebar_accent()
        } else {
            theme::sidebar_faint()
        },
    );
    painter.text(
        Pos2::new(rect.left() + 38.0, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(11.5),
        if selected {
            theme::sidebar_text()
        } else {
            theme::sidebar_muted()
        },
    );
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    mark(&response, WidgetType::Button, name, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

pub fn search_box(ui: &mut Ui, query: &mut String, hint: &str) -> Response {
    let mut response = None;
    let outer = Frame::NONE
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
        })
        .response
        .rect;
    let response = response.expect("search box always adds a text edit");
    // Focus shows as the pill's own border turning accent, rather than a second
    // rectangle drawn inside it.
    if response.has_focus() {
        ui.painter().rect_stroke(
            outer.shrink(0.5),
            CornerRadius::same(10),
            Stroke::new(1.0_f32, theme::c().accent),
            StrokeKind::Inside,
        );
    }
    response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, hint));
    response
}

pub fn seek_bar(ui: &mut Ui, position: f64, duration: f64) -> Option<f64> {
    let width = ui.available_width().max(80.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::click_and_drag());
    crate::ui::focus::focus_on_click(&response);
    let hovered = response.hovered() || response.dragged();
    let fraction = if duration > 0.0 {
        (position / duration).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };

    // Expose the position as a slider value: without it the bar only announced
    // its name, so screen readers could not say where playback is.
    response.widget_info(|| {
        egui::WidgetInfo::slider(true, (fraction as f64) * 100.0, "播放进度")
    });

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
    mark(
        &response,
        WidgetType::Slider,
        &format!(
            "播放进度 {} / {}",
            crate::model::format_duration(position),
            crate::model::format_duration(duration)
        ),
        None,
    );
    if response.has_focus() {
        crate::ui::focus::ring_rect(ui, rect, 4);
    }

    if response.has_focus() {
        let step = ui.input(|input| {
            if input.key_pressed(egui::Key::ArrowLeft) {
                Some(-5.0_f64)
            } else if input.key_pressed(egui::Key::ArrowRight) {
                Some(5.0_f64)
            } else {
                None
            }
        });
        if let Some(step) = step {
            return Some((position + step).clamp(0.0, duration.max(0.0)));
        }
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
    crate::ui::focus::focus_on_click(&response);
    let rail = Rect::from_center_size(rect.center(), Vec2::new(width, 4.0));
    ui.painter()
        .rect_filled(rail, CornerRadius::same(2), theme::c().border);
    let fraction = volume.clamp(0.0, 1.0);
    // Same as the seek bar: publish the level so assistive tech can read it.
    response.widget_info(|| {
        egui::WidgetInfo::slider(true, (fraction as f64) * 100.0, "音量")
    });
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
    mark(
        &response,
        WidgetType::Slider,
        &format!("音量 {:.0}%", volume.clamp(0.0, 1.0) * 100.0),
        None,
    );
    if response.has_focus() {
        crate::ui::focus::ring_rect(ui, rect, 4);
        let step = ui.input(|input| {
            if input.key_pressed(egui::Key::ArrowUp) {
                Some(0.05_f32)
            } else if input.key_pressed(egui::Key::ArrowDown) {
                Some(-0.05_f32)
            } else {
                None
            }
        });
        if let Some(step) = step {
            return Some((volume + step).clamp(0.0, 1.0));
        }
    }
    if let Some(pointer) = response.interact_pointer_pos() {
        if response.clicked() || response.dragged() {
            let t = ((pointer.x - rail.left()) / rail.width()).clamp(0.0, 1.0);
            return Some(t);
        }
    }
    None
}

pub fn text_button(ui: &mut Ui, text: impl Into<egui::WidgetText>) -> Response {
    let response = ui.button(text).on_hover_cursor(CursorIcon::PointingHand);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let button = Button::new(RichText::new(text).color(theme::c().on_accent).strong().size(14.0))
        .fill(theme::c().accent)
        .corner_radius(10)
        .min_size(Vec2::new(96.0, 40.0));
    let response = ui.add(button).on_hover_cursor(CursorIcon::PointingHand);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

pub fn secondary_button(ui: &mut Ui, text: &str) -> Response {
    let button = Button::new(RichText::new(text).color(theme::c().text).size(13.0))
        .fill(theme::c().card)
        .stroke(Stroke::new(1.0_f32, theme::c().border))
        .corner_radius(10)
        .min_size(Vec2::new(72.0, 38.0));
    let response = ui.add(button).on_hover_cursor(CursorIcon::PointingHand);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

/// A compact full-width secondary button for narrow settings content.
pub fn secondary_button_sized(ui: &mut Ui, text: &str, width: f32) -> Response {
    let button = Button::new(RichText::new(text).color(theme::c().text).size(12.0))
        .fill(theme::c().card)
        .stroke(Stroke::new(1.0_f32, theme::c().border))
        .corner_radius(8)
        .min_size(Vec2::new(width, 28.0));
    let response = ui.add(button).on_hover_cursor(CursorIcon::PointingHand);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

pub fn danger_button(ui: &mut Ui, text: &str) -> Response {
    let button = Button::new(RichText::new(text).color(theme::c().danger).size(13.0))
        .fill(theme::c().danger.gamma_multiply(0.12))
        .stroke(Stroke::new(1.0_f32, theme::c().danger.gamma_multiply(0.45)))
        .corner_radius(10)
        .min_size(Vec2::new(72.0, 38.0));
    let response = ui.add(button).on_hover_cursor(CursorIcon::PointingHand);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

pub fn rating_stars(ui: &mut Ui, rating: u8, star_size: f32) -> Option<u8> {
    let gap = 2.0;
    let width = star_size * 5.0 + gap * 4.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, star_size + 4.0), Sense::click());
    let pointer = response.hover_pos();
    let hovered = pointer.map(|pos| {
        let local = (pos.x - rect.left()).max(0.0);
        ((local / (star_size + gap)).floor() as u8 + 1).min(5)
    });
    for index in 0..5 {
        let star = Rect::from_center_size(
            Pos2::new(
                rect.left() + star_size * 0.5 + index as f32 * (star_size + gap),
                rect.center().y,
            ),
            Vec2::splat(star_size),
        );
        let filled = index < rating || hovered.is_some_and(|hover| index < hover);
        icons::paint(
            ui.painter(),
            star,
            if filled { Icon::StarFilled } else { Icon::Star },
            if index < rating || filled {
                theme::c().accent
            } else {
                theme::c().faint
            },
        );
    }
    let clicked = response.clicked().then(|| {
        pointer.map(|pos| {
            let local = (pos.x - rect.left()).max(0.0);
            ((local / (star_size + gap)).floor() as u8 + 1).min(5)
        })
    });
    let result = clicked.flatten();
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    mark(
        &response,
        WidgetType::Slider,
        &format!("评分：{rating} / 5"),
        None,
    );
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    result
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
    let response = ui.add(button).on_hover_cursor(CursorIcon::PointingHand);
    mark(&response, WidgetType::Button, text, Some(selected));
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
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

/// A compact outlined pill used for segmented choices and tool presets.
pub fn pill(ui: &mut Ui, text: &str, selected: bool) -> Response {
    let color = if selected {
        theme::c().on_accent
    } else {
        theme::c().muted
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(11.5), color);
    let width = (galley.size().x + 26.0).max(46.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::click());
    let hovered = response.hovered();
    let press = ui.ctx().animate_bool_with_time(
        ui.id().with(("press", text)),
        response.is_pointer_button_down_on(),
        crate::ui::motion::duration(crate::ui::motion::PRESS),
    );
    let paint_rect = Rect::from_center_size(rect.center(), rect.size() * (1.0 - 0.025 * press));
    let radius = CornerRadius::same(99);
    let fill = if selected {
        theme::c().accent
    } else if hovered {
        theme::c().hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(paint_rect, radius, fill);
    ui.painter().rect_stroke(
        paint_rect,
        radius,
        Stroke::new(
            1.0_f32,
            if selected {
                theme::c().accent
            } else {
                theme::c().border
            },
        ),
        StrokeKind::Inside,
    );
    ui.painter().galley(
        Pos2::new(
            paint_rect.center().x - galley.size().x * 0.5,
            paint_rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        color,
    );
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    mark(&response, WidgetType::Button, text, Some(selected));
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

/// Small primary button for the shared 32px dialog action band.
pub fn primary_button_compact(ui: &mut Ui, text: &str) -> Response {
    compact_button(ui, text, true)
}

/// Small secondary button for the shared 32px dialog action band.
pub fn secondary_button_compact(ui: &mut Ui, text: &str) -> Response {
    compact_button(ui, text, false)
}

fn compact_button(ui: &mut Ui, text: &str, primary: bool) -> Response {
    let enabled = ui.is_enabled();
    let text_color = if !enabled {
        theme::c().faint
    } else if primary {
        theme::c().on_accent
    } else {
        theme::c().text
    };
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        FontId::proportional(12.5),
        text_color,
    );
    let width = (galley.size().x + 36.0).max(64.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    let hovered = enabled && response.hovered();
    let press = ui.ctx().animate_bool_with_time(
        ui.id().with(("press", text)),
        response.is_pointer_button_down_on(),
        crate::ui::motion::duration(crate::ui::motion::PRESS),
    );
    let paint_rect = Rect::from_center_size(rect.center(), rect.size() * (1.0 - 0.025 * press));
    let radius = CornerRadius::same(crate::ui::tokens::RADIUS_CONTROL);
    let (fill, stroke) = if !enabled {
        (theme::c().border_soft, Color32::TRANSPARENT)
    } else if primary {
        (
            if hovered {
                theme::c().accent.gamma_multiply(1.06)
            } else {
                theme::c().accent
            },
            Color32::TRANSPARENT,
        )
    } else {
        (
            if hovered {
                theme::c().hover
            } else {
                theme::c().card
            },
            theme::c().border,
        )
    };
    ui.painter().rect_filled(paint_rect, radius, fill);
    if stroke != Color32::TRANSPARENT {
        ui.painter().rect_stroke(
            paint_rect,
            radius,
            Stroke::new(1.0_f32, stroke),
            StrokeKind::Inside,
        );
    }
    ui.painter().galley(
        Pos2::new(
            paint_rect.center().x - galley.size().x * 0.5,
            paint_rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        text_color,
    );
    let response = if enabled {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    };
    mark(&response, WidgetType::Button, text, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

/// Small animated switch used by the equalizer and overlay settings.
pub fn toggle_switch(ui: &mut Ui, on: &mut bool) -> Response {
    toggle_switch_named(ui, on, "开关")
}

/// Same switch with an explicit AccessKit name for the surface it belongs to.
pub fn toggle_switch_named(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
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
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    mark(&response, WidgetType::Checkbox, label, Some(*on));
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

pub(crate) fn mark(response: &Response, typ: WidgetType, label: &str, selected: Option<bool>) {
    response.widget_info(|| match selected {
        Some(selected) => WidgetInfo::selected(typ, true, selected, label),
        None => WidgetInfo::labeled(typ, true, label),
    });
}

pub fn divider(ui: &mut Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0, theme::divider());
}

/// Primary button with a hand-painted vector icon before the label.
pub fn primary_button_icon(ui: &mut Ui, icon: Icon, text: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(112.0, 40.0), Sense::click());
    let hovered = response.hovered();
    let fill = if hovered {
        theme::c().accent.gamma_multiply(1.06)
    } else {
        theme::c().accent
    };
    let press = ui.ctx().animate_bool_with_time(
        ui.id().with(("press", text)),
        response.is_pointer_button_down_on(),
        crate::ui::motion::duration(crate::ui::motion::PRESS),
    );
    let paint_rect = Rect::from_center_size(
        rect.center(),
        rect.size() * (1.0 - 0.025 * press),
    );
    ui.painter()
        .rect_filled(paint_rect, CornerRadius::same(10), fill);
    paint_icon_label(
        ui,
        paint_rect,
        icon,
        text,
        theme::c().on_accent,
        14.0,
        true,
    );
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    mark(&response, WidgetType::Button, text, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

/// Secondary button with a hand-painted vector icon before the label.
pub fn secondary_button_icon(ui: &mut Ui, icon: Icon, text: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(92.0, 38.0), Sense::click());
    let hovered = response.hovered();
    let press = ui.ctx().animate_bool_with_time(
        ui.id().with(("press", text)),
        response.is_pointer_button_down_on(),
        crate::ui::motion::duration(crate::ui::motion::PRESS),
    );
    let paint_rect = Rect::from_center_size(
        rect.center(),
        rect.size() * (1.0 - 0.025 * press),
    );
    ui.painter().rect_filled(
        paint_rect,
        CornerRadius::same(10),
        if hovered { theme::c().hover } else { theme::c().card },
    );
    ui.painter().rect_stroke(
        paint_rect,
        CornerRadius::same(10),
        Stroke::new(1.0_f32, theme::c().border),
        egui::StrokeKind::Inside,
    );
    paint_icon_label(
        ui,
        paint_rect,
        icon,
        text,
        theme::c().text,
        13.0,
        false,
    );
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    mark(&response, WidgetType::Button, text, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

fn paint_icon_label(
    ui: &Ui,
    rect: Rect,
    icon: Icon,
    text: &str,
    color: Color32,
    font_size: f32,
    strong: bool,
) {
    let font = if strong {
        FontId::proportional(font_size)
    } else {
        FontId::proportional(font_size)
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, color);
    let icon_size = 16.0;
    let gap = 7.0;
    let total = icon_size + gap + galley.size().x;
    let left = rect.center().x - total / 2.0;
    let icon_rect = Rect::from_min_size(
        Pos2::new(left, rect.center().y - icon_size / 2.0),
        Vec2::splat(icon_size),
    );
    icons::paint(ui.painter(), icon_rect, icon, color);
    let text_pos = Pos2::new(
        left + icon_size + gap,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, color);
}
/// Dashed outline for a rounded rectangle, sampled as a closed path.
pub fn dashed_round_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: f32,
    stroke: egui::Stroke,
) {
    let mut points = Vec::new();
    let corners = [
        (rect.right_bottom() - egui::vec2(radius, radius), 0.0_f32),
        (rect.left_bottom() + egui::vec2(radius, -radius), 90.0_f32),
        (rect.left_top() + egui::vec2(radius, radius), 180.0_f32),
        (rect.right_top() + egui::vec2(-radius, radius), 270.0_f32),
    ];
    for (center, start) in corners {
        for step in 0..=7 {
            let angle = (start + 90.0 * step as f32 / 7.0).to_radians();
            points.push(center + egui::vec2(angle.cos(), angle.sin()) * radius);
        }
    }
    points.push(points[0]);
    painter.add(egui::Shape::dashed_line(&points, stroke, 4.0, 3.0));
}
