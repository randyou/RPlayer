//! Mini player viewport: 410x146 always-on-top card in classic or compact style.

use std::sync::{Arc, Mutex};

use eframe::egui;
use egui::{
    Align, CornerRadius, Frame, Id, Layout, Pos2, Rect, RichText,
    Sense, Stroke, Vec2, ViewportBuilder, ViewportCommand,
};

use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

use super::{mini_viewport, MiniCache, OverlayCommand, OverlayState, MINI_SIZE};

pub fn show_mini(
    ctx: &egui::Context,
    state: Arc<Mutex<OverlayState>>,
    commands: Arc<Mutex<Vec<OverlayCommand>>>,
    cache: Arc<Mutex<MiniCache>>,
    initial_position: Option<Pos2>,
    always_on_top: bool,
) {
    let mut builder = ViewportBuilder::default()
        .with_title("RPlayer 迷你播放器")
        .with_decorations(false)
        .with_transparent(true)
        .with_window_level(if always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        })
        .with_resizable(false)
        .with_inner_size(MINI_SIZE)
        .with_min_inner_size(MINI_SIZE)
        .with_max_inner_size(MINI_SIZE)
        .with_taskbar(false);
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
        if let Some(rect) = ctx.input(|input| input.viewport().inner_rect) {
            if (rect.width() - MINI_SIZE.x).abs() > 2.0
                || (rect.height() - MINI_SIZE.y).abs() > 2.0
            {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(MINI_SIZE));
            }
        }
        let mut close = false;
        let mut open_main = false;
        // Spec D15: the 4x24 action group sits at 55% until the window is
        // hovered, then fades to 100% in 140ms. Hit-testing ignores opacity, so
        // the buttons stay clickable while faint.
        let hover_t = ctx.animate_bool_with_time(
            Id::new("mini-actions-hover"),
            ctx.input(|input| input.pointer.hover_pos().is_some()),
            0.14,
        );
        let action_alpha = 0.55 + 0.45 * hover_t;
        let mut dragging = false;
        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                let card = ui.max_rect();
                let card_shape = card;
                // Fill the whole client area first; otherwise the outermost
                // pixel shows the black window background as a border.
                ui.painter().rect_filled(card, CornerRadius::ZERO, theme::c().panel);
                ui.painter().rect_filled(
                    card_shape,
                    CornerRadius::same(16),
                    theme::c().panel,
                );
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        Pos2::new(card.left(), card.top() + 12.0),
                        Pos2::new(card.left() + 5.0, card.bottom() - 12.0),
                    ),
                    CornerRadius::same(3),
                    theme::c().accent,
                );

                if snapshot.mini_style == "compact" {
                    draw_mini_compact(
                        ui,
                        &snapshot,
                        &commands,
                        &mut close,
                        &mut open_main,
                        action_alpha,
                    );
                } else {
                ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(card.shrink2(Vec2::new(14.0, 12.0)))
                        .layout(Layout::top_down(Align::LEFT)),
                    |ui| {
                        ui.horizontal(|ui| {
                            let (cover_rect, cover) =
                                ui.allocate_exact_size(Vec2::splat(92.0), Sense::drag());
                            if cover.dragged() {
                                dragging = true;
                                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                            }
                            draw_cover(ui, cover_rect, &snapshot, &cache);

                            ui.add_space(10.0);
                            // Classic keeps a 92px cover, so the title can only use
                            // what is left after the 4x24 action strip (120) and
                            // one gap; 250 stays the absolute cap from spec D15.
                            let title_max = (ui.available_width() - 128.0).clamp(80.0, 250.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 1.0;
                                        // Spec D15: one line, never wider than 250px.
                                        ui.set_max_width(title_max);
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(if snapshot.has_track {
                                                    &snapshot.title
                                                } else {
                                                    "RPlayer 迷你播放器"
                                                })
                                                .size(14.5)
                                                .strong()
                                                .color(theme::c().text),
                                            )
                                            // The title area doubles as the window drag
                                            // handle; text selection would steal the drag.
                                            .truncate()
                                            .selectable(false),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(if snapshot.has_track {
                                                    mini_subtitle(&snapshot)
                                                } else {
                                                    "从音乐库选择一首歌".to_string()
                                                })
                                                .size(9.0)
                                                .color(theme::c().muted),
                                            )
                                            .truncate()
                                            .selectable(false),
                                        );
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                                        ui.multiply_opacity(action_alpha);
                                        ui.spacing_mut().item_spacing.x = 8.0;
                                        if widgets::icon_button(ui, Icon::Close, 24.0, "关闭迷你播放器")
                                            .clicked()
                                        {
                                            close = true;
                                        }
                                        if widgets::icon_button(
                                            ui,
                                            Icon::Expand,
                                            24.0,
                                            "切回主播放器",
                                        )
                                        .clicked()
                                        {
                                            open_main = true;
                                        }
                                        if widgets::glyph_button_tinted(
                                            ui,
                                            "词",
                                            24.0,
                                            theme::c().muted,
                                            snapshot.desktop_lyrics_open,
                                            "显示 / 隐藏桌面歌词",
                                        )
                                        .clicked()
                                        {
                                            if let Ok(mut queue) = commands.lock() {
                                                queue.push(OverlayCommand::ToggleLyrics);
                                            }
                                        }
                                        if widgets::icon_button_tinted(
                                            ui,
                                            Icon::Pin,
                                            24.0,
                                            theme::c().muted,
                                            snapshot.mini_on_top,
                                            if snapshot.mini_on_top {
                                                "取消始终置顶"
                                            } else {
                                                "始终显示在最上层"
                                            },
                                        )
                                        .clicked()
                                        {
                                            if let Ok(mut queue) = commands.lock() {
                                                queue.push(OverlayCommand::SetMiniOnTop(
                                                    !snapshot.mini_on_top,
                                                ));
                                            }
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

                            });
                        });
                    },
                );
                }

                // Transport sits on the bottom padding of the card, under the
                // cover column, so it does not depend on how tall the text stack
                // turned out. Spec: 28px round play, 26px boxes either side.
                if snapshot.mini_style != "compact" {
                    let transport_rect = Rect::from_min_size(
                        Pos2::new(card.left() + 112.0, card.bottom() - 12.0 - 28.0),
                        Vec2::new((card.width() - 112.0 - 14.0).max(80.0), 28.0),
                    );
                    ui.allocate_new_ui(
                        egui::UiBuilder::new()
                            .max_rect(transport_rect)
                            .layout(Layout::left_to_right(Align::Center)),
                        |ui| {
                            mini_transport(ui, &snapshot, &commands);
                        },
                    );
                }

                // Title/artist drag strip: avoids covering transport buttons or
                // the seek bar, while the cover remains draggable as well.
                let drag_rect = if snapshot.mini_style == "compact" {
                    Rect::from_min_size(
                        card.min + Vec2::new(12.0, 6.0),
                        Vec2::new((card.width() - 142.0).max(80.0), 66.0),
                    )
                } else {
                    Rect::from_min_size(
                        card.min + Vec2::new(112.0, 10.0),
                        Vec2::new((card.width() - 242.0).max(80.0), 40.0),
                    )
                };
                let drag = ui.interact(drag_rect, Id::new("mini-title-drag"), Sense::drag());
                if drag.dragged() {
                    dragging = true;
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag.drag_stopped() {
                    let pos = ctx
                        .input(|input| input.viewport().outer_rect)
                        .map(|rect| rect.min)
                        .unwrap_or(card.min);
                    snap_and_store_mini(ctx, &snapshot, &commands, pos, card.size());
                }
                if dragging {
                    paint_drag_outline(ui, card);
                }
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
        // Commands are handled by the (possibly minimized) main viewport.
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    });
}

/// Spec D15: while the window is being dragged, outline the card with a 1px
/// dashed accent border (radius 16, inset by half a pixel so the stroke lands
/// fully inside the client area).
fn paint_drag_outline(ui: &egui::Ui, card: Rect) {
    fn arc(points: &mut Vec<Pos2>, center: Pos2, start: f32, radius: f32) {
        for step in 0..=6 {
            let angle = start + step as f32 / 6.0 * std::f32::consts::FRAC_PI_2;
            points.push(center + Vec2::angled(angle) * radius);
        }
    }

    let rect = card.shrink(0.5);
    let radius = 15.5;
    let mut points = Vec::with_capacity(32);
    points.push(Pos2::new(rect.left() + radius, rect.top()));
    points.push(Pos2::new(rect.right() - radius, rect.top()));
    arc(
        &mut points,
        Pos2::new(rect.right() - radius, rect.top() + radius),
        -std::f32::consts::FRAC_PI_2,
        radius,
    );
    points.push(Pos2::new(rect.right(), rect.bottom() - radius));
    arc(
        &mut points,
        Pos2::new(rect.right() - radius, rect.bottom() - radius),
        0.0,
        radius,
    );
    points.push(Pos2::new(rect.left() + radius, rect.bottom()));
    arc(
        &mut points,
        Pos2::new(rect.left() + radius, rect.bottom() - radius),
        std::f32::consts::FRAC_PI_2,
        radius,
    );
    points.push(Pos2::new(rect.left(), rect.top() + radius));
    arc(
        &mut points,
        Pos2::new(rect.left() + radius, rect.top() + radius),
        std::f32::consts::PI,
        radius,
    );
    points.push(points[0]);
    for shape in egui::Shape::dashed_line(
        &points,
        egui::Stroke::new(1.0_f32, theme::c().accent),
        4.0,
        3.0,
    ) {
        ui.painter().add(shape);
    }
}

fn clamp_mini_position(ctx: &egui::Context, pos: Pos2, size: Vec2) -> Pos2 {
    let monitor = ctx
        .input(|input| input.viewport().monitor_size)
        .unwrap_or(egui::vec2(1920.0, 1080.0));
    Pos2::new(
        pos.x.clamp(0.0, (monitor.x - size.x).max(0.0)),
        pos.y.clamp(0.0, (monitor.y - size.y).max(0.0)),
    )
}

fn snap_and_store_mini(
    ctx: &egui::Context,
    snapshot: &OverlayState,
    commands: &Arc<Mutex<Vec<OverlayCommand>>>,
    pos: Pos2,
    size: Vec2,
) {
    let monitor = ctx
        .input(|input| input.viewport().monitor_size)
        .unwrap_or(egui::vec2(1920.0, 1080.0));
    let mut pos = clamp_mini_position(ctx, pos, size);
    if snapshot.mini_snap {
        let threshold = 28.0;
        if pos.x < threshold {
            pos.x = 0.0;
        } else if (monitor.x - (pos.x + size.x)).abs() < threshold {
            pos.x = (monitor.x - size.x).max(0.0);
        }
        if pos.y < threshold {
            pos.y = 0.0;
        } else if (monitor.y - (pos.y + size.y)).abs() < threshold {
            pos.y = (monitor.y - size.y).max(0.0);
        }
    }
    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos));
    if let Ok(mut queue) = commands.lock() {
        queue.push(OverlayCommand::SetMiniPosition(pos.x, pos.y));
    }
}

/// `艺术家 · 编号`, with `未知艺术家` when the tag is empty (mini player spec).
fn mini_subtitle(snapshot: &OverlayState) -> String {
    let artist = snapshot.artist.trim();
    let artist = if artist.is_empty() {
        "未知艺术家"
    } else {
        artist
    };
    match snapshot.track_number {
        Some(number) => format!("{artist} · {number:02}"),
        None => artist.to_string(),
    }
}

/// Transport row of the mini player: 28px round play, prev/next boxes either
/// side, favourite pinned to the right edge.
fn mini_transport(
    ui: &mut egui::Ui,
    snapshot: &OverlayState,
    commands: &Arc<Mutex<Vec<OverlayCommand>>>,
) {
    ui.spacing_mut().item_spacing.x = 5.0;
    if widgets::circle_button(
        ui,
        if snapshot.playing {
            Icon::Pause
        } else {
            Icon::Play
        },
        28.0,
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
    ui.add_space(6.0);
    if widgets::icon_button(ui, Icon::Prev, 26.0, "上一首").clicked() {
        if let Ok(mut queue) = commands.lock() {
            queue.push(OverlayCommand::Previous);
        }
    }
    if widgets::icon_button(ui, Icon::Next, 26.0, "下一首").clicked() {
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
}

fn draw_mini_compact(
    ui: &mut egui::Ui,
    snapshot: &OverlayState,
    commands: &Arc<Mutex<Vec<OverlayCommand>>>,
    close: &mut bool,
    open_main: &mut bool,
    action_alpha: f32,
) {
    let card = ui.max_rect();

    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(card.shrink2(Vec2::new(12.0, 10.0)))
            .layout(Layout::top_down(Align::Center)),
        |ui| {
            ui.horizontal(|ui| {
                // No caption: the compact header is just the four actions on
                // the right, exactly as the design sheet draws it.
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.multiply_opacity(action_alpha);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if widgets::icon_button(ui, Icon::Close, 24.0, "关闭迷你播放器").clicked() {
                        *close = true;
                    }
                    if widgets::icon_button(ui, Icon::Expand, 24.0, "切回主播放器").clicked() {
                        *open_main = true;
                    }
                    if widgets::glyph_button_tinted(
                        ui,
                        "词",
                        24.0,
                        theme::c().muted,
                        snapshot.desktop_lyrics_open,
                        "显示 / 隐藏桌面歌词",
                    )
                    .clicked()
                    {
                        if let Ok(mut queue) = commands.lock() {
                            queue.push(OverlayCommand::ToggleLyrics);
                        }
                    }
                    if widgets::icon_button_tinted(
                        ui,
                        Icon::Pin,
                        24.0,
                        theme::c().muted,
                        snapshot.mini_on_top,
                        if snapshot.mini_on_top {
                            "取消始终置顶"
                        } else {
                            "始终显示在最上层"
                        },
                    )
                    .clicked()
                    {
                        if let Ok(mut queue) = commands.lock() {
                            queue.push(OverlayCommand::SetMiniOnTop(!snapshot.mini_on_top));
                        }
                    }
                });
            });

            ui.add_space(2.0);
            // Spec: single line, never wider than 250px; the hover text carries
            // the full title when it had to be elided.
            let title_text = if snapshot.has_track {
                snapshot.title.clone()
            } else {
                "RPlayer 迷你播放器".to_string()
            };
            ui.allocate_ui_with_layout(
                Vec2::new(250.0, 18.0),
                Layout::top_down(Align::Center),
                |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&title_text)
                                .size(14.5)
                                .strong()
                                .color(theme::c().text),
                        )
                        .truncate()
                        .selectable(false),
                    )
                    .on_hover_text(&title_text);
                },
            );
            ui.add(
                egui::Label::new(
                    RichText::new(if snapshot.has_track {
                        mini_subtitle(snapshot)
                    } else {
                        "从音乐库选择一首歌".to_string()
                    })
                    .size(9.0)
                    .color(theme::c().muted),
                )
                .truncate()
                .selectable(false),
            );

            ui.add_space(3.0);
            if let Some(seconds) = widgets::seek_bar(ui, snapshot.position, snapshot.duration) {
                if let Ok(mut queue) = commands.lock() {
                    queue.push(OverlayCommand::Seek(seconds));
                }
            }

            ui.add_space(3.0);
            let width = ui.available_width();
            // Spec: 28px round play, 26px boxes with 12px glyphs either side.
            // The favourite sits alone in the bottom-right corner.
            let transport = 26.0 + 10.0 + 28.0 + 10.0 + 26.0;
            ui.allocate_ui_with_layout(
                Vec2::new(width, 32.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.add_space(((width - transport) / 2.0).max(0.0));
                    if widgets::icon_button(ui, Icon::Prev, 26.0, "上一首").clicked() {
                        if let Ok(mut queue) = commands.lock() {
                            queue.push(OverlayCommand::Previous);
                        }
                    }
                    ui.add_space(10.0);
                    if widgets::circle_button(
                        ui,
                        if snapshot.playing {
                            Icon::Pause
                        } else {
                            Icon::Play
                        },
                        28.0,
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
                    ui.add_space(10.0);
                    if widgets::icon_button(ui, Icon::Next, 26.0, "下一首").clicked() {
                        if let Ok(mut queue) = commands.lock() {
                            queue.push(OverlayCommand::Next);
                        }
                    }
                },
            );

            let favorite_rect = Rect::from_min_size(
                Pos2::new(card.right() - 12.0 - 26.0, card.bottom() - 12.0 - 26.0),
                Vec2::splat(26.0),
            );
            ui.allocate_new_ui(
                egui::UiBuilder::new()
                    .max_rect(favorite_rect)
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| {
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
                },
            );
        },
    );
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
            crate::theme::ON_MEDIA,
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

