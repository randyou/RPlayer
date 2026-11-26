use super::*;

pub(crate) fn nav_item(
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
pub(crate) fn nav_icon_item(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
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

pub(crate) fn playlist_item(ui: &mut Ui, name: &str, selected: bool) -> Response {
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

