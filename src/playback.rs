//! Mapping between a track's own timeline and the underlying audio file.
//!
//! Normal tracks start at zero. CUE virtual tracks share one audio file and
//! expose only their own `[cue_start, cue_end]` window, so every UI position,
//! seek and session value is relative to that window.

use crate::model::Track;

pub fn start(track: &Track, resume: Option<f64>) -> f64 {
    let relative = resume.unwrap_or(0.0).max(0.0);
    absolute(track, relative)
}

/// Converts a saved session position to the track-relative timeline.
///
/// Schema 1 stored an absolute file position. CUE virtual tracks now persist
/// their own segment position, so old CUE sessions must be rebased once.
pub fn resume(track: &Track, saved: f64, schema: u32) -> f64 {
    let raw = saved.max(0.0);
    if schema < 2 && track.cue_start.is_some() {
        relative(track, raw)
    } else {
        raw
    }
}

pub fn absolute(track: &Track, relative: f64) -> f64 {
    let base = track.cue_start.unwrap_or(0.0);
    base + clamp_relative(track, relative)
}

pub fn relative(track: &Track, absolute: f64) -> f64 {
    let base = track.cue_start.unwrap_or(0.0);
    clamp_relative(track, absolute - base)
}

/// True once the shared audio file has played past this track's CUE end.
pub fn reached_end(track: &Track, absolute: f64) -> bool {
    track
        .cue_end
        .is_some_and(|end| absolute >= end - 0.02)
}

fn clamp_relative(track: &Track, relative: f64) -> f64 {
    let duration = track.duration.max(0.0);
    if duration > 0.0 {
        relative.clamp(0.0, duration)
    } else {
        relative.max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cue_track() -> Track {
        Track {
            id: 9,
            path: PathBuf::from("album.flac"),
            title: "Second".into(),
            artist: "A".into(),
            album: "B".into(),
            genre: String::new(),
            year: None,
            track_number: Some(2),
            duration: 60.0,
            added_at: 0,
            modified: 0,
            cue_source: Some(PathBuf::from("album.cue")),
            cue_start: Some(100.0),
            cue_end: Some(160.0),
        }
    }

    #[test]
    fn cue_positions_are_relative_to_the_segment() {
        let track = cue_track();
        assert_eq!(start(&track, None), 100.0);
        assert_eq!(start(&track, Some(12.5)), 112.5);
        assert_eq!(absolute(&track, 60.0), 160.0);
        assert_eq!(relative(&track, 130.0), 30.0);
        assert_eq!(relative(&track, 90.0), 0.0);
        assert_eq!(relative(&track, 200.0), 60.0);
    }

    #[test]
    fn cue_end_is_detected_but_plain_tracks_never_end_early() {
        let cue = cue_track();
        assert!(!reached_end(&cue, 159.0));
        assert!(reached_end(&cue, 159.99));
        assert!(reached_end(&cue, 160.0));

        let plain = Track {
            cue_source: None,
            cue_start: None,
            cue_end: None,
            duration: 60.0,
            ..cue.clone()
        };
        assert!(!reached_end(&plain, 59.99));
        assert!(!reached_end(&plain, 61.0));
    }

    #[test]
    fn old_cue_sessions_are_rebased_but_new_ones_are_relative() {
        let cue = cue_track();
        assert_eq!(resume(&cue, 130.0, 1), 30.0);
        assert_eq!(resume(&cue, 30.0, 2), 30.0);
        // Schema 0 also predates track-relative positions.
        assert_eq!(resume(&cue, 30.0, 0), 0.0);
        assert_eq!(resume(&cue, -5.0, 2), 0.0);

        let plain = Track {
            cue_source: None,
            cue_start: None,
            cue_end: None,
            duration: 60.0,
            ..cue.clone()
        };
        assert_eq!(resume(&plain, 42.0, 1), 42.0);
    }
}
