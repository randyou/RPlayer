use super::*;

pub(crate) fn chip(ui: &mut Ui, text: &str, selected: bool) -> Response {
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

/// A compact outlined pill used for segmented choices and tool presets.
pub(crate) fn pill(ui: &mut Ui, text: &str, selected: bool) -> Response {
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

