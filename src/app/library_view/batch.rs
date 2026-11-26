use super::*;

const BATCH_BAR_HEIGHT: f32 = 38.0;
/// At or above this count the bar collapses to `清空全部` / `退出批量`.
pub(crate) const BATCH_LARGE_THRESHOLD: usize = 20;

/// The three documented states of the multi-select bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BatchBarMode {
    /// Nothing selected: hint only, no destructive action.
    Empty,
    /// A handful of tracks: per-track actions are available.
    Small,
    /// A large selection: only `清空全部` / `退出批量`.
    Large,
}

pub(crate) fn batch_bar_mode(count: usize) -> BatchBarMode {
    if count == 0 {
        BatchBarMode::Empty
    } else if count >= BATCH_LARGE_THRESHOLD {
        BatchBarMode::Large
    } else {
        BatchBarMode::Small
    }
}

impl RPlayerApp {
    pub(crate) fn batch_bar(&mut self, ui: &mut Ui, ids: &[u64]) {
        let count = self.checked.len();
        let elsewhere = self
            .checked
            .iter()
            .filter(|id| !ids.contains(id))
            .count();
        let mode = batch_bar_mode(count);
        let large = mode == BatchBarMode::Large;

        Frame::NONE
            .fill(theme::accent_alpha(0.06))
            .stroke(Stroke::new(1.0_f32, theme::accent_alpha(0.16)))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(12, 0))
            .show(ui, |ui| {
                ui.set_height(BATCH_BAR_HEIGHT);
                ui.horizontal_centered(|ui| {
                    if count == 0 {
                        ui.label(
                            RichText::new("批量操作：Ctrl 点选 · Shift 范围选择")
                                .size(11.5)
                                .color(theme::c().muted),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!("已选 {count} 首"))
                                .size(11.5)
                                .color(theme::c().accent),
                        );
                        if elsewhere > 0 {
                            ui.label(
                                RichText::new(format!("· 另有 {elsewhere} 首不在当前视图"))
                                    .size(10.5)
                                    .color(theme::c().faint),
                            );
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::chip(ui, "退出批量", false).clicked() {
                            self.batch_mode = false;
                            self.checked.clear();
                            self.selection_anchor = None;
                        }
                        if count > 0 {
                            let clear_label = if large { "清空全部" } else { "清空" };
                            if widgets::chip(ui, clear_label, false).clicked() {
                                self.checked.clear();
                            }
                        }
                        if large {
                            return;
                        }
                        if !ids.is_empty() && widgets::chip(ui, "全选当前结果", false).clicked() {
                            self.set_all_checked(ids, true);
                        }
                        if count == 0 {
                            return;
                        }
                        if matches!(self.view, View::Playlist(_) | View::Favorites)
                            && widgets::chip(ui, "移除选中", false).clicked()
                        {
                            self.batch_remove_from_current();
                            return;
                        }
                        if widgets::chip(ui, "编辑标签…", false).clicked() {
                            let ids: Vec<u64> = self.checked.iter().copied().collect();
                            self.dialog = Dialog::BatchTags {
                                ids,
                                title: String::new(),
                                artist: String::new(),
                                album: String::new(),
                                genre: String::new(),
                                year: String::new(),
                            };
                        }
                        if widgets::chip(ui, "加入队列", false).clicked() {
                            let ids: Vec<u64> = self.checked.iter().copied().collect();
                            let added = self.append_to_queue(&ids);
                            if added > 0 {
                                self.toast(
                                    format!("已加入播放队列：{added} 首"),
                                    crate::app::ToastKind::Info,
                                );
                            }
                        }
                        self.batch_playlist_menu(ui);
                    });
                });
            });
    }

    /// `加入歌单` dropdown for the batch bar. Uses the same menu items as the
    /// per-track context menu so both entry points stay consistent.
    pub(crate) fn batch_playlist_menu(&mut self, ui: &mut Ui) {
        let playlists: Vec<(u64, String)> = self
            .library
            .playlists
            .iter()
            .map(|playlist| (playlist.id, playlist.name.clone()))
            .collect();
        ui.menu_button(
            RichText::new("加入歌单").size(11.5).color(theme::c().text),
            |ui| {
                menu::begin(ui);
                if playlists.is_empty() {
                    menu::hint(ui, "还没有播放列表");
                } else {
                    for (playlist_id, name) in &playlists {
                        if menu::item(ui, Some(Icon::List), name, None, false).clicked() {
                            self.batch_add_to_playlist(*playlist_id);
                            ui.close_menu();
                        }
                    }
                    menu::separator(ui);
                }
                if menu::item(ui, Some(Icon::Plus), "新建播放列表…", None, false).clicked() {
                    self.dialog = Dialog::NewPlaylist {
                        name: String::new(),
                    };
                    self.focus_pending = true;
                    ui.close_menu();
                }
            },
        );
    }

}
