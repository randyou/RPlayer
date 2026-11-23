//! Persistent right-hand Now Playing rail, inspired by Spotify's desktop
//! Now Playing view: cover, metadata, lyric preview and upcoming queue without
//! leaving the library.

use eframe::egui;
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Label, Layout, Margin, Pos2, Rect,
    RichText, ScrollArea, Sense, Stroke, Ui, Vec2,
};

use super::RPlayerApp;
use crate::model;
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

impl RPlayerApp {
    pub(super) fn rail_available(ctx: &egui::Context) -> bool {
        ctx.screen_rect().width() >= 1200.0
    }

    pub(super) fn now_rail_panel(&mut self, ctx: &egui::Context) {
        if !self.now_rail_open || self.current.is_none() || !Self::rail_available(ctx) {
            return;
        }

        egui::SidePanel::right("rplayer-now-rail")
            .exact_width(318.0)
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(theme::c().sidebar)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .inner_margin(Margin::symmetric(16, 14)),
            )
            .show(ctx, |ui| self.rail_contents(ui));
    }

    fn rail_contents(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if self.rail_queue {
                    "播放队列"
                } else {
                    "正在播放"
                })
                .size(15.0)
                .strong()
                .color(theme::c().text),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, Icon::Close, 24.0, "收起侧栏").clicked() {
                    self.now_rail_open = false;
                }
            });
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if widgets::chip(ui, "正在播放", !self.rail_queue).clicked() {
                self.rail_queue = false;
            }
            if widgets::chip(
                ui,
                &format!("队列 {}", self.queue.len()),
                self.rail_queue,
            )
            .clicked()
            {
                self.rail_queue = true;
            }
        });
        ui.add_space(12.0);

        if self.rail_queue {
            self.rail_queue_list(ui);
        } else {
            ScrollArea::vertical()
                .id_salt("rplayer-rail-now")
                .auto_shrink([false, false])
                .show(ui, |ui| self.rail_now(ui));
        }
    }

    fn rail_now(&mut self, ui: &mut Ui) {
        let Some(track) = self.current.and_then(|id| self.library.track(id)).cloned() else {
            return;
        };

        let cover_size = ui.available_width().min(190.0);
        let (cover_rect, cover_response) =
            ui.allocate_exact_size(Vec2::splat(cover_size), Sense::click());
        let cover_response = cover_response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("打开正在播放");
        ui.painter().rect_filled(
            cover_rect,
            CornerRadius::same(16),
            theme::c().card,
        );
        if let Some(texture) = &self.cover {
            ui.painter().add(egui::Shape::image(
                texture.id(),
                cover_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            ));
        } else {
            ui.painter().circle_filled(
                cover_rect.center(),
                cover_rect.width() * 0.34,
                theme::accent_alpha(0.05),
            );
            ui.painter().circle_stroke(
                cover_rect.center(),
                cover_rect.width() * 0.27,
                Stroke::new(1.0_f32, theme::accent_alpha(0.12)),
            );
            icons::paint(
                ui.painter(),
                Rect::from_center_size(cover_rect.center(), Vec2::splat(46.0)),
                Icon::Music,
                theme::c().faint,
            );
            if self.cover_fetching == self.current {
                ui.painter().text(
                    Pos2::new(cover_rect.center().x, cover_rect.bottom() - 22.0),
                    Align2::CENTER_CENTER,
                    "正在匹配封面…",
                    FontId::proportional(10.5),
                    theme::c().faint,
                );
            } else if self.library.auto_cover {
                ui.painter().text(
                    Pos2::new(cover_rect.center().x, cover_rect.bottom() - 22.0),
                    Align2::CENTER_CENTER,
                    "未找到封面 · 可重试",
                    FontId::proportional(10.5),
                    theme::c().faint,
                );
            }
        }
        if cover_response.clicked() {
            self.rail_queue = false;
            self.expanded = true;
        }

        ui.add_space(10.0);
        ui.add(
            Label::new(
                RichText::new(&track.title)
                    .size(17.0)
                    .strong()
                    .color(theme::c().text),
            )
            .truncate(),
        );
        ui.add(
            Label::new(RichText::new(&track.artist).size(11.5).color(theme::c().muted))
                .truncate(),
        );
        ui.add(
            Label::new(RichText::new(&track.album).size(10.5).color(theme::c().faint))
                .truncate(),
        );

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if let Some(source) = self.cover_source {
                source_badge(ui, &format!("封面·{}", source.short()));
            }
            if self.lyrics_for == Some(track.id) && self.lyrics.is_some() {
                source_badge(
                    ui,
                    &format!(
                        "歌词·{}",
                        self.lyrics_source.as_deref().unwrap_or("本地")
                    ),
                );
            }
            if self.cover_fetching == self.current {
                source_badge(ui, "匹配中");
            }
        });

        ui.add_space(8.0);
        widgets::divider(ui);
        ui.add_space(7.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("歌词")
                    .size(10.5)
                    .strong()
                    .color(theme::c().faint),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        Label::new(
                            RichText::new("查看全部")
                                .size(10.0)
                                .color(theme::c().accent),
                        )
                        .sense(Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.expanded = true;
                }
            });
        });
        ui.add_space(5.0);
        self.rail_lyrics(ui, &track);

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.spacing_mut().item_spacing.y = 2.0;
            let favorite = self.library.is_favorite(track.id);
            if widgets::icon_button_tinted(
                ui,
                if favorite {
                    Icon::HeartFilled
                } else {
                    Icon::Heart
                },
                28.0,
                if favorite { theme::c().accent } else { theme::c().muted },
                favorite,
                if favorite { "取消收藏" } else { "收藏" },
            )
            .clicked()
            {
                self.toggle_favorite(track.id);
            }
            if widgets::icon_button(
                ui,
                Icon::Download,
                28.0,
                "从 LRCLIB 下载歌词",
            )
            .clicked()
            {
                self.request_lyrics(ui.ctx(), track.id, true);
            }
            if widgets::icon_button(ui, Icon::Refresh, 28.0, "重新匹配封面").clicked() {
                self.refresh_cover(ui.ctx());
            }
            if widgets::icon_button(ui, Icon::Expand, 28.0, "打开完整播放页").clicked() {
                self.rail_queue = false;
                self.expanded = true;
            }
            if widgets::icon_button_tinted(
                ui,
                Icon::Lyrics,
                28.0,
                theme::c().muted,
                self.desktop_lyrics_open,
                "桌面歌词",
            )
            .clicked()
            {
                self.toggle_desktop_lyrics(ui.ctx());
            }
            if widgets::icon_button_tinted(
                ui,
                Icon::MiniPlayer,
                28.0,
                theme::c().muted,
                self.mini_open,
                "迷你播放器",
            )
            .clicked()
            {
                self.toggle_mini(ui.ctx());
            }
        });

    }

    fn rail_lyrics(&mut self, ui: &mut Ui, track: &model::Track) {
        let current = (self.current_position() - self.lyrics_offset() as f64).max(0.0);
        let (lines, active) = {
            let Some(lyrics) = self
                .lyrics
                .as_ref()
                .filter(|_| self.lyrics_for == Some(track.id))
            else {
                ui.label(
                    RichText::new("暂无歌词")
                        .size(12.0)
                        .color(theme::c().muted),
                );
                ui.label(
                    RichText::new("可自动从 LRCLIB 匹配，或右键歌曲手动下载")
                        .size(10.0)
                        .color(theme::c().faint),
                );
                return;
            };
            let lines: Vec<(String, f64)> = lyrics
                .lines
                .iter()
                .map(|line| (line.text.clone(), line.time))
                .collect();
            (lines, lyrics.active_index(current))
        };

        let start = active.map(|index| index.saturating_sub(1)).unwrap_or(0);
        let visible = 3;
        let mut shown = 0;
        for (index, (text, time)) in lines.iter().enumerate().skip(start).take(visible) {
            if text.trim().is_empty() {
                continue;
            }
            let is_active = active == Some(index);
            let response = ui.add(
                Label::new(
                    RichText::new(text)
                        .size(if is_active { 13.0 } else { 11.0 })
                        .strong()
                        .color(if is_active {
                            theme::c().accent
                        } else if active.is_some_and(|active| index.abs_diff(active) == 1) {
                            theme::c().text
                        } else {
                            theme::c().faint
                        }),
                )
                .truncate()
                .sense(Sense::click()),
            );
            if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                self.seek_to(*time);
            }
            shown += 1;
        }
        if shown == 0 {
            ui.label(
                RichText::new("这首歌暂无逐行歌词")
                    .size(11.0)
                    .color(theme::c().faint),
            );
        }
    }

    fn rail_queue_list(&mut self, ui: &mut Ui) {
        let queue = self.queue.clone();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} 首歌曲", queue.len()))
                    .size(10.5)
                    .color(theme::c().faint),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if !queue.is_empty()
                    && ui
                        .add(
                            Label::new(
                                RichText::new("清空")
                                    .size(10.5)
                                    .color(theme::c().muted),
                            )
                            .sense(Sense::click()),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                {
                    self.clear_queue();
                }
            });
        });
        ui.add_space(6.0);

        if queue.is_empty() {
            ui.add_space(28.0);
            ui.vertical_centered(|ui| {
                icons::paint(
                    ui.painter(),
                    Rect::from_center_size(
                        ui.cursor().center() + Vec2::new(0.0, 10.0),
                        Vec2::splat(24.0),
                    ),
                    Icon::Queue,
                    theme::c().faint,
                );
                ui.add_space(26.0);
                ui.label(RichText::new("队列空空的").size(12.0).color(theme::c().muted));
            });
            return;
        }

        ScrollArea::vertical()
            .id_salt("rplayer-rail-queue")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (index, id) in queue.iter().enumerate() {
                    let Some(track) = self.library.track(*id).cloned() else {
                        continue;
                    };
                    let current = self.current == Some(*id);
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 42.0),
                        Sense::click_and_drag(),
                    );
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if response.drag_started() {
                        self.queue_drag = Some(index);
                    }
                    if response.hovered() && self.queue_drag.is_some_and(|from| from != index) {
                        ui.painter().hline(
                            rect.x_range(),
                            rect.top(),
                            Stroke::new(2.0_f32, theme::c().accent),
                        );
                        if ui.input(|input| input.pointer.any_released()) {
                            if let Some(from) = self.queue_drag.take() {
                                self.reorder_queue(from, index);
                            }
                        }
                    }
                    if current {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(8),
                            theme::accent_alpha(0.09),
                        );
                    } else if response.hovered() {
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(8), theme::c().card);
                    }
                    if current {
                        ui.painter().rect_filled(
                            Rect::from_min_size(
                                Pos2::new(rect.left(), rect.top() + 9.0),
                                Vec2::new(2.5, rect.height() - 18.0),
                            ),
                            CornerRadius::same(2),
                            theme::c().accent,
                        );
                    }
                    ui.painter().text(
                        Pos2::new(rect.left() + 14.0, rect.center().y - 6.0),
                        Align2::LEFT_CENTER,
                        &track.title,
                        FontId::proportional(11.5),
                        if current {
                            theme::c().accent
                        } else {
                            theme::c().text
                        },
                    );
                    ui.painter().text(
                        Pos2::new(rect.left() + 14.0, rect.center().y + 8.0),
                        Align2::LEFT_CENTER,
                        &track.artist,
                        FontId::proportional(9.5),
                        theme::c().faint,
                    );
                    ui.painter().text(
                        Pos2::new(rect.right() - 2.0, rect.center().y),
                        Align2::RIGHT_CENTER,
                        model::format_duration(track.duration),
                        FontId::monospace(9.5),
                        theme::c().faint,
                    );
                    if response.clicked() && self.queue_drag.is_none() {
                        let queue = self.queue.clone();
                        self.play_queue(ui.ctx(), queue, index);
                    }
                }
            });
        if ui.input(|input| input.pointer.any_released()) {
            self.queue_drag = None;
        }
    }
}

fn source_badge(ui: &mut Ui, text: &str) {
    Frame::NONE
        .fill(theme::accent_alpha(0.10))
        .stroke(Stroke::new(1.0_f32, theme::accent_alpha(0.25)))
        .corner_radius(CornerRadius::same(99))
        .inner_margin(Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(9.0).color(theme::c().accent));
        });
}
