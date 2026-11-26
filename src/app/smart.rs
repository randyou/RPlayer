//! Smart-playlist evaluation and its rule builder.

use eframe::egui;
use egui::{
    Align, Color32, CornerRadius, FontId, Frame, Layout, Margin, Pos2, Rect, Response, RichText,
    Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

use super::{RPlayerApp, ToastKind, View};
use crate::model::{SmartPlaylist, SmartRule, Track};
use crate::theme;
use crate::ui::icons::Icon;
use crate::ui::{tokens, widgets};

pub(super) const FIELDS: [(&str, &str); 8] = [
    ("title", "标题"),
    ("artist", "艺术家"),
    ("album", "专辑"),
    ("genre", "流派"),
    ("year", "年份"),
    ("rating", "评分"),
    ("play_count", "播放次数"),
    ("skip_count", "跳过次数"),
];

pub(super) const OPS: [&str; 6] = ["包含", "是", ">=", "<=", ">", "<"];

/// Design D8 is 620×430 including the 16px frame margin.
const WINDOW_SIZE: Vec2 = Vec2::new(620.0, 430.0);
const MARGIN: f32 = 16.0;
const CONTENT_SIZE: Vec2 = Vec2::new(WINDOW_SIZE.x - MARGIN * 2.0, WINDOW_SIZE.y - MARGIN * 2.0);
const RULE_ROW: f32 = 30.0;
const FIELD_W: f32 = 116.0;
const OP_W: f32 = 88.0;
const INPUT_H: f32 = 32.0;
const FOOTER_H: f32 = 39.0;

#[derive(Debug, Clone)]
pub(super) struct SmartDraft {
    pub id: Option<u64>,
    pub name: String,
    pub match_all: bool,
    pub limit: usize,
    pub rules: Vec<SmartRule>,
}

impl Default for SmartDraft {
    fn default() -> Self {
        Self {
            id: None,
            name: "我的智能歌单".to_string(),
            match_all: true,
            limit: 0,
            rules: vec![SmartRule::default()],
        }
    }
}

impl RPlayerApp {
    pub(super) fn open_smart_builder(&mut self) {
        self.smart_builder = Some(SmartDraft::default());
    }

    pub(super) fn edit_smart_playlist(&mut self, id: u64) {
        let Some(playlist) = self.library.smart_playlist(id) else {
            return;
        };
        self.smart_builder = Some(SmartDraft {
            id: Some(id),
            name: playlist.name.clone(),
            match_all: playlist.match_all,
            limit: playlist.limit,
            rules: playlist.rules.clone(),
        });
    }

    pub(super) fn smart_ids(&self, playlist: &SmartPlaylist) -> Vec<u64> {
        let mut ids: Vec<u64> = self
            .library
            .tracks
            .iter()
            .filter(|track| self.smart_matches(track, playlist))
            .map(|track| track.id)
            .collect();
        if playlist.limit > 0 {
            ids.truncate(playlist.limit);
        }
        ids
    }

    fn smart_matches(&self, track: &Track, playlist: &SmartPlaylist) -> bool {
        if playlist.rules.is_empty() {
            return true;
        }
        let matches = |rule: &SmartRule| self.rule_matches(track, rule);
        if playlist.match_all {
            playlist.rules.iter().all(matches)
        } else {
            playlist.rules.iter().any(matches)
        }
    }

    fn rule_matches(&self, track: &Track, rule: &SmartRule) -> bool {
        let stats = self.stats.get(track.id);
        let text = match rule.field.as_str() {
            "title" => track.title.to_lowercase(),
            "artist" => track.artist.to_lowercase(),
            "album" => track.album.to_lowercase(),
            "genre" => track.genre.to_lowercase(),
            _ => String::new(),
        };
        let number = match rule.field.as_str() {
            "year" => track.year.unwrap_or(0) as f64,
            "rating" => stats.rating as f64,
            "play_count" => stats.play_count as f64,
            "skip_count" => stats.skip_count as f64,
            _ => 0.0,
        };
        let value = rule.value.trim().to_lowercase();
        if value.is_empty() {
            return false;
        }
        match rule.op.as_str() {
            "包含" => text.contains(&value),
            "是" => text == value,
            ">=" => number >= value.parse::<f64>().unwrap_or(0.0),
            "<=" => number <= value.parse::<f64>().unwrap_or(0.0),
            ">" => number > value.parse::<f64>().unwrap_or(0.0),
            "<" => number < value.parse::<f64>().unwrap_or(0.0),
            _ => false,
        }
    }

    pub(super) fn smart_builder_window(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.smart_builder.take() else {
            return;
        };
        let mut save = false;
        let mut cancel = false;

        // Popups read the context style, so scope it for the combo menus.
        let previous_style = ctx.style();
        {
            let mut scoped = (*previous_style).clone();
            scoped.visuals.window_fill = theme::c().panel;
            scoped.visuals.panel_fill = theme::c().panel;
            scoped.visuals.window_stroke = Stroke::new(1.0_f32, theme::c().border_soft);
            scoped.visuals.window_corner_radius = CornerRadius::same(tokens::RADIUS_CONTROL);
            scoped.visuals.selection.bg_fill = theme::accent_alpha(0.16);
            scoped.visuals.selection.stroke = Stroke::new(1.0_f32, theme::c().accent);
            ctx.set_style(scoped);
        }

        egui::Window::new("rplayer-smart-builder")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_size(CONTENT_SIZE)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                Frame::NONE
                    .fill(theme::c().panel)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(tokens::RADIUS_CARD))
                    .inner_margin(Margin::same(MARGIN as i8))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 18],
                        blur: 44,
                        spread: 0,
                        color: theme::c().shadow,
                    }),
            )
            .show(ctx, |ui| {
                ui.set_width(CONTENT_SIZE.x);
                ui.spacing_mut().interact_size.y = RULE_ROW;
                style_window(ui);
                let preview_count = self
                    .smart_ids(&SmartPlaylist {
                        id: 0,
                        name: String::new(),
                        match_all: draft.match_all,
                        rules: draft.rules.clone(),
                        limit: draft.limit,
                        created_at: 0,
                    })
                    .len();
                header(ui, &draft, &mut cancel);
                ui.add_space(4.0);
                ui.label(
                    RichText::new("名称")
                        .font(tokens::mono(8.5))
                        .color(theme::c().muted),
                );
                ui.add_space(2.0);
                let _ = text_field(
                    ui,
                    ("smart-name", 0),
                    &mut draft.name,
                    "智能歌单名称",
                    CONTENT_SIZE.x,
                    false,
                    false,
                );
                ui.add_space(4.0);
                match_row(
                    ui,
                    &mut draft.match_all,
                    preview_count,
                    &mut draft.limit,
                );
                ui.add_space(4.0);

                let mut remove = None;
                let rule_count = draft.rules.len();
                for (index, rule) in draft.rules.iter_mut().enumerate() {
                    let incomplete = rule.value.trim().is_empty();
                    if rule_row(ui, index, rule, incomplete) {
                        remove = Some(index);
                    }
                    if index + 1 < rule_count {
                        ui.add_space(4.0);
                    }
                }
                if let Some(index) = remove {
                    draft.rules.remove(index);
                }

                ui.add_space(4.0);
                let missing: Vec<usize> = draft
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(_, rule)| rule.value.trim().is_empty())
                    .map(|(index, _)| index)
                    .collect();
                let can_save =
                    missing.is_empty() && !draft.name.trim().is_empty() && !draft.rules.is_empty();
                if dashed_add(ui, "添加条件") {
                    draft.rules.push(SmartRule {
                        field: "artist".to_string(),
                        op: "包含".to_string(),
                        value: String::new(),
                    });
                }
                ui.add_space(6.0);
                if let Some(first) = missing.first() {
                    ui.label(
                        RichText::new(format!("第 {} 条规则缺少值", first + 1))
                            .size(9.5)
                            .color(theme::signal()),
                    );
                } else {
                    ui.add_space(12.0);
                }
                ui.add_space(4.0);
                preview_block(ui, preview_count, missing.len());
                let rest = ui.available_height() - FOOTER_H;
                if rest > 0.0 {
                    ui.add_space(rest);
                }
                footer(ui, can_save, &mut save, &mut cancel);
            });
        ctx.set_style((*previous_style).clone());

        let can_save = !draft.name.trim().is_empty()
            && !draft.rules.is_empty()
            && draft.rules.iter().all(|rule| !rule.value.trim().is_empty());
        if save && can_save {
            let name = draft.name.trim().to_string();
            let id = if let Some(id) = draft.id {
                if let Some(playlist) = self
                    .library
                    .smart_playlists
                    .iter_mut()
                    .find(|playlist| playlist.id == id)
                {
                    playlist.name = name.clone();
                    playlist.match_all = draft.match_all;
                    playlist.rules = draft.rules.clone();
                    playlist.limit = draft.limit;
                }
                id
            } else {
                self.library.create_smart_playlist(
                    name.clone(),
                    draft.match_all,
                    draft.rules.clone(),
                    draft.limit,
                )
            };
            self.view = View::Smart(id);
            self.persist();
            self.toast(
                format!(
                    "{}智能歌单「{name}」",
                    if draft.id.is_some() { "已更新" } else { "已创建" }
                ),
                ToastKind::Success,
            );
        } else if !cancel {
            self.smart_builder = Some(draft);
        }
    }
}

/// Mock styling: combos and drag values look like flat card boxes with a
/// hairline border rather than egui's default tinted widgets.
fn style_window(ui: &mut Ui) {
    let palette = theme::c();
    let style = ui.style_mut();
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::proportional(11.5),
    );
    let widgets = &mut style.visuals.widgets;
    for state in [
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        state.weak_bg_fill = palette.card;
        state.bg_fill = palette.card;
        state.bg_stroke = Stroke::new(1.0_f32, palette.border);
        state.corner_radius = CornerRadius::same(tokens::RADIUS_CONTROL);
        state.fg_stroke = Stroke::new(1.0_f32, palette.text);
    }
    widgets.hovered.weak_bg_fill = palette.hover;
    widgets.hovered.bg_fill = palette.hover;
    widgets.active.weak_bg_fill = palette.hover;
    widgets.active.bg_fill = palette.hover;
    style.visuals.selection.bg_fill = theme::accent_alpha(0.18);
    style.visuals.selection.stroke = Stroke::new(1.0_f32, palette.accent);
}

fn header(ui: &mut Ui, draft: &SmartDraft, cancel: &mut bool) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(
                RichText::new("SMART PLAYLIST")
                    .font(tokens::mono(8.5))
                    .color(theme::c().faint),
            );
            ui.label(
                RichText::new(if draft.id.is_some() {
                    "编辑智能歌单"
                } else {
                    "新建智能歌单"
                })
                .font(tokens::serif(27.0))
                .color(theme::c().text),
            );
            ui.label(
                RichText::new("规则实时计算，保存后自动更新")
                    .size(9.5)
                    .color(theme::c().muted),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
            if widgets::icon_button(ui, Icon::Close, 24.0, "关闭智能歌单构建器").clicked() {
                *cancel = true;
            }
        });
    });
}

fn match_row(ui: &mut Ui, match_all: &mut bool, count: usize, limit: &mut usize) {
    ui.horizontal(|ui| {
        if segment(ui, "全部条件", *match_all).clicked() {
            *match_all = true;
        }
        if segment(ui, "任一条件", !*match_all).clicked() {
            *match_all = false;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(
                egui::DragValue::new(limit)
                    .range(0..=500)
                    .speed(1.0)
                    .custom_formatter(|value, _| format!("{}", value as i64)),
            )
            .on_hover_text("结果最多保留的数量；0 表示不限制");
            ui.label(
                RichText::new(format!("{count} 首 · 限制"))
                    .font(tokens::mono(9.0))
                    .color(theme::c().faint),
            );
        });
    });
}

fn rule_row(ui: &mut Ui, index: usize, rule: &mut SmartRule, incomplete: bool) -> bool {
    let width = ui.available_width();
    let (rect, row_response) = ui.allocate_exact_size(Vec2::new(width, RULE_ROW), Sense::hover());
    let mut remove = false;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(22.0, RULE_ROW),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(format!("{:02}", index + 1))
                            .font(tokens::mono(8.5))
                            .color(theme::c().faint),
                    );
                },
            );
            let field_text = field_label(&rule.field).to_string();
            combo(
                ui,
                ("smart-field", index),
                &mut rule.field,
                FIELD_W,
                field_text,
                |ui, field| {
                    for (id, label) in FIELDS {
                        ui.selectable_value(field, id.to_string(), label);
                    }
                },
            );
            ui.add_space(6.0);
            let op_text = rule.op.clone();
            combo(
                ui,
                ("smart-op", index),
                &mut rule.op,
                OP_W,
                op_text,
                |ui, op| {
                    for candidate in OPS {
                        ui.selectable_value(op, candidate.to_string(), candidate);
                    }
                },
            );
            ui.add_space(6.0);
            let value_width = (ui.available_width() - 26.0).max(120.0);
            let _ = text_field(
                ui,
                ("smart-value", index),
                &mut rule.value,
                "值",
                value_width,
                incomplete,
                false,
            );
            if row_response.hovered() {
                ui.add_space(2.0);
                if widgets::icon_button(ui, Icon::Trash, 22.0, "删除这条规则").clicked() {
                    remove = true;
                }
            }
        },
    );
    remove
}

fn combo(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    width: f32,
    selected: String,
    options: impl FnOnce(&mut Ui, &mut String),
) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, RULE_ROW),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_width(width);
            egui::ComboBox::from_id_salt(id_salt)
                .width(width)
                .height(240.0)
                .selected_text(selected)
                .show_ui(ui, |ui| options(ui, value));
        },
    );
}

/// One framed single-line input matching the mock's 8px rounded boxes.
fn text_field(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    hint: &str,
    width: f32,
    danger: bool,
    mono: bool,
) -> Response {
    let stroke = if danger {
        Stroke::new(1.0_f32, theme::c().danger)
    } else {
        Stroke::new(1.0_f32, theme::c().border)
    };
    let mut response = None;
    Frame::NONE
        .fill(theme::c().card)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(tokens::RADIUS_CONTROL))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            let font = if mono {
                tokens::mono(11.0)
            } else {
                FontId::proportional(11.5)
            };
            let edit = ui.add_sized(
                Vec2::new(width - 20.0, INPUT_H - 14.0),
                egui::TextEdit::singleline(value)
                    .id_salt(id_salt)
                    .frame(false)
                    .font(font)
                    .text_color(theme::c().text)
                    .hint_text(hint),
            );
            response = Some(edit);
        });
    response.expect("text field always draws")
}

fn segment(ui: &mut Ui, text: &str, selected: bool) -> Response {
    let color = if selected {
        theme::c().on_accent
    } else {
        theme::c().muted
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(11.5), color);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(galley.size().x + 52.0, 30.0), Sense::click());
    let hovered = response.hovered();
    let radius = CornerRadius::same(tokens::RADIUS_CONTROL);
    let fill = if selected {
        theme::c().accent
    } else if hovered {
        theme::c().hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(
            1.0_f32,
            if selected {
                theme::c().accent
            } else {
                theme::c().border
            },
        ),
        StrokeKind::Inside,
    );
    ui.painter().galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        color,
    );
    widgets::mark(&response, egui::WidgetType::Button, text, Some(selected));
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn dashed_add(ui: &mut Ui, text: &str) -> bool {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        FontId::proportional(11.5),
        theme::c().muted,
    );
    let width = galley.size().x + 54.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
    let color = if response.hovered() {
        theme::c().accent
    } else {
        theme::c().muted
    };
    dashed_round_rect(
        ui.painter(),
        rect,
        tokens::RADIUS_CONTROL as f32,
        Stroke::new(1.0_f32, color.gamma_multiply(0.8)),
        4.0,
        3.0,
    );
    let plus_center = Pos2::new(rect.left() + 18.0, rect.center().y);
    ui.painter().line_segment(
        [
            Pos2::new(plus_center.x - 4.0, plus_center.y),
            Pos2::new(plus_center.x + 4.0, plus_center.y),
        ],
        Stroke::new(1.0_f32, color),
    );
    ui.painter().line_segment(
        [
            Pos2::new(plus_center.x, plus_center.y - 4.0),
            Pos2::new(plus_center.x, plus_center.y + 4.0),
        ],
        Stroke::new(1.0_f32, color),
    );
    ui.painter().galley(
        Pos2::new(
            plus_center.x + 10.0,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        color,
    );
    widgets::mark(&response, egui::WidgetType::Button, text, None);
    crate::ui::focus::focus_on_click(&response);
    crate::ui::focus::ring(ui, &response);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn dashed_round_rect(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    stroke: Stroke,
    dash: f32,
    gap: f32,
) {
    let mut points = Vec::with_capacity(40);
    let corners = [
        (rect.right_top() + Vec2::new(-radius, radius), -90.0),
        (rect.right_bottom() + Vec2::new(-radius, -radius), 0.0),
        (rect.left_bottom() + Vec2::new(radius, -radius), 90.0),
        (rect.left_top() + Vec2::new(radius, radius), 180.0),
    ];
    for (center, start) in corners {
        for step in 0..=6 {
            let angle = (start + 90.0 * step as f32 / 6.0).to_radians();
            points.push(Pos2::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            ));
        }
    }
    points.push(points[0]);
    painter.extend(Shape::dashed_line(&points, stroke, dash, gap));
}

fn preview_block(ui: &mut Ui, count: usize, missing: usize) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 54.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0, theme::accent_alpha(0.10));
    painter.rect_filled(
        Rect::from_min_size(rect.left_top(), Vec2::new(2.0, rect.height())),
        0,
        theme::c().accent,
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(14.0, 9.0)))
            .layout(Layout::top_down(Align::Min)),
        |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("PREVIEW")
                        .font(tokens::mono(8.0))
                        .color(theme::c().muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{count} 首匹配"))
                            .font(tokens::serif(15.0))
                            .color(theme::c().accent),
                    );
                });
            });
            ui.add_space(2.0);
            ui.label(
                RichText::new(if missing > 0 {
                    format!(
                        "补全 {missing} 条规则后才能保存。规则会随评分与播放统计实时更新。"
                    )
                } else {
                    "规则会随评分与播放统计实时更新。".to_string()
                })
                .size(9.5)
                .color(theme::c().muted),
            );
        },
    );
}

fn footer(ui: &mut Ui, can_save: bool, save: &mut bool, cancel: &mut bool) {
    widgets::divider(ui);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_enabled_ui(can_save, |ui| {
                if widgets::primary_button_compact(ui, "保存智能歌单").clicked() {
                    *save = true;
                }
            });
            ui.add_space(8.0);
            if widgets::secondary_button_compact(ui, "取消").clicked() {
                *cancel = true;
            }
        });
    });
}

fn field_label(field: &str) -> &'static str {
    FIELDS
        .iter()
        .find(|(id, _)| *id == field)
        .map(|(_, label)| *label)
        .unwrap_or("字段")
}

