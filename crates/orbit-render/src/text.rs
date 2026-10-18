//! Minimal single-line text drawing with ab_glyph coverage masks.

use crate::Font;
use ab_glyph::{point, Font as _, Glyph, PxScale, ScaleFont};
use tiny_skia::{Color, Pixmap, Transform};

/// Draws `text` centred on (`cx`, `cy`) in logical units. Only the
/// transform's scale and translation are honoured, which is all the ring
/// uses (no rotation). `bold` fakes extra weight by double-striking.
#[allow(clippy::too_many_arguments)]
pub fn draw_centered(
    pm: &mut Pixmap,
    font: &Font,
    text: &str,
    size: f32,
    cx: f32,
    cy: f32,
    color: Color,
    ts: Transform,
    bold: bool,
) {
    let px = size * ts.sx;
    let scaled = font.face.as_scaled(PxScale::from(px));

    // Lay out on a baseline at y = 0.
    let mut glyphs: Vec<Glyph> = Vec::new();
    let mut x = 0.0f32;
    let mut prev = None;
    for ch in text.chars() {
        let id = scaled.glyph_id(ch);
        if let Some(p) = prev {
            x += scaled.kern(p, id);
        }
        glyphs.push(id.with_scale_and_position(px, point(x, 0.0)));
        x += scaled.h_advance(id);
        prev = Some(id);
    }
    let width = x;
    // Centre the cap height on cy.
    let cap = scaled.ascent() * 0.72;
    let origin_x = (cx * ts.sx + ts.tx) - width / 2.0;
    let origin_y = (cy * ts.sy + ts.ty) + cap / 2.0;

    let (w, h) = (pm.width() as i32, pm.height() as i32);
    let c = color.to_color_u8();
    let data = pm.pixels_mut();
    let strikes: &[f32] = if bold { &[0.0, 0.45] } else { &[0.0] };
    for g in glyphs {
        for &dx in strikes {
            let mut g = g.clone();
            g.position = point(g.position.x + origin_x + dx, g.position.y + origin_y);
            let Some(outline) = font.face.outline_glyph(g) else { continue };
            let b = outline.px_bounds();
            outline.draw(|gx, gy, cov| {
                let x = b.min.x as i32 + gx as i32;
                let y = b.min.y as i32 + gy as i32;
                if x < 0 || y < 0 || x >= w || y >= h {
                    return;
                }
                let a = (cov.clamp(0.0, 1.0) * c.alpha() as f32 / 255.0).min(1.0);
                if a <= 0.0 {
                    return;
                }
                let dst = &mut data[(y * w + x) as usize];
                // Source-over in premultiplied space.
                let inv = 1.0 - a;
                let blend = |s: u8, d: u8| (s as f32 * a + d as f32 * inv).round().min(255.0) as u8;
                let r = blend(c.red(), dst.red());
                let gg = blend(c.green(), dst.green());
                let bb = blend(c.blue(), dst.blue());
                let aa = (255.0 * a + dst.alpha() as f32 * inv).round().min(255.0) as u8;
                if let Some(px) = tiny_skia::PremultipliedColorU8::from_rgba(r.min(aa), gg.min(aa), bb.min(aa), aa) {
                    *dst = px;
                }
            });
        }
    }
}
