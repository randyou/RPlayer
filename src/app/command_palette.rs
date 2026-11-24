//! Keyboard-first action palette. A stable action list makes the same
//! operations available to keyboard users and automation without relying on
//! screen coordinates.

use eframe::egui;
use egui::{Align, Frame, Key, Layout, Margin, RichText, Stroke, Vec2};

use super::{RPlayerApp, View};
use crate::theme;
use crate::ui::widgets;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    PlayPause,
    Next,
    Previous,
    LibraryView,
    AlbumView,
    ArtistView,
    Favorites,
    Recent,
    Search,
    Settings,
    Equalizer,
    DesktopLyrics,
    MiniPlayer,
    Theme,
    Scan,
    HealthCheck,
    UndoBatchTags,
    FindDuplicates,
    ClearQueue,
    Shortcuts,
}

struct Item {
    command: Command,
    id: &'static str,
    label: &'static str,
    keywords: &'static str,
}

impl Command {
    fn group(self) -> u8 {
        match self {
            Command::LibraryView
            | Command::AlbumView
            | Command::ArtistView
            | Command::Favorites
            | Command::Recent => 0,
            Command::PlayPause | Command::Next | Command::Previous | Command::Search => 1,
            _ => 2,
        }
    }

    fn group_label(self) -> &'static str {
        match self.group() {
            0 => "跳转",
            1 => "播放与搜索",
            _ => "操作",
        }
    }
}

const ITEMS: [Item; 20] = [
    Item {
        command: Command::PlayPause,
        id: "player.play_pause",
        label: "播放 / 暂停",
        keywords: "play pause 播放 暂停",
    },
    Item {
        command: Command::Next,
        id: "player.next",
        label: "下一首",
        keywords: "next 下一首",
    },
    Item {
        command: Command::Previous,
        id: "player.previous",
        label: "上一首",
        keywords: "previous back 上一首",
    },
    Item {
        command: Command::LibraryView,
        id: "view.library",
        label: "打开本地曲库",
        keywords: "library songs 曲库 歌曲",
    },
    Item {
        command: Command::AlbumView,
        id: "view.albums",
        label: "按专辑浏览",
        keywords: "album 专辑",
    },
    Item {
        command: Command::ArtistView,
        id: "view.artists",
        label: "按艺术家浏览",
        keywords: "artist 艺术家",
    },
    Item {
        command: Command::Favorites,
        id: "view.favorites",
        label: "打开我的收藏",
        keywords: "favorite 收藏",
    },
    Item {
        command: Command::Recent,
        id: "view.recent",
        label: "打开最近播放",
        keywords: "recent 最近",
    },
    Item {
        command: Command::Search,
        id: "library.search",
        label: "聚焦搜索框",
        keywords: "search find 搜索 查找",
    },
    Item {
        command: Command::Settings,
        id: "settings.open",
        label: "打开设置",
        keywords: "settings 设置",
    },
    Item {
        command: Command::Equalizer,
        id: "equalizer.open",
        label: "打开均衡器",
        keywords: "eq equalizer 均衡器",
    },
    Item {
        command: Command::DesktopLyrics,
        id: "lyrics.desktop",
        label: "切换桌面歌词",
        keywords: "lyrics desktop 歌词",
    },
    Item {
        command: Command::MiniPlayer,
        id: "player.mini",
        label: "切换迷你播放器",
        keywords: "mini 迷你",
    },
    Item {
        command: Command::Theme,
        id: "theme.cycle",
        label: "切换浅色 / 深色主题",
        keywords: "theme dark light 主题 浅色 深色",
    },
    Item {
        command: Command::Scan,
        id: "library.scan",
        label: "重新扫描音乐库",
        keywords: "scan refresh 扫描 刷新",
    },
    Item {
        command: Command::HealthCheck,
        id: "library.health",
        label: "曲库健康检查",
        keywords: "health missing 健康 缺失 检查",
    },
    Item {
        command: Command::UndoBatchTags,
        id: "library.undo_tags",
        label: "撤销上次批量标签",
        keywords: "undo tags batch 撤销 标签 批量",
    },
    Item {
        command: Command::FindDuplicates,
        id: "library.duplicates",
        label: "查找重复歌曲",
        keywords: "duplicate duplicates 重复 查重",
    },
    Item {
        command: Command::ClearQueue,
        id: "queue.clear",
        label: "清空播放队列",
        keywords: "clear queue 清空 队列",
    },
    Item {
        command: Command::Shortcuts,
        id: "help.shortcuts",
        label: "查看键盘快捷键",
        keywords: "keyboard shortcut help 快捷键 键盘 帮助",
    },
];

impl RPlayerApp {
    pub(super) fn toggle_command_palette(&mut self) {
        self.command_open = !self.command_open;
        if self.command_open {
            self.command_anim_start = std::time::Instant::now();
            self.command_query.clear();
            self.command_index = 0;
            self.command_focus = true;
        }
    }

    pub(super) fn command_palette(&mut self, ctx: &egui::Context) {
        if !self.command_open {
            return;
        }

        let (up, down, enter, escape, home, end, page_up, page_down) = ctx.input(|input| {
            (
                input.key_pressed(Key::ArrowUp),
                input.key_pressed(Key::ArrowDown),
                input.key_pressed(Key::Enter),
                input.key_pressed(Key::Escape),
                input.key_pressed(Key::Home),
                input.key_pressed(Key::End),
                input.key_pressed(Key::PageUp),
                input.key_pressed(Key::PageDown),
            )
        });
        if escape {
            self.command_open = false;
            return;
        }
        let filtered = filtered_commands(&self.command_query);
        if filtered.is_empty() {
            self.command_index = 0;
        } else {
            self.command_index = self.command_index.min(filtered.len() - 1);
        }
        if up && !filtered.is_empty() {
            self.command_index = if self.command_index == 0 {
                filtered.len() - 1
            } else {
                self.command_index - 1
            };
        }
        if down && !filtered.is_empty() {
            self.command_index = (self.command_index + 1) % filtered.len();
        }
        let mut scroll_selected = up || down;
        if home && !filtered.is_empty() {
            self.command_index = 0;
            scroll_selected = true;
        }
        if end && !filtered.is_empty() {
            self.command_index = filtered.len() - 1;
            scroll_selected = true;
        }
        if page_up && !filtered.is_empty() {
            self.command_index = self.command_index.saturating_sub(6);
            scroll_selected = true;
        }
        if page_down && !filtered.is_empty() {
            self.command_index = (self.command_index + 6).min(filtered.len() - 1);
            scroll_selected = true;
        }

        let appear = if crate::ui::motion::reduced() {
            1.0
        } else {
            (self.command_anim_start.elapsed().as_secs_f32()
                / crate::ui::motion::FAST)
                .clamp(0.0, 1.0)
        };
        if appear < 1.0 {
            ctx.request_repaint();
        }
        let mut execute = None;
        egui::Window::new("rplayer-command-palette")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_size(Vec2::new(640.0, 430.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                Frame::NONE
                    .fill(theme::with_alpha(theme::c().panel, appear))
                    .stroke(Stroke::new(1.0_f32, theme::with_alpha(theme::c().border, appear)))
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(Margin::same(18))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 16],
                        blur: 36,
                        spread: 0,
                        color: theme::with_alpha(theme::c().shadow, appear),
                    }),
            )
            .show(ctx, |ui| {
                ui.multiply_opacity(appear);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("命令")
                            .font(crate::ui::tokens::mono(9.5))
                            .color(theme::c().faint),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("Esc 关闭")
                                .size(9.5)
                                .color(theme::c().faint),
                        );
                    });
                });
                ui.add_space(8.0);
                let response = Frame::NONE
                    .fill(theme::c().bg)
                    .stroke(Stroke::new(1.0_f32, theme::c().border_soft))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.command_query)
                                .hint_text("搜索曲目、专辑，或输入命令…")
                                .frame(false)
                                .desired_width(f32::INFINITY),
                        )
                    })
                    .inner;
                if std::mem::take(&mut self.command_focus) {
                    response.request_focus();
                }
                if response.changed() {
                    self.command_index = 0;
                    scroll_selected = true;
                }
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .max_height(292.0)
                    .show(ui, |ui| {
                        let mut last_group = None;
                        for (index, item) in filtered.iter().enumerate() {
                            if last_group != Some(item.command.group()) {
                                last_group = Some(item.command.group());
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(item.command.group_label())
                                        .font(crate::ui::tokens::mono(8.5))
                                        .color(theme::c().faint),
                                );
                                ui.add_space(2.0);
                            }
                            let selected = index == self.command_index;
                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 34.0),
                                egui::Sense::click(),
                            );
                            let response =
                                response.on_hover_cursor(egui::CursorIcon::PointingHand);
                            let painter = ui.painter();
                            if selected {
                                painter.rect_filled(
                                    rect,
                                    egui::CornerRadius::same(7),
                                    theme::accent_soft(),
                                );
                                painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        rect.min + Vec2::new(1.0, 6.0),
                                        Vec2::new(2.0, rect.height() - 12.0),
                                    ),
                                    egui::CornerRadius::same(1),
                                    theme::c().accent,
                                );
                            } else if response.hovered() {
                                painter.rect_filled(
                                    rect,
                                    egui::CornerRadius::same(7),
                                    theme::c().hover,
                                );
                            }
                            painter.text(
                                egui::Pos2::new(rect.left() + 14.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                item.label,
                                egui::FontId::proportional(12.5),
                                if selected {
                                    theme::c().accent
                                } else {
                                    theme::c().text
                                },
                            );
                            if selected {
                                painter.text(
                                    egui::Pos2::new(rect.right() - 12.0, rect.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    "Enter",
                                    crate::ui::tokens::mono(8.5),
                                    theme::c().faint,
                                );
                            }
                            if response.clicked() {
                                execute = Some(item.command);
                            }
                            if selected {
                                if scroll_selected {
                                    response.scroll_to_me(Some(egui::Align::Center));
                                }
                                if enter {
                                    execute = Some(item.command);
                                }
                            }
                        }
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    crate::ui::components::keycap(ui, "↑↓");
                    ui.label(
                        RichText::new("选择")
                            .size(9.5)
                            .color(theme::c().faint),
                    );
                    ui.add_space(8.0);
                    crate::ui::components::keycap(ui, "Enter");
                    ui.label(
                        RichText::new("执行")
                            .size(9.5)
                            .color(theme::c().faint),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} 项结果", filtered.len()))
                                .size(9.5)
                                .color(theme::c().faint),
                        );
                    });
                });
            });

        if let Some(command) = execute {
            self.command_open = false;
            self.run_command(ctx, command);
        }
    }

    fn run_command(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::PlayPause => self.toggle_play(ctx),
            Command::Next => self.next_track(ctx, false),
            Command::Previous => self.previous_track(ctx),
            Command::LibraryView => self.switch_view(View::Library),
            Command::AlbumView => self.switch_view(View::Albums),
            Command::ArtistView => self.switch_view(View::Artists),
            Command::Favorites => self.switch_view(View::Favorites),
            Command::Recent => self.switch_view(View::Recent),
            Command::Search => self.focus_search = true,
            Command::Settings => self.settings_open = !self.settings_open,
            Command::Equalizer => self.toggle_eq(),
            Command::DesktopLyrics => self.toggle_desktop_lyrics(ctx),
            Command::MiniPlayer => self.toggle_mini(ctx),
            Command::Theme => self.toggle_theme(ctx),
            Command::Scan => self.start_scan(),
            Command::HealthCheck => self.start_health_check(),
            Command::UndoBatchTags => self.undo_last_tag_batch(),
            Command::FindDuplicates => self.open_duplicates(),
            Command::ClearQueue => self.clear_queue(),
            Command::Shortcuts => self.dialog = super::Dialog::Shortcuts,
        }
    }
}

fn filtered_commands(query: &str) -> Vec<&'static Item> {
    let query = query.trim().to_lowercase();
    let mut items: Vec<&'static Item> = ITEMS
        .iter()
        .filter(|item| {
            query.is_empty()
                || item.label.to_lowercase().contains(&query)
                || item.keywords.contains(&query)
                || item.id.contains(&query)
        })
        .collect();
    items.sort_by_key(|item| item.command.group());
    items
}

#[cfg(test)]
mod tests {
    use super::ITEMS;
    use std::collections::HashSet;

    #[test]
    fn command_ids_are_unique() {
        let mut ids = HashSet::new();
        for item in ITEMS {
            assert!(ids.insert(item.id), "duplicate command id: {}", item.id);
        }
    }

    #[test]
    fn command_groups_are_ordered_jump_play_action() {
        use super::Command;
        assert_eq!(Command::LibraryView.group(), 0);
        assert_eq!(Command::PlayPause.group(), 1);
        assert_eq!(Command::Settings.group(), 2);
        assert_eq!(Command::Search.group(), 1);
    }

    #[test]
    fn command_group_labels_are_distinct() {
        use super::Command;
        let labels = [
            Command::LibraryView.group_label(),
            Command::PlayPause.group_label(),
            Command::Settings.group_label(),
        ];
        assert_eq!(labels[0], "跳转");
        assert_eq!(labels[1], "播放与搜索");
        assert_eq!(labels[2], "操作");
    }
}
