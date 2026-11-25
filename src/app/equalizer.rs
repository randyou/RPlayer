//! Floating ten-band equalizer surface.

use eframe::egui;
use egui::{
    Align, Align2, CornerRadius, FontId, Frame, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui,
    Vec2,
};

use super::RPlayerApp;
use crate::eq::{EqSettings, BAND_COUNT, BAND_LABELS, MAX_GAIN, MIN_GAIN, PRESETS};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

impl RPlayerApp {
    pub(super) fn equalizer_window(&mut self, ctx: &egui::Context) {
        if !self.eq_open {
            return;
        }

        let mut settings = self.eq.settings();
        let mut changed = false;
        let mut close = false;

        let previous_theme = theme::preset();
        if previous_theme != theme::ThemePreset::Forest {
            theme::set_preset(theme::ThemePreset::Forest);
        }

        egui::Window::new("rplayer-equalizer")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::RIGHT_CENTER, Vec2::new(-22.0, 0.0))
            .fixed_size(Vec2::new(548.0, 404.0))
            .frame(
                Frame::NONE
                    .fill(theme::c().panel)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(18))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 12],
                        blur: 30,
                        spread: 0,
                        color: theme::c().shadow,
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::hover());
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(11),
                        theme::accent_alpha(0.12),
                    );
                    icons::paint(
                        ui.painter(),
                        rect.shrink(9.0),
                        Icon::Equalizer,
                        theme::c().accent,
                    );
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(
                            RichText::new("EQUALIZER")
                                .font(crate::ui::tokens::mono(8.0))
                                .color(theme::c().faint),
                        );
                        ui.label(
                            RichText::new("均衡器")
                                .font(crate::ui::tokens::serif(24.0))
                                .color(theme::c().text),
                        );
                        ui.label(
                            RichText::new("10 段实时调节")
                                .size(10.5)
                                .color(theme::c().faint),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, Icon::Close, 28.0, "关闭均衡器").clicked() {
                            close = true;
                        }
                        if widgets::toggle_switch(ui, &mut settings.enabled).changed() {
                            changed = true;
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(if settings.enabled { "已启用" } else { "已关闭" })
                                .size(10.5)
                                .color(if settings.enabled {
                                    theme::c().accent
                                } else {
                                    theme::c().faint
                                }),
                        );
                    });
                });

                ui.add_space(14.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (index, (name, _)) in PRESETS.iter().enumerate() {
                        let selected = settings.preset == Some(index);
                        if widgets::chip(ui, name, selected).clicked() {
                            settings = EqSettings::from_preset(index);
                            changed = true;
                        }
                    }
                });

                ui.add_space(14.0);
                widgets::divider(ui);
                ui.add_space(14.0);

                ui.scope(|ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for index in 0..BAND_COUNT {
                            ui.allocate_ui_with_layout(
                                Vec2::new(40.0, 214.0),
                                Layout::top_down(Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!("{:+.1}", settings.gains[index]))
                                            .font(FontId::monospace(9.5))
                                            .color(if settings.gains[index].abs() > 0.01 {
                                                theme::c().accent
                                            } else {
                                                theme::c().faint
                                            }),
                                    );
                                    if let Some(value) =
                                        fader(ui, settings.gains[index], settings.enabled)
                                    {
                                        settings.gains[index] = value;
                                        settings.preset = None;
                                        changed = true;
                                    }
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(BAND_LABELS[index])
                                            .size(10.0)
                                            .strong()
                                            .color(theme::c().muted),
                                    );
                                },
                            );
                        }
                    });
                });

                ui.add_space(8.0);
                widgets::divider(ui);
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("前级")
                            .size(11.0)
                            .strong()
                            .color(theme::c().muted),
                    );
                    let response = ui.add(
                        egui::Slider::new(&mut settings.preamp, MIN_GAIN..=MAX_GAIN)
                            .show_value(false)
                            .trailing_fill(true),
                    );
                    if response.changed() {
                        changed = true;
                    }
                    ui.label(
                        RichText::new(format!("{:+.1} dB", settings.preamp))
                            .font(FontId::monospace(10.0))
                            .color(theme::c().faint),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::chip(ui, "重置", false).clicked() {
                            let enabled = settings.enabled;
                            settings = EqSettings::default();
                            settings.enabled = enabled;
                            changed = true;
                        }
                        if settings.is_flat() {
                            ui.label(
                                RichText::new("当前为平直曲线")
                                    .size(10.0)
                                    .color(theme::c().faint),
                            );
                        }
                    });
                });
            });

        if previous_theme != theme::ThemePreset::Forest {
            theme::set_preset(previous_theme);
        }

        if changed {
            self.eq.update(settings.clone());
            self.library.eq = settings;
            self.eq_dirty = true;
        }
        if close {
            if self.eq_dirty {
                self.persist();
                self.eq_dirty = false;
            }
            self.eq_open = false;
        }
        if self.eq_dirty && !ctx.input(|input| input.pointer.any_down()) {
            self.persist();
            self.eq_dirty = false;
        }
    }
}

/// A thin vertical fader. Returns the new gain while the user drags.
fn fader(ui: &mut Ui, value: f32, enabled: bool) -> Option<f32> {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(26.0, 170.0), Sense::click_and_drag());
    let painter = ui.painter();
    let rail = Rect::from_center_size(rect.center(), Vec2::new(5.0, rect.height() - 16.0));
    painter.rect_filled(rail, CornerRadius::same(3), theme::c().border);

    let fraction = ((value - MIN_GAIN) / (MAX_GAIN - MIN_GAIN)).clamp(0.0, 1.0);
    let knob_y = rail.bottom() - rail.height() * fraction;
    let zero_y = rail.center().y;
    let fill = Rect::from_min_max(
        Pos2::new(rail.left(), knob_y.min(zero_y)),
        Pos2::new(rail.right(), knob_y.max(zero_y)),
    );
    if fill.height() > 0.5 {
        painter.rect_filled(
            fill,
            CornerRadius::same(3),
            if enabled {
                theme::accent_alpha(0.55)
            } else {
                theme::c().border
            },
        );
    }
    painter.line_segment(
        [
            Pos2::new(rail.left() - 6.0, zero_y),
            Pos2::new(rail.right() + 6.0, zero_y),
        ],
        Stroke::new(1.0_f32, theme::c().border),
    );

    let active = response.hovered() || response.dragged();
    painter.circle_filled(
        Pos2::new(rail.center().x, knob_y),
        if active { 7.0 } else { 5.5 },
        if enabled { theme::c().panel } else { theme::c().card },
    );
    painter.circle_stroke(
        Pos2::new(rail.center().x, knob_y),
        if active { 7.0 } else { 5.5 },
        Stroke::new(
            2.0_f32,
            if enabled {
                theme::c().accent
            } else {
                theme::c().faint
            },
        ),
    );

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.double_clicked() {
        return Some(0.0);
    }
    let pointer = response.interact_pointer_pos()?;
    if response.clicked() || response.dragged() {
        let t = ((rail.bottom() - pointer.y) / rail.height()).clamp(0.0, 1.0);
        let gain = MIN_GAIN + t * (MAX_GAIN - MIN_GAIN);
        return Some((gain * 2.0).round() / 2.0);
    }
    None
}
