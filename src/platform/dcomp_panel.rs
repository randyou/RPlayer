//! DirectComposition lyric panel.
//!
//! The lyric text window is an eframe color-key window; this module owns a
//! separate composition window behind it. The panel is drawn by D3D11 into an
//! `IDCompositionSurface` with premultiplied alpha, so panel opacity is real
//! and independent from text opacity.

#![cfg(target_os = "windows")]

use std::sync::{Mutex, OnceLock};

use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::{HWND, LRESULT, POINT, RECT};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionSurface, IDCompositionTarget,
    IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
};
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DwmSetWindowAttribute, DWMWINDOWATTRIBUTE, DWM_BLURBEHIND, DWM_BB_ENABLE,
};
use windows::Win32::Graphics::Dxgi::{IDXGIDevice, IDXGISurface};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, FindWindowW, GetClientRect, PostMessageW, RegisterClassW, SendMessageW, SetWindowPos, ShowWindow, HCURSOR, HICON,
    SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE, WM_ERASEBKGND, WNDCLASSW, WNDPROC,
    WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
};

struct Panel {
    // Only touched on the winit UI thread.
    hwnd: HWND,
    d3d: ID3D11Device,
    context: ID3D11DeviceContext,
    dcomp: IDCompositionDevice,
    _target: IDCompositionTarget,
    visual: IDCompositionVisual,
    surface: IDCompositionSurface,
    width: u32,
    height: u32,
}

unsafe impl Send for Panel {}

static PANEL_OWNER: std::sync::atomic::AtomicIsize =
    std::sync::atomic::AtomicIsize::new(0);

static PANEL: OnceLock<Mutex<Option<Panel>>> = OnceLock::new();

fn panel() -> &'static Mutex<Option<Panel>> {
    PANEL.get_or_init(|| Mutex::new(None))
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn panel_proc(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> LRESULT {
    if msg == WM_ERASEBKGND {
        return LRESULT(1);
    }
    #[link(name = "user32")]
    extern "system" {
        fn ScreenToClient(hwnd: HWND, point: *mut windows::Win32::Foundation::POINT) -> i32;
    }
    if msg == 0x0084 {
        let mut point = windows::Win32::Foundation::POINT {
            x: (lparam.0 as i32) & 0xFFFF,
            y: ((lparam.0 as i32) >> 16) & 0xFFFF,
        };
        let mut rect = windows::Win32::Foundation::RECT::default();
        let _ = unsafe { ScreenToClient(hwnd, &mut point) };
        let _ = unsafe { GetClientRect(hwnd, &mut rect) };
        let width = rect.right - rect.left;
        let controls = (point.x < 230 && point.y < 130)
            || (point.x > width - 56 && point.y < 46);
        return LRESULT(1);
    }
    if msg == 0x0201 {
        let owner = PANEL_OWNER.load(std::sync::atomic::Ordering::SeqCst);
        if owner != 0 {
            let x = (lparam.0 as i32) & 0xFFFF;
            let y = ((lparam.0 as i32) >> 16) & 0xFFFF;
            let mut rect = windows::Win32::Foundation::RECT::default();
            let _ = unsafe { GetClientRect(hwnd, &mut rect) };
            let width = rect.right - rect.left;
            let controls = (x < 230 && y < 130) || (x > width - 56 && y < 46);
            if controls {
                let owner_hwnd = HWND(owner as *mut core::ffi::c_void);
                let packed = (((y as isize) << 16) | (x as isize)) as isize;
                unsafe {
                    let _ = PostMessageW(owner_hwnd, 0x0201, windows::Win32::Foundation::WPARAM(1), windows::Win32::Foundation::LPARAM(packed));
                    let _ = PostMessageW(owner_hwnd, 0x0202, windows::Win32::Foundation::WPARAM(0), windows::Win32::Foundation::LPARAM(packed));
                }
            } else {
                unsafe {
                    let _ = ReleaseCapture();
                    let _ = SendMessageW(
                        HWND(owner as *mut core::ffi::c_void),
                        0x00A1,
                        windows::Win32::Foundation::WPARAM(2),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
            return LRESULT(0);
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn create_window() -> Option<HWND> {
    let class = wide("RPlayerDcompPanel");
    let title = wide("RPlayerLyricDcomp");
    let module = unsafe { GetModuleHandleW(PCWSTR::null()).ok()? };
    unsafe {
        let wc = WNDCLASSW {
            style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
            lpfnWndProc: Some(panel_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: windows::Win32::Foundation::HINSTANCE(module.0),
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class.as_ptr()),
        };
        let _ = RegisterClassW(&wc);
        CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE
                | WS_EX_TRANSPARENT,
            PCWSTR(class.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            windows::Win32::Foundation::HINSTANCE(module.0),
            None,
        )
        .ok()
    }
}

fn create_panel(hwnd: HWND) -> windows::core::Result<Panel> {
    unsafe {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
        let d3d = device.ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;
        let context = context
            .ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;
        let dxgi: IDXGIDevice = d3d.cast()?;
        let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
        let no_border: u32 = 0xFFFF_FFFE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(34),
            &no_border as *const u32 as *const core::ffi::c_void,
            4,
        );
        let square: u32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(33),
            &square as *const u32 as *const core::ffi::c_void,
            4,
        );
        let no_frame: u32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(2),
            &no_frame as *const u32 as *const core::ffi::c_void,
            4,
        );
        let mut blur = DWM_BLURBEHIND::default();
        blur.dwFlags = DWM_BB_ENABLE;
        blur.fEnable = true.into();
        let _ = DwmEnableBlurBehindWindow(hwnd, &blur);
        let target = dcomp.CreateTargetForHwnd(hwnd, false)?;
        let visual = dcomp.CreateVisual()?;
        target.SetRoot(&visual)?;
        let surface = dcomp.CreateSurface(
            1,
            1,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )?;
        visual.SetContent(&surface)?;
        let clip = dcomp.CreateRectangleClip()?;
        clip.SetLeft2(0.0)?;
        clip.SetTop2(0.0)?;
        clip.SetRight2(1.0)?;
        clip.SetBottom2(1.0)?;
        let radius = 18.0;
        clip.SetTopLeftRadiusX2(radius)?;
        clip.SetTopLeftRadiusY2(radius)?;
        clip.SetTopRightRadiusX2(radius)?;
        clip.SetTopRightRadiusY2(radius)?;
        clip.SetBottomLeftRadiusX2(radius)?;
        clip.SetBottomLeftRadiusY2(radius)?;
        clip.SetBottomRightRadiusX2(radius)?;
        clip.SetBottomRightRadiusY2(radius)?;
        visual.SetClip(&clip)?;
        dcomp.Commit()?;
        Ok(Panel {
            hwnd,
            d3d,
            context,
            dcomp,
            _target: target,
            visual,
            surface,
            width: 1,
            height: 1,
        })
    }
}

fn draw(panel: &mut Panel, width: u32, height: u32, alpha: f32, rgb: (u8, u8, u8)) -> windows::core::Result<()> {
    unsafe {
        if width != panel.width || height != panel.height {
            let surface = panel.dcomp.CreateSurface(
                width,
                height,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_ALPHA_MODE_PREMULTIPLIED,
            )?;
            panel.visual.SetContent(&surface)?;
            panel.surface = surface;
            let clip = panel.dcomp.CreateRectangleClip()?;
            clip.SetLeft2(0.0)?;
            clip.SetTop2(0.0)?;
            clip.SetRight2(width as f32)?;
            clip.SetBottom2(height as f32)?;
            clip.SetTopLeftRadiusX2(18.0)?;
            clip.SetTopLeftRadiusY2(18.0)?;
            clip.SetTopRightRadiusX2(18.0)?;
            clip.SetTopRightRadiusY2(18.0)?;
            clip.SetBottomLeftRadiusX2(18.0)?;
            clip.SetBottomLeftRadiusY2(18.0)?;
            clip.SetBottomRightRadiusX2(18.0)?;
            clip.SetBottomRightRadiusY2(18.0)?;
            panel.visual.SetClip(&clip)?;
            panel.width = width;
            panel.height = height;
        }
        let mut offset = POINT::default();
        let surface: IDXGISurface = panel.surface.BeginDraw(None, &mut offset)?;
        let texture: windows::Win32::Graphics::Direct3D11::ID3D11Texture2D = surface.cast()?;
        let mut rtv: Option<ID3D11RenderTargetView> = None;
        panel.d3d.CreateRenderTargetView(&texture, None, Some(&mut rtv))?;
        let rtv = rtv.ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;
        let a = alpha.clamp(0.0, 1.0);
        let color = [
            (rgb.0 as f32 / 255.0) * a,
            (rgb.1 as f32 / 255.0) * a,
            (rgb.2 as f32 / 255.0) * a,
            a,
        ];
        panel.context.ClearRenderTargetView(&rtv, &color);
        panel.surface.EndDraw()?;
        panel.dcomp.Commit()?;
        let _ = RECT::default();
    }
    Ok(())
}

/// Returns true when the panel is handled by DirectComposition.
pub fn update(title: &str, rect: (i32, i32, i32, i32), alpha: f32, rgb: (u8, u8, u8)) -> bool {
    if alpha <= 0.001 {
        if let Ok(mut guard) = panel().lock() {
            if let Some(panel) = guard.as_mut() {
                unsafe {
                    let _ = ShowWindow(panel.hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
                }
            }
        }
        return true;
    }
    let mut guard = match panel().lock() {
        Ok(guard) => guard,
        Err(_) => return false,
    };
    if guard.is_none() {
        let Some(hwnd) = create_window() else {
            return false;
        };
        match create_panel(hwnd) {
            Ok(created) => *guard = Some(created),
            Err(_) => {
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
                }
                return false;
            }
        }
    }
    let Some(panel) = guard.as_mut() else {
        return false;
    };
    let owner_wide = wide(title);
    let owner = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(owner_wide.as_ptr())).ok() };
    let Some(owner) = owner else {
        return false;
    };
    PANEL_OWNER.store(owner.0 as isize, std::sync::atomic::Ordering::SeqCst);
    unsafe {
        if SetWindowPos(
            panel.hwnd,
            owner,
            rect.0,
            rect.1,
            rect.2.max(1),
            rect.3.max(1),
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .is_err()
        {
            return false;
        }
        let _ = ShowWindow(panel.hwnd, SW_SHOWNOACTIVATE);
    }
    draw(panel, rect.2.max(1) as u32, rect.3.max(1) as u32, alpha, rgb).is_ok()
}
