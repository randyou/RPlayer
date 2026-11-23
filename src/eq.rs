//! Ten-band peaking equalizer and its real-time audio source wrapper.
//!
//! Settings live behind a small shared handle so the audio thread can pick up
//! slider changes without restarting playback.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::source::{SeekError, Source};
use serde::{Deserialize, Serialize};

pub const BAND_COUNT: usize = 10;
pub const BAND_FREQUENCIES: [f32; BAND_COUNT] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
];
pub const BAND_LABELS: [&str; BAND_COUNT] =
    ["31", "62", "125", "250", "500", "1K", "2K", "4K", "8K", "16K"];
pub const MIN_GAIN: f32 = -12.0;
pub const MAX_GAIN: f32 = 12.0;

pub const PRESETS: &[(&str, [f32; BAND_COUNT])] = &[
    ("平直", [0.0; BAND_COUNT]),
    ("流行", [-1.0, -0.5, 1.5, 3.0, 2.5, 0.5, -1.0, -1.0, 0.0, 1.0]),
    ("摇滚", [3.5, 2.5, 1.0, -0.5, -1.0, 0.0, 2.0, 3.0, 3.5, 2.5]),
    ("爵士", [2.0, 1.0, 0.0, 1.0, 2.0, 2.0, 1.0, 0.0, 1.0, 2.0]),
    ("低音", [7.0, 6.0, 4.0, 2.0, 0.0, 0.0, -1.0, -2.0, -2.5, -3.0]),
    ("人声", [-3.0, -2.0, 0.0, 2.0, 4.0, 4.0, 3.0, 1.0, 0.0, -1.0]),
    ("古典", [2.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, -1.0, 1.0, 2.0]),
    ("电子", [4.0, 3.0, 1.0, 0.0, -1.0, 1.0, 2.0, 2.5, 3.0, 2.0]),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EqSettings {
    pub enabled: bool,
    pub preamp: f32,
    pub gains: [f32; BAND_COUNT],
    /// Index into [`PRESETS`]; `None` means the listener moved a band manually.
    pub preset: Option<usize>,
}

impl Default for EqSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            preamp: 0.0,
            gains: [0.0; BAND_COUNT],
            preset: Some(0),
        }
    }
}

impl EqSettings {
    pub fn from_preset(index: usize) -> Self {
        let index = index.min(PRESETS.len() - 1);
        Self {
            enabled: true,
            preamp: 0.0,
            gains: PRESETS[index].1,
            preset: Some(index),
        }
    }

    pub fn sanitized(mut self) -> Self {
        self.preamp = self.preamp.clamp(-12.0, 12.0);
        for gain in &mut self.gains {
            *gain = gain.clamp(MIN_GAIN, MAX_GAIN);
        }
        if let Some(index) = self.preset {
            if index >= PRESETS.len() {
                self.preset = None;
            }
        }
        self
    }

    pub fn is_flat(&self) -> bool {
        self.gains.iter().all(|gain| gain.abs() < 0.01) && self.preamp.abs() < 0.01
    }
}

/// Settings shared with the audio thread.
pub struct EqShared {
    settings: Mutex<EqSettings>,
    version: AtomicU64,
}

impl EqShared {
    pub fn new(settings: EqSettings) -> Arc<Self> {
        Arc::new(Self {
            settings: Mutex::new(settings.sanitized()),
            version: AtomicU64::new(1),
        })
    }

    pub fn settings(&self) -> EqSettings {
        self.settings
            .lock()
            .map(|settings| settings.clone())
            .unwrap_or_default()
    }

    pub fn update(&self, settings: EqSettings) {
        let settings = settings.sanitized();
        if let Ok(mut current) = self.settings.lock() {
            *current = settings;
            self.version.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn snapshot(&self) -> (EqSettings, u64) {
        (
            self.settings(),
            self.version.load(Ordering::Relaxed),
        )
    }
}

#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn peaking(sample_rate: f32, frequency: f32, q: f32, gain_db: f32) -> Self {
        if gain_db.abs() < 0.001 || sample_rate <= 0.0 {
            return Self::identity();
        }
        let frequency = frequency.clamp(20.0, sample_rate * 0.46);
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = std::f32::consts::TAU * frequency / sample_rate;
        let alpha = omega.sin() / (2.0 * q);
        let cos = omega.cos();

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha / a;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// A `rodio` source that applies a cascade of peaking filters per channel.
pub struct EqualizerSource<S>
where
    S: Source<Item = f32>,
{
    inner: S,
    channels: usize,
    sample_rate: u32,
    channel: usize,
    filters: Vec<Vec<Biquad>>,
    preamp: f32,
    enabled: bool,
    shared: Arc<EqShared>,
    version: u64,
    counter: u64,
}

impl<S> EqualizerSource<S>
where
    S: Source<Item = f32>,
{
    pub fn new(inner: S, shared: Arc<EqShared>) -> Self {
        let channels = inner.channels().max(1) as usize;
        let sample_rate = inner.sample_rate();
        let (settings, version) = shared.snapshot();
        let mut source = Self {
            inner,
            channels,
            sample_rate,
            channel: 0,
            filters: Vec::new(),
            preamp: 1.0,
            enabled: settings.enabled,
            shared,
            version,
            counter: 0,
        };
        source.rebuild(&settings);
        source
    }

    fn rebuild(&mut self, settings: &EqSettings) {
        self.enabled = settings.enabled;
        let max_boost = settings
            .gains
            .iter()
            .copied()
            .fold(0.0_f32, f32::max)
            .max(0.0);
        // Leave automatic headroom so boosted presets do not hard-clip the
        // output; the preamp slider still adds its own offset on top.
        let headroom = 10.0_f32.powf(-max_boost * 0.6 / 20.0);
        self.preamp = 10.0_f32.powf(settings.preamp / 20.0) * headroom;
        self.filters = (0..self.channels)
            .map(|_| {
                (0..BAND_COUNT)
                    .map(|band| {
                        Biquad::peaking(
                            self.sample_rate as f32,
                            BAND_FREQUENCIES[band],
                            1.35,
                            settings.gains[band],
                        )
                    })
                    .collect()
            })
            .collect();
    }

    fn refresh_if_needed(&mut self) {
        let (settings, version) = self.shared.snapshot();
        if version != self.version {
            self.version = version;
            self.rebuild(&settings);
        }
    }
}

impl<S> Iterator for EqualizerSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        self.counter = self.counter.wrapping_add(1);
        if self.counter % 1024 == 0 {
            self.refresh_if_needed();
        }

        let channel = self.channel;
        self.channel = (self.channel + 1) % self.channels;

        if !self.enabled {
            return Some(sample);
        }

        let mut value = sample * self.preamp;
        if let Some(filters) = self.filters.get_mut(channel) {
            for filter in filters {
                value = filter.process(value);
            }
        }
        Some(value.clamp(-1.0, 1.0))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<S> Source for EqualizerSource<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.channels as u16
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(pos)?;
        for channel in &mut self.filters {
            for filter in channel {
                filter.reset();
            }
        }
        self.channel = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_all_bands() {
        for (_, gains) in PRESETS {
            assert_eq!(gains.len(), BAND_COUNT);
            assert!(gains.iter().all(|gain| *gain >= MIN_GAIN && *gain <= MAX_GAIN));
        }
    }

    #[test]
    fn flat_biquad_is_transparent() {
        let mut filter = Biquad::peaking(44_100.0, 1_000.0, 1.35, 0.0);
        let input = [0.2_f32, -0.1, 0.5, -0.25];
        for sample in input {
            assert!((filter.process(sample) - sample).abs() < 1e-6);
        }
    }

    #[test]
    fn equalizer_source_keeps_samples_finite() {
        let source = rodio::buffer::SamplesBuffer::new(2, 44_100, vec![0.3_f32; 2_048]);
        let shared = EqShared::new(EqSettings::from_preset(4));
        let output: Vec<f32> = EqualizerSource::new(source, shared).take(512).collect();
        assert_eq!(output.len(), 512);
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().all(|sample| sample.abs() <= 1.0));
    }
}
