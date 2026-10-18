//! macOS presenter: a borderless, clear NSWindow whose layer shows the frame
//! as a CGImage. `orderFrontRegardless` displays it without activating
//! TheOrbit, so the app under the cursor keeps focus.

use super::Presenter;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send, MainThreadMarker};
use objc2_app_kit::{NSBackingStoreType, NSColor, NSScreen, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use orbit_input::Point;
use orbit_render::Pixmap;
use std::ffi::c_void;

type CFTypeRef = *const c_void;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorSpaceCreateDeviceRGB() -> CFTypeRef;
    fn CGDataProviderCreateWithCFData(data: CFTypeRef) -> CFTypeRef;
    #[allow(clippy::too_many_arguments)]
    fn CGImageCreate(
        width: usize,
        height: usize,
        bits_per_component: usize,
        bits_per_pixel: usize,
        bytes_per_row: usize,
        space: CFTypeRef,
        bitmap_info: u32,
        provider: CFTypeRef,
        decode: *const f64,
        should_interpolate: bool,
        intent: i32,
    ) -> CFTypeRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFDataCreate(alloc: CFTypeRef, bytes: *const u8, len: isize) -> CFTypeRef;
    fn CFRelease(obj: CFTypeRef);
}

/// kCGImageAlphaPremultipliedLast with default (big-endian RGBA) byte order,
/// which is exactly tiny-skia's pixel layout.
const PREMULTIPLIED_RGBA: u32 = 1;
const SCREEN_SAVER_LEVEL: isize = 1000;

pub struct MacPresenter {
    mtm: MainThreadMarker,
    window: Retained<NSWindow>,
    scale: f64,
}

impl MacPresenter {
    pub fn new() -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(10.0, 10.0));
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                rect,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));
        window.setIgnoresMouseEvents(true);
        window.setHasShadow(false);
        window.setLevel(SCREEN_SAVER_LEVEL);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        window.contentView()?.setWantsLayer(true);
        Some(Self { mtm, window, scale: 1.0 })
    }

    /// Height of the primary screen; Cocoa's y axis starts at its bottom.
    fn primary_height(&self) -> f64 {
        NSScreen::screens(self.mtm).firstObject().map(|s| s.frame().size.height).unwrap_or(0.0)
    }

    fn set_contents(&self, frame: &Pixmap) {
        let Some(view) = self.window.contentView() else { return };
        unsafe {
            let data = frame.data();
            let cf = CFDataCreate(std::ptr::null(), data.as_ptr(), data.len() as isize);
            let provider = CGDataProviderCreateWithCFData(cf);
            let space = CGColorSpaceCreateDeviceRGB();
            let image = CGImageCreate(
                frame.width() as usize,
                frame.height() as usize,
                8,
                32,
                frame.width() as usize * 4,
                space,
                PREMULTIPLIED_RGBA,
                provider,
                std::ptr::null(),
                false,
                0,
            );
            let layer: *mut AnyObject = msg_send![&*view, layer];
            if !layer.is_null() && !image.is_null() {
                // Swap the image instantly instead of cross-fading.
                let _: () = msg_send![class!(CATransaction), begin];
                let _: () = msg_send![class!(CATransaction), setDisableActions: true];
                let _: () = msg_send![layer, setContents: image as *mut AnyObject];
                let _: () = msg_send![layer, setContentsScale: self.scale];
                let _: () = msg_send![class!(CATransaction), commit];
            }
            for obj in [image, space, provider, cf] {
                if !obj.is_null() {
                    CFRelease(obj);
                }
            }
        }
    }
}

impl Presenter for MacPresenter {
    fn scale_at(&self, at: Point) -> f64 {
        let y = self.primary_height() - at.y;
        NSScreen::screens(self.mtm)
            .iter()
            .find(|s| {
                let f = s.frame();
                at.x >= f.origin.x && at.x < f.origin.x + f.size.width && y >= f.origin.y && y < f.origin.y + f.size.height
            })
            .map(|s| s.backingScaleFactor())
            .unwrap_or(2.0)
    }

    fn show(&mut self, at: Point, frame: &Pixmap, scale: f64) {
        self.scale = scale;
        let side = frame.width() as f64 / scale;
        let origin = NSPoint::new(at.x - side / 2.0, self.primary_height() - at.y - side / 2.0);
        self.window.setFrame_display(NSRect::new(origin, NSSize::new(side, side)), false);
        self.set_contents(frame);
        self.window.orderFrontRegardless();
    }

    fn update(&mut self, frame: &Pixmap) {
        self.set_contents(frame);
    }

    fn hide(&mut self) {
        self.window.orderOut(None);
    }
}
