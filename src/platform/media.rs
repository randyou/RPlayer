use std::sync::mpsc::{self, Receiver};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommand {
    PlayPause,
    Next,
    Previous,
    Stop,
}

#[cfg(target_os = "windows")]
pub fn start(wake: impl Fn() + Send + 'static) -> Option<Receiver<MediaCommand>> {
    use std::ptr;
    use winapi::um::winuser::{
        GetMessageW, RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT, MSG, VK_MEDIA_NEXT_TRACK,
        VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, WM_HOTKEY,
    };

    let (sender, receiver) = mpsc::channel();
    let thread = std::thread::Builder::new()
        .name("rplayer-media-keys".into())
        .spawn(move || unsafe {
            let bindings = [
                (1, VK_MEDIA_PLAY_PAUSE, MediaCommand::PlayPause),
                (2, VK_MEDIA_NEXT_TRACK, MediaCommand::Next),
                (3, VK_MEDIA_PREV_TRACK, MediaCommand::Previous),
                (4, VK_MEDIA_STOP, MediaCommand::Stop),
            ];
            let mut registered = Vec::new();
            for (id, key, command) in bindings {
                if RegisterHotKey(ptr::null_mut(), id, MOD_NOREPEAT as u32, key as u32) != 0 {
                    registered.push((id, command));
                }
            }
            if registered.is_empty() {
                return;
            }

            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&mut message, ptr::null_mut(), 0, 0) > 0 {
                if message.message == WM_HOTKEY {
                    let id = message.wParam as i32;
                    if let Some((_, command)) = registered.iter().find(|(hotkey, _)| *hotkey == id) {
                        if sender.send(*command).is_err() {
                            break;
                        }
                        // Wake the UI instead of waiting for its idle tick.
                        wake();
                    }
                }
            }
            for (id, _) in registered {
                UnregisterHotKey(ptr::null_mut(), id);
            }
        });

    match thread {
        Ok(_) => Some(receiver),
        Err(_) => None,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn start(_wake: impl Fn() + Send + 'static) -> Option<Receiver<MediaCommand>> {
    None
}
