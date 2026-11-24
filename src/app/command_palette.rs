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
    ClearQueue,
}

struct Item {
    command: Command,
    id: &'static str,
    label: &'static str,
    keywords: &'static str,
}

const ITEMS: [Item; 16] = [
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
        command: Command::ClearQueue,
        id: "queue.clear",
        label: "清空播放队列",
        keywords: "clear queue 清空 队列",
    },
];

impl RPlayerApp {
    pub(super) fn toggle_command_palette(&mut self) {
        self.command_open = !self.command_open;
        if self.command_open {
            self.command_query.clear();
            self.command_index = 0;
            self.command_focus = true;
        }
    }

    pub(super) fn command_palette(&mut self, ctx: &egui::Context) {
        if !self.command_open {
            return;
        }

        let (up, down, enter, escape) = ctx.input(|input| {
            (
                input.key_pressed(Key::ArrowUp),
                input.key_pressed(Key::ArrowDown),
                input.key_pressed(Key::Enter),
                input.key_pressed(Key::Escape),
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

        let mut execute = None;
        egui::Window::new("rplayer-command-palette")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_size(Vec2::new(520.0, 360.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                Frame::NONE
                    .fill(theme::c().panel)
                    .stroke(Stroke::new(1.0_f32, theme::c().border))
                    .corner_radius(egui::CornerRadius::same(16))
                    .inner_margin(Margin::same(16))
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
                        RichText::new("命令")
                            .size(11.0)
                            .strong()
                            .color(theme::c().faint),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("↑↓ 选择 · Enter 执行 · Esc 关闭")
                                .size(9.5)
                                .color(theme::c().faint),
                        );
                    });
                });
                ui.add_space(8.0);
                let response = Frame::NONE
                    .fill(theme::c().card)
                    .stroke(Stroke::new(1.0_f32, theme::c().border))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.command_query)
                                .hint_text("输入命令，例如：专辑、设置、播放…")
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
                }
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .show(ui, |ui| {
                        for (index, item) in filtered.iter().enumerate() {
                            let selected = index == self.command_index;
                            let response = ui.add(
                                egui::Button::new(
                                    RichText::new(format!("{}  ·  {}", item.label, item.id))
                                        .size(12.0)
                                        .color(if selected {
                                            theme::c().accent
                                        } else {
                                            theme::c().text
                                        }),
                                )
                                .fill(if selected {
                                    theme::accent_alpha(0.12)
                                } else {
                                    egui::Color32::TRANSPARENT
                                })
                                .stroke(Stroke::NONE)
                                .corner_radius(8)
                                .min_size(Vec2::new(ui.available_width(), 30.0)),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                            if response.clicked() {
                                execute = Some(item.command);
                            }
                            if selected && enter {
                                execute = Some(item.command);
                            }
                        }
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
            Command::ClearQueue => self.clear_queue(),
        }
    }
}

fn filtered_commands(query: &str) -> Vec<&'static Item> {
    let query = query.trim().to_lowercase();
    ITEMS
        .iter()
        .filter(|item| {
            query.is_empty()
                || item.label.to_lowercase().contains(&query)
                || item.keywords.contains(&query)
                || item.id.contains(&query)
        })
        .collect()
}
