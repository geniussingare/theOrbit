//! Timing and easing for the ring's motion. Animations are short and
//! time-based, so callers only need to render while `animating()` is true.

use std::f32::consts::TAU;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static REDUCED_MOTION: AtomicBool = AtomicBool::new(false);

/// Follows the OS "reduce motion" setting: every transition jumps straight
/// to its end state.
pub fn set_reduced_motion(on: bool) {
    REDUCED_MOTION.store(on, Ordering::Relaxed);
}

pub fn reduced_motion() -> bool {
    REDUCED_MOTION.load(Ordering::Relaxed)
}

pub const OPEN: Duration = Duration::from_millis(180);
pub const CLOSE: Duration = Duration::from_millis(140);
pub const HOVER: Duration = Duration::from_millis(130);
/// About 60 frames per second while something is moving.
pub const FRAME: Duration = Duration::from_millis(16);

/// Linear progress of an animation in `[0, 1]`.
pub fn progress(start: Instant, duration: Duration, now: Instant) -> f32 {
    if reduced_motion() {
        return 1.0;
    }
    (now.saturating_duration_since(start).as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

/// Overshoots slightly before settling, for a springy pop.
pub fn ease_out_back(t: f32) -> f32 {
    const C1: f32 = 1.4;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

/// Signed shortest rotation from angle `a` to angle `b`.
pub fn shortest_delta(a: f32, b: f32) -> f32 {
    let d = (b - a).rem_euclid(TAU);
    if d > TAU / 2.0 {
        d - TAU
    } else {
        d
    }
}

/// Where the hover highlight is drawn: its angle (clockwise from the top)
/// and how visible it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Highlight {
    pub angle: f32,
    pub opacity: f32,
}

/// Glides the highlight between segments; retargeting mid-flight continues
/// from wherever the highlight currently is.
#[derive(Debug, Clone)]
pub struct HoverTween {
    from: Highlight,
    to: Highlight,
    target: Option<usize>,
    start: Instant,
}

impl HoverTween {
    pub fn new(now: Instant) -> Self {
        let none = Highlight { angle: 0.0, opacity: 0.0 };
        Self { from: none, to: none, target: None, start: now }
    }

    pub fn target(&self) -> Option<usize> {
        self.target
    }

    pub fn set(&mut self, target: Option<usize>, count: usize, now: Instant) {
        if target == self.target {
            return;
        }
        let current = self.at(now);
        let span = TAU / count.max(1) as f32;
        let to = match target {
            Some(i) => Highlight { angle: i as f32 * span, opacity: 1.0 },
            None => Highlight { angle: current.angle, opacity: 0.0 },
        };
        // Appearing from nothing fades in in place instead of sweeping.
        let from = if current.opacity < 0.02 { Highlight { angle: to.angle, opacity: 0.0 } } else { current };
        *self = Self { from, to, target, start: now };
    }

    pub fn at(&self, now: Instant) -> Highlight {
        let t = ease_out_cubic(progress(self.start, HOVER, now));
        Highlight {
            angle: self.from.angle + shortest_delta(self.from.angle, self.to.angle) * t,
            opacity: self.from.opacity + (self.to.opacity - self.from.opacity) * t,
        }
    }

    pub fn done(&self, now: Instant) -> bool {
        progress(self.start, HOVER, now) >= 1.0
    }
}

/// Whole-ring scale and opacity for the open/close transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub scale: f32,
    pub opacity: f32,
}

impl Pose {
    pub const REST: Pose = Pose { scale: 1.0, opacity: 1.0 };

    /// Springs out from the cursor.
    pub fn opening(t: f32) -> Pose {
        Pose { scale: 0.82 + 0.18 * ease_out_back(t), opacity: ease_out_cubic(t) }
    }

    /// Swells away when an item was chosen, shrinks back when cancelled.
    pub fn closing(t: f32, chosen: bool) -> Pose {
        let q = ease_in_cubic(t);
        let scale = if chosen { 1.0 + 0.07 * q } else { 1.0 - 0.12 * q };
        Pose { scale, opacity: 1.0 - ease_out_cubic(t) }
    }

    pub fn is_rest(&self) -> bool {
        (self.scale - 1.0).abs() < 1e-3 && self.opacity > 0.999
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_endpoints() {
        for f in [ease_out_cubic, ease_in_cubic, ease_out_back] {
            assert!(f(0.0).abs() < 1e-5);
            assert!((f(1.0) - 1.0).abs() < 1e-5);
        }
        assert!(ease_out_back(0.7) > 1.0, "back easing overshoots");
    }

    #[test]
    fn shortest_way_round() {
        assert!((shortest_delta(0.1, TAU - 0.1) + 0.2).abs() < 1e-5);
        assert!((shortest_delta(TAU - 0.1, 0.1) - 0.2).abs() < 1e-5);
    }

    #[test]
    fn tween_glides_and_settles() {
        let t0 = Instant::now();
        let mut tw = HoverTween::new(t0);
        tw.set(Some(0), 4, t0);
        // Appears in place at the top.
        assert_eq!(tw.at(t0).angle, 0.0);
        let t1 = t0 + HOVER;
        assert!(tw.done(t1));
        tw.set(Some(3), 4, t1);
        // Segment 3 (left) is reached by turning anticlockwise through 0.
        let mid = tw.at(t1 + HOVER / 2);
        assert!(mid.angle < 0.0 && mid.angle > -TAU / 4.0);
        assert!((tw.at(t1 + HOVER).angle + TAU / 4.0).abs() < 1e-4);
        assert_eq!(tw.at(t1 + HOVER).opacity, 1.0);
    }

    #[test]
    fn poses_end_where_expected() {
        assert_eq!(Pose::opening(1.0), Pose::REST);
        assert_eq!(Pose::closing(1.0, true).opacity, 0.0);
        assert!(Pose::closing(1.0, false).scale < 1.0);
    }
}
