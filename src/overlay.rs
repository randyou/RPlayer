//! Always-on-top desktop lyrics and mini player viewports.
//!
//! The main app owns playback; these windows are pure views. They publish a
//! snapshot through a mutex and send tiny commands back through a queue.

use std::sync::{Arc, Mutex};

use eframe::egui;
use egui::{
    Align, Align2, Color32, ColorImage, CornerRadius, FontId, Frame, Id, Layout, Pos2, Rect,
    RichText, Sense, Stroke, TextureHandle, Vec2, ViewportBuilder, ViewportCommand, ViewportId,
};

use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

const LYRICS_SIZE: Vec2 = Vec2::new(860.0, 158.0);
const MINI_SIZE: Vec2 = Vec2::new(410.0, 146.0);

#[derive(Clone, Default)]
pub struct OverlayState {
    pub has_track: bool,
    pub title: String,
    pub artist: String,
    pub position: f64,
    pub duration: f64,
    pub playing: bool,
    pub favorite: bool,
    pub lyric_previous: String,
    pub lyric_current: String,
    pub lyric_next: String,
    pub has_lyrics: bool,
    pub lyrics_opacity: f32,
    pub lyrics_font: f32,
    pub lyrics_locked: bool,
    pub cover_id: Option<u64>,
    pub cover: Option<Arc<ColorImage>>,
}

#[derive(Debug, Clone, Copy)]
pub enum OverlayCommand {
    TogglePlay,
    Previous,
    Next,
    ToggleFavorite,
    Seek(f64),
    SetLyricsOpacity(f32),
    SetLyricsFont(f32),
    SetLyricsLocked(bool),
    CloseLyrics,
    CloseMini,
    OpenMain,
}

pub fn lyrics_viewport() -> ViewportId {
    ViewportId::from_hash_of("rplayer-desktop-lyrics")
}

pub fn mini_viewport() -> ViewportId {
    ViewportId::from_hash_of("rplayer-mini-player")
}

#[derive(Default)]
pub struct MiniCache {
    cover_id: Option<u64>,
    texture: Option<TextureHandle>,
}

pub fn show_lyrics(
    ctx: &egui::Context,
    state: Arc<Mutex<OverlayState>>,
    commands: Arc<Mutex<Vec<OverlayCommand>>>,
    initial_position: Option<Pos2>,
) {
    let mut builder = ViewportBuilder::default()
        .with_title("RPlayer 桌面歌词")
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_resizable(false)
        .with_inner_size(LYRICS_SIZE)
        .with_min_inner_size(LYRICS_SIZE)
        .with_taskbar(false)
        .with_active(false);
    if let Some(position) = initial_position {
        builder = builder.with_position(position);
    }

    ctx.show_viewport_deferred(lyrics_viewport(), builder, move |ctx, _class| {
        if ctx.input(|input| input.viewport().close_requested()) {
            if let Ok(mut queue) = commands.lock() {
                queue.push(OverlayCommand::CloseLyrics);
            }
            return;
        }

        let snapshot = state.lock().map(|state| state.clone()).unwrap_or_default();
        let mut close = false;
        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                ui.set_min_size(LYRICS_SIZE);
                let capsule = ui.max_rect();
                let dark = theme::c().dark;
                ui.painter()
                    .rect_filled(capsule, CornerRadius::ZERO, theme::c().panel);
                ui.painter().rect_stroke(
                    capsule.shrink(1.0),
                    CornerRadius::ZERO,
                    Stroke::new(
                        1.0_f32,
                        if dark {
                            theme::white_alpha(0.10)
                        } else {
                            theme::black_alpha(0.08)
                        },
                    ),
                    egui::StrokeKind::Inside,
                );

                let shadow = if dark {
                    theme::black_alpha(0.85)
                } else {
                    theme::white_alpha(0.95)
                };
                let center_x = capsule.center().x;

                if !snapshot.has_track {
                    shadow_text(
                        ui.painter(),
                        Pos2::new(center_x, capsule.center().y - 10.0),
                        "RPlayer 桌面歌词",
                        FontId::proportional(22.0),
                        theme::c().text,
                        shadow,
                    );
                    shadow_text(
                        ui.painter(),
                        Pos2::new(center_x, capsule.center().y + 22.0),
                        "播放歌曲后，歌词会在这里逐行滚动",
                        FontId::proportional(12.0),
                        theme::c().muted,
                        shadow,
                    );
                } else {
                    let lyric = if snapshot.lyric_current.trim().is_empty() {
                        if snapshot.has_lyrics {
                            "♪"
                        } else {
                            "暂无歌词"
                        }
                    } else {
                        snapshot.lyric_current.as_str()
                    };
                    if !snapshot.has_lyrics {
                        shadow_text(
                            ui.painter(),
                            Pos2::new(
                                center_x,
                                capsule.top() + 31.0,
                            ),
                            &format!("{} · {}", snapshot.title, snapshot.artist),
                            FontId::proportional(12.5),
                            theme::c().faint,
                            shadow,
                        );
                    } else if !snapshot.lyric_previous.trim().is_empty() {
                        shadow_text(
                            ui.painter(),
                            Pos2::new(center_x, capsule.top() + 31.0),
                            &snapshot.lyric_previous,
                            FontId::proportional(12.5),
                            theme::c().faint,
                            shadow,
                        );
                    }
                    shadow_text(
                        ui.painter(),
                        Pos2::new(center_x, capsule.center().y + 1.0),
                        lyric,
                        FontId::proportional(snapshot.lyrics_font),
                        theme::c().accent,
                        shadow,
                    );
                    if !snapshot.has_lyrics {
                        shadow_text(
                            ui.painter(),
                            Pos2::new(center_x, capsule.bottom() - 27.0),
                            "将同名 .lrc 放在歌曲目录，RPlayer 会自动加载",
                            FontId::proportional(11.0),
                            theme::c().faint,
                            shadow,
                        );
                    } else if !snapshot.lyric_next.trim().is_empty() {
                        shadow_text(
                            ui.painter(),
                            Pos2::new(center_x, capsule.bottom() - 27.0),
                            &snapshot.lyric_next,
                            FontId::proportional(12.5),
                            theme::c().faint,
                            shadow,
                        );
                    }
                }

                let close_rect = Rect::from_min_size(
                    Pos2::new(capsule.right() - 36.0, capsule.top() + 10.0),
                    Vec2::splat(24.0),
                );
                let close_response = ui.interact(close_rect, Id::new("lyrics-close"), Sense::click());
                let close_hovered = close_response.hovered();
                if close_hovered {
                    ui.painter().circle_filled(
                        close_rect.center(),
                        12.0,
                        theme::white_alpha(0.10),
                    );
                }
                icons::paint(
                    ui.painter(),
                    close_rect.shrink(6.0),
                    Icon::Close,
                    if close_hovered {
                        theme::c().text
                    } else {
                        theme::c().muted
                    },
                );
                if close_response.clicked() {
                    close = true;
                }

                let drag = ui.interact(
                    Rect::from_min_max(
                        capsule.min,
                        Pos2::new(capsule.right() - 44.0, capsule.bottom()),
                    ),
                    Id::new("lyrics-drag"),
                    Sense::drag(),
                );
                if drag.dragged() && !snapshot.lyrics_locked {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                let mut opacity = snapshot.lyrics_opacity;
                let mut font = snapshot.lyrics_font;
                let mut locked = snapshot.lyrics_locked;
                drag.on_hover_cursor(if snapshot.lyrics_locked {
                    egui::CursorIcon::Default
                } else {
                    egui::CursorIcon::Grab
                })
                    .context_menu(|ui| {
                        ui.label(
                            RichText::new("背景透明度")
                                .size(11.0)
                                .color(theme::c().muted),
                        );
                        let response = ui.add(
                            egui::Slider::new(&mut opacity, 0.35..=1.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        ui.label(
                            RichText::new(format!("{:.0}%", opacity * 100.0))
                                .size(10.0)
                                .color(theme::c().faint),
                        );
                        if response.changed() {
                            if let Ok(mut queue) = commands.lock() {
                                queue.push(OverlayCommand::SetLyricsOpacity(opacity));
                            }
                        }
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new("字号")
                                .size(11.0)
                                .color(theme::c().muted),
                        );
                        let font_response = ui.add(
                            egui::Slider::new(&mut font, 18.0..=40.0)
                                .show_value(true)
                                .trailing_fill(true),
                        );
                        if font_response.changed() {
                            if let Ok(mut queue) = commands.lock() {
                                queue.push(OverlayCommand::SetLyricsFont(font));
                            }
                        }
                        if ui.checkbox(&mut locked, "锁定位置").changed() {
                            if let Ok(mut queue) = commands.lock() {
                                queue.push(OverlayCommand::SetLyricsLocked(locked));
                            }
                        }
                        ui.separator();
                        if ui.button("关闭桌面歌词").clicked() {
                            close = true;
                            ui.close_menu();
                        }
                    });
            });

        if close {
            if let Ok(mut queue) = commands.lock() {
                queue.push(OverlayCommand::CloseLyrics);
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    });
}

pub fn show_mini(
    ctx: &egui::Context,
    state: Arc<Mutex<OverlayState>>,
    commands: Arc<Mutex<Vec<OverlayCommand>>>,
    cache: Arc<Mutex<MiniCache>>,
    initial_position: Option<Pos2>,
) {
    let mut builder = ViewportBuilder::default()
        .with_title("RPlayer 迷你播放器")
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_resizable(false)
        .with_inner_size(MINI_SIZE)
        .with_min_inner_size(MINI_SIZE)
        .with_taskbar(false)
        .with_active(false);
    if let Some(position) = initial_position {
        builder = builder.with_position(position);
    }

    ctx.show_viewport_deferred(mini_viewport(), builder, move |ctx, _class| {
        if ctx.input(|input| input.viewport().close_requested()) {
            if let Ok(mut queue) = commands.lock() {
                queue.push(OverlayCommand::CloseMini);
            }
            return;
        }

        let snapshot = state.lock().map(|state| state.clone()).unwrap_or_default();
        let mut close = false;
        let mut open_main = false;
        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                let card = ui.max_rect();
                ui.painter()
                    .rect_filled(card, CornerRadius::ZERO, theme::c().panel);
                ui.painter().rect_stroke(
                    card.shrink(1.0),
                    CornerRadius::ZERO,
                    Stroke::new(1.0_f32, theme::c().border),
                    egui::StrokeKind::Inside,
                );
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        Pos2::new(card.left() + 1.0, card.top() + 14.0),
                        Pos2::new(card.left() + 5.0, card.bottom() - 14.0),
                    ),
                    CornerRadius::same(2),
                    theme::c().accent,
                );

                ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(card.shrink2(Vec2::new(14.0, 12.0)))
                        .layout(Layout::top_down(Align::LEFT)),
                    |ui| {
                        ui.horizontal(|ui| {
                            let (cover_rect, cover) =
                                ui.allocate_exact_size(Vec2::splat(92.0), Sense::drag());
                            if cover.dragged() {
                                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                            }
                            draw_cover(ui, cover_rect, &snapshot, &cache);

                            ui.add_space(10.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 1.0;
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(if snapshot.has_track {
                                                    &snapshot.title
                                                } else {
                                                    "RPlayer 迷你播放器"
                                                })
                                                .size(14.0)
                                                .strong()
                                                .color(theme::c().text),
                                            )
                                            .truncate(),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(if snapshot.has_track {
                                                    &snapshot.artist
                                                } else {
                                                    "从音乐库选择一首歌"
                                                })
                                                .size(11.0)
                                                .color(theme::c().muted),
                                            )
                                            .truncate(),
                                        );
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                                        if widgets::icon_button(ui, Icon::Close, 22.0, "关闭迷你播放器")
                                            .clicked()
                                        {
                                            close = true;
                                        }
                                        if widgets::icon_button(
                                            ui,
                                            Icon::Expand,
                                            22.0,
                                            "打开主窗口",
                                        )
                                        .clicked()
                                        {
                                            open_main = true;
                                        }
                                    });
                                });

                                ui.add_space(4.0);
                                if let Some(seconds) =
                                    widgets::seek_bar(ui, snapshot.position, snapshot.duration)
                                {
                                    if let Ok(mut queue) = commands.lock() {
                                        queue.push(OverlayCommand::Seek(seconds));
                                    }
                                }
                                ui.add_space(1.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(crate::model::format_duration(
                                            snapshot.position,
                                        ))
                                        .font(FontId::monospace(9.0))
                                        .color(theme::c().faint),
                                    );
                                    ui.with_layout(
                                        Layout::right_to_left(Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new(crate::model::format_duration(
                                                    snapshot.duration,
                                                ))
                                                .font(FontId::monospace(9.0))
                                                .color(theme::c().faint),
                                            );
                                        },
                                    );
                                });

                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 5.0;
                                    if widgets::circle_button(
                                        ui,
                                        if snapshot.playing {
                                            Icon::Pause
                                        } else {
                                            Icon::Play
                                        },
                                        32.0,
                                        theme::c().accent,
                                        theme::c().on_accent,
                                        if snapshot.playing { "暂停" } else { "播放" },
                                    )
                                    .clicked()
                                    {
                                        if let Ok(mut queue) = commands.lock() {
                                            queue.push(OverlayCommand::TogglePlay);
                                        }
                                    }
                                    if widgets::icon_button(ui, Icon::Prev, 26.0, "上一首").clicked()
                                    {
                                        if let Ok(mut queue) = commands.lock() {
                                            queue.push(OverlayCommand::Previous);
                                        }
                                    }
                                    if widgets::icon_button(ui, Icon::Next, 26.0, "下一首").clicked()
                                    {
                                        if let Ok(mut queue) = commands.lock() {
                                            queue.push(OverlayCommand::Next);
                                        }
                                    }
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if widgets::icon_button_tinted(
                                            ui,
                                            if snapshot.favorite {
                                                Icon::HeartFilled
                                            } else {
                                                Icon::Heart
                                            },
                                            26.0,
                                            theme::c().muted,
                                            snapshot.favorite,
                                            "收藏",
                                        )
                                        .clicked()
                                        {
                                            if let Ok(mut queue) = commands.lock() {
                                                queue.push(OverlayCommand::ToggleFavorite);
                                            }
                                        }
                                    });
                                });
                            });
                        });
                    },
                );
            });

        if close {
            if let Ok(mut queue) = commands.lock() {
                queue.push(OverlayCommand::CloseMini);
            }
        }
        if open_main {
            if let Ok(mut queue) = commands.lock() {
                queue.push(OverlayCommand::OpenMain);
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    });
}

fn draw_cover(
    ui: &mut egui::Ui,
    rect: Rect,
    state: &OverlayState,
    cache: &Arc<Mutex<MiniCache>>,
) {
    let mut cache = cache.lock().unwrap_or_else(|error| error.into_inner());
    if cache.cover_id != state.cover_id {
        cache.cover_id = state.cover_id;
        cache.texture = state.cover.as_ref().map(|image| {
            ui.ctx().load_texture(
                "rplayer-mini-cover",
                (**image).clone(),
                egui::TextureOptions::LINEAR,
            )
        });
    }

    if let Some(texture) = &cache.texture {
        ui.painter().add(egui::Shape::image(
            texture.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        ));
    } else {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(12),
            theme::c().card,
        );
        icons::paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), Vec2::splat(34.0)),
            Icon::Music,
            theme::c().faint,
        );
    }
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(1.0_f32, theme::c().border_soft),
        egui::StrokeKind::Inside,
    );
}

fn shadow_text(
    painter: &egui::Painter,
    pos: Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    shadow: Color32,
) {
    painter.text(
        pos + Vec2::new(1.0, 2.0),
        Align2::CENTER_CENTER,
        text,
        font.clone(),
        shadow,
    );
    painter.text(pos, Align2::CENTER_CENTER, text, font, color);
}
