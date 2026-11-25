//! Small platform integration helpers that egui does not expose directly.

pub mod media;
pub mod smtc_client;
mod dcomp_panel;

/// Creates the Start Menu shortcut carrying `System.AppUserModel.ID` on first
/// run. Windows media controls use that shortcut to resolve the app's display
/// name instead of "Unknown app".
#[cfg(target_os = "windows")]
pub fn ensure_aumid_shortcut() {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let script = exe.with_file_name("register-aumid.ps1");
    if !script.is_file() {
        return;
    }
    let Ok(appdata) = std::env::var("APPDATA") else {
        return;
    };
    let shortcut = std::path::PathBuf::from(appdata)
        .join("Microsoft\\Windows\\Start Menu\\Programs\\RPlayer.lnk");
    if shortcut.is_file() {
        return;
    }
    let mut command = Command::new("pwsh");
    command
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script.to_string_lossy().as_ref(),
            "-Target",
            exe.to_string_lossy().as_ref(),
        ])
        .creation_flags(0x0800_0000);
    let _ = command.spawn();
}
/// Remembers the player window so later lookups do not mistake an
/// always-on-top overlay (lyrics strip, tray popup, mini player) for it. The
/// player is the largest visible window of this process whose title is not one
/// of the overlay titles; a cached handle that stopped existing is refreshed.
#[cfg(target_os = "windows")]
pub fn cache_main_window() -> isize {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use winapi::shared::windef::RECT;
    use winapi::um::winuser::{
        EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindow,
        IsWindowVisible,
    };

    static BEST: AtomicIsize = AtomicIsize::new(0);
    static AREA: AtomicIsize = AtomicIsize::new(0);
    static PID: AtomicIsize = AtomicIsize::new(0);

    let cached = MAIN_CACHE.load(Ordering::SeqCst);
    if cached != 0 && unsafe { IsWindow(cached as _) } != 0 {
        return cached;
    }

    unsafe extern "system" fn callback(hwnd: winapi::shared::windef::HWND, _: isize) -> i32 {
        let mut owner = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        if owner as isize != PID.load(Ordering::SeqCst)
            || unsafe { IsWindowVisible(hwnd) } == 0
        {
            return 1;
        }
        let mut title = [0u16; 128];
        let len = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
        let title = String::from_utf16_lossy(&title[..len.max(0) as usize]);
        if title.contains("桌面歌词") || title.contains("托盘菜单") || title.contains("迷你播放器")
        {
            return 1;
        }
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return 1;
        }
        let area = ((rect.right - rect.left).max(0) as isize)
            * ((rect.bottom - rect.top).max(0) as isize);
        // Ignore helper windows (winit's event target is 6x6); the player is
        // always a real window, so wait until one shows up instead of caching
        // whatever is on screen during the first frame.
        if area < 300_000 {
            return 1;
        }
        if area > AREA.load(Ordering::SeqCst) {
            AREA.store(area, Ordering::SeqCst);
            BEST.store(hwnd as isize, Ordering::SeqCst);
        }
        1
    }

    PID.store(std::process::id() as isize, Ordering::SeqCst);
    BEST.store(0, Ordering::SeqCst);
    AREA.store(0, Ordering::SeqCst);
    unsafe {
        EnumWindows(Some(callback), 0);
    }
    let found = BEST.load(Ordering::SeqCst);
    if found != 0 {
        MAIN_CACHE.store(found, Ordering::SeqCst);
    }
    found
}

#[cfg(target_os = "windows")]
fn cached_main_window() -> Option<isize> {
    use std::sync::atomic::Ordering;
    #[link(name = "user32")]
    extern "system" {
        fn IsWindow(hwnd: isize) -> i32;
    }
    let hwnd = MAIN_CACHE.load(Ordering::SeqCst);
    if hwnd != 0 && unsafe { IsWindow(hwnd) } != 0 {
        Some(hwnd)
    } else {
        None
    }
}

static MAIN_CACHE: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[cfg(target_os = "windows")]
pub mod tray;

/// First visible top-level window owned by this process. Used by the SMTC
/// helper, which needs the main HWND to attach the system media card.
#[cfg(target_os = "windows")]
pub fn main_window_handle() -> Option<isize> {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use winapi::shared::windef::RECT;
    use winapi::um::winuser::{
        EnumWindows, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
        IsWindowVisible,
    };

    static FOUND: AtomicIsize = AtomicIsize::new(0);
    static BEST_AREA: AtomicIsize = AtomicIsize::new(0);
    static PID: AtomicIsize = AtomicIsize::new(0);

    unsafe extern "system" fn callback(hwnd: winapi::shared::windef::HWND, _: isize) -> i32 {
        let mut owner = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        if owner as isize != PID.load(Ordering::SeqCst)
            || unsafe { IsWindowVisible(hwnd) } == 0
        {
            return 1;
        }
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return 1;
        }
        let area = ((rect.right - rect.left).max(0) as isize)
            * ((rect.bottom - rect.top).max(0) as isize);
        if area > BEST_AREA.load(Ordering::SeqCst) {
            BEST_AREA.store(area, Ordering::SeqCst);
            FOUND.store(hwnd as isize, Ordering::SeqCst);
        }
        1
    }

    let process = std::process::id() as isize;
    PID.store(process, Ordering::SeqCst);
    if let Some(cached) = cached_main_window() {
        return Some(cached);
    }
    let foreground = unsafe { GetForegroundWindow() };
    if !foreground.is_null() {
        let mut owner = 0u32;
        unsafe { GetWindowThreadProcessId(foreground, &mut owner) };
        if owner as isize == process {
            return Some(foreground as isize);
        }
    }

    FOUND.store(0, Ordering::SeqCst);
    BEST_AREA.store(0, Ordering::SeqCst);
    unsafe {
        EnumWindows(Some(callback), 0);
    }
    match FOUND.load(Ordering::SeqCst) {
        0 => None,
        hwnd => Some(hwnd),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn main_window_handle() -> Option<isize> {
    None
}

/// Ask the Windows compositor to use a dark or light native title bar.
///
/// The main HWND is discovered once by process id so switching themes after the
/// window title has changed still hits the right window (and never one of the
/// always-on-top overlay viewports).
#[cfg(target_os = "windows")]
pub fn apply_titlebar(dark: bool) {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicIsize, Ordering};

    static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(
            callback: Option<unsafe extern "system" fn(isize, isize) -> i32>,
            lparam: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, process_id: *mut u32) -> u32;
        fn IsWindowVisible(hwnd: isize) -> i32;
        fn GetWindowRect(hwnd: isize, rect: *mut RectI) -> i32;
    }

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            attribute: u32,
            value: *const c_void,
            value_size: u32,
        ) -> i32;
    }

    #[repr(C)]
    struct RectI {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    unsafe extern "system" fn find_main(hwnd: isize, lparam: isize) -> i32 {
        let mut process_id = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
        if process_id != std::process::id() || unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }
        let mut rect = RectI {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return 1;
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width >= 800 && height >= 520 {
            let target = lparam as *mut isize;
            unsafe { *target = hwnd };
            return 0;
        }
        1
    }

    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_USE_IMMERSIVE_DARK_MODE_PRE_20H1: u32 = 19;

    let mut hwnd = MAIN_HWND.load(Ordering::Relaxed);
    if hwnd == 0 {
        let mut found: isize = 0;
        unsafe {
            EnumWindows(Some(find_main), &mut found as *mut isize as isize);
        }
        if found == 0 {
            return;
        }
        hwnd = found;
        MAIN_HWND.store(found, Ordering::Relaxed);
    }

    let enabled: u32 = u32::from(dark);
    let value = &enabled as *const u32 as *const c_void;
    unsafe {
        // Windows 11 / Windows 10 20H1+; fall back for older Windows 10 builds.
        if DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, value, 4) != 0 {
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE_PRE_20H1, value, 4);
        }
    }
}

/// Applies a rounded native window region to a borderless overlay. Without
/// this, Windows keeps the rectangular black backing behind transparent egui
/// viewports; the region gives the lyrics bar and mini player a clean capsule
/// silhouette on every desktop.
#[cfg(target_os = "windows")]
pub fn apply_round_window(title: &str, radius: u32) -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn GetClientRect(hwnd: isize, rect: *mut RectI) -> i32;
        fn GetDpiForWindow(hwnd: isize) -> u32;
        fn SetWindowRgn(hwnd: isize, region: isize, redraw: i32) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRoundRectRgn(
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
            ellipse_width: i32,
            ellipse_height: i32,
        ) -> isize;
    }

    #[repr(C)]
    struct RectI {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd == 0 {
        return false;
    }
    let mut rect = RectI {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
        return false;
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return false;
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let dpi = if dpi == 0 { 96 } else { dpi };
    let scaled = (radius as u64 * dpi as u64 / 96) as i32;
    let region = unsafe {
        CreateRoundRectRgn(
            0,
            0,
            width + 1,
            height + 1,
            scaled * 2,
            scaled * 2,
        )
    };
    if region == 0 {
        return false;
    }
    unsafe { SetWindowRgn(hwnd, region, 1) != 0 }
}

#[cfg(not(target_os = "windows"))]
pub fn apply_round_window(_title: &str, _radius: u32) -> bool {
    true
}

/// Makes a borderless overlay translucent as a whole. Windows' GL surface does
/// not always preserve per-pixel alpha for child viewports, but a layered
/// window with a constant alpha reliably shows the desktop through the lyric
/// card and keeps the text crisp at normal opacity levels.
#[cfg(target_os = "windows")]
pub fn set_window_alpha(title: &str, opacity: f32) -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn GetWindowLongPtrW(hwnd: isize, index: i32) -> isize;
        fn SetWindowLongPtrW(hwnd: isize, index: i32, value: isize) -> isize;
        fn SetLayeredWindowAttributes(
            hwnd: isize,
            color_key: u32,
            alpha: u8,
            flags: u32,
        ) -> i32;
    }

    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_LAYERED: isize = 0x0008_0000;
    const LWA_ALPHA: u32 = 0x0000_0002;

    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd == 0 {
        return false;
    }
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    if style & WS_EX_LAYERED == 0 {
        unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED) };
    }
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    unsafe { SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA) != 0 }
}

/// Enables color-key transparency for the fully transparent lyric panel.
/// The GL surface does not expose per-pixel alpha on this renderer; a keyed
/// background lets the desktop show through while text stays opaque.
#[cfg(target_os = "windows")]
pub fn set_window_color_key(title: &str, enabled: bool) -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn GetWindowLongPtrW(hwnd: isize, index: i32) -> isize;
        fn SetWindowLongPtrW(hwnd: isize, index: i32, value: isize) -> isize;
        fn SetLayeredWindowAttributes(
            hwnd: isize,
            color_key: u32,
            alpha: u8,
            flags: u32,
        ) -> i32;
        fn SetWindowPos(
            hwnd: isize,
            after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
        fn GetClientRect(hwnd: isize, rect: *mut RectI) -> i32;
        fn SetWindowRgn(hwnd: isize, region: isize, redraw: i32) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> isize;
    }

    #[repr(C)]
    struct RectI {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_LAYERED: isize = 0x0008_0000;
    const WS_EX_NOACTIVATE: isize = 0x0800_0000;
    const LWA_ALPHA: u32 = 0x0000_0002;
    const LWA_COLORKEY: u32 = 0x0000_0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    // COLORREF is 0x00BBGGRR. Deliberately an unusual color.
    const KEY: u32 = 0x00A0_A0A0;

    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd == 0 {
        return false;
    }
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    // An always-on-top overlay must never take activation: while it held the
    // foreground, the first press on the player's title bar only activated the
    // player again and the drag gesture was swallowed (v2.1.0 issue #2).
    let mut next = style | WS_EX_NOACTIVATE;
    if next & WS_EX_LAYERED == 0 {
        next |= WS_EX_LAYERED;
    }
    if next != style {
        unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next) };
    }
    unsafe {
        let result = if enabled {
            SetLayeredWindowAttributes(hwnd, KEY, 255, LWA_COLORKEY) != 0
        } else {
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA) != 0
        };
        if enabled {
            let mut rect = RectI {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetClientRect(hwnd, &mut rect) != 0 {
                // Drop the outermost pixel so DWM's dark frame cannot show
                // through the color-key surface.
                let region = CreateRectRgn(
                    1,
                    1,
                    (rect.right - 1).max(2),
                    (rect.bottom - 1).max(2),
                );
                if region != 0 {
                    SetWindowRgn(hwnd, region, 1);
                }
            }
        } else {
            SetWindowRgn(hwnd, 0, 1);
        }
        SetWindowPos(
            hwnd,
            0,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
        result
    }
}

/// Brings the player window back to the foreground. Used right after an
/// overlay appears, because Windows hands the foreground to the new window
/// even when it never activates (v2.1.0 issue #2).
#[cfg(target_os = "windows")]
pub fn focus_main_window() {
    #[link(name = "user32")]
    extern "system" {
        fn SetForegroundWindow(hwnd: isize) -> i32;
        fn GetForegroundWindow() -> isize;
    }

    if let Some(main) = main_window_handle() {
        if main != 0 {
            let ok = unsafe { SetForegroundWindow(main) };
            crate::platform::drag_probe(
                "foreground",
                &format!(
                    "focus main={main} ok={ok} now={}",
                    unsafe { GetForegroundWindow() }
                ),
            );
        }
    }
}

/// Windows hands the foreground to a freshly created top-level window even
/// when it is non-activating. While that overlay owns the foreground the first
/// press on the player's title bar is spent re-activating the player and the
/// drag gesture is swallowed, so give it straight back.
#[cfg(target_os = "windows")]
pub fn release_overlay_foreground(title: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> isize;
        fn GetWindowTextW(hwnd: isize, buf: *mut u16, count: i32) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
    }

    let Some(main) = main_window_handle() else {
        crate::platform::drag_probe("foreground", "no-main");
        return;
    };
    let fg = unsafe { GetForegroundWindow() };
    if fg == 0 || fg == main {
        return;
    }
    // Compare by title on the window that actually owns the foreground;
    // FindWindow is unreliable while an older hidden overlay still exists.
    let mut buf = [0u16; 64];
    let len = unsafe { GetWindowTextW(fg, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return;
    }
    let current = String::from_utf16_lossy(&buf[..len as usize]);
    if current != title {
        return;
    }
    let ok = unsafe { SetForegroundWindow(main) };
    crate::platform::drag_probe(
        "foreground",
        &format!(
            "handback main={main} overlay={fg} ok={ok} now={}",
            unsafe { GetForegroundWindow() }
        ),
    );
}

/// Disables the Windows 11 DWM border (and rounded corners) on an overlay.
#[cfg(target_os = "windows")]
pub fn set_window_borderless(title: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
    }
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            attribute: u32,
            value: *const std::ffi::c_void,
            size: u32,
        ) -> i32;
    }

    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd == 0 {
        return;
    }
    // DWMWA_BORDER_COLOR = 34, DWMWA_COLOR_NONE = 0xFFFFFFFE
    let no_border: u32 = 0xFFFF_FFFE;
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            34,
            &no_border as *const u32 as *const std::ffi::c_void,
            4,
        );
        // DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_DONOTROUND = 1
        let square: u32 = 1;
        DwmSetWindowAttribute(
            hwnd,
            33,
            &square as *const u32 as *const std::ffi::c_void,
            4,
        );
        // DWMWA_NCRENDERING_POLICY = 2, DWMNCRP_DISABLED = 1
        let no_frame: u32 = 1;
        DwmSetWindowAttribute(
            hwnd,
            2,
            &no_frame as *const u32 as *const std::ffi::c_void,
            4,
        );
    }
}

#[cfg(target_os = "windows")]
pub fn cursor_in_window(title: &str) -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn GetWindowRect(hwnd: isize, rect: *mut RectI) -> i32;
        fn GetCursorPos(point: *mut PointI) -> i32;
    }
    #[repr(C)]
    struct RectI {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct PointI {
        x: i32,
        y: i32,
    }

    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd == 0 {
        return false;
    }
    let mut rect = RectI {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut point = PointI { x: 0, y: 0 };
    unsafe {
        if GetWindowRect(hwnd, &mut rect) == 0 || GetCursorPos(&mut point) == 0 {
            return false;
        }
    }
    point.x >= rect.left
        && point.x < rect.right
        && point.y >= rect.top
        && point.y < rect.bottom
}

#[cfg(not(target_os = "windows"))]
pub fn cursor_in_window(_title: &str) -> bool {
    false
}

/// True while the left mouse button is held down and the cursor sits outside
/// the window with this title. The tray popup is created without activation
/// (a focus-stealing undecorated transparent window fails to be created on
/// Windows), so it cannot use focus loss to dismiss itself.
#[cfg(target_os = "windows")]
pub fn left_button_outside(title: &str) -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn GetAsyncKeyState(key: i32) -> i16;
    }
    const VK_LBUTTON: i32 = 0x01;
    let down = unsafe { GetAsyncKeyState(VK_LBUTTON) as u16 & 0x8000 != 0 };
    down && !cursor_in_window(title)
}

#[cfg(not(target_os = "windows"))]
pub fn left_button_outside(_title: &str) -> bool {
    false
}

/// Work area (left, top, right, bottom) in physical pixels of the monitor that
/// contains the cursor. The tray popup anchors to the notification area, which
/// lives on a specific monitor, and egui's `monitor_size` is `None` for the
/// root viewport - so it has to ask Win32 instead of guessing 1920x1080.
#[cfg(target_os = "windows")]
pub fn cursor_monitor_work_area() -> Option<(f32, f32, f32, f32)> {
    #[link(name = "user32")]
    extern "system" {
        fn MonitorFromPoint(point: PointI, flags: u32) -> isize;
        fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
    }
    #[repr(C)]
    struct PointI {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    struct RectI {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct MonitorInfo {
        size: u32,
        monitor: RectI,
        work: RectI,
        flags: u32,
    }
    const MONITOR_DEFAULTTONEAREST: u32 = 2;

    #[link(name = "user32")]
    extern "system" {
        fn GetCursorPos(point: *mut PointI) -> i32;
    }

    let mut point = PointI { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return None;
    }
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    if monitor == 0 {
        return None;
    }
    let mut info = MonitorInfo {
        size: std::mem::size_of::<MonitorInfo>() as u32,
        monitor: RectI {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        work: RectI {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        flags: 0,
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    Some((
        info.work.left as f32,
        info.work.top as f32,
        info.work.right as f32,
        info.work.bottom as f32,
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn cursor_monitor_work_area() -> Option<(f32, f32, f32, f32)> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn set_window_borderless(_title: &str) {}

#[cfg(not(target_os = "windows"))]
pub fn set_window_color_key(_title: &str, _enabled: bool) -> bool {
    true
}

#[cfg(not(target_os = "windows"))]
pub fn set_window_alpha(_title: &str, _opacity: f32) -> bool {
    true
}

#[cfg(not(target_os = "windows"))]
pub fn apply_titlebar(_dark: bool) {}

/// Independent per-pixel-alpha panel window behind the lyric text window.
/// `UpdateLayeredWindow` does not depend on the GL/WGPU swapchain alpha, so
/// panel opacity and text opacity are fully independent.
#[cfg(target_os = "windows")]
pub fn update_lyrics_panel(
    title: &str,
    rect: (i32, i32, i32, i32),
    alpha: f32,
    rgb: (u8, u8, u8),
    radius: f32,
) {
    if dcomp_panel::update(title, rect, alpha, rgb) {
        return;
    }
    update_lyrics_panel_impl(title, rect, alpha, rgb, radius);
}

#[cfg(target_os = "windows")]
pub fn hide_lyrics_panel() {
    update_lyrics_panel("RPlayer 桌面歌词", (0, 0, 0, 0), 0.0, (0, 0, 0), 0.0);
}

#[cfg(not(target_os = "windows"))]
pub fn hide_lyrics_panel() {}

#[cfg(target_os = "windows")]
fn update_lyrics_panel_impl(
    title: &str,
    rect: (i32, i32, i32, i32),
    alpha: f32,
    rgb: (u8, u8, u8),
    radius: f32,
) {
    use std::sync::atomic::{AtomicIsize, Ordering};

    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn GetModuleHandleW(name: *const u16) -> isize;
        fn RegisterClassW(class: *const PanelClass) -> u16;
        fn CreateWindowExW(
            ex_style: u32, class_name: *const u16, window_name: *const u16,
            style: u32, x: i32, y: i32, w: i32, h: i32,
            parent: isize, menu: isize, instance: isize,
            param: *mut std::ffi::c_void,
        ) -> isize;
        fn DefWindowProcW(hwnd: isize, msg: u32, wp: usize, lp: isize) -> isize;
        fn SetWindowPos(hwnd: isize, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
        fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        fn GetDC(hwnd: isize) -> isize;
        fn ReleaseDC(hwnd: isize, dc: isize) -> i32;
        fn UpdateLayeredWindow(
            hwnd: isize, screen: isize, dst: *const PointI, size: *const SizeI,
            src: isize, src_pos: *const PointI, color_key: u32,
            blend: *const Blend, flags: u32,
        ) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateCompatibleDC(dc: isize) -> isize;
        fn DeleteDC(dc: isize) -> i32;
        fn CreateDIBSection(
            dc: isize, info: *const BitmapInfo, usage: u32,
            bits: *mut *mut std::ffi::c_void, section: isize, offset: u32,
        ) -> isize;
        fn SelectObject(dc: isize, object: isize) -> isize;
        fn DeleteObject(object: isize) -> i32;
    }

    #[repr(C)]
    struct PanelClass {
        style: u32,
        wnd_proc: Option<unsafe extern "system" fn(isize, u32, usize, isize) -> isize>,
        cls_extra: i32, wnd_extra: i32, instance: isize,
        icon: isize, cursor: isize, background: isize,
        menu_name: *const u16, class_name: *const u16,
    }
    #[repr(C)]
    struct PointI { x: i32, y: i32 }
    #[repr(C)]
    struct SizeI { cx: i32, cy: i32 }
    #[repr(C)]
    struct Blend { blend_op: u8, blend_flags: u8, alpha: u8, format: u8 }
    #[repr(C)]
    struct BitmapInfoHeader {
        size: u32, width: i32, height: i32, planes: u16, bit_count: u16,
        compression: u32, size_image: u32, x_pels: i32, y_pels: i32,
        clr_used: u32, clr_important: u32,
    }
    #[repr(C)]
    struct BitmapInfo { header: BitmapInfoHeader, colors: [u32; 3] }

    unsafe extern "system" fn proc(
        hwnd: isize, msg: u32, wp: usize, lp: isize,
    ) -> isize {
        DefWindowProcW(hwnd, msg, wp, lp)
    }

    static PANEL: AtomicIsize = AtomicIsize::new(0);
    static REGISTERED: AtomicIsize = AtomicIsize::new(0);
    let class_name: Vec<u16> = "RPlayerLyricPanel\0".encode_utf16().collect();
    let mut hwnd = PANEL.load(Ordering::SeqCst);
    if hwnd == 0 {
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            if REGISTERED.swap(1, Ordering::SeqCst) == 0 {
                let class = PanelClass {
                    style: 0, wnd_proc: Some(proc), cls_extra: 0, wnd_extra: 0,
                    instance, icon: 0, cursor: 0, background: 0,
                    menu_name: std::ptr::null(), class_name: class_name.as_ptr(),
                };
                RegisterClassW(&class);
            }
            hwnd = CreateWindowExW(
                0x0008_0000 | 0x0000_0020 | 0x0000_0080 | 0x0000_0008,
                class_name.as_ptr(), std::ptr::null(), 0x8000_0000,
                0, 0, 0, 0, 0, 0, instance, std::ptr::null_mut(),
            );
            if hwnd == 0 { return; }
            PANEL.store(hwnd, Ordering::SeqCst);
        }
    }
    let _ = std::fs::write(
        std::env::temp_dir().join("rplayer-panel.log"),
        format!("alpha={alpha} rect={rect:?}\n"),
    );
    if alpha <= 0.001 {
        unsafe { ShowWindow(hwnd, 0); }
        return;
    }
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let owner = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if owner == 0 { return; }
    let w = rect.2.max(1);
    let h = rect.3.max(1);
    let a = (alpha.clamp(0.0, 1.0) * 255.0) as u32;
    let info = BitmapInfo {
        header: BitmapInfoHeader {
            size: std::mem::size_of::<BitmapInfoHeader>() as u32,
            width: w, height: -h, planes: 1, bit_count: 32,
            compression: 0, size_image: 0, x_pels: 0, y_pels: 0,
            clr_used: 0, clr_important: 0,
        },
        colors: [0; 3],
    };
    let screen = unsafe { GetDC(0) };
    let mem = unsafe { CreateCompatibleDC(screen) };
    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    let bitmap = unsafe {
        CreateDIBSection(mem, &info, 0, &mut bits, 0, 0)
    };
    if bitmap == 0 || bits.is_null() {
        unsafe { DeleteDC(mem); ReleaseDC(0, screen); }
        return;
    }
    let old = unsafe { SelectObject(mem, bitmap) };
    let pixels = unsafe { std::slice::from_raw_parts_mut(bits as *mut u32, (w * h) as usize) };
    let r = rgb.0 as u32;
    let g = rgb.1 as u32;
    let b = rgb.2 as u32;
    let radius = radius.max(0.0);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32;
            let fy = y as f32;
            let inside = if fx < radius && fy < radius {
                let dx = radius - fx; let dy = radius - fy;
                dx * dx + dy * dy <= radius * radius
            } else if fx >= w as f32 - radius && fy < radius {
                let dx = fx - (w as f32 - radius); let dy = radius - fy;
                dx * dx + dy * dy <= radius * radius
            } else if fx < radius && fy >= h as f32 - radius {
                let dx = radius - fx; let dy = fy - (h as f32 - radius);
                dx * dx + dy * dy <= radius * radius
            } else if fx >= w as f32 - radius && fy >= h as f32 - radius {
                let dx = fx - (w as f32 - radius); let dy = fy - (h as f32 - radius);
                dx * dx + dy * dy <= radius * radius
            } else { true };
            let aa = if inside { a } else { 0 };
            pixels[(y * w + x) as usize] = (aa << 24)
                | (((r * aa) / 255) << 16)
                | (((g * aa) / 255) << 8)
                | ((b * aa) / 255);
        }
    }
    let dst = PointI { x: rect.0, y: rect.1 };
    let size = SizeI { cx: w, cy: h };
    let src_pos = PointI { x: 0, y: 0 };
    let blend = Blend { blend_op: 0, blend_flags: 0, alpha: 255, format: 1 };
    unsafe {
        SetWindowPos(hwnd, owner, rect.0, rect.1, w, h, 0x0010 | 0x0040);
        ShowWindow(hwnd, 4);
        UpdateLayeredWindow(hwnd, screen, &dst, &size, mem, &src_pos, 0, &blend, 2);
        SelectObject(mem, old);
        DeleteObject(bitmap);
        DeleteDC(mem);
        ReleaseDC(0, screen);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn update_lyrics_panel(_title: &str, _rect: (i32, i32, i32, i32), _alpha: f32, _rgb: (u8, u8, u8), _radius: f32) {}
static LYRICS_OLD_PROC: std::sync::atomic::AtomicIsize =
    std::sync::atomic::AtomicIsize::new(0);
static LYRICS_TARGET: std::sync::atomic::AtomicIsize =
    std::sync::atomic::AtomicIsize::new(0);

#[cfg(target_os = "windows")]
unsafe extern "system" fn lyrics_native_proc(
    hwnd: isize,
    msg: u32,
    wp: usize,
    lp: isize,
) -> isize {
    use std::sync::atomic::Ordering;
    #[link(name = "user32")]
    extern "system" {
        fn GetClientRect(hwnd: isize, rect: *mut RectI) -> i32;
        fn GetWindowRect(hwnd: isize, rect: *mut RectI) -> i32;
        fn ScreenToClient(hwnd: isize, point: *mut PointI) -> i32;
        fn ClientToScreen(hwnd: isize, point: *mut PointI) -> i32;
        fn CallWindowProcW(prev: isize, hwnd: isize, msg: u32, wp: usize, lp: isize) -> isize;
        fn DefWindowProcW(hwnd: isize, msg: u32, wp: usize, lp: isize) -> isize;
    }
    #[repr(C)]
    struct RectI { left: i32, top: i32, right: i32, bottom: i32 }
    #[repr(C)]
    struct PointI { x: i32, y: i32 }

    if msg == 0x0084 {
        let mut point = PointI {
            x: (lp as i32) & 0xFFFF,
            y: ((lp as i32) >> 16) & 0xFFFF,
        };
        let mut rect = RectI { left: 0, top: 0, right: 0, bottom: 0 };
        unsafe {
            ScreenToClient(hwnd, &mut point);
            GetClientRect(hwnd, &mut rect);
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let controls = point.x < 230 && point.y < 130
            || point.x > width - 56 && point.y < 46;
        if controls {
            return 1; // HTCLIENT: egui draws and hit-tests its own buttons.
        }
        // The lyric window is a wide always-on-top strip, but only the capsule
        // is visible. Clicks outside it have to fall through to whatever is
        // behind, otherwise the overlay blocks the main window's title bar.
        let inset = 16;
        let inside = point.x >= inset
            && point.y >= inset
            && point.x <= width - inset
            && point.y <= height - inset;
        if !inside {
            return -1; // HTTRANSPARENT
        }
        // Never swallow the main window's own drag strip: when the overlay is
        // parked over the title bar, a press there has to move the player, not
        // the lyric strip. WM_NCHITTEST already reports screen coordinates.
        let screen_point = PointI {
            x: (lp as i32) & 0xFFFF,
            y: ((lp as i32) >> 16) & 0xFFFF,
        };
        unsafe {
            if let Some(main) = crate::platform::main_window_handle() {
                let mut main_rect = RectI { left: 0, top: 0, right: 0, bottom: 0 };
                if GetWindowRect(main, &mut main_rect) != 0 {
                    let strip = main_rect.top + (crate::ui::tokens::TOPBAR_HEIGHT as i32);
                    if screen_point.y >= main_rect.top
                        && screen_point.y <= strip
                        && screen_point.x >= main_rect.left
                        && screen_point.x <= main_rect.right
                    {
                        return -1;
                    }
                }
            }
        }
        return 2; // HTCAPTION: drag the lyric strip.
    }
    // The lyric strip is a non-activating overlay, but Windows still treats it
    // as a normal window when clicked: the click activates the overlay, the
    // player loses the foreground, and the very next title-bar press is spent
    // re-activating the player instead of starting a drag (v2.1.0 issue #2).
    if msg == 0x0021 {
        return 3; // MA_NOACTIVATE
    }
    if msg == 0x0006 {
        // WM_ACTIVATE: if something still activated us, hand the foreground
        // back to the player immediately.
        if (wp & 0xFFFF) != 0 {
            if let Some(main) = crate::platform::main_window_handle() {
                if main != 0 && main != hwnd {
                    extern "system" {
                        fn SetForegroundWindow(hwnd: isize) -> i32;
                    }
                    unsafe { SetForegroundWindow(main) };
                }
            }
        }
        return 0;
    }
    // The system runs a modal move loop while the caption is dragged, so the
    // event loop stops painting. Hide the DComp capsule for that window instead
    // of letting it trail behind the text.
    if msg == 0x0231 {
        crate::platform::dcomp_panel::set_visible(false);
        return 0;
    }
    if msg == 0x0232 {
        crate::platform::dcomp_panel::set_visible(true);
        return 0;
    }
    let old = LYRICS_OLD_PROC.load(Ordering::SeqCst);
    unsafe {
        if old != 0 {
            CallWindowProcW(old, hwnd, msg, wp, lp)
        } else {
            DefWindowProcW(hwnd, msg, wp, lp)
        }
    }
}

/// Development aid: appends one line to `RPLAYER_DRAG_DEBUG` so a drag that
/// never reaches the window manager can be told apart from one that does.
pub fn drag_probe(id: &str, event: &str) {
    let Ok(path) = std::env::var("RPLAYER_DRAG_DEBUG") else {
        return;
    };
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{event} {id}");
    }
}

/// Clips a window to a rounded rectangle so the corners outside the drawn
/// panel do not fall back to the framebuffer clear colour. eframe clears each
/// viewport with the panel fill before egui paints, and a transparent popup
/// cannot drop that fill, so the OS has to cut the corners instead.
#[cfg(target_os = "windows")]
pub fn set_rounded_region(title: &str, radius: f32) -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn GetWindowRect(hwnd: isize, rect: *mut RectI) -> i32;
        fn SetWindowRgn(hwnd: isize, region: isize, redraw: i32) -> i32;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRoundRectRgn(
            left: i32, top: i32, right: i32, bottom: i32,
            width: i32, height: i32,
        ) -> isize;
    }
    #[repr(C)]
    struct RectI {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd == 0 {
        return false;
    }
    let mut rect = RectI { left: 0, top: 0, right: 0, bottom: 0 };
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return false;
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return false;
    }
    let diameter = (radius * 2.0).max(2.0) as i32;
    let region = unsafe { CreateRoundRectRgn(0, 0, width + 1, height + 1, diameter, diameter) };
    if region == 0 {
        return false;
    }
    // SetWindowRgn takes ownership of the region.
    unsafe { SetWindowRgn(hwnd, region, 1) != 0 }
}

#[cfg(not(target_os = "windows"))]
pub fn set_rounded_region(_title: &str, _radius: f32) -> bool {
    false
}

/// Lets the user drag the lyric window from any non-control area.
#[cfg(target_os = "windows")]
pub fn enable_lyrics_native_drag(title: &str) {
    use std::sync::atomic::Ordering;
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn SetWindowLongPtrW(hwnd: isize, index: i32, value: isize) -> isize;
    }
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd == 0 {
        return;
    }
    // The lyric viewport is recreated every time the overlay is toggled, and a
    // new HWND starts with the stock window proc - so the subclass has to be
    // reinstalled rather than latched once per process.
    if LYRICS_TARGET.load(Ordering::SeqCst) == hwnd {
        return;
    }
    let old = unsafe {
        SetWindowLongPtrW(hwnd, -4, lyrics_native_proc as *const () as isize)
    };
    if old != 0 {
        LYRICS_OLD_PROC.store(old, Ordering::SeqCst);
        LYRICS_TARGET.store(hwnd, Ordering::SeqCst);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn enable_lyrics_native_drag(_title: &str) {}