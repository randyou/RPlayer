//! Tray events, the tray menu popup, and close-to-tray behavior.
//!
//! The menu is drawn in its own borderless viewport instead of a Win32 popup so
//! it can use the same rows, icons and palette as the context menus. The native
//! side (`platform::tray`) only reports "right click at (x, y)".

use std::sync::{Arc, Mutex};

use eframe::egui::{
    self, Align2, CornerRadius, FontId, Pos2, Stroke, Vec2, ViewportBuilder,
    ViewportCommand, ViewportId,
};

use super::RPlayerApp;
use crate::theme;
use crate::ui::icons::Icon;
use crate::ui::menu;

/// Popup width in points (matches the context-menu grammar, one step narrower).
pub(super) const MENU_WIDTH: f32 = 268.0;
const PAD: f32 = 8.0;
const HEADER: f32 = 54.0;
const SEP: f32 = 9.0;
/// Show, play/pause, previous, next, lyrics, settings, quit.
const ROWS: f32 = 7.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TrayMenuCommand {
    Show,
    PlayToggle,
    Previous,
    Next,
    DesktopLyrics,
    Settings,
    Exit,
    Close,
}

pub(super) struct TrayMenu {
    pub pos: Pos2,
    pub opened_at: f64,
    pub commands: Arc<Mutex<Vec<TrayMenuCommand>>>,
}

pub(super) fn tray_menu_viewport() -> ViewportId {
    ViewportId::from_hash_of("rplayer-tray-menu")
}

fn menu_height() -> f32 {
    PAD * 2.0 + HEADER + menu::ROW_HEIGHT * ROWS + SEP * 2.0
}

impl RPlayerApp {
    pub(super) fn poll_tray(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "windows")]
        {
            if self.tray.is_none() {
                return;
            }
            for action in crate::platform::tray::Tray::poll() {
                match action {
                    crate::platform::tray::TrayAction::Show => self.show_main_window(ctx),
                    crate::platform::tray::TrayAction::TogglePlay => self.toggle_play(ctx),
                    crate::platform::tray::TrayAction::Previous => self.previous_track(ctx),
                    crate::platform::tray::TrayAction::Next => self.next_track(ctx, false),
                    crate::platform::tray::TrayAction::DesktopLyrics => {
                        self.toggle_desktop_lyrics(ctx)
                    }
                    crate::platform::tray::TrayAction::Settings => {
                        self.show_main_window(ctx);
                        self.settings_open = true;
                    }
                    crate::platform::tray::TrayAction::Exit => self.exit_from_tray(ctx),
                    crate::platform::tray::TrayAction::OpenMenu(x, y) => {
                        self.open_tray_menu(ctx, Pos2::new(x as f32, y as f32))
                    }
                }
            }
            // No timer here: the tray heartbeat thread pokes the root viewport,
            // and polling twice per frame would only raise idle CPU.
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = ctx;
        }
    }

    /// Open the popup just below/right of the cursor, flipped when it would run
    /// off the monitor.
    pub(super) fn open_tray_menu(&mut self, ctx: &egui::Context, screen: Pos2) {
        let ppp = ctx.pixels_per_point().max(0.5);
        let mut pos = Pos2::new(screen.x / ppp, screen.y / ppp);
        let height = menu_height();
        // Prefer the real work area of the monitor under the cursor: the tray
        // lives in the notification area, and egui reports no monitor size for
        // the root viewport, so the old 1920x1080 fallback put the popup
        // half off-screen on a 1440-wide display.
        let (screen_size, min_pos) = match crate::platform::cursor_monitor_work_area() {
            Some((left, top, right, bottom)) => (
                Vec2::new(right / ppp, bottom / ppp),
                Vec2::new(left / ppp, top / ppp),
            ),
            None => (
                ctx.input(|input| input.viewport().monitor_size)
                    .unwrap_or(Vec2::new(1920.0, 1080.0)),
                Vec2::ZERO,
            ),
        };
        if pos.x + MENU_WIDTH > screen_size.x {
            pos.x = (screen_size.x - MENU_WIDTH).max(min_pos.x);
        }
        if pos.y + height > screen_size.y {
            pos.y = (pos.y - height).max(min_pos.y);
        }
        self.tray_menu = Some(TrayMenu {
            pos,
            opened_at: ctx.input(|input| input.time),
            commands: Arc::new(Mutex::new(Vec::new())),
        });
        ctx.request_repaint();
    }

    pub(super) fn show_tray_menu(&mut self, ctx: &egui::Context) {
        let Some(open) = self.tray_menu.as_ref() else {
            return;
        };
        let (pos, opened_at) = (open.pos, open.opened_at);
        let commands = open.commands.clone();
        // The closure below owns a clone; this handle drains the results after
        // the viewport has drawn for this frame.
        let queue = open.commands.clone();
        let state = self.overlay_state.clone();
        let height = menu_height();

        let builder = ViewportBuilder::default()
            .with_title("RPlayer 托盘菜单")
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_taskbar(false)
            .with_inner_size([MENU_WIDTH, height])
            .with_min_inner_size([MENU_WIDTH, height])
            .with_max_inner_size([MENU_WIDTH, height])
            .with_position(pos)
            .with_active(false);

        // Immediate (synchronous) viewport: the popup must exist on the same
        // frame it was requested, otherwise the next frame is an idle one and
        // the menu would not appear until something else wakes the app.
        ctx.show_viewport_immediate(tray_menu_viewport(), builder, move |ctx, _class| {
            let snapshot = state
                .lock()
                .map(|state| state.clone())
                .unwrap_or_default();
            let mut clicked: Option<TrayMenuCommand> = None;
            let mut close = false;

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    let card = ui.max_rect();
                    ui.painter().rect_filled(
                        card,
                        CornerRadius::same(10),
                        theme::c().panel,
                    );
                    ui.painter().rect_stroke(
                        card.shrink(0.5),
                        CornerRadius::same(10),
                        Stroke::new(1.0_f32, theme::c().border_soft),
                        egui::StrokeKind::Inside,
                    );

                    ui.allocate_new_ui(
                        egui::UiBuilder::new().max_rect(card.shrink(PAD)),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.vertical(|ui| {
                                // Header: what is playing right now.
                                ui.add_space(2.0);
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), HEADER - 4.0),
                                    egui::Sense::hover(),
                                );
                                let left = rect.left() + 2.0;
                                ui.painter().text(
                                    Pos2::new(left, rect.top() + 7.0),
                                    Align2::LEFT_TOP,
                                    "RPLAYER",
                                    FontId::monospace(8.2),
                                    theme::c().faint,
                                );
                                let (title, sub) = if snapshot.has_track {
                                    let artist = if snapshot.artist.is_empty() {
                                        "未知艺术家".to_owned()
                                    } else {
                                        snapshot.artist.clone()
                                    };
                                    (snapshot.title.clone(), artist)
                                } else {
                                    ("本地音乐库".to_owned(), "未在播放".to_owned())
                                };
                                ui.painter().text(
                                    Pos2::new(left, rect.top() + 20.0),
                                    Align2::LEFT_TOP,
                                    &title,
                                    FontId::proportional(12.5),
                                    theme::c().text,
                                );
                                ui.painter().text(
                                    Pos2::new(left, rect.top() + 38.0),
                                    Align2::LEFT_TOP,
                                    &sub,
                                    FontId::proportional(10.0),
                                    theme::c().muted,
                                );

                                let width = ui.available_width();
                                if menu::row(
                                    ui,
                                    width,
                                    Some(Icon::PanelRight),
                                    "显示主窗口",
                                    None,
                                    false,
                                    false,
                                )
                                .clicked()
                                {
                                    clicked = Some(TrayMenuCommand::Show);
                                }
                                let (label, icon) = if snapshot.playing {
                                    ("暂停", Icon::Pause)
                                } else {
                                    ("播放", Icon::Play)
                                };
                                if menu::row(
                                    ui,
                                    width,
                                    Some(icon),
                                    label,
                                    Some("Space"),
                                    false,
                                    false,
                                )
                                .clicked()
                                {
                                    clicked = Some(TrayMenuCommand::PlayToggle);
                                }
                                if menu::row(
                                    ui,
                                    width,
                                    Some(Icon::Prev),
                                    "上一首",
                                    None,
                                    false,
                                    false,
                                )
                                .clicked()
                                {
                                    clicked = Some(TrayMenuCommand::Previous);
                                }
                                if menu::row(
                                    ui,
                                    width,
                                    Some(Icon::Next),
                                    "下一首",
                                    None,
                                    false,
                                    false,
                                )
                                .clicked()
                                {
                                    clicked = Some(TrayMenuCommand::Next);
                                }
                                menu::separator_sized(ui, width);
                                if menu::row(
                                    ui,
                                    width,
                                    Some(Icon::Lyrics),
                                    "桌面歌词",
                                    Some("Ctrl+L"),
                                    false,
                                    snapshot.desktop_lyrics_open,
                                )
                                .clicked()
                                {
                                    clicked = Some(TrayMenuCommand::DesktopLyrics);
                                }
                                if menu::row(
                                    ui,
                                    width,
                                    Some(Icon::Settings),
                                    "设置",
                                    Some("Ctrl+,"),
                                    false,
                                    false,
                                )
                                .clicked()
                                {
                                    clicked = Some(TrayMenuCommand::Settings);
                                }
                                menu::separator_sized(ui, width);
                                if menu::row(
                                    ui,
                                    width,
                                    Some(Icon::Close),
                                    "退出 RPlayer",
                                    None,
                                    true,
                                    false,
                                )
                                .clicked()
                                {
                                    clicked = Some(TrayMenuCommand::Exit);
                                }
                            });
                        },
                    );
                });

            if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
                close = true;
            }
            // The popup is created inactive on purpose: a focus-stealing,
            // undecorated transparent window fails to be created on Windows.
            // It is anchored to the tray icon, so the cursor normally sits
            // outside its rect right after opening - a native menu therefore
            // dismisses on an outside click, never on hover-out.
            let now = ctx.input(|input| input.time);
            if now - opened_at > 0.25
                && crate::platform::left_button_outside("RPlayer 托盘菜单")
            {
                close = true;
            }
            if let Some(command) = clicked {
                if let Ok(mut queue) = commands.lock() {
                    queue.push(command);
                }
                close = true;
            }
            if close {
                if let Ok(mut queue) = commands.lock() {
                    queue.push(TrayMenuCommand::Close);
                }
            }
            // Poll for the outside click without spinning: the menu lives for
            // seconds, but the poll has to be fast enough to catch a quick tap.
            ctx.request_repaint_after(std::time::Duration::from_millis(40));
        });

        let mut commands_rx = Vec::new();
        if let Ok(mut queue) = queue.lock() {
            commands_rx.append(&mut queue);
        }
        if commands_rx.is_empty() {
            return;
        }
        self.close_tray_menu(ctx);
        for command in commands_rx {
            match command {
                TrayMenuCommand::Show => self.show_main_window(ctx),
                TrayMenuCommand::PlayToggle => self.toggle_play(ctx),
                TrayMenuCommand::Previous => self.previous_track(ctx),
                TrayMenuCommand::Next => self.next_track(ctx, false),
                TrayMenuCommand::DesktopLyrics => self.toggle_desktop_lyrics(ctx),
                TrayMenuCommand::Settings => {
                    self.show_main_window(ctx);
                    self.settings_open = true;
                }
                TrayMenuCommand::Exit => self.exit_from_tray(ctx),
                TrayMenuCommand::Close => {}
            }
        }
    }

    fn close_tray_menu(&mut self, ctx: &egui::Context) {
        if self.tray_menu.take().is_some() {
            ctx.send_viewport_cmd_to(tray_menu_viewport(), ViewportCommand::Close);
        }
    }

    fn show_main_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        if let Some(pos) = self.main_window_pos.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn exit_from_tray(&mut self, ctx: &egui::Context) {
        self.allow_exit = true;
        self.credit_current_track(false);
        self.persist();
        self.save_session();
        self.save_stats();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}


