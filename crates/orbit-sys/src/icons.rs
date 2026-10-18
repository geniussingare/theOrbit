//! Native icons for app / file items, returned as PNG bytes.
//!
//! macOS uses the Finder icon. Other platforms fall back to a monogram until
//! their native icon extraction is added.

use orbit_core::{ActionType, OrbitItem};

pub fn for_item(item: &OrbitItem) -> Option<Vec<u8>> {
    if item.icon.as_deref().is_some_and(|i| !i.trim().is_empty()) {
        return None; // the user picked their own monogram
    }
    match item.kind {
        ActionType::App | ActionType::File => native_icon(item.value.trim()),
        _ => None,
    }
}

/// Edge length of extracted icons in pixels (crisp at 44 pt on Retina).
pub const ICON_PX: f64 = 96.0;

#[cfg(target_os = "macos")]
fn native_icon(value: &str) -> Option<Vec<u8>> {
    use objc2::AllocAnyThread;
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSWorkspace};
    use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};

    let path = resolve_mac_path(value)?;
    // Drain AppKit's temporaries right away; icons are large.
    objc2::rc::autoreleasepool(|_| {
        let image = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path.to_str()?));
        // Ask for one small rendition instead of serialising every size (up to 1024 px).
        let mut rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(ICON_PX, ICON_PX));
        let cg = unsafe { image.CGImageForProposedRect_context_hints(&mut rect, None, None) }?;
        let rep = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg);
        let png = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?;
        Some(png.to_vec())
    })
}

#[cfg(target_os = "macos")]
fn resolve_mac_path(value: &str) -> Option<std::path::PathBuf> {
    let direct = crate::actions::expand_tilde(value);
    if direct.is_absolute() {
        return direct.exists().then_some(direct);
    }
    // "Safari" or "Visual Studio Code" → look in the usual application folders.
    let name = if value.ends_with(".app") { value.to_string() } else { format!("{value}.app") };
    ["/Applications", "/System/Applications", "/System/Applications/Utilities", "/Applications/Utilities"]
        .iter()
        .map(|dir| std::path::Path::new(dir).join(&name))
        .find(|p| p.exists())
}

#[cfg(not(target_os = "macos"))]
fn native_icon(_value: &str) -> Option<Vec<u8>> {
    None
}
