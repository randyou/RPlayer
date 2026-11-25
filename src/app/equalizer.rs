//! Floating ten-band equalizer surface.
//!
//! The panel is always painted with the Nocturne palette: it behaves like a
//! piece of hardware and keeps its dark face on the paper theme too.

use eframe::egui;
use egui::{
    Align, Align2, CornerRadius, CursorIcon, FontId, Frame, Key, Layout, Pos2, Rect, RichText, Sense,
    Stroke, Ui, Vec2,
};

use super::RPlayerApp;
use crate::eq::{EqSettings, BAND_COUNT, BAND_LABELS, MAX_GAIN, MIN_GAIN, PRESETS};
use crate::theme;
use crate::ui::icons::Icon;
use crate::ui::{tokens, widgets};

/// Content box; the 16px frame margin on each side makes the window 820×250.
const PANEL_SIZE: Vec2 = Vec2::new(788.0, 218.0);
const FADER_PITCH: f32 = 62.0;
const FADER_HEIGHT: f32 = 82.0;

impl RPlayerApp {
    pub(super) fn equalizer_window(&mut self, ctx: &egui::Context) {
        if !self.eq_open {
            return;
        }

        let mut settings = self.eq.settings();
        let mut changed = false;
        let mut close = false;

        theme::scoped(&theme::DARK, || {
            egui::Window::new("rplayer-equalizer")
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(PANEL_SIZE)
                .frame(
                    Frame::NONE
                        .fill(theme::c().panel)
                        .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(16))
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 18],
                            blur: 44,
                            spread: 0,
                            color: theme::c().shadow,
                        }),
                )
                .show(ctx, |ui| {
                    header(ui, &mut settings, &mut changed, &mut close);
                    ui.add_space(8.0);
                    preset_row(ui, &mut settings, &mut changed);
                    ui.add_space(10.0);
                    fader_row(ui, &mut settings, &mut changed);
                    ui.add_space(4.0);
                    footer(ui, &mut settings, &mut changed);
                });
        });

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

fn header(ui: &mut Ui, settings: &mut EqSettings, changed: &mut bool, close: &mut bool) {
    ui.horizontal(|ui| {
        ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
            ui.spacing_mut().item_spacing.x = 9.0;
            ui.label(
                RichText::new("EQUALIZER")
                    .font(tokens::mono(8.5))
                    .color(theme::c().faint),
            );
            ui.label(
                RichText::new("均衡器")
                    .font(tokens::serif(24.0))
                    .color(theme::c().text),
            );
            ui.label(
                RichText::new("10 段 · 仅作用于当前输出")
                    .size(9.5)
                    .color(theme::c().faint),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::icon_button(ui, Icon::Close, 24.0, "关闭均衡器").clicked() {
                *close = true;
            }
            ui.add_space(8.0);
            if widgets::toggle_switch_named(ui, &mut settings.enabled, "已启用").changed() {
                *changed = true;
            }
            ui.add_space(7.0);
            ui.label(
                RichText::new(if settings.enabled { "已启用" } else { "已关闭" })
                    .size(10.0)
                    .color(if settings.enabled {
                        theme::c().accent
                    } else {
                        theme::c().faint
                    }),
            );
        });
    });
}

fn preset_row(ui: &mut Ui, settings: &mut EqSettings, changed: &mut bool) {
    egui::ScrollArea::horizontal()
        .id_salt("eq-presets")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (index, (name, _)) in PRESETS.iter().enumerate() {
                    let selected = settings.preset == Some(index);
                    if widgets::pill(ui, name, selected).clicked() {
                        *settings = EqSettings::from_preset(index);
                        *changed = true;
                    }
                }
            });
        });
}

fn fader_row(ui: &mut Ui, settings: &mut EqSettings, changed: &mut bool) {
    let row_width = FADER_PITCH * BAND_COUNT as f32;
    let pad = ((ui.available_width() - row_width) * 0.5).max(0.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(pad);
        for index in 0..BAND_COUNT {
            ui.allocate_ui_with_layout(
                Vec2::new(FADER_PITCH, FADER_HEIGHT + 18.0),
                Layout::top_down(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 5.0;
                    if let Some(value) = fader(ui, index, settings.gains[index], settings.enabled) {
                        settings.gains[index] = value;
                        settings.preset = None;
                        *changed = true;
                    }
                    ui.label(
                        RichText::new(BAND_LABELS[index])
                            .font(tokens::mono(8.5))
                            .color(theme::c().faint),
                    );
                },
            );
        }
    });
}

fn footer(ui: &mut Ui, settings: &mut EqSettings, changed: &mut bool) {
    ui.add_space(4.0);
    widgets::divider(ui);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.label(
                RichText::new("PREAMP")
                    .font(tokens::mono(8.5))
                    .color(theme::c().faint),
            );
            ui.add_space(6.0);
            preamp_field(ui, settings, changed);
            ui.add_space(16.0);
            ui.label(
                RichText::new("修改即时生效，不写入音频文件")
                    .size(9.5)
                    .color(theme::c().faint),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::pill(ui, "重置", false).clicked() {
                let enabled = settings.enabled;
                *settings = EqSettings::default();
                settings.enabled = enabled;
                *changed = true;
            }
        });
    });
}

/// Preamp read-out: drag horizontally to trim the output level.
fn preamp_field(ui: &mut Ui, settings: &mut EqSettings, changed: &mut bool) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(58.0, 18.0), Sense::click_and_drag());
    let active = response.hovered() || response.dragged();
    ui.painter().text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        format!("{:+.1} dB", settings.preamp),
        FontId::monospace(9.5),
        if active {
            theme::c().accent
        } else {
            theme::c().text
        },
    );
    response.widget_info(|| egui::WidgetInfo::slider(true, settings.preamp as f64, "前级"));
    let response = response.on_hover_cursor(CursorIcon::ResizeHorizontal);
    let response = response.on_hover_text("拖动调节前级电平");
    if response.dragged() {
        let dx = ui.input(|input| input.pointer.delta().x);
        settings.preamp = (settings.preamp + dx * 0.1).clamp(MIN_GAIN, MAX_GAIN);
        settings.preset = None;
        *changed = true;
    }
    crate::ui::focus::focus_on_click(&response);
    if response.has_focus() {
        let delta = ui.input(|input| {
            if input.key_pressed(Key::ArrowLeft) {
                -0.5
            } else if input.key_pressed(Key::ArrowRight) {
                0.5
            } else {
                0.0
            }
        });
        if delta != 0.0 {
            settings.preamp = (settings.preamp + delta).clamp(MIN_GAIN, MAX_GAIN);
            settings.preset = None;
            *changed = true;
        }
    }
}

/// A thin vertical fader. Returns the new gain while the user drags.
fn fader(ui: &mut Ui, index: usize, value: f32, enabled: bool) -> Option<f32> {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(FADER_PITCH, FADER_HEIGHT), Sense::click_and_drag());
    let painter = ui.painter();
    let center_x = rect.center().x;
    let rail = Rect::from_min_max(
        Pos2::new(center_x - 0.75, rect.top() + 5.0),
        Pos2::new(center_x + 0.75, rect.bottom() - 5.0),
    );
    painter.line_segment(
        [rail.center_top(), rail.center_bottom()],
        Stroke::new(
            1.5_f32,
            if enabled {
                theme::c().border
            } else {
                theme::c().border_soft
            },
        ),
    );

    let fraction = ((value - MIN_GAIN) / (MAX_GAIN - MIN_GAIN)).clamp(0.0, 1.0);
    let knob_y = rail.bottom() - rail.height() * fraction;
    let active = enabled && (response.hovered() || response.dragged());
    let radius = if active { 6.5 } else { 5.5 };
    painter.circle_filled(
        Pos2::new(center_x, knob_y),
        radius,
        theme::c().panel,
    );
    painter.circle_stroke(
        Pos2::new(center_x, knob_y),
        radius,
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
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let response = response.on_hover_text(format!(
        "{} Hz  {:+.1} dB",
        BAND_LABELS[index], value
    ));
    response.widget_info(|| {
        egui::WidgetInfo::slider(
            true,
            value as f64,
            &format!("{} Hz 段增益", BAND_LABELS[index]),
        )
    });
    crate::ui::focus::focus_on_click(&response);
    if response.has_focus() {
        crate::ui::focus::ring_rect(ui, rect.shrink(8.0), 8);
        let delta = ui.input(|input| {
            if input.key_pressed(Key::ArrowUp) {
                0.5
            } else if input.key_pressed(Key::ArrowDown) {
                -0.5
            } else {
                0.0
            }
        });
        if delta != 0.0 {
            return Some((value + delta).clamp(MIN_GAIN, MAX_GAIN));
        }
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
