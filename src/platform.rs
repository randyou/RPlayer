//! Small platform integration helpers that egui does not expose directly.

pub mod media;
pub mod smtc_client;

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
    const LWA_ALPHA: u32 = 0x0000_0002;
    const LWA_COLORKEY: u32 = 0x0000_0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    // COLORREF is 0x00BBGGRR. Deliberately an unusual color.
    const KEY: u32 = 0x0003_0201;

    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd == 0 {
        return false;
    }
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    if style & WS_EX_LAYERED == 0 {
        unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED) };
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

/// Frosted-glass backdrop behind the lyric text window. Keeping it in a
/// separate native window means panel opacity never affects text opacity.
#[cfg(target_os = "windows")]
pub fn update_lyrics_backdrop(
    title: &str,
    rect: (i32, i32, i32, i32),
    alpha: f32,
    rgb: (u8, u8, u8),
) {
    use std::sync::atomic::{AtomicIsize, Ordering};

    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn GetModuleHandleW(name: *const u16) -> isize;
        fn RegisterClassW(class: *const BackdropClass) -> u16;
        fn CreateWindowExW(
            ex_style: u32,
            class_name: *const u16,
            window_name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            parent: isize,
            menu: isize,
            instance: isize,
            param: *mut std::ffi::c_void,
        ) -> isize;
        fn DefWindowProcW(hwnd: isize, msg: u32, wp: usize, lp: isize) -> isize;
        fn SetWindowPos(
            hwnd: isize,
            after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
        fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        fn SetLayeredWindowAttributes(
            hwnd: isize,
            color_key: u32,
            alpha: u8,
            flags: u32,
        ) -> i32;
        fn BeginPaint(hwnd: isize, paint: *mut PaintStruct) -> isize;
        fn EndPaint(hwnd: isize, paint: *const PaintStruct) -> i32;
        fn FillRect(hdc: isize, rect: *const RectI, brush: isize) -> i32;
        fn GetClientRect(hwnd: isize, rect: *mut RectI) -> i32;
        fn InvalidateRect(hwnd: isize, rect: *const RectI, erase: i32) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateSolidBrush(color: u32) -> isize;
        fn DeleteObject(object: isize) -> i32;
    }

    #[repr(C)]
    struct RectI {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct PaintStruct {
        hdc: isize,
        f_erase: i32,
        rc_paint: RectI,
        f_restore: i32,
        f_inc_update: i32,
        reserved: [u8; 32],
    }

    #[repr(C)]
    struct BackdropClass {
        style: u32,
        wnd_proc: Option<unsafe extern "system" fn(isize, u32, usize, isize) -> isize>,
        cls_extra: i32,
        wnd_extra: i32,
        instance: isize,
        icon: isize,
        cursor: isize,
        background: isize,
        menu_name: *const u16,
        class_name: *const u16,
    }
    #[repr(C)]
    struct AccentPolicy {
        state: i32,
        flags: i32,
        gradient_color: u32,
        animation_id: i32,
    }
    #[repr(C)]
    struct CompositionData {
        attribute: i32,
        data: *mut std::ffi::c_void,
        size: usize,
    }

    static BACKDROP_COLOR: std::sync::atomic::AtomicU32 =
        std::sync::atomic::AtomicU32::new(0x00F2_F2F2);

    unsafe extern "system" fn proc(
        hwnd: isize,
        msg: u32,
        wp: usize,
        lp: isize,
    ) -> isize {
        if msg == 0x000F {
            // WM_PAINT: fill with the current panel tint. Layered alpha then
            // blends the solid tint with the desktop so content behind shows.
            let mut paint: PaintStruct = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut paint);
            let mut rect: RectI = std::mem::zeroed();
            GetClientRect(hwnd, &mut rect);
            let brush = CreateSolidBrush(BACKDROP_COLOR.load(Ordering::SeqCst));
            if brush != 0 {
                FillRect(hdc, &rect, brush);
                DeleteObject(brush);
            }
            EndPaint(hwnd, &paint);
            return 0;
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }

    static BACKDROP: AtomicIsize = AtomicIsize::new(0);
    static REGISTERED: AtomicIsize = AtomicIsize::new(0);

    let class_name = "RPlayerLyricBackdrop\0".encode_utf16().collect::<Vec<_>>();
    let mut hwnd = BACKDROP.load(Ordering::SeqCst);
    if hwnd == 0 {
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            if REGISTERED.swap(1, Ordering::SeqCst) == 0 {
                let class = BackdropClass {
                    style: 0,
                    wnd_proc: Some(proc),
                    cls_extra: 0,
                    wnd_extra: 0,
                    instance,
                    icon: 0,
                    cursor: 0,
                    background: CreateSolidBrush(0x00F2_F2F2),
                    menu_name: std::ptr::null(),
                    class_name: class_name.as_ptr(),
                };
                RegisterClassW(&class);
            }
            hwnd = CreateWindowExW(
                0x0008_0000 | 0x0000_0020 | 0x0000_0080 | 0x0000_0008, // LAYERED|TRANSPARENT|TOOLWINDOW|NOACTIVATE
                class_name.as_ptr(),
                std::ptr::null(),
                0x8000_0000, // WS_POPUP
                0,
                0,
                0,
                0,
                0,
                0,
                instance,
                std::ptr::null_mut(),
            );
            if hwnd == 0 {
                return;
            }
            SetLayeredWindowAttributes(hwnd, 0, 255, 0x0000_0002);
            BACKDROP.store(hwnd, Ordering::SeqCst);
        }
    }
    if hwnd == 0 {
        return;
    }
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let owner = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if owner == 0 {
        return;
    }
    if alpha <= 0.001 {
        unsafe {
            ShowWindow(hwnd, 0);
        }
        return;
    }
    unsafe {
        SetWindowPos(
            hwnd,
            owner,
            rect.0,
            rect.1,
            rect.2,
            rect.3,
            0x0010 | 0x0040, // NOACTIVATE | SHOWWINDOW
        );
        ShowWindow(hwnd, 4); // SW_SHOWNOACTIVATE
        let a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
        BACKDROP_COLOR.store(
            ((rgb.2 as u32) << 16) | ((rgb.1 as u32) << 8) | rgb.0 as u32,
            Ordering::SeqCst,
        );
        SetLayeredWindowAttributes(hwnd, 0, a, 0x0000_0002);
        InvalidateRect(hwnd, std::ptr::null(), 1);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn update_lyrics_backdrop(
    _title: &str,
    _rect: (i32, i32, i32, i32),
    _alpha: f32,
    _rgb: (u8, u8, u8),
) {
}

#[cfg(not(target_os = "windows"))]
pub fn cursor_in_window(_title: &str) -> bool {
    false
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
