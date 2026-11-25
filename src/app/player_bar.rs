//! Bottom transport bar and the play-queue popover.

use eframe::egui;
use egui::{
    Align, Align2, CornerRadius, FontId, Frame, Label, Layout, Margin, Pos2, Rect,
    RichText, ScrollArea, Sense, Stroke, Ui, Vec2,
};

use super::RPlayerApp;
use crate::model;
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

impl RPlayerApp {
    pub(super) fn player_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("rplayer-player-bar")
            .exact_height(92.0)
            .frame(
                Frame::NONE
                    .fill(theme::sidebar_deep())
                    .inner_margin(Margin::symmetric(16, 6)),
            )
            .show(ctx, |ui| {
                let screen = ui.ctx().screen_rect();
                let bar_top = ui.max_rect().top();
                let bar_bottom = ui.max_rect().bottom();
                let full = Rect::from_min_max(
                    Pos2::new(screen.left() + 20.0, bar_top),
                    Pos2::new(screen.right() - 20.0, bar_bottom),
                );
                let width = full.width();
                // Symmetric side columns keep the transport cluster at the
                // real window centre instead of the centre of leftover space.
                let side = if width >= 1040.0 { 335.0 } else { 280.0 };
                let center = (width - side * 2.0 - 24.0).max(300.0);

                ui.scope_builder(egui::UiBuilder::new().max_rect(full), |ui| {
                    ui.set_clip_rect(screen);
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.allocate_ui_with_layout(
                            Vec2::new(side, ui.available_height()),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.set_min_width(side);
                                self.player_info(ui);
                            },
                        );

                        ui.add_space(12.0);
                        // 50px transport row + 3px gap + 18px seek row. Using
                        // the content height (instead of stretching to the
                        // whole bar) lets the parent `Align::Center` place the
                        // entire transport block on the bar's vertical center.
                        ui.allocate_ui_with_layout(
                            Vec2::new(center, 71.0),
                            Layout::top_down(Align::Center),
                            |ui| {
                                ui.set_min_width(center);
                                self.player_controls(ui, ctx);
                            },
                        );

                        ui.add_space(12.0);
                        ui.allocate_ui_with_layout(
                            Vec2::new(side, ui.available_height()),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                ui.set_min_width(side);
                                self.player_extras(ui);
                            },
                        );
                    });
                });
            });
    }

    fn player_info(&mut self, ui: &mut Ui) {
        let cover_size = 48.0;
        let (cover_rect, cover_response) =
            ui.allocate_exact_size(Vec2::splat(cover_size), Sense::click());
        if let Some(texture) = self.cover_for_current() {
            ui.painter().add(egui::Shape::image(
                texture.id(),
                cover_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                crate::theme::ON_MEDIA,
            ));
        } else {
            let painter = ui.painter();
            painter.rect_filled(cover_rect, CornerRadius::same(2), theme::sidebar_line());
            icons::paint(
                painter,
                cover_rect.shrink(14.0),
                Icon::Music,
                theme::sidebar_faint(),
            );
        }
        if cover_response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("打开正在播放")
            .clicked()
            && self.current.is_some()
        {
            self.expanded = true;
            self.queue_open = false;
        }

        ui.add_space(12.0);
        let track = self.current.and_then(|id| self.library.track(id)).cloned();
        let info = ui.allocate_ui_with_layout(
            Vec2::new((ui.available_width()).max(80.0), 48.0),
            Layout::top_down(Align::LEFT),
            |ui| {
                ui.set_max_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 1.0;
                match &track {
                    Some(track) => {
                        ui.add(
                            Label::new(
                                RichText::new(&track.title)
                                    .size(12.5)
                                    .strong()
                                    .color(theme::sidebar_text()),
                            )
                            .truncate(),
                        );
                        ui.add(
                            Label::new(
                                RichText::new(&track.artist)
                                    .size(10.0)
                                    .color(theme::sidebar_muted()),
                            )
                            .truncate(),
                        );
                    }
                    None => {
                        ui.label(
                            RichText::new("未在播放")
                                .size(12.5)
                                .color(theme::sidebar_muted()),
                        );
                        ui.label(
                            RichText::new("从音乐库中选择一首歌")
                                .size(9.5)
                                .color(theme::sidebar_faint()),
                        );
                    }
                }
            },
        );
        if info
            .response
            .interact(Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
            && self.current.is_some()
        {
            self.expanded = true;
            self.queue_open = false;
        }
    }

    fn player_controls(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let width = ui.available_width();
        let controls_width = 158.0;

        ui.allocate_ui_with_layout(
            Vec2::new(width, 50.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.add_space(((width - controls_width) / 2.0).max(0.0));

                if widgets::sidebar_icon_button(ui, Icon::Prev, 30.0, "上一首").clicked() {
                    self.previous_track(ctx);
                }

                let enabled = self.current.is_some() || !self.library.tracks.is_empty();
                if widgets::play_button(ui, self.is_playing(), enabled).clicked() {
                    self.toggle_play(ctx);
                }

                if widgets::sidebar_icon_button(ui, Icon::Next, 30.0, "下一首").clicked() {
                    self.next_track(ctx, false);
                }

                if widgets::sidebar_icon_button_tinted(
                    ui,
                    self.play_mode.icon(),
                    30.0,
                    self.play_mode.active(),
                    self.play_mode.tooltip(),
                )
                .clicked()
                {
                    self.cycle_play_mode();
                }
            },
        );

        ui.add_space(3.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(
                RichText::new(model::format_duration(self.current_position()))
                    .font(FontId::monospace(10.5))
                    .color(theme::c().faint),
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
                    .color(theme::c().faint),
            );
        });
    }

    fn player_extras(&mut self, ui: &mut Ui) {
        let compact = ui.available_width() < 300.0;
        let icon_size = if compact { 28.0 } else { 31.0 };
        ui.spacing_mut().item_spacing.x = if compact { 3.0 } else { 4.0 };

        let rail_ready = Self::rail_available(ui.ctx()) && self.current.is_some();
        // The panel toggle also exists in the header; drop it first when the
        // right column gets tight, keeping volume + equalizer + lyrics usable.
        if !compact
            && widgets::sidebar_icon_button_tinted(
                ui,
                Icon::PanelRight,
                34.0,
                rail_ready && self.now_rail_open,
                "显示 / 收起正在播放侧栏",
            )
            .clicked()
        {
            if self.current.is_some() && Self::rail_available(ui.ctx()) {
                self.now_rail_open = !self.now_rail_open;
                if self.now_rail_open {
                    self.rail_queue = false;
                    self.queue_open = false;
                }
            } else {
                self.toast("窗口再宽一些就能显示播放侧栏", super::ToastKind::Info);
            }
        }

        let queue_active = self.queue_open
            || (rail_ready && self.now_rail_open && self.rail_queue);
        if widgets::sidebar_icon_button_tinted(ui, Icon::Queue, icon_size, queue_active, "播放队列")
        .clicked()
        {
            if rail_ready {
                if self.now_rail_open && self.rail_queue {
                    self.rail_queue = false;
                } else {
                    self.now_rail_open = true;
                    self.rail_queue = true;
                    self.queue_open = false;
                }
            } else {
                self.queue_open = !self.queue_open;
            }
        }

        if widgets::sidebar_glyph_button(
            ui,
            "词",
            icon_size,
            self.desktop_lyrics_open,
            "显示 / 隐藏桌面歌词",
        )
        .clicked()
        {
            self.toggle_desktop_lyrics(ui.ctx());
        }

        if widgets::sidebar_icon_button_tinted(ui, Icon::Equalizer, icon_size, self.eq_open, "均衡器")
        .clicked()
        {
            self.toggle_eq();
        }

        if widgets::sidebar_icon_button_tinted(ui, Icon::MiniPlayer, icon_size, self.mini_open, "迷你播放器")
        .clicked()
        {
            self.toggle_mini(ui.ctx());
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
        if widgets::sidebar_icon_button(ui, icon, icon_size, if muted { "取消静音" } else { "静音" }).clicked()
        {
            self.toggle_mute();
        }

        if widgets::sidebar_icon_button(ui, Icon::ChevronUp, icon_size, "打开正在播放").clicked()
            && self.current.is_some()
        {
            self.queue_open = false;
            self.expanded = true;
        }
    }

    pub(super) fn queue_window(&mut self, ctx: &egui::Context) {
        let slide = ctx.animate_bool_with_time(
            egui::Id::new("rplayer-queue-motion"),
            self.queue_open,
            crate::ui::motion::PAGE,
        );
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
            .anchor(
                Align2::RIGHT_TOP,
                Vec2::new(
                    (1.0 - slide) * 36.0,
                    crate::ui::tokens::TOPBAR_HEIGHT,
                ),
            )
            .title_bar(false)
            .resizable(false)
            .fixed_size(Vec2::new(
                360.0,
                (ctx.screen_rect().height()
                    - crate::ui::tokens::TOPBAR_HEIGHT
                    - crate::ui::tokens::PLAYER_HEIGHT)
                    .max(280.0),
            ))
            .frame(
                Frame::NONE
                    .fill(theme::c().panel)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::ZERO)
                    .inner_margin(Margin::same(18))
                    .shadow(egui::epaint::Shadow::NONE),
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
                                .color(theme::c().text),
                        );
                        ui.label(
                            RichText::new(format!("{} 首歌曲", queue.len()))
                                .size(10.5)
                                .color(theme::c().faint),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, Icon::Close, 26.0, "关闭").clicked() {
                            action = Action::Close;
                        }
                        if !queue.is_empty() {
                            let clear = ui.add(
                                Label::new(RichText::new("清空").size(11.0).color(theme::c().muted))
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
                            theme::c().faint,
                        );
                        ui.add_space(28.0);
                        ui.label(RichText::new("队列空空的").size(12.0).color(theme::c().muted));
                        ui.label(
                            RichText::new("双击歌曲后，这里会显示接下来要播放的内容")
                                .size(10.5)
                                .color(theme::c().faint),
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
                                painter.rect_filled(rect, CornerRadius::same(8), theme::c().card);
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
                                        theme::c().accent,
                                    );
                                }
                            } else {
                                painter.text(
                                    number_pos,
                                    Align2::CENTER_CENTER,
                                    format!("{}", index + 1),
                                    FontId::proportional(10.5),
                                    theme::c().faint,
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
                                if current { theme::c().accent } else { theme::c().text },
                            );
                            title_clip.text(
                                Pos2::new(text_left, rect.center().y + 8.0),
                                Align2::LEFT_CENTER,
                                &track.artist,
                                FontId::proportional(10.0),
                                theme::c().faint,
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
