//! Global 54px top bar: breadcrumb, search, quick actions.

use eframe::egui;
use egui::{
    Align, Color32, CursorIcon, Frame, Id, Layout, Margin, Pos2, Rect, ResizeDirection, Response,
    RichText, Sense, Stroke, Ui, Vec2, ViewportCommand,
};

use super::{RPlayerApp, View};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::{tokens, widgets};

#[derive(Clone, Copy)]
enum WindowButton {
    Minimize,
    Maximize,
    Close,
}

#[derive(Clone, Copy)]
struct ResizeState {
    direction: ResizeDirection,
    start_pointer: Pos2,
    start_rect: (f32, f32, f32, f32),
}

#[cfg(target_os = "windows")]
fn native_window_rect() -> Option<(isize, (f32, f32, f32, f32))> {
    use winapi::shared::windef::{HWND, RECT};
    use winapi::um::winuser::GetWindowRect;

    let hwnd = crate::platform::main_window_handle()?;
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok = unsafe { GetWindowRect(hwnd as HWND, &mut rect) };
    (ok != 0).then_some((
        hwnd,
        (
            rect.left as f32,
            rect.top as f32,
            (rect.right - rect.left) as f32,
            (rect.bottom - rect.top) as f32,
        ),
    ))
}

#[cfg(target_os = "windows")]
fn native_cursor_pos() -> Option<Pos2> {
    use winapi::shared::windef::POINT;
    use winapi::um::winuser::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    (ok != 0).then_some(Pos2::new(point.x as f32, point.y as f32))
}

#[cfg(target_os = "windows")]
fn native_resize_window(hwnd: isize, x: f32, y: f32, width: f32, height: f32) {
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};

    unsafe {
        SetWindowPos(
            hwnd as HWND,
            std::ptr::null_mut(),
            x.round() as i32,
            y.round() as i32,
            width.round() as i32,
            height.round() as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

impl RPlayerApp {
    /// Borderless windows need their own edge hit-testing and resize loop.
    pub(super) fn window_resize_edges(&self, ctx: &egui::Context) {
        const EDGE: f32 = 6.0;
        const MIN_W: f32 = 1120.0;
        const MIN_H: f32 = 720.0;
        let id = Id::new("rplayer-window-resize");

        let Some(pos) = native_cursor_pos() else {
            return;
        };
        let down = ctx.input(|input| input.pointer.primary_down());
        let pressed = ctx.input(|input| input.pointer.primary_pressed());
        let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));

        if !down {
            ctx.data_mut(|data| data.remove::<ResizeState>(id));
        }

        if maximized {
            return;
        }

        let Some((hwnd, start_rect)) = native_window_rect() else {
            return;
        };
        let edge = EDGE * ctx.pixels_per_point();
        let left = pos.x - start_rect.0 <= edge;
        let right = start_rect.0 + start_rect.2 - pos.x <= edge;
        // The top strip belongs to the titlebar drag handle; top-edge resizing
        // remains intentionally disabled until hit-testing is reliable.
        let top = false;
        let bottom = start_rect.1 + start_rect.3 - pos.y <= edge;

        if down {
            if let Some(state) = ctx.data(|data| data.get_temp::<ResizeState>(id)) {
                let delta = pos - state.start_pointer;
                let left = matches!(
                    state.direction,
                    ResizeDirection::West
                        | ResizeDirection::NorthWest
                        | ResizeDirection::SouthWest
                );
                let right = matches!(
                    state.direction,
                    ResizeDirection::East
                        | ResizeDirection::NorthEast
                        | ResizeDirection::SouthEast
                );
                let top = matches!(
                    state.direction,
                    ResizeDirection::North
                        | ResizeDirection::NorthWest
                        | ResizeDirection::NorthEast
                );
                let bottom = matches!(
                    state.direction,
                    ResizeDirection::South
                        | ResizeDirection::SouthWest
                        | ResizeDirection::SouthEast
                );

                let start = state.start_rect;
                let ppp = ctx.pixels_per_point();
                let Some(cursor) = native_cursor_pos() else {
                    return;
                };
                let delta = cursor - state.start_pointer;
                let min_width = MIN_W * ppp;
                let min_height = MIN_H * ppp;
                let mut width = start.2;
                let mut height = start.3;
                let mut x = start.0;
                let mut y = start.1;
                if left {
                    width = (start.2 - delta.x).max(min_width);
                    x = start.0 + (start.2 - width);
                } else if right {
                    width = (start.2 + delta.x).max(min_width);
                }
                if top {
                    height = (start.3 - delta.y).max(min_height);
                    y = start.1 + (start.3 - height);
                } else if bottom {
                    height = (start.3 + delta.y).max(min_height);
                }

                native_resize_window(hwnd, x, y, width, height);
                return;
            }
        }

        let direction = match (top, bottom, left, right) {
            (true, _, true, _) => ResizeDirection::NorthWest,
            (true, _, _, true) => ResizeDirection::NorthEast,
            (_, true, true, _) => ResizeDirection::SouthWest,
            (_, true, _, true) => ResizeDirection::SouthEast,
            (true, _, _, _) => ResizeDirection::North,
            (_, true, _, _) => ResizeDirection::South,
            (_, _, true, _) => ResizeDirection::West,
            (_, _, _, true) => ResizeDirection::East,
            _ => return,
        };

        let cursor = match (top, bottom, left, right) {
            (true, _, true, _) | (_, true, _, true) => CursorIcon::ResizeNwSe,
            (true, _, _, true) | (_, true, true, _) => CursorIcon::ResizeNeSw,
            (true, _, _, _) | (_, true, _, _) => CursorIcon::ResizeVertical,
            _ => CursorIcon::ResizeHorizontal,
        };
        ctx.set_cursor_icon(cursor);

        if !pressed {
            return;
        }

        ctx.data_mut(|data| {
            data.insert_temp(
                id,
                ResizeState {
                    direction,
                    start_pointer: pos,
                    start_rect,
                },
            );
        });
    }

    pub(super) fn topbar_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("rplayer-topbar")
            .exact_height(tokens::TOPBAR_HEIGHT)
            .frame(
                Frame::NONE
                    .fill(theme::c().bg)
                    .inner_margin(Margin::symmetric(18, 0)),
            )
            .show(ctx, |ui| {
                ui.painter().line_segment(
                    [
                        egui::Pos2::new(ui.max_rect().left(), ui.max_rect().bottom() - 0.5),
                        egui::Pos2::new(ui.max_rect().right(), ui.max_rect().bottom() - 0.5),
                    ],
                    egui::Stroke::new(1.0_f32, theme::c().border_soft),
                );
                ui.horizontal_centered(|ui| {
                    self.topbar_breadcrumb(ui);

                    let compact = ui.available_width() < 900.0;
                    let search_width = if compact { 250.0 } else { 360.0 };
                    ui.add_space(((ui.available_width() - search_width - 300.0) * 0.5).max(8.0));
                    let response = ui
                        .allocate_ui_with_layout(
                            Vec2::new(search_width, 32.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                widgets::search_box(ui, &mut self.search, "搜索曲目、专辑或艺术家")
                            },
                        )
                        .inner;
                    if std::mem::take(&mut self.focus_search) {
                        response.request_focus();
                    }
                    if response.has_focus() {
                        crate::ui::focus::ring_rect(ui, response.rect, 16);
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.topbar_actions(ui, compact);
                    });
                });

                // The breadcrumb doubles as the borderless window's drag handle.
                let drag = ui.interact(
                    Rect::from_min_size(
                        ui.max_rect().min,
                        Vec2::new(220.0, tokens::TOPBAR_HEIGHT),
                    ),
                    Id::new("rplayer-titlebar-drag"),
                    Sense::click_and_drag(),
                );
                let drag = drag.on_hover_cursor(egui::CursorIcon::Grab);
                let on_edge = ctx
                    .input(|input| input.pointer.latest_pos())
                    .is_some_and(|pos| {
                        let rect = ctx.screen_rect();
                        pos.x - rect.left() <= 6.0
                            || rect.right() - pos.x <= 6.0
                            || rect.bottom() - pos.y <= 6.0
                    });
                if drag.clicked() && self.expanded {
                    self.expanded = false;
                }
                if drag.drag_started() && !on_edge {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
            });
    }

    fn topbar_breadcrumb(&mut self, ui: &mut Ui) {
        let (icon, label) = if self.expanded {
            (Icon::ChevronDown, "正在播放")
        } else {
            match self.view {
                View::Library => (Icon::Disc, "本地曲库"),
                View::Albums => (Icon::Disc, "专辑"),
                View::Artists => (Icon::Disc, "艺术家"),
                View::Favorites => (Icon::Heart, "我的收藏"),
                View::Recent => (Icon::Clock, "最近播放"),
                View::Folders => (Icon::Folder, "音乐目录"),
                View::Playlist(_) => (Icon::Music, "播放列表"),
                View::Smart(_) => (Icon::List, "智能歌单"),
            }
        };
        let response = ui
            .allocate_ui_with_layout(
                Vec2::new(220.0, tokens::TOPBAR_HEIGHT),
                Layout::left_to_right(Align::Center),
                |ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), Sense::hover());
                    icons::paint(ui.painter(), rect, icon, theme::c().faint);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!("资料库  /  {label}"))
                            .font(tokens::mono(9.5))
                            .color(theme::c().muted),
                    );
                },
            )
            .response
            .interact(Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() && self.expanded {
            self.expanded = false;
        }
    }

    fn topbar_actions(&mut self, ui: &mut Ui, compact: bool) {
        ui.spacing_mut().item_spacing.x = 4.0;

        // Right-to-left: the first item lands on the far right.
        if self.window_button(ui, WindowButton::Close).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
        if self.window_button(ui, WindowButton::Maximize).clicked() {
            let maximized = ui
                .ctx()
                .input(|input| input.viewport().maximized.unwrap_or(false));
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
        }
        if self.window_button(ui, WindowButton::Minimize).clicked() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Minimized(true));
        }

        ui.add_space(8.0);
        let online = !self.library.folders.is_empty();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let (dot, _) = ui.allocate_exact_size(Vec2::splat(5.0), Sense::hover());
            ui.painter().circle_filled(
                dot.center(),
                2.5,
                if online {
                    theme::c().accent
                } else {
                    theme::c().faint
                },
            );
            ui.label(
                RichText::new(if online { "本地" } else { "离线" })
                    .font(tokens::mono(8.5))
                    .color(theme::c().faint),
            );
        });

        ui.add_space(4.0);
        let icon = if self.theme_mode.is_dark() {
            Icon::Sun
        } else {
            Icon::Moon
        };
        if widgets::icon_button(
            ui,
            icon,
            28.0,
            if self.theme_mode.is_dark() {
                "切换到浅色主题"
            } else {
                "切换到深色主题"
            },
        )
        .clicked()
        {
            self.toggle_theme(ui.ctx());
        }
        if widgets::icon_button_tinted(
            ui,
            Icon::Settings,
            28.0,
            theme::c().muted,
            self.settings_open,
            "设置",
        )
        .clicked()
        {
            self.settings_open = !self.settings_open;
        }
        if !compact {
            if widgets::icon_button_tinted(
                ui,
                Icon::Equalizer,
                28.0,
                theme::c().muted,
                self.eq_open,
                "均衡器",
            )
            .clicked()
            {
                self.toggle_eq();
            }
            if widgets::icon_button_tinted(
                ui,
                Icon::MiniPlayer,
                28.0,
                theme::c().muted,
                self.mini_open,
                "迷你播放器",
            )
            .clicked()
            {
                self.toggle_mini(ui.ctx());
            }
            if widgets::glyph_button_tinted(
                ui,
                "词",
                28.0,
                theme::c().muted,
                self.desktop_lyrics_open,
                "桌面歌词",
            )
            .clicked()
            {
                self.toggle_desktop_lyrics(ui.ctx());
            }
        }
    }

    fn window_button(&self, ui: &mut Ui, kind: WindowButton) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(46.0, tokens::TOPBAR_HEIGHT), Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let hovered = response.hovered();
        if hovered {
            let fill = if matches!(kind, WindowButton::Close) {
                theme::signal()
            } else {
                theme::c().hover
            };
            ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, fill);
        }
        let color = if hovered && matches!(kind, WindowButton::Close) {
            Color32::WHITE
        } else if hovered {
            theme::c().text
        } else {
            theme::c().muted
        };
        let stroke = Stroke::new(1.15_f32, color);
        let center = rect.center();
        match kind {
            WindowButton::Minimize => {
                ui.painter().line_segment(
                    [
                        Pos2::new(center.x - 5.0, center.y),
                        Pos2::new(center.x + 5.0, center.y),
                    ],
                    stroke,
                );
            }
            WindowButton::Maximize => {
                ui.painter().rect_stroke(
                    Rect::from_center_size(center, Vec2::splat(9.0)),
                    egui::CornerRadius::ZERO,
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
            WindowButton::Close => {
                ui.painter().line_segment(
                    [
                        Pos2::new(center.x - 4.5, center.y - 4.5),
                        Pos2::new(center.x + 4.5, center.y + 4.5),
                    ],
                    stroke,
                );
                ui.painter().line_segment(
                    [
                        Pos2::new(center.x + 4.5, center.y - 4.5),
                        Pos2::new(center.x - 4.5, center.y + 4.5),
                    ],
                    stroke,
                );
            }
        }
        if response.has_focus() {
            crate::ui::focus::ring_rect(ui, rect.shrink(2.0), 0);
        }
        response
    }
}
