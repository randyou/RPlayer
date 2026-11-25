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

use winapi::shared::guiddef::GUID;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HICON, HWND, POINT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::{
    ExtractIconExW, Shell_NotifyIconW, NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use winapi::um::winuser::{
    CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetMessageW, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
    ShowWindow, TranslateMessage, UpdateWindow, IDI_APPLICATION, SW_RESTORE, WM_APP, WM_CLOSE,
    WM_CONTEXTMENU, WM_DESTROY, WS_POPUP, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW,
};

/// Tooltip state pushed from the UI thread; the tray window adopts it on
/// `WM_TRAY_TIP` (design gap D2: three states, 500 ms delay handled by us).
static TRAY_TIP: OnceLock<Mutex<String>> = OnceLock::new();
static TRAY_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
static TRAY_ICON: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
const WM_TRAY_TIP: UINT = WM_APP + 3;

/// Fixed identity for the notification-area icon.
///
/// Without `NIF_GUID` the shell keys the icon to the executable image, so every
/// rebuilt binary registers as a brand-new icon and Windows 11 drops it into
/// the overflow flyout again - which reads as "the tray icon disappeared".
/// A stable GUID keeps the user's "show in the notification area" choice.
const TRAY_GUID: GUID = GUID {
    Data1: 0x5A6D_3E19,
    Data2: 0x8C24,
    Data3: 0x4F7B,
    Data4: [0x9B, 0x36, 0x2E, 0x51, 0x7A, 0x64, 0x0C, 0x11],
};

fn tip_slot() -> &'static Mutex<String> {
    TRAY_TIP.get_or_init(|| Mutex::new(String::from("RPlayer")))
}

/// Update the tray tooltip from the UI thread. Cheap when nothing changed.
pub fn set_tray_tooltip(text: &str) {
    let hwnd = TRAY_HWND.load(std::sync::atomic::Ordering::Relaxed);
    if hwnd == 0 {
        return;
    }
    let changed = match tip_slot().lock() {
        Ok(mut slot) => {
            if slot.as_str() == text {
                false
            } else {
                slot.clear();
                slot.push_str(text);
                true
            }
        }
        Err(_) => false,
    };
    if changed {
        unsafe {
            PostMessageW(hwnd as HWND, WM_TRAY_TIP, 0, 0);
        }
    }
}

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

/// Message a second app instance broadcasts to ask this one to show itself.
fn show_request_message() -> UINT {
    use std::sync::OnceLock;
    use winapi::um::winuser::RegisterWindowMessageW;

    static MSG: OnceLock<UINT> = OnceLock::new();
    *MSG.get_or_init(|| {
        let name: Vec<u16> = "RPlayerShowMainWindow"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe { RegisterWindowMessageW(name.as_ptr()) }
    })
}

/// Returns true when this process is the only player; false means another copy
/// already holds the session and this one should ask it to come forward and
/// exit (v2 regression A7: two copies used to mean two tray icons and two
/// always-on-top lyric strips).
pub fn single_instance_guard() -> bool {
    use std::sync::OnceLock;

    const ERROR_ALREADY_EXISTS: u32 = 183;

    // Declared here rather than pulling the `synchapi`/`errhandlingapi` winapi
    // features in for two calls.
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(
            attributes: *mut core::ffi::c_void,
            owner: i32,
            name: *const u16,
        ) -> *mut core::ffi::c_void;
        fn GetLastError() -> u32;
    }

    // The handle is kept for the process lifetime, which is what holds the
    // mutex; dropping it would release the guard.
    static HANDLE: OnceLock<isize> = OnceLock::new();
    let name: Vec<u16> = "Local\\RPlayer.SingleInstance"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr()) };
    let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    HANDLE.get_or_init(|| handle as isize);
    !already
}

/// Tells the already-running copy to show its window.
pub fn request_show_from_other_instance() {
    use winapi::um::winuser::{PostMessageW, HWND_BROADCAST};

    unsafe {
        PostMessageW(HWND_BROADCAST, show_request_message(), 0, 0);
    }
}

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
    TRAY_ICON.store(icon as isize, std::sync::atomic::Ordering::Relaxed);
    TRAY_HWND.store(hwnd as isize, std::sync::atomic::Ordering::Relaxed);
    if !add_icon(hwnd) {
        DestroyWindow(hwnd);
        return None;
    }
    UpdateWindow(hwnd);
    Some(hwnd)
}

/// Shell message broadcast when Explorer (re)starts. Icons added before the
/// restart are gone afterwards, so the tray has to re-register itself.
fn taskbar_created_message() -> UINT {
    use winapi::um::winuser::RegisterWindowMessageW;

    static MSG: OnceLock<UINT> = OnceLock::new();
    *MSG.get_or_init(|| {
        let name = wide("TaskbarCreated");
        unsafe { RegisterWindowMessageW(name.as_ptr()) }
    })
}

/// Fills the identity fields shared by every `Shell_NotifyIconW` call.
unsafe fn icon_data(hwnd: HWND, flags: UINT) -> NOTIFYICONDATAW {
    let mut data: NOTIFYICONDATAW = std::mem::zeroed();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 0;
    data.uFlags = flags | NIF_GUID;
    data.guidItem = TRAY_GUID;
    data
}

/// Adds (or re-adds) the icon under its stable GUID.
fn add_icon(hwnd: HWND) -> bool {
    unsafe {
        let mut data = icon_data(hwnd, NIF_MESSAGE | NIF_ICON | NIF_TIP);
        data.uCallbackMessage = WM_TRAY;
        data.hIcon = TRAY_ICON.load(std::sync::atomic::Ordering::Relaxed) as HICON;
        let current = match tip_slot().lock() {
            Ok(slot) => slot.clone(),
            Err(_) => String::from("RPlayer"),
        };
        let tip = wide(&current);
        let count = tip.len().min(data.szTip.len());
        data.szTip[..count].copy_from_slice(&tip[..count]);
        Shell_NotifyIconW(NIM_ADD, &mut data) != 0
    }
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
            if event == WM_LBUTTONDBLCLK {
                // Double click is transport (design gap D3): play / pause.
                push(TrayAction::TogglePlay);
            } else if event == WM_LBUTTONUP {
                push(TrayAction::Show);
                restore_main_window();
            } else if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                let mut point: POINT = std::mem::zeroed();
                GetCursorPos(&mut point);
                push(TrayAction::OpenMenu(point.x, point.y));
            }
            0
        }
        WM_TRAY_TIP => {
            let text = match tip_slot().lock() {
                Ok(slot) => slot.clone(),
                Err(_) => String::new(),
            };
            unsafe {
                let mut data = icon_data(hwnd, NIF_TIP);
                let tip = wide(&text);
                let count = tip.len().min(data.szTip.len());
                data.szTip[..count].copy_from_slice(&tip[..count]);
                Shell_NotifyIconW(NIM_MODIFY, &mut data);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                let mut data = icon_data(hwnd, 0);
                Shell_NotifyIconW(NIM_DELETE, &mut data);
            }
            PostQuitMessage(0);
            0
        }
        _ if message == taskbar_created_message() => {
            // Explorer restarted: every notification icon was dropped with it.
            add_icon(hwnd);
            0
        }
        _ if message == show_request_message() => {
            // A second copy of the app was started; it exits after asking the
            // running instance to come forward (single-instance behaviour).
            push(TrayAction::Show);
            restore_main_window();
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
