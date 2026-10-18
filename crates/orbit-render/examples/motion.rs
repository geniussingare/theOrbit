//! Renders animation keyframes side by side into one PNG:
//! `cargo run -p orbit-render --example motion -- out.png`
use orbit_render::anim::{Highlight, Pose};
use orbit_render::{compose, monogram, Font, Pixmap, PixmapPaint, Ring, RingCache, Segment, Theme, Transform};
use std::f32::consts::TAU;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "motion.png".into());
    let theme = Theme::orbit();
    let titles = ["Finder", "Safari", "Terminal", "Notes", "Screenshot", "Downloads", "Settings", "Google"];
    let segments: Vec<Segment> =
        titles.iter().map(|t| Segment { title: t.to_string(), icon: None, monogram: monogram(t) }).collect();
    let font = Font::system();
    let span = TAU / 8.0;
    let ring = |hover, pointer| Ring {
        segments: &segments, radius: 150.0, dead_zone: 42.0, hover, scale: 1.0,
        font: font.as_ref(), theme: &theme, pointer, idle_hint: Some("Cancel"),
    };
    let cache = RingCache::new(&ring(None, None));
    let none = Highlight { angle: 0.0, opacity: 0.0 };
    let frames = [
        // Opening, 40% in.
        compose(cache.frame_animated(&ring(None, None), none), Pose::opening(0.4)),
        // Idle.
        cache.frame_animated(&ring(None, None), none),
        // Highlight gliding from Safari towards Terminal.
        cache.frame_animated(&ring(Some(2), Some(1.6 * span)), Highlight { angle: 1.55 * span, opacity: 1.0 }),
        // Settled on Terminal.
        cache.frame_animated(&ring(Some(2), Some(2.0 * span)), Highlight { angle: 2.0 * span, opacity: 1.0 }),
        // Chosen: swelling away.
        compose(cache.frame_animated(&ring(Some(2), Some(2.0 * span)), Highlight { angle: 2.0 * span, opacity: 1.0 }), Pose::closing(0.5, true)),
    ];
    let w = frames[0].width();
    let mut sheet = Pixmap::new(w * frames.len() as u32, w).unwrap();
    sheet.fill(orbit_render::Color::from_rgba8(64, 68, 84, 255));
    for (i, f) in frames.iter().enumerate() {
        sheet.draw_pixmap((i as u32 * w) as i32, 0, f.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    sheet.save_png(&out).unwrap();
}
