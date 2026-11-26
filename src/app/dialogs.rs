//! Modal dialogs: playlist naming, confirmations, folder management and About.

use std::path::PathBuf;

use eframe::egui;
use egui::{
    Align, Align2, Button, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Label, Layout,
    Margin, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2,
};

use super::{Dialog, RPlayerApp};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

const FORM_FIELD_WIDTH: f32 = 300.0;

/// (group, [(keys, action)]) - 21 entries, laid out in two columns per D10.
const SHORTCUT_GROUPS: &[(&str, &[(&str, &str)])] = &[
    (
        "全局",
        &[
            ("Ctrl K", "打开命令面板"),
            ("Ctrl F", "聚焦搜索"),
            ("Ctrl O", "添加音乐文件夹"),
            ("Ctrl N", "新建播放列表"),
            ("Ctrl ,", "打开设置"),
            ("Ctrl T", "切换 Paper / Nocturne"),
            ("F1", "打开快捷键帮助"),
        ],
    ),
    (
        "播放",
        &[
            ("Space", "播放 / 暂停（无焦点时）"),
            ("Ctrl E", "打开均衡器"),
            ("Ctrl L", "桌面歌词开关"),
            ("Ctrl M", "迷你播放器开关"),
        ],
    ),
    (
        "列表与选择",
        &[
            ("↑ / ↓", "移动焦点"),
            ("Home / End", "跳到首行 / 末行"),
            ("PgUp / PgDn", "翻页"),
            ("Ctrl A", "全选当前列表"),
            ("Ctrl ← / →", "上一首 / 下一首"),
            ("Delete", "移除选中项"),
        ],
    ),
    (
        "焦点",
        &[
            ("Tab", "下一个控件"),
            ("Shift Tab", "上一个控件"),
            ("Enter", "执行主操作"),
            ("Esc", "关闭浮层 / 取消"),
        ],
    ),
];

const SHORTCUT_COUNT: usize = 21;

fn shortcut_key(ui: &mut Ui, keys: &str) {
    Frame::NONE
        .fill(theme::c().card)
        .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new(keys)
                    .font(FontId::monospace(10.0))
                    .color(theme::c().muted),
            );
        });
}

fn shortcut_row(ui: &mut Ui, keys: &str, action: &str) {
    ui.horizontal(|ui| {
        ui.set_height(24.0);
        ui.allocate_ui_with_layout(
            Vec2::new(88.0, 24.0),
            Layout::left_to_right(Align::Center),
            |ui| shortcut_key(ui, keys),
        );
        ui.label(RichText::new(action).size(11.5).color(theme::c().text));
    });
}

fn shortcut_group(ui: &mut Ui, title: &str, rows: &[(&str, &str)], width: f32) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 0.0),
        Layout::top_down(Align::Min),
        |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(
                RichText::new(title)
                    .font(crate::ui::tokens::mono(8.5))
                    .color(theme::c().faint),
            );
            ui.add_space(4.0);
            for (keys, action) in rows {
                shortcut_row(ui, keys, action);
            }
            ui.add_space(10.0);
        },
    );
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
        if app.last_dialog == Dialog::None {
            app.dialog_focus_before = ctx.memory(|memory| memory.focused());
        }
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
    // Keep candidates must be resolved before pp.dialog is mutably borrowed.
    let dup_keeps: Vec<Option<u64>> = match &app.dialog {
        Dialog::Duplicates { groups } => groups
            .iter()
            .map(|group| duplicate_keep(app, &group.track_ids))
            .collect(),
        _ => Vec::new(),
    };

    let dialog_width = match &app.dialog {
        Dialog::Shortcuts => 780.0,
        Dialog::About => 340.0,
        _ => 460.0,
    };

    let modal = egui::Modal::new(Id::new("rplayer-modal"))
        .backdrop_color(crate::theme::scrim((175.0 * enter) as u8))
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
                    color: crate::theme::scrim((90.0 * enter) as u8),
                }),
        )
        .show(ctx, |ui| {
            ui.multiply_opacity(enter);
            ui.set_width(dialog_width);
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
                    let action = dialog_footer(ui, "创建", false, !name.trim().is_empty());
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
                    let action = dialog_footer(ui, "保存", false, !name.trim().is_empty());
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
                    let action = dialog_footer(ui, "删除", true, true);
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
                    let action = dialog_footer(ui, "移除", true, true);
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
                    let action = dialog_footer(ui, "保存标签", false, true);
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
                    let action = dialog_footer(ui, "批量保存", false, true);
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
                    ui.set_width(680.0);
                    let group_count = groups.len();
                    let file_count: usize = groups.iter().map(|g| g.track_ids.len()).sum();
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("DUPLICATES")
                                    .font(crate::ui::tokens::mono(8.5))
                                    .color(theme::c().faint),
                            );
                            ui.add_space(3.0);
                            ui.label(
                                RichText::new("重复歌曲")
                                    .font(crate::ui::tokens::serif(26.0))
                                    .color(theme::c().text),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(format!(
                                    "{group_count} 组 · {file_count} 个文件候选 · 同一标题、艺术家且时长相差不超过 2 秒"
                                ))
                                .size(10.0)
                                .color(theme::c().muted),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            if widgets::icon_button(ui, Icon::Close, 22.0, "关闭").clicked() {
                                outcome = Outcome::Close;
                            }
                        });
                    });
                    ui.add_space(10.0);

                    if groups.is_empty() {
                        ui.label(
                            RichText::new("没有发现重复歌曲")
                                .size(12.0)
                                .color(theme::c().muted),
                        );
                    } else {
                        ScrollArea::vertical()
                            .id_salt("duplicate-groups")
                            .max_height(196.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (gi, group) in groups.iter().enumerate() {
                                    let keep = dup_keeps.get(gi).copied().flatten();
                                    let Some(keep) = keep else { continue };
                                    let title = app
                                        .library
                                        .track(keep)
                                        .map(|t| t.title.clone())
                                        .unwrap_or_else(|| "重复歌曲".to_string());
                                    let artist = app
                                        .library
                                        .track(keep)
                                        .map(|t| t.artist.clone())
                                        .unwrap_or_default();
                                    let head = if artist.trim().is_empty() {
                                        title.clone()
                                    } else {
                                        format!("{title} — {artist}")
                                    };

                                    Frame::NONE
                                        .fill(theme::c().card)
                                        .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                                        .corner_radius(CornerRadius::same(10))
                                        .inner_margin(Margin::same(0))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            // Group head.
                                            let (head_rect, _) = ui.allocate_exact_size(
                                                Vec2::new(ui.available_width(), 34.0),
                                                Sense::hover(),
                                            );
                                            ui.painter().text(
                                                Pos2::new(head_rect.left() + 14.0, head_rect.center().y),
                                                Align2::LEFT_CENTER,
                                                &head,
                                                crate::ui::tokens::serif(12.5),
                                                theme::c().text,
                                            );
                                            ui.painter().text(
                                                Pos2::new(head_rect.right() - 14.0, head_rect.center().y),
                                                Align2::RIGHT_CENTER,
                                                format!("{} 个文件", group.track_ids.len()),
                                                crate::ui::tokens::mono(8.5),
                                                theme::c().faint,
                                            );
                                            ui.painter().hline(
                                                head_rect.x_range(),
                                                head_rect.bottom(),
                                                Stroke::new(1.0_f32, theme::c().border_soft),
                                            );

                                            let mut order = group.track_ids.clone();
                                            order.sort_by_key(|id| (*id != keep, *id));
                                            for id in &order {
                                                let Some(track) = app.library.track(*id) else {
                                                    continue;
                                                };
                                                let is_keep = *id == keep;
                                                let row_h = 42.0;
                                                let (row, resp) = ui.allocate_exact_size(
                                                    Vec2::new(ui.available_width(), row_h),
                                                    Sense::click(),
                                                );
                                                if is_keep {
                                                    ui.painter().rect_filled(
                                                        row,
                                                        CornerRadius::ZERO,
                                                        theme::accent_alpha(0.08),
                                                    );
                                                    ui.painter().rect_filled(
                                                        Rect::from_min_size(
                                                            row.left_top(),
                                                            Vec2::new(2.0, row.height()),
                                                        ),
                                                        CornerRadius::ZERO,
                                                        theme::c().accent,
                                                    );
                                                }
                                                let name = track
                                                    .path
                                                    .file_name()
                                                    .map(|n| n.to_string_lossy().to_string())
                                                    .unwrap_or_default();
                                                ui.painter().text(
                                                    Pos2::new(row.left() + 16.0, row.top() + 5.0),
                                                    Align2::LEFT_TOP,
                                                    &name,
                                                    crate::ui::tokens::mono(10.5),
                                                    theme::c().text,
                                                );
                                                ui.painter().text(
                                                    Pos2::new(row.left() + 16.0, row.bottom() - 5.0),
                                                    Align2::LEFT_BOTTOM,
                                                    format!(
                                                        "{} · {} · {}",
                                                        track.format_label(),
                                                        crate::model::format_duration(track.duration),
                                                        track
                                                            .path
                                                            .parent()
                                                            .map(|p| p.display().to_string())
                                                            .unwrap_or_default()
                                                    ),
                                                    crate::ui::tokens::mono(8.0),
                                                    theme::c().faint,
                                                );
                                                ui.painter().text(
                                                    Pos2::new(row.right() - 16.0, row.center().y),
                                                    Align2::RIGHT_CENTER,
                                                    if is_keep { "保留" } else { "忽略" },
                                                    crate::ui::tokens::mono(8.5),
                                                    if is_keep {
                                                        theme::c().accent
                                                    } else {
                                                        theme::c().faint
                                                    },
                                                );
                                                let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
                                                resp.widget_info(|| {
                                                    egui::WidgetInfo::labeled(
                                                        egui::WidgetType::Button,
                                                        is_keep,
                                                        format!(
                                                            "{}：{}",
                                                            if is_keep { "保留" } else { "忽略" },
                                                            track.path.display()
                                                        ),
                                                    )
                                                });
                                                if resp.clicked() {
                                                    outcome = Outcome::KeepDuplicate {
                                                        keep: *id,
                                                        group: group.track_ids.clone(),
                                                    };
                                                }
                                                ui.painter().hline(
                                                    row.x_range(),
                                                    row.bottom(),
                                                    Stroke::new(1.0_f32, theme::c().border_soft),
                                                );
                                            }

                                            // Card footer: group actions.
                                            ui.add_space(6.0);
                                            ui.horizontal(|ui| {
                                                ui.add_space(10.0);
                                                if widgets::secondary_button_sized(ui, "仅保留此首", 96.0)
                                                    .clicked()
                                                {
                                                    outcome = Outcome::KeepDuplicate {
                                                        keep,
                                                        group: group.track_ids.clone(),
                                                    };
                                                }
                                                if widgets::text_button(ui, "定位").clicked() {
                                                    if let Some(track) = app.library.track(keep) {
                                                        outcome = Outcome::RevealDuplicate(
                                                            track
                                                                .cue_source
                                                                .clone()
                                                                .unwrap_or_else(|| {
                                                                    track.path.clone()
                                                                }),
                                                        );
                                                    }
                                                }
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        ui.add_space(10.0);
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
                                            ui.add_space(8.0);
                                        });
                                    ui.add_space(6.0);
                                }
                            });
                    }

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("只删除重复的文件记录，其余移出曲库，不删除磁盘文件。")
                                .size(9.5)
                                .color(theme::c().faint),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if let Some(group) = groups.first() {
                                let extra = group.track_ids.len().saturating_sub(1);
                                if extra > 0 {
                                    let keep = dup_keeps.first().copied().flatten();
                                    if let Some(keep) = keep {
                                        if widgets::danger_button(
                                            ui,
                                            &format!("删除 {extra} 个重复项"),
                                        )
                                        .clicked()
                                        {
                                            outcome = Outcome::KeepDuplicate {
                                                keep,
                                                group: group.track_ids.clone(),
                                            };
                                        }
                                    }
                                }
                            }
                            ui.add_space(6.0);
                            if widgets::secondary_button_sized(ui, "取消", 64.0).clicked() {
                                outcome = Outcome::Close;
                            }
                        });
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
                    let action = dialog_footer(ui, "搜索候选", false, true);
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
                Dialog::Shortcuts => {
                    ui.set_width(780.0);
                    if dialog_header_close(
                        ui,
                        "KEYBOARD",
                        "键盘快捷键",
                        "焦点在控件上时，Space 激活控件，不触发全局播放。",
                    ) {
                        outcome = Outcome::Close;
                    }
                    ui.add_space(16.0);

                    let col_gap = 24.0;
                    let col_w = (ui.available_width() - col_gap) * 0.5;
                    let left = &SHORTCUT_GROUPS[0..2];
                    let right = &SHORTCUT_GROUPS[2..4];
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            for (title, rows) in left {
                                shortcut_group(ui, title, rows, col_w);
                            }
                            ui.add_space(4.0);
                            ui.add(
                                Label::new(
                                    RichText::new(
                                        "所有快捷键均可重绑，冲突时以后设置者生效。Tab 顺序遵循视觉顺序。",
                                    )
                                    .size(9.5)
                                    .color(theme::c().faint),
                                )
                                .wrap(),
                            );
                        });
                        ui.add_space(col_gap);
                        ui.vertical(|ui| {
                            for (title, rows) in right {
                                shortcut_group(ui, title, rows, col_w);
                            }
                            ui.add_space(4.0);
                            ui.add(
                                Label::new(
                                    RichText::new(
                                        "不区分大小写；输入框内不拦截文字键，冲突时以当前焦点为准。",
                                    )
                                    .size(9.5)
                                    .color(theme::c().faint),
                                )
                                .wrap(),
                            );
                        });
                    });

                    ui.add_space(10.0);
                    widget_divider(ui);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} SHORTCUTS · 2 COLUMNS",
                                SHORTCUT_COUNT
                            ))
                            .font(crate::ui::tokens::mono(8.5))
                            .color(theme::c().faint),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if widgets::primary_button_compact(ui, "关闭").clicked() {
                                outcome = Outcome::Close;
                            }
                        });
                    });
                }

                Dialog::About => {
                    ui.set_width(340.0);
                    if dialog_header_close(ui, "ABOUT", "关于 RPlayer", "") {
                        outcome = Outcome::Close;
                    }
                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                        icons::logo(ui.painter(), rect);
                        ui.add_space(12.0);
                        ui.vertical(|ui| {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new("RPlayer")
                                    .font(crate::ui::tokens::serif(20.0))
                                    .color(theme::c().text),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{} · 本地优先",
                                    env!("CARGO_PKG_VERSION")
                                ))
                                .font(crate::ui::tokens::mono(9.0))
                                .color(theme::c().faint),
                            );
                        });
                    });

                    ui.add_space(14.0);
                    about_row(ui, "数据目录", &crate::storage::data_dir().display().to_string());
                    about_row(ui, "开源许可", "egui · Symphonia · LRCLIB");
                    about_row(ui, "构建", "windows-gnu · release");

                    ui.add_space(12.0);
                    widget_divider(ui);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("NO LOGIN · NO UPLOAD")
                                .font(crate::ui::tokens::mono(8.5))
                                .color(theme::c().faint),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if widgets::primary_button_compact(ui, "关闭").clicked() {
                                outcome = Outcome::Close;
                            }
                        });
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
        Outcome::Close => {
            app.dialog = Dialog::None;
        }
        Outcome::None => {
            if should_close {
                app.dialog = Dialog::None;
            }
        }
    }

    if app.dialog == Dialog::None {
        app.restore_dialog_focus(ctx);
    }
}

/// Default keep candidate for a duplicate group: prefer lossless formats,
/// then the oldest library entry, then the shortest path.
fn duplicate_keep(app: &RPlayerApp, ids: &[u64]) -> Option<u64> {
    ids.iter().copied().min_by_key(|id| {
        let Some(track) = app.library.track(*id) else {
            return (2u8, u64::MAX, usize::MAX);
        };
        let ext = track
            .path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let lossless = if matches!(ext.as_str(), "flac" | "wav" | "aiff" | "alac") {
            0u8
        } else {
            1u8
        };
        (lossless, track.added_at, track.path.as_os_str().len())
    })
}

fn dialog_header_close(ui: &mut Ui, eyebrow: &str, title: &str, subtitle: &str) -> bool {
    let mut closed = false;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(eyebrow)
                    .font(crate::ui::tokens::mono(8.5))
                    .color(theme::c().faint),
            );
            ui.add_space(5.0);
            ui.label(
                RichText::new(title)
                    .font(crate::ui::tokens::serif(23.0))
                    .color(theme::c().text),
            );
            if !subtitle.is_empty() {
                ui.add_space(3.0);
                ui.add(Label::new(RichText::new(subtitle).size(9.5).color(theme::c().muted)).wrap());
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            if widgets::icon_button(ui, Icon::Close, 22.0, "关闭").clicked() {
                closed = true;
            }
        });
    });
    closed
}

fn widget_divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, theme::c().border_soft);
}

fn about_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(76.0, 22.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.label(
                    RichText::new(label)
                        .font(crate::ui::tokens::mono(8.5))
                        .color(theme::c().faint),
                );
            },
        );
        ui.add(
            Label::new(
                RichText::new(value)
                    .font(crate::ui::tokens::mono(9.5))
                    .color(theme::c().muted),
            )
            .truncate(),
        );
    });
    ui.add_space(2.0);
}

fn dialog_header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new("RPLAYER · DIALOG")
            .font(crate::ui::tokens::mono(8.5))
            .color(theme::c().faint),
    );
    ui.add_space(5.0);
    ui.label(
        RichText::new(title)
            .font(crate::ui::tokens::serif(23.0))
            .color(theme::c().text),
    );
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
fn dialog_footer(
    ui: &mut Ui,
    confirm_label: &str,
    danger: bool,
    enabled: bool,
) -> FooterAction {
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
            if enabled {
                if ui
                    .add(button)
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    action = FooterAction::Confirm;
                }
            } else {
                let disabled = Button::new(
                    RichText::new(confirm_label)
                        .size(13.0)
                        .color(theme::c().faint),
                )
                .fill(theme::c().card)
                .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                .corner_radius(10)
                .min_size(Vec2::new(84.0, 36.0));
                ui.add_enabled(false, disabled);
            }
            if widgets::secondary_button(ui, "取消").clicked() {
                action = FooterAction::Cancel;
            }
        });
    });
    action
}
