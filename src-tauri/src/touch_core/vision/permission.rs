#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

/// Query screen-recording permission without prompting.
pub fn has_screen_capture_permission() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

/// Ask macOS for screen-recording permission. This may show the system prompt;
/// call it only from an explicit user action.
pub fn request_screen_capture_permission() -> bool {
    unsafe { CGRequestScreenCaptureAccess() }
}
