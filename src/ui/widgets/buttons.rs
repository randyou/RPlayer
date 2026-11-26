use super::*;

pub(crate) fn icon_button(ui: &mut Ui, icon: Icon, size: f32, tooltip: &str) -> Response {
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
pub(crate) fn sidebar_icon_button(ui: &mut Ui, icon: Icon, size: f32, tooltip: &str) -> Response {
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
pub(crate) fn sidebar_icon_button_tinted(
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

pub(crate) fn icon_button_tinted(
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
pub(crate) fn sidebar_glyph_button(
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
pub(crate) fn glyph_button_tinted(
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

pub(crate) fn circle_button(
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

pub(crate) fn play_button(ui: &mut Ui, playing: bool, enabled: bool) -> Response {
    play_button_sized(ui, playing, enabled, 50.0)
}

/// Same control at a caller-chosen diameter (D13 compact player bar uses 40).
pub(crate) fn play_button_sized(ui: &mut Ui, playing: bool, enabled: bool, size: f32) -> Response {
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

pub(crate) fn text_button(ui: &mut Ui, text: impl Into<egui::WidgetText>) -> Response {
    let response = ui.button(text).on_hover_cursor(CursorIcon::PointingHand);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

pub(crate) fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let button = Button::new(RichText::new(text).color(theme::c().on_accent).strong().size(14.0))
        .fill(theme::c().accent)
        .corner_radius(10)
        .min_size(Vec2::new(96.0, 40.0));
    let response = ui.add(button).on_hover_cursor(CursorIcon::PointingHand);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

pub(crate) fn secondary_button(ui: &mut Ui, text: &str) -> Response {
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
pub(crate) fn secondary_button_sized(ui: &mut Ui, text: &str, width: f32) -> Response {
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

pub(crate) fn danger_button(ui: &mut Ui, text: &str) -> Response {
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

/// Small primary button for the shared 32px dialog action band.
pub(crate) fn primary_button_compact(ui: &mut Ui, text: &str) -> Response {
    compact_button(ui, text, true)
}

/// Small secondary button for the shared 32px dialog action band.
pub(crate) fn secondary_button_compact(ui: &mut Ui, text: &str) -> Response {
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

/// Primary button with a hand-painted vector icon before the label.
pub(crate) fn primary_button_icon(ui: &mut Ui, icon: Icon, text: &str) -> Response {
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
pub(crate) fn secondary_button_icon(ui: &mut Ui, icon: Icon, text: &str) -> Response {
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
