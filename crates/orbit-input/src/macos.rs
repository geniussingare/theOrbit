//! macOS backend: a Quartz event tap on the session event stream.
//!
//! Needs the Accessibility permission (System Settings → Privacy & Security →
//! Accessibility), otherwise `CGEventTapCreate` returns null.

use crate::{Engine, Point, Replay, StartError, Verdict, SYNTHETIC_MARKER};
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{mpsc, Arc, OnceLock};

type CFTypeRef = *const c_void;
type CGEventRef = *mut c_void;
type CFMachPortRef = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

type TapCallback = extern "C" fn(proxy: *mut c_void, kind: u32, event: CGEventRef, user: *mut c_void) -> CGEventRef;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(tap: u32, place: u32, options: u32, mask: u64, cb: TapCallback, user: *mut c_void) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    fn CGEventSourceCreate(state: i32) -> CFTypeRef;
    fn CGEventCreateMouseEvent(source: CFTypeRef, kind: u32, at: CGPoint, button: u32) -> CGEventRef;
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
    static kAXTrustedCheckOptionPrompt: CFTypeRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(alloc: CFTypeRef, port: CFMachPortRef, order: isize) -> CFTypeRef;
    fn CFRunLoopGetCurrent() -> CFTypeRef;
    fn CFRunLoopAddSource(rl: CFTypeRef, source: CFTypeRef, mode: CFTypeRef);
    fn CFRunLoopRun();
    fn CFRelease(obj: CFTypeRef);
    fn CFDictionaryCreate(
        alloc: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        count: isize,
        key_cb: *const c_void,
        value_cb: *const c_void,
    ) -> CFTypeRef;
    static kCFRunLoopCommonModes: CFTypeRef;
    static kCFBooleanTrue: CFTypeRef;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
}

const HID_EVENT_TAP: u32 = 0;
const SESSION_EVENT_TAP: u32 = 1;
const HEAD_INSERT: u32 = 0;
const TAP_DEFAULT: u32 = 0;

const RIGHT_DOWN: u32 = 3;
const RIGHT_UP: u32 = 4;
const RIGHT_DRAGGED: u32 = 7;
const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

const FIELD_CLICK_STATE: u32 = 1;
const FIELD_SOURCE_USER_DATA: u32 = 42;
const SOURCE_STATE_HID: i32 = 1;
const BUTTON_RIGHT: u32 = 1;

static ENGINE: OnceLock<Arc<Engine>> = OnceLock::new();
static TAP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

pub fn is_trusted(prompt: bool) -> bool {
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [if prompt { kCFBooleanTrue } else { ptr::null() }];
        if !prompt {
            return AXIsProcessTrustedWithOptions(ptr::null());
        }
        let dict = CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks as *const _,
            &kCFTypeDictionaryValueCallBacks as *const _,
        );
        let trusted = AXIsProcessTrustedWithOptions(dict);
        CFRelease(dict);
        trusted
    }
}

struct MacReplay;

impl MacReplay {
    fn post(kind: u32, at: Point) {
        unsafe {
            let source = CGEventSourceCreate(SOURCE_STATE_HID);
            let event = CGEventCreateMouseEvent(source, kind, CGPoint { x: at.x, y: at.y }, BUTTON_RIGHT);
            if !event.is_null() {
                CGEventSetIntegerValueField(event, FIELD_CLICK_STATE, 1);
                CGEventSetIntegerValueField(event, FIELD_SOURCE_USER_DATA, SYNTHETIC_MARKER);
                CGEventPost(HID_EVENT_TAP, event);
                CFRelease(event);
            }
            if !source.is_null() {
                CFRelease(source);
            }
        }
    }
}

impl Replay for MacReplay {
    fn click(&self, at: Point) {
        Self::post(RIGHT_DOWN, at);
        Self::post(RIGHT_UP, at);
    }
    fn press(&self, at: Point) {
        Self::post(RIGHT_DOWN, at);
    }
}

extern "C" fn on_event(_proxy: *mut c_void, kind: u32, event: CGEventRef, _user: *mut c_void) -> CGEventRef {
    if kind == TAP_DISABLED_BY_TIMEOUT || kind == TAP_DISABLED_BY_USER_INPUT {
        // macOS disables slow taps; turn ourselves back on.
        let tap = TAP.load(Ordering::Acquire);
        if !tap.is_null() {
            unsafe { CGEventTapEnable(tap, true) };
        }
        return event;
    }
    let Some(engine) = ENGINE.get() else { return event };
    unsafe {
        if CGEventGetIntegerValueField(event, FIELD_SOURCE_USER_DATA) == SYNTHETIC_MARKER {
            return event;
        }
        let loc = CGEventGetLocation(event);
        let at = Point::new(loc.x, loc.y);
        let verdict = match kind {
            RIGHT_DOWN => engine.on_down(at),
            RIGHT_DRAGGED => engine.on_drag(at),
            RIGHT_UP => engine.on_up(at),
            _ => Verdict::Pass,
        };
        match verdict {
            Verdict::Pass => event,
            Verdict::Swallow => ptr::null_mut(),
        }
    }
}

pub fn start<F>(make_engine: F) -> Result<Arc<Engine>, StartError>
where
    F: FnOnce(Box<dyn Replay>) -> Arc<Engine>,
{
    if !is_trusted(false) {
        return Err(StartError::PermissionDenied);
    }
    let engine = ENGINE.get_or_init(|| make_engine(Box::new(MacReplay))).clone();
    if !TAP.load(Ordering::Acquire).is_null() {
        return Ok(engine);
    }

    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("orbit-event-tap".into())
        .spawn(move || unsafe {
            let mask = (1u64 << RIGHT_DOWN) | (1u64 << RIGHT_UP) | (1u64 << RIGHT_DRAGGED);
            let tap = CGEventTapCreate(SESSION_EVENT_TAP, HEAD_INSERT, TAP_DEFAULT, mask, on_event, ptr::null_mut());
            if tap.is_null() {
                let _ = tx.send(Err(StartError::PermissionDenied));
                return;
            }
            TAP.store(tap, Ordering::Release);
            let source = CFMachPortCreateRunLoopSource(ptr::null(), tap, 0);
            CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            let _ = tx.send(Ok(()));
            CFRunLoopRun();
        })
        .map_err(|e| StartError::Failed(e.to_string()))?;

    rx.recv().map_err(|e| StartError::Failed(e.to_string()))??;
    Ok(engine)
}
