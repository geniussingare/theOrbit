//! Windows backend: a low-level mouse hook (`WH_MOUSE_LL`).
//!
//! Mouse-move events are only observed, never swallowed: blocking them in a
//! low-level hook would freeze the cursor.

use crate::{Engine, Point, Replay, StartError, Verdict, SYNTHETIC_MARKER};
use std::sync::{mpsc, Arc, OnceLock};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, MOUSE_EVENT_FLAGS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, HC_ACTION, MSG,
    MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

static ENGINE: OnceLock<Arc<Engine>> = OnceLock::new();

struct WinReplay;

fn send(flags: &[MOUSE_EVENT_FLAGS]) {
    let inputs: Vec<INPUT> = flags
        .iter()
        .map(|&f| INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT { dwFlags: f, dwExtraInfo: SYNTHETIC_MARKER as usize, ..Default::default() },
            },
        })
        .collect();
    // Inject from another thread: the hook procedure must return quickly.
    std::thread::spawn(move || unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    });
}

impl Replay for WinReplay {
    fn click(&self, _at: Point) {
        send(&[MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP]);
    }
    fn press(&self, _at: Point) {
        send(&[MOUSEEVENTF_RIGHTDOWN]);
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        if let Some(engine) = ENGINE.get() {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            if info.dwExtraInfo != SYNTHETIC_MARKER as usize {
                let at = Point::new(info.pt.x as f64, info.pt.y as f64);
                let verdict = match wparam.0 as u32 {
                    WM_RBUTTONDOWN => engine.on_down(at),
                    WM_RBUTTONUP => engine.on_up(at),
                    WM_MOUSEMOVE => {
                        engine.on_drag(at);
                        Verdict::Pass
                    }
                    _ => Verdict::Pass,
                };
                if verdict == Verdict::Swallow {
                    return LRESULT(1);
                }
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

pub fn start<F>(make_engine: F) -> Result<Arc<Engine>, StartError>
where
    F: FnOnce(Box<dyn Replay>) -> Arc<Engine>,
{
    if let Some(engine) = ENGINE.get() {
        return Ok(engine.clone());
    }
    let engine = ENGINE.get_or_init(|| make_engine(Box::new(WinReplay))).clone();

    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("orbit-mouse-hook".into())
        .spawn(move || unsafe {
            let module = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
            match SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), module, 0) {
                Ok(_) => {
                    let _ = tx.send(Ok(()));
                }
                Err(e) => {
                    let _ = tx.send(Err(StartError::Failed(e.to_string())));
                    return;
                }
            }
            // Low-level hooks are serviced through this thread's message loop.
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .map_err(|e| StartError::Failed(e.to_string()))?;

    rx.recv().map_err(|e| StartError::Failed(e.to_string()))??;
    Ok(engine)
}
