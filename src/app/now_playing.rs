//! Full-window “now playing” view: ambient artwork background, large cover,
//! synced lyrics and a queue side panel.

use eframe::egui;
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Label, Layout, Margin, Pos2, Rect,
    RichText, ScrollArea, Sense, Stroke, Ui, Vec2,
};

use super::{RepeatMode, RPlayerApp};
use crate::model;
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

impl RPlayerApp {
    pub(super) fn now_playing(&mut self, ctx: &egui::Context) {
        if self.current.is_none() {
            self.expanded = false;
            return;
        }

        self.now_queue_panel(ctx);

        egui::TopBottomPanel::bottom("rplayer-now-controls")
            .exact_height(112.0)
            .frame(
                Frame::NONE
                    .fill(theme::c().sidebar)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .inner_margin(Margin {
                        left: 26,
                        right: 26,
                        top: 9,
                        bottom: 10,
                    }),
            )
            .show(ctx, |ui| self.now_controls(ui, ctx));

        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                ambient_background(ui, self.cover_tint);

                ui.horizontal(|ui| {
                    if widgets::icon_button(ui, Icon::ChevronDown, 34.0, "收起正在播放").clicked() {
                        self.expanded = false;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("正在播放")
                            .size(11.5)
                            .strong()
                            .color(theme::c().muted),
                    );
                    ui.label(RichText::new("·").size(11.0).color(theme::c().faint));
                    ui.label(
                        RichText::new("RPlayer")
                            .size(11.0)
                            .color(theme::c().faint),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button_tinted(
                            ui,
                            Icon::Queue,
                            32.0,
                            theme::c().muted,
                            self.queue_open,
                            "播放队列",
                        )
                        .clicked()
                        {
                            self.queue_open = !self.queue_open;
                        }

                        let favorite = self.current.is_some_and(|id| self.library.is_favorite(id));
                        if widgets::icon_button_tinted(
                            ui,
                            if favorite {
                                Icon::HeartFilled
                            } else {
                                Icon::Heart
                            },
                            32.0,
                            if favorite { theme::c().accent } else { theme::c().muted },
                            favorite,
                            if favorite { "取消收藏" } else { "收藏" },
                        )
                        .clicked()
                        {
                            if let Some(id) = self.current {
                                self.toggle_favorite(id);
                            }
                        }

                        ui.add_space(2.0);
                        if widgets::icon_button(
                            ui,
                            if self.theme_mode.is_dark() {
                                Icon::Sun
                            } else {
                                Icon::Moon
                            },
                            32.0,
                            "切换主题",
                        )
                        .clicked()
                        {
                            self.toggle_theme(ui.ctx());
                        }
                        if widgets::icon_button_tinted(
                            ui,
                            Icon::Equalizer,
                            32.0,
                            theme::c().muted,
                            self.eq_open,
                            "均衡器",
                        )
                        .clicked()
                        {
                            self.toggle_eq();
                        }
                        if widgets::icon_button_tinted(
                            ui,
                            Icon::Lyrics,
                            32.0,
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
                            32.0,
                            theme::c().muted,
                            self.mini_open,
                            "迷你播放器",
                        )
                        .clicked()
                        {
                            self.toggle_mini(ui.ctx());
                        }
                        if widgets::icon_button_tinted(
                            ui,
                            Icon::Settings,
                            32.0,
                            theme::c().muted,
                            self.settings_open,
                            "设置",
                        )
                        .clicked()
                        {
                            self.settings_open = !self.settings_open;
                        }
                    });
                });

                ui.add_space(8.0);

                let Some(track) = self.current.and_then(|id| self.library.track(id)).cloned() else {
                    return;
                };
                let content_height = ui.available_height();
                let cover_size = (content_height - 40.0).clamp(210.0, 380.0);
                let left_width = (ui.available_width() * 0.44).max(cover_size + 24.0);

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(left_width, content_height),
                        Layout::top_down(Align::Center),
                        |ui| {
                            let top_gap = ((content_height - cover_size) * 0.5 - 10.0).max(0.0);
                            ui.add_space(top_gap);
                            if let Some(texture) = self.cover_for_current() {
                                ui.add(
                                    egui::Image::new(egui::load::SizedTexture::new(
                                        texture.id(),
                                        Vec2::splat(cover_size),
                                    ))
                                    .corner_radius(CornerRadius::same(18)),
                                );
                            } else {
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(cover_size), Sense::hover());
                                let painter = ui.painter();
                                painter.rect_filled(rect, CornerRadius::same(18), theme::c().card);
                                painter.rect_stroke(
                                    rect,
                                    CornerRadius::same(18),
                                    Stroke::new(1.0_f32, theme::c().border),
                                    egui::StrokeKind::Inside,
                                );
                                painter.circle_filled(
                                    rect.center(),
                                    rect.width() * 0.34,
                                    theme::accent_alpha(0.05),
                                );
                                painter.circle_stroke(
                                    rect.center(),
                                    rect.width() * 0.27,
                                    Stroke::new(1.0_f32, theme::accent_alpha(0.12)),
                                );
                                icons::paint(
                                    painter,
                                    Rect::from_center_size(rect.center(), Vec2::splat(70.0)),
                                    Icon::Music,
                                    theme::c().faint,
                                );
                            }
                        },
                    );

                    ui.add_space(12.0);

                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), content_height),
                        Layout::top_down(Align::LEFT),
                        |ui| {
                            ui.add_space(26.0);
                            ui.add(
                                Label::new(
                                    RichText::new(&track.title)
                                        .size(32.0)
                                        .strong()
                                        .color(theme::c().text),
                                )
                                .truncate(),
                            );
                            ui.add_space(5.0);
                            ui.add(
                                Label::new(
                                    RichText::new(&track.artist).size(14.0).color(theme::c().muted),
                                )
                                .truncate(),
                            );
                            ui.add_space(2.0);
                            ui.add(
                                Label::new(
                                    RichText::new(format!(
                                        "{}  ·  {}  ·  {}",
                                        track.album,
                                        track.format_label(),
                                        model::format_duration(track.duration)
                                    ))
                                    .size(10.5)
                                    .color(theme::c().faint),
                                )
                                .truncate(),
                            );
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                let current_rating = self.rating(track.id);
                                if let Some(value) =
                                    widgets::rating_stars(ui, current_rating, 16.0)
                                {
                                    let next = if current_rating == value { 0 } else { value };
                                    self.set_rating(track.id, next);
                                }
                                let stats = self.stats.get(track.id);
                                if stats.play_count > 0 {
                                    ui.label(
                                        RichText::new(format!(
                                            "播放 {} 次 · 跳过 {} 次",
                                            stats.play_count, stats.skip_count
                                        ))
                                        .size(10.0)
                                        .color(theme::c().faint),
                                    );
                                }
                            });
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if let Some(source) = self.cover_source {
                                    page_badge(ui, &format!("封面·{}", source.short()));
                                }
                                if self.cover_fetching == self.current {
                                    page_badge(ui, "匹配中");
                                } else if self.cover.is_none() && self.library.auto_cover {
                                    page_badge(ui, "未匹配");
                                }
                                if widgets::icon_button(
                                    ui,
                                    Icon::Refresh,
                                    24.0,
                                    "重新匹配封面",
                                )
                                .clicked()
                                {
                                    self.refresh_cover(ui.ctx());
                                }
                                if widgets::icon_button(
                                    ui,
                                    Icon::Download,
                                    24.0,
                                    "下载歌词",
                                )
                                .clicked()
                                {
                                    self.download_current_lyrics(ui.ctx());
                                }
                            });
                            ui.add_space(12.0);

                            self.lyrics_panel(ui, &track, content_height);
                        },
                    );
                });
            });
    }

    fn lyrics_panel(&mut self, ui: &mut Ui, track: &model::Track, content_height: f32) {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("歌词")
                    .size(10.0)
                    .strong()
                    .color(theme::c().faint),
            );
            if let Some(source) = &self.lyrics_source {
                ui.label(
                    RichText::new(source)
                        .size(9.0)
                        .color(theme::c().faint),
                );
            }
            if widgets::icon_button(
                ui,
                Icon::Download,
                22.0,
                "从 LRCLIB 下载当前歌词",
            )
            .clicked()
            {
                self.download_current_lyrics(ui.ctx());
            }
            if self.lyrics_fetching == Some(track.id) {
                ui.label(
                    RichText::new("正在下载…")
                        .size(10.0)
                        .color(theme::c().accent),
                );
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let offset = self.lyrics_offset();
                if widgets::icon_button(ui, Icon::Plus, 20.0, "歌词延后 0.5 秒").clicked() {
                    self.adjust_lyrics_offset(0.5);
                }
                ui.label(
                    RichText::new(format!("{offset:+.1}s"))
                        .size(9.5)
                        .color(theme::c().faint),
                );
                if widgets::icon_button(ui, Icon::Refresh, 20.0, "歌词提前 0.5 秒").clicked() {
                    self.adjust_lyrics_offset(-0.5);
                }
            });
        });
        ui.add_space(10.0);

        let current = (self.current_position() - self.lyrics_offset() as f64).max(0.0);
        let (lines, active) = {
            let Some(lyrics) = self
                .lyrics
                .as_ref()
                .filter(|_| self.lyrics_for == Some(track.id))
            else {
                self.lyrics_placeholder(ui, track);
                return;
            };
            if lyrics.is_empty() {
                self.lyrics_placeholder(ui, track);
                return;
            }
            let lines: Vec<(String, f64)> = lyrics
                .lines
                .iter()
                .map(|line| (line.text.clone(), line.time))
                .collect();
            (lines, lyrics.active_index(current))
        };

        let start = active.map(|active| active.saturating_sub(4)).unwrap_or(0);
        let visible = 9;
        let line_height = 34.0;
        let top_gap = ((content_height - 160.0 - visible as f32 * line_height) * 0.5).max(0.0);
        ui.add_space(top_gap.min(24.0));

        let mut shown = 0;
        for (index, (text, time)) in lines.iter().enumerate().skip(start).take(visible) {
            if text.trim().is_empty() {
                ui.add_space(10.0);
                continue;
            }
            let is_active = active == Some(index);
            let color = if is_active {
                theme::c().accent
            } else if active.is_none() && index == start {
                theme::c().text
            } else if active.is_some_and(|active| index.abs_diff(active) == 1) {
                theme::c().muted
            } else {
                theme::c().faint
            };
            let size = if is_active { 22.0 } else { 14.0 };
            let response = ui.add(
                Label::new(
                    RichText::new(text)
                        .size(size)
                        .strong()
                        .color(color),
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
            ui.add_space(if is_active { 10.0 } else { 7.0 });
            shown += 1;
        }
        if shown == 0 {
            self.lyrics_placeholder(ui, track);
        }
    }

    fn lyrics_placeholder(&mut self, ui: &mut Ui, track: &model::Track) {
        ui.add_space(22.0);
        ui.vertical_centered(|ui| {
            Frame::NONE
                .fill(theme::c().card)
                .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                .corner_radius(CornerRadius::same(16))
                .inner_margin(Margin::same(22))
                .show(ui, |ui| {
                    ui.set_width(390.0);
                    icons::paint(
                        ui.painter(),
                        Rect::from_center_size(
                            ui.cursor().center() + Vec2::new(0.0, 20.0),
                            Vec2::splat(26.0),
                        ),
                        Icon::Lyrics,
                        theme::c().faint,
                    );
                    ui.add_space(38.0);
                    ui.label(
                        RichText::new("还没有歌词")
                            .size(20.0)
                            .strong()
                            .color(theme::c().text),
                    );
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(
                            "同名 .lrc、内嵌歌词会自动加载；也可以从 LRCLIB 联网匹配。",
                        )
                        .size(11.0)
                        .color(theme::c().muted),
                    );
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        let busy = self.lyrics_fetching == Some(track.id);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if busy {
                                        "正在搜索…"
                                    } else {
                                        "从 LRCLIB 搜索歌词"
                                    })
                                    .size(11.5)
                                    .color(theme::c().accent),
                                )
                                .fill(theme::accent_alpha(0.10))
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    theme::accent_alpha(0.28),
                                ))
                                .corner_radius(9)
                                .min_size(Vec2::new(0.0, 32.0)),
                            )
                            .clicked()
                        {
                            self.request_lyrics(ui.ctx(), track.id, true);
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("下载当前歌词")
                                        .size(11.0)
                                        .color(theme::c().muted),
                                )
                                .fill(theme::c().panel)
                                .stroke(Stroke::new(1.0_f32, theme::c().border))
                                .corner_radius(9)
                                .min_size(Vec2::new(0.0, 32.0)),
                            )
                            .clicked()
                        {
                            self.download_current_lyrics(ui.ctx());
                        }
                    });
                });
        });
    }

    fn now_controls(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(
                RichText::new(model::format_duration(self.current_position()))
                    .font(FontId::monospace(10.5))
                    .color(theme::c().muted),
            );
            let width = (ui.available_width() - 92.0).max(120.0);
            let seek = ui
                .allocate_ui_with_layout(
                    Vec2::new(width, 18.0),
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
                    .color(theme::c().muted),
            );
        });

        ui.add_space(3.0);

        let width = ui.available_width();
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new((width * 0.28).max(150.0), 56.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    if widgets::icon_button_tinted(
                        ui,
                        Icon::Shuffle,
                        30.0,
                        theme::c().muted,
                        self.shuffle,
                        "随机播放",
                    )
                    .clicked()
                    {
                        self.shuffle = !self.shuffle;
                    }
                    let repeat_active = self.repeat != RepeatMode::Off;
                    if widgets::icon_button_tinted(
                        ui,
                        self.repeat.icon(),
                        30.0,
                        theme::c().muted,
                        repeat_active,
                        self.repeat.tooltip(),
                    )
                    .clicked()
                    {
                        self.cycle_repeat();
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("正在播放")
                            .size(10.5)
                            .color(theme::c().faint),
                    );
                },
            );

            let controls_width = 210.0;
            ui.allocate_ui_with_layout(
                Vec2::new((width * 0.44).max(controls_width), 56.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.add_space(((ui.available_width() - controls_width) / 2.0).max(0.0));
                    ui.spacing_mut().item_spacing.x = 10.0;
                    if widgets::icon_button(ui, Icon::Prev, 34.0, "上一首").clicked() {
                        self.previous_track(ctx);
                    }
                    if widgets::play_button(ui, self.is_playing(), true).clicked() {
                        self.toggle_play(ctx);
                    }
                    if widgets::icon_button(ui, Icon::Next, 34.0, "下一首").clicked() {
                        self.next_track(ctx, false);
                    }
                },
            );

            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 56.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    if widgets::icon_button_tinted(
                        ui,
                        Icon::Queue,
                        30.0,
                        theme::c().muted,
                        self.queue_open,
                        "播放队列",
                    )
                    .clicked()
                    {
                        self.queue_open = !self.queue_open;
                    }
                    if widgets::icon_button(
                        ui,
                        if self.volume <= 0.005 {
                            Icon::VolumeMuted
                        } else if self.volume < 0.5 {
                            Icon::VolumeLow
                        } else {
                            Icon::VolumeHigh
                        },
                        28.0,
                        "静音",
                    )
                    .clicked()
                    {
                        self.toggle_mute();
                    }
                    if let Some(volume) = widgets::volume_bar(ui, self.volume) {
                        self.set_volume(volume);
                    }
                },
            );
        });
    }

    fn now_queue_panel(&mut self, ctx: &egui::Context) {
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

        egui::SidePanel::right("rplayer-now-queue-panel")
            .exact_width(316.0)
            .frame(
                Frame::NONE
                    .fill(Color32::from_rgb(14, 18, 17))
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .inner_margin(Margin::symmetric(18, 20)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(
                            RichText::new("播放队列")
                                .size(16.0)
                                .strong()
                                .color(theme::c().text),
                        );
                        ui.label(
                            RichText::new(format!("共 {} 首", queue.len()))
                                .size(10.5)
                                .color(theme::c().faint),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, Icon::Close, 26.0, "关闭队列").clicked() {
                            action = Action::Close;
                        }
                        if !queue.is_empty()
                            && widgets::icon_button(ui, Icon::Trash, 26.0, "清空队列").clicked()
                        {
                            action = Action::Clear;
                        }
                    });
                });

                ui.add_space(12.0);
                widgets::divider(ui);
                ui.add_space(5.0);

                if queue.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("队列空空的")
                                .size(12.0)
                                .color(theme::c().faint),
                        );
                    });
                    return;
                }

                ScrollArea::vertical()
                    .id_salt("rplayer-now-queue-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for (index, id) in queue.iter().enumerate() {
                            let Some(track) = self.library.track(*id).cloned() else {
                                continue;
                            };
                            let current = self.current == Some(*id);
                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 44.0),
                                Sense::click(),
                            );
                            let response =
                                response.on_hover_cursor(egui::CursorIcon::PointingHand);
                            if current {
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::same(8),
                                    theme::accent_alpha(0.09),
                                );
                            } else if response.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::same(8),
                                    theme::c().card,
                                );
                            }

                            let index_rect = Rect::from_min_size(
                                Pos2::new(rect.left() + 10.0, rect.top()),
                                Vec2::new(24.0, rect.height()),
                            );
                            if current {
                                let time = ui.input(|i| i.time) as f32;
                                painter_eq(ui.painter(), index_rect.center(), time, self.is_playing());
                            } else {
                                ui.painter().text(
                                    index_rect.center(),
                                    Align2::CENTER_CENTER,
                                    format!("{:02}", index + 1),
                                    FontId::monospace(10.5),
                                    theme::c().faint,
                                );
                            }

                            let text_rect = Rect::from_min_max(
                                Pos2::new(rect.left() + 38.0, rect.top()),
                                Pos2::new(rect.right() - 46.0, rect.bottom()),
                            );
                            ui.painter().with_clip_rect(text_rect).text(
                                Pos2::new(text_rect.left(), text_rect.center().y - 1.0),
                                Align2::LEFT_CENTER,
                                format!("{} · {}", track.title, track.artist),
                                FontId::proportional(11.5),
                                if current { theme::c().accent } else { theme::c().text },
                            );
                            ui.painter().text(
                                Pos2::new(rect.right() - 8.0, rect.center().y),
                                Align2::RIGHT_CENTER,
                                model::format_duration(track.duration),
                                FontId::monospace(10.0),
                                theme::c().faint,
                            );

                            if response.clicked() {
                                action = Action::Play(index);
                            }
                        }
                    });
            });

        match action {
            Action::Play(index) => {
                self.play_queue(ctx, queue, index);
            }
            Action::Clear => {
                if let Some(audio) = self.audio.as_mut() {
                    audio.stop();
                }
                self.queue.clear();
                self.queue_index = 0;
                self.current = None;
                self.cover = None;
                self.cover_for = None;
                self.lyrics = None;
                self.lyrics_for = None;
                self.queue_open = false;
                self.expanded = false;
            }
            Action::Close => {
                self.queue_open = false;
            }
            Action::None => {}
        }
    }
}

fn ambient_background(ui: &Ui, tint: Option<Color32>) {
    let rect = ui.max_rect();
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, theme::c().bg);
    let (mut top, mut bottom) = if theme::c().dark {
        (
            Color32::from_rgb(12, 20, 17),
            Color32::from_rgb(7, 10, 9),
        )
    } else {
        (
            Color32::from_rgb(250, 252, 248),
            Color32::from_rgb(234, 240, 232),
        )
    };
    if let Some(tint) = tint {
        // Derive a quiet version of the album palette instead of a generic
        // mint wash.
        top = blend(top, tint, 0.24);
        bottom = blend(bottom, tint, 0.14);
    }

    // Smooth four-vertex gradient; 36 hand-painted bands used to leave
    // visible horizontal seams on large windows.
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    ui.painter().add(egui::Shape::mesh(mesh));

    // Soft ambient glow behind the artwork. With premultiplied accent
    // colours this stays a whisper of mint instead of a solid disk.
    if let Some(tint) = tint {
        ui.painter().circle_filled(
            Pos2::new(
                rect.left() + rect.width() * 0.28,
                rect.top() + rect.height() * 0.42,
            ),
            rect.width() * 0.42,
            with_alpha(tint, if theme::c().dark { 0.20 } else { 0.14 }),
        );
    }
    let center = Pos2::new(
        rect.left() + rect.width() * 0.26,
        rect.top() + rect.height() * 0.54,
    );
    ui.painter().circle_filled(
        center,
        rect.width() * 0.36,
        theme::accent_alpha(0.035),
    );
    ui.painter().circle_filled(
        Pos2::new(
            rect.left() + rect.width() * 0.72,
            rect.top() + rect.height() * 0.20,
        ),
        rect.width() * 0.26,
        theme::accent_alpha(0.018),
    );
}


fn page_badge(ui: &mut Ui, text: &str) {
    Frame::NONE
        .fill(theme::accent_alpha(0.10))
        .stroke(Stroke::new(1.0_f32, theme::accent_alpha(0.25)))
        .corner_radius(CornerRadius::same(99))
        .inner_margin(Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(9.0).color(theme::c().accent));
        });
}

fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    let alpha = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
    Color32::from_rgba_premultiplied(
        (u16::from(color.r()) * u16::from(alpha) / 255) as u8,
        (u16::from(color.g()) * u16::from(alpha) / 255) as u8,
        (u16::from(color.b()) * u16::from(alpha) / 255) as u8,
        alpha,
    )
}

fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 * (1.0 - t) + b.r() as f32 * t) as u8,
        (a.g() as f32 * (1.0 - t) + b.g() as f32 * t) as u8,
        (a.b() as f32 * (1.0 - t) + b.b() as f32 * t) as u8,
    )
}

fn painter_eq(painter: &egui::Painter, center: Pos2, time: f32, playing: bool) {
    for bar in 0..4 {
        let phase = time * 6.0 + bar as f32 * 1.4;
        let height = if playing {
            4.0 + (phase.sin() * 0.5 + 0.5) * 10.0
        } else {
            4.0 + bar as f32 * 1.2
        };
        let x = center.x - 7.0 + bar as f32 * 4.0;
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(x, center.y + 7.0 - height),
                Vec2::new(2.4, height),
            ),
            CornerRadius::same(1),
            theme::c().accent,
        );
    }
}
