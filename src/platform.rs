//! Small platform integration helpers that egui does not expose directly.

pub mod media;

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

#[cfg(not(target_os = "windows"))]
pub fn set_window_alpha(_title: &str, _opacity: f32) -> bool {
    true
}

#[cfg(not(target_os = "windows"))]
pub fn apply_titlebar(_dark: bool) {}
