//! Hand-painted vector icons.
//!
//! RPlayer deliberately avoids an icon font: every glyph is drawn from lines
//! and polygons so it stays crisp at any DPI and inherits the mint accent.

use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Play,
    Pause,
    Next,
    Prev,
    Shuffle,
    Repeat,
    RepeatOne,
    VolumeHigh,
    VolumeLow,
    VolumeMuted,
    Search,
    Folder,
    Music,
    List,
    Plus,
    Close,
    More,
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ChevronRight,
    Trash,
    Edit,
    Clock,
    Disc,
    Check,
    Grid,
    Queue,
    Heart,
    HeartFilled,
    Sun,
    Moon,
}

struct Canvas {
    painter: Painter,
    rect: Rect,
    color: Color32,
    stroke: f32,
}

impl Canvas {
    fn p(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(
            self.rect.min.x + self.rect.width() * x,
            self.rect.min.y + self.rect.height() * y,
        )
    }

    fn line(&self, a: (f32, f32), b: (f32, f32)) {
        self.painter.line_segment(
            [self.p(a.0, a.1), self.p(b.0, b.1)],
            Stroke::new(self.stroke, self.color),
        );
    }

    fn polyline(&self, points: &[(f32, f32)]) {
        self.painter.add(Shape::line(
            points.iter().map(|&(x, y)| self.p(x, y)).collect(),
            Stroke::new(self.stroke, self.color),
        ));
    }

    fn fill(&self, points: &[(f32, f32)]) {
        self.painter.add(Shape::convex_polygon(
            points.iter().map(|&(x, y)| self.p(x, y)).collect(),
            self.color,
            Stroke::NONE,
        ));
    }

    fn bar(&self, x0: f32, x1: f32, y0: f32, y1: f32, radius: f32) {
        let rect = Rect::from_min_max(self.p(x0, y0), self.p(x1, y1));
        self.painter
            .rect_filled(rect, radius * self.rect.width(), self.color);
    }

    fn arc(&self, cx: f32, cy: f32, radius: f32, from: f32, to: f32) {
        let steps = 16;
        let points: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let t = from + (to - from) * (i as f32 / steps as f32);
                self.p(cx + radius * t.cos(), cy + radius * t.sin())
            })
            .collect();
        self.painter.add(Shape::line(
            points,
            Stroke::new(self.stroke * 0.9, self.color),
        ));
    }

    fn disc(&self, cx: f32, cy: f32, radius: f32) {
        self.painter
            .circle_filled(self.p(cx, cy), radius * self.rect.width(), self.color);
    }

    fn ring(&self, cx: f32, cy: f32, radius: f32) {
        self.painter.circle_stroke(
            self.p(cx, cy),
            radius * self.rect.width(),
            Stroke::new(self.stroke, self.color),
        );
    }
}

/// Paints an icon centered in `rect`, inset by `padding`.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let side = rect.width().min(rect.height());
    if side <= 0.0 {
        return;
    }
    let square = Rect::from_center_size(rect.center(), Vec2::splat(side));
    let c = Canvas {
        painter: painter.clone(),
        rect: square,
        color,
        stroke: (side / 24.0 * 1.9).clamp(1.2, 3.0),
    };

    match icon {
        Icon::Play => {
            c.fill(&[(0.36, 0.22), (0.79, 0.50), (0.36, 0.78)]);
        }
        Icon::Pause => {
            c.bar(0.33, 0.44, 0.23, 0.77, 0.03);
            c.bar(0.57, 0.68, 0.23, 0.77, 0.03);
        }
        Icon::Next => {
            c.fill(&[(0.28, 0.24), (0.62, 0.50), (0.28, 0.76)]);
            c.bar(0.66, 0.75, 0.24, 0.76, 0.02);
        }
        Icon::Prev => {
            c.fill(&[(0.72, 0.24), (0.38, 0.50), (0.72, 0.76)]);
            c.bar(0.25, 0.34, 0.24, 0.76, 0.02);
        }
        Icon::Shuffle => {
            c.polyline(&[(0.10, 0.30), (0.32, 0.30), (0.62, 0.70), (0.78, 0.70)]);
            c.polyline(&[(0.68, 0.60), (0.78, 0.70), (0.68, 0.80)]);
            c.polyline(&[(0.10, 0.70), (0.32, 0.70), (0.44, 0.56)]);
            c.polyline(&[(0.44, 0.44), (0.62, 0.30), (0.78, 0.30)]);
            c.polyline(&[(0.68, 0.20), (0.78, 0.30), (0.68, 0.40)]);
        }
        Icon::Repeat => {
            c.arc(0.50, 0.50, 0.30, -2.55, 2.55);
            c.polyline(&[(0.68, 0.14), (0.82, 0.26), (0.67, 0.36)]);
            c.arc(0.50, 0.50, 0.30, 0.6, 3.6);
            c.polyline(&[(0.32, 0.86), (0.18, 0.74), (0.33, 0.64)]);
        }
        Icon::RepeatOne => {
            c.arc(0.50, 0.50, 0.30, -2.55, 2.55);
            c.polyline(&[(0.68, 0.14), (0.82, 0.26), (0.67, 0.36)]);
            c.arc(0.50, 0.50, 0.30, 0.6, 3.6);
            c.polyline(&[(0.32, 0.86), (0.18, 0.74), (0.33, 0.64)]);
            c.line((0.47, 0.36), (0.47, 0.64));
            c.polyline(&[(0.38, 0.44), (0.47, 0.36), (0.56, 0.44)]);
        }
        Icon::VolumeHigh | Icon::VolumeLow | Icon::VolumeMuted => {
            c.fill(&[
                (0.10, 0.40),
                (0.27, 0.40),
                (0.45, 0.22),
                (0.45, 0.78),
                (0.27, 0.60),
                (0.10, 0.60),
            ]);
            c.arc(0.44, 0.50, 0.16, -0.95, 0.95);
            if matches!(icon, Icon::VolumeHigh) {
                c.arc(0.44, 0.50, 0.28, -0.95, 0.95);
            }
            if matches!(icon, Icon::VolumeMuted) {
                c.line((0.62, 0.34), (0.86, 0.66));
                c.line((0.86, 0.34), (0.62, 0.66));
            }
        }
        Icon::Search => {
            c.ring(0.43, 0.43, 0.25);
            c.line((0.61, 0.61), (0.83, 0.83));
        }
        Icon::Folder => {
            let tab = Rect::from_min_max(c.p(0.09, 0.24), c.p(0.48, 0.38));
            let body = Rect::from_min_max(c.p(0.09, 0.33), c.p(0.91, 0.78));
            c.painter.rect_filled(tab, side * 0.035, c.color);
            c.painter.rect_filled(body, side * 0.035, c.color);
            let cut = Rect::from_min_max(c.p(0.16, 0.39), c.p(0.84, 0.71));
            c.painter.rect_filled(cut, side * 0.02, theme::PANEL);
        }
        Icon::Music => {
            c.disc(0.36, 0.73, 0.12);
            c.line((0.48, 0.72), (0.48, 0.24));
            c.polyline(&[(0.48, 0.24), (0.67, 0.28), (0.78, 0.37)]);
            c.line((0.48, 0.36), (0.78, 0.45));
        }
        Icon::List => {
            for (i, y) in [0.27, 0.50, 0.73].iter().enumerate() {
                c.disc(0.16, *y, 0.045 + i as f32 * 0.0);
                c.line((0.31, *y), (0.87, *y));
            }
        }
        Icon::Plus => {
            c.line((0.50, 0.20), (0.50, 0.80));
            c.line((0.20, 0.50), (0.80, 0.50));
        }
        Icon::Close => {
            c.line((0.24, 0.24), (0.76, 0.76));
            c.line((0.76, 0.24), (0.24, 0.76));
        }
        Icon::More => {
            c.disc(0.24, 0.50, 0.065);
            c.disc(0.50, 0.50, 0.065);
            c.disc(0.76, 0.50, 0.065);
        }
        Icon::ChevronDown => {
            c.polyline(&[(0.24, 0.40), (0.50, 0.64), (0.76, 0.40)]);
        }
        Icon::ChevronUp => {
            c.polyline(&[(0.24, 0.60), (0.50, 0.36), (0.76, 0.60)]);
        }
        Icon::ChevronLeft => {
            c.polyline(&[(0.60, 0.22), (0.36, 0.50), (0.60, 0.78)]);
        }
        Icon::ChevronRight => {
            c.polyline(&[(0.40, 0.22), (0.64, 0.50), (0.40, 0.78)]);
        }
        Icon::Trash => {
            c.line((0.18, 0.29), (0.82, 0.29));
            c.polyline(&[(0.38, 0.29), (0.40, 0.20), (0.60, 0.20), (0.62, 0.29)]);
            c.polyline(&[(0.25, 0.29), (0.30, 0.82), (0.70, 0.82), (0.75, 0.29)]);
            c.line((0.44, 0.41), (0.44, 0.69));
            c.line((0.57, 0.41), (0.57, 0.69));
        }
        Icon::Edit => {
            c.polyline(&[
                (0.25, 0.75),
                (0.31, 0.57),
                (0.69, 0.19),
                (0.81, 0.31),
                (0.43, 0.69),
                (0.25, 0.75),
            ]);
        }
        Icon::Clock => {
            c.ring(0.50, 0.50, 0.33);
            c.line((0.50, 0.50), (0.50, 0.29));
            c.line((0.50, 0.50), (0.66, 0.60));
        }
        Icon::Disc => {
            c.ring(0.50, 0.50, 0.36);
            c.ring(0.50, 0.50, 0.12);
        }
        Icon::Check => {
            c.polyline(&[(0.20, 0.52), (0.42, 0.73), (0.81, 0.27)]);
        }
        Icon::Grid => {
            for (x, y) in [(0.16, 0.16), (0.56, 0.16), (0.16, 0.56), (0.56, 0.56)] {
                let r = Rect::from_min_size(c.p(x, y), Vec2::splat(side * 0.28));
                c.painter.rect_stroke(
                    r,
                    side * 0.03,
                    Stroke::new(c.stroke, c.color),
                    egui::StrokeKind::Inside,
                );
            }
        }
        Icon::Queue => {
            c.polyline(&[(0.12, 0.30), (0.62, 0.30)]);
            c.polyline(&[(0.12, 0.50), (0.62, 0.50)]);
            c.polyline(&[(0.12, 0.70), (0.48, 0.70)]);
            c.fill(&[(0.64, 0.60), (0.88, 0.73), (0.64, 0.86)]);
        }
        Icon::Heart | Icon::HeartFilled => {
            let points = [
                (0.50, 0.84),
                (0.16, 0.51),
                (0.14, 0.34),
                (0.25, 0.21),
                (0.40, 0.21),
                (0.50, 0.34),
                (0.60, 0.21),
                (0.75, 0.21),
                (0.86, 0.34),
                (0.84, 0.51),
                (0.50, 0.84),
            ];
            if matches!(icon, Icon::HeartFilled) {
                c.fill(&points);
            } else {
                c.polyline(&points);
            }
        }
        Icon::Sun => {
            c.ring(0.50, 0.50, 0.17);
            for angle in 0..8 {
                let radians = angle as f32 * std::f32::consts::TAU / 8.0;
                let inner = 0.28;
                let outer = 0.40;
                c.line(
                    (
                        0.50 + inner * radians.cos(),
                        0.50 + inner * radians.sin(),
                    ),
                    (
                        0.50 + outer * radians.cos(),
                        0.50 + outer * radians.sin(),
                    ),
                );
            }
        }
        Icon::Moon => {
            c.arc(0.50, 0.50, 0.31, -1.35, 1.35);
            c.arc(0.63, 0.50, 0.24, 1.75, 4.50);
        }
    }
}

/// The RPlayer mark: a mint squircle with a dark "R" and two waveform bars.
pub fn logo(painter: &Painter, rect: Rect) {
    painter.rect_filled(rect, rect.width() * 0.28, theme::ACCENT);
    let glyph = rect.shrink(rect.width() * 0.14);
    painter.text(
        glyph.center() + Vec2::new(-rect.width() * 0.10, 0.0),
        Align2::CENTER_CENTER,
        "R",
        FontId::proportional(rect.width() * 0.46),
        theme::BG,
    );
    for (i, h) in [0.16f32, 0.27, 0.20].iter().enumerate() {
        let x = glyph.right() - rect.width() * 0.12 + i as f32 * rect.width() * 0.07;
        let bar = Rect::from_center_size(
            Pos2::new(x, glyph.center().y),
            Vec2::new(rect.width() * 0.032, rect.height() * h),
        );
        painter.rect_filled(bar, rect.width() * 0.02, theme::BG);
    }
}
