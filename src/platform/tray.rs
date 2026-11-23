//! Native Win32 system tray icon and popup menu.
//!
//! `tray-icon`/`muda` delivered menu events through an extra event loop that
//! could go silent while the eframe root window was minimized. This module owns
//! a message-only window on a dedicated thread, so menu commands are always
//! pumped and queued independently of the UI.

use std::sync::{Mutex, OnceLock};

use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HICON, HMENU, HWND, POINT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::{
    ExtractIconExW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NOTIFYICONDATAW,
};
use winapi::um::winuser::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, PostMessageW, PostQuitMessage,
    RegisterClassW, SetForegroundWindow, ShowWindow, TrackPopupMenu, TranslateMessage,
    UpdateWindow, IDI_APPLICATION, MF_STRING, SW_RESTORE, DestroyIcon,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_CLOSE, WM_CONTEXTMENU, WM_DESTROY, WS_POPUP,
    WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
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

fn push(action: TrayAction) {
    if let Ok(mut queue) = actions().lock() {
        queue.push(action);
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

    let menu = CreatePopupMenu();
    if menu.is_null() {
        DestroyWindow(hwnd);
        return None;
    }
    for (id, label) in [
        (1001, "显示主窗口"),
        (1002, "播放 / 暂停"),
        (1003, "上一首"),
        (1004, "下一首"),
        (1005, "桌面歌词"),
        (1006, "设置"),
        (1007, "退出 RPlayer"),
    ] {
        let text = wide(label);
        AppendMenuW(menu, MF_STRING, id, text.as_ptr());
    }
    MENU.set(menu as isize).ok();

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
        DestroyMenu(menu);
        DestroyWindow(hwnd);
        return None;
    }
    UpdateWindow(hwnd);
    Some(hwnd)
}

static MENU: OnceLock<isize> = OnceLock::new();

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
                show_menu(hwnd);
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

unsafe fn show_menu(hwnd: HWND) {
    let Some(menu) = MENU.get().copied() else {
        return;
    };
    let mut point: POINT = std::mem::zeroed();
    GetCursorPos(&mut point);
    SetForegroundWindow(hwnd);
    let command = TrackPopupMenu(
        menu as HMENU,
        TPM_RETURNCMD | TPM_RIGHTBUTTON,
        point.x,
        point.y,
        0,
        hwnd,
        std::ptr::null_mut(),
    );
    // Required so the popup dismisses when clicking elsewhere.
    PostMessageW(hwnd, WM_NULL, 0, 0);
    let action = match command {
        1001 => Some(TrayAction::Show),
        1002 => Some(TrayAction::TogglePlay),
        1003 => Some(TrayAction::Previous),
        1004 => Some(TrayAction::Next),
        1005 => Some(TrayAction::DesktopLyrics),
        1006 => Some(TrayAction::Settings),
        1007 => Some(TrayAction::Exit),
        _ => None,
    };
    if let Some(action) = action {
        push(action);
        restore_main_window();
    }
}

fn restore_main_window() {
    unsafe {
        use winapi::um::winuser::SetForegroundWindow;
        if let Some(hwnd) = crate::platform::main_window_handle() {
            let hwnd = hwnd as HWND;
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }
}
