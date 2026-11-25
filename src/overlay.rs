//! Always-on-top desktop lyrics and mini player viewports.
//!
//! The main app owns playback; these windows are pure views. They publish a
//! snapshot through a mutex and send tiny commands back through a queue.

use std::sync::{Arc, Mutex};

use eframe::egui;
use egui::{
    Align, Align2, Color32, ColorImage, CornerRadius, FontId, Frame, Id, Layout, Pos2, Rect,
    Margin, RichText, Sense, Stroke, TextureHandle, Vec2, ViewportBuilder, ViewportCommand,
    ViewportId,
};

use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::menu;
use crate::ui::widgets;

const LYRICS_SIZE: Vec2 = Vec2::new(860.0, 158.0);

const MINI_SIZE: Vec2 = Vec2::new(410.0, 146.0);

#[derive(Clone, Default)]
pub struct OverlayState {
    pub has_track: bool,
    pub title: String,
    pub artist: String,
    /// Album track number, shown as `艺术家 · 编号` on the mini player.
    pub track_number: Option<u32>,
    pub position: f64,
    pub duration: f64,
    pub playing: bool,
    pub favorite: bool,
    pub lyric_previous: String,
    pub lyric_current: String,
    pub lyric_next: String,
    pub has_lyrics: bool,
    pub lyrics_opacity: f32,
    pub lyrics_text_opacity: f32,
    pub lyrics_font: f32,
    pub lyrics_locked: bool,
    pub mini_style: String,
    pub mini_on_top: bool,
    pub mini_snap: bool,
    pub desktop_lyrics_open: bool,
    /// Mini player viewport is up; the tray menu mirrors it as a check mark.
    pub mini_open: bool,
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
    SetLyricsTextOpacity(f32),
    SetLyricsFont(f32),
    SetLyricsLocked(bool),
    SetLyricsPosition(f32, f32),
    ToggleLyrics,
    SetMiniOnTop(bool),
    SetMiniSnap(bool),
    SetMiniPosition(f32, f32),
    CloseLyrics,
    CloseMini,
    OpenMain,
    OpenLyricsSettings,
    FetchLyrics,
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
                    Color32::from_rgb(160, 160, 160),
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

                let shadow = theme::black_alpha(0.62);
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
                        Color32::from_rgb(0xF7, 0xF2, 0xE8),
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

                let drag = ui.interact(
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
                let mut controls_open = was_open && !locked;
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
                                    if widgets::secondary_button(ui, "打开桌面歌词设置").clicked() {
                                        if let Ok(mut queue) = commands.lock() {
                                            queue.push(OverlayCommand::OpenLyricsSettings);
                                        }
                                        controls_open = false;
                                    }
                                });
                        });
                    popup_rect = area.response.rect;
                }
                if controls_open && !button.clicked() {
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
        let previous_theme = theme::preset();
        if previous_theme != theme::ThemePreset::Forest {
            theme::set_preset(theme::ThemePreset::Forest);
        }
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
                                                .size(14.5)
                                                .strong()
                                                .color(theme::c().text),
                                            )
                                            .truncate(),
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
                                            .truncate(),
                                        );
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
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
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag.drag_stopped() {
                    let pos = ctx
                        .input(|input| input.viewport().outer_rect)
                        .map(|rect| rect.min)
                        .unwrap_or(card.min);
                    snap_and_store_mini(ctx, &snapshot, &commands, pos, card.size());
                }
            });

        if previous_theme != theme::ThemePreset::Forest {
            theme::set_preset(previous_theme);
        }

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
                        .truncate(),
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
                .truncate(),
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
    alpha: f32,
) {
    painter.text(
        pos + Vec2::new(1.0, 2.0),
        Align2::CENTER_CENTER,
        text,
        font.clone(),
        with_alpha(shadow, alpha),
    );
    painter.text(
        pos,
        Align2::CENTER_CENTER,
        text,
        font,
        with_alpha(color, alpha),
    );
}

fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (color.a() as f32 * alpha.clamp(0.0, 1.0)) as u8,
    )
}
