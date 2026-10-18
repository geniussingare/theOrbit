//! TheOrbit's mark: a ring with a centre dot and a small satellite, drawn
//! procedurally so the tray and window icons need no image assets.

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Shader, Stroke, Transform};

fn solid(color: Color) -> Paint<'static> {
    Paint { shader: Shader::SolidColor(color), anti_alias: true, ..Paint::default() }
}

/// Draws the orbit mark filling a `size`×`size` pixmap in one `color`
/// (black for a macOS template image).
pub fn mark(size: u32, color: Color) -> Pixmap {
    let mut pm = Pixmap::new(size, size).expect("non-zero size");
    let ts = Transform::from_scale(size as f32, size as f32);
    draw_mark(&mut pm, ts, color, color);
    pm
}

/// The mark in unit space: an open ring (with a gap for the satellite), a
/// solid core and the satellite itself.
fn draw_mark(pm: &mut Pixmap, ts: Transform, ring: Color, accent: Color) {
    let (c, r) = (0.5, 0.34);
    // Satellite sits at 45° (upper right); leave a gap in the ring around it.
    let sat = std::f32::consts::FRAC_PI_4;
    let gap = 0.42;
    let mut pb = PathBuilder::new();
    let steps = 72;
    for i in 0..=steps {
        let a = sat + gap + (std::f32::consts::TAU - 2.0 * gap) * i as f32 / steps as f32;
        let (x, y) = (c + r * a.sin(), c - r * a.cos());
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    if let Some(p) = pb.finish() {
        let stroke = Stroke { width: 0.085, line_cap: tiny_skia::LineCap::Round, ..Stroke::default() };
        pm.stroke_path(&p, &solid(ring), &stroke, ts, None);
    }
    for (x, y, rad, color) in [(c, c, 0.12, ring), (c + r * sat.sin(), c - r * sat.cos(), 0.075, accent)] {
        let mut pb = PathBuilder::new();
        pb.push_circle(x, y, rad);
        if let Some(p) = pb.finish() {
            pm.fill_path(&p, &solid(color), FillRule::Winding, ts, None);
        }
    }
}

/// Straight (non-premultiplied) RGBA bytes, as tray and window icon APIs expect.
pub fn to_straight_rgba(pm: &Pixmap) -> Vec<u8> {
    pm.pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

/// The full-colour app icon: the orbit mark on a dark rounded tile.
pub fn app_icon(size: u32) -> Pixmap {
    use tiny_skia::{GradientStop, LinearGradient, Point, SpreadMode};

    let mut pm = Pixmap::new(size, size).expect("non-zero size");
    let s = size as f32;
    let ts = Transform::from_scale(s, s);

    // Rounded tile with a subtle diagonal gradient (macOS-style inset).
    let (x0, x1, r) = (0.0625, 0.9375, 0.19);
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + r, x0);
    pb.line_to(x1 - r, x0);
    pb.quad_to(x1, x0, x1, x0 + r);
    pb.line_to(x1, x1 - r);
    pb.quad_to(x1, x1, x1 - r, x1);
    pb.line_to(x0 + r, x1);
    pb.quad_to(x0, x1, x0, x1 - r);
    pb.line_to(x0, x0 + r);
    pb.quad_to(x0, x0, x0 + r, x0);
    pb.close();
    if let (Some(tile), Some(shader)) = (
        pb.finish(),
        LinearGradient::new(
            Point::from_xy(x0, x0),
            Point::from_xy(x1, x1),
            vec![
                GradientStop::new(0.0, Color::from_rgba8(34, 31, 58, 255)),
                GradientStop::new(1.0, Color::from_rgba8(12, 11, 22, 255)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        ),
    ) {
        let p = Paint { shader, anti_alias: true, ..Paint::default() };
        pm.fill_path(&tile, &p, FillRule::Winding, ts, None);
    }

    // The mark, inset to ~62% of the tile, with a violet satellite.
    let k = 0.62;
    let inner = ts.pre_translate((1.0 - k) / 2.0, (1.0 - k) / 2.0).pre_scale(k, k);
    draw_mark(&mut pm, inner, Color::from_rgba8(242, 240, 255, 255), Color::from_rgba8(139, 114, 255, 255));
    pm
}
