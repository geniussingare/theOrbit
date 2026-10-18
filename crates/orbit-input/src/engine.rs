//! Platform-independent gesture state machine.
//!
//! ```text
//!            right down (swallowed)
//!   Idle ───────────────────────────▶ Pending ──hold timer──▶ Active ──up──▶ Idle (Close)
//!    ▲                                 │   │                    │
//!    │◀── up: replay a normal click ───┘   └─ moved > threshold: replay down ─▶ Passthrough
//!    │◀──────────────────────────── up (passed to apps) ─────────────────────────┘
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    fn dist(self, o: Point) -> f64 {
        (self.x - o.x).hypot(self.y - o.y)
    }
}

/// What the native hook should do with the event it just reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Swallow,
}

/// Events for the UI layer. Coordinates are in the backend's native screen
/// space (points on macOS, physical pixels on Windows/X11).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Output {
    Open(Point),
    Move(Point),
    Close(Point),
}

/// Re-injects right-button events the engine swallowed.
pub trait Replay: Send + Sync {
    /// A full click (down + up), used when the button was released early.
    fn click(&self, at: Point);
    /// Only the press, used when the user started a right-drag.
    fn press(&self, at: Point);
}

#[derive(Debug, Clone, Copy)]
enum State {
    Idle,
    Pending { origin: Point, gen: u64 },
    Active,
    Passthrough,
}

pub struct Engine {
    state: Mutex<State>,
    gen: AtomicU64,
    enabled: AtomicBool,
    hold_ms: AtomicU64,
    drag_threshold_bits: AtomicU64,
    replay: Box<dyn Replay>,
    sink: Box<dyn Fn(Output) + Send + Sync>,
}

impl Engine {
    pub fn new(replay: Box<dyn Replay>, sink: impl Fn(Output) + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State::Idle),
            gen: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
            hold_ms: AtomicU64::new(350),
            drag_threshold_bits: AtomicU64::new(8f64.to_bits()),
            replay,
            sink: Box::new(sink),
        })
    }

    pub fn configure(&self, enabled: bool, hold_ms: u64, drag_threshold: f64) {
        self.enabled.store(enabled, Ordering::Relaxed);
        self.hold_ms.store(hold_ms.max(50), Ordering::Relaxed);
        self.drag_threshold_bits.store(drag_threshold.max(0.0).to_bits(), Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn on_down(self: &Arc<Self>, at: Point) -> Verdict {
        let mut state = self.state.lock().unwrap();
        if !self.is_enabled() {
            *state = State::Passthrough;
            return Verdict::Pass;
        }
        let gen = self.gen.fetch_add(1, Ordering::Relaxed) + 1;
        *state = State::Pending { origin: at, gen };
        drop(state);

        let me = Arc::clone(self);
        let hold = Duration::from_millis(self.hold_ms.load(Ordering::Relaxed));
        std::thread::spawn(move || {
            std::thread::sleep(hold);
            let mut state = me.state.lock().unwrap();
            if let State::Pending { origin, gen: g } = *state {
                if g == gen {
                    *state = State::Active;
                    drop(state);
                    (me.sink)(Output::Open(origin));
                }
            }
        });
        Verdict::Swallow
    }

    pub fn on_drag(&self, at: Point) -> Verdict {
        let mut state = self.state.lock().unwrap();
        match *state {
            State::Pending { origin, .. } => {
                let threshold = f64::from_bits(self.drag_threshold_bits.load(Ordering::Relaxed));
                if at.dist(origin) > threshold {
                    *state = State::Passthrough;
                    drop(state);
                    self.replay.press(origin);
                }
                Verdict::Pass
            }
            State::Active => {
                drop(state);
                (self.sink)(Output::Move(at));
                Verdict::Swallow
            }
            State::Idle | State::Passthrough => Verdict::Pass,
        }
    }

    pub fn on_up(&self, at: Point) -> Verdict {
        let mut state = self.state.lock().unwrap();
        let prev = std::mem::replace(&mut *state, State::Idle);
        drop(state);
        match prev {
            State::Pending { origin, .. } => {
                self.replay.click(origin);
                Verdict::Swallow
            }
            State::Active => {
                (self.sink)(Output::Close(at));
                Verdict::Swallow
            }
            State::Idle | State::Passthrough => Verdict::Pass,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[derive(Default)]
    struct Rec(Mutex<Vec<&'static str>>);
    impl Replay for Arc<Rec> {
        fn click(&self, _: Point) {
            self.0.lock().unwrap().push("click");
        }
        fn press(&self, _: Point) {
            self.0.lock().unwrap().push("press");
        }
    }

    fn engine() -> (Arc<Engine>, Arc<Rec>, mpsc::Receiver<Output>) {
        let rec = Arc::new(Rec::default());
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let e = Engine::new(Box::new(rec.clone()), move |o| tx.lock().unwrap().send(o).unwrap());
        e.configure(true, 60, 8.0);
        (e, rec, rx)
    }

    #[test]
    fn short_click_is_replayed() {
        let (e, rec, rx) = engine();
        assert_eq!(e.on_down(Point::new(0., 0.)), Verdict::Swallow);
        assert_eq!(e.on_up(Point::new(0., 0.)), Verdict::Swallow);
        assert_eq!(*rec.0.lock().unwrap(), ["click"]);
        std::thread::sleep(Duration::from_millis(120));
        assert!(rx.try_recv().is_err(), "timer must not open after release");
    }

    #[test]
    fn hold_opens_and_release_closes() {
        let (e, rec, rx) = engine();
        e.on_down(Point::new(10., 10.));
        assert_eq!(rx.recv_timeout(Duration::from_secs(1)).unwrap(), Output::Open(Point::new(10., 10.)));
        assert_eq!(e.on_drag(Point::new(50., 10.)), Verdict::Swallow);
        assert_eq!(rx.recv().unwrap(), Output::Move(Point::new(50., 10.)));
        assert_eq!(e.on_up(Point::new(50., 10.)), Verdict::Swallow);
        assert_eq!(rx.recv().unwrap(), Output::Close(Point::new(50., 10.)));
        assert!(rec.0.lock().unwrap().is_empty());
    }

    #[test]
    fn early_drag_becomes_passthrough() {
        let (e, rec, rx) = engine();
        e.on_down(Point::new(0., 0.));
        assert_eq!(e.on_drag(Point::new(3., 0.)), Verdict::Pass);
        assert_eq!(e.on_drag(Point::new(30., 0.)), Verdict::Pass);
        assert_eq!(*rec.0.lock().unwrap(), ["press"]);
        std::thread::sleep(Duration::from_millis(120));
        assert!(rx.try_recv().is_err());
        assert_eq!(e.on_up(Point::new(30., 0.)), Verdict::Pass);
    }

    #[test]
    fn disabled_passes_everything() {
        let (e, rec, _rx) = engine();
        e.configure(false, 60, 8.0);
        assert_eq!(e.on_down(Point::new(0., 0.)), Verdict::Pass);
        assert_eq!(e.on_up(Point::new(0., 0.)), Verdict::Pass);
        assert!(rec.0.lock().unwrap().is_empty());
    }
}
