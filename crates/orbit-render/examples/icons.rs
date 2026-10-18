//! Regenerates the app icon set from `logo::app_icon`:
//! `cargo run -p orbit-render --example icons -- assets/icons`
//! Writes PNGs, a macOS `.iconset` folder (for `iconutil`) and a Windows `.ico`.
use orbit_render::logo::app_icon;
use std::path::Path;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "assets/icons".into());
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    for (name, size) in [("32x32.png", 32), ("64x64.png", 64), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 1024)] {
        app_icon(size).save_png(dir.join(name)).unwrap();
    }

    let iconset = dir.join("icon.iconset");
    std::fs::create_dir_all(&iconset).unwrap();
    for base in [16, 32, 128, 256, 512] {
        app_icon(base).save_png(iconset.join(format!("icon_{base}x{base}.png"))).unwrap();
        app_icon(base * 2).save_png(iconset.join(format!("icon_{base}x{base}@2x.png"))).unwrap();
    }

    // ICO with PNG-encoded entries (supported since Windows Vista).
    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let pngs: Vec<Vec<u8>> = sizes.iter().map(|&s| app_icon(s).encode_png().unwrap()).collect();
    let mut ico = vec![0, 0, 1, 0];
    ico.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&s, png) in sizes.iter().zip(&pngs) {
        let dim = if s >= 256 { 0 } else { s as u8 };
        ico.extend_from_slice(&[dim, dim, 0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes()); // planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for png in &pngs {
        ico.extend_from_slice(png);
    }
    std::fs::write(dir.join("icon.ico"), ico).unwrap();
}
