//! Thin playback layer around `rodio`.
//!
//! The rest of the app only deals with a current file, play/pause, seek and
//! volume; it never touches a decoder directly.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use lofty::file::TaggedFileExt;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::eq::{EqShared, EqualizerSource};

/// Counts the moment a decoded track is exhausted. The app uses this to swap
/// metadata for a track that was already appended to the same sink, so there is
/// no stop/start gap between album tracks.
struct BoundarySource<S> {
    inner: S,
    boundary: Arc<AtomicU64>,
    finished: bool,
}

impl<S> BoundarySource<S> {
    fn new(inner: S, boundary: Arc<AtomicU64>) -> Self {
        Self {
            inner,
            boundary,
            finished: false,
        }
    }
}

impl<S> Iterator for BoundarySource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.inner.next() {
            return Some(sample);
        }
        if !self.finished {
            self.finished = true;
            self.boundary.fetch_add(1, Ordering::SeqCst);
        }
        None
    }
}

impl<S> Source for BoundarySource<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.inner.try_seek(pos)
    }
}

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
    /// Absolute sink position where the current track started. When the next
    /// track is appended, this moves to the boundary instead of the sink being
    /// recreated.
    track_origin: f64,
    boundary: Arc<AtomicU64>,
    queued_next: bool,
    current_path: Option<std::path::PathBuf>,
    replaygain_mode: String,
    replaygain: f32,
    queued_path: Option<std::path::PathBuf>,
    queued_gain: f32,
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
            track_origin: 0.0,
            boundary: Arc::new(AtomicU64::new(0)),
            queued_next: false,
            current_path: None,
            replaygain_mode: "off".to_string(),
            replaygain: 1.0,
            queued_path: None,
            queued_gain: 1.0,
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
        self.replaygain = gain_for(path, &self.replaygain_mode);
        let sink = Sink::try_new(&self.handle).context("create audio sink")?;
        sink.set_volume(self.volume * self.replaygain);
        let converted = decoder.convert_samples::<f32>();
        let boundary = Arc::new(AtomicU64::new(0));
        sink.append(BoundarySource::new(
            EqualizerSource::new(converted, self.eq.clone()),
            boundary.clone(),
        ));
        sink.play();
        self.sink = Some(sink);
        self.boundary = boundary;
        self.track_origin = 0.0;
        self.queued_next = false;
        self.paused = false;
        self.reported_finished = false;
        self.position_offset = start_offset;
        Ok(())
    }

    /// Appends the next track to the existing sink. The audio thread switches
    /// sources at the exact sample boundary, without a stop/start gap.
    pub fn queue_next(&mut self, path: &Path) -> Result<()> {
        if self.sink.is_none() || self.queued_next {
            anyhow::bail!("no active sink or a track is already queued");
        }
        let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
        let reader = BufReader::new(file);
        let decoder = Decoder::new(reader).with_context(|| format!("decode {}", path.display()))?;
        let converted = decoder.convert_samples::<f32>();
        let source = BoundarySource::new(
            EqualizerSource::new(converted, self.eq.clone()),
            self.boundary.clone(),
        );
        if let Some(sink) = &self.sink {
            sink.append(source);
            self.queued_next = true;
            self.queued_path = Some(path.to_path_buf());
            self.queued_gain = gain_for(path, &self.replaygain_mode);
            Ok(())
        } else {
            anyhow::bail!("no active sink");
        }
    }

     /// Current value of the gapless boundary counter.
    pub fn boundary_count(&self) -> u64 {
        self.boundary.load(Ordering::SeqCst)
    }

    /// Moves the logical position to the start of the already-appended next
    /// source. Called by the app when a boundary event arrives.
    pub fn begin_next_track(&mut self) {
        self.track_origin = self
            .sink
            .as_ref()
            .map(|sink| sink.get_pos().as_secs_f64())
            .unwrap_or(0.0);
        self.position_offset = 0.0;
        self.queued_next = false;
        self.reported_finished = false;
        if let Some(path) = self.queued_path.take() {
            self.current_path = Some(path);
            self.replaygain = self.queued_gain;
            self.apply_volume();
        }
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.paused = false;
        self.reported_finished = false;
        self.position_offset = 0.0;
        self.track_origin = 0.0;
        self.queued_next = false;
        self.queued_path = None;
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
        self.apply_volume();
    }

    fn apply_volume(&self) {
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume * self.replaygain);
        }
    }

    pub fn set_replaygain_mode(&mut self, mode: &str) {
        if self.replaygain_mode == mode {
            return;
        }
        self.replaygain_mode = mode.to_string();
        self.replaygain = self
            .current_path
            .as_deref()
            .map(|path| gain_for(path, &self.replaygain_mode))
            .unwrap_or(1.0);
        self.apply_volume();
    }


    /// Current playback position, in seconds.
    pub fn position(&self) -> f64 {
        self.sink
            .as_ref()
            .map(|sink| {
                let elapsed = (sink.get_pos().as_secs_f64() - self.track_origin).max(0.0);
                self.position_offset + elapsed
            })
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

fn gain_for(path: &Path, mode: &str) -> f32 {
    if mode == "off" {
        return 1.0;
    }
    let Ok(tagged) = lofty::read_from_path(path) else {
        return 1.0;
    };
    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return 1.0;
    };
    let track = tag.get_string(&lofty::tag::ItemKey::ReplayGainTrackGain);
    let album = tag.get_string(&lofty::tag::ItemKey::ReplayGainAlbumGain);
    let raw = if mode == "album" {
        album.or(track)
    } else {
        track.or(album)
    };
    raw.and_then(parse_db)
        .map(|db| {
            // Conservative limiter against broken tags.
            10f32.powf(db.clamp(-12.0, 12.0) / 20.0)
        })
        .unwrap_or(1.0)
}

fn parse_db(value: &str) -> Option<f32> {
    value
        .trim()
        .trim_end_matches("dB")
        .trim_end_matches("db")
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::parse_db;

    #[test]
    fn parses_replaygain_db_values() {
        assert_eq!(parse_db("-7.50 dB"), Some(-7.5));
        assert_eq!(parse_db("+3.2db"), Some(3.2));
        assert_eq!(parse_db("nonsense"), None);
    }
}
