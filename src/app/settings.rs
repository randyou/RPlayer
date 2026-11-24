//! Appearance and lyric preferences.

use eframe::egui;
use egui::{
    Align, Align2, CornerRadius, Frame, Layout, RichText, Sense, Stroke, Vec2,
};

use super::RPlayerApp;
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
            (available.x - 24.0).clamp(320.0, 470.0),
            (available.y - 48.0).clamp(240.0, 640.0),
        );
        let content_height = (settings_size.y - 118.0).max(180.0);

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
                            RichText::new("外观、桌面歌词与在线内容")
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

                ui.add_space(12.0);
                egui::ScrollArea::vertical()
                    .max_height(content_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
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
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            for preset in ThemePreset::VISIBLE {
                                if widgets::chip(
                                    ui,
                                    preset.label(),
                                    self.theme_preset == preset,
                                )
                                .clicked()
                                {
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
                    });

                ui.add_space(16.0);
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

                ui.add_space(16.0);
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

                ui.add_space(16.0);
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

                ui.add_space(16.0);
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

                ui.add_space(16.0);
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
                        ui.horizontal(|ui| {
                            if widgets::secondary_button(ui, "备份数据…").clicked() {
                                backup_requested = true;
                            }
                            ui.add_space(4.0);
                            if widgets::secondary_button(ui, "恢复数据…").clicked() {
                                restore_requested = true;
                            }
                        });
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

fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(10.5)
            .strong()
            .color(theme::c().muted),
    );
    ui.add_space(6.0);
}
