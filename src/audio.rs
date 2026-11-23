//! Thin playback layer around `rodio`.
//!
//! The rest of the app only deals with a current file, play/pause, seek and
//! volume; it never touches a decoder directly.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

pub struct AudioEngine {
    // The stream must outlive every sink created from its handle.
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    volume: f32,
    paused: bool,
    reported_finished: bool,
}

impl AudioEngine {
    pub fn new(volume: f32) -> Result<Self> {
        let (stream, handle) =
            OutputStream::try_default().context("no audio output device available")?;
        Ok(Self {
            _stream: stream,
            handle,
            sink: None,
            volume: volume.clamp(0.0, 1.0),
            paused: false,
            reported_finished: false,
        })
    }

    /// Replaces the current stream with `path`. Format support comes from
    /// Symphonia, so mp3/flac/wav/ogg/m4a/aac all share this path.
    pub fn play_path(&mut self, path: &Path) -> Result<()> {
        let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
        let reader = BufReader::new(file);
        let decoder = Decoder::new(reader).with_context(|| format!("decode {}", path.display()))?;

        self.stop();
        let sink = Sink::try_new(&self.handle).context("create audio sink")?;
        sink.set_volume(self.volume);
        sink.append(decoder);
        sink.play();
        self.sink = Some(sink);
        self.paused = false;
        self.reported_finished = false;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.paused = false;
        self.reported_finished = false;
    }

    pub fn toggle_pause(&mut self) {
        let Some(sink) = &self.sink else {
            return;
        };
        if self.paused {
            sink.play();
            self.paused = false;
        } else {
            sink.pause();
            self.paused = true;
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        let Some(sink) = &self.sink else {
            return;
        };
        if paused {
            sink.pause();
        } else {
            sink.play();
        }
        self.paused = paused;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn has_source(&self) -> bool {
        self.sink.is_some()
    }

    /// True while audio is actively advancing.
    pub fn is_playing(&self) -> bool {
        match &self.sink {
            Some(sink) => !self.paused && !sink.empty(),
            None => false,
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Current playback position, in seconds.
    pub fn position(&self) -> f64 {
        self.sink
            .as_ref()
            .map(|sink| sink.get_pos().as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn seek(&self, seconds: f64) {
        if let Some(sink) = &self.sink {
            let seconds = seconds.max(0.0);
            let _ = sink.try_seek(Duration::from_secs_f64(seconds));
        }
    }

    /// Consumes the "track reached its end" edge exactly once.
    pub fn take_finished(&mut self) -> bool {
        let ended = self
            .sink
            .as_ref()
            .map(|sink| sink.empty() && !self.paused)
            .unwrap_or(false);
        if ended && !self.reported_finished {
            self.reported_finished = true;
            return true;
        }
        false
    }
}
