//! Windows system tray integration.

use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};

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
        start_event_thread();
        Some(Self { _icon: tray })
    }

    pub fn poll() -> Vec<TrayAction> {
        let mut queue = actions().lock().unwrap_or_else(|error| error.into_inner());
        std::mem::take(&mut *queue)
    }
}

fn actions() -> &'static Mutex<Vec<TrayAction>> {
    static QUEUE: OnceLock<Mutex<Vec<TrayAction>>> = OnceLock::new();
    QUEUE.get_or_init(|| Mutex::new(Vec::new()))
}

fn push(action: TrayAction) {
    if let Ok(mut queue) = actions().lock() {
        queue.push(action);
    }
}

fn start_event_thread() {
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        loop {
        let mut has_action = false;
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    ..
                } | TrayIconEvent::DoubleClick { .. }
            ) {
                push(TrayAction::Show);
                has_action = true;
            }
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            let action = match event.id.0.as_str() {
                "show" => TrayAction::Show,
                "toggle" => TrayAction::TogglePlay,
                "previous" => TrayAction::Previous,
                "next" => TrayAction::Next,
                "lyrics" => TrayAction::DesktopLyrics,
                "settings" => TrayAction::Settings,
                "exit" => TrayAction::Exit,
                _ => continue,
            };
            push(action);
            has_action = true;
        }
        if has_action {
            // The UI may be minimized; unminimize natively so egui resumes
            // updating and can consume the queued action immediately.
            restore_main_window();
        }
        std::thread::sleep(std::time::Duration::from_millis(80));
        }
    });
}

fn restore_main_window() {
    #[cfg(target_os = "windows")]
    unsafe {
        use winapi::um::winuser::{SetForegroundWindow, ShowWindow, SW_RESTORE};
        if let Some(hwnd) = crate::platform::main_window_handle() {
            let hwnd = hwnd as winapi::shared::windef::HWND;
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }
}
