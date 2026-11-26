use super::*;

/// Small animated switch used by the equalizer and overlay settings.
pub(crate) fn toggle_switch(ui: &mut Ui, on: &mut bool) -> Response {
    toggle_switch_named(ui, on, "开关")
}

/// Same switch with an explicit AccessKit name for the surface it belongs to.
pub(crate) fn toggle_switch_named(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
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

