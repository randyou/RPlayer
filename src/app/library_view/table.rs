use super::*;

impl RPlayerApp {
    pub(crate) fn table_header(&mut self, ui: &mut Ui, columns: Columns) {
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
        if columns.shows_album()
            && sort_column(
                ui,
                columns.album_rect(rect),
                "专辑",
                SortField::Album,
                sort,
                false,
            )
        {
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

    pub(crate) fn track_table(&mut self, ui: &mut Ui, ids: &[u64], columns: Columns) {
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
                let mut row_label = format!(
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
                // S9: rows are list buttons, not switches - announce state in the
                // name instead of a toggled flag so narration never says 关闭.
                if current {
                    row_label.push_str("（正在播放）");
                } else if checked {
                    row_label.push_str("（已选）");
                }
                response.widget_info(move || {
                    WidgetInfo::labeled(WidgetType::Button, true, row_label.clone())
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

                    if columns.shows_album() {
                        let album_rect = columns.album_rect(rect);
                        painter.with_clip_rect(album_rect).text(
                            Pos2::new(album_rect.left() + 2.0, album_rect.center().y),
                            Align2::LEFT_CENTER,
                            &track.album,
                            FontId::proportional(11.5),
                            if current { theme::c().muted } else { theme::c().faint },
                        );
                    }

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
                    if !response.context_menu_opened() {
                        response.dnd_set_drag_payload(drag_ids);
                    }

                    response.context_menu(|ui| {
                        menu::begin(ui);
                        menu::heading(ui, "曲目");
                        if menu::item(ui, Some(Icon::Play), "播放", Some("Enter"), false).clicked() {
                            self.play_track(ui.ctx(), id);
                            ui.close_menu();
                        }
                        let favorite = self.library.is_favorite(id);
                        if menu::item(
                            ui,
                            Some(if favorite { Icon::HeartFilled } else { Icon::Heart }),
                            if favorite { "取消收藏" } else { "添加到我的收藏" },
                            None,
                            false,
                        )
                        .clicked()
                        {
                            self.toggle_favorite(id);
                            ui.close_menu();
                        }

                        menu::submenu(ui, Some(Icon::Star), "评分", |ui| {
                            let current = self.rating(id);
                            for value in 1..=5u8 {
                                let label = if value == current {
                                    format!("{value} 星（当前）")
                                } else {
                                    format!("{value} 星")
                                };
                                if menu::item(
                                    ui,
                                    Some(if value <= current {
                                        Icon::StarFilled
                                    } else {
                                        Icon::Star
                                    }),
                                    &label,
                                    None,
                                    false,
                                )
                                .clicked()
                                {
                                    self.set_rating(id, value);
                                    ui.close_menu();
                                }
                            }
                            if current > 0 {
                                menu::separator(ui);
                                if menu::item(ui, Some(Icon::Trash), "清除评分", None, false)
                                    .clicked()
                                {
                                    self.set_rating(id, 0);
                                    ui.close_menu();
                                }
                            }
                        });

                        menu::submenu(ui, Some(Icon::Plus), "添加到播放列表", |ui| {
                            let playlists: Vec<(u64, String)> = self
                                .library
                                .playlists
                                .iter()
                                .map(|playlist| (playlist.id, playlist.name.clone()))
                                .collect();
                            if playlists.is_empty() {
                                menu::hint(ui, "还没有播放列表");
                                if menu::item(ui, Some(Icon::Plus), "新建播放列表…", None, false)
                                    .clicked()
                                {
                                    self.dialog = Dialog::NewPlaylist {
                                        name: String::new(),
                                    };
                                    self.focus_pending = true;
                                    ui.close_menu();
                                }
                            } else {
                                for (playlist_id, name) in playlists {
                                    if menu::item(ui, Some(Icon::List), &name, None, false).clicked()
                                    {
                                        self.add_track_to_playlist(playlist_id, id);
                                        ui.close_menu();
                                    }
                                }
                            }
                        });

                        if matches!(self.view, View::Playlist(_))
                            && menu::item(ui, Some(Icon::List), "从此播放列表移除", None, false)
                                .clicked()
                        {
                            self.remove_track_from_view_playlist(id);
                            ui.close_menu();
                        }

                        menu::separator(ui);
                        menu::heading(ui, "查找与歌词");
                        if menu::item(ui, Some(Icon::Search), "查找重复歌曲…", None, false)
                            .clicked()
                        {
                            self.open_duplicates();
                            ui.close_menu();
                        }
                        if menu::item(ui, Some(Icon::Edit), "编辑歌词…", None, false).clicked() {
                            self.current = Some(id);
                            self.load_lyrics_for(&track, id);
                            crate::app::lyrics_edit::open(self);
                            ui.close_menu();
                        }
                        if menu::item(ui, Some(Icon::Refresh), "重新匹配歌词…", None, false)
                            .clicked()
                        {
                            self.open_lyrics_match(id);
                            ui.close_menu();
                        }
                        if menu::item(ui, Some(Icon::Download), "从 LRCLIB 下载歌词", None, false)
                            .clicked()
                        {
                            self.request_lyrics(ui.ctx(), id, true);
                            ui.close_menu();
                        }

                        menu::separator(ui);
                        if menu::item(ui, Some(Icon::Folder), "在文件夹中显示", None, false)
                            .clicked()
                        {
                            self.reveal_in_file_manager(&track.path);
                            ui.close_menu();
                        }
                        if menu::item(ui, Some(Icon::Edit), "编辑标签…", None, false).clicked() {
                            if let Some(track) = self.library.track(id) {
                                self.dialog = Dialog::EditTags {
                                    edit: crate::tag_edit::TagEdit::from_track(track),
                                };
                                self.focus_pending = true;
                            }
                            ui.close_menu();
                        }

                        menu::separator(ui);
                        if menu::item(
                            ui,
                            Some(Icon::Trash),
                            "从音乐库移除…",
                            None,
                            true,
                        )
                        .clicked()
                        {
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

pub(crate) fn painter_eq(painter: &egui::Painter, center: Pos2, time: f32, playing: bool) {
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

pub(crate) fn progress_bar(ui: &mut Ui, fraction: f32, width: f32) {
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
pub(crate) enum TableMove {
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

pub(crate) fn table_target(
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

pub(crate) fn table_scroll_offset(index: usize, viewport: f32, row_height: f32) -> f32 {
    (index as f32 * row_height - (viewport - row_height) * 0.5).max(0.0)
}

