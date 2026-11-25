//! Tray event handling and close-to-tray behavior.

use eframe::egui;

use super::RPlayerApp;

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
