//! Radial menu geometry.
//!
//! Angles are measured clockwise from 12 o'clock in screen space (y grows
//! downwards). Segment 0 is centred at the top, segment 1 follows clockwise.

use std::f64::consts::TAU;

/// Angle in radians, clockwise from the top, in `[0, TAU)`.
pub fn angle_from_top(dx: f64, dy: f64) -> f64 {
    let a = dx.atan2(-dy);
    if a < 0.0 {
        a + TAU
    } else {
        a
    }
}

/// Returns the segment under the pointer, given its offset from the menu
/// centre. `None` while the pointer is inside the dead zone (release = cancel).
/// Points beyond the outer radius still select, like a classic pie menu.
pub fn segment_at(dx: f64, dy: f64, count: usize, dead_zone: f64) -> Option<usize> {
    if count == 0 || dx.hypot(dy) < dead_zone {
        return None;
    }
    let span = TAU / count as f64;
    let idx = ((angle_from_top(dx, dy) + span / 2.0) / span).floor() as usize;
    Some(idx % count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_zone_cancels() {
        assert_eq!(segment_at(3.0, 4.0, 8, 10.0), None);
        assert_eq!(segment_at(100.0, 0.0, 0, 10.0), None);
    }

    #[test]
    fn four_segments_follow_compass() {
        assert_eq!(segment_at(0.0, -100.0, 4, 10.0), Some(0)); // up
        assert_eq!(segment_at(100.0, 0.0, 4, 10.0), Some(1)); // right
        assert_eq!(segment_at(0.0, 100.0, 4, 10.0), Some(2)); // down
        assert_eq!(segment_at(-100.0, 0.0, 4, 10.0), Some(3)); // left
    }

    #[test]
    fn top_segment_wraps_around_zero() {
        // Slightly left of 12 o'clock still belongs to segment 0.
        assert_eq!(segment_at(-5.0, -100.0, 8, 10.0), Some(0));
        assert_eq!(segment_at(5.0, -100.0, 8, 10.0), Some(0));
    }

    #[test]
    fn far_outside_still_selects() {
        assert_eq!(segment_at(5000.0, 0.0, 4, 10.0), Some(1));
    }

    #[test]
    fn single_item_takes_everything() {
        assert_eq!(segment_at(0.0, 50.0, 1, 10.0), Some(0));
    }
}
