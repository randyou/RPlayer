//! Shared motion tokens and the reduced-motion switch.

use std::sync::atomic::{AtomicBool, Ordering};

pub const PRESS: f32 = 0.08;
pub const FAST: f32 = 0.12;
pub const PAGE: f32 = 0.18;
pub const SLOW: f32 = 0.25;

static REDUCED: AtomicBool = AtomicBool::new(false);

pub fn set_reduced(reduced: bool) {
    REDUCED.store(reduced, Ordering::Relaxed);
}

pub fn reduced() -> bool {
    REDUCED.load(Ordering::Relaxed)
}

/// Collapse animation durations when the user asks for reduced motion.
pub fn duration(seconds: f32) -> f32 {
    if reduced() {
        0.0
    } else {
        seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduced_motion_collapses_durations() {
        set_reduced(true);
        assert_eq!(duration(PAGE), 0.0);
        set_reduced(false);
        assert_eq!(duration(PAGE), PAGE);
    }
}