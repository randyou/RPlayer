//! Local lyric editor: fix text and per-line timestamps.

use eframe::egui;

use super::{RPlayerApp, ToastKind};
use crate::theme;
use crate::ui::icons::Icon;
use crate::ui::widgets;

pub(super) fn open(app: &mut RPlayerApp) {
    let Some(id) = app.current else { return; };
    let Some(track) = app.library.track(id) else { return; };
    let lyrics = app.lyrics.clone().unwrap_or_default();
    app.lyrics_editor = Some(crate::lyrics::LyricsEditor::from_lyrics(
        id,
        track.title.clone(),
        &lyrics,
    ));
}

pub(super) fn show(app: &mut RPlayerApp, ctx: &egui::Context) {
    let Some(mut editor) = app.lyrics_editor.take() else {
        return;
    };
    let mut open = true;
    let mut save = false;
    let mut cancel = false;
    let position = app.audio.as_ref().map(|audio| audio.position()).unwrap_or(0.0);
    egui::Window::new("编辑歌词")
        .open(&mut open)
        .default_width(660.0)
        .max_height(640.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(&editor.title)
                    .size(11.0)
                    .color(theme::c().muted),
            );
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .max_height(460.0)
                .show(ui, |ui| {
                    let mut remove = None;
                    for (index, row) in editor.rows.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut row.time)
                                    .desired_width(78.0)
                                    .hint_text("[00:00.00]"),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut row.text)
                                    .desired_width(450.0),
                            );
                            if widgets::icon_button(ui, Icon::Clock, 22.0, "使用当前播放时间")
                                .clicked()
                            {
                                row.time = crate::lyrics::format_timestamp(position);
                            }
                            if widgets::icon_button(ui, Icon::Trash, 22.0, "删除这一句")
                                .clicked()
                            {
                                remove = Some(index);
                            }
                        });
                    }
                    if let Some(index) = remove {
                        editor.rows.remove(index);
                    }
                });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if widgets::secondary_button(ui, "新增一句").clicked() {
                    editor.rows.push(crate::lyrics::EditorRow {
                        time: crate::lyrics::format_timestamp(position),
                        text: String::new(),
                    });
                }
                ui.label(
                    egui::RichText::new("时间格式 mm:ss.xx；留空表示纯文本歌词")
                        .size(10.0)
                        .color(theme::c().faint),
                );
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if widgets::primary_button(ui, "保存歌词").clicked() {
                    save = true;
                }
                if widgets::secondary_button(ui, "取消").clicked() {
                    cancel = true;
                }
            });
        });
    if save {
        if let Some(lyrics) = editor.build() {
            match crate::lyrics::store_custom(editor.track_id, &lyrics) {
                Ok(()) => {
                    app.lyrics = Some(lyrics);
                    app.lyrics_source = Some("编辑".to_string());
                    app.toast("歌词已保存到本地编辑缓存", ToastKind::Success);
                }
                Err(error) => {
                    app.toast(format!("保存歌词失败：{error}"), ToastKind::Error);
                }
            }
        } else {
            app.toast("没有可保存的歌词行，或时间格式无效", ToastKind::Error);
            app.lyrics_editor = Some(editor);
        }
    } else if open && !cancel {
        app.lyrics_editor = Some(editor);
    }
}
