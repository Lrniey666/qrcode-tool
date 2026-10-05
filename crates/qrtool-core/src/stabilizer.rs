//! Debounce for live camera scanning.
//!
//! A code must be seen in `required` consecutive analysed frames before it counts, and the
//! same content is not reported again until `cooldown` has passed. This prevents one QR code
//! held in front of the camera from opening a stream of browser tabs, and filters out
//! one-frame misreads.

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Stabilizer {
    required: usize,
    cooldown: Duration,
    streaks: HashMap<String, usize>,
    fired: HashMap<String, Instant>,
}

impl Stabilizer {
    pub fn new(required: usize, cooldown: Duration) -> Self {
        Self {
            required: required.max(1),
            cooldown,
            streaks: HashMap::new(),
            fired: HashMap::new(),
        }
    }

    /// Feeds the texts decoded from one frame. Returns texts that became stable in this frame.
    pub fn feed(&mut self, texts: &[String], now: Instant) -> Vec<String> {
        // Reset streaks for anything not seen in this frame.
        self.streaks.retain(|t, _| texts.contains(t));
        self.fired
            .retain(|_, at| now.saturating_duration_since(*at) < self.cooldown);

        let mut out = Vec::new();
        for t in texts {
            let streak = self.streaks.entry(t.clone()).or_insert(0);
            *streak += 1;
            if *streak == self.required && !self.fired.contains_key(t) {
                self.fired.insert(t.clone(), now);
                out.push(t.clone());
            }
        }
        out
    }

    /// Forgets everything (e.g. when the camera window is reopened).
    pub fn reset(&mut self) {
        self.streaks.clear();
        self.fired.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_consecutive_frames_and_cooldown() {
        let mut s = Stabilizer::new(3, Duration::from_secs(5));
        let t0 = Instant::now();
        let a = vec!["A".to_string()];
        assert!(s.feed(&a, t0).is_empty());
        assert!(s.feed(&a, t0).is_empty());
        assert_eq!(s.feed(&a, t0), vec!["A".to_string()]);
        // keeps being seen: not reported again
        assert!(s.feed(&a, t0).is_empty());
        // disappears and comes back within cooldown: not reported
        assert!(s.feed(&[], t0).is_empty());
        for _ in 0..3 {
            assert!(s.feed(&a, t0 + Duration::from_secs(1)).is_empty());
        }
        // after cooldown it can fire again
        s.feed(&[], t0 + Duration::from_secs(10));
        let later = t0 + Duration::from_secs(10);
        s.feed(&a, later);
        s.feed(&a, later);
        assert_eq!(s.feed(&a, later), vec!["A".to_string()]);
    }

    #[test]
    fn interrupted_streak_resets() {
        let mut s = Stabilizer::new(2, Duration::from_secs(1));
        let t = Instant::now();
        let a = vec!["A".to_string()];
        s.feed(&a, t);
        s.feed(&[], t);
        assert!(s.feed(&a, t).is_empty());
        assert_eq!(s.feed(&a, t), a);
    }
}
