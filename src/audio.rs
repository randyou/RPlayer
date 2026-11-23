//! Thin playback layer around `rodio`.
//!
//! The rest of the app only deals with a current file, play/pause, seek and
//! volume; it never touches a decoder directly.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::eq::{EqShared, EqualizerSource};

pub struct AudioEngine {
    // The stream must outlive every sink created from its handle.
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    volume: f32,
    eq: std::sync::Arc<EqShared>,
    paused: bool,
    reported_finished: bool,
    /// When a decoder is opened at an offset, `Sink::get_pos` still counts
    /// from zero; this offset keeps the UI position honest.
    position_offset: f64,
    current_path: Option<std::path::PathBuf>,
}

impl AudioEngine {
    pub fn new(volume: f32, eq: std::sync::Arc<EqShared>) -> Result<Self> {
        let (stream, handle) =
            OutputStream::try_default().context("no audio output device available")?;
        Ok(Self {
            _stream: stream,
            handle,
            sink: None,
            volume: volume.clamp(0.0, 1.0),
            eq,
            paused: false,
            reported_finished: false,
            position_offset: 0.0,
            current_path: None,
        })
    }

    /// Replaces the current stream with `path`. Format support comes from
    /// Symphonia, so mp3/flac/wav/ogg/m4a/aac all share this path.
    pub fn play_path(&mut self, path: &Path) -> Result<()> {
        self.play_path_at(path, 0.0)
    }

    /// Opens a decoder and optionally seeks before playback starts. Seeking
    /// the decoder is more reliable than seeking a freshly-created sink.
    pub fn play_path_at(&mut self, path: &Path, start: f64) -> Result<()> {
        let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut decoder =
            Decoder::new(reader).with_context(|| format!("decode {}", path.display()))?;
        let mut start_offset = 0.0;
        if start > 0.0 {
            if decoder
                .try_seek(Duration::from_secs_f64(start))
                .is_ok()
            {
                start_offset = start;
            } else {
                // Some Symphonia streams (notably certain FLAC files) do not
                // expose a seek index. Decode-and-discard is slower but keeps
                // session restore and seeking functional.
                let rate = f64::from(decoder.sample_rate());
                let channels = f64::from(decoder.channels().max(1));
                let skip = (start * rate * channels).max(0.0) as u64;
                for _ in 0..skip {
                    if decoder.next().is_none() {
                        break;
                    }
                }
                start_offset = start;
            }
        }

        self.stop();
        self.current_path = Some(path.to_path_buf());
        let sink = Sink::try_new(&self.handle).context("create audio sink")?;
        sink.set_volume(self.volume);
        let converted = decoder.convert_samples::<f32>();
        sink.append(EqualizerSource::new(converted, self.eq.clone()));
        sink.play();
        self.sink = Some(sink);
        self.paused = false;
        self.reported_finished = false;
        self.position_offset = start_offset;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.paused = false;
        self.reported_finished = false;
        self.position_offset = 0.0;
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
            .map(|sink| self.position_offset + sink.get_pos().as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn seek(&mut self, seconds: f64) {
        let seconds = seconds.max(0.0);
        let seek_ok = self
            .sink
            .as_ref()
            .map(|sink| {
                sink.try_seek(Duration::from_secs_f64(seconds))
                    .is_ok()
            })
            .unwrap_or(false);
        if seek_ok {
            // Sink's own position now tracks the requested point.
            self.position_offset = 0.0;
            return;
        }
        // Reopen the decoder at the requested offset when the source cannot
        // seek in place. Keeps the old pause state.
        let Some(path) = self.current_path.clone() else {
            return;
        };
        let paused = self.paused;
        if self.play_path_at(&path, seconds).is_ok() && paused {
            self.set_paused(true);
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
