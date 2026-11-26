//! Desktop lyrics viewport: transparent, always-on-top, click-through unless
//! hovered. It only renders the snapshot; playback stays in the main window.

use std::sync::{Arc, Mutex};

use eframe::egui;
use egui::{
    Align2, CornerRadius, FontId, Frame, Id, Margin, Pos2, Rect, RichText,
    Sense, Stroke, Vec2, ViewportBuilder, ViewportCommand,
};

use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::menu;
use crate::ui::widgets;

use super::{lyrics_viewport, shadow_text, with_alpha, LYRICS_SIZE, OverlayCommand, OverlayState};

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
        .with_max_inner_size(LYRICS_SIZE)
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

        crate::platform::set_window_color_key("RPlayer 桌面歌词", true);
        if let Some(rect) = ctx.input(|input| input.viewport().inner_rect) {
            if (rect.width() - LYRICS_SIZE.x).abs() > 2.0
                || (rect.height() - LYRICS_SIZE.y).abs() > 2.0
            {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(LYRICS_SIZE));
            }
        }
        // The strip follows the app theme: force-feeding the Forest preset here
        // made the strip (and its opacity panel) disagree with the rest of the
        // window whenever the theme changed mid-session.
        let snapshot = state.lock().map(|state| state.clone()).unwrap_or_default();
        let mut close = false;
        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                ui.set_min_size(LYRICS_SIZE);
                let capsule = ui.max_rect();
                let dark = theme::c().dark;
                let hover = snapshot.lyrics_opacity <= 0.001
                    && crate::platform::cursor_in_window("RPlayer 桌面歌词");
                let panel_alpha = if snapshot.lyrics_opacity <= 0.001 {
                    if hover {
                        0.78
                    } else {
                        0.0
                    }
                } else {
                    snapshot.lyrics_opacity.clamp(0.0, 1.0)
                };
                let text_alpha = snapshot.lyrics_text_opacity.clamp(0.0, 1.0);
                let text_passes = 1;
                // Key-color fill makes the text window itself transparent while
                // the independent UpdateLayeredWindow panel sits behind it.
                ui.painter().rect_filled(
                    capsule.expand(16.0),
                    CornerRadius::ZERO,
                    crate::theme::COLOR_KEY,
                );
                let outer = ctx
                    .input(|input| input.viewport().outer_rect)
                    .unwrap_or(capsule);
                let panel_color = theme::c().panel;
                crate::platform::update_lyrics_panel(
                    "RPlayer 桌面歌词",
                    (
                        outer.min.x as i32,
                        outer.min.y as i32,
                        outer.width() as i32,
                        outer.height() as i32,
                    ),
                    panel_alpha,
                    (panel_color.r(), panel_color.g(), panel_color.b()),
                    18.0,
                );
                if panel_alpha > 0.001 {
                    ui.painter().rect_stroke(
                        capsule.shrink(1.0),
                        CornerRadius::same(18),
                        Stroke::new(
                            1.0_f32,
                            with_alpha(
                                if dark {
                                    theme::white_alpha(0.10)
                                } else {
                                    theme::black_alpha(0.08)
                                },
                                panel_alpha,
                            ),
                        ),
                        egui::StrokeKind::Inside,
                    );
                }

                // D14: 0 0 1px halo - dark text on paper, light text over Nocturne.
                let shadow = if dark {
                    theme::white_alpha(0.22)
                } else {
                    theme::black_alpha(0.35)
                };
                let center_x = capsule.center().x;

                for _ in 0..text_passes {
                if !snapshot.has_track {
                    shadow_text(
                        ui.painter(),
                        Pos2::new(center_x, capsule.center().y - 10.0),
                        "RPlayer 桌面歌词",
                        FontId::proportional(22.0),
                        theme::c().text,
                        shadow,
                        text_alpha,
                    );
                    shadow_text(
                        ui.painter(),
                        Pos2::new(center_x, capsule.center().y + 22.0),
                        "播放歌曲后，歌词会在这里逐行滚动",
                        FontId::proportional(12.0),
                        theme::c().muted,
                        shadow,
                        text_alpha,
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
                        text_alpha,
                        );
                    } else if !snapshot.lyric_previous.trim().is_empty() {
                        shadow_text(
                            ui.painter(),
                            Pos2::new(center_x, capsule.top() + 31.0),
                            &snapshot.lyric_previous,
                            FontId::proportional(12.5),
                            theme::c().faint,
                            shadow,
                        text_alpha,
                        );
                    }
                    shadow_text(
                        ui.painter(),
                        Pos2::new(center_x, capsule.center().y + 1.0),
                        lyric,
                        crate::ui::tokens::serif(snapshot.lyrics_font),
                        theme::c().text,
                        shadow,
                        text_alpha,
                    );
                    if !snapshot.has_lyrics {
                        shadow_text(
                            ui.painter(),
                            Pos2::new(center_x, capsule.bottom() - 43.0),
                            "将同名 .lrc 放在歌曲目录，或",
                            FontId::proportional(10.5),
                            theme::c().faint,
                            shadow,
                        text_alpha,
                        );
                        shadow_text(
                            ui.painter(),
                            Pos2::new(center_x, capsule.bottom() - 23.0),
                            "获取歌词",
                            FontId::proportional(11.5),
                            theme::accent_hover(),
                            shadow,
                            text_alpha,
                        );
                        let underline = Rect::from_center_size(
                            Pos2::new(center_x, capsule.bottom() - 15.0),
                            Vec2::new(48.0, 1.0),
                        );
                        ui.painter().rect_filled(
                            underline,
                            0,
                            with_alpha(theme::accent_hover(), text_alpha),
                        );
                    } else if !snapshot.lyric_next.trim().is_empty() {
                        shadow_text(
                            ui.painter(),
                            Pos2::new(center_x, capsule.bottom() - 27.0),
                            &snapshot.lyric_next,
                            FontId::proportional(12.5),
                            theme::c().faint,
                            shadow,
                        text_alpha,
                        );
                    }
                }
                }

                let _drag = ui.interact(
                    capsule,
                    Id::new("lyrics-drag"),
                    Sense::click_and_drag(),
                );

                let locked = snapshot.lyrics_locked;
    // Publish the lock to the native hit-test so the OS stops offering its own
    // caption drag while the strip is locked.
    crate::platform::set_lyrics_locked(locked);
                let lock_rect = Rect::from_min_size(
                    Pos2::new(capsule.right() - 64.0, capsule.top() + 10.0),
                    Vec2::splat(24.0),
                );
                let lock_response =
                    ui.interact(lock_rect, Id::new("lyrics-lock"), Sense::click());
                if lock_response.hovered() && panel_alpha > 0.0 {
                    ui.painter().circle_filled(
                        lock_rect.center(),
                        12.0,
                        with_alpha(theme::white_alpha(0.10), panel_alpha),
                    );
                }
                icons::paint(
                    ui.painter(),
                    lock_rect.shrink(6.0),
                    Icon::Lock,
                    with_alpha(
                        if lock_response.hovered() {
                            theme::c().text
                        } else {
                            theme::c().muted
                        },
                        panel_alpha,
                    ),
                );
                if lock_response.clicked() {
                    if let Ok(mut queue) = commands.lock() {
                        queue.push(OverlayCommand::SetLyricsLocked(!locked));
                    }
                }
                let close_rect = if locked {
                    Rect::NOTHING
                } else {
                    Rect::from_min_size(
                        Pos2::new(capsule.right() - 36.0, capsule.top() + 10.0),
                        Vec2::splat(24.0),
                    )
                };
                let close_response = ui.interact(close_rect, Id::new("lyrics-close"), Sense::click());
                let close_hovered = close_response.hovered();
                if close_hovered && panel_alpha > 0.0 {
                    ui.painter().circle_filled(
                        close_rect.center(),
                        12.0,
                        with_alpha(theme::white_alpha(0.10), panel_alpha),
                    );
                }
                icons::paint(
                    ui.painter(),
                    close_rect.shrink(6.0),
                    Icon::Close,
                    with_alpha(
                        if close_hovered {
                            theme::c().text
                        } else {
                            theme::c().muted
                        },
                        panel_alpha,
                    ),
                );
                if close_response.clicked() {
                    close = true;
                }
                if locked && panel_alpha > 0.02 {
                    let lock_rect = Rect::from_min_size(
                        Pos2::new(capsule.right() - 154.0, capsule.top() + 8.0),
                        Vec2::splat(18.0),
                    );
                    icons::paint(
                        ui.painter(),
                        lock_rect,
                        Icon::Lock,
                        with_alpha(theme::c().muted, panel_alpha),
                    );
                    ui.painter().text(
                        Pos2::new(capsule.right() - 96.0, capsule.top() + 17.0),
                        Align2::RIGHT_CENTER,
                        "已锁定 · 设置→桌面歌词 解锁",
                        FontId::proportional(9.5),
                        with_alpha(theme::c().muted, panel_alpha),
                    );
                }

                let drag = ui.interact(
                    Rect::from_min_max(
                        Pos2::new(capsule.left() + 44.0, capsule.top() + 34.0),
                        Pos2::new(capsule.right() - 100.0, capsule.bottom()),
                    ),
                    Id::new("lyrics-drag"),
                    Sense::click_and_drag(),
                );
                // The strip is too short to host a popup of its own: egui would
                // place a context menu below the pointer, outside the window, and
                // clip it away. Right clicks therefore open the same controls
                // panel the gear button toggles (lock / settings / close).
                let right_clicked = drag.secondary_clicked() || crate::platform::take_lyrics_menu_request();
                if drag.double_clicked() && locked {
                    if let Ok(mut queue) = commands.lock() {
                        queue.push(OverlayCommand::SetLyricsLocked(false));
                    }
                }
                if drag.dragged() && !locked {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag.drag_stopped() {
                    if let Some(rect) = ctx.input(|input| input.viewport().outer_rect) {
                        if let Ok(mut queue) = commands.lock() {
                            queue.push(OverlayCommand::SetLyricsPosition(
                                rect.min.x, rect.min.y,
                            ));
                        }
                    }
                }
                drag.on_hover_cursor(if snapshot.lyrics_locked {
                    egui::CursorIcon::Default
                } else {
                    egui::CursorIcon::Grab
                })
                    .context_menu(|ui| {
                        menu::begin(ui);
                        menu::heading(ui, "桌面歌词");
                        let lock_label = if locked { "解锁位置" } else { "锁定位置" };
                        if menu::item(ui, Some(Icon::Lock), lock_label, None, false).clicked() {
                            if let Ok(mut queue) = commands.lock() {
                                queue.push(OverlayCommand::SetLyricsLocked(!locked));
                            }
                            ui.close_menu();
                        }
                        if menu::item(ui, Some(Icon::Settings), "桌面歌词设置…", None, false)
                            .clicked()
                        {
                            if let Ok(mut queue) = commands.lock() {
                                queue.push(OverlayCommand::OpenLyricsSettings);
                            }
                            ui.close_menu();
                        }
                        menu::separator(ui);
                        if menu::item(ui, Some(Icon::Close), "关闭桌面歌词", None, true)
                            .clicked()
                        {
                            close = true;
                            ui.close_menu();
                        }
                    });

                if !snapshot.has_lyrics && snapshot.has_track {
                    let cta = Rect::from_center_size(
                        Pos2::new(center_x, capsule.bottom() - 23.0),
                        Vec2::new(76.0, 24.0),
                    );
                    let response = ui
                        .interact(cta, Id::new("lyrics-fetch"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if response.clicked() {
                        if let Ok(mut queue) = commands.lock() {
                            queue.push(OverlayCommand::FetchLyrics);
                        }
                    }
                }

                // A gear opens the two opacity sliders. Its opacity follows the
                // panel, with a hover boost when the panel is fully transparent.
                let controls_id = Id::new("lyrics-opacity-open");
                let was_open = ctx
                    .data(|data| data.get_temp::<bool>(controls_id))
                    .unwrap_or(false);
                let mut controls_open = (was_open || right_clicked) && !locked;
                let button_rect = if locked {
                    Rect::NOTHING
                } else {
                    Rect::from_min_size(
                        Pos2::new(capsule.left() + 10.0, capsule.top() + 7.0),
                        Vec2::splat(24.0),
                    )
                };
                let button = ui.interact(
                    button_rect,
                    Id::new("lyrics-opacity-button"),
                    Sense::click(),
                );
                let gear_alpha = panel_alpha;
                if (button.hovered() || controls_open) && gear_alpha > 0.0 {
                    ui.painter().rect_filled(
                        button_rect,
                        CornerRadius::same(7),
                        with_alpha(theme::c().hover, gear_alpha),
                    );
                }
                icons::paint(
                    ui.painter(),
                    button_rect.shrink(5.0),
                    Icon::Settings,
                    with_alpha(
                        if button.hovered() {
                            theme::c().text
                        } else {
                            theme::c().muted
                        },
                        gear_alpha,
                    ),
                );
                if button.clicked() {
                    controls_open = !controls_open;
                }
                let mut popup_rect = Rect::NOTHING;
                if controls_open {
                    // While a slider is being dragged its value must not be taken
                    // from the snapshot again: the command only reaches the player
                    // a frame later, so re-seeding from it made the handle spring
                    // back under the pointer (v2.1.0 issue).
                    let drag_id = Id::new("lyrics-opacity-dragging");
                    let dragging: Option<(f32, f32)> = ctx.data(|data| data.get_temp(drag_id));
                    let (mut panel_value, mut text_value) = dragging
                        .unwrap_or((snapshot.lyrics_opacity, snapshot.lyrics_text_opacity));
                    let area = egui::Area::new(Id::new("lyrics-opacity-controls"))
                        .fixed_pos(capsule.left_top() + Vec2::new(10.0, 34.0))
                        .order(egui::Order::Foreground)
                        .show(ctx, |ui| {
                            Frame::NONE
                                .fill(theme::c().panel)
                                .stroke(Stroke::new(1.0_f32, theme::c().border))
                                .corner_radius(CornerRadius::same(10))
                                .inner_margin(Margin::symmetric(10, 8))
                                .show(ui, |ui| {
                                    ui.set_width(190.0);
                                    ui.label(
                                        RichText::new("面板不透明度")
                                            .size(10.0)
                                            .color(theme::c().muted),
                                    );
                                    let panel = ui.add(
                                        egui::Slider::new(&mut panel_value, 0.0..=1.0)
                                            .show_value(false)
                                            .trailing_fill(true),
                                    );
                                    if panel.changed() {
                                        if let Ok(mut queue) = commands.lock() {
                                            queue.push(OverlayCommand::SetLyricsOpacity(
                                                panel_value,
                                            ));
                                        }
                                    }
                                    ui.label(
                                        RichText::new("歌词文字不透明度")
                                            .size(10.0)
                                            .color(theme::c().muted),
                                    );
                                    let text = ui.add(
                                        egui::Slider::new(&mut text_value, 0.0..=1.0)
                                            .show_value(false)
                                            .trailing_fill(true),
                                    );
                                    if text.changed() {
                                        if let Ok(mut queue) = commands.lock() {
                                            queue.push(OverlayCommand::SetLyricsTextOpacity(
                                                text_value,
                                            ));
                                        }
                                    }
                                    if panel.dragged() || text.dragged() {
                                        ctx.data_mut(|data| {
                                            data.insert_temp(drag_id, (panel_value, text_value))
                                        });
                                    } else {
                                        ctx.data_mut(|data| {
                                            data.remove::<(f32, f32)>(drag_id)
                                        });
                                    }
                                    ui.add_space(4.0);
                                    // One compact row: the strip is only 158 px
                                    // tall, so three stacked buttons would push the
                                    // last one outside the window and clip it.
                                    ui.horizontal(|ui| {
                                        if widgets::secondary_button(ui, "设置…").clicked() {
                                            if let Ok(mut queue) = commands.lock() {
                                                queue.push(OverlayCommand::OpenLyricsSettings);
                                            }
                                            controls_open = false;
                                        }
                                        let lock_label = if locked { "解锁" } else { "锁定" };
                                        if widgets::secondary_button(ui, lock_label).clicked() {
                                            if let Ok(mut queue) = commands.lock() {
                                                queue.push(OverlayCommand::SetLyricsLocked(!locked));
                                            }
                                            controls_open = false;
                                        }
                                        if widgets::secondary_button(ui, "关闭").clicked() {
                                            if let Ok(mut queue) = commands.lock() {
                                                queue.push(OverlayCommand::CloseLyrics);
                                            }
                                            controls_open = false;
                                        }
                                    });
                                });
                        });
                    popup_rect = area.response.rect;
                }
                if controls_open && was_open && !right_clicked && !button.clicked() {
                    let clicked_outside = ctx.input(|input| {
                        input.pointer.any_click()
                            && input.pointer.latest_pos().is_some_and(|position| {
                                !popup_rect.contains(position)
                                    && !button_rect.contains(position)
                            })
                    });
                    // The strip never takes focus (WS_EX_NOACTIVATE), so clicks on
                    // other windows never reach egui; asking the platform keeps the
                    // old focus check from closing this panel the frame after it
                    // opened.
                    let clicked_elsewhere = clicked_outside
                        || crate::platform::left_button_outside("RPlayer 桌面歌词");
                    let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
                    if clicked_elsewhere || escape {
                        controls_open = false;
                    }
                }
                if controls_open != was_open {
                    ctx.data_mut(|data| data.insert_temp(controls_id, controls_open));
                }
            });

        if close {
            if let Ok(mut queue) = commands.lock() {
                queue.push(OverlayCommand::CloseLyrics);
            }
        }
        // Playing keeps a 30 fps tick so the lyric line can advance; paused
        // frames only need to react to explicit wakes from the main viewport.
        // While paused the strip still has to follow drags and opacity/typography
        // tweaks; 500 ms made those feel delayed, 250 ms keeps the window quiet
        // without the controls trailing the pointer (v2.1.0 issue #3).
        let interval = if snapshot.playing { 33 } else { 250 };
        ctx.request_repaint_after(std::time::Duration::from_millis(interval));
    });
}

