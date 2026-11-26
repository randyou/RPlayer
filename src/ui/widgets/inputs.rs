use super::*;

pub(crate) fn search_box(ui: &mut Ui, query: &mut String, hint: &str) -> Response {
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

pub(crate) fn seek_bar(ui: &mut Ui, position: f64, duration: f64) -> Option<f64> {
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
pub(crate) fn volume_bar(ui: &mut Ui, volume: f32) -> Option<f32> {
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

pub(crate) fn rating_stars(ui: &mut Ui, rating: u8, star_size: f32) -> Option<u8> {
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

