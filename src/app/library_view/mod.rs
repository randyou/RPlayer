//! Central content area: header, actions, sortable track table, folders page
//! and the various empty states.

mod batch;
mod error_card;
mod table;

use table::progress_bar;
#[cfg(test)]
mod tests;


use eframe::egui;
use egui::{
    Align, Align2, CornerRadius, FontId, Frame, Label, Layout, Margin, Pos2, Rect, RichText,
    ScrollArea, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType,
};

use super::{Dialog, RPlayerApp, View};
use crate::model::{self, SortField, SortState};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::menu;
use crate::ui::widgets;

const ROW_HEIGHT: f32 = 56.0;
const HEADER_HEIGHT: f32 = 38.0;
const SIDE_PAD: f32 = 18.0;
/// GAP 09 fixes the multi-select bar at 38 px so the table does not jump when
/// it appears.
#[derive(Clone, Copy)]
pub(crate) struct Columns {
    index: f32,
    title: f32,
    album: f32,
    duration: f32,
    actions: f32,
}

impl Columns {
    /// `show_album = false` collapses the album column (D13 compact tier).
    fn for_width_opt(width: f32, show_album: bool) -> Self {
        let index = 40.0;
        let album = if show_album {
            (width * 0.27).clamp(180.0, 360.0)
        } else {
            0.0
        };
        let duration = 64.0;
        let actions = 64.0;
        let title = (width - SIDE_PAD * 2.0 - index - album - duration - actions).max(180.0);
        Self {
            index,
            title,
            album,
            duration,
            actions,
        }
    }

    fn shows_album(&self) -> bool {
        self.album > 0.0
    }

    fn index_rect(&self, row: Rect) -> Rect {
        Rect::from_min_size(
            Pos2::new(row.left() + SIDE_PAD, row.top()),
            Vec2::new(self.index, row.height()),
        )
    }

    fn title_rect(&self, row: Rect) -> Rect {
        let left = row.left() + SIDE_PAD + self.index;
        Rect::from_min_size(
            Pos2::new(left, row.top()),
            Vec2::new(self.title, row.height()),
        )
    }

    fn album_rect(&self, row: Rect) -> Rect {
        let left = self.title_rect(row).right();
        Rect::from_min_size(
            Pos2::new(left, row.top()),
            Vec2::new(self.album, row.height()),
        )
    }

    fn duration_rect(&self, row: Rect) -> Rect {
        let left = self.album_rect(row).right();
        Rect::from_min_size(
            Pos2::new(left, row.top()),
            Vec2::new(self.duration, row.height()),
        )
    }

    fn actions_rect(&self, row: Rect) -> Rect {
        Rect::from_min_size(
            Pos2::new(row.right() - SIDE_PAD - self.actions, row.top()),
            Vec2::new(self.actions, row.height()),
        )
    }
}

impl RPlayerApp {
    /// D12: one inline, expandable error card above the view content.
    fn central_header(&mut self, ui: &mut Ui, ids: &[u64], total_seconds: f64) {
        let (title, subtitle) = match self.view {
            View::Albums => (
                "专辑".to_string(),
                format!(
                    "{} 张专辑 · {} 首歌曲",
                    self.album_groups(ids).len(),
                    ids.len()
                ),
            ),
            View::Artists => (
                "艺术家".to_string(),
                format!(
                    "{} 位艺术家 · {} 首歌曲",
                    self.artist_groups(ids).len(),
                    ids.len()
                ),
            ),
            _ => self.view_header(ids.len(), total_seconds),
        };
        ui.horizontal(|ui| {
            if let View::Playlist(pid) = self.view {
                if let Some(playlist) = self.library.playlists.iter().find(|p| p.id == pid) {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(120.0), Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(rect, CornerRadius::same(2), theme::c().card);
                    painter.rect_stroke(
                        rect,
                        CornerRadius::same(2),
                        Stroke::new(1.0_f32, theme::c().border_soft),
                        egui::StrokeKind::Inside,
                    );
                    let mid = rect.center();
                    let quadrant = Vec2::splat(rect.width() * 0.5);
                    painter.rect_filled(
                        Rect::from_min_size(rect.min, quadrant),
                        CornerRadius::ZERO,
                        theme::accent_alpha(0.10),
                    );
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(mid.x, rect.top()),
                            quadrant,
                        ),
                        CornerRadius::ZERO,
                        theme::c().card,
                    );
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(rect.left(), mid.y),
                            quadrant,
                        ),
                        CornerRadius::ZERO,
                        theme::c().card,
                    );
                    painter.rect_filled(
                        Rect::from_min_size(mid, quadrant),
                        CornerRadius::ZERO,
                        theme::accent_alpha(0.06),
                    );
                    painter.text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        playlist
                            .name
                            .chars()
                            .next()
                            .unwrap_or('R')
                            .to_string(),
                        crate::ui::tokens::serif(42.0),
                        theme::c().text,
                    );
                    ui.add_space(10.0);
                }
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let kicker = match self.view {
                    View::Albums => "COLLECTION 02",
                    View::Artists => "ARTISTS 03",
                    View::Favorites => "FAVORITES",
                    View::Recent => "RECENT",
                    View::Playlist(_) => "PLAYLIST",
                    View::Smart(_) => "SMART PLAYLIST",
                    View::Folders => "FOLDERS",
                    View::Library => "LIBRARY 01",
                };
                ui.label(
                    RichText::new(kicker)
                        .font(crate::ui::tokens::mono(9.0))
                        .color(theme::c().faint),
                );
                let title_size = if crate::app::layout::Tier::of_ctx(ui.ctx()).shows_rail() {
                    42.0
                } else {
                    32.0
                };
                ui.label(
                    RichText::new(title)
                        .font(crate::ui::tokens::serif(title_size))
                        .color(theme::c().text),
                );
                ui.label(RichText::new(subtitle).size(11.5).color(theme::c().muted));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let scan = widgets::icon_button(ui, Icon::Repeat, 32.0, "重新扫描音乐库");
                if scan.clicked() && !self.scanning && !self.library.folders.is_empty() {
                    self.start_scan();
                }
                ui.add_space(4.0);
                if widgets::icon_button_tinted(
                    ui,
                    Icon::PanelRight,
                    32.0,
                    theme::c().muted,
                    self.now_rail_open && self.current.is_some(),
                    "显示 / 收起正在播放侧栏",
                )
                .clicked()
                {
                    if self.current.is_some() {
                        self.now_rail_open = !self.now_rail_open;
                        if self.now_rail_open {
                            self.rail_queue = false;
                        }
                    } else {
                        self.toast("先播放一首歌曲", crate::app::ToastKind::Info);
                    }
                }
            });
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            if !ids.is_empty() && widgets::primary_button_icon(ui, crate::ui::icons::Icon::Play, "播放全部").clicked() {
                self.play_queue(ui.ctx(), ids.to_vec(), 0);
            }
            let table_view = !matches!(
                self.view,
                View::Folders | View::Albums | View::Artists
            );
            if table_view && widgets::secondary_button_icon(ui, crate::ui::icons::Icon::Plus, "添加").clicked() {
                self.open_folder_picker();
            }

            if table_view {
                let (label, icon) = if self.batch_mode {
                    ("完成", crate::ui::icons::Icon::Check)
                } else {
                    ("批量操作", crate::ui::icons::Icon::List)
                };
                if widgets::secondary_button_icon(ui, icon, label).clicked() {
                    self.batch_mode = !self.batch_mode;
                    if !self.batch_mode {
                        self.checked.clear();
                    }
                }
                if widgets::secondary_button(ui, "重复歌曲").clicked() {
                    self.open_duplicates();
                }
            }

            if self.batch_mode {
                ui.add_space(4.0);
            }
        });
    }

    /// Multi-select action bar (GAP 09).
    ///
    /// Fixed 38 px single row with three states:
    /// - 0 selected: hint only, every destructive action hidden;
    /// - 1..=19: per-track actions (playlist, queue, tags, remove, clear);
    /// - >=20: collapses to `清空全部` / `退出批量` so a large selection cannot
    ///   accidentally run a per-track action.
    fn folders_page(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            if widgets::primary_button(ui, "添加文件夹").clicked() {
                self.open_folder_picker();
            }
            if widgets::secondary_button(ui, "重新扫描").clicked()
                && !self.library.folders.is_empty()
            {
                self.start_scan();
            }
            if widgets::secondary_button(ui, if self.health_checking { "检查中…" } else { "检查曲库" })
                .clicked()
                && !self.health_checking
            {
                self.start_health_check();
            }
            if self.scanning {
                ui.label(
                    RichText::new(format!("正在扫描 {}/{}", self.scan_done, self.scan_total))
                        .size(11.0)
                        .color(theme::c().accent),
                );
            }
        });
        ui.add_space(14.0);

        if self.library.folders.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(70.0);
                widgets::empty_illustration(ui, 96.0);
                ui.add_space(14.0);
                ui.label(
                    RichText::new("还没有音乐目录")
                        .size(17.0)
                        .strong()
                        .color(theme::c().text),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("添加一个文件夹，RPlayer 会递归扫描其中的音频文件。")
                        .size(12.0)
                        .color(theme::c().muted),
                );
            });
            return;
        }

        let folders = self.library.folders.clone();
        ScrollArea::vertical()
            .id_salt("rplayer-folders")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for folder in folders {
                    let count = self
                        .library
                        .tracks
                        .iter()
                        .filter(|track| crate::app::path_is_under(&folder, &track.path))
                        .count();
                    Frame::NONE
                        .fill(theme::c().card)
                        .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::symmetric(14, 11))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (icon_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                                icons::paint(
                                    ui.painter(),
                                    icon_rect,
                                    Icon::Folder,
                                    theme::c().accent,
                                );
                                ui.add_space(6.0);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    ui.add(
                                        Label::new(
                                            RichText::new(folder.display().to_string())
                                                .size(12.5)
                                                .color(theme::c().text),
                                        )
                                        .truncate(),
                                    );
                                    ui.label(
                                        RichText::new(format!("{count} 首歌曲"))
                                            .size(10.5)
                                            .color(theme::c().faint),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let remove =
                                        widgets::icon_button(ui, Icon::Trash, 28.0, "移除此目录");
                                    remove.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            true,
                                            format!("移除此目录：{}", folder.display()),
                                        )
                                    });
                                    if remove.clicked() {
                                        self.remove_folder(&folder);
                                        self.toast("已移除音乐目录", crate::app::ToastKind::Info);
                                    }
                                });
                            });
                        });
                }
            });
    }

    fn empty_library_state(&mut self, ui: &mut Ui) {
        if self.scanning {
            let fraction = if self.scan_total > 0 {
                self.scan_done as f32 / self.scan_total as f32
            } else {
                0.0
            };
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                widgets::empty_illustration(ui, 96.0);
                ui.add_space(14.0);
                ui.label(
                    RichText::new("正在扫描音乐")
                        .size(17.0)
                        .strong()
                        .color(theme::c().text),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("{} / {}", self.scan_done, self.scan_total))
                        .size(11.5)
                        .color(theme::c().muted),
                );
                ui.add_space(12.0);
                progress_bar(ui, fraction, 260.0);
            });
            return;
        }

        ui.vertical_centered(|ui| {
            let folders_empty = self.library.folders.is_empty();
            ui.add_space(30.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(96.0), Sense::hover());
            icons::logo(ui.painter(), rect);
            ui.add_space(18.0);
            ui.label(
                RichText::new(if folders_empty {
                    "把音乐放回桌面。"
                } else {
                    "这个文件夹里没有音乐。"
                })
                .size(25.0)
                .strong()
                .color(theme::c().text),
            );
            ui.add_space(6.0);
            ui.add(
                Label::new(
                    RichText::new(if folders_empty {
                        "RPlayer 只读取你的文件夹，不上传、不联网也能播放。"
                    } else {
                        "已添加的文件夹里没有发现可播放的音频，检查扩展名，或换一个文件夹。"
                    })
                    .size(12.5)
                    .color(theme::c().muted),
                )
                .wrap(),
            );
            ui.add_space(22.0);

            let total_width = if folders_empty { 190.0 } else { 250.0 };
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.add_space(((ui.available_width() - total_width) / 2.0).max(0.0));
                if folders_empty {
                    if widgets::primary_button_icon(
                        ui,
                        crate::ui::icons::Icon::Folder,
                        "添加音乐文件夹",
                    )
                    .clicked()
                    {
                        self.open_folder_picker();
                    }
                } else {
                    if widgets::primary_button_icon(ui, crate::ui::icons::Icon::Repeat, "重新扫描")
                        .clicked()
                    {
                        self.start_scan();
                    }
                    if widgets::secondary_button(ui, "管理文件夹").clicked() {
                        self.view = View::Folders;
                    }
                }
            });

            ui.add_space(12.0);
            ui.label(
                RichText::new(if folders_empty {
                    "MP3 · FLAC · WAV · OGG · M4A · AAC"
                } else {
                    "已添加的文件夹保留在音乐目录中，不会删除磁盘文件"
                })
                .size(10.0)
                .color(theme::c().faint),
            );
        });
    }

    fn no_results_state(&mut self, ui: &mut Ui) {
        let has_search = !self.search.trim().is_empty();
        let (title, subtitle) = if has_search {
            ("没有找到匹配的歌曲", "试试其他关键词，或清除搜索条件")
        } else {
            match self.view {
                View::Favorites => ("这里还没有收藏的歌曲", "点击歌曲右侧的爱心，把它加入收藏"),
                View::Recent => ("还没有播放记录", "从曲库中播放一首歌，这里会留下记录"),
                View::Playlist(_) => (
                    "这个播放列表还是空的",
                    "在曲库中右键歌曲，把它添加到播放列表",
                ),
                _ => ("没有找到匹配的歌曲", "试试其他关键词，或清除搜索条件"),
            }
        };

        ui.vertical_centered(|ui| {
            ui.add_space(72.0);
            widgets::empty_illustration(ui, 88.0);
            ui.add_space(14.0);
            ui.label(
                RichText::new(title)
                    .size(16.0)
                    .strong()
                    .color(theme::c().text),
            );
            ui.add_space(4.0);
            ui.label(RichText::new(subtitle).size(11.5).color(theme::c().muted));
            ui.add_space(14.0);
            if has_search && widgets::secondary_button(ui, "清除搜索").clicked() {
                self.search.clear();
            }
        });
    }
}

