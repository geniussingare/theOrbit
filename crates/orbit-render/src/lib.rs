//! Draws the orbit ring into a premultiplied RGBA pixmap on the CPU.
//!
//! The idle ring is rendered once per open and cached; animation frames only
//! redraw the moving highlight and the hub on top of it (about a
//! millisecond), and only while something is moving. Geometry matches `orbit_core::geometry`: angles run
//! clockwise from 12 o'clock and segment 0 is centred at the top.

pub mod anim;
mod font;
pub mod logo;
mod text;

pub use font::Font;
pub use tiny_skia::{Color, FilterQuality, Pixmap, PixmapPaint, Transform};

use std::f32::consts::TAU;
use tiny_skia::{FillRule, GradientStop, Paint, PathBuilder, Point, RadialGradient, Rect, Shader, SpreadMode, Stroke};

/// Space around the ring for the hover pop-out and the soft shadow.
pub const MARGIN: f32 = 36.0;
/// Gap between segments, in radians.
const GAP: f32 = 0.035;

/// Side of the square overlay, in logical pixels, for a given ring radius.
pub fn side(radius: f32) -> f32 {
    2.0 * (radius + MARGIN)
}

/// Everything the ring needs to draw one segment.
pub struct Segment {
    pub title: String,
    /// Native icon, if one was found. Drawn instead of the monogram.
    pub icon: Option<Pixmap>,
    /// 1–2 characters shown when there is no icon.
    pub monogram: String,
}

/// Colours of the ring. One preset today ([`Theme::orbit`]); more can be
/// added later without touching the renderer.
pub struct Theme {
    pub segment: Color,
    pub segment_active: Color,
    pub outline: Color,
    pub hub: Color,
    pub ink: Color,
    pub ink_dim: Color,
    pub monogram_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::orbit()
    }
}

impl Theme {
    /// Calm dark ring with a soft violet accent.
    pub fn orbit() -> Self {
        Self {
            segment: rgba(24, 23, 36, 0.90),
            segment_active: rgba(124, 92, 255, 0.95),
            outline: rgba(255, 255, 255, 0.08),
            hub: rgba(14, 13, 24, 0.95),
            ink: rgba(244, 242, 255, 1.0),
            ink_dim: rgba(244, 242, 255, 0.70),
            monogram_bg: rgba(255, 255, 255, 0.10),
        }
    }
}

fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::from_rgba8(r, g, b, (a * 255.0).round() as u8)
}

#[derive(Clone, Copy)]
pub struct Ring<'a> {
    pub segments: &'a [Segment],
    pub radius: f32,
    pub dead_zone: f32,
    pub hover: Option<usize>,
    /// Device pixels per logical pixel (2.0 on Retina).
    pub scale: f32,
    pub font: Option<&'a Font>,
    pub theme: &'a Theme,
    /// Direction of the pointer from the centre (radians, clockwise from the
    /// top), drawn as a small notch on the hub so the ring feels connected
    /// to the cursor. `None` while the pointer is still near the centre.
    pub pointer: Option<f32>,
    /// Hub caption while nothing is selected (e.g. "Cancel").
    pub idle_hint: Option<&'a str>,
}

fn polar(c: f32, r: f32, a: f32) -> (f32, f32) {
    (c + r * a.sin(), c - r * a.cos())
}

/// Appends an arc from angle `a0` to `a1` (either direction) as cubic Béziers.
fn arc(pb: &mut PathBuilder, c: f32, r: f32, a0: f32, a1: f32) {
    let pieces = ((a1 - a0).abs() / (TAU / 4.0)).ceil().max(1.0) as usize;
    let step = (a1 - a0) / pieces as f32;
    for i in 0..pieces {
        let t0 = a0 + step * i as f32;
        let t1 = t0 + step;
        let k = 4.0 / 3.0 * (step / 4.0).tan() * r;
        let (x0, y0) = polar(c, r, t0);
        let (x3, y3) = polar(c, r, t1);
        // d/dθ of polar() is (r·cosθ, r·sinθ).
        pb.cubic_to(x0 + k * t0.cos(), y0 + k * t0.sin(), x3 - k * t1.cos(), y3 - k * t1.sin(), x3, y3);
    }
}

fn wedge(c: f32, r0: f32, r1: f32, a0: f32, a1: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let (x, y) = polar(c, r1, a0);
    pb.move_to(x, y);
    arc(&mut pb, c, r1, a0, a1);
    let (x, y) = polar(c, r0, a1);
    pb.line_to(x, y);
    arc(&mut pb, c, r0, a1, a0);
    pb.close();
    pb.finish()
}

fn annulus(c: f32, r0: f32, r1: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.push_circle(c, c, r1);
    pb.push_circle(c, c, r0);
    pb.finish()
}

fn paint(color: Color) -> Paint<'static> {
    Paint { shader: Shader::SolidColor(color), anti_alias: true, ..Paint::default() }
}

/// Inner radius of the segments (the hub sits inside it).
pub fn inner_radius(radius: f32, dead_zone: f32) -> f32 {
    (dead_zone + 6.0).max(radius * 0.38)
}

/// Precomputed layout for one ring.
struct Layout {
    ts: Transform,
    c: f32,
    r0: f32,
    r1: f32,
    mid: f32,
    span: f32,
    icon_size: f32,
    show_labels: bool,
}

impl Layout {
    fn new(ring: &Ring) -> Self {
        let s = ring.scale.max(0.5);
        let n = ring.segments.len();
        let r0 = inner_radius(ring.radius, ring.dead_zone);
        let r1 = ring.radius;
        let mid = (r0 + r1) / 2.0;
        let span = TAU / n.max(1) as f32;
        Self {
            ts: Transform::from_scale(s, s),
            c: side(ring.radius) / 2.0,
            r0,
            r1,
            mid,
            span,
            icon_size: 44f32.min((r1 - r0) * 0.5).min(if n > 1 { mid * span * 0.5 } else { 44.0 }),
            show_labels: n > 0 && mid * span > 60.0,
        }
    }
}

/// Renders the full ring with `ring.hover` highlighted.
pub fn render(ring: &Ring) -> Pixmap {
    let lay = Layout::new(ring);
    let mut pm = base(ring, &lay);
    if let Some(i) = ring.hover {
        draw_segment(&mut pm, ring, &lay, i, true);
    }
    draw_hub(&mut pm, ring, &lay);
    pm
}

/// Caches the idle ring so hover changes only redraw one segment and the hub
/// (a memcpy plus a small fill instead of a full frame).
pub struct RingCache {
    base: Pixmap,
}

impl RingCache {
    pub fn new(ring: &Ring) -> Self {
        let lay = Layout::new(ring);
        Self { base: base(ring, &lay) }
    }

    /// `ring` must describe the same segments/geometry the cache was built from.
    pub fn frame(&self, ring: &Ring) -> Pixmap {
        let lay = Layout::new(ring);
        let mut pm = self.base.clone();
        if let Some(i) = ring.hover {
            draw_segment(&mut pm, ring, &lay, i, true);
        }
        draw_hub(&mut pm, ring, &lay);
        pm
    }

    /// A frame with the highlight at an arbitrary (animated) position.
    /// `ring.hover` picks the title shown in the hub.
    pub fn frame_animated(&self, ring: &Ring, hl: anim::Highlight) -> Pixmap {
        let lay = Layout::new(ring);
        let mut pm = self.base.clone();
        draw_highlight(&mut pm, ring, &lay, hl);
        draw_hub(&mut pm, ring, &lay);
        pm
    }
}

/// Applies the whole-ring open/close pose (scale about the centre and fade).
/// Returns the frame untouched when it is at rest.
pub fn compose(frame: Pixmap, pose: anim::Pose) -> Pixmap {
    if pose.is_rest() {
        return frame;
    }
    let mut out = Pixmap::new(frame.width(), frame.height()).expect("same size as frame");
    if pose.opacity <= 0.0 || pose.scale <= 0.0 {
        return out;
    }
    let c = frame.width() as f32 / 2.0;
    let ts = Transform::from_translate(c, c).pre_scale(pose.scale, pose.scale).pre_translate(-c, -c);
    let p = PixmapPaint { opacity: pose.opacity.min(1.0), quality: FilterQuality::Bilinear, ..PixmapPaint::default() };
    out.draw_pixmap(0, 0, frame.as_ref(), &p, ts, None);
    out
}

fn angular_distance(a: f32, b: f32) -> f32 {
    anim::shortest_delta(a, b).abs()
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: f32, y: f32| x + (y - x) * t;
    Color::from_rgba(mix(a.red(), b.red()), mix(a.green(), b.green()), mix(a.blue(), b.blue()), mix(a.alpha(), b.alpha()))
        .unwrap_or(b)
}

/// Draws the accent wedge at `hl.angle`, then redraws the icons it passes
/// over so they stay on top; the closer a segment is to the highlight, the
/// more it pops.
fn draw_highlight(pm: &mut Pixmap, ring: &Ring, lay: &Layout, hl: anim::Highlight) {
    let n = ring.segments.len();
    if n == 0 || hl.opacity <= 0.01 {
        return;
    }
    let theme = ring.theme;
    let pop = 1.0 + 0.06 * hl.opacity;
    let (hx, hy) = polar(lay.c, lay.mid, hl.angle);
    let ts = lay.ts.pre_translate(hx, hy).pre_scale(pop, pop).pre_translate(-hx, -hy);
    let path = if n == 1 {
        annulus(lay.c, lay.r0, lay.r1)
    } else {
        wedge(lay.c, lay.r0, lay.r1, hl.angle - lay.span / 2.0 + GAP, hl.angle + lay.span / 2.0 - GAP)
    };
    if let Some(path) = path {
        let mut fill = theme.segment_active;
        fill.apply_opacity(hl.opacity);
        pm.fill_path(&path, &paint(fill), FillRule::EvenOdd, ts, None);
        let mut edge = theme.outline;
        edge.apply_opacity(hl.opacity);
        pm.stroke_path(&path, &paint(edge), &Stroke { width: 1.0, ..Stroke::default() }, ts, None);
    }

    let pos = hl.angle.rem_euclid(TAU) / lay.span;
    let i0 = pos.floor() as usize % n;
    let i1 = (i0 + 1) % n;
    for i in if i0 == i1 { vec![i0] } else { vec![i0, i1] } {
        let a = i as f32 * lay.span;
        let near = (1.0 - angular_distance(hl.angle, a) / lay.span).clamp(0.0, 1.0) * hl.opacity;
        if near <= 0.01 {
            continue; // not under the highlight; the cached base already shows it
        }
        let (ix, iy) = polar(lay.c, lay.mid, a);
        let k = 1.0 + 0.06 * near;
        let ts_i = lay.ts.pre_translate(ix, iy).pre_scale(k, k).pre_translate(-ix, -iy);
        draw_content(pm, ring, lay, i, ts_i, near);
    }
}

/// Shadow plus every segment in its idle state.
fn base(ring: &Ring, lay: &Layout) -> Pixmap {
    let size_px = (side(ring.radius) * ring.scale.max(0.5)).ceil() as u32;
    let mut pm = Pixmap::new(size_px, size_px).expect("non-zero pixmap");
    let (c, r1) = (lay.c, lay.r1);

    // Soft shadow: one radial-gradient annulus just outside the ring.
    let reach = r1 + 22.0;
    if let (Some(shader), Some(path)) = (
        RadialGradient::new(
            Point::from_xy(c, c + 3.0),
            0.0,
            Point::from_xy(c, c + 3.0),
            reach,
            vec![
                GradientStop::new((r1 - 4.0) / reach, rgba(0, 0, 0, 0.30)),
                GradientStop::new(1.0, rgba(0, 0, 0, 0.0)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        ),
        annulus(c, r1 - 4.0, reach),
    ) {
        let p = Paint { shader, anti_alias: true, ..Paint::default() };
        pm.fill_path(&path, &p, FillRule::EvenOdd, lay.ts, None);
    }

    for i in 0..ring.segments.len() {
        draw_segment(&mut pm, ring, lay, i, false);
    }
    pm
}

fn draw_segment(pm: &mut Pixmap, ring: &Ring, lay: &Layout, i: usize, active: bool) {
    if i >= ring.segments.len() {
        return;
    }
    let theme = ring.theme;
    let n = ring.segments.len();
    let a = i as f32 * lay.span;
    let (ix, iy) = polar(lay.c, lay.mid, a);
    // The hovered segment pops out slightly around its icon centre.
    let ts = if active { lay.ts.pre_translate(ix, iy).pre_scale(1.06, 1.06).pre_translate(-ix, -iy) } else { lay.ts };
    let outline = Stroke { width: 1.0, ..Stroke::default() };

    let path = if n == 1 {
        annulus(lay.c, lay.r0, lay.r1)
    } else {
        wedge(lay.c, lay.r0, lay.r1, a - lay.span / 2.0 + GAP, a + lay.span / 2.0 - GAP)
    };
    if let Some(path) = path {
        let fill = if active { theme.segment_active } else { theme.segment };
        pm.fill_path(&path, &paint(fill), FillRule::EvenOdd, ts, None);
        pm.stroke_path(&path, &paint(theme.outline), &outline, ts, None);
    }
    draw_content(pm, ring, lay, i, ts, if active { 1.0 } else { 0.0 });
}

/// Icon (or monogram) and label of segment `i`. `active` in `[0, 1]` blends
/// the label from dim to full ink.
fn draw_content(pm: &mut Pixmap, ring: &Ring, lay: &Layout, i: usize, ts: Transform, active: f32) {
    let Some(seg) = ring.segments.get(i) else { return };
    let theme = ring.theme;
    let (ix, iy) = polar(lay.c, lay.mid, i as f32 * lay.span);
    let icon_size = lay.icon_size;
    let icon_y = if lay.show_labels { iy - 7.0 } else { iy };
    match &seg.icon {
        Some(icon) => {
            let k = icon_size / icon.width().max(icon.height()) as f32;
            let t = ts
                .pre_translate(ix - icon.width() as f32 * k / 2.0, icon_y - icon.height() as f32 * k / 2.0)
                .pre_scale(k, k);
            let p = PixmapPaint { quality: FilterQuality::Bicubic, ..PixmapPaint::default() };
            pm.draw_pixmap(0, 0, icon.as_ref(), &p, t, None);
        }
        None => {
            let half = icon_size * 0.45;
            if let Some(rect) = Rect::from_xywh(ix - half, icon_y - half, half * 2.0, half * 2.0) {
                let mut pb = PathBuilder::new();
                pb.push_oval(rect);
                if let Some(p) = pb.finish() {
                    pm.fill_path(&p, &paint(theme.monogram_bg), FillRule::Winding, ts, None);
                }
            }
            if let Some(font) = ring.font {
                text::draw_centered(pm, font, &seg.monogram, icon_size * 0.42, ix, icon_y, theme.ink, ts, true);
            }
        }
    }
    if lay.show_labels {
        if let Some(font) = ring.font {
            let color = lerp_color(theme.ink_dim, theme.ink, active);
            let label = truncate(&seg.title, 12);
            text::draw_centered(pm, font, &label, 11.0, ix, iy + icon_size / 2.0 + 9.0, color, ts, false);
        }
    }
}

/// Hub with the hovered item's name.
fn draw_hub(pm: &mut Pixmap, ring: &Ring, lay: &Layout) {
    let theme = ring.theme;
    let outline = Stroke { width: 1.0, ..Stroke::default() };
    let mut pb = PathBuilder::new();
    pb.push_circle(lay.c, lay.c, (lay.r0 - 8.0).max(8.0));
    if let Some(p) = pb.finish() {
        pm.fill_path(&p, &paint(theme.hub), FillRule::Winding, lay.ts, None);
        pm.stroke_path(&p, &paint(theme.outline), &outline, lay.ts, None);
    }
    if let Some(angle) = ring.pointer {
        // A short accent arc in the gap between hub and segments.
        let r = lay.r0 - 4.0;
        let mut pb = PathBuilder::new();
        let (x, y) = polar(lay.c, r, angle - 0.32);
        pb.move_to(x, y);
        arc(&mut pb, lay.c, r, angle - 0.32, angle + 0.32);
        if let Some(p) = pb.finish() {
            // Ink, not accent: it usually sits right next to the highlighted wedge.
            let notch = Stroke { width: 2.5, line_cap: tiny_skia::LineCap::Round, ..Stroke::default() };
            pm.stroke_path(&p, &paint(theme.ink), &notch, lay.ts, None);
        }
    }
    let Some(font) = ring.font else { return };
    match ring.hover.and_then(|i| ring.segments.get(i)) {
        Some(seg) => text::draw_centered(pm, font, &truncate(&seg.title, 14), 13.0, lay.c, lay.c, theme.ink, lay.ts, true),
        None => {
            if let Some(hint) = ring.idle_hint {
                text::draw_centered(pm, font, hint, 12.0, lay.c, lay.c, theme.ink_dim, lay.ts, false);
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    } else {
        s.to_string()
    }
}

/// First letter of up to two words, e.g. "Visual Studio" → "VS".
pub fn monogram(title: &str) -> String {
    let letters: String = title
        .split_whitespace()
        .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
        .take(2)
        .collect::<String>()
        .to_uppercase();
    if letters.is_empty() {
        "•".into()
    } else {
        letters
    }
}

/// Decodes a PNG (e.g. an app icon) for use as a segment icon.
pub fn decode_png(bytes: &[u8]) -> Option<Pixmap> {
    Pixmap::decode_png(bytes).ok()
}

/// Converts the premultiplied RGBA pixmap to premultiplied BGRA, the layout
/// Windows layered windows and 32-bit X11 visuals expect.
pub fn to_bgra(pm: &Pixmap) -> Vec<u8> {
    let mut out = pm.data().to_vec();
    for px in out.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segs(n: usize) -> Vec<Segment> {
        (0..n).map(|i| Segment { title: format!("Item {i}"), icon: None, monogram: "I".into() }).collect()
    }

    fn pixel(pm: &Pixmap, x: f32, y: f32, scale: f32) -> tiny_skia::PremultipliedColorU8 {
        pm.pixel((x * scale) as u32, (y * scale) as u32).unwrap()
    }

    #[test]
    fn size_follows_radius_and_scale() {
        let theme = Theme::default();
        let items = segs(4);
        let pm = render(&Ring { segments: &items, radius: 100.0, dead_zone: 40.0, hover: None, scale: 2.0, font: None, theme: &theme, pointer: None, idle_hint: None });
        assert_eq!(pm.width(), (side(100.0) * 2.0) as u32);
    }

    #[test]
    fn hover_changes_segment_colour_and_corners_stay_clear() {
        let theme = Theme::default();
        let items = segs(4);
        let ring = |hover| Ring { segments: &items, radius: 100.0, dead_zone: 40.0, hover, scale: 1.0, font: None, theme: &theme, pointer: None, idle_hint: None };
        let idle = render(&ring(None));
        let hot = render(&ring(Some(1)));
        let c = side(100.0) / 2.0;
        // A point inside segment 1 (to the right of centre), away from the icon.
        let (x, y) = (c + 92.0, c + 12.0);
        assert_ne!(pixel(&idle, x, y, 1.0), pixel(&hot, x, y, 1.0));
        // Segment 3 (left) is untouched by hovering segment 1.
        assert_eq!(pixel(&idle, c - 92.0, c + 12.0, 1.0), pixel(&hot, c - 92.0, c + 12.0, 1.0));
        assert_eq!(idle.pixel(0, 0).unwrap().alpha(), 0);
    }

    #[test]
    fn monograms() {
        assert_eq!(monogram("Visual Studio Code"), "VS");
        assert_eq!(monogram("safari"), "S");
        assert_eq!(monogram("  "), "•");
    }

    #[test]
    fn bgra_swaps_channels() {
        let mut pm = Pixmap::new(1, 1).unwrap();
        pm.fill(Color::from_rgba8(255, 0, 0, 255));
        assert_eq!(to_bgra(&pm), vec![0, 0, 255, 255]);
    }
}
