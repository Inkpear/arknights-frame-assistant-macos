//! Window-only screen capture through ScreenCaptureKit.
//!
//! The old `CGWindowListCreateImage` / `CGDisplayCreateImage` APIs are
//! obsoleted on macOS 15+, so this module uses ScreenCaptureKit.
//!
//! A dedicated capture thread owns a live `SCStream` for the game window.
//! `SCShareableContent::get()` (window enumeration) and stream startup are
//! paid once; each quick action only waits for the next frame (~1/60s) and
//! copies it. The stream is stopped after 60s without requests.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Condvar, LazyLock, Mutex, Once};
use std::thread;
use std::time::{Duration, Instant};

use image::RgbaImage;
use mado::WindowBounds;
use screencapturekit::cm::CMSampleBufferExt;
use screencapturekit::cv::CVPixelBufferLockFlags;
use screencapturekit::prelude::*;
use screencapturekit::shareable_content::{SCShareableContent, SCShareableContentInfo};

use super::permission;

unsafe extern "C" {
    fn sc_initialize_core_graphics();
}

static INIT_CG: Once = Once::new();

fn init_core_graphics() {
    INIT_CG.call_once(|| {
        // Window filters can crash with CGS_REQUIRE_INIT if CoreGraphics has
        // not been initialized in the process yet.
        unsafe { sc_initialize_core_graphics() };
    });
}

#[derive(Debug)]
pub enum CaptureError {
    PermissionDenied,
    WindowNotFound,
    CaptureFailed(String),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "screen recording permission denied"),
            Self::WindowNotFound => write!(f, "game window not found"),
            Self::CaptureFailed(message) => write!(f, "capture failed: {message}"),
        }
    }
}

impl std::error::Error for CaptureError {}

pub struct CapturedFrame {
    /// Tightly packed BGRA rows, full capture height (title bar included).
    pub bgra: Vec<u8>,
    pub width: usize,
    pub height: usize,
    /// Game content rect in screen points. The image origin maps here.
    pub content: WindowBounds,
    /// Image pixels per screen point.
    pub scale_x: f64,
    pub scale_y: f64,
    pub capture_ms: u128,
    /// Time the request waited in the capture thread's queue (before the
    /// stream/frame work started).
    pub queue_ms: u64,
    /// Diagnostics for the debug JSON.
    pub window_frame: WindowBounds,
    pub crop_top: usize,
}

struct CaptureRequest {
    window_id: u32,
    title: Option<String>,
    window_frame: WindowBounds,
    content_bounds: WindowBounds,
    sent_at: Instant,
    reply: Sender<Result<CapturedFrame, CaptureError>>,
}

enum CaptureCommand {
    Capture(CaptureRequest),
    /// Start/refresh the stream ahead of the actual capture so setup overlaps
    /// the action's animation wait.
    Warm(WarmRequest),
}

struct WarmRequest {
    window_id: u32,
    title: Option<String>,
    window_frame: WindowBounds,
}

struct RawFrame {
    width: usize,
    height: usize,
    /// Tightly packed BGRA rows.
    data: Vec<u8>,
}

struct StreamShared {
    pending: AtomicBool,
    frame: Mutex<Option<RawFrame>>,
    ready: Condvar,
}

struct FrameHandler {
    shared: Arc<StreamShared>,
}

impl SCStreamOutputTrait for FrameHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, output_type: SCStreamOutputType) {
        if !matches!(output_type, SCStreamOutputType::Screen) {
            return;
        }
        if !self.shared.pending.swap(false, Ordering::AcqRel) {
            return;
        }
        let ok = self.capture_frame(&sample);
        if !ok {
            // No usable frame in this callback; let the next one satisfy the
            // pending request instead of timing out.
            self.shared.pending.store(true, Ordering::Release);
        }
    }
}

impl FrameHandler {
    fn capture_frame(&self, sample: &CMSampleBuffer) -> bool {
        let Some(pixel_buffer) = sample.pixel_buffer() else {
            return false;
        };
        let Ok(guard) = pixel_buffer.lock(CVPixelBufferLockFlags::READ_ONLY) else {
            return false;
        };
        let width = guard.width();
        let height = guard.height();
        let stride = guard.bytes_per_row();
        if width == 0 || height == 0 || stride < width * 4 {
            return false;
        }
        let Some(slice) = (unsafe { guard.as_slice() }) else {
            return false;
        };
        let mut data = vec![0u8; width * height * 4];
        for y in 0..height {
            let src = &slice[y * stride..y * stride + width * 4];
            data[y * width * 4..(y + 1) * width * 4].copy_from_slice(src);
        }
        let Ok(mut frame) = self.shared.frame.lock() else {
            return false;
        };
        *frame = Some(RawFrame {
            width,
            height,
            data,
        });
        self.shared.ready.notify_all();
        true
    }
}

struct StreamState {
    stream: SCStream,
    shared: Arc<StreamShared>,
}

impl Drop for StreamState {
    fn drop(&mut self) {
        let _ = self.stream.stop_capture();
    }
}

/// Cached live ScreenCaptureKit objects. This struct is only touched from the
/// dedicated capture thread, so the non-Send SCK pointers never cross threads.
struct CachedWindow {
    window_id: u32,
    frame: WindowBounds,
    _window: SCWindow,
    filter: SCContentFilter,
    capture_rect: CGRect,
    stream: Option<StreamState>,
}

pub struct CaptureService {
    tx: Sender<CaptureCommand>,
}

static CAPTURE_SERVICE: LazyLock<CaptureService> = LazyLock::new(CaptureService::start);

impl CaptureService {
    fn start() -> Self {
        let (tx, rx) = channel();
        let spawned = thread::Builder::new()
            .name("afa-vision-capture".to_string())
            .spawn(move || capture_loop(rx));
        if let Err(error) = spawned {
            log::error!("Failed to start vision capture thread: {error}");
        }
        Self { tx }
    }

    fn capture(
        &self,
        window_id: u32,
        title: Option<&str>,
        window_frame: &WindowBounds,
        content_bounds: &WindowBounds,
    ) -> Result<CapturedFrame, CaptureError> {
        if !permission::has_screen_capture_permission() {
            return Err(CaptureError::PermissionDenied);
        }
        let (reply, receiver) = channel();
        self.tx
            .send(CaptureCommand::Capture(CaptureRequest {
                window_id,
                title: title.map(str::to_string),
                window_frame: window_frame.clone(),
                content_bounds: content_bounds.clone(),
                sent_at: Instant::now(),
                reply,
            }))
            .map_err(|_| CaptureError::CaptureFailed("capture thread stopped".to_string()))?;
        receiver
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| CaptureError::CaptureFailed("capture timed out".to_string()))?
    }

    fn warm(&self, window_id: u32, title: Option<&str>, window_frame: &WindowBounds) {
        let _ = self.tx.send(CaptureCommand::Warm(WarmRequest {
            window_id,
            title: title.map(str::to_string),
            window_frame: window_frame.clone(),
        }));
    }
}

/// Capture the game window identified by `window_id`, then crop the title bar
/// so the returned image is exactly the game content area.
pub fn capture_game_window(
    window_id: u32,
    title: Option<&str>,
    window_frame: &WindowBounds,
    content_bounds: &WindowBounds,
) -> Result<CapturedFrame, CaptureError> {
    CAPTURE_SERVICE.capture(window_id, title, window_frame, content_bounds)
}

/// Start/refresh the game-window stream without waiting for a frame. Called
/// when a quick action begins, so stream setup overlaps the animation wait.
pub fn warm_game_window(window_id: u32, title: Option<&str>, window_frame: &WindowBounds) {
    CAPTURE_SERVICE.warm(window_id, title, window_frame);
}

fn capture_loop(commands: Receiver<CaptureCommand>) {
    crate::touch_core::qos::promote_current_thread();
    init_core_graphics();
    let mut cached: Option<CachedWindow> = None;
    loop {
        match commands.recv_timeout(Duration::from_secs(60)) {
            Ok(CaptureCommand::Capture(request)) => {
                let result = capture_request(&request, &mut cached);
                let result = match result {
                    Err(CaptureError::CaptureFailed(_)) => {
                        // A stale window/filter/stream is the common cause;
                        // refresh everything and try once more.
                        cached = None;
                        capture_request(&request, &mut cached)
                    }
                    other => other,
                };
                let _ = request.reply.send(result);
            }
            Ok(CaptureCommand::Warm(request)) => {
                warm_request(&request, &mut cached);
            }
            Err(RecvTimeoutError::Timeout) => {
                // Stop the stream after a minute of inactivity. The filter
                // and window stay cached, so the next request restarts the
                // stream without re-enumerating windows.
                if let Some(cache) = cached.as_mut() {
                    cache.stream = None;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn warm_request(request: &WarmRequest, cached: &mut Option<CachedWindow>) {
    let started = Instant::now();
    let needs_refresh = cached.as_ref().is_none_or(|cache| {
        cache.window_id != request.window_id || !same_frame(&cache.frame, &request.window_frame)
    });
    let mut refreshed = false;
    if needs_refresh {
        match refresh_window(
            request.window_id,
            request.title.as_deref(),
            &request.window_frame,
        ) {
            Ok(cache) => {
                *cached = Some(cache);
                refreshed = true;
            }
            Err(error) => {
                log::warn!("Vision warm-up failed: {error}");
                return;
            }
        }
    }
    let Some(cache) = cached.as_mut() else {
        return;
    };
    let mut stream_started = false;
    if cache.stream.is_none() {
        match start_stream(&cache.filter, &cache.capture_rect) {
            Ok(stream) => {
                cache.stream = Some(stream);
                stream_started = true;
            }
            Err(error) => log::warn!("Vision warm-up stream failed: {error}"),
        }
    }
    let elapsed_ms = started.elapsed().as_millis();
    if refreshed || stream_started || elapsed_ms >= 5 {
        log::info!(
            "Vision warm-up: refresh={refreshed}, stream_start={stream_started}, {elapsed_ms}ms"
        );
    }
}

fn capture_request(
    request: &CaptureRequest,
    cached: &mut Option<CachedWindow>,
) -> Result<CapturedFrame, CaptureError> {
    let started = Instant::now();
    let queue_ms = started
        .saturating_duration_since(request.sent_at)
        .as_millis() as u64;
    let needs_refresh = cached.as_ref().is_none_or(|cache| {
        cache.window_id != request.window_id || !same_frame(&cache.frame, &request.window_frame)
    });
    if needs_refresh {
        *cached = Some(refresh_window(
            request.window_id,
            request.title.as_deref(),
            &request.window_frame,
        )?);
    }
    let cache = cached.as_mut().ok_or(CaptureError::WindowNotFound)?;
    if cache.stream.is_none() {
        cache.stream = Some(start_stream(&cache.filter, &cache.capture_rect)?);
    }
    let shared = cache
        .stream
        .as_ref()
        .map(|stream| stream.shared.clone())
        .ok_or(CaptureError::WindowNotFound)?;
    let raw = wait_for_frame(&shared, Duration::from_millis(300))?;
    build_frame(
        raw,
        &request.content_bounds,
        &request.window_frame,
        &cache.capture_rect,
        started,
        queue_ms,
    )
}

fn refresh_window(
    window_id: u32,
    title: Option<&str>,
    window_frame: &WindowBounds,
) -> Result<CachedWindow, CaptureError> {
    let shareable = SCShareableContent::get()
        .map_err(|e| CaptureError::CaptureFailed(format!("SCShareableContent: {e}")))?;
    let windows = shareable.windows();
    let window = windows
        .iter()
        .find(|window| window.window_id() == window_id)
        .or_else(|| {
            title.and_then(|title| {
                windows
                    .iter()
                    .find(|window| window.title().as_deref() == Some(title))
            })
        });
    let Some(window) = window else {
        return Err(CaptureError::WindowNotFound);
    };
    let filter = SCContentFilter::create()
        .with_window(window)
        .build()
        .map_err(|e| CaptureError::CaptureFailed(format!("content filter: {e}")))?;
    let info = SCShareableContentInfo::for_filter(&filter);
    let capture_rect = info
        .as_ref()
        .map(|info| info.content_rect())
        .unwrap_or_else(|| window.frame());
    Ok(CachedWindow {
        window_id,
        frame: window_frame.clone(),
        _window: window.clone(),
        filter,
        capture_rect,
        stream: None,
    })
}

fn start_stream(
    filter: &SCContentFilter,
    capture_rect: &CGRect,
) -> Result<StreamState, CaptureError> {
    let width = capture_rect.size.width.round().max(1.0) as u32;
    let height = capture_rect.size.height.round().max(1.0) as u32;
    let config = SCStreamConfiguration::new()
        .with_width(width)
        .with_height(height)
        .with_pixel_format(PixelFormat::BGRA)
        .with_fps(60)
        .with_queue_depth(1)
        .with_shows_cursor(false);
    let shared = Arc::new(StreamShared {
        pending: AtomicBool::new(false),
        frame: Mutex::new(None),
        ready: Condvar::new(),
    });
    let mut stream = SCStream::new(filter, &config)
        .map_err(|e| CaptureError::CaptureFailed(format!("stream: {e}")))?;
    stream
        .add_output_handler(
            FrameHandler {
                shared: shared.clone(),
            },
            SCStreamOutputType::Screen,
        )
        .map_err(|e| CaptureError::CaptureFailed(format!("stream handler: {e}")))?;
    stream
        .start_capture()
        .map_err(|e| CaptureError::CaptureFailed(format!("start stream: {e}")))?;
    Ok(StreamState { stream, shared })
}

fn wait_for_frame(shared: &StreamShared, timeout: Duration) -> Result<RawFrame, CaptureError> {
    {
        let mut frame = shared
            .frame
            .lock()
            .map_err(|_| CaptureError::CaptureFailed("frame mutex poisoned".to_string()))?;
        *frame = None;
    }
    shared.pending.store(true, Ordering::Release);

    let deadline = Instant::now() + timeout;
    let mut guard = shared
        .frame
        .lock()
        .map_err(|_| CaptureError::CaptureFailed("frame mutex poisoned".to_string()))?;
    loop {
        if let Some(frame) = guard.take() {
            return Ok(frame);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(CaptureError::CaptureFailed(
                "stream frame timeout".to_string(),
            ));
        }
        let (next, _) = shared
            .ready
            .wait_timeout(guard, remaining)
            .map_err(|_| CaptureError::CaptureFailed("frame condvar poisoned".to_string()))?;
        guard = next;
    }
}

fn build_frame(
    raw: RawFrame,
    content_bounds: &WindowBounds,
    window_frame: &WindowBounds,
    capture_rect: &CGRect,
    started: Instant,
    queue_ms: u64,
) -> Result<CapturedFrame, CaptureError> {
    let capture_scale_y = raw.height as f64 / capture_rect.size.height.max(1.0);
    let expected_content_h = (content_bounds.height * capture_scale_y).round() as usize;
    let crop_top = raw
        .height
        .saturating_sub(expected_content_h)
        .min(raw.height.saturating_sub(1));
    let content_h = raw.height - crop_top;

    let scale_x = raw.width as f64 / content_bounds.width.max(1.0);
    let scale_y = content_h as f64 / content_bounds.height.max(1.0);
    if !scale_x.is_finite() || !scale_y.is_finite() || scale_x <= 0.0 || scale_y <= 0.0 {
        return Err(CaptureError::CaptureFailed(
            "invalid capture scale".to_string(),
        ));
    }

    Ok(CapturedFrame {
        bgra: raw.data,
        width: raw.width,
        height: raw.height,
        content: content_bounds.clone(),
        scale_x,
        scale_y,
        capture_ms: started.elapsed().as_millis(),
        queue_ms,
        window_frame: window_frame.clone(),
        crop_top,
    })
}

/// Convert a tightly packed BGRA buffer to RGBA. Only used for debug PNGs.
pub(crate) fn bgra_to_rgba(data: &[u8], width: usize, height: usize) -> Option<RgbaImage> {
    if width == 0 || height == 0 || data.len() < width * height * 4 {
        return None;
    }
    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in data.chunks_exact(width * 4).take(height) {
        for px in row.chunks_exact(4) {
            rgba.push(px[2]);
            rgba.push(px[1]);
            rgba.push(px[0]);
            rgba.push(255);
        }
    }
    RgbaImage::from_raw(width as u32, height as u32, rgba)
}

fn same_frame(a: &WindowBounds, b: &WindowBounds) -> bool {
    (a.x - b.x).abs() < 1.0
        && (a.y - b.y).abs() < 1.0
        && (a.width - b.width).abs() < 1.0
        && (a.height - b.height).abs() < 1.0
}
