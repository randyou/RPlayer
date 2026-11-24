//! Modal dialogs: playlist naming, confirmations, folder management and About.

use std::path::PathBuf;

use eframe::egui;
use egui::{
    Align, Button, Color32, CornerRadius, FontId, Frame, Id, Label, Layout, Margin, RichText,
    ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2,
};

use super::{Dialog, RPlayerApp};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

const FORM_FIELD_WIDTH: f32 = 300.0;

const SHORTCUTS: &[(&str, &str)] = &[
    ("Tab / Shift+Tab", "在控件之间移动焦点"),
    ("Space / Enter", "激活当前焦点控件"),
    ("Esc", "关闭弹窗或面板"),
    ("F1", "打开快捷键帮助"),
    ("Ctrl+K", "打开命令面板"),
    ("Ctrl+F", "聚焦搜索框"),
    ("Ctrl+O", "添加音乐文件夹"),
    ("Ctrl+N", "新建播放列表"),
    ("Ctrl+A", "全选当前列表"),
    ("Ctrl+,", "打开设置"),
    ("Ctrl+T", "切换浅色 / 深色主题"),
    ("Ctrl+E", "打开均衡器"),
    ("Ctrl+L", "切换桌面歌词"),
    ("Ctrl+M", "切换迷你播放器"),
    ("Space", "播放 / 暂停（无焦点时）"),
    ("← / →", "快退 / 快进 5 秒"),
    ("↑ / ↓", "列表内移动焦点；无焦点时调整音量"),
    ("Home / End", "跳到列表首行 / 末行"),
    ("PageUp / PageDown", "列表向上 / 向下翻页"),
    ("Ctrl+← / →", "上一首 / 下一首"),
    ("Delete", "移除选中的歌曲"),
];

fn shortcut_key(ui: &mut Ui, keys: &str) {
    Frame::NONE
        .fill(theme::c().card)
        .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(keys)
                    .font(FontId::monospace(10.5))
                    .color(theme::c().muted),
            );
        });
}

fn fade(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (f32::from(color.a()) * alpha.clamp(0.0, 1.0)) as u8,
    )
}

enum Outcome {
    None,
    Close,
    CreatePlaylist(String),
    RenamePlaylist(u64, String),
    DeletePlaylist(u64),
    RemoveTrack(u64),
    SaveTags(crate::tag_edit::TagEdit),
    BatchTags {
        ids: Vec<u64>,
        title: String,
        artist: String,
        album: String,
        genre: String,
        year: String,
    },
    AddFolder,
    RemoveFolder(PathBuf),
    Rescan,
    LyricsSearch {
        id: u64,
        title: String,
        artist: String,
        album: String,
    },
    UseLyrics {
        id: u64,
        text: String,
    },
    KeepDuplicate {
        keep: u64,
        group: Vec<u64>,
    },
    IgnoreDuplicates(Vec<u64>),
    RevealDuplicate(PathBuf),
}

pub(super) fn show(app: &mut RPlayerApp, ctx: &egui::Context) {
    if app.dialog == Dialog::None {
        return;
    }

    if app.dialog != app.last_dialog {
        app.dialog_anim_start = std::time::Instant::now();
        app.last_dialog = app.dialog.clone();
    }
    let enter = if crate::ui::motion::reduced() {
        1.0
    } else {
        (app.dialog_anim_start.elapsed().as_secs_f32() / crate::ui::motion::FAST).clamp(0.0, 1.0)
    };
    if enter < 1.0 {
        ctx.request_repaint();
    }

    let mut outcome = Outcome::None;

    let modal = egui::Modal::new(Id::new("rplayer-modal"))
        .backdrop_color(Color32::from_black_alpha((175.0 * enter) as u8))
        .frame(
            Frame::NONE
                .fill(fade(theme::c().panel, enter))
                .stroke(Stroke::new(
                    1.0_f32,
                    fade(theme::c().border_soft, enter),
                ))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(20))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 12],
                    blur: 30,
                    spread: 0,
                    color: Color32::from_black_alpha((90.0 * enter) as u8),
                }),
        )
        .show(ctx, |ui| {
            ui.multiply_opacity(enter);
            ui.set_width(460.0);
            match &mut app.dialog {
                Dialog::None => {}
                Dialog::NewPlaylist { name } => {
                    dialog_header(ui, "新建播放列表", "给它起一个名字，之后可以随时重命名。");
                    ui.add_space(14.0);
                    let response = name_field(ui, name, "例如：深夜开车");
                    if app.focus_pending {
                        response.request_focus();
                        app.focus_pending = false;
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("播放列表只保存歌曲引用，不会移动或复制文件。")
                            .size(10.5)
                            .color(theme::c().faint),
                    );
                    ui.add_space(18.0);

                    let entered =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let action = dialog_footer(ui, "创建", false);
                    if action == FooterAction::Cancel {
                        outcome = Outcome::Close;
                    } else if !name.trim().is_empty()
                        && (action == FooterAction::Confirm || entered)
                    {
                        outcome = Outcome::CreatePlaylist(name.clone());
                    }
                }

                Dialog::RenamePlaylist { id, name } => {
                    dialog_header(ui, "重命名播放列表", "换一个更贴切的名字。");
                    ui.add_space(14.0);
                    let response = name_field(ui, name, "播放列表名称");
                    if app.focus_pending {
                        response.request_focus();
                        app.focus_pending = false;
                    }
                    ui.add_space(20.0);
                    let entered =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let action = dialog_footer(ui, "保存", false);
                    if action == FooterAction::Cancel {
                        outcome = Outcome::Close;
                    } else if !name.trim().is_empty()
                        && (action == FooterAction::Confirm || entered)
                    {
                        outcome = Outcome::RenamePlaylist(*id, name.clone());
                    }
                }

                Dialog::ConfirmDeletePlaylist { id } => {
                    let id = *id;
                    let name = app
                        .library
                        .playlist(id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "这个播放列表".to_string());
                    dialog_header(ui, "删除播放列表？", "");
                    ui.add_space(6.0);
                    ui.add(
                        Label::new(
                            RichText::new(format!(
                                "「{name}」会被删除，但其中的歌曲仍会保留在音乐库。"
                            ))
                            .size(12.5)
                            .color(theme::c().muted),
                        )
                        .wrap(),
                    );
                    ui.add_space(20.0);
                    let action = dialog_footer(ui, "删除", true);
                    if action == FooterAction::Confirm {
                        outcome = Outcome::DeletePlaylist(id);
                    } else if action == FooterAction::Cancel {
                        outcome = Outcome::Close;
                    }
                }

                Dialog::ConfirmRemoveTrack { id } => {
                    let id = *id;
                    let title = app
                        .library
                        .track(id)
                        .map(|t| t.title.clone())
                        .unwrap_or_else(|| "这首歌".to_string());
                    dialog_header(ui, "从音乐库移除？", "");
                    ui.add_space(6.0);
                    ui.add(
                        Label::new(
                            RichText::new(format!(
                                "《{title}》会从 RPlayer 的音乐库中移除。\n磁盘上的音频文件不会被删除。"
                            ))
                            .size(12.5)
                            .color(theme::c().muted),
                        )
                        .wrap(),
                    );
                    ui.add_space(20.0);
                    let action = dialog_footer(ui, "移除", true);
                    if action == FooterAction::Confirm {
                        outcome = Outcome::RemoveTrack(id);
                    } else if action == FooterAction::Cancel {
                        outcome = Outcome::Close;
                    }
                }

                Dialog::EditTags { edit } => {
                    dialog_header(ui, "编辑标签", "写入音频文件前会记录原始字段。");
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(edit.path.to_string_lossy())
                            .size(9.5)
                            .color(theme::c().faint),
                    );
                    ui.add_space(10.0);
                    tag_field(ui, "标题", &mut edit.title);
                    tag_field(ui, "艺术家", &mut edit.artist);
                    tag_field(ui, "专辑", &mut edit.album);
                    tag_field(ui, "流派", &mut edit.genre);
                    tag_field(ui, "年份", &mut edit.year);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("保存后立即重新读取文件标签；失败时原字段记录在 tag_backups.jsonl。")
                            .size(10.0)
                            .color(theme::c().faint),
                    );
                    ui.add_space(12.0);
                    let action = dialog_footer(ui, "保存标签", false);
                    if action == FooterAction::Cancel {
                        outcome = Outcome::Close;
                    } else if action == FooterAction::Confirm && !edit.title.trim().is_empty() {
                        outcome = Outcome::SaveTags(edit.clone());
                    }
                }

                Dialog::BatchTags {
                    ids,
                    title,
                    artist,
                    album,
                    genre,
                    year,
                } => {
                    dialog_header(
                        ui,
                        "批量编辑标签",
                        "只写入填写了内容的字段；每首歌写入前都会备份原字段。",
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!("将应用到 {} 首歌曲", ids.len()))
                            .size(11.0)
                            .color(theme::c().accent),
                    );
                    ui.add_space(10.0);
                    tag_field(ui, "标题", title);
                    tag_field(ui, "艺术家", artist);
                    tag_field(ui, "专辑", album);
                    tag_field(ui, "流派", genre);
                    tag_field(ui, "年份", year);
                    ui.add_space(12.0);
                    let action = dialog_footer(ui, "批量保存", false);
                    if action == FooterAction::Cancel {
                        outcome = Outcome::Close;
                    } else if action == FooterAction::Confirm {
                        outcome = Outcome::BatchTags {
                            ids: ids.clone(),
                            title: title.clone(),
                            artist: artist.clone(),
                            album: album.clone(),
                            genre: genre.clone(),
                            year: year.clone(),
                        };
                    }
                }

                Dialog::Duplicates { groups } => {
                    ui.set_width(560.0);
                    dialog_header(
                        ui,
                        "重复歌曲",
                        "同名、同艺术家且时长相差不超过 2 秒的歌曲。移除只删曲库记录，不删除磁盘文件。",
                    );
                    ui.add_space(12.0);
                    if groups.is_empty() {
                        ui.label(
                            RichText::new("没有发现重复歌曲")
                                .size(12.0)
                                .color(theme::c().muted),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!("找到 {} 组重复歌曲", groups.len()))
                                .size(10.5)
                                .color(theme::c().muted),
                        );
                        ui.add_space(6.0);
                        ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                            for group in groups.iter() {
                                let title = group
                                    .track_ids
                                    .first()
                                    .and_then(|id| app.library.track(*id))
                                    .map(|track| track.title.clone())
                                    .unwrap_or_else(|| "重复歌曲".to_string());
                                Frame::NONE
                                    .fill(theme::c().card)
                                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                                    .corner_radius(CornerRadius::same(10))
                                    .inner_margin(Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(&title)
                                                    .size(12.0)
                                                    .strong()
                                                    .color(theme::c().text),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} 首",
                                                    group.track_ids.len()
                                                ))
                                                .size(10.0)
                                                .color(theme::c().muted),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    if widgets::text_button(ui, "忽略本组")
                                                        .clicked()
                                                    {
                                                        outcome = Outcome::IgnoreDuplicates(
                                                            group.track_ids.clone(),
                                                        );
                                                    }
                                                },
                                            );
                                        });
                                        ui.add_space(4.0);
                                        for id in &group.track_ids {
                                            let Some(track) = app.library.track(*id) else {
                                                continue;
                                            };
                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} · {} · {} · {}",
                                                            track.artist,
                                                            track.album,
                                                            track.format_label(),
                                                            crate::model::format_duration(
                                                                track.duration
                                                            )
                                                        ))
                                                        .size(10.0)
                                                        .color(theme::c().muted),
                                                    );
                                                    ui.add(
                                                        Label::new(
                                                            RichText::new(
                                                                track.path.display().to_string(),
                                                            )
                                                            .size(9.0)
                                                            .color(theme::c().faint),
                                                        )
                                                        .truncate(),
                                                    );
                                                });
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        let keep = widgets::secondary_button(
                                                            ui,
                                                            "仅保留此首",
                                                        );
                                                        keep.widget_info(|| {
                                                            egui::WidgetInfo::labeled(
                                                                egui::WidgetType::Button,
                                                                true,
                                                                format!(
                                                                    "仅保留：{}（{}）",
                                                                    track.title,
                                                                    track.path.display()
                                                                ),
                                                            )
                                                        });
                                                        if keep.clicked() {
                                                            outcome = Outcome::KeepDuplicate {
                                                                keep: *id,
                                                                group: group.track_ids.clone(),
                                                            };
                                                        }
                                                        let locate =
                                                            widgets::text_button(ui, "定位");
                                                        locate.widget_info(|| {
                                                            egui::WidgetInfo::labeled(
                                                                egui::WidgetType::Button,
                                                                true,
                                                                format!(
                                                                    "在文件夹中显示：{}",
                                                                    track.path.display()
                                                                ),
                                                            )
                                                        });
                                                        if locate.clicked() {
                                                            outcome = Outcome::RevealDuplicate(
                                                                track
                                                                    .cue_source
                                                                    .clone()
                                                                    .unwrap_or_else(|| {
                                                                        track.path.clone()
                                                                    }),
                                                            );
                                                        }
                                                    },
                                                );
                                            });
                                            ui.add_space(2.0);
                                        }
                                    });
                                ui.add_space(5.0);
                            }
                        });
                    }
                    ui.add_space(12.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::primary_button(ui, "关闭").clicked() {
                            outcome = Outcome::Close;
                        }
                    });
                }
                Dialog::LyricsMatch {
                    id,
                    title,
                    artist,
                    album,
                    candidates,
                    searching,
                    error,
                } => {
                    dialog_header(
                        ui,
                        "重新匹配歌词",
                        "修改搜索条件，搜索候选后选择正确的一版。",
                    );
                    ui.add_space(12.0);
                    tag_field(ui, "歌名", title);
                    tag_field(ui, "艺术家", artist);
                    tag_field(ui, "专辑", album);
                    ui.add_space(6.0);
                    if *searching {
                        ui.label(
                            RichText::new("正在搜索候选…")
                                .size(10.5)
                                .color(theme::c().accent),
                        );
                    } else if let Some(message) = error {
                        ui.label(
                            RichText::new(message.as_str())
                                .size(10.5)
                                .color(theme::c().danger),
                        );
                    } else {
                        ui.label(
                            RichText::new("留空艺术家/专辑可扩大搜索范围。")
                                .size(10.5)
                                .color(theme::c().faint),
                        );
                    }
                    if !candidates.is_empty() {
                        ui.add_space(8.0);
                        ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                            for candidate in candidates.iter() {
                                Frame::NONE
                                    .fill(theme::c().card)
                                    .corner_radius(CornerRadius::same(8))
                                    .inner_margin(Margin::symmetric(8, 6))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    RichText::new(&candidate.title)
                                                        .size(12.0)
                                                        .strong()
                                                        .color(theme::c().text),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} · {} · {:.0}s",
                                                        candidate.artist,
                                                        candidate.album,
                                                        candidate.duration
                                                    ))
                                                    .size(10.0)
                                                    .color(theme::c().muted),
                                                );
                                            });
                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                if widgets::text_button(ui, "使用").clicked() {
                                                    outcome = Outcome::UseLyrics {
                                                        id: *id,
                                                        text: candidate.text.clone(),
                                                    };
                                                }
                                                if candidate.synced {
                                                    ui.label(
                                                        RichText::new("时间轴")
                                                            .size(9.0)
                                                            .color(theme::c().accent),
                                                    );
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        });
                    }
                    ui.add_space(10.0);
                    let action = dialog_footer(ui, "搜索候选", false);
                    if action == FooterAction::Cancel {
                        outcome = Outcome::Close;
                    } else if action == FooterAction::Confirm && !title.trim().is_empty() {
                        outcome = Outcome::LyricsSearch {
                            id: *id,
                            title: title.clone(),
                            artist: artist.clone(),
                            album: album.clone(),
                        };
                    }
                }
                Dialog::Folders => {
                    dialog_header(
                        ui,
                        "音乐文件夹",
                        "RPlayer 会递归扫描这些文件夹中的音频文件。",
                    );
                    ui.add_space(12.0);

                    let folders = app.library.folders.clone();
                    if folders.is_empty() {
                        Frame::NONE
                            .fill(theme::c().card)
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(Margin::symmetric(14, 18))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new("还没有添加音乐文件夹")
                                            .size(12.0)
                                            .color(theme::c().muted),
                                    );
                                    ui.label(
                                        RichText::new("支持 MP3 / FLAC / WAV / OGG / M4A / AAC")
                                            .size(10.5)
                                            .color(theme::c().faint),
                                    );
                                });
                            });
                    } else {
                        ScrollArea::vertical()
                            .id_salt("rplayer-folders")
                            .max_height(224.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for folder in &folders {
                                    ui.horizontal(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            Vec2::splat(17.0),
                                            Sense::hover(),
                                        );
                                        icons::paint(
                                            ui.painter(),
                                            rect,
                                            Icon::Folder,
                                            theme::c().muted,
                                        );
                                        let available = (ui.available_width() - 34.0).max(60.0);
                                        ui.allocate_ui_with_layout(
                                            Vec2::new(available, 20.0),
                                            Layout::left_to_right(Align::Center),
                                            |ui| {
                                                ui.add(
                                                    Label::new(
                                                        RichText::new(
                                                            folder.display().to_string(),
                                                        )
                                                        .size(11.0)
                                                        .color(theme::c().muted),
                                                    )
                                                    .truncate(),
                                                );
                                            },
                                        );
                                        if widgets::icon_button(
                                            ui,
                                            Icon::Trash,
                                            26.0,
                                            "移除这个文件夹",
                                        )
                                        .clicked()
                                        {
                                            outcome = Outcome::RemoveFolder(folder.clone());
                                        }
                                    });
                                    ui.add_space(2.0);
                                }
                            });
                    }

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if widgets::secondary_button(ui, "添加文件夹").clicked() {
                            outcome = Outcome::AddFolder;
                        }
                        if !folders.is_empty()
                            && widgets::secondary_button(ui, "重新扫描").clicked()
                        {
                            outcome = Outcome::Rescan;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if widgets::secondary_button(ui, "关闭").clicked() {
                                outcome = Outcome::Close;
                            }
                        });
                    });
                }

                Dialog::Shortcuts => {
                    ui.set_width(700.0);
                    dialog_header(
                        ui,
                        "键盘快捷键",
                        "主要操作都可以只用键盘完成，焦点所在控件会显示绿色描边。",
                    );
                    ui.add_space(14.0);
                    let half = SHORTCUTS.len().div_ceil(2);
                    egui::Grid::new("shortcut-grid")
                        .num_columns(4)
                        .spacing([18.0, 8.0])
                        .show(ui, |ui| {
                            for index in 0..half {
                                shortcut_key(ui, SHORTCUTS[index].0);
                                ui.label(
                                    RichText::new(SHORTCUTS[index].1)
                                        .size(11.5)
                                        .color(theme::c().text),
                                );
                                if let Some((keys, action)) = SHORTCUTS.get(index + half) {
                                    shortcut_key(ui, keys);
                                    ui.label(
                                        RichText::new(*action)
                                            .size(11.5)
                                            .color(theme::c().text),
                                    );
                                } else {
                                    ui.label("");
                                    ui.label("");
                                }
                                ui.end_row();
                            }
                        });
                    ui.add_space(16.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::primary_button(ui, "关闭").clicked() {
                            outcome = Outcome::Close;
                        }
                    });
                }

                Dialog::About => {
                    let top = ((ui.available_height() - 360.0) * 0.5).max(0.0);
                    ui.add_space(top);
                    ui.vertical_centered(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(64.0), Sense::hover());
                        icons::logo(ui.painter(), rect);
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("RPlayer")
                                .size(21.0)
                                .strong()
                                .color(theme::c().text),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} · 本地音乐播放器",
                                env!("CARGO_PKG_VERSION")
                            ))
                            .size(11.0)
                            .color(theme::c().muted),
                        );
                        ui.add_space(12.0);
                        ui.add(
                            Label::new(
                                RichText::new(
                                    "纯粹、快速的桌面音乐播放器。\n所有音乐与播放列表都保存在你的电脑上。",
                                )
                                .size(11.5)
                                .color(theme::c().muted),
                            )
                            .wrap(),
                        );
                    });

                    ui.add_space(14.0);
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            let width = ui.available_width();
                            ui.add_space(((width - 290.0) * 0.5).max(0.0));
                            widgets::chip(ui, "Rust", false);
                            widgets::chip(ui, "egui", false);
                            widgets::chip(ui, "rodio", false);
                            widgets::chip(ui, "lofty", false);
                        });

                        ui.add_space(14.0);
                        ui.label(
                            RichText::new("数据目录")
                                .size(10.0)
                                .color(theme::c().faint),
                        );
                        ui.add(
                            Label::new(
                                RichText::new(crate::storage::data_dir().display().to_string())
                                    .font(FontId::monospace(10.5))
                                    .color(theme::c().muted),
                            )
                            .wrap(),
                        );

                        ui.add_space(18.0);
                        if widgets::primary_button(ui, "好的").clicked() {
                            outcome = Outcome::Close;
                        }
                    });
                }
            }
        });

    let should_close = modal.should_close();

    match outcome {
        Outcome::CreatePlaylist(name) => {
            app.create_playlist(name);
            app.dialog = Dialog::None;
        }
        Outcome::RenamePlaylist(id, name) => {
            app.rename_playlist(id, name);
            app.dialog = Dialog::None;
        }
        Outcome::DeletePlaylist(id) => {
            app.delete_playlist(id);
            app.dialog = Dialog::None;
        }
        Outcome::RemoveTrack(id) => {
            app.remove_track(id);
            app.dialog = Dialog::None;
        }
        Outcome::SaveTags(edit) => {
            match crate::tag_edit::save(&edit) {
                Ok(track) => {
                    app.replace_track(track);
                    app.toast("标签已保存", super::ToastKind::Success);
                }
                Err(err) => {
                    app.toast(format!("写入标签失败：{err:#}"), super::ToastKind::Error);
                }
            }
            app.dialog = Dialog::None;
        }
        Outcome::LyricsSearch { id, title, artist, album } => {
            app.start_lyrics_search(id, title, artist, album);
        }
        Outcome::UseLyrics { id, text } => {
            app.apply_lyrics_candidate(id, &text);
        }
        Outcome::KeepDuplicate { keep, group } => {
            app.keep_duplicate_track(keep, &group);
        }
        Outcome::IgnoreDuplicates(group) => {
            app.ignore_duplicate_group(&group);
        }
        Outcome::RevealDuplicate(path) => {
            app.reveal_path(&path);
        }
        Outcome::BatchTags {
            ids,
            title,
            artist,
            album,
            genre,
            year,
        } => {
            let mut ok = 0usize;
            let mut failed = 0usize;
            let mut saved = Vec::new();
            for id in ids {
                let Some(track) = app.library.track(id).cloned() else {
                    continue;
                };
                let mut edit = crate::tag_edit::TagEdit::from_track(&track);
                if !title.trim().is_empty() {
                    edit.title = title.clone();
                }
                if !artist.trim().is_empty() {
                    edit.artist = artist.clone();
                }
                if !album.trim().is_empty() {
                    edit.album = album.clone();
                }
                if !genre.trim().is_empty() {
                    edit.genre = genre.clone();
                }
                if !year.trim().is_empty() {
                    edit.year = year.clone();
                }
                match crate::tag_edit::save(&edit) {
                    Ok(updated) => {
                        app.replace_track(updated);
                        saved.push(edit);
                        ok += 1;
                    }
                    Err(_) => failed += 1,
                }
            }
            if !saved.is_empty() {
                app.library.last_tag_batch = Some(saved);
                app.persist();
            }
            app.toast(
                format!("批量标签：成功 {ok} 首，失败 {failed} 首"),
                if failed == 0 {
                    super::ToastKind::Success
                } else {
                    super::ToastKind::Error
                },
            );
            app.dialog = Dialog::None;
        }
        Outcome::AddFolder => {
            app.open_folder_picker();
        }
        Outcome::RemoveFolder(folder) => {
            app.remove_folder(&folder);
        }
        Outcome::Rescan => {
            app.start_scan();
            app.dialog = Dialog::None;
        }
        Outcome::Close => {
            app.dialog = Dialog::None;
        }
        Outcome::None => {
            if should_close {
                app.dialog = Dialog::None;
            }
        }
    }
}

fn dialog_header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(19.0).strong().color(theme::c().text));
    if !subtitle.is_empty() {
        ui.add_space(3.0);
        ui.add(Label::new(RichText::new(subtitle).size(11.5).color(theme::c().muted)).wrap());
    }
}

fn tag_field(ui: &mut Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(44.0, 34.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(11.0).color(theme::c().muted));
            },
        );
        let width = FORM_FIELD_WIDTH.min(ui.available_width().max(120.0));
        ui.add_space((ui.available_width() - width).max(0.0));
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), theme::c().card);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(8),
            Stroke::new(1.0_f32, theme::c().border),
            egui::StrokeKind::Inside,
        );
        let edit_rect = rect.shrink2(Vec2::new(8.0, 6.0));
        let response = ui.put(
            edit_rect,
            TextEdit::singleline(value)
                .frame(false)
                .text_color(theme::c().text)
                .desired_width(edit_rect.width())
                .clip_text(true),
        );
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                CornerRadius::same(8),
                Stroke::new(2.0_f32, theme::accent_alpha(0.75)),
                egui::StrokeKind::Inside,
            );
        }
    });
    ui.add_space(3.0);
}

fn name_field(ui: &mut Ui, name: &mut String, hint: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let width = FORM_FIELD_WIDTH.min(ui.available_width().max(120.0));
        ui.add_space((ui.available_width() - width).max(0.0));
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(10), theme::c().card);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(1.0_f32, theme::c().border),
            egui::StrokeKind::Inside,
        );
        let edit_rect = rect.shrink2(Vec2::new(10.0, 6.0));
        let response = ui.put(
            edit_rect,
            TextEdit::singleline(name)
                .hint_text(RichText::new(hint).size(12.5).color(theme::c().faint))
                .font(FontId::proportional(13.5))
                .text_color(theme::c().text)
                .frame(false)
                .desired_width(edit_rect.width())
                .clip_text(true)
                .margin(Margin::symmetric(2, 2)),
        );
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                CornerRadius::same(10),
                Stroke::new(2.0_f32, theme::accent_alpha(0.75)),
                egui::StrokeKind::Inside,
            );
        }
        response
    })
    .inner
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FooterAction {
    None,
    Confirm,
    Cancel,
}

/// Returns the primary or cancel action clicked in the dialog footer.
fn dialog_footer(ui: &mut Ui, confirm_label: &str, danger: bool) -> FooterAction {
    let mut action = FooterAction::None;
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let button = if danger {
                Button::new(
                    RichText::new(confirm_label)
                        .size(13.0)
                        .strong()
                        .color(theme::c().on_accent),
                )
                .fill(theme::c().danger)
                .corner_radius(10)
                .min_size(Vec2::new(84.0, 36.0))
            } else {
                Button::new(
                    RichText::new(confirm_label)
                        .size(13.0)
                        .strong()
                        .color(theme::c().on_accent),
                )
                .fill(theme::c().accent)
                .corner_radius(10)
                .min_size(Vec2::new(84.0, 36.0))
            };
            if ui
                .add(button)
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                action = FooterAction::Confirm;
            }
            if widgets::secondary_button(ui, "取消").clicked() {
                action = FooterAction::Cancel;
            }
        });
    });
    action
}
