//! Health-check worker glue and report window.

use eframe::egui;
use egui::{
    Align, Align2, CornerRadius, Frame, Layout, Margin, Pos2, Rect, RichText, ScrollArea, Sense,
    Stroke, UiBuilder, Vec2,
};

use super::{Dialog, RPlayerApp, ToastKind};
use crate::theme;
use crate::ui::widgets;

/// Which stat card the health window is drilled into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HealthPane {
    Missing,
    EmptyTags,
    Duplicates,
}

impl RPlayerApp {
    pub(super) fn start_health_check(&mut self) {
        if self.health_checking {
            return;
        }
        self.health_checking = true;
        let tracks = self.library.tracks.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(crate::health::scan(&tracks));
        });
        self.health_rx = Some(rx);
    }

    pub(super) fn poll_health(&mut self) {
        let Some(rx) = &self.health_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(report) => {
                self.health_report = Some(report);
                self.health_rx = None;
                self.health_checking = false;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.health_rx = None;
                self.health_checking = false;
            }
        }
    }

    fn relink_missing_root(&mut self) {
        let Some(old_root) = rfd::FileDialog::new()
            .set_title("选择缺失文件原来的根目录")
            .pick_folder()
        else {
            return;
        };
        let Some(new_root) = rfd::FileDialog::new()
            .set_title("选择新的根目录")
            .pick_folder()
        else {
            return;
        };
        let missing = self
            .health_report
            .as_ref()
            .map(|report| report.missing.clone())
            .unwrap_or_default();
        let mut ok = 0usize;
        for id in missing {
            let Some(old) = self.library.track(id).cloned() else {
                continue;
            };
            if old.path.exists() {
                continue;
            }
            let Some(candidate) = crate::health::relink_path(&old.path, &old_root, &new_root)
            else {
                continue;
            };
            if !candidate.exists() {
                continue;
            }
            if let Ok(track) = crate::model::Track::from_path(&candidate) {
                if let Some(cue) = old.cue_source.clone() {
                    ok += crate::health::relink_cue_tracks(
                        &mut self.library.tracks,
                        &cue,
                        &candidate,
                        track.duration,
                    );
                } else {
                    let track = crate::health::relinked(&old, track);
                    if let Some(slot) =
                        self.library.tracks.iter_mut().find(|track| track.id == id)
                    {
                        *slot = track;
                    }
                    ok += 1;
                }
            }
        }
        self.persist();
        if let Some(report) = &mut self.health_report {
            report.missing.retain(|id| {
                self.library
                    .track(*id)
                    .is_some_and(|track| !track.path.exists())
            });
        }
        self.toast(
            format!("批量重新定位完成：{ok} 首"),
            if ok > 0 { ToastKind::Success } else { ToastKind::Error },
        );
    }

    fn relink_missing_track(&mut self, id: u64) {
        let Some(old) = self.library.track(id).cloned() else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title("重新定位音频文件")
            .add_filter("音频文件", crate::model::SUPPORTED_EXTENSIONS)
            .pick_file()
        else {
            return;
        };
        match crate::model::Track::from_path(&path) {
            Ok(new_track) => {
                if let Some(cue) = old.cue_source.clone() {
                    let changed = crate::health::relink_cue_tracks(
                        &mut self.library.tracks,
                        &cue,
                        &path,
                        new_track.duration,
                    );
                    if let Some(report) = &mut self.health_report {
                        report.missing.retain(|missing| {
                            self.library
                                .track(*missing)
                                .is_some_and(|track| !track.path.exists())
                        });
                    }
                    self.persist();
                    self.toast(
                        format!("已重新定位 CUE 整轨：{changed} 首曲目"),
                        ToastKind::Success,
                    );
                } else {
                    let track = crate::health::relinked(&old, new_track);
                    if let Some(slot) =
                        self.library.tracks.iter_mut().find(|track| track.id == id)
                    {
                        *slot = track;
                    }
                    if let Some(report) = &mut self.health_report {
                        report.missing.retain(|missing| *missing != id);
                    }
                    self.persist();
                    self.toast("已重新定位文件", ToastKind::Success);
                }
            }
            Err(error) => {
                self.toast(format!("无法读取新文件：{error}"), ToastKind::Error);
            }
        }
    }

    pub(super) fn health_window(&mut self, ctx: &egui::Context) {
        let Some(report) = self.health_report.clone() else {
            return;
        };
        const SIZE: Vec2 = Vec2::new(720.0, 340.0);
        const MARGIN: f32 = 16.0;
        const HEADER_H: f32 = 66.0;
        const CARDS_H: f32 = 64.0;
        const FOOTER_H: f32 = 39.0;
        const GAP: f32 = 8.0;
        let width = super::layout::fit_width(ctx, SIZE.x);
        let content = Vec2::new(width - MARGIN * 2.0, SIZE.y - MARGIN * 2.0);

        let mut close = false;
        let mut rerun = false;
        let mut relink_root = false;
        let mut relink_id: Option<u64> = None;
        let mut edit_ids: Vec<u64> = Vec::new();
        let mut open_duplicates = false;
        let mut pane = self.health_pane;
        let checking = self.health_checking;

        egui::Window::new("rplayer-health")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_size(content)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                Frame::NONE
                    .fill(theme::c().panel)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::same(16))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 18],
                        blur: 44,
                        spread: 0,
                        color: theme::c().shadow,
                    }),
            )
            .show(ctx, |ui| {
                ui.set_width(content.x);
                ui.spacing_mut().item_spacing = Vec2::ZERO;

                let (head, _) = ui.allocate_exact_size(Vec2::new(content.x, HEADER_H), Sense::hover());
                let painter = ui.painter().clone();
                painter.text(
                    Pos2::new(head.left(), head.top()),
                    Align2::LEFT_TOP,
                    "LIBRARY HEALTH",
                    crate::ui::tokens::mono(8.5),
                    theme::c().faint,
                );
                painter.text(
                    Pos2::new(head.left(), head.top() + 12.0),
                    Align2::LEFT_TOP,
                    "曲库健康检查",
                    crate::ui::tokens::serif(26.0),
                    theme::c().text,
                );
                let subline = if checking {
                    "正在重新检查…".to_string()
                } else {
                    format!(
                        "已扫描 {} 首 · {} 项需要关注",
                        report.scanned,
                        report.issue_count()
                    )
                };
                painter.text(
                    Pos2::new(head.left(), head.top() + 46.0),
                    Align2::LEFT_TOP,
                    subline,
                    egui::FontId::proportional(10.0),
                    theme::c().muted,
                );

                let mut head_ui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(head)
                        .layout(Layout::right_to_left(Align::Min)),
                );
                if widgets::icon_button(&mut head_ui, crate::ui::icons::Icon::Close, 22.0, "关闭")
                    .clicked()
                {
                    close = true;
                }

                ui.add_space(GAP);
                let (cards, _) =
                    ui.allocate_exact_size(Vec2::new(content.x, CARDS_H), Sense::hover());
                let painter = ui.painter().clone();
                painter.rect_filled(cards, CornerRadius::same(8), theme::c().card);
                painter.rect_stroke(
                    cards,
                    CornerRadius::same(8),
                    Stroke::new(1.0_f32, theme::c().border_soft),
                    egui::StrokeKind::Inside,
                );
                let third = content.x / 3.0;
                let entries: [(HealthPane, &str, usize); 3] = [
                    (HealthPane::Missing, "缺失文件", report.missing.len()),
                    (HealthPane::EmptyTags, "空标签", report.empty_tags.len()),
                    (HealthPane::Duplicates, "重复候选", report.duplicates.len()),
                ];
                for (index, (target, label, count)) in entries.iter().enumerate() {
                    let left = cards.left() + third * index as f32;
                    let cell = Rect::from_min_max(
                        Pos2::new(left, cards.top()),
                        Pos2::new(left + third, cards.bottom()),
                    );
                    if index > 0 {
                        painter.line_segment(
                            [
                                Pos2::new(left, cards.top() + 10.0),
                                Pos2::new(left, cards.bottom() - 10.0),
                            ],
                            Stroke::new(1.0_f32, theme::c().border_soft),
                        );
                    }
                    let response =
                        ui.interact(cell, ui.id().with(("health-card", index)), Sense::click());
                    let active = pane == Some(*target);
                    if active {
                        painter.rect_filled(
                            cell.shrink2(Vec2::new(1.0, 1.0)),
                            CornerRadius::same(7),
                            theme::accent_alpha(0.08),
                        );
                    } else if response.hovered() {
                        painter.rect_filled(
                            cell.shrink2(Vec2::new(1.0, 1.0)),
                            CornerRadius::same(7),
                            theme::c().hover,
                        );
                    }
                    painter.text(
                        Pos2::new(cell.left() + 14.0, cell.top() + 9.0),
                        Align2::LEFT_TOP,
                        count.to_string(),
                        crate::ui::tokens::serif(30.0),
                        if *count > 0 {
                            theme::signal()
                        } else {
                            theme::c().accent
                        },
                    );
                    painter.text(
                        Pos2::new(cell.left() + 14.0, cell.bottom() - 10.0),
                        Align2::LEFT_BOTTOM,
                        *label,
                        crate::ui::tokens::mono(8.5),
                        theme::c().faint,
                    );
                    if active {
                        painter.hline(
                            (cell.left() + 10.0)..=(cell.right() - 10.0),
                            cell.bottom() - 2.0,
                            Stroke::new(1.5_f32, theme::c().accent),
                        );
                    }
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            active,
                            format!("{label} {count}"),
                        )
                    });
                    if response.clicked() {
                        pane = if active { None } else { Some(*target) };
                    }
                }

                ui.add_space(GAP);
                let body_h = content.y - HEADER_H - GAP - CARDS_H - GAP - FOOTER_H;
                ScrollArea::vertical()
                    .id_salt("health-rows")
                    .max_height(body_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(content.x);
                        if report.issue_count() == 0 {
                            ui.add_space(14.0);
                            ui.vertical_centered(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(
                                    Vec2::splat(24.0),
                                    Sense::hover(),
                                );
                                crate::ui::icons::paint(
                                    ui.painter(),
                                    icon_rect,
                                    crate::ui::icons::Icon::Check,
                                    theme::c().accent,
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new("没有发现明显问题")
                                        .font(crate::ui::tokens::serif(18.0))
                                        .color(theme::c().accent),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(format!(
                                        "已扫描 {} 首，缺失文件 0 · 空标签 0 · 重复候选 0。",
                                        report.scanned
                                    ))
                                    .size(9.5)
                                    .color(theme::c().muted),
                                );
                            });
                        } else {
                            match pane {
                                None => {
                                    let first_missing = report
                                        .missing
                                        .first()
                                        .and_then(|id| self.library.track(*id))
                                        .map(|track| track.path.display().to_string())
                                        .unwrap_or_else(|| "已扫描但文件不存在".to_string());
                                    if !report.missing.is_empty()
                                        && issue_row(
                                            ui,
                                            content.x,
                                            crate::ui::icons::Icon::Warning,
                                            theme::signal(),
                                            &format!("{} 首歌曲找不到文件", report.missing.len()),
                                            &first_missing,
                                            "定位",
                                        )
                                    {
                                        relink_root = true;
                                    }
                                    if !report.empty_tags.is_empty() {
                                        let genre_only = report.empty_tags.iter().all(|id| {
                                            self.library
                                                .track(*id)
                                                .is_some_and(|track| track.genre.trim().is_empty())
                                        });
                                        let title = if genre_only {
                                            format!("{} 首缺少流派", report.empty_tags.len())
                                        } else {
                                            format!("{} 首标签不完整", report.empty_tags.len())
                                        };
                                        if issue_row(
                                            ui,
                                            content.x,
                                            crate::ui::icons::Icon::Music,
                                            theme::c().muted,
                                            &title,
                                            "标题 / 艺术家 / 专辑 / 流派存在空值",
                                            "去编辑",
                                        ) {
                                            edit_ids = report.empty_tags.clone();
                                        }
                                    }
                                    if !report.duplicates.is_empty() {
                                        let files: usize =
                                            report.duplicates.iter().map(|group| group.ids.len()).sum();
                                        if issue_row(
                                            ui,
                                            content.x,
                                            crate::ui::icons::Icon::Duplicate,
                                            theme::c().accent,
                                            &format!("{} 组重复候选", report.duplicates.len()),
                                            &format!("{files} 个文件 · 同一标题 / 艺术家 / 时长"),
                                            "查看",
                                        ) {
                                            open_duplicates = true;
                                        }
                                    }
                                }
                                Some(HealthPane::Missing) => {
                                    for id in &report.missing {
                                        let Some(track) = self.library.track(*id) else {
                                            continue;
                                        };
                                        if issue_row(
                                            ui,
                                            content.x,
                                            crate::ui::icons::Icon::Warning,
                                            theme::signal(),
                                            &track.title,
                                            &track.path.display().to_string(),
                                            "定位",
                                        ) {
                                            relink_id = Some(*id);
                                        }
                                    }
                                }
                                Some(HealthPane::EmptyTags) => {
                                    for id in &report.empty_tags {
                                        let Some(track) = self.library.track(*id) else {
                                            continue;
                                        };
                                        let mut empty = Vec::new();
                                        if track.title.trim().is_empty() {
                                            empty.push("标题");
                                        }
                                        if track.artist.trim().is_empty() {
                                            empty.push("艺术家");
                                        }
                                        if track.album.trim().is_empty() {
                                            empty.push("专辑");
                                        }
                                        if track.genre.trim().is_empty() {
                                            empty.push("流派");
                                        }
                                        if issue_row(
                                            ui,
                                            content.x,
                                            crate::ui::icons::Icon::Music,
                                            theme::c().muted,
                                            &format!("{} — {}", track.title, track.artist),
                                            &format!("缺少：{}", empty.join(" / ")),
                                            "去编辑",
                                        ) {
                                            edit_ids = vec![*id];
                                        }
                                    }
                                }
                                Some(HealthPane::Duplicates) => {
                                    for group in &report.duplicates {
                                        if issue_row(
                                            ui,
                                            content.x,
                                            crate::ui::icons::Icon::Duplicate,
                                            theme::c().accent,
                                            &format!("{} — {}", group.title, group.artist),
                                            &format!("{} 个文件候选", group.ids.len()),
                                            "查看",
                                        ) {
                                            open_duplicates = true;
                                        }
                                    }
                                }
                            }
                        }
                    });

                let (footer, _) =
                    ui.allocate_exact_size(Vec2::new(content.x, FOOTER_H), Sense::hover());
                let painter = ui.painter().clone();
                painter.line_segment(
                    [
                        Pos2::new(footer.left(), footer.top()),
                        Pos2::new(footer.right(), footer.top()),
                    ],
                    Stroke::new(1.0_f32, theme::divider()),
                );
                painter.text(
                    Pos2::new(footer.left(), footer.center().y),
                    Align2::LEFT_CENTER,
                    "检查不会删除音频，也不会自动删除文件。",
                    egui::FontId::proportional(9.5),
                    theme::c().faint,
                );
                let mut foot_ui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(footer.shrink2(Vec2::new(0.0, 4.0)))
                        .layout(Layout::right_to_left(Align::Center)),
                );
                if widgets::secondary_button_sized(&mut foot_ui, "重新检查", 84.0).clicked() {
                    rerun = true;
                }
            });

        if rerun {
            self.start_health_check();
        }
        if relink_root {
            self.relink_missing_root();
        }
        if let Some(id) = relink_id {
            self.relink_missing_track(id);
        }
        if !edit_ids.is_empty() {
            self.dialog = Dialog::BatchTags {
                ids: edit_ids,
                title: String::new(),
                artist: String::new(),
                album: String::new(),
                genre: String::new(),
                year: String::new(),
            };
        }
        if close || open_duplicates {
            self.health_report = None;
            self.health_pane = None;
        } else {
            self.health_pane = pane;
        }
        if open_duplicates {
            self.open_duplicates();
        }
    }
}

fn issue_row(
    ui: &mut egui::Ui,
    width: f32,
    icon: crate::ui::icons::Icon,
    tint: egui::Color32,
    title: &str,
    detail: &str,
    action: &str,
) -> bool {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 44.0), Sense::hover());
    let painter = ui.painter().clone().with_clip_rect(rect);
    painter.line_segment(
        [
            Pos2::new(rect.left(), rect.bottom() - 0.5),
            Pos2::new(rect.right(), rect.bottom() - 0.5),
        ],
        Stroke::new(1.0_f32, theme::divider()),
    );
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 20.0, rect.center().y),
        Vec2::splat(15.0),
    );
    crate::ui::icons::paint(&painter, icon_rect, icon, tint);
    painter.text(
        Pos2::new(rect.left() + 38.0, rect.top() + 7.0),
        Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(12.5),
        theme::c().text,
    );
    painter.text(
        Pos2::new(rect.left() + 38.0, rect.top() + 25.0),
        Align2::LEFT_TOP,
        detail,
        egui::FontId::proportional(9.5),
        theme::c().muted,
    );

    let button_rect = Rect::from_min_size(
        Pos2::new(rect.right() - 12.0 - 64.0, rect.center().y - 14.0),
        Vec2::new(64.0, 28.0),
    );
    let button = egui::Button::new(
        RichText::new(action)
            .size(11.0)
            .color(theme::c().text),
    )
    .fill(theme::c().card)
    .stroke(Stroke::new(1.0_f32, theme::c().border))
    .corner_radius(CornerRadius::same(8));
    let response = ui.put(button_rect, button);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, false, format!("{action}：{title}"))
    });
    response.clicked()
}
