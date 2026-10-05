//! Keeps camera results on screen for a while after the code leaves the picture, so the
//! links do not jump around or vanish while the user reaches for them.

use std::time::{Duration, Instant};

use crate::scan::ScanItem;

/// A code that was seen recently.
#[derive(Debug, Clone)]
pub struct Held {
    pub item: ScanItem,
    pub last_seen: Instant,
    /// Seen in the newest analysed frame.
    pub live: bool,
}

/// Merges the codes of a new frame into `held`. Known codes keep their place in the list
/// (so the selection and the on-screen order stay stable) and their outline moves smoothly;
/// new codes are appended; codes not seen for longer than `hold` are dropped.
pub fn merge(held: &mut Vec<Held>, fresh: Vec<ScanItem>, now: Instant, hold: Duration) {
    for h in held.iter_mut() {
        h.live = false;
    }
    for it in fresh {
        if let Some(h) = held.iter_mut().find(|h| h.item.text == it.text) {
            let b = &mut h.item.bounds;
            // Light smoothing against jitter between frames.
            b.x = b.x * 0.4 + it.bounds.x * 0.6;
            b.y = b.y * 0.4 + it.bounds.y * 0.6;
            b.w = b.w * 0.4 + it.bounds.w * 0.6;
            b.h = b.h * 0.4 + it.bounds.h * 0.6;
            h.last_seen = now;
            h.live = true;
        } else {
            held.push(Held {
                item: it,
                last_seen: now,
                live: true,
            });
        }
    }
    expire(held, now, hold);
}

/// Drops codes that have been out of view for longer than `hold`. Returns `true` if
/// something was removed.
pub fn expire(held: &mut Vec<Held>, now: Instant, hold: Duration) -> bool {
    let before = held.len();
    held.retain(|h| h.live || now.saturating_duration_since(h.last_seen) <= hold);
    held.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;
    use qrtool_decode::{Detection, Point};

    fn item(text: &str, x: f32) -> ScanItem {
        ScanItem::from_detection(
            Detection {
                text: text.into(),
                bytes: text.as_bytes().to_vec(),
                format: "QRCode".into(),
                quad: [
                    Point { x, y: 0.0 },
                    Point { x: x + 10.0, y: 0.0 },
                    Point { x: x + 10.0, y: 10.0 },
                    Point { x, y: 10.0 },
                ],
            },
            0,
        )
    }

    #[test]
    fn codes_stay_for_the_hold_time_and_keep_their_order() {
        let hold = Duration::from_secs(8);
        let t0 = Instant::now();
        let mut held = Vec::new();
        merge(&mut held, vec![item("a", 0.0), item("b", 50.0)], t0, hold);
        // "a" leaves the picture, "c" appears.
        merge(
            &mut held,
            vec![item("b", 52.0), item("c", 90.0)],
            t0 + Duration::from_secs(1),
            hold,
        );
        let texts: Vec<_> = held.iter().map(|h| h.item.text.as_str()).collect();
        assert_eq!(texts, ["a", "b", "c"]);
        assert!(!held[0].live && held[1].live && held[2].live);
        // Still there after 7 s, gone after 9 s.
        assert!(!expire(&mut held, t0 + Duration::from_secs(7), hold));
        merge(
            &mut held,
            vec![],
            t0 + Duration::from_secs(9) + Duration::from_millis(1),
            hold,
        );
        assert!(held.is_empty());
    }

    #[test]
    fn zero_hold_shows_only_visible_codes() {
        let t0 = Instant::now();
        let mut held = Vec::new();
        merge(&mut held, vec![item("a", 0.0)], t0, Duration::ZERO);
        merge(&mut held, vec![], t0 + Duration::from_millis(40), Duration::ZERO);
        assert!(held.is_empty());
    }
}
