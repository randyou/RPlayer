use super::*;

pub(crate) fn empty_illustration(ui: &mut Ui, size: f32) {
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

pub(crate) fn divider(ui: &mut Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0, theme::divider());
}

/// Dashed outline for a rounded rectangle, sampled as a closed path.
pub(crate) fn dashed_round_rect(
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
