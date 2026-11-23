//! Bottom transport bar and the play-queue popover.

use eframe::egui;
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Label, Layout, Margin, Pos2, Rect,
    RichText, ScrollArea, Sense, Stroke, Ui, Vec2,
};

use super::{RPlayerApp, RepeatMode};
use crate::model;
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

impl RPlayerApp {
    pub(super) fn player_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("rplayer-player-bar")
            .exact_height(104.0)
            .frame(
                Frame::NONE
                    .fill(theme::SIDEBAR)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER_SOFT))
                    .inner_margin(Margin::symmetric(20, 10)),
            )
            .show(ctx, |ui| {
                let available = ui.available_width();
                let left = (available * 0.30).clamp(230.0, 380.0);
                let right = 206.0;
                let center = (available - left - right - 28.0).max(300.0);

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(left, ui.available_height()),
                        Layout::left_to_right(Align::Center),
                        |ui| self.player_info(ui),
                    );

                    ui.add_space(14.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(center, ui.available_height()),
                        Layout::top_down(Align::Center),
                        |ui| self.player_controls(ui, ctx),
                    );

                    ui.add_space(14.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(right, ui.available_height()),
                        Layout::right_to_left(Align::Center),
                        |ui| self.player_extras(ui),
                    );
                });
            });
    }

    fn player_info(&self, ui: &mut Ui) {
        let cover_size = 56.0;
        if let Some(texture) = self.cover_for_current() {
            ui.add(
                egui::Image::new(egui::load::SizedTexture::new(
                    texture.id(),
                    Vec2::splat(cover_size),
                ))
                .corner_radius(CornerRadius::same(9)),
            );
        } else {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(cover_size), Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(rect, CornerRadius::same(9), theme::CARD);
            painter.rect_stroke(
                rect,
                CornerRadius::same(9),
                Stroke::new(1.0_f32, theme::BORDER_SOFT),
                egui::StrokeKind::Inside,
            );
            icons::paint(painter, rect.shrink(17.0), Icon::Music, theme::FAINT);
        }

        ui.add_space(12.0);
        let track = self.current.and_then(|id| self.library.track(id));
        ui.allocate_ui_with_layout(
            Vec2::new((ui.available_width()).max(80.0), 56.0),
            Layout::top_down(Align::LEFT),
            |ui| {
                ui.set_max_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 2.0;
                match track {
                    Some(track) => {
                        ui.add(
                            Label::new(
                                RichText::new(&track.title)
                                    .size(13.5)
                                    .strong()
                                    .color(theme::TEXT),
                            )
                            .truncate(),
                        );
                        ui.add(
                            Label::new(RichText::new(&track.artist).size(11.5).color(theme::MUTED))
                                .truncate(),
                        );
                        ui.add(
                            Label::new(RichText::new(&track.album).size(10.5).color(theme::FAINT))
                                .truncate(),
                        );
                    }
                    None => {
                        ui.label(RichText::new("未在播放").size(13.0).color(theme::MUTED));
                        ui.label(
                            RichText::new("从音乐库中选择一首歌")
                                .size(10.5)
                                .color(theme::FAINT),
                        );
                    }
                }
            },
        );
    }

    fn player_controls(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let width = ui.available_width();
        let controls_width = 194.0;

        ui.allocate_ui_with_layout(
            Vec2::new(width, 50.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.add_space(((width - controls_width) / 2.0).max(0.0));

                if widgets::icon_button_tinted(
                    ui,
                    Icon::Shuffle,
                    30.0,
                    theme::MUTED,
                    self.shuffle,
                    "随机播放",
                )
                .clicked()
                {
                    self.shuffle = !self.shuffle;
                }

                if widgets::icon_button(ui, Icon::Prev, 30.0, "上一首").clicked() {
                    self.previous_track(ctx);
                }

                let enabled = self.current.is_some() || !self.library.tracks.is_empty();
                if widgets::play_button(ui, self.is_playing(), enabled).clicked() {
                    self.toggle_play(ctx);
                }

                if widgets::icon_button(ui, Icon::Next, 30.0, "下一首").clicked() {
                    self.next_track(ctx, false);
                }

                let repeat_active = self.repeat != RepeatMode::Off;
                if widgets::icon_button_tinted(
                    ui,
                    self.repeat.icon(),
                    30.0,
                    theme::MUTED,
                    repeat_active,
                    self.repeat.tooltip(),
                )
                .clicked()
                {
                    self.cycle_repeat();
                }
            },
        );

        ui.add_space(3.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(
                RichText::new(model::format_duration(self.current_position()))
                    .font(FontId::monospace(10.5))
                    .color(theme::FAINT),
            );

            let bar_width = (ui.available_width() - 96.0).max(80.0);
            let seek = ui
                .allocate_ui_with_layout(
                    Vec2::new(bar_width, 18.0),
                    Layout::left_to_right(Align::Center),
                    |ui| widgets::seek_bar(ui, self.current_position(), self.current_duration()),
                )
                .inner;
            if let Some(seconds) = seek {
                self.seek_to(seconds);
            }

            ui.label(
                RichText::new(model::format_duration(self.current_duration()))
                    .font(FontId::monospace(10.5))
                    .color(theme::FAINT),
            );
        });
    }

    fn player_extras(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.x = 6.0;

        if widgets::icon_button_tinted(
            ui,
            Icon::Queue,
            34.0,
            theme::MUTED,
            self.queue_open,
            "播放队列",
        )
        .clicked()
        {
            self.queue_open = !self.queue_open;
        }

        if let Some(volume) = widgets::volume_bar(ui, self.volume) {
            self.set_volume(volume);
        }

        let muted = self.volume <= 0.005;
        let icon = if muted {
            Icon::VolumeMuted
        } else if self.volume < 0.5 {
            Icon::VolumeLow
        } else {
            Icon::VolumeHigh
        };
        if widgets::icon_button(ui, icon, 30.0, if muted { "取消静音" } else { "静音" }).clicked()
        {
            self.toggle_mute();
        }
    }

    pub(super) fn queue_window(&mut self, ctx: &egui::Context) {
        if !self.queue_open {
            return;
        }

        enum Action {
            None,
            Play(usize),
            Clear,
            Close,
        }

        let queue = self.queue.clone();
        let mut action = Action::None;

        egui::Window::new("播放队列")
            .id(egui::Id::new("rplayer-queue-window"))
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-20.0, -116.0))
            .title_bar(false)
            .resizable(false)
            .default_width(360.0)
            .max_height(438.0)
            .frame(
                Frame::NONE
                    .fill(theme::PANEL)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER))
                    .corner_radius(CornerRadius::same(16))
                    .inner_margin(Margin::same(14))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 14],
                        blur: 44,
                        spread: 0,
                        color: Color32::from_black_alpha(150),
                    }),
            )
            .show(ctx, |ui| {
                ui.set_min_width(332.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(
                            RichText::new("播放队列")
                                .size(15.0)
                                .strong()
                                .color(theme::TEXT),
                        );
                        ui.label(
                            RichText::new(format!("{} 首歌曲", queue.len()))
                                .size(10.5)
                                .color(theme::FAINT),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, Icon::Close, 26.0, "关闭").clicked() {
                            action = Action::Close;
                        }
                        if !queue.is_empty() {
                            let clear = ui.add(
                                Label::new(RichText::new("清空").size(11.0).color(theme::MUTED))
                                    .sense(Sense::click()),
                            );
                            if clear
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                action = Action::Clear;
                            }
                        }
                    });
                });

                ui.add_space(8.0);
                widgets::divider(ui);
                ui.add_space(4.0);

                if queue.is_empty() {
                    ui.add_space(26.0);
                    ui.vertical_centered(|ui| {
                        icons::paint(
                            ui.painter(),
                            Rect::from_center_size(
                                ui.cursor().center() + Vec2::new(0.0, 12.0),
                                Vec2::splat(22.0),
                            ),
                            Icon::Queue,
                            theme::FAINT,
                        );
                        ui.add_space(28.0);
                        ui.label(RichText::new("队列空空的").size(12.0).color(theme::MUTED));
                        ui.label(
                            RichText::new("双击歌曲后，这里会显示接下来要播放的内容")
                                .size(10.5)
                                .color(theme::FAINT),
                        );
                    });
                    ui.add_space(26.0);
                    return;
                }

                let max_height = 318.0;
                ScrollArea::vertical()
                    .id_salt("rplayer-queue-list")
                    .max_height(max_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for (index, id) in queue.iter().enumerate() {
                            let Some(track) = self.library.track(*id).cloned() else {
                                continue;
                            };
                            let current = self.current == Some(*id);
                            let width = ui.available_width();
                            let (rect, response) =
                                ui.allocate_exact_size(Vec2::new(width, 42.0), Sense::click());
                            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                            let painter = ui.painter();

                            if current {
                                painter.rect_filled(
                                    rect,
                                    CornerRadius::same(8),
                                    theme::accent_alpha(0.09),
                                );
                            } else if response.hovered() {
                                painter.rect_filled(rect, CornerRadius::same(8), theme::CARD);
                            }

                            let number_pos = Pos2::new(rect.left() + 16.0, rect.center().y);
                            if current {
                                let time = ui.input(|i| i.time) as f32;
                                let playing = self.is_playing();
                                let bars = 3;
                                for bar in 0..bars {
                                    let phase = time * 7.0 + bar as f32 * 1.5;
                                    let height = if playing {
                                        3.0 + (phase.sin() * 0.5 + 0.5) * 9.0
                                    } else {
                                        3.0 + bar as f32 * 1.5
                                    };
                                    painter.rect_filled(
                                        Rect::from_min_size(
                                            Pos2::new(
                                                number_pos.x - 5.0 + bar as f32 * 4.0,
                                                number_pos.y + 6.0 - height,
                                            ),
                                            Vec2::new(2.2, height),
                                        ),
                                        CornerRadius::same(1),
                                        theme::ACCENT,
                                    );
                                }
                            } else {
                                painter.text(
                                    number_pos,
                                    Align2::CENTER_CENTER,
                                    format!("{}", index + 1),
                                    FontId::proportional(10.5),
                                    theme::FAINT,
                                );
                            }

                            let text_left = rect.left() + 34.0;
                            let title_clip = painter.with_clip_rect(Rect::from_min_max(
                                Pos2::new(text_left, rect.top()),
                                Pos2::new(rect.right() - 12.0, rect.bottom()),
                            ));
                            title_clip.text(
                                Pos2::new(text_left, rect.center().y - 7.5),
                                Align2::LEFT_CENTER,
                                &track.title,
                                FontId::proportional(12.0),
                                if current { theme::ACCENT } else { theme::TEXT },
                            );
                            title_clip.text(
                                Pos2::new(text_left, rect.center().y + 8.0),
                                Align2::LEFT_CENTER,
                                &track.artist,
                                FontId::proportional(10.0),
                                theme::FAINT,
                            );

                            if response.clicked() {
                                action = Action::Play(index);
                            }
                        }
                    });
            });

        match action {
            Action::Play(index) => self.play_queue(ctx, queue, index),
            Action::Clear => {
                self.queue.clear();
                self.queue_index = 0;
                if let Some(audio) = &mut self.audio {
                    audio.stop();
                }
                self.current = None;
                self.cover = None;
                self.cover_for = None;
            }
            Action::Close => self.queue_open = false,
            Action::None => {}
        }
    }
}
