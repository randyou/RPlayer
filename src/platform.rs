//! Small platform integration helpers that egui does not expose directly.

/// Ask the Windows compositor to render our native title bar in dark mode.
///
/// egui's `ViewportCommand::SetTheme` relies on winit, which does not always
/// update the non-client area on Windows 10/11. DwmSetWindowAttribute is the
/// documented switch and makes the frame match the app's midnight surfaces.
#[cfg(target_os = "windows")]
pub fn apply_dark_titlebar() {
    use std::ffi::c_void;

    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
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

    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_USE_IMMERSIVE_DARK_MODE_PRE_20H1: u32 = 19;

    let title: Vec<u16> = "RPlayer\0".encode_utf16().collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if hwnd == 0 {
        return;
    }

    let enabled: u32 = 1;
    let value = &enabled as *const u32 as *const c_void;
    unsafe {
        // Windows 11 / Windows 10 20H1+; fall back for older Windows 10 builds.
        if DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, value, 4) != 0 {
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE_PRE_20H1, value, 4);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn apply_dark_titlebar() {}
