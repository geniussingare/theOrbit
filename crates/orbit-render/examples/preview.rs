//! Renders a sample ring to a PNG: `cargo run -p orbit-render --example preview -- out.png`
use orbit_render::{monogram, render, Font, Ring, RingCache, Segment, Theme};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "ring.png".into());
    let titles = ["Finder", "Safari", "Terminal", "Notes", "Screenshot", "Downloads", "Settings", "Google"];
    let segments: Vec<Segment> =
        titles.iter().map(|t| Segment { title: t.to_string(), icon: None, monogram: monogram(t) }).collect();
    let font = Font::system();
    let theme = Theme::default();
    for hover in [None, Some(2), Some(3)] {
        let t = std::time::Instant::now();
        let _ = render(&Ring { segments: &segments, radius: 150.0, dead_zone: 42.0, hover, scale: 2.0, font: font.as_ref(), theme: &theme, pointer: None, idle_hint: None });
        println!("warm render {:?}", t.elapsed());
    }
    let ring = |hover| Ring { segments: &segments, radius: 150.0, dead_zone: 42.0, hover, scale: 2.0, font: font.as_ref(), theme: &theme, pointer: None, idle_hint: None };
    let t = std::time::Instant::now();
    let cache = RingCache::new(&ring(None));
    println!("cache build {:?}", t.elapsed());
    for hover in [Some(0), Some(4)] {
        let t = std::time::Instant::now();
        let _ = cache.frame(&ring(hover));
        println!("hover frame {:?}", t.elapsed());
    }
    let t = std::time::Instant::now();
    let pm = render(&Ring { segments: &segments, radius: 150.0, dead_zone: 42.0, hover: Some(1), scale: 2.0, font: font.as_ref(), theme: &theme, pointer: None, idle_hint: None });
    println!("rendered {}x{} in {:?}", pm.width(), pm.height(), t.elapsed());
    pm.save_png(&out).unwrap();
}
