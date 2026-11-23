//! Windows system tray integration.

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Show,
    TogglePlay,
    Previous,
    Next,
    DesktopLyrics,
    Settings,
    Exit,
}

pub struct Tray {
    _icon: TrayIcon,
}

impl Tray {
    pub fn create() -> Option<Self> {
        let menu = Menu::new();
        let show = MenuItem::with_id("show", "显示主窗口", true, None);
        let toggle = MenuItem::with_id("toggle", "播放 / 暂停", true, None);
        let previous = MenuItem::with_id("previous", "上一首", true, None);
        let next = MenuItem::with_id("next", "下一首", true, None);
        let lyrics = MenuItem::with_id("lyrics", "桌面歌词", true, None);
        let settings = MenuItem::with_id("settings", "设置", true, None);
        let exit = MenuItem::with_id("exit", "退出 RPlayer", true, None);
        let separator = PredefinedMenuItem::separator();
        menu.append_items(&[
            &show,
            &toggle,
            &previous,
            &next,
            &separator,
            &lyrics,
            &settings,
            &separator,
            &exit,
        ])
        .ok()?;

        let image = image::load_from_memory(include_bytes!("../../assets/icon-256.png")).ok()?;
        let rgba = image.to_rgba8();
        let icon = Icon::from_rgba(
            rgba.as_raw().to_vec(),
            rgba.width(),
            rgba.height(),
        )
        .ok()?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("RPlayer")
            .with_icon(icon)
            .build()
            .ok()?;
        Some(Self { _icon: tray })
    }

    pub fn poll() -> Vec<TrayAction> {
        let mut actions = Vec::new();
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                actions.push(TrayAction::Show);
            } else if let TrayIconEvent::DoubleClick { .. } = event {
                actions.push(TrayAction::Show);
            }
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            actions.push(match event.id.0.as_str() {
                "show" => TrayAction::Show,
                "toggle" => TrayAction::TogglePlay,
                "previous" => TrayAction::Previous,
                "next" => TrayAction::Next,
                "lyrics" => TrayAction::DesktopLyrics,
                "settings" => TrayAction::Settings,
                "exit" => TrayAction::Exit,
                _ => continue,
            });
        }
        actions
    }
}
