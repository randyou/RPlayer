//! Appearance and lyric preferences.

use eframe::egui;
use egui::{
    Align, Align2, CornerRadius, FontId, Frame, Layout, Pos2, RichText, Sense, Stroke, Vec2,
};

use super::{NetFailure, RPlayerApp};
use crate::theme::{self, ThemePreset};
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

impl RPlayerApp {
    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }

        let mut close = false;
        let mut requested_theme: Option<ThemePreset> = None;
        let mut reduced_motion = self.library.reduced_motion;
        let mut requested_replaygain: Option<String> = None;
        let mut opacity = self.library.desktop_lyrics_opacity;
        let mut text_opacity = self.library.desktop_lyrics_text_opacity;
        let mut lyrics_font = if self.library.desktop_lyrics_font > 0.0 {
            self.library.desktop_lyrics_font
        } else {
            25.0
        };
        let mut lyrics_locked = self.library.desktop_lyrics_locked;
        let mut mini_style = if self.library.mini_style.is_empty() {
            "classic".to_string()
        } else {
            self.library.mini_style.clone()
        };
        let mut mini_on_top = self.library.mini_on_top;
        let mut mini_snap = self.library.mini_snap;
        let mut offline_mode = self.library.offline_mode;
        let mut auto_lyrics = self.library.auto_lyrics;
        let mut auto_cover = self.library.auto_cover;
        let mut clear_covers = false;
        let mut lyrics_open = self.desktop_lyrics_open;
        let mut download = false;
        let mut backup_requested = false;
        let mut restore_requested = false;
        let mut backup_media = false;
        let mut settings_only = false;
        let cover_cache = crate::cover::cache_size();

        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.settings_open = false;
            return;
        }

        // Keep the header (and its close button) inside the visible area even
        // on high-DPI / short windows, then scroll the long preference body.
        let available = ctx.screen_rect().size();
        let settings_size = Vec2::new(
            (available.x - 40.0).clamp(640.0, 760.0),
            (available.y - 80.0).clamp(500.0, 600.0),
        );
        let content_height = (settings_size.y - 118.0).max(300.0);

        egui::Window::new("rplayer-settings")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(settings_size)
            .frame(
                Frame::NONE
                    .fill(theme::c().panel)
                    .stroke(Stroke::new(1.0_f32, theme::c().border))
                    .corner_radius(CornerRadius::same(20))
                    .inner_margin(egui::Margin::same(22))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 18],
                        blur: 48,
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
                        Icon::Settings,
                        theme::c().accent,
                    );
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(
                            RichText::new("设置")
                                .size(18.0)
                                .strong()
                                .color(theme::c().text),
                        );
                        ui.label(
                            RichText::new("主题、桌面歌词与数据")
                                .size(10.5)
                                .color(theme::c().faint),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, Icon::Close, 28.0, "关闭设置").clicked() {
                            close = true;
                        }
                    });
                });

                ui.add_space(8.0);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(148.0, content_height),
                        Layout::top_down(Align::LEFT),
                        |ui| {
                            ui.add_space(6.0);
                            for (index, label) in [
                                "外观",
                                "桌面歌词",
                                "迷你播放器",
                                "离线模式",
                                "在线内容",
                                "数据",
                            ]
                            .iter()
                            .enumerate()
                            {
                                if settings_nav_item(ui, label, self.settings_tab == index)
                                    .clicked()
                                {
                                    self.settings_tab = index;
                                }
                            }
                        },
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .max_height(content_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                if self.settings_tab == 0 {
                section(ui, "外观");
                Frame::NONE
                    .fill(theme::c().card)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("主题")
                                .size(12.0)
                                .color(theme::c().text),
                        );
                        ui.add_space(7.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            for preset in ThemePreset::VISIBLE {
                                let (rect, response) = ui
                                    .allocate_exact_size(Vec2::new(220.0, 64.0), Sense::click());
                                let (canvas, ink, accent) = if preset.is_dark() {
                                    (
                                        egui::Color32::from_rgb(0x0F, 0x12, 0x0F),
                                        egui::Color32::from_rgb(0xF0, 0xE9, 0xDA),
                                        egui::Color32::from_rgb(0x8F, 0xB7, 0x9A),
                                    )
                                } else {
                                    (
                                        egui::Color32::from_rgb(0xF4, 0xEF, 0xE5),
                                        egui::Color32::from_rgb(0x1D, 0x1B, 0x16),
                                        egui::Color32::from_rgb(0x2E, 0x5B, 0x44),
                                    )
                                };
                                let painter = ui.painter();
                                painter.rect_filled(rect, CornerRadius::same(8), canvas);
                                painter.rect_stroke(
                                    rect,
                                    CornerRadius::same(8),
                                    Stroke::new(
                                        if self.theme_preset == preset { 2.0 } else { 1.0 },
                                        if self.theme_preset == preset {
                                            theme::c().accent
                                        } else {
                                            theme::c().border_soft
                                        },
                                    ),
                                    egui::StrokeKind::Inside,
                                );
                                painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        Pos2::new(rect.left() + 14.0, rect.top() + 14.0),
                                        Vec2::new(34.0, 36.0),
                                    ),
                                    CornerRadius::same(2),
                                    accent,
                                );
                                painter.text(
                                    Pos2::new(rect.left() + 58.0, rect.top() + 24.0),
                                    Align2::LEFT_CENTER,
                                    preset.label(),
                                    FontId::proportional(13.0),
                                    ink,
                                );
                                painter.text(
                                    Pos2::new(rect.left() + 58.0, rect.top() + 42.0),
                                    Align2::LEFT_CENTER,
                                    if preset.is_dark() { "Nocturne" } else { "Paper" },
                                    crate::ui::tokens::mono(8.5),
                                    if preset.is_dark() {
                                        egui::Color32::from_rgb(0xB0, 0xA5, 0x94)
                                    } else {
                                        egui::Color32::from_rgb(0x6B, 0x64, 0x57)
                                    },
                                );
                                crate::ui::focus::focus_on_click(&response);
                                crate::ui::focus::ring(ui, &response);
                                response.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::Button,
                                        true,
                                        self.theme_preset == preset,
                                        format!("主题：{}", preset.label()),
                                    )
                                });
                                if response.clicked() {
                                    requested_theme = Some(preset);
                                }
                            }
                        });
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new(format!(
                                "当前：{} · {}",
                                self.theme_preset.label(),
                                if self.theme_preset.is_dark() {
                                    "深色"
                                } else {
                                    "浅色"
                                }
                            ))
                            .size(10.0)
                            .color(theme::c().faint),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("减少动态效果")
                                    .size(11.0)
                                    .color(theme::c().muted),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                widgets::toggle_switch(ui, &mut reduced_motion);
                            });
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("ReplayGain")
                                    .size(11.0)
                                    .color(theme::c().muted),
                            );
                            for (id, label) in
                                [("off", "关闭"), ("track", "单曲"), ("album", "专辑")]
                            {
                                if widgets::chip(ui, label, self.library.replaygain == id)
                                    .clicked()
                                {
                                    requested_replaygain = Some(id.to_string());
                                }
                            }
                        });
                        ui.add_space(10.0);
                        let (preview, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 104.0),
                            Sense::hover(),
                        );
                        let painter = ui.painter();
                        painter.rect_filled(preview, CornerRadius::same(8), theme::c().bg);
                        painter.rect_stroke(
                            preview,
                            CornerRadius::same(8),
                            Stroke::new(1.0_f32, theme::c().border_soft),
                            egui::StrokeKind::Inside,
                        );
                        let side = egui::Rect::from_min_size(
                            preview.min,
                            Vec2::new(86.0, preview.height()),
                        );
                        painter.rect_filled(
                            side,
                            CornerRadius { nw: 8, sw: 8, ne: 0, se: 0 },
                            theme::sidebar_deep(),
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                Pos2::new(side.left() + 12.0, side.top() + 18.0),
                                Vec2::new(52.0, 7.0),
                            ),
                            CornerRadius::same(2),
                            theme::c().accent,
                        );
                        for index in 0..3 {
                            painter.rect_filled(
                                egui::Rect::from_min_size(
                                    Pos2::new(side.left() + 12.0, side.top() + 40.0 + index as f32 * 16.0),
                                    Vec2::new(56.0, 5.0),
                                ),
                                CornerRadius::same(2),
                                theme::with_alpha(theme::sidebar_text(), 0.42),
                            );
                        }
                        painter.text(
                            Pos2::new(preview.left() + 108.0, preview.top() + 26.0),
                            Align2::LEFT_CENTER,
                            "本地曲库",
                            crate::ui::tokens::serif(20.0),
                            theme::c().text,
                        );
                        for index in 0..2 {
                            painter.rect_filled(
                                egui::Rect::from_min_size(
                                    Pos2::new(preview.left() + 108.0, preview.top() + 52.0 + index as f32 * 20.0),
                                    Vec2::new(preview.width() - 132.0, 7.0),
                                ),
                                CornerRadius::same(2),
                                if index == 0 {
                                    theme::accent_soft()
                                } else {
                                    theme::c().card
                                },
                            );
                        }
                    });
                }

                if self.settings_tab == 1 {
                section(ui, "桌面歌词");
                Frame::NONE
                    .fill(theme::c().card)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("显示桌面歌词")
                                    .size(12.0)
                                    .color(theme::c().text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                widgets::toggle_switch(ui, &mut lyrics_open);
                            });
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("面板不透明度")
                                    .size(12.0)
                                    .color(theme::c().text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{:.0}%", opacity * 100.0))
                                        .font(egui::FontId::monospace(10.0))
                                        .color(theme::c().faint),
                                );
                                ui.add(
                                    egui::Slider::new(&mut opacity, 0.0..=1.0)
                                        .show_value(false)
                                        .trailing_fill(true),
                                );
                            });
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("歌词文字不透明度")
                                    .size(12.0)
                                    .color(theme::c().text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{:.0}%", text_opacity * 100.0))
                                        .font(egui::FontId::monospace(10.0))
                                        .color(theme::c().faint),
                                );
                                ui.add(
                                    egui::Slider::new(&mut text_opacity, 0.0..=1.0)
                                        .show_value(false)
                                        .trailing_fill(true),
                                );
                            });
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("字号")
                                    .size(12.0)
                                    .color(theme::c().text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{lyrics_font:.0}px"))
                                        .font(egui::FontId::monospace(10.0))
                                        .color(theme::c().faint),
                                );
                                ui.add(
                                    egui::Slider::new(&mut lyrics_font, 18.0..=40.0)
                                        .show_value(false)
                                        .trailing_fill(true),
                                );
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("锁定桌面歌词位置")
                                    .size(12.0)
                                    .color(theme::c().text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                widgets::toggle_switch(ui, &mut lyrics_locked);
                            });
                        });
                    });

                }

                if self.settings_tab == 2 {
                section(ui, "迷你播放器");
                Frame::NONE
                    .fill(theme::c().card)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("样式")
                                .size(12.0)
                                .color(theme::c().text),
                        );
                        ui.add_space(7.0);
                        ui.horizontal(|ui| {
                            for (id, label) in [("classic", "经典"), ("compact", "紧凑")] {
                                if widgets::chip(ui, label, mini_style == id).clicked() {
                                    mini_style = id.to_string();
                                }
                            }
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("始终显示在最上层")
                                    .size(12.0)
                                    .color(theme::c().text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                widgets::toggle_switch(ui, &mut mini_on_top);
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("拖到屏幕边缘时自动吸附")
                                    .size(12.0)
                                    .color(theme::c().text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                widgets::toggle_switch(ui, &mut mini_snap);
                            });
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("底部播放栏的迷你播放器按钮可随时切换主/迷你模式。")
                                .size(10.0)
                                .color(theme::c().faint),
                        );
                    });

                }

                if self.settings_tab == 3 {
                section(ui, "离线模式");
                Frame::NONE
                    .fill(theme::c().card)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("默认离线")
                                    .size(12.0)
                                    .color(theme::c().text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                widgets::toggle_switch(ui, &mut offline_mode);
                            });
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "开启后不会自动联网；手动点击封面/歌词仍可在需要时联网补全。",
                            )
                            .size(10.5)
                            .color(theme::c().faint),
                        );
                    });

                ui.add_space(10.0);
                Frame::NONE
                    .fill(theme::c().card)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("本地可用")
                                .size(10.0)
                                .color(theme::c().muted),
                        );
                        ui.add_space(4.0);
                        for line in [
                            "本地曲库与播放队列可用",
                            "已缓存的封面与歌词可读",
                            "播放与进度不受网络影响",
                        ] {
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::splat(12.0),
                                    Sense::hover(),
                                );
                                icons::paint(
                                    ui.painter(),
                                    rect,
                                    Icon::Check,
                                    theme::c().accent,
                                );
                                ui.label(
                                    RichText::new(line)
                                        .size(11.0)
                                        .color(theme::c().text),
                                );
                            });
                        }
                    });

                if offline_mode {
                    ui.add_space(10.0);
                    Frame::NONE
                        .fill(theme::signal_soft())
                        .stroke(Stroke::new(1.0_f32, theme::with_alpha(theme::signal(), 0.30)))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("本次补全没有完成")
                                        .size(12.0)
                                        .color(theme::signal()),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new("OFFLINE")
                                            .font(crate::ui::tokens::mono(8.5))
                                            .color(theme::signal()),
                                    );
                                });
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(
                                    "当前是离线模式。这首歌曲没有本地封面，但播放不受影响。",
                                )
                                .size(10.5)
                                .color(theme::c().muted),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("本次联网")
                                                .size(11.0)
                                                .color(theme::c().on_accent),
                                        )
                                        .fill(theme::c().accent)
                                        .corner_radius(8)
                                        .min_size(Vec2::new(0.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    offline_mode = false;
                                    if self.current.is_some() {
                                        download = true;
                                    }
                                }
                                let _ = ui.add(
                                    egui::Button::new(
                                        RichText::new("保持离线")
                                            .size(11.0)
                                            .color(theme::c().muted),
                                    )
                                    .fill(theme::c().card)
                                    .stroke(Stroke::new(1.0_f32, theme::c().border))
                                    .corner_radius(8)
                                    .min_size(Vec2::new(0.0, 28.0)),
                                );
                            });
                        });
                }

                }

                if self.settings_tab == 4 {
                section(ui, "在线内容");
                Frame::NONE
                    .fill(theme::c().card)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("自动匹配缺失封面")
                                    .size(12.0)
                                    .color(theme::c().text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                widgets::toggle_switch(ui, &mut auto_cover);
                            });
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("MusicBrainz + Cover Art Archive")
                                    .size(10.0)
                                    .color(theme::c().faint),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Label::new(
                                            RichText::new(format!(
                                                "清除缓存 {:.1} MB",
                                                cover_cache as f64 / 1_048_576.0
                                            ))
                                            .size(10.0)
                                            .color(theme::c().muted),
                                        )
                                        .sense(Sense::click()),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    clear_covers = true;
                                }
                            });
                        });
                        ui.add_space(9.0);
                        widgets::divider(ui);
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("自动下载缺失歌词")
                                    .size(12.0)
                                    .color(theme::c().text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                widgets::toggle_switch(ui, &mut auto_lyrics);
                            });
                        });
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new(
                                "没有本地 .lrc 时，从 LRCLIB 免费歌词库自动搜索并缓存。",
                            )
                            .size(10.5)
                            .color(theme::c().faint),
                        );
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.hyperlink_to(
                                RichText::new("LRCLIB · 免费开放歌词库")
                                    .size(10.5)
                                    .color(theme::c().accent),
                                "https://lrclib.net",
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let fetching = self.lyrics_fetching.is_some();
                                let enabled = self.current.is_some() && !fetching;
                                if ui
                                    .add_enabled(
                                        enabled,
                                        egui::Button::new(
                                            RichText::new(if fetching {
                                                "正在下载…"
                                            } else {
                                                "下载当前歌词"
                                            })
                                            .size(11.0)
                                            .color(if enabled {
                                                theme::c().accent
                                            } else {
                                                theme::c().faint
                                            }),
                                        )
                                        .fill(theme::accent_alpha(0.10))
                                        .stroke(Stroke::new(
                                            1.0_f32,
                                            theme::accent_alpha(0.28),
                                        ))
                                        .corner_radius(9)
                                        .min_size(Vec2::new(0.0, 30.0)),
                                    )
                                    .clicked()
                                {
                                    download = true;
                                }
                            });
                        });
                    });

                ui.add_space(12.0);
                section(ui, "最近一次联网");
                let failure = self.last_net_failure;
                Frame::NONE
                    .fill(if failure.is_some() {
                        theme::signal_soft()
                    } else {
                        theme::c().card
                    })
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        let (title, detail) = match failure {
                            None => (
                                "没有待处理的失败",
                                "自动匹配封面或歌词失败时，这里会给出一个可以继续的动作。",
                            ),
                            Some(NetFailure::NotFound) => (
                                "没有找到匹配的内容",
                                "本地曲库、播放和已缓存内容都没有变化。可以换用文件名或手动选择。",
                            ),
                            Some(NetFailure::Timeout) => (
                                "请求超时",
                                "网络在超时时间内没有响应；本地内容和缓存仍然可用。",
                            ),
                            Some(NetFailure::RateLimited) => (
                                "请求过于频繁",
                                "服务暂时限制了请求；稍后再试，当前播放不受影响。",
                            ),
                            Some(NetFailure::Offline) => (
                                "当前离线",
                                "这是手动发起的联网请求；可以保持离线，或仅本次联网。",
                            ),
                        };
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(title)
                                    .size(12.0)
                                    .color(if failure.is_some() {
                                        theme::signal()
                                    } else {
                                        theme::c().text
                                    }),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if failure.is_some() {
                                    ui.label(
                                        RichText::new("LAST REQUEST")
                                            .font(crate::ui::tokens::mono(8.5))
                                            .color(theme::c().faint),
                                    );
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(detail)
                                .size(10.5)
                                .color(theme::c().muted),
                        );
                        if failure.is_some() {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if failure == Some(NetFailure::RateLimited) {
                                    let _ = ui.add_enabled(
                                        false,
                                        egui::Button::new(
                                            RichText::new("稍后重试")
                                                .size(11.0)
                                                .color(theme::c().faint),
                                        )
                                        .fill(theme::c().card)
                                        .stroke(Stroke::new(1.0_f32, theme::c().border))
                                        .corner_radius(8)
                                        .min_size(Vec2::new(0.0, 28.0)),
                                    );
                                } else if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("重试当前歌词")
                                                .size(11.0)
                                                .color(theme::c().on_accent),
                                        )
                                        .fill(theme::c().accent)
                                        .corner_radius(8)
                                        .min_size(Vec2::new(0.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    offline_mode = false;
                                    if self.current.is_some() {
                                        download = true;
                                    }
                                }
                            });
                        }
                    });

                }

                if self.settings_tab == 5 {
                section(ui, "数据");
                Frame::NONE
                    .fill(theme::c().card)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("备份与恢复")
                                .size(12.0)
                                .color(theme::c().text),
                        );
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(
                                "备份设置、曲库索引、播放统计和会话；封面歌词缓存可选。恢复前会自动再备份一份。",
                            )
                            .size(10.5)
                            .color(theme::c().faint),
                        );
                        ui.add_space(9.0);
                        let narrow = ui.available_width() < 360.0;
                        if narrow {
                            let width = ui.available_width();
                            if widgets::secondary_button_sized(ui, "备份数据…", width).clicked() {
                                backup_requested = true;
                            }
                            ui.add_space(4.0);
                            if widgets::secondary_button_sized(ui, "恢复数据…", width).clicked() {
                                restore_requested = true;
                            }
                        } else {
                            ui.horizontal(|ui| {
                                if widgets::secondary_button(ui, "备份数据…").clicked() {
                                    backup_requested = true;
                                }
                                ui.add_space(4.0);
                                if widgets::secondary_button(ui, "恢复数据…").clicked() {
                                    restore_requested = true;
                                }
                            });
                        }
                        ui.add_space(5.0);
                        ui.checkbox(
                            &mut backup_media,
                            RichText::new("备份时包含封面和歌词缓存")
                                .size(10.5)
                                .color(theme::c().muted),
                        );
                        ui.checkbox(
                            &mut settings_only,
                            RichText::new("恢复时只恢复设置与播放列表")
                                .size(10.5)
                                .color(theme::c().muted),
                        );
                    });
                }

                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("RPlayer v{}", env!("CARGO_PKG_VERSION")))
                            .size(10.0)
                            .color(theme::c().faint),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("本地优先 · 不上传音乐")
                                .size(10.0)
                                .color(theme::c().faint),
                        );
                    });
                });
                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::secondary_button(ui, "完成").clicked() {
                        close = true;
                    }
                });
                    });
                    });
                });
            });

        if let Some(preset) = requested_theme {
            self.set_theme_preset(ctx, preset);
        }
        if self.library.reduced_motion != reduced_motion {
            self.library.reduced_motion = reduced_motion;
            crate::ui::motion::set_reduced(reduced_motion);
            self.persist();
        }
        if let Some(mode) = requested_replaygain {
            if self.library.replaygain != mode {
                self.library.replaygain = mode.clone();
                if let Some(audio) = &mut self.audio {
                    audio.set_replaygain_mode(&mode);
                }
                self.persist();
                self.toast(
                    format!(
                        "ReplayGain：{}",
                        match mode.as_str() {
                            "track" => "单曲",
                            "album" => "专辑",
                            _ => "关闭",
                        }
                    ),
                    super::ToastKind::Info,
                );
            }
        }
        if (opacity - self.library.desktop_lyrics_opacity).abs() > 0.001 {
            self.library.desktop_lyrics_opacity = opacity.clamp(0.0, 1.0);
            self.prefs_dirty = true;
        }
        if (text_opacity - self.library.desktop_lyrics_text_opacity).abs() > 0.001 {
            self.library.desktop_lyrics_text_opacity = text_opacity.clamp(0.0, 1.0);
            self.prefs_dirty = true;
        }
        if (lyrics_font - self.library.desktop_lyrics_font).abs() > 0.01 {
            self.library.desktop_lyrics_font = lyrics_font.clamp(18.0, 40.0);
            self.prefs_dirty = true;
        }
        if lyrics_locked != self.library.desktop_lyrics_locked {
            self.library.desktop_lyrics_locked = lyrics_locked;
            self.prefs_dirty = true;
        }
        if mini_style != self.library.mini_style {
            self.library.mini_style = mini_style;
            self.prefs_dirty = true;
        }
        if mini_on_top != self.library.mini_on_top {
            self.library.mini_on_top = mini_on_top;
            self.prefs_dirty = true;
            if self.mini_open {
                ctx.send_viewport_cmd_to(
                    crate::overlay::mini_viewport(),
                    egui::ViewportCommand::WindowLevel(if mini_on_top {
                        egui::WindowLevel::AlwaysOnTop
                    } else {
                        egui::WindowLevel::Normal
                    }),
                );
            }
        }
        if mini_snap != self.library.mini_snap {
            self.library.mini_snap = mini_snap;
            self.prefs_dirty = true;
        }
        if offline_mode != self.library.offline_mode {
            self.library.offline_mode = offline_mode;
            self.persist();
        }
        if auto_lyrics != self.library.auto_lyrics {
            self.library.auto_lyrics = auto_lyrics;
            self.persist();
        }
        if auto_cover != self.library.auto_cover {
            self.library.auto_cover = auto_cover;
            self.persist();
            if auto_cover && self.cover.is_none() {
                if let Some(id) = self.current {
                    self.request_cover(ctx, id, false);
                }
            }
        }
        if clear_covers {
            let _ = crate::cover::clear_cache();
            self.cover_attempted.clear();
            self.cover_for = None;
            self.cover = None;
            self.cover_image = None;
            self.cover_source = None;
            self.cover_tint = None;
            if let Some(id) = self.current {
                self.load_cover(ctx, id);
            }
            self.toast("封面缓存已清空", super::ToastKind::Info);
        }
        if lyrics_open != self.desktop_lyrics_open {
            self.toggle_desktop_lyrics(ctx);
        }
        if download {
            self.download_current_lyrics(ctx);
        }
        if backup_requested {
            self.backup_data(backup_media);
        }
        if restore_requested {
            self.restore_data(ctx, settings_only);
        }
        if close {
            self.settings_open = false;
        }
    }

    fn backup_data(&mut self, include_media: bool) {
        let Some(parent) = rfd::FileDialog::new()
            .set_title("选择备份保存位置")
            .pick_folder()
        else {
            return;
        };
        match crate::backup::create(&crate::storage::data_dir(), &parent, include_media) {
            Ok(path) => self.toast(
                format!(
                    "备份完成：{}",
                    path.file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string())
                ),
                super::ToastKind::Success,
            ),
            Err(error) => self.toast(
                format!("备份失败：{error:#}"),
                super::ToastKind::Error,
            ),
        }
    }

    fn restore_data(&mut self, ctx: &egui::Context, settings_only: bool) {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("选择 RPlayer 备份文件夹")
            .pick_folder()
        else {
            return;
        };
        let info = match crate::backup::inspect(&folder) {
            Ok(info) => info,
            Err(error) => {
                self.toast(
                    format!("无法恢复：{error:#}"),
                    super::ToastKind::Error,
                );
                return;
            }
        };
        let description = if settings_only {
            "将用备份中的设置、播放列表和收藏覆盖当前设置，曲库与播放统计保持不变。\n\n恢复前会自动备份当前数据。".to_string()
        } else {
            format!(
                "将恢复 {} 首曲目、播放统计和设置，当前数据会被覆盖。\n\n恢复前会自动备份当前数据。",
                info.tracks
            )
        };
        let confirmed = rfd::MessageDialog::new()
            .set_title("恢复数据")
            .set_description(description)
            .set_buttons(rfd::MessageButtons::YesNo)
            .set_level(rfd::MessageLevel::Warning)
            .show()
            == rfd::MessageDialogResult::Yes;
        if !confirmed {
            return;
        }
        match crate::backup::restore(&crate::storage::data_dir(), &folder, settings_only) {
            Ok(report) => {
                self.reload_after_restore(ctx, settings_only);
                self.toast(
                    if report.settings_only {
                        "设置与播放列表已恢复".to_string()
                    } else {
                        format!("已恢复 {} 个数据文件", report.files)
                    },
                    super::ToastKind::Success,
                );
            }
            Err(error) => self.toast(
                format!("恢复失败：{error:#}"),
                super::ToastKind::Error,
            ),
        }
    }

    fn reload_after_restore(&mut self, ctx: &egui::Context, settings_only: bool) {
        if !settings_only {
            if let Some(audio) = &mut self.audio {
                audio.stop();
            }
            self.current = None;
            self.queue.clear();
            self.queue_index = 0;
            self.selected = None;
            self.checked.clear();
            self.lyrics = None;
            self.lyrics_for = None;
            self.lyrics_editor = None;
            self.cover = None;
            self.cover_image = None;
            self.cover_for = None;
            self.cover_source = None;
            self.cover_tint = None;
            self.play_credited = false;
        }
        self.library = crate::storage::load_library();
        self.volume = self.library.volume.clamp(0.0, 1.0);
        if let Some(audio) = &mut self.audio {
            audio.set_volume(self.volume);
            audio.set_replaygain_mode(&self.library.replaygain);
        }
        self.eq.update(self.library.eq.clone());
        self.stats = crate::stats::PlaybackStats::load();
        self.stats_dirty = false;
        crate::ui::motion::set_reduced(self.library.reduced_motion);
        if let Some(preset) = ThemePreset::from_id(&self.library.theme_preset) {
            self.set_theme_preset(ctx, preset);
        }
        self.prefs_dirty = false;
    }
}

fn settings_nav_item(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let response = ui.add_sized(
        [132.0, 34.0],
        egui::Button::new(
            RichText::new(label)
                .size(12.0)
                .color(if selected {
                    theme::c().accent
                } else {
                    theme::c().muted
                }),
        )
        .fill(if selected {
            theme::accent_soft()
        } else {
            egui::Color32::TRANSPARENT
        })
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(8)),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            true,
            selected,
            format!("设置页：{label}"),
        )
    });
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
}

fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(10.5)
            .strong()
            .color(theme::c().muted),
    );
    ui.add_space(6.0);
}
