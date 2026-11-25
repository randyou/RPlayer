//! Native Win32 system tray icon.
//!
//! The icon lives on a dedicated message-only window thread so tray events are
//! always pumped, even while the eframe root window is minimized.
//!
//! The menu itself is **not** a Win32 popup menu: `src/app/tray.rs` draws it in
//! a borderless egui viewport so it shares the design system (row height, icons,
//! checked state, dark/light palette) with every other surface. This module only
//! reports the cursor position of a right click.

use std::sync::{Mutex, OnceLock};

use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HICON, HWND, POINT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::{
    ExtractIconExW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NOTIFYICONDATAW,
};
use winapi::um::winuser::{
    CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetMessageW, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
    ShowWindow, TranslateMessage, UpdateWindow, IDI_APPLICATION, SW_RESTORE, WM_APP, WM_CLOSE,
    WM_CONTEXTMENU, WM_DESTROY, WS_POPUP, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW,
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
    /// Right click: open the app-drawn menu at these screen coordinates.
    OpenMenu(i32, i32),
}

pub struct Tray {
    hwnd: isize,
}

impl Tray {
    pub fn create() -> Option<Self> {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let hwnd = unsafe { create_tray_window() };
            let _ = sender.send(hwnd.map(|hwnd| hwnd as isize).unwrap_or(0));
            if hwnd.is_none() {
                return;
            }
            unsafe {
                let mut message = std::mem::zeroed();
                while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        });
        let hwnd = receiver
            .recv_timeout(std::time::Duration::from_secs(3))
            .ok()?;
        if hwnd == 0 {
            None
        } else {
            Some(Self { hwnd })
        }
    }

    /// Called whenever a tray event is queued, so the UI wakes immediately
    /// instead of waiting for the slow idle heartbeat.
    pub fn set_wake(wake: impl Fn() + Send + 'static) {
        *wake_slot().lock().unwrap_or_else(|error| error.into_inner()) = Some(Box::new(wake));
    }

    pub fn poll() -> Vec<TrayAction> {
        let mut queue = actions().lock().unwrap_or_else(|error| error.into_inner());
        std::mem::take(&mut *queue)
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        unsafe {
            PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0);
        }
    }
}

fn actions() -> &'static Mutex<Vec<TrayAction>> {
    static QUEUE: OnceLock<Mutex<Vec<TrayAction>>> = OnceLock::new();
    QUEUE.get_or_init(|| Mutex::new(Vec::new()))
}

fn wake_slot() -> &'static Mutex<Option<Box<dyn Fn() + Send>>> {
    static WAKE: OnceLock<Mutex<Option<Box<dyn Fn() + Send>>>> = OnceLock::new();
    WAKE.get_or_init(|| Mutex::new(None))
}

fn push(action: TrayAction) {
    if let Ok(mut queue) = actions().lock() {
        queue.push(action);
    }
    if let Ok(wake) = wake_slot().lock() {
        if let Some(wake) = wake.as_ref() {
            wake();
        }
    }
}

const WM_TRAY: UINT = WM_APP + 1;

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn create_tray_window() -> Option<HWND> {
    let class_name = wide(&format!("RPlayerTrayWindow{}", std::process::id()));
    let instance = GetModuleHandleW(std::ptr::null());
    let mut class: WNDCLASSW = std::mem::zeroed();
    class.lpfnWndProc = Some(wndproc);
    class.hInstance = instance;
    class.lpszClassName = class_name.as_ptr();
    RegisterClassW(&class);

    let title = wide("RPlayer Tray");
    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        title.as_ptr(),
        WS_POPUP,
        0,
        0,
        0,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        instance,
        std::ptr::null_mut(),
    );
    if hwnd.is_null() {
        return None;
    }

    let mut icon: HICON = std::ptr::null_mut();
    if let Ok(exe) = std::env::current_exe() {
        let exe = wide(&exe.to_string_lossy());
        let mut large: HICON = std::ptr::null_mut();
        ExtractIconExW(exe.as_ptr(), 0, &mut large, &mut icon, 1);
        if !large.is_null() {
            // Small is what the notification area uses; large is not needed.
            DestroyIcon(large);
        }
    }
    if icon.is_null() {
        icon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
    }

    let mut data: NOTIFYICONDATAW = std::mem::zeroed();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = icon;
    let tip = wide("RPlayer");
    let count = tip.len().min(data.szTip.len());
    data.szTip[..count].copy_from_slice(&tip[..count]);
    if Shell_NotifyIconW(NIM_ADD, &mut data) == 0 {
        DestroyWindow(hwnd);
        return None;
    }
    UpdateWindow(hwnd);
    Some(hwnd)
}

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    message: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_TRAY => {
            let event = lparam as u32;
            if event == WM_LBUTTONUP || event == WM_LBUTTONDBLCLK {
                push(TrayAction::Show);
                restore_main_window();
            } else if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                let mut point: POINT = std::mem::zeroed();
                GetCursorPos(&mut point);
                push(TrayAction::OpenMenu(point.x, point.y));
            }
            0
        }
        WM_DESTROY => {
            let mut data: NOTIFYICONDATAW = std::mem::zeroed();
            data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            data.hWnd = hwnd;
            data.uID = 1;
            Shell_NotifyIconW(NIM_DELETE, &mut data);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

fn restore_main_window() {
    unsafe {
        if let Some(hwnd) = crate::platform::main_window_handle() {
            let hwnd = hwnd as HWND;
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }
}
