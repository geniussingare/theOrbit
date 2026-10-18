//! Loads a system UI font, memory-mapped so only the glyphs we use get paged in.

use ab_glyph::FontRef;

pub struct Font {
    pub(crate) face: FontRef<'static>,
    bytes: &'static [u8],
}

#[cfg(target_os = "macos")]
const CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/SFNS.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
];

#[cfg(target_os = "windows")]
const CANDIDATES: &[&str] = &[
    "C:\\Windows\\Fonts\\segoeui.ttf",
    "C:\\Windows\\Fonts\\arial.ttf",
];

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
];

impl Font {
    /// The first usable system UI font, or `None` (labels are then skipped).
    pub fn system() -> Option<Font> {
        CANDIDATES.iter().find_map(|p| Self::from_path(p))
    }

    pub fn from_path(path: &str) -> Option<Font> {
        let file = std::fs::File::open(path).ok()?;
        // SAFETY: system font files are not modified while we run.
        let map = unsafe { memmap2::Mmap::map(&file) }.ok()?;
        // The font lives for the whole process.
        let bytes: &'static [u8] = Box::leak(Box::new(map));
        let face = FontRef::try_from_slice_and_index(bytes, 0).ok()?;
        Some(Font { face, bytes })
    }

    /// The raw font file (memory-mapped), e.g. to share it with a UI toolkit.
    pub fn bytes(&self) -> &'static [u8] {
        self.bytes
    }

    /// Whether the font can draw every character of `text`.
    pub fn covers(&self, text: &str) -> bool {
        use ab_glyph::Font as _;
        text.chars().all(|c| c.is_whitespace() || self.face.glyph_id(c).0 != 0)
    }
}
