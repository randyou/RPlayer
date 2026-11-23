//! Smart-playlist evaluation and its rule builder.

use eframe::egui;
use egui::{Align, Frame, Layout, Margin, RichText, Stroke, Vec2};

use super::{RPlayerApp, ToastKind, View};
use crate::model::{SmartPlaylist, SmartRule, Track};
use crate::theme;
use crate::ui::widgets;

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

        egui::Window::new("rplayer-smart-builder")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_size(Vec2::new(540.0, 470.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                Frame::NONE
                    .fill(theme::c().panel)
                    .stroke(Stroke::new(1.0_f32, theme::c().border))
                    .corner_radius(egui::CornerRadius::same(16))
                    .inner_margin(Margin::same(18))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 16],
                        blur: 36,
                        spread: 0,
                        color: theme::c().shadow,
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if draft.id.is_some() {
                            "编辑智能歌单"
                        } else {
                            "新建智能歌单"
                        })
                            .size(18.0)
                            .strong()
                            .color(theme::c().text),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, crate::ui::icons::Icon::Close, 26.0, "关闭")
                            .clicked()
                        {
                            cancel = true;
                        }
                    });
                });
                ui.label(
                    RichText::new("规则会随评分、播放统计和曲库变化实时更新。")
                        .size(10.5)
                        .color(theme::c().muted),
                );
                ui.add_space(12.0);
                ui.add(
                    egui::TextEdit::singleline(&mut draft.name)
                        .hint_text("智能歌单名称")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("满足").size(11.0).color(theme::c().muted));
                    if widgets::chip(ui, "全部条件", draft.match_all).clicked() {
                        draft.match_all = true;
                    }
                    if widgets::chip(ui, "任一条件", !draft.match_all).clicked() {
                        draft.match_all = false;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("限制 {} 首", draft.limit))
                                .size(10.0)
                                .color(theme::c().faint),
                        );
                        ui.add(
                            egui::DragValue::new(&mut draft.limit)
                                .range(0..=500)
                                .speed(1.0),
                        );
                    });
                });
                ui.add_space(8.0);

                let mut remove = None;
                for (index, rule) in draft.rules.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt(("smart-field", index))
                            .width(92.0)
                            .selected_text(field_label(&rule.field))
                            .show_ui(ui, |ui| {
                                for (id, label) in FIELDS {
                                    ui.selectable_value(&mut rule.field, id.to_string(), label);
                                }
                            });
                        egui::ComboBox::from_id_salt(("smart-op", index))
                            .width(66.0)
                            .selected_text(&rule.op)
                            .show_ui(ui, |ui| {
                                for op in OPS {
                                    ui.selectable_value(&mut rule.op, op.to_string(), op);
                                }
                            });
                        ui.add(
                            egui::TextEdit::singleline(&mut rule.value)
                                .desired_width(105.0)
                                .hint_text("值"),
                        );
                        if widgets::icon_button(ui, crate::ui::icons::Icon::Close, 24.0, "删除条件")
                            .clicked()
                        {
                            remove = Some(index);
                        }
                    });
                    ui.add_space(4.0);
                }
                if let Some(index) = remove {
                    draft.rules.remove(index);
                }
                if widgets::chip(ui, "＋ 添加条件", false).clicked() {
                    draft.rules.push(SmartRule {
                        field: "artist".to_string(),
                        op: "包含".to_string(),
                        value: String::new(),
                    });
                }

                ui.add_space(10.0);
                ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
                    ui.horizontal(|ui| {
                        if widgets::primary_button(ui, "保存智能歌单").clicked() {
                            save = true;
                        }
                        if widgets::secondary_button(ui, "取消").clicked() {
                            cancel = true;
                        }
                    });
                });
            });

        if save && !draft.name.trim().is_empty() && !draft.rules.is_empty() {
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

fn field_label(field: &str) -> &'static str {
    FIELDS
        .iter()
        .find(|(id, _)| *id == field)
        .map(|(_, label)| *label)
        .unwrap_or("字段")
}
