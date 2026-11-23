//! Small helper process that owns the Windows System Media Transport Controls
//! session. Keeping WinRT in a separate executable keeps the main RPlayer
//! binary light and avoids a large WinRT link in the UI process.
//!
//! Protocol (tab separated, one message per line):
//!   T <title> <artist> <album>
//!   S <0|1>
//!   P <position> <duration>
//!   C
//! Replies: play / pause / next / previous / stop.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    use std::io::{BufRead, Write};

    use windows::core::{factory, HSTRING};
    use windows::Foundation::{TimeSpan, TypedEventHandler};
    use windows::Media::{
        MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
        SystemMediaTransportControlsButton, SystemMediaTransportControlsTimelineProperties,
    };
    use windows::Storage::StorageFile;
    use windows::Storage::Streams::RandomAccessStreamReference;
    use windows::Win32::Foundation::{HINSTANCE, HWND};
    use windows::Win32::System::WinRT::{
        ISystemMediaTransportControlsInterop, RoInitialize, RO_INIT_SINGLETHREADED,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, HMENU, WINDOW_EX_STYLE, WINDOW_STYLE,
    };

    #[link(name = "shell32")]
    extern "system" {
        fn SetCurrentProcessExplicitAppUserModelID(app_id: *const u16) -> i32;
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn RegCreateKeyExW(
            key: *mut std::ffi::c_void,
            sub_key: *const u16,
            reserved: u32,
            class: *const u16,
            options: u32,
            sam: u32,
            security: *mut std::ffi::c_void,
            result: *mut *mut std::ffi::c_void,
            disposition: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            key: *mut std::ffi::c_void,
            value: *const u16,
            reserved: u32,
            kind: u32,
            data: *const u8,
            len: u32,
        ) -> i32;
        fn RegCloseKey(key: *mut std::ffi::c_void) -> i32;
    }

    let wide = |value: &str| -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    };

    let app_id = "RPlayer";
    let sub_key = wide(&format!("Software\\Classes\\AppUserModelId\\{app_id}"));
    let display_name = wide("RPlayer");
    let mut key = std::ptr::null_mut();
    let mut disposition = 0u32;
    let created = unsafe {
        RegCreateKeyExW(
            0x8000_0001usize as *mut std::ffi::c_void,
            sub_key.as_ptr(),
            0,
            std::ptr::null(),
            0,
            0x2_0006,
            std::ptr::null_mut(),
            &mut key,
            &mut disposition,
        )
    };
    if created == 0 {
        let value_name = wide("DisplayName");
        unsafe {
            let _ = RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                1,
                display_name.as_ptr() as *const u8,
                (display_name.len() * 2) as u32,
            );
            let _ = RegCloseKey(key);
        }
    }
    let app_id_wide = wide(app_id);
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(app_id_wide.as_ptr());
    }

    let Some(hwnd) = std::env::args().nth(1).and_then(|arg| arg.parse::<isize>().ok()) else {
        return;
    };
    if hwnd == 0 {
        return;
    }

    let _ = unsafe { RoInitialize(RO_INIT_SINGLETHREADED) };

    let hidden = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            windows::core::w!("STATIC"),
            windows::core::w!("RPlayer SMTC"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND(std::ptr::null_mut()),
            HMENU(std::ptr::null_mut()),
            HINSTANCE(std::ptr::null_mut()),
            None,
        )
    };
    let Ok(hidden) = hidden else {
        return;
    };

    let Ok(interop) =
        factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()
    else {
        return;
    };
    let controls = unsafe {
        interop.GetForWindow::<_, SystemMediaTransportControls>(hidden)
    };
    let Ok(controls) = controls else {
        return;
    };

    let _ = controls.SetIsEnabled(true);
    let _ = controls.SetIsPlayEnabled(true);
    let _ = controls.SetIsPauseEnabled(true);
    let _ = controls.SetIsNextEnabled(true);
    let _ = controls.SetIsPreviousEnabled(true);
    let _ = controls.SetIsStopEnabled(true);
    let _ = controls.SetPlaybackStatus(MediaPlaybackStatus::Paused);

    let handler = TypedEventHandler::new(
        move |_sender: &Option<SystemMediaTransportControls>,
              args: &Option<windows::Media::SystemMediaTransportControlsButtonPressedEventArgs>| {
            if let Some(args) = args {
                if let Ok(button) = args.Button() {
                    let reply = if button == SystemMediaTransportControlsButton::Play {
                        "play"
                    } else if button == SystemMediaTransportControlsButton::Pause {
                        "pause"
                    } else if button == SystemMediaTransportControlsButton::Next {
                        "next"
                    } else if button == SystemMediaTransportControlsButton::Previous {
                        "previous"
                    } else if button == SystemMediaTransportControlsButton::Stop {
                        "stop"
                    } else {
                        ""
                    };
                    if !reply.is_empty() {
                        let mut out = std::io::stdout();
                        let _ = writeln!(out, "{reply}");
                        let _ = out.flush();
                    }
                }
            }
            Ok(())
        },
    );
    if controls.ButtonPressed(&handler).is_err() {
        return;
    }

    let mut timeline = SystemMediaTransportControlsTimelineProperties::new().ok();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines().map_while(Result::ok) {
        let mut parts = line.split('\t');
        match parts.next() {
            Some("A") => {
                let path = parts.next().unwrap_or("");
                if !path.is_empty() {
                    if let Ok(operation) = StorageFile::GetFileFromPathAsync(&HSTRING::from(path)) {
                        if let Ok(file) = operation.get() {
                            if let Ok(thumbnail) =
                                RandomAccessStreamReference::CreateFromFile(&file)
                            {
                                if let Ok(updater) = controls.DisplayUpdater() {
                                    let _ = updater.SetThumbnail(&thumbnail);
                                    let _ = updater.Update();
                                }
                            }
                        }
                    }
                }
            }
            Some("T") => {
                let title = parts.next().unwrap_or("未知歌曲");
                let artist = parts.next().unwrap_or("未知艺术家");
                let album = parts.next().unwrap_or("");
                if let Ok(updater) = controls.DisplayUpdater() {
                    let _ = updater.SetType(MediaPlaybackType::Music);
                    if let Ok(properties) = updater.MusicProperties() {
                        let _ = properties.SetTitle(&HSTRING::from(title));
                        let _ = properties.SetArtist(&HSTRING::from(artist));
                        let _ = properties.SetAlbumTitle(&HSTRING::from(album));
                    }
                    let _ = updater.Update();
                }
            }
            Some("S") => {
                let playing = parts.next() == Some("1");
                let _ = controls.SetPlaybackStatus(if playing {
                    MediaPlaybackStatus::Playing
                } else {
                    MediaPlaybackStatus::Paused
                });
            }
            Some("P") => {
                let position = parts.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                let duration = parts.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                if let Some(timeline) = timeline.as_ref() {
                    let ticks = |seconds: f64| TimeSpan {
                        Duration: (seconds.max(0.0) * 10_000_000.0) as i64,
                    };
                    let _ = timeline.SetStartTime(ticks(0.0));
                    let _ = timeline.SetMinSeekTime(ticks(0.0));
                    let _ = timeline.SetMaxSeekTime(ticks(duration));
                    let _ = timeline.SetEndTime(ticks(duration));
                    let _ = timeline.SetPosition(ticks(position));
                    let _ = controls.UpdateTimelineProperties(timeline);
                }
            }
            Some("C") => {
                let _ = controls.SetPlaybackStatus(MediaPlaybackStatus::Closed);
            }
            _ => {}
        }
    }
}

#[cfg(not(windows))]
fn main() {}
