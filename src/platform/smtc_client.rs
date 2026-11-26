//! Parent-side client for the optional Windows SMTC helper process.
//!
//! The helper is a separate executable that owns the media transport controls
//! window. Everything it prints is a button press from the system flyout; every
//! line we write feeds the flyout. It has no console, so failures are appended
//! to `%TEMP%\rplayer-smtc.log` and the parent restarts it a few times.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use super::media::MediaCommand;
use crate::model::Track;

const MAX_RESTARTS: u8 = 3;

pub struct SmtcClient {
    child: Child,
    stdin: Option<ChildStdin>,
    commands: Receiver<MediaCommand>,
    hwnd: isize,
    restarts: u8,
    last_track: Option<u64>,
    last_playing: Option<bool>,
    last_position: Instant,
}

impl SmtcClient {
    pub fn new() -> Option<Self> {
        let hwnd = super::main_window_handle()?;
        let (child, stdin, commands) = spawn_helper(hwnd)?;
        Some(Self {
            child,
            stdin: Some(stdin),
            commands,
            hwnd,
            restarts: 0,
            last_track: None,
            last_playing: None,
            last_position: Instant::now() - Duration::from_secs(5),
        })
    }

    pub fn poll(&mut self) -> Vec<MediaCommand> {
        let mut commands = Vec::new();
        while let Ok(command) = self.commands.try_recv() {
            commands.push(command);
        }
        if self.restarts < MAX_RESTARTS {
            if let Ok(Some(status)) = self.child.try_wait() {
                eprintln!(
                    "[RPlayer] rplayer-smtc exited ({status}); restarting ({}/{MAX_RESTARTS})",
                    self.restarts + 1
                );
                self.restarts += 1;
                match spawn_helper(self.hwnd) {
                    Some((child, stdin, receiver)) => {
                        self.child = child;
                        self.stdin = Some(stdin);
                        self.commands = receiver;
                        // Force a full resync so the fresh helper shows the
                        // current track instead of staying blank.
                        self.last_track = None;
                        self.last_playing = None;
                    }
                    None => {
                        self.stdin = None;
                    }
                }
            }
        }
        commands
    }

    pub fn sync(
        &mut self,
        track: Option<&Track>,
        playing: bool,
        position: f64,
        duration: f64,
        cover: Option<&eframe::egui::ColorImage>,
    ) {
        let track_id = track.map(|track| track.id);
        if track_id != self.last_track {
            self.last_track = track_id;
            if let Some(track) = track {
                let sanitize = |value: &str| value.replace(['\t', '\n', '\r'], " ");
                self.send(format!(
                    "T\t{}\t{}\t{}",
                    sanitize(&track.title),
                    sanitize(&track.artist),
                    sanitize(&track.album)
                ));
                let path = write_cover(cover);
                self.send(format!("A\t{}", path.display()));
            } else {
                self.send("C".to_string());
            }
        }

        if self.last_playing != Some(playing) {
            self.last_playing = Some(playing);
            self.send(format!("S\t{}", if playing { "1" } else { "0" }));
        }

        if playing && self.last_position.elapsed() >= Duration::from_secs(1) {
            self.last_position = Instant::now();
            self.send(format!("P\t{position:.3}\t{duration:.3}"));
        }
    }

    fn send(&mut self, line: String) {
        let Some(stdin) = self.stdin.as_mut() else {
            return;
        };
        let _ = writeln!(stdin, "{line}");
        let _ = stdin.flush();
    }
}

fn spawn_helper(hwnd: isize) -> Option<(Child, ChildStdin, Receiver<MediaCommand>)> {
    let helper = match helper_path() {
        Some(path) => path,
        None => {
            eprintln!(
                "[RPlayer] rplayer-smtc helper not found next to the executable or in the dev build"
            );
            return None;
        }
    };
    let stderr = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
        .map(Stdio::from)
        .unwrap_or_else(|_| Stdio::null());

    let mut command = Command::new(helper);
    command
        .arg(hwnd.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(stderr);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    let mut child = command.spawn().ok()?;
    let stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;

    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("rplayer-smtc-client".into())
        .spawn(move || read_commands(stdout, sender))
        .ok()?;

    Some((child, stdin, receiver))
}

fn read_commands(stdout: ChildStdout, sender: Sender<MediaCommand>) {
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let command = match line.trim() {
            "play" | "pause" => Some(MediaCommand::PlayPause),
            "next" => Some(MediaCommand::Next),
            "previous" => Some(MediaCommand::Previous),
            "stop" => Some(MediaCommand::Stop),
            _ => None,
        };
        if let Some(command) = command {
            if sender.send(command).is_err() {
                break;
            }
        }
    }
}

fn write_cover(cover: Option<&eframe::egui::ColorImage>) -> PathBuf {
    let path = std::env::temp_dir().join("rplayer-smtc-cover.png");
    if let Some(cover) = cover {
        let mut bytes = Vec::with_capacity(cover.pixels.len() * 4);
        for pixel in &cover.pixels {
            bytes.extend_from_slice(&[pixel.r(), pixel.g(), pixel.b(), pixel.a()]);
        }
        if let Some(image) =
            image::RgbaImage::from_raw(cover.size[0] as u32, cover.size[1] as u32, bytes)
        {
            let _ = image.save(&path);
            return path;
        }
    }
    // A transparent 1x1 clears a previous thumbnail on track changes without
    // cover art.
    let _ = image::RgbaImage::new(1, 1).save(&path);
    path
}

fn log_path() -> PathBuf {
    std::env::temp_dir().join("rplayer-smtc.log")
}

fn helper_path() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "rplayer-smtc.exe"
    } else {
        "rplayer-smtc"
    };
    let sibling = std::env::current_exe().ok()?.with_file_name(name);
    if sibling.is_file() {
        return Some(sibling);
    }
    // Development builds live in `target/<profile>/` while the helper is a
    // separate crate under `tools/smtc-helper/target/<profile>/`. Looking there
    // keeps SMTC working for `cargo run`/`cargo build` without packaging.
    dev_helper_paths(name)
}

fn dev_helper_paths(name: &str) -> Option<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    ["release", "debug"].iter().find_map(|profile| {
        let candidate = manifest
            .join("tools")
            .join("smtc-helper")
            .join("target")
            .join(profile)
            .join(name);
        candidate.is_file().then_some(candidate)
    })
}
