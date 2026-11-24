//! Central content area: header, actions, sortable track table, folders page
//! and the various empty states.

use eframe::egui;
use egui::{
    Align, Align2, CornerRadius, FontId, Frame, Label, Layout, Margin, Pos2, Rect, RichText,
    ScrollArea, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType,
};

use super::{Dialog, RPlayerApp, View};
use crate::model::{self, SortField, SortState};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

const ROW_HEIGHT: f32 = 56.0;
const HEADER_HEIGHT: f32 = 38.0;
const SIDE_PAD: f32 = 18.0;

#[derive(Clone, Copy)]
struct Columns {
    index: f32,
    title: f32,
    album: f32,
    duration: f32,
    actions: f32,
}

impl Columns {
    fn for_width(width: f32) -> Self {
        let index = 40.0;
        let album = (width * 0.27).clamp(180.0, 360.0);
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
    pub(super) fn central_panel(&mut self, ctx: &egui::Context, ids: &[u64], total_seconds: f64) {
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(theme::c().bg).inner_margin(Margin {
                left: 32,
                right: 28,
                top: 22,
                bottom: 12,
            }))
            .show(ctx, |ui| {
                let dt = ctx.input(|input| input.stable_dt).min(0.1);
                if crate::ui::motion::reduced() {
                    self.view_fade = 1.0;
                } else {
                    self.view_fade = (self.view_fade + dt * 4.0).min(1.0);
                }
                if self.view_fade < 1.0 {
                    ctx.request_repaint();
                }
                let fade = 1.0 - (1.0 - self.view_fade).powi(3);
                ui.add_space((1.0 - fade) * 10.0);
                ui.multiply_opacity(fade);

                if self.view == View::Albums && self.album_focus.is_some() {
                    self.albums_page(ui, ids);
                    return;
                }
                if self.view == View::Artists && self.artist_focus.is_some() {
                    self.artists_page(ui, ids);
                    return;
                }

                self.central_header(ui, ids, total_seconds);
                ui.add_space(10.0);
                if matches!(self.view, View::Library | View::Albums | View::Artists) {
                    self.browse_tabs(ui);
                    ui.add_space(12.0);
                } else {
                    ui.add_space(4.0);
                }

                if self.view == View::Folders {
                    self.folders_page(ui);
                    return;
                }
                if self.view == View::Albums {
                    self.albums_page(ui, ids);
                    return;
                }
                if self.view == View::Artists {
                    self.artists_page(ui, ids);
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
                    .fill(theme::c().bg)
                    .inner_margin(Margin::symmetric(0, 0))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
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
                ui.label(
                    RichText::new(title)
                        .font(crate::ui::tokens::serif(42.0))
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
                        self.toast("先播放一首歌曲", super::ToastKind::Info);
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
                            "批量操作：Ctrl 点选 · Shift 范围选择".to_string()
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
                        if widgets::chip(ui, "加入播放队列", false).clicked() {
                            let ids: Vec<u64> = self.checked.iter().copied().collect();
                            let added = self.append_to_queue(&ids);
                            if added > 0 {
                                self.toast(
                                    format!("已加入播放队列：{added} 首"),
                                    super::ToastKind::Info,
                                );
                            }
                        }
                        if widgets::chip(ui, "编辑标签…", false).clicked() {
                            let ids: Vec<u64> = self.checked.iter().copied().collect();
                            if !ids.is_empty() {
                                self.dialog = Dialog::BatchTags {
                                    ids,
                                    title: String::new(),
                                    artist: String::new(),
                                    album: String::new(),
                                    genre: String::new(),
                                    year: String::new(),
                                };
                            }
                        }
                    });
                }
            });
    }

    fn table_header(&mut self, ui: &mut Ui, columns: Columns) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, HEADER_HEIGHT), Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::ZERO, theme::c().bg);
        ui.painter().line_segment(
            [
                Pos2::new(rect.left(), rect.bottom() - 0.5),
                Pos2::new(rect.right(), rect.bottom() - 0.5),
            ],
            Stroke::new(1.0_f32, theme::c().border_soft),
        );

        let header_font = crate::ui::tokens::mono(9.5);
        ui.painter().text(
            columns.index_rect(rect).center(),
            Align2::CENTER_CENTER,
            "#",
            header_font.clone(),
            theme::c().faint,
        );

        let sort = self.sort;
        let mut clicked: Option<SortField> = None;

        if sort_column(
            ui,
            columns.title_rect(rect),
            "曲目",
            SortField::Title,
            sort,
            false,
        ) {
            clicked = Some(SortField::Title);
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
        let page_rows = (ui.available_height() / ROW_HEIGHT).floor().max(1.0) as usize;
        let mut area = ScrollArea::vertical()
            .id_salt("rplayer-track-table")
            .auto_shrink([false, false]);
        if let Some(index) = self.table_scroll_to.take() {
            let viewport = ui.available_height().max(ROW_HEIGHT);
            let offset = table_scroll_offset(index, viewport, ROW_HEIGHT);
            area = area.vertical_scroll_offset(offset);
        }
        area.show_rows(ui, ROW_HEIGHT, ids.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.set_width(width);

            let focused = ui.ctx().memory(|memory| memory.focused());
            let focused_index = self.table_focus.and_then(|id| {
                let index = ids.iter().position(|candidate| *candidate == id)?;
                (focused == Some(track_row_id(ui, id))).then_some(index)
            });
            if self.table_focus.is_some() && focused_index.is_none() {
                if !ids.iter().any(|id| Some(*id) == self.table_focus) {
                    self.table_focus = None;
                }
            }

            let restoring = std::mem::take(&mut self.table_restore);
            if restoring {
                let target = self
                    .table_focus
                    .filter(|id| ids.contains(id))
                    .or_else(|| self.selected.filter(|id| ids.contains(id)))
                    .or_else(|| ids.first().copied());
                if let Some(id) = target {
                    if let Some(index) = ids.iter().position(|candidate| *candidate == id) {
                        self.table_focus = Some(id);
                        self.table_scroll_to = Some(index);
                        ui.ctx()
                            .memory_mut(|memory| memory.request_focus(track_row_id(ui, id)));
                    }
                }
            }

            if let Some(current) = focused_index {
                let movement = ui.input(|input| {
                    if input.key_pressed(egui::Key::ArrowUp) {
                        Some(TableMove::Up)
                    } else if input.key_pressed(egui::Key::ArrowDown) {
                        Some(TableMove::Down)
                    } else if input.key_pressed(egui::Key::Home) {
                        Some(TableMove::Home)
                    } else if input.key_pressed(egui::Key::End) {
                        Some(TableMove::End)
                    } else if input.key_pressed(egui::Key::PageUp) {
                        Some(TableMove::PageUp)
                    } else if input.key_pressed(egui::Key::PageDown) {
                        Some(TableMove::PageDown)
                    } else {
                        None
                    }
                });
                if let Some(movement) = movement {
                    if let Some(target) = table_target(current, ids.len(), movement, page_rows) {
                        if target != current {
                            let id = ids[target];
                            self.table_focus = Some(id);
                            self.table_scroll_to = Some(target);
                            ui.ctx()
                                .memory_mut(|memory| memory.request_focus(track_row_id(ui, id)));
                        }
                    }
                }
            }

            for index in range {
                let id = ids[index];
                let Some(track) = self.library.track(id).cloned() else {
                    continue;
                };
                let (_, rect) = ui.allocate_space(Vec2::new(width, ROW_HEIGHT));
                let response =
                    ui.interact(rect, track_row_id(ui, id), Sense::click_and_drag());
                let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                crate::ui::focus::focus_on_click(&response);
                if response.has_focus() {
                    self.table_focus = Some(id);
                }
                let current = self.current == Some(id);
                let selected = self.selected == Some(id);
                let checked = self.checked.contains(&id);
                let hovered = response.hovered();
                let hover_t = ui.ctx().animate_bool_with_time(
                    egui::Id::new(("track-row-hover", id)),
                    hovered && !current && !selected && !checked,
                    crate::ui::motion::duration(crate::ui::motion::FAST),
                );
                let row_label = format!(
                    "{} — {} · {} · {} · {}",
                    track.title,
                    track.artist,
                    track.album,
                    track
                        .path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .unwrap_or("?")
                        .to_uppercase(),
                    track.path.display()
                );
                response.widget_info(move || {
                    WidgetInfo::selected(
                        WidgetType::Button,
                        true,
                        current || checked,
                        row_label.clone(),
                    )
                });

                    let painter = ui.painter();
                    let pill = rect;
                    let row_radius = CornerRadius::ZERO;
                    if current {
                        painter.rect_filled(pill, row_radius, theme::accent_soft());
                        painter.rect_filled(
                            Rect::from_min_size(
                                Pos2::new(pill.left() + 0.5, pill.top() + 8.0),
                                Vec2::new(2.0, pill.height() - 16.0),
                            ),
                            CornerRadius::same(1),
                            theme::c().accent,
                        );
                    } else if checked {
                        painter.rect_filled(pill, row_radius, theme::accent_alpha(0.06));
                    } else if selected {
                        painter.rect_filled(pill, row_radius, theme::c().hover);
                    } else if hover_t > 0.001 {
                        let fill = theme::c().card;
                        painter.rect_filled(
                            pill,
                            row_radius,
                            egui::Color32::from_rgba_unmultiplied(
                                fill.r(),
                                fill.g(),
                                fill.b(),
                                (255.0 * hover_t) as u8,
                            ),
                        );
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
                        Pos2::new(title_rect.left() + 19.0, title_rect.center().y),
                        Vec2::splat(38.0),
                    );
                    painter.rect_filled(
                        cover_rect,
                        CornerRadius::same(2),
                        theme::c().card,
                    );
                    icons::paint(
                        painter,
                        cover_rect.shrink(10.0),
                        Icon::Music,
                        if current { theme::c().accent } else { theme::c().faint },
                    );

                    let text_left = title_rect.left() + 46.0;
                    let title_font = FontId::proportional(13.0);
                    let title_color = theme::c().text;
                    let title_clip = painter.with_clip_rect(Rect::from_min_max(
                        Pos2::new(text_left, title_rect.top()),
                        Pos2::new(title_rect.right() - 2.0, title_rect.bottom()),
                    ));
                    title_clip.text(
                        Pos2::new(text_left, title_rect.center().y - 7.0),
                        Align2::LEFT_CENTER,
                        &track.title,
                        title_font.clone(),
                        title_color,
                    );
                    let galley = painter.layout_no_wrap(track.title.clone(), title_font, title_color);
                    let badge_x = text_left + galley.size().x + 8.0;
                    if badge_x + 28.0 < title_rect.right() {
                        let badge = Rect::from_min_size(
                            Pos2::new(badge_x, title_rect.center().y - 14.0),
                            Vec2::new(24.0, 13.0),
                        );
                        painter.rect_filled(badge, CornerRadius::same(3), theme::accent_soft());
                        painter.text(
                            badge.center(),
                            Align2::CENTER_CENTER,
                            track.format_label(),
                            crate::ui::tokens::mono(8.0),
                            theme::c().accent,
                        );
                    }

                    painter.with_clip_rect(title_rect).text(
                        Pos2::new(text_left, title_rect.center().y + 9.0),
                        Align2::LEFT_CENTER,
                        &track.artist,
                        FontId::proportional(9.5),
                        theme::c().muted,
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
                            columns.duration_rect(rect).right() - 4.0,
                            columns.duration_rect(rect).center().y,
                        ),
                        Align2::RIGHT_CENTER,
                        model::format_duration(track.duration),
                        crate::ui::tokens::mono(10.5),
                        theme::c().faint,
                    );

                    let actions_rect = columns.actions_rect(rect);
                    let heart_rect = Rect::from_center_size(
                        Pos2::new(actions_rect.left() + 16.0, actions_rect.center().y),
                        Vec2::splat(26.0),
                    );
                    let favorite = self.library.is_favorite(id);
                    let pointer = response.hover_pos();
                    let heart_hovered = pointer.is_some_and(|pos| heart_rect.contains(pos));
                    if favorite || hovered || current || selected {
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
                    }

                    let more_rect = Rect::from_center_size(
                        Pos2::new(actions_rect.right() - 10.0, actions_rect.center().y),
                        Vec2::splat(26.0),
                    );
                    if hovered || current || selected {
                        icons::paint(
                            ui.painter(),
                            more_rect.shrink(7.0),
                            Icon::More,
                            theme::c().faint,
                        );
                    }

                    let click_pos = response.interact_pointer_pos();
                    let heart_clicked =
                        response.clicked() && click_pos.is_some_and(|pos| heart_rect.contains(pos));
                    let keyboard_enter =
                        response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    if keyboard_enter {
                        self.play_track(ui.ctx(), id);
                    } else if heart_clicked {
                        self.toggle_favorite(id);
                    } else if response.double_clicked() {
                        self.play_track(ui.ctx(), id);
                    } else if response.clicked() {
                        let (shift, command) = ui.input(|input| {
                            (
                                input.modifiers.shift,
                                input.modifiers.command || input.modifiers.ctrl,
                            )
                        });
                        if shift {
                            self.select_range(ids, id);
                        } else if command {
                            self.batch_mode = true;
                            self.toggle_checked(id);
                        } else if self.batch_mode {
                            self.toggle_checked(id);
                        } else {
                            self.selected = Some(id);
                            self.selection_anchor = Some(id);
                        }
                    }

                    let drag_ids: Vec<u64> = if self.checked.contains(&id) {
                        ids.iter()
                            .filter(|candidate| self.checked.contains(*candidate))
                            .copied()
                            .collect()
                    } else {
                        vec![id]
                    };
                    response.dnd_set_drag_payload(drag_ids);

                    response.context_menu(|ui| {
                        if widgets::text_button(ui, "播放").clicked() {
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
                                if widgets::text_button(ui, label).clicked() {
                                    self.set_rating(id, value);
                                    ui.close_menu();
                                }
                            }
                            if current > 0 && widgets::text_button(ui, "清除评分").clicked() {
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
                                if widgets::text_button(ui, "新建播放列表…").clicked() {
                                    self.dialog = Dialog::NewPlaylist {
                                        name: String::new(),
                                    };
                                    self.focus_pending = true;
                                    ui.close_menu();
                                }
                            } else {
                                for (playlist_id, name) in playlists {
                                    if widgets::text_button(ui, name).clicked() {
                                        self.add_track_to_playlist(playlist_id, id);
                                        ui.close_menu();
                                    }
                                }
                            }
                        });

                        if matches!(self.view, View::Playlist(_))
                            && widgets::text_button(ui, "从此播放列表移除").clicked()
                        {
                            self.remove_track_from_view_playlist(id);
                            ui.close_menu();
                        }

                        ui.separator();
                        if widgets::text_button(ui, "查找重复歌曲…").clicked() {
                            self.open_duplicates();
                            ui.close_menu();
                        }
                        if widgets::text_button(ui, "编辑歌词…").clicked() {
                            self.current = Some(id);
                            self.load_lyrics_for(&track, id);
                            super::lyrics_edit::open(self);
                            ui.close_menu();
                        }
                        if widgets::text_button(ui, "重新匹配歌词…").clicked() {
                            self.open_lyrics_match(id);
                            ui.close_menu();
                        }
                        if widgets::text_button(ui, "从 LRCLIB 下载歌词").clicked() {
                            self.request_lyrics(ui.ctx(), id, true);
                            ui.close_menu();
                        }
                        if widgets::text_button(ui, "在文件夹中显示").clicked() {
                            self.reveal_in_file_manager(&track.path);
                            ui.close_menu();
                        }
                        if widgets::text_button(ui, "编辑标签…").clicked() {
                            if let Some(track) = self.library.track(id) {
                                self.dialog = Dialog::EditTags {
                                    edit: crate::tag_edit::TagEdit::from_track(track),
                                };
                                self.focus_pending = true;
                            }
                            ui.close_menu();
                        }
                        if widgets::text_button(ui, "从音乐库移除…").clicked() {
                            self.dialog = Dialog::ConfirmRemoveTrack { id };
                            ui.close_menu();
                        }
                    });

                    if response.has_focus() {
                        crate::ui::focus::ring_rect(ui, rect, 9);
                    }
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
                if widgets::primary_button_icon(ui, crate::ui::icons::Icon::Folder, "添加音乐文件夹").clicked() {
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

fn track_row_id(ui: &Ui, id: u64) -> egui::Id {
    ui.id().with(("track-row", id))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TableMove {
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

fn table_target(
    current: usize,
    len: usize,
    movement: TableMove,
    page_rows: usize,
) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let current = current.min(len - 1);
    let page = page_rows.max(1);
    let target = match movement {
        TableMove::Up => current.saturating_sub(1),
        TableMove::Down => (current + 1).min(len - 1),
        TableMove::Home => 0,
        TableMove::End => len - 1,
        TableMove::PageUp => current.saturating_sub(page),
        TableMove::PageDown => (current + page).min(len - 1),
    };
    Some(target)
}

fn table_scroll_offset(index: usize, viewport: f32, row_height: f32) -> f32 {
    (index as f32 * row_height - (viewport - row_height) * 0.5).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::{table_scroll_offset, table_target, TableMove};

    #[test]
    fn table_navigation_clamps_at_edges() {
        assert_eq!(table_target(0, 5, TableMove::Up, 3), Some(0));
        assert_eq!(table_target(4, 5, TableMove::Down, 3), Some(4));
        assert_eq!(table_target(2, 5, TableMove::Home, 3), Some(0));
        assert_eq!(table_target(2, 5, TableMove::End, 3), Some(4));
    }

    #[test]
    fn table_paging_uses_page_size_and_clamps() {
        assert_eq!(table_target(5, 20, TableMove::PageDown, 4), Some(9));
        assert_eq!(table_target(5, 20, TableMove::PageUp, 4), Some(1));
        assert_eq!(table_target(18, 20, TableMove::PageDown, 4), Some(19));
        assert_eq!(table_target(1, 3, TableMove::PageUp, 4), Some(0));
    }

    #[test]
    fn empty_table_has_no_target() {
        assert_eq!(table_target(0, 0, TableMove::Down, 5), None);
    }

    #[test]
    fn table_scroll_keeps_target_near_the_center() {
        assert_eq!(table_scroll_offset(0, 600.0, 50.0), 0.0);
        assert_eq!(table_scroll_offset(10, 600.0, 50.0), 225.0);
        assert_eq!(table_scroll_offset(2, 400.0, 50.0), 0.0);
    }
}
