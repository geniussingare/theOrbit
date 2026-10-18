//! Reads the OS "reduce motion" accessibility preference.

/// True when the user asked the OS to minimise animation.
pub fn prefers_reduced_motion() -> bool {
    #[cfg(target_os = "macos")]
    {
        objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
    }
    #[cfg(target_os = "windows")]
    {
        use windows::core::BOOL;
        use windows::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        };
        let mut enabled = BOOL(1);
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                Some(&mut enabled as *mut BOOL as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };
        ok.is_ok() && !enabled.as_bool()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // GNOME and most GTK desktops expose this setting.
        std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "enable-animations"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "false")
            .unwrap_or(false)
    }
}
