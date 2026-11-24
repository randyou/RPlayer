//! Health-check worker glue and report window.

use eframe::egui;
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Vec2};

use super::{RPlayerApp, ToastKind};
use crate::health::HealthReport;
use crate::theme;
use crate::ui::widgets;

impl RPlayerApp {
    pub(super) fn start_health_check(&mut self) {
        if self.health_checking {
            return;
        }
        self.health_checking = true;
        self.health_report = None;
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
            let Some(candidate) = crate::health::relink_path(&old.path, &old_root, &new_root)
            else {
                continue;
            };
            if !candidate.exists() {
                continue;
            }
            if let Ok(track) = crate::model::Track::from_path(&candidate) {
                let track = crate::health::relinked(&old, track);
                if let Some(slot) = self.library.tracks.iter_mut().find(|track| track.id == id) {
                    *slot = track;
                }
                ok += 1;
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
            Ok(track) => {
                let track = crate::health::relinked(&old, track);
                if let Some(slot) = self.library.tracks.iter_mut().find(|track| track.id == id) {
                    *slot = track;
                }
                if let Some(report) = &mut self.health_report {
                    report.missing.retain(|missing| *missing != id);
                }
                self.persist();
                self.toast("已重新定位文件", ToastKind::Success);
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
        let mut close = false;
        let mut remove_missing = false;
        let mut relink_id: Option<u64> = None;
        let mut relink_root = false;

        egui::Window::new("rplayer-health")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_size(Vec2::new(600.0, 480.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                Frame::NONE
                    .fill(theme::c().panel)
                    .stroke(Stroke::new(1.0_f32, theme::c().border))
                    .corner_radius(CornerRadius::same(16))
                    .inner_margin(Margin::same(18))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 16],
                        blur: 40,
                        spread: 0,
                        color: theme::c().shadow,
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("曲库健康检查")
                            .size(18.0)
                            .strong()
                            .color(theme::c().text),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, crate::ui::icons::Icon::Close, 26.0, "关闭")
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
                ui.label(
                    RichText::new(format!(
                        "已扫描 {} 首 · {} 项需要关注",
                        report.scanned,
                        report.issue_count()
                    ))
                    .size(11.0)
                    .color(theme::c().muted),
                );
                ui.add_space(10.0);

                ScrollArea::vertical()
                    .max_height(330.0)
                    .show(ui, |ui| {
                        section(ui, "缺失文件", report.missing.len());
                        for id in &report.missing {
                            if let Some(track) = self.library.track(*id) {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&track.title)
                                                .size(11.5)
                                                .color(theme::c().text),
                                        );
                                        ui.label(
                                            RichText::new(track.path.to_string_lossy())
                                                .size(9.5)
                                                .color(theme::c().faint),
                                        );
                                    });
                                    ui.with_layout(
                                        Layout::right_to_left(Align::Center),
                                        |ui| {
                                            if track.cue_source.is_none()
                                                && widgets::text_button(ui, "重新定位…").clicked()
                                            {
                                                relink_id = Some(*id);
                                            }
                                        },
                                    );
                                });
                            }
                        }
                        section(ui, "空标签", report.empty_tags.len());
                        for id in &report.empty_tags {
                            if let Some(track) = self.library.track(*id) {
                                row(ui, &track.title, "标题/艺术家/专辑存在空值");
                            }
                        }
                        section(ui, "重复候选", report.duplicates.len());
                        for group in &report.duplicates {
                            row(
                                ui,
                                &format!("{} — {}", group.title, group.artist),
                                &format!("{} 个文件，时长相差 ≤2 秒", group.ids.len()),
                            );
                        }
                        if report.issue_count() == 0 {
                            ui.add_space(30.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new("没有发现明显问题")
                                        .size(14.0)
                                        .color(theme::c().accent),
                                );
                            });
                        }
                    });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if !report.missing.is_empty() {
                        if widgets::secondary_button(ui, "批量替换根目录…").clicked() {
                            relink_root = true;
                        }
                        if widgets::danger_button(
                            ui,
                            &format!("移除 {} 条失效记录", report.missing.len()),
                        )
                        .clicked()
                        {
                            remove_missing = true;
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::primary_button(ui, "关闭").clicked() {
                            close = true;
                        }
                    });
                });
                ui.label(
                    RichText::new("快速检查不会解码音频；重复项只提示，不会自动删除。")
                        .size(9.5)
                        .color(theme::c().faint),
                );
            });

        if remove_missing {
            let missing = report.missing.clone();
            self.library.tracks.retain(|track| !missing.contains(&track.id));
            self.library.prune_playlists();
            self.prune_transport();
            self.persist();
            self.toast(
                format!("已移除 {} 条失效记录", missing.len()),
                ToastKind::Info,
            );
            if let Some(current) = &mut self.health_report {
                current.missing.clear();
            }
        }
        if let Some(id) = relink_id {
            self.relink_missing_track(id);
        }
        if relink_root {
            self.relink_missing_root();
        }
        if close {
            self.health_report = None;
        }
    }
}

fn section(ui: &mut egui::Ui, title: &str, count: usize) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(format!("{title}  {count}"))
            .size(12.0)
            .strong()
            .color(if count > 0 {
                theme::c().text
            } else {
                theme::c().faint
            }),
    );
    ui.add_space(2.0);
}

fn row(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    Frame::NONE
        .fill(theme::c().card)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.label(RichText::new(title).size(11.5).color(theme::c().text));
            ui.label(RichText::new(subtitle).size(9.5).color(theme::c().faint));
        });
    ui.add_space(3.0);
}
