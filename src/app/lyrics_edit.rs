//! Local lyric editor: fix text and per-line timestamps.
//!
//! Layout follows design D7: 820×430, 36px rows, a highlighted row for the
//! current playback position, and one fixed action band at the bottom.

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout, Margin, Pos2, Rect,
    RichText, Sense, Stroke, Ui, Vec2,
};

use super::{RPlayerApp, ToastKind};
use crate::lyrics::{format_timestamp_plain, LyricsEditor};
use crate::theme;
use crate::ui::icons::Icon;
use crate::ui::{tokens, widgets};

const WINDOW_SIZE: Vec2 = Vec2::new(820.0, 430.0);
const FRAME_MARGIN: f32 = 16.0;
/// Content box: `fixed_size` in egui describes the content, not the frame.
const CONTENT_SIZE: Vec2 = Vec2::new(
    WINDOW_SIZE.x - FRAME_MARGIN * 2.0,
    WINDOW_SIZE.y - FRAME_MARGIN * 2.0,
);
const ROW_HEIGHT: f32 = 34.0;
const TIME_WIDTH: f32 = 62.0;
const LYRIC_LEFT: f32 = 96.0;
const ICON_SIZE: f32 = 22.0;
/// Height reserved for the divider and the 32px action band.
const FOOTER_HEIGHT: f32 = 39.0;
const ADD_ROW_HEIGHT: f32 = 30.0;

enum RowAction {
    Seek(f64),
    Remove,
}

pub(super) fn open(app: &mut RPlayerApp) {
    let Some(id) = app.current else { return; };
    let Some(track) = app.library.track(id) else { return; };
    let lyrics = app.lyrics.clone().unwrap_or_default();
    let artist = if track.artist.trim().is_empty() {
        "未知艺术家".to_string()
    } else {
        track.artist.clone()
    };
    app.lyrics_editor = Some(LyricsEditor::from_lyrics(
        id,
        track.title.clone(),
        artist,
        &lyrics,
    ));
}

pub(super) fn show(app: &mut RPlayerApp, ctx: &egui::Context) {
    let Some(mut editor) = app.lyrics_editor.take() else {
        return;
    };
    let mut save = false;
    let mut close = false;
    let mut seek_to = None;
    let position = app.audio.as_ref().map(|audio| audio.position()).unwrap_or(0.0);
    let active_row = editor.current_index(position);
    let can_save = !editor.rows.is_empty() && !editor.any_invalid();

    let window = egui::Window::new("编辑歌词")
        .id(Id::new("rplayer-lyrics-editor"))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .fixed_size(CONTENT_SIZE)
        .frame(
            Frame::NONE
                .fill(theme::c().panel)
                .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                .corner_radius(CornerRadius::same(tokens::RADIUS_CARD))
                .inner_margin(Margin::same(FRAME_MARGIN as i8))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 18],
                    blur: 44,
                    spread: 0,
                    color: theme::c().shadow,
                }),
        );

    window.show(ctx, |ui| {
        ui.set_width(CONTENT_SIZE.x);
        header(ui, &editor, &mut close);
        ui.add_space(2.0);
        table_header(ui);
        ui.add_space(2.0);

        let mut remove = None;
        let mut add = false;
        let list_height =
            (ui.available_height() - ADD_ROW_HEIGHT - 4.0 - FOOTER_HEIGHT).max(ROW_HEIGHT * 2.0);
        egui::ScrollArea::vertical()
            .id_salt("lyrics-editor-rows")
            .max_height(list_height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for index in 0..editor.rows.len() {
                    let active = active_row == Some(index);
                    if let Some(action) = row(ui, &mut editor, index, active) {
                        match action {
                            RowAction::Seek(time) => seek_to = Some(time),
                            RowAction::Remove => remove = Some(index),
                        }
                    }
                }
            });
        if let Some(index) = remove {
            editor.rows.remove(index);
        }
        if add_row_button(ui) {
            add = true;
        }
        if add {
            let time = if editor.plain {
                String::new()
            } else {
                format_timestamp_plain(position)
            };
            editor.rows.push(crate::lyrics::EditorRow {
                time,
                text: String::new(),
            });
        }

        ui.add_space(6.0);
        footer(ui, can_save, &mut save, &mut close);
    });

    if let Some(time) = seek_to {
        let was_playing = app.audio.as_ref().is_some_and(|audio| audio.is_playing());
        app.seek_to(time);
        if !was_playing {
            app.toggle_play(ctx);
        }
    }
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
                    app.lyrics_editor = Some(editor);
                }
            }
        } else {
            app.toast("没有可保存的歌词行，或时间格式无效", ToastKind::Error);
            app.lyrics_editor = Some(editor);
        }
    } else if !close {
        app.lyrics_editor = Some(editor);
    }
}

fn header(ui: &mut Ui, editor: &LyricsEditor, close: &mut bool) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(
                RichText::new("LYRICS EDITOR")
                    .font(tokens::mono(8.5))
                    .color(theme::c().faint),
            );
            ui.label(
                RichText::new("编辑歌词")
                    .font(tokens::serif(28.0))
                    .color(theme::c().text),
            );
            ui.label(
                RichText::new(format!("{} · {} · 本地编辑", editor.title, editor.artist))
                    .size(9.5)
                    .color(theme::c().muted),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
            if widgets::icon_button(ui, Icon::Close, 24.0, "关闭歌词编辑器").clicked() {
                *close = true;
            }
            if editor.is_dirty() {
                unsaved_chip(ui);
            }
        });
    });
}

fn unsaved_chip(ui: &mut Ui) {
    Frame::NONE
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, theme::c().danger.gamma_multiply(0.65)))
        .corner_radius(CornerRadius::same(99))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new("未保存")
                    .size(9.5)
                    .color(theme::c().danger),
            );
        });
}

fn table_header(ui: &mut Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::hover());
    let painter = ui.painter();
    for (x, text) in [(0.0, "时间"), (LYRIC_LEFT - 12.0, "歌词")] {
        painter.text(
            Pos2::new(rect.left() + 12.0 + x, rect.center().y),
            Align2::LEFT_CENTER,
            text,
            tokens::mono(8.0),
            theme::c().faint,
        );
    }
    widgets::divider(ui);
}

fn row(
    ui: &mut Ui,
    editor: &mut LyricsEditor,
    index: usize,
    active: bool,
) -> Option<RowAction> {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::hover());
    let painter = ui.painter();
    if active {
        painter.rect_filled(rect, CornerRadius::same(6), theme::accent_alpha(0.10));
        painter.rect_filled(
            Rect::from_min_size(rect.left_top(), Vec2::new(2.0, rect.height())),
            CornerRadius::same(1),
            theme::c().accent,
        );
    }
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(rect.left(), rect.bottom() - 1.0),
            Vec2::new(rect.width(), 1.0),
        ),
        0,
        theme::divider(),
    );

    let invalid = editor.invalid_row(index);
    let mut action = None;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(12.0, 0.0)))
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            let item = &mut editor.rows[index];
            let time_color = if invalid {
                theme::c().danger
            } else if active {
                theme::c().accent
            } else {
                theme::c().muted
            };
            ui.add(
                egui::TextEdit::singleline(&mut item.time)
                    .id(Id::new(("lyrics-editor-time", index)))
                    .frame(false)
                    .font(tokens::mono(10.5))
                    .text_color(time_color)
                    .desired_width(TIME_WIDTH)
                    .hint_text("00:00.00")
                    .margin(Margin::symmetric(0, 7)),
            )
            .on_hover_text("时间码 mm:ss.xx；留空表示纯文本行");
            ui.add_space(LYRIC_LEFT - 12.0 - TIME_WIDTH);
            let icons_width = ICON_SIZE * 2.0 + 14.0;
            let lyric_width = (ui.available_width() - icons_width).max(80.0);
            ui.add(
                egui::TextEdit::singleline(&mut item.text)
                    .id(Id::new(("lyrics-editor-text", index)))
                    .frame(false)
                    .font(FontId::proportional(12.5))
                    .text_color(theme::c().text)
                    .desired_width(lyric_width)
                    .hint_text("这一句的歌词")
                    .margin(Margin::symmetric(0, 7)),
            )
            .on_hover_text("这一句的歌词");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, Icon::Trash, ICON_SIZE, "删除这一句").clicked() {
                    action = Some(RowAction::Remove);
                }
                let time = crate::lyrics::parse_time(&item.time);
                let response =
                    widgets::icon_button(ui, Icon::Play, ICON_SIZE, "从这一句开始播放");
                if response.clicked() {
                    action = Some(RowAction::Seek(time.unwrap_or(0.0)));
                }
                ui.add_space(2.0);
            });
        },
    );
    action
}

fn add_row_button(ui: &mut Ui) -> bool {
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, ADD_ROW_HEIGHT), Sense::click());
    let hovered = response.hovered();
    let color = if hovered {
        theme::c().accent
    } else {
        theme::c().muted
    };
    crate::ui::icons::paint(
        ui.painter(),
        Rect::from_center_size(
            Pos2::new(rect.left() + 22.0, rect.center().y),
            Vec2::splat(12.0),
        ),
        Icon::Plus,
        color,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 34.0, rect.center().y),
        Align2::LEFT_CENTER,
        "新增一句",
        FontId::proportional(12.0),
        color,
    );
    let activated = response.clicked()
        || (response.has_focus()
            && ui.input(|input| {
                input.key_pressed(egui::Key::Space) || input.key_pressed(egui::Key::Enter)
            }));
    widgets::mark(&response, egui::WidgetType::Button, "新增一句", None);
    crate::ui::focus::focus_on_click(&response);
    if response.has_focus() {
        crate::ui::focus::ring_rect(ui, rect, 6);
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("在当前播放时间附近新增一句");
    activated
}

fn footer(ui: &mut Ui, can_save: bool, save: &mut bool, close: &mut bool) {
    widgets::divider(ui);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("TIME")
                .font(tokens::mono(8.0))
                .strong()
                .color(theme::c().muted),
        );
        ui.label(
            RichText::new("00:00.00  /  空文本 = 纯文本行")
                .font(tokens::mono(8.0))
                .color(theme::c().faint),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_enabled_ui(can_save, |ui| {
                if widgets::primary_button_compact(ui, "保存歌词").clicked() {
                    *save = true;
                }
            });
            ui.add_space(8.0);
            if widgets::secondary_button_compact(ui, "取消").clicked() {
                *close = true;
            }
        });
    });
}
