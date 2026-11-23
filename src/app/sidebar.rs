//! Left navigation rail: brand, library views, playlists and scan status.

use eframe::egui;
use egui::{Align, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};

use super::{Dialog, RPlayerApp, View};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

impl RPlayerApp {
    pub(super) fn sidebar_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("rplayer-sidebar")
            .exact_width(252.0)
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(theme::SIDEBAR)
                    .inner_margin(Margin::symmetric(14, 16)),
            )
            .show(ctx, |ui| self.sidebar_contents(ui));
    }

    fn sidebar_contents(&mut self, ui: &mut Ui) {
        // Brand
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
            icons::logo(ui.painter(), rect);
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(
                    RichText::new("RPlayer")
                        .size(17.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.label(
                    RichText::new("本地音乐播放器")
                        .size(10.5)
                        .color(theme::FAINT),
                );
            });
        });

        ui.add_space(16.0);
        if widgets::primary_button_full(ui, "添加音乐文件夹").clicked() {
            self.open_folder_picker();
        }
        ui.add_space(14.0);

        // Primary views
        if widgets::nav_item(
            ui,
            Icon::Disc,
            "本地曲库",
            Some(self.library.tracks.len()),
            self.view == View::Library,
        )
        .clicked()
        {
            self.switch_view(View::Library);
        }

        if widgets::nav_item(
            ui,
            Icon::Heart,
            "我的收藏",
            Some(self.library.favorites.len()),
            self.view == View::Favorites,
        )
        .clicked()
        {
            self.switch_view(View::Favorites);
        }

        if widgets::nav_item(
            ui,
            Icon::Clock,
            "最近播放",
            Some(self.library.recent.len()),
            self.view == View::Recent,
        )
        .clicked()
        {
            self.switch_view(View::Recent);
        }

        if widgets::nav_item(
            ui,
            Icon::Folder,
            "音乐目录",
            Some(self.library.folders.len()),
            self.view == View::Folders,
        )
        .clicked()
        {
            self.switch_view(View::Folders);
        }

        if widgets::nav_item(
            ui,
            Icon::Queue,
            "播放队列",
            Some(self.queue.len()),
            self.queue_open,
        )
        .clicked()
        {
            self.queue_open = !self.queue_open;
        }

        // Playlists
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("播放列表")
                    .size(11.0)
                    .strong()
                    .color(theme::FAINT),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, Icon::Plus, 22.0, "新建播放列表").clicked() {
                    self.dialog = Dialog::NewPlaylist {
                        name: String::new(),
                    };
                    self.focus_pending = true;
                }
            });
        });

        let playlists: Vec<(u64, String)> = self
            .library
            .playlists
            .iter()
            .map(|playlist| (playlist.id, playlist.name.clone()))
            .collect();

        let scroll_height = (ui.available_height() - 142.0).max(70.0);
        ScrollArea::vertical()
            .id_salt("rplayer-playlists")
            .max_height(scroll_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if playlists.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("还没有播放列表")
                            .size(11.5)
                            .color(theme::FAINT),
                    );
                    ui.label(
                        RichText::new("点击 + 创建第一个")
                            .size(10.5)
                            .color(theme::FAINT.gamma_multiply(0.75)),
                    );
                }
                for (id, name) in &playlists {
                    let selected = self.view == View::Playlist(*id);
                    let response = widgets::playlist_item(ui, name, selected);
                    if response.clicked() {
                        self.switch_view(View::Playlist(*id));
                    }
                    response.context_menu(|ui| {
                        if ui.button("播放").clicked() {
                            if let Some(playlist) = self.library.playlist(*id) {
                                let queue: Vec<u64> = playlist
                                    .track_ids
                                    .iter()
                                    .filter(|track_id| self.library.track(**track_id).is_some())
                                    .copied()
                                    .collect();
                                if !queue.is_empty() {
                                    self.play_queue(ui.ctx(), queue, 0);
                                }
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("重命名").clicked() {
                            self.dialog = Dialog::RenamePlaylist {
                                id: *id,
                                name: name.clone(),
                            };
                            self.focus_pending = true;
                            ui.close_menu();
                        }
                        if ui.button("删除播放列表…").clicked() {
                            self.dialog = Dialog::ConfirmDeletePlaylist { id: *id };
                            ui.close_menu();
                        }
                    });
                }
            });

        // Push the footer to the bottom of the rail.
        ui.add_space((ui.available_height() - 118.0).max(4.0));
        self.sidebar_footer(ui);
    }

    fn sidebar_footer(&mut self, ui: &mut Ui) {
        Frame::NONE
            .fill(theme::CARD)
            .stroke(Stroke::new(1.0_f32, theme::BORDER_SOFT))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(Margin::same(11))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if self.scanning {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("正在扫描音乐")
                                .size(11.5)
                                .strong()
                                .color(theme::ACCENT),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{} / {}", self.scan_done, self.scan_total))
                                    .size(10.0)
                                    .color(theme::FAINT),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    let fraction = if self.scan_total > 0 {
                        self.scan_done as f32 / self.scan_total as f32
                    } else {
                        0.0
                    };
                    scan_progress(ui, fraction);
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("正在读取标签与封面…")
                            .size(10.0)
                            .color(theme::FAINT),
                    );
                } else {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                        icons::paint(ui.painter(), rect, Icon::Folder, theme::MUTED);
                        ui.label(
                            RichText::new(format!("{} 个音乐文件夹", self.library.folders.len()))
                                .size(11.5)
                                .color(theme::TEXT),
                        );
                    });
                    ui.label(
                        RichText::new(format!(
                            "{} · {}",
                            self.library.tracks.len(),
                            crate::model::format_duration(self.library.total_duration())
                        ))
                        .size(10.0)
                        .color(theme::FAINT),
                    );
                    ui.add_space(4.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 26.0],
                            egui::Button::new(
                                RichText::new("管理文件夹").size(11.0).color(theme::TEXT),
                            )
                            .fill(theme::HOVER)
                            .stroke(Stroke::new(1.0_f32, theme::BORDER))
                            .corner_radius(7),
                        )
                        .clicked()
                    {
                        self.switch_view(View::Folders);
                    }
                }
            });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("v0.2.0").size(10.0).color(theme::FAINT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let about = ui
                    .add(
                        egui::Label::new(
                            RichText::new("关于 RPlayer").size(10.0).color(theme::MUTED),
                        )
                        .sense(Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if about.clicked() {
                    self.dialog = Dialog::About;
                }
            });
        });
    }
}

fn scan_progress(ui: &mut Ui, fraction: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 4.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(2), theme::BORDER);
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        let filled =
            egui::Rect::from_min_size(rect.min, Vec2::new(rect.width() * fraction, rect.height()));
        ui.painter()
            .rect_filled(filled, egui::CornerRadius::same(2), theme::ACCENT);
    }
}
