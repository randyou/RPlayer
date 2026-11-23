//! Central content area: header, search, sortable track table and empty states.

use eframe::egui;
use egui::{
    Align, Align2, CornerRadius, FontId, Frame, Label, Layout, Margin, Pos2, Rect, RichText,
    ScrollArea, Sense, Shape, Stroke, Ui, Vec2,
};

use super::{Dialog, RPlayerApp, View};
use crate::model::{self, SortField, SortState};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

const ROW_HEIGHT: f32 = 44.0;
const HEADER_HEIGHT: f32 = 34.0;
const SIDE_PAD: f32 = 14.0;

#[derive(Clone, Copy)]
struct Columns {
    index: f32,
    title: f32,
    artist: f32,
    album: f32,
    duration: f32,
}

impl Columns {
    fn for_width(width: f32) -> Self {
        let artist = (width * 0.22).max(120.0);
        let album = (width * 0.20).max(100.0);
        let duration = 62.0;
        let index = 38.0;
        let title = (width - SIDE_PAD * 2.0 - index - artist - album - duration - 16.0).max(96.0);
        Self {
            index,
            title,
            artist,
            album,
            duration,
        }
    }

    fn title_rect(&self, row: Rect) -> Rect {
        let left = row.left() + SIDE_PAD + self.index;
        Rect::from_min_size(
            Pos2::new(left, row.top()),
            Vec2::new(self.title, row.height()),
        )
    }

    fn artist_rect(&self, row: Rect) -> Rect {
        let left = self.title_rect(row).right();
        Rect::from_min_size(
            Pos2::new(left, row.top()),
            Vec2::new(self.artist, row.height()),
        )
    }

    fn album_rect(&self, row: Rect) -> Rect {
        let left = self.artist_rect(row).right();
        Rect::from_min_size(
            Pos2::new(left, row.top()),
            Vec2::new(self.album, row.height()),
        )
    }

    fn duration_rect(&self, row: Rect) -> Rect {
        Rect::from_min_size(
            Pos2::new(row.right() - SIDE_PAD - self.duration, row.top()),
            Vec2::new(self.duration, row.height()),
        )
    }

    fn index_rect(&self, row: Rect) -> Rect {
        Rect::from_min_size(
            Pos2::new(row.left() + SIDE_PAD, row.top()),
            Vec2::new(self.index, row.height()),
        )
    }
}

impl RPlayerApp {
    pub(super) fn central_panel(&mut self, ctx: &egui::Context, ids: &[u64], total_seconds: f64) {
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(theme::BG).inner_margin(Margin {
                left: 24,
                right: 20,
                top: 20,
                bottom: 12,
            }))
            .show(ctx, |ui| {
                self.central_header(ui, ids, total_seconds);
                ui.add_space(16.0);

                if self.library.tracks.is_empty() {
                    self.empty_library_state(ui);
                    return;
                }

                let columns = Columns::for_width(ui.available_width());
                self.table_header(ui, columns);

                if ids.is_empty() {
                    self.no_results_state(ui);
                } else {
                    self.track_table(ui, ids, columns);
                }
            });
    }

    fn central_header(&mut self, ui: &mut Ui, ids: &[u64], total_seconds: f64) {
        let (title, subtitle) = self.view_header(ids.len(), total_seconds);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.label(RichText::new(title).size(26.0).strong().color(theme::TEXT));
                ui.horizontal(|ui| {
                    ui.label(RichText::new(subtitle).size(12.0).color(theme::MUTED));
                    if !ids.is_empty() {
                        ui.add_space(6.0);
                        let play_all = ui.add(
                            Label::new(RichText::new("播放全部").size(11.5).color(theme::ACCENT))
                                .sense(Sense::click()),
                        );
                        if play_all
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.play_queue(ui.ctx(), ids.to_vec(), 0);
                        }
                    }
                });
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let scan = widgets::icon_button(ui, Icon::Repeat, 34.0, "重新扫描音乐库");
                if scan.clicked() && !self.scanning && !self.library.folders.is_empty() {
                    self.start_scan();
                }

                ui.add_space(4.0);
                let response = ui
                    .allocate_ui_with_layout(
                        Vec2::new(252.0, 34.0),
                        Layout::left_to_right(Align::Center),
                        |ui| widgets::search_box(ui, &mut self.search, "搜索歌曲、艺术家、专辑"),
                    )
                    .inner;
                if std::mem::take(&mut self.focus_search) {
                    response.request_focus();
                }
            });
        });
    }

    fn table_header(&mut self, ui: &mut Ui, columns: Columns) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, HEADER_HEIGHT), Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(10), theme::CARD);

        let index_rect = columns.index_rect(rect);
        ui.painter().text(
            Pos2::new(index_rect.left() + 5.0, index_rect.center().y),
            Align2::LEFT_CENTER,
            "#",
            FontId::proportional(11.0),
            theme::FAINT,
        );

        let sort = self.sort;
        let mut clicked: Option<SortField> = None;

        if sort_column(
            ui,
            columns.title_rect(rect),
            "标题",
            SortField::Title,
            sort,
            false,
        ) {
            clicked = Some(SortField::Title);
        }
        if sort_column(
            ui,
            columns.artist_rect(rect),
            "艺术家",
            SortField::Artist,
            sort,
            false,
        ) {
            clicked = Some(SortField::Artist);
        }
        if sort_column(
            ui,
            columns.album_rect(rect),
            "专辑",
            SortField::Album,
            sort,
            false,
        ) {
            clicked = Some(SortField::Album);
        }
        if sort_column(
            ui,
            columns.duration_rect(rect),
            "时长",
            SortField::Duration,
            sort,
            true,
        ) {
            clicked = Some(SortField::Duration);
        }

        if let Some(field) = clicked {
            self.sort = SortState {
                field,
                ascending: if self.sort.field == field {
                    !self.sort.ascending
                } else {
                    true
                },
            };
        }
    }

    fn track_table(&mut self, ui: &mut Ui, ids: &[u64], columns: Columns) {
        let width = ui.available_width();
        ScrollArea::vertical()
            .id_salt("rplayer-track-table")
            .auto_shrink([false, false])
            .show_rows(ui, ROW_HEIGHT, ids.len(), |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.set_width(width);
                for index in range {
                    let id = ids[index];
                    let Some(track) = self.library.track(id).cloned() else {
                        continue;
                    };
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::click());
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let current = self.current == Some(id);
                    let selected = self.selected == Some(id);
                    let hovered = response.hovered();

                    let painter = ui.painter();
                    let pill = rect.shrink2(Vec2::new(0.0, 1.0));
                    if current {
                        painter.rect_filled(pill, CornerRadius::same(9), theme::accent_alpha(0.07));
                        painter.rect_filled(
                            Rect::from_min_size(
                                Pos2::new(pill.left() + 1.0, pill.center().y - 9.0),
                                Vec2::new(3.0, 18.0),
                            ),
                            CornerRadius::same(2),
                            theme::ACCENT,
                        );
                    } else if selected {
                        painter.rect_filled(pill, CornerRadius::same(9), theme::HOVER);
                    } else if hovered {
                        painter.rect_filled(pill, CornerRadius::same(9), theme::CARD);
                    }

                    let index_rect = columns.index_rect(rect);
                    if current {
                        let playing = self.is_playing();
                        let time = ui.input(|i| i.time) as f32;
                        painter_eq(painter, index_rect.center(), time, playing);
                    } else if hovered {
                        icons::paint(
                            painter,
                            Rect::from_center_size(index_rect.center(), Vec2::splat(11.0)),
                            Icon::Play,
                            theme::TEXT,
                        );
                    } else {
                        painter.text(
                            index_rect.center(),
                            Align2::CENTER_CENTER,
                            format!("{}", index + 1),
                            FontId::proportional(11.5),
                            theme::FAINT,
                        );
                    }

                    let title_rect = columns.title_rect(rect);
                    let title_clip =
                        painter.with_clip_rect(title_rect.shrink2(Vec2::new(2.0, 0.0)));
                    title_clip.text(
                        Pos2::new(title_rect.left() + 2.0, title_rect.center().y),
                        Align2::LEFT_CENTER,
                        &track.title,
                        FontId::proportional(13.5),
                        if current { theme::ACCENT } else { theme::TEXT },
                    );

                    let artist_rect = columns.artist_rect(rect);
                    let artist_clip = painter.with_clip_rect(artist_rect);
                    artist_clip.text(
                        Pos2::new(artist_rect.left() + 2.0, artist_rect.center().y),
                        Align2::LEFT_CENTER,
                        &track.artist,
                        FontId::proportional(12.5),
                        if current { theme::TEXT } else { theme::MUTED },
                    );

                    let album_rect = columns.album_rect(rect);
                    let album_clip = painter.with_clip_rect(album_rect);
                    album_clip.text(
                        Pos2::new(album_rect.left() + 2.0, album_rect.center().y),
                        Align2::LEFT_CENTER,
                        &track.album,
                        FontId::proportional(12.0),
                        if current { theme::MUTED } else { theme::FAINT },
                    );

                    let duration_rect = columns.duration_rect(rect);
                    painter.text(
                        Pos2::new(duration_rect.right() - 2.0, duration_rect.center().y),
                        Align2::RIGHT_CENTER,
                        model::format_duration(track.duration),
                        FontId::proportional(11.5),
                        if current { theme::MUTED } else { theme::FAINT },
                    );

                    if response.double_clicked() {
                        self.play_track(ui.ctx(), id);
                    } else if response.clicked() {
                        self.selected = Some(id);
                    }

                    response.context_menu(|ui| {
                        if ui.button("播放").clicked() {
                            self.play_track(ui.ctx(), id);
                            ui.close_menu();
                        }

                        ui.menu_button("添加到播放列表", |ui| {
                            if self.library.playlists.is_empty() {
                                ui.label(
                                    RichText::new("还没有播放列表")
                                        .size(11.5)
                                        .color(theme::FAINT),
                                );
                                if ui.button("新建播放列表…").clicked() {
                                    self.dialog = Dialog::NewPlaylist {
                                        name: String::new(),
                                    };
                                    self.focus_pending = true;
                                    ui.close_menu();
                                }
                            } else {
                                let playlists: Vec<(u64, String)> = self
                                    .library
                                    .playlists
                                    .iter()
                                    .map(|p| (p.id, p.name.clone()))
                                    .collect();
                                for (playlist_id, name) in playlists {
                                    if ui.button(name).clicked() {
                                        self.add_track_to_playlist(playlist_id, id);
                                        ui.close_menu();
                                    }
                                }
                            }
                        });

                        if matches!(self.view, View::Playlist(_))
                            && ui.button("从此播放列表移除").clicked()
                        {
                            self.remove_track_from_view_playlist(id);
                            ui.close_menu();
                        }

                        ui.separator();
                        if ui.button("在文件夹中显示").clicked() {
                            self.reveal_in_file_manager(&track.path);
                            ui.close_menu();
                        }
                        if ui.button("从音乐库移除…").clicked() {
                            self.dialog = Dialog::ConfirmRemoveTrack { id };
                            ui.close_menu();
                        }
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
            ui.add_space((ui.available_height() * 0.20).max(30.0));
            ui.vertical_centered(|ui| {
                widgets::empty_illustration(ui, 96.0);
                ui.add_space(12.0);
                ui.label(
                    RichText::new("正在扫描你的音乐库")
                        .size(21.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new("正在读取标签、时长与封面，请稍候…")
                        .size(12.5)
                        .color(theme::MUTED),
                );
                ui.add_space(18.0);
                progress_bar(ui, fraction, 300.0);
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("{} / {}", self.scan_done, self.scan_total))
                        .size(11.0)
                        .color(theme::FAINT),
                );
            });
            return;
        }

        ui.add_space((ui.available_height() * 0.16).max(24.0));
        ui.vertical_centered(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(92.0), Sense::hover());
            icons::logo(ui.painter(), rect);

            ui.add_space(18.0);
            ui.label(
                RichText::new("让本地音乐，回到桌面")
                    .size(25.0)
                    .strong()
                    .color(theme::TEXT),
            );
            ui.add_space(6.0);
            ui.add(
                Label::new(
                    RichText::new(
                        "RPlayer 是一个纯粹、快速的本地播放器。\n添加音乐文件夹，它会把歌曲整理进你的音乐库。",
                    )
                    .size(12.5)
                    .color(theme::MUTED),
                )
                .wrap(),
            );
            ui.add_space(22.0);

            let total_width = 300.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.add_space(((ui.available_width() - total_width) / 2.0).max(0.0));
                if widgets::primary_button(ui, "添加音乐文件夹").clicked() {
                    self.open_folder_picker();
                }
                if !self.library.folders.is_empty()
                    && widgets::secondary_button(ui, "重新扫描").clicked()
                {
                    self.start_scan();
                }
            });

            if !self.library.folders.is_empty() {
                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!(
                        "已添加 {} 个文件夹，没有发现可播放的音频文件",
                        self.library.folders.len()
                    ))
                    .size(11.0)
                    .color(theme::FAINT),
                );
            }
        });
    }

    fn no_results_state(&mut self, ui: &mut Ui) {
        ui.add_space((ui.available_height() * 0.22).max(20.0));
        ui.vertical_centered(|ui| {
            if self.search.trim().is_empty() && matches!(self.view, View::Playlist(_)) {
                widgets::empty_illustration(ui, 86.0);
                ui.add_space(12.0);
                ui.label(
                    RichText::new("这个播放列表还是空的")
                        .size(19.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new("在音乐库中右键歌曲，把它添加到播放列表")
                        .size(12.0)
                        .color(theme::MUTED),
                );
                ui.add_space(16.0);
                if widgets::secondary_button(ui, "去音乐库看看").clicked() {
                    self.view = View::Library;
                }
            } else {
                ui.label(
                    RichText::new("没有找到匹配的歌曲")
                        .size(19.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("试试其他关键词，或清除搜索条件")
                        .size(12.0)
                        .color(theme::MUTED),
                );
                ui.add_space(14.0);
                if widgets::secondary_button(ui, "清除搜索").clicked() {
                    self.search.clear();
                }
            }
        });
    }
}

fn sort_column(
    ui: &mut Ui,
    rect: Rect,
    label: &str,
    field: SortField,
    sort: SortState,
    align_right: bool,
) -> bool {
    let response = ui.interact(rect, ui.id().with(("rplayer-sort", label)), Sense::click());
    let active = sort.field == field;
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect.shrink(3.0), CornerRadius::same(6), theme::HOVER);
    }

    let color = if active { theme::TEXT } else { theme::MUTED };
    let font = FontId::proportional(11.0);
    let galley = painter.layout_no_wrap(label.to_string(), font, color);
    let x = if align_right {
        rect.right() - 10.0 - galley.size().x
    } else {
        rect.left() + 2.0
    };
    let y = rect.center().y - galley.size().y / 2.0;
    painter.galley(Pos2::new(x, y), galley.clone(), color);

    if active {
        let cx = if align_right {
            x - 12.0
        } else {
            x + galley.size().x + 12.0
        };
        let cy = rect.center().y;
        let points = if sort.ascending {
            vec![
                Pos2::new(cx - 4.0, cy + 2.0),
                Pos2::new(cx + 4.0, cy + 2.0),
                Pos2::new(cx, cy - 3.0),
            ]
        } else {
            vec![
                Pos2::new(cx - 4.0, cy - 2.0),
                Pos2::new(cx + 4.0, cy - 2.0),
                Pos2::new(cx, cy + 3.0),
            ]
        };
        painter.add(Shape::convex_polygon(points, theme::ACCENT, Stroke::NONE));
    }

    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn painter_eq(painter: &egui::Painter, center: Pos2, time: f32, playing: bool) {
    let bars = 4;
    let gap = 3.6;
    let start_x = center.x - (bars as f32 - 1.0) * gap / 2.0 - 1.0;
    for bar in 0..bars {
        let phase = time * 7.0 + bar as f32 * 1.4;
        let height = if playing {
            4.0 + (phase.sin() * 0.5 + 0.5) * 11.0
        } else {
            4.0 + bar as f32 * 1.8
        };
        let rect = Rect::from_min_size(
            Pos2::new(start_x + bar as f32 * gap, center.y + 8.0 - height),
            Vec2::new(2.4, height),
        );
        painter.rect_filled(rect, CornerRadius::same(1), theme::ACCENT);
    }
}

fn progress_bar(ui: &mut Ui, fraction: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 6.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(3), theme::BORDER);
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        let filled =
            Rect::from_min_size(rect.min, Vec2::new(rect.width() * fraction, rect.height()));
        ui.painter()
            .rect_filled(filled, CornerRadius::same(3), theme::ACCENT);
    }
}
