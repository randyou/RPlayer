//! Central content area: header, actions, sortable track table, folders page
//! and the various empty states.

use eframe::egui;
use egui::{
    Align, Align2, CornerRadius, FontId, Frame, Label, Layout, Margin, Pos2, Rect, RichText,
    ScrollArea, Sense, Stroke, Ui, Vec2,
};

use super::{Dialog, RPlayerApp, View};
use crate::model::{self, SortField, SortState};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

const ROW_HEIGHT: f32 = 46.0;
const HEADER_HEIGHT: f32 = 34.0;
const SIDE_PAD: f32 = 14.0;

#[derive(Clone, Copy)]
struct Columns {
    index: f32,
    title: f32,
    artist: f32,
    album: f32,
    rating: f32,
    format: f32,
    duration: f32,
    actions: f32,
}

impl Columns {
    fn for_width(width: f32) -> Self {
        let index = 36.0;
        let artist = (width * 0.15).clamp(90.0, 150.0);
        let album = (width * 0.15).clamp(90.0, 160.0);
        let rating = 60.0;
        let format = 44.0;
        let duration = 50.0;
        let actions = 56.0;
        let title = (width
            - SIDE_PAD * 2.0
            - index
            - artist
            - album
            - rating
            - format
            - duration
            - actions)
            .max(96.0);
        Self {
            index,
            title,
            artist,
            album,
            rating,
            format,
            duration,
            actions,
        }
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

    fn rating_rect(&self, row: Rect) -> Rect {
        let left = self.album_rect(row).right();
        Rect::from_min_size(
            Pos2::new(left, row.top()),
            Vec2::new(self.rating, row.height()),
        )
    }

    fn format_rect(&self, row: Rect) -> Rect {
        let left = self.rating_rect(row).right();
        Rect::from_min_size(
            Pos2::new(left, row.top()),
            Vec2::new(self.format, row.height()),
        )
    }

    fn duration_rect(&self, row: Rect) -> Rect {
        let left = self.format_rect(row).right();
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
    pub(super) fn central_panel(&mut self, ctx: &egui::Context, ids: &[u64], total_seconds: f64) {
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(theme::c().bg).inner_margin(Margin {
                left: 24,
                right: 20,
                top: 20,
                bottom: 12,
            }))
            .show(ctx, |ui| {
                self.central_header(ui, ids, total_seconds);
                ui.add_space(14.0);

                if self.view == View::Folders {
                    self.folders_page(ui);
                    return;
                }

                if self.library.tracks.is_empty() {
                    self.empty_library_state(ui);
                    return;
                }

                if self.batch_mode {
                    self.batch_bar(ui, ids);
                    ui.add_space(8.0);
                }

                Frame::NONE
                    .fill(theme::c().panel)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::symmetric(0, 4))
                    .show(ui, |ui| {
                        let columns = Columns::for_width(ui.available_width());
                        self.table_header(ui, columns);

                        if ids.is_empty() {
                            self.no_results_state(ui);
                        } else {
                            self.track_table(ui, ids, columns);
                        }
                    });
            });
    }

    fn central_header(&mut self, ui: &mut Ui, ids: &[u64], total_seconds: f64) {
        let (title, subtitle) = self.view_header(ids.len(), total_seconds);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(title).size(27.0).strong().color(theme::c().text));
                ui.label(RichText::new(subtitle).size(12.0).color(theme::c().muted));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let scan = widgets::icon_button(ui, Icon::Repeat, 34.0, "重新扫描音乐库");
                if scan.clicked() && !self.scanning && !self.library.folders.is_empty() {
                    self.start_scan();
                }

                ui.add_space(4.0);
                let response = ui
                    .allocate_ui_with_layout(
                        Vec2::new(232.0, 34.0),
                        Layout::left_to_right(Align::Center),
                        |ui| widgets::search_box(ui, &mut self.search, "搜索歌曲、艺术家、专辑"),
                    )
                    .inner;
                if std::mem::take(&mut self.focus_search) {
                    response.request_focus();
                }

                ui.add_space(6.0);
                let compact = self.current.is_some()
                    && self.now_rail_open
                    && Self::rail_available(ui.ctx());
                Frame::NONE
                    .fill(theme::c().card)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(99))
                    .inner_margin(Margin::symmetric(4, 3))
                    .show(ui, |ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(if compact { 128.0 } else { 188.0 }, 30.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                            ui.spacing_mut().item_spacing.x = 1.0;
                            let theme_icon = if self.theme_mode.is_dark() {
                                Icon::Sun
                            } else {
                                Icon::Moon
                            };
                            if widgets::icon_button(
                                ui,
                                theme_icon,
                                30.0,
                                if self.theme_mode.is_dark() {
                                    "切换到浅色主题"
                                } else {
                                    "切换到深色主题"
                                },
                            )
                            .clicked()
                            {
                                self.toggle_theme(ui.ctx());
                            }
                            if widgets::icon_button_tinted(
                                ui,
                                Icon::Equalizer,
                                30.0,
                                theme::c().muted,
                                self.eq_open,
                                "均衡器",
                            )
                            .clicked()
                            {
                                self.toggle_eq();
                            }
                            if !compact {
                                if widgets::icon_button_tinted(
                                    ui,
                                    Icon::Lyrics,
                                    30.0,
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
                                    30.0,
                                    theme::c().muted,
                                    self.mini_open,
                                    "迷你播放器",
                                )
                                .clicked()
                                {
                                    self.toggle_mini(ui.ctx());
                                }
                            }
                            if widgets::icon_button_tinted(
                                ui,
                                Icon::Settings,
                                30.0,
                                theme::c().muted,
                                self.settings_open,
                                "设置",
                            )
                            .clicked()
                            {
                                self.settings_open = !self.settings_open;
                            }
                            if widgets::icon_button_tinted(
                                ui,
                                Icon::PanelRight,
                                30.0,
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
                                    self.toast("先播放一首歌曲", super::ToastKind::Info);
                                }
                            }
                        });
                    });
            });
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            if !ids.is_empty() && widgets::primary_button(ui, "播放全部").clicked() {
                self.play_queue(ui.ctx(), ids.to_vec(), 0);
            }
            if self.view != View::Folders && widgets::secondary_button(ui, "添加").clicked() {
                self.open_folder_picker();
            }

            if self.view != View::Folders {
                let label = if self.batch_mode { "完成" } else { "批量操作" };
                if widgets::secondary_button(ui, label).clicked() {
                    self.batch_mode = !self.batch_mode;
                    if !self.batch_mode {
                        self.checked.clear();
                    }
                }
            }

            if self.batch_mode {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("已选 {} 首", self.checked.len()))
                        .size(11.5)
                        .color(theme::c().accent),
                );
                if widgets::chip(ui, "全选", false).clicked() {
                    self.set_all_checked(ids, true);
                }
                if widgets::chip(ui, "清空", false).clicked() {
                    self.checked.clear();
                }
                if matches!(self.view, View::Playlist(_) | View::Favorites)
                    && !self.checked.is_empty()
                    && widgets::chip(ui, "移除选中", false).clicked()
                {
                    self.batch_remove_from_current();
                }
            }
        });
    }

    fn batch_bar(&mut self, ui: &mut Ui, _ids: &[u64]) {
        Frame::NONE
            .fill(theme::accent_alpha(0.06))
            .stroke(Stroke::new(1.0_f32, theme::accent_alpha(0.16)))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(12, 7))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if self.checked.is_empty() {
                            "批量操作：点击歌曲勾选".to_string()
                        } else {
                            format!("已选择 {} 首歌曲", self.checked.len())
                        })
                        .size(11.5)
                        .color(if self.checked.is_empty() {
                            theme::c().muted
                        } else {
                            theme::c().accent
                        }),
                    );
                });

                if !self.checked.is_empty() {
                    let playlists: Vec<(u64, String)> = self
                        .library
                        .playlists
                        .iter()
                        .map(|playlist| (playlist.id, playlist.name.clone()))
                        .collect();
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("加入歌单")
                                .size(11.5)
                                .color(theme::c().faint),
                        );
                        if playlists.is_empty() {
                            if widgets::chip(ui, "新建歌单…", false).clicked() {
                                self.dialog = Dialog::NewPlaylist {
                                    name: String::new(),
                                };
                                self.focus_pending = true;
                            }
                        } else {
                            for (playlist_id, name) in &playlists {
                                if widgets::chip(ui, name, false).clicked() {
                                    self.batch_add_to_playlist(*playlist_id);
                                }
                            }
                        }
                    });
                }
            });
    }

    fn table_header(&mut self, ui: &mut Ui, columns: Columns) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, HEADER_HEIGHT), Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(9), theme::c().card);

        ui.painter().text(
            Pos2::new(columns.index_rect(rect).left() + 5.0, rect.center().y),
            Align2::LEFT_CENTER,
            "#",
            FontId::proportional(11.0),
            theme::c().faint,
        );

        let sort = self.sort;
        let mut clicked: Option<SortField> = None;

        if sort_column(
            ui,
            columns.title_rect(rect),
            "曲名",
            SortField::Title,
            sort,
            false,
        ) {
            clicked = Some(SortField::Title);
        }
        if sort_column(
            ui,
            columns.artist_rect(rect),
            "歌手",
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

        ui.painter().text(
            Pos2::new(columns.rating_rect(rect).left() + 2.0, rect.center().y),
            Align2::LEFT_CENTER,
            "评分",
            FontId::proportional(11.0),
            theme::c().faint,
        );

        ui.painter().text(
            Pos2::new(columns.format_rect(rect).left() + 6.0, rect.center().y),
            Align2::LEFT_CENTER,
            "格式",
            FontId::proportional(11.0),
            theme::c().faint,
        );

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

        ui.painter().text(
            Pos2::new(columns.actions_rect(rect).center().x, rect.center().y),
            Align2::CENTER_CENTER,
            "操作",
            FontId::proportional(11.0),
            theme::c().faint,
        );

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
                    let checked = self.checked.contains(&id);
                    let hovered = response.hovered();

                    let painter = ui.painter();
                    let pill = rect.shrink2(Vec2::new(0.0, 1.0));
                    if current {
                        painter.rect_filled(pill, CornerRadius::same(9), theme::accent_alpha(0.08));
                        painter.rect_filled(
                            Rect::from_min_size(
                                Pos2::new(pill.left() + 1.0, pill.center().y - 9.0),
                                Vec2::new(3.0, 18.0),
                            ),
                            CornerRadius::same(2),
                            theme::c().accent,
                        );
                    } else if checked {
                        painter.rect_filled(pill, CornerRadius::same(9), theme::accent_alpha(0.05));
                    } else if selected {
                        painter.rect_filled(pill, CornerRadius::same(9), theme::c().hover);
                    } else if hovered {
                        painter.rect_filled(pill, CornerRadius::same(9), theme::c().card);
                    }

                    let index_rect = columns.index_rect(rect);
                    if self.batch_mode {
                        let box_rect =
                            Rect::from_center_size(index_rect.center(), Vec2::splat(15.0));
                        painter.rect_filled(
                            box_rect,
                            CornerRadius::same(4),
                            if checked { theme::c().accent } else { theme::c().card },
                        );
                        painter.rect_stroke(
                            box_rect,
                            CornerRadius::same(4),
                            Stroke::new(
                                1.0_f32,
                                if checked { theme::c().accent } else { theme::c().border },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        if checked {
                            icons::paint(painter, box_rect.shrink(2.5), Icon::Check, theme::c().on_accent);
                        }
                    } else if current {
                        let playing = self.is_playing();
                        let time = ui.input(|i| i.time) as f32;
                        painter_eq(painter, index_rect.center(), time, playing);
                    } else if hovered {
                        icons::paint(
                            painter,
                            Rect::from_center_size(index_rect.center(), Vec2::splat(11.0)),
                            Icon::Play,
                            theme::c().text,
                        );
                    } else {
                        painter.text(
                            index_rect.center(),
                            Align2::CENTER_CENTER,
                            format!("{}", index + 1),
                            FontId::proportional(11.0),
                            theme::c().faint,
                        );
                    }

                    let title_rect = columns.title_rect(rect);
                    let cover_rect = Rect::from_center_size(
                        Pos2::new(title_rect.left() + 15.0, title_rect.center().y),
                        Vec2::splat(28.0),
                    );
                    painter.rect_filled(cover_rect, CornerRadius::same(6), theme::c().card);
                    icons::paint(
                        painter,
                        cover_rect.shrink(7.0),
                        Icon::Music,
                        if current { theme::c().accent } else { theme::c().faint },
                    );

                    let text_left = title_rect.left() + 36.0;
                    let title_clip = painter.with_clip_rect(Rect::from_min_max(
                        Pos2::new(text_left, title_rect.top()),
                        Pos2::new(title_rect.right() - 2.0, title_rect.bottom()),
                    ));
                    title_clip.text(
                        Pos2::new(text_left, title_rect.center().y),
                        Align2::LEFT_CENTER,
                        &track.title,
                        FontId::proportional(13.0),
                        if current { theme::c().accent } else { theme::c().text },
                    );

                    let artist_rect = columns.artist_rect(rect);
                    let artist_clip = painter.with_clip_rect(artist_rect);
                    artist_clip.text(
                        Pos2::new(artist_rect.left() + 2.0, artist_rect.center().y),
                        Align2::LEFT_CENTER,
                        &track.artist,
                        FontId::proportional(12.0),
                        if current { theme::c().text } else { theme::c().muted },
                    );

                    let album_rect = columns.album_rect(rect);
                    painter.with_clip_rect(album_rect).text(
                        Pos2::new(album_rect.left() + 2.0, album_rect.center().y),
                        Align2::LEFT_CENTER,
                        &track.album,
                        FontId::proportional(11.5),
                        if current { theme::c().muted } else { theme::c().faint },
                    );

                    painter.text(
                        Pos2::new(
                            columns.format_rect(rect).left() + 6.0,
                            columns.format_rect(rect).center().y,
                        ),
                        Align2::LEFT_CENTER,
                        track.format_label(),
                        FontId::proportional(10.0),
                        if current { theme::c().muted } else { theme::c().faint },
                    );

                    painter.text(
                        Pos2::new(
                            columns.duration_rect(rect).right() - 2.0,
                            columns.duration_rect(rect).center().y,
                        ),
                        Align2::RIGHT_CENTER,
                        model::format_duration(track.duration),
                        FontId::proportional(11.0),
                        if current { theme::c().muted } else { theme::c().faint },
                    );

                    let rating_rect = columns.rating_rect(rect);
                    let rating = self.rating(id);
                    let pointer = response.hover_pos();
                    let hover_rating = pointer.and_then(|pos| {
                        if rating_rect.contains(pos) {
                            Some(
                                (((pos.x - rating_rect.left()) / 11.0).floor() as u8 + 1).min(5),
                            )
                        } else {
                            None
                        }
                    });
                    for index in 0..5 {
                        let star = Rect::from_center_size(
                            Pos2::new(
                                rating_rect.left() + 4.0 + index as f32 * 11.0,
                                rating_rect.center().y,
                            ),
                            Vec2::splat(9.0),
                        );
                        let filled = index < rating || hover_rating.is_some_and(|h| index < h);
                        icons::paint(
                            ui.painter(),
                            star,
                            if filled { Icon::StarFilled } else { Icon::Star },
                            if filled {
                                theme::c().accent
                            } else {
                                theme::c().faint
                            },
                        );
                    }
                    let rating_clicked = if response.clicked() {
                        pointer.and_then(|pos| {
                            if rating_rect.contains(pos) {
                                Some(
                                    (((pos.x - rating_rect.left()) / 11.0).floor() as u8 + 1)
                                        .min(5),
                                )
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    };

                    let actions_rect = columns.actions_rect(rect);
                    let heart_rect = Rect::from_center_size(
                        Pos2::new(actions_rect.left() + 18.0, actions_rect.center().y),
                        Vec2::splat(26.0),
                    );
                    let favorite = self.library.is_favorite(id);
                    let heart_hovered = pointer.is_some_and(|pos| heart_rect.contains(pos));
                    icons::paint(
                        ui.painter(),
                        heart_rect.shrink(6.0),
                        if favorite {
                            Icon::HeartFilled
                        } else {
                            Icon::Heart
                        },
                        if favorite {
                            theme::c().accent
                        } else if heart_hovered {
                            theme::c().text
                        } else {
                            theme::c().faint
                        },
                    );

                    let more_rect = Rect::from_center_size(
                        Pos2::new(actions_rect.right() - 12.0, actions_rect.center().y),
                        Vec2::splat(26.0),
                    );
                    icons::paint(ui.painter(), more_rect.shrink(7.0), Icon::More, theme::c().faint);

                    let click_pos = response.interact_pointer_pos();
                    let heart_clicked =
                        response.clicked() && click_pos.is_some_and(|pos| heart_rect.contains(pos));
                    if let Some(value) = rating_clicked {
                        let next = if self.rating(id) == value { 0 } else { value };
                        self.set_rating(id, next);
                    } else if heart_clicked {
                        self.toggle_favorite(id);
                    } else if response.double_clicked() {
                        self.play_track(ui.ctx(), id);
                    } else if response.clicked() {
                        if self.batch_mode {
                            self.toggle_checked(id);
                        } else {
                            self.selected = Some(id);
                        }
                    }

                    response.context_menu(|ui| {
                        if ui.button("播放").clicked() {
                            self.play_track(ui.ctx(), id);
                            ui.close_menu();
                        }
                        if ui
                            .button(if self.library.is_favorite(id) {
                                "取消收藏"
                            } else {
                                "添加到我的收藏"
                            })
                            .clicked()
                        {
                            self.toggle_favorite(id);
                            ui.close_menu();
                        }

                        ui.menu_button("评分", |ui| {
                            let current = self.rating(id);
                            for value in 1..=5u8 {
                                let label = if value == current {
                                    format!("{value} 星（当前）")
                                } else {
                                    format!("{value} 星")
                                };
                                if ui.button(label).clicked() {
                                    self.set_rating(id, value);
                                    ui.close_menu();
                                }
                            }
                            if current > 0 && ui.button("清除评分").clicked() {
                                self.set_rating(id, 0);
                                ui.close_menu();
                            }
                        });

                        ui.menu_button("添加到播放列表", |ui| {
                            let playlists: Vec<(u64, String)> = self
                                .library
                                .playlists
                                .iter()
                                .map(|playlist| (playlist.id, playlist.name.clone()))
                                .collect();
                            if playlists.is_empty() {
                                ui.label(
                                    RichText::new("还没有播放列表")
                                        .size(11.5)
                                        .color(theme::c().faint),
                                );
                                if ui.button("新建播放列表…").clicked() {
                                    self.dialog = Dialog::NewPlaylist {
                                        name: String::new(),
                                    };
                                    self.focus_pending = true;
                                    ui.close_menu();
                                }
                            } else {
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
                        if ui.button("从 LRCLIB 下载歌词").clicked() {
                            self.request_lyrics(ui.ctx(), id, true);
                            ui.close_menu();
                        }
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
                        .filter(|track| super::path_is_under(&folder, &track.path))
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
                                    if widgets::icon_button(ui, Icon::Trash, 28.0, "移除此目录")
                                        .clicked()
                                    {
                                        self.remove_folder(&folder);
                                        self.toast("已移除音乐目录", super::ToastKind::Info);
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
            ui.add_space(30.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(96.0), Sense::hover());
            icons::logo(ui.painter(), rect);
            ui.add_space(18.0);
            ui.label(
                RichText::new("让本地音乐，回到桌面")
                    .size(25.0)
                    .strong()
                    .color(theme::c().text),
            );
            ui.add_space(6.0);
            ui.add(
                Label::new(
                    RichText::new(
                        "RPlayer 是一个纯粹、快速的本地播放器。\n添加音乐文件夹，它会把歌曲整理进你的音乐库。",
                    )
                    .size(12.5)
                    .color(theme::c().muted),
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
                    .color(theme::c().faint),
                );
            }
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

fn sort_column(
    ui: &mut Ui,
    rect: Rect,
    label: &str,
    field: SortField,
    sort: SortState,
    align_right: bool,
) -> bool {
    let response = ui.interact(rect, egui::Id::new(("rplayer-sort", label)), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let color = if sort.field == field {
        theme::c().text
    } else {
        theme::c().faint
    };
    let anchor = if align_right {
        Align2::RIGHT_CENTER
    } else {
        Align2::LEFT_CENTER
    };
    let x = if align_right {
        rect.right() - 2.0
    } else {
        rect.left() + 2.0
    };
    ui.painter().text(
        Pos2::new(x, rect.center().y),
        anchor,
        label,
        FontId::proportional(11.0),
        color,
    );

    if sort.field == field {
        let center = Pos2::new(
            if align_right {
                rect.right() - 4.0
            } else {
                rect.left() + 4.0 + ui.painter().layout_no_wrap(
                    label.to_string(),
                    FontId::proportional(11.0),
                    color,
                ).size().x
                    + 8.0
            },
            rect.center().y,
        );
        let points = if sort.ascending {
            vec![
                Pos2::new(center.x - 3.5, center.y + 2.0),
                Pos2::new(center.x, center.y - 2.0),
                Pos2::new(center.x + 3.5, center.y + 2.0),
            ]
        } else {
            vec![
                Pos2::new(center.x - 3.5, center.y - 2.0),
                Pos2::new(center.x, center.y + 2.0),
                Pos2::new(center.x + 3.5, center.y - 2.0),
            ]
        };
        ui.painter().add(egui::Shape::convex_polygon(
            points,
            theme::c().accent,
            Stroke::NONE,
        ));
    }

    response.clicked()
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

fn progress_bar(ui: &mut Ui, fraction: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 5.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(99), theme::c().border);
    let filled = Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
    );
    ui.painter()
        .rect_filled(filled, CornerRadius::same(99), theme::c().accent);
}
