//! Parent-side client for the optional Windows SMTC helper process.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use super::media::MediaCommand;
use crate::model::Track;

pub struct SmtcClient {
    _child: Child,
    stdin: ChildStdin,
    commands: Receiver<MediaCommand>,
    last_track: Option<u64>,
    last_playing: Option<bool>,
    last_position: Instant,
}

impl SmtcClient {
    pub fn new() -> Option<Self> {
        let hwnd = super::main_window_handle()?;
        let helper = helper_path()?;
        let mut command = Command::new(helper);
        command
            .arg(hwnd.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
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
            .spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    let command = match line.as_str() {
                        "play" => Some(MediaCommand::PlayPause),
                        "pause" => Some(MediaCommand::PlayPause),
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
            })
            .ok()?;

        Some(Self {
            _child: child,
            stdin,
            commands: receiver,
            last_track: None,
            last_playing: None,
            last_position: Instant::now() - Duration::from_secs(5),
        })
    }

    pub fn poll(&self) -> Vec<MediaCommand> {
        let mut commands = Vec::new();
        while let Ok(command) = self.commands.try_recv() {
            commands.push(command);
        }
        commands
    }

    pub fn sync(&mut self, track: Option<&Track>, playing: bool, position: f64, duration: f64) {
        let track_id = track.map(|track| track.id);
        if track_id != self.last_track {
            self.last_track = track_id;
            if let Some(track) = track {
                let sanitize = |value: &str| value.replace(['\t', '\n', '\r'], " ");
                let _ = writeln!(
                    self.stdin,
                    "T\t{}\t{}\t{}",
                    sanitize(&track.title),
                    sanitize(&track.artist),
                    sanitize(&track.album)
                );
            } else {
                let _ = writeln!(self.stdin, "C");
            }
            let _ = self.stdin.flush();
        }

        if self.last_playing != Some(playing) {
            self.last_playing = Some(playing);
            let _ = writeln!(self.stdin, "S\t{}", if playing { "1" } else { "0" });
            let _ = self.stdin.flush();
        }

        if playing && self.last_position.elapsed() >= Duration::from_secs(1) {
            self.last_position = Instant::now();
            let _ = writeln!(self.stdin, "P\t{position:.3}\t{duration:.3}");
            let _ = self.stdin.flush();
        }
    }
}

fn helper_path() -> Option<std::path::PathBuf> {
    let sibling = std::env::current_exe().ok()?.with_file_name(if cfg!(windows) {
        "rplayer-smtc.exe"
    } else {
        "rplayer-smtc"
    });
    if sibling.is_file() {
        Some(sibling)
    } else {
        None
    }
}
