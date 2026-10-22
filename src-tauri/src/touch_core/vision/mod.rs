pub mod capture;
pub mod detector;
pub mod permission;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use image::RgbaImage;
use mado::WindowBounds;
use serde::{Deserialize, Serialize};
use serde_json::json;

use self::capture::{CaptureError, CapturedFrame, bgra_to_rgba, capture_game_window};
use self::detector::{
    Detection, Outcome, Params, Rect, build_work_bgra, detect, draw_circle, draw_line,
    work_to_image,
};

/// Which in-battle button to resolve.
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonKind {
    Skill,
    Retreat,
}

/// User-facing vision settings. Disabled by default until the user grants
/// screen-recording permission and tests the detector on their setup.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct VisionConfig {
    pub enabled: bool,
    pub white_threshold: u8,
    /// Extra wait after the action's WaitAnimation, before capturing.
    pub extra_delay_ms: u64,
    /// Save the captured frame + overlay + metadata for tuning.
    pub debug_capture: bool,
    /// Detect and log, but do not click the skill/retreat button.
    pub dry_run: bool,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            white_threshold: 240,
            extra_delay_ms: 0,
            debug_capture: false,
            dry_run: false,
        }
    }
}

impl VisionConfig {
    pub fn params(&self) -> Params {
        Params {
            white_threshold: self.white_threshold,
            ..Default::default()
        }
    }
}

/// Last vision result, mirrored into the status payload. Written from the
/// hotkey action path, read by the status builder.
#[derive(Debug, Clone, Default)]
pub struct VisionRuntimeStatus {
    pub last_ok: Option<bool>,
    /// `"vision"` or `"dry_run"`.
    pub last_mode: Option<String>,
    /// Stable reason code for i18n: permission_denied, window_not_found,
    /// capture_failed, no_diamond, button_out_of_bounds.
    pub last_reason: Option<String>,
    pub last_latency_ms: Option<u64>,
    pub last_capture_ms: Option<u64>,
    pub last_confidence: Option<f64>,
    pub last_debug_path: Option<String>,
}

#[derive(Debug, Clone)]
pub enum VisionDecision {
    Click((f64, f64)),
    /// No click: dry-run, no diamond, or the runtime is unavailable. The
    /// compatibility mode never falls back to the UI ratios.
    Skip,
}

pub struct VisionRuntime {
    pub window_id: u32,
    pub window_title: Option<String>,
    pub window_frame: WindowBounds,
    pub content_bounds: WindowBounds,
    pub config: VisionConfig,
    pub debug_dir: PathBuf,
    pub status: Arc<Mutex<VisionRuntimeStatus>>,
}

impl VisionRuntime {
    pub fn new(
        window_id: u32,
        window_title: Option<String>,
        window_frame: WindowBounds,
        content_bounds: WindowBounds,
        config: VisionConfig,
        debug_dir: PathBuf,
        status: Arc<Mutex<VisionRuntimeStatus>>,
    ) -> Self {
        Self {
            window_id,
            window_title,
            window_frame,
            content_bounds,
            config,
            debug_dir,
            status,
        }
    }

    /// Start/refresh the capture stream immediately. The action's animation
    /// wait then overlaps the one-time stream setup.
    pub fn warm(&self) {
        capture::warm_game_window(
            self.window_id,
            self.window_title.as_deref(),
            &self.window_frame,
        );
    }

    /// Capture the game window, detect the diamond, and return the requested
    /// button position. This is called from the synchronous action execution
    /// path after `WaitAnimation`.
    pub fn resolve(&self, kind: ButtonKind) -> VisionDecision {
        let started = Instant::now();
        std::thread::sleep(Duration::from_millis(self.config.extra_delay_ms));
        let params = self.config.params();
        let mode = if self.config.dry_run {
            "dry_run"
        } else {
            "vision"
        };

        let captured = match capture_game_window(
            self.window_id,
            self.window_title.as_deref(),
            &self.window_frame,
            &self.content_bounds,
        ) {
            Ok(captured) => captured,
            Err(error) => {
                let reason = match error {
                    CaptureError::PermissionDenied => "permission_denied",
                    CaptureError::WindowNotFound => "window_not_found",
                    CaptureError::CaptureFailed(_) => "capture_failed",
                };
                log::warn!("Vision capture failed ({reason}): {error}");
                self.set_status(VisionStatusUpdate {
                    ok: false,
                    mode: Some(mode.to_string()),
                    reason: Some(reason.to_string()),
                    latency_ms: Some(started.elapsed().as_millis() as u64),
                    capture_ms: None,
                    confidence: None,
                    debug_path: None,
                });
                return VisionDecision::Skip;
            }
        };

        let content_h = captured.height - captured.crop_top;
        let content_bgra = &captured.bgra[captured.crop_top * captured.width * 4..];
        let roi = Rect::new(0, 0, captured.width, content_h);
        let build_started = Instant::now();
        let work = build_work_bgra(content_bgra, captured.width, content_h, roi, &params);
        let build_ms = build_started.elapsed().as_millis() as u64;
        let detect_started = Instant::now();
        let outcome = detect(&work, &params);
        let detect_ms = detect_started.elapsed().as_millis() as u64;
        let Some(detection) = outcome.detection.as_ref() else {
            let latency = started.elapsed().as_millis() as u64;
            let debug_path = if self.config.debug_capture {
                self.save_debug(
                    kind,
                    &captured,
                    &work,
                    &outcome,
                    (0.0, 0.0),
                    Some("no_diamond"),
                )
            } else {
                None
            };
            self.set_status(VisionStatusUpdate {
                ok: false,
                mode: Some(mode.to_string()),
                reason: Some("no_diamond".to_string()),
                latency_ms: Some(latency),
                capture_ms: Some(captured.capture_ms as u64),
                confidence: None,
                debug_path,
            });
            log::info!(
                "Vision {kind:?} found no diamond: {} ({} candidates); queue {}ms, build {}ms, detect {}ms; no click",
                outcome.reason.as_deref().unwrap_or("no diamond"),
                outcome.candidates.len(),
                captured.queue_ms,
                build_ms,
                detect_ms
            );
            return VisionDecision::Skip;
        };

        let work_point = match kind {
            ButtonKind::Skill => detection.skill,
            ButtonKind::Retreat => detection.retreat,
        };
        let image_point = work_to_image(work_point, roi, work.block);
        let screen = (
            captured.content.x + f64::from(image_point.x) / captured.scale_x,
            captured.content.y + f64::from(image_point.y) / captured.scale_y,
        );
        if !in_bounds(screen, &captured.content) {
            let latency = started.elapsed().as_millis() as u64;
            let debug_path = if self.config.debug_capture {
                self.save_debug(
                    kind,
                    &captured,
                    &work,
                    &outcome,
                    screen,
                    Some("button_out_of_bounds"),
                )
            } else {
                None
            };
            self.set_status(VisionStatusUpdate {
                ok: false,
                mode: Some(mode.to_string()),
                reason: Some("button_out_of_bounds".to_string()),
                latency_ms: Some(latency),
                capture_ms: Some(captured.capture_ms as u64),
                confidence: Some(f64::from(detection.confidence)),
                debug_path,
            });
            log::warn!("Vision button is outside the content rect: {screen:?}; no click");
            return VisionDecision::Skip;
        }

        let latency = started.elapsed().as_millis() as u64;
        let debug_path = if self.config.debug_capture {
            self.save_debug(kind, &captured, &work, &outcome, screen, None)
        } else {
            None
        };
        self.set_status(VisionStatusUpdate {
            ok: true,
            mode: Some(mode.to_string()),
            reason: None,
            latency_ms: Some(latency),
            capture_ms: Some(captured.capture_ms as u64),
            confidence: Some(f64::from(detection.confidence)),
            debug_path,
        });
        log::info!(
            "Vision {kind:?}: captured {}x{} in {}ms (queue {}ms), build {}ms, detect {}ms, confidence {:.2}, point ({:.1}, {:.1}){}",
            captured.width,
            content_h,
            captured.capture_ms,
            captured.queue_ms,
            build_ms,
            detect_ms,
            detection.confidence,
            screen.0,
            screen.1,
            if self.config.dry_run {
                " [dry-run: click skipped]"
            } else {
                ""
            }
        );
        if self.config.dry_run {
            VisionDecision::Skip
        } else {
            VisionDecision::Click(screen)
        }
    }

    fn set_status(&self, update: VisionStatusUpdate) {
        if let Ok(mut status) = self.status.lock() {
            status.last_ok = Some(update.ok);
            status.last_mode = update.mode;
            status.last_reason = update.reason;
            status.last_latency_ms = update.latency_ms;
            status.last_capture_ms = update.capture_ms;
            status.last_confidence = update.confidence;
            status.last_debug_path = update.debug_path;
        }
    }

    fn save_debug(
        &self,
        kind: ButtonKind,
        captured: &CapturedFrame,
        work: &detector::WorkImage,
        outcome: &Outcome,
        clicked: (f64, f64),
        failure: Option<&str>,
    ) -> Option<String> {
        let dir = debug_case_dir(&self.debug_dir, kind);
        if let Err(error) = std::fs::create_dir_all(&dir) {
            log::warn!(
                "Failed to create vision debug dir {}: {error}",
                dir.display()
            );
            return None;
        }
        let content_h = captured.height - captured.crop_top;
        let content_bgra = &captured.bgra[captured.crop_top * captured.width * 4..];
        let Some(frame) = bgra_to_rgba(content_bgra, captured.width, content_h) else {
            log::warn!("Failed to convert vision frame for debug");
            return None;
        };
        let roi = Rect::new(0, 0, captured.width, content_h);
        let mut overlay = frame.clone();
        for line in outcome.candidates.iter().take(20) {
            let a = work_to_image(line.p0, roi, work.block);
            let b = work_to_image(line.p1, roi, work.block);
            draw_line(&mut overlay, a, b, image::Rgba([0, 170, 100, 255]), 2);
        }
        if let Some(detection) = &outcome.detection {
            draw_detection(&mut overlay, detection, roi, work.block);
        }
        if let Err(error) = frame.save(dir.join("frame.png")) {
            log::warn!("Failed to save vision frame: {error}");
        }
        if let Err(error) = overlay.save(dir.join("overlay.png")) {
            log::warn!("Failed to save vision overlay: {error}");
        }
        let meta = json!({
            "kind": format!("{kind:?}"),
            "failure": failure,
            "clicked": [clicked.0, clicked.1],
            "window_frame": &captured.window_frame,
            "content": &captured.content,
            "capture_scale": [captured.scale_x, captured.scale_y],
            "capture_crop_top": captured.crop_top,
            "capture_ms": captured.capture_ms,
            "capture_queue_ms": captured.queue_ms,
            "image": { "width": captured.width, "height": content_h },
            "block": work.block,
            "work": { "width": work.w, "height": work.h },
            "threshold": self.config.white_threshold,
            "detection": outcome.detection.as_ref(),
            "candidates": &outcome.candidates,
            "reason": &outcome.reason,
        });
        if let Err(error) = std::fs::write(
            dir.join("meta.json"),
            serde_json::to_string_pretty(&meta).unwrap_or_default(),
        ) {
            log::warn!("Failed to save vision meta: {error}");
        }
        prune_debug_dir(&self.debug_dir, 20);
        Some(dir.to_string_lossy().to_string())
    }
}

struct VisionStatusUpdate {
    ok: bool,
    mode: Option<String>,
    reason: Option<String>,
    latency_ms: Option<u64>,
    capture_ms: Option<u64>,
    confidence: Option<f64>,
    debug_path: Option<String>,
}

fn in_bounds(point: (f64, f64), bounds: &WindowBounds) -> bool {
    point.0 >= bounds.x
        && point.1 >= bounds.y
        && point.0 <= bounds.x + bounds.width
        && point.1 <= bounds.y + bounds.height
}

fn draw_detection(img: &mut RgbaImage, detection: &Detection, roi: Rect, block: usize) {
    use image::Rgba;
    for line in [&detection.line_a, &detection.line_b] {
        let a = work_to_image(line.p0, roi, block);
        let b = work_to_image(line.p1, roi, block);
        draw_line(img, a, b, Rgba([0, 255, 80, 255]), 4);
    }
    for vertex in [
        detection.top,
        detection.right,
        detection.bottom,
        detection.left,
    ] {
        draw_circle(
            img,
            work_to_image(vertex, roi, block),
            9,
            Rgba([60, 160, 255, 255]),
        );
    }
    draw_circle(
        img,
        work_to_image(detection.retreat, roi, block),
        14,
        Rgba([255, 80, 80, 255]),
    );
    draw_circle(
        img,
        work_to_image(detection.skill, roi, block),
        14,
        Rgba([255, 210, 0, 255]),
    );
}

fn debug_case_dir(root: &Path, kind: ButtonKind) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    root.join(format!("{stamp}-{kind:?}"))
}

fn prune_debug_dir(root: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut dirs: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .collect();
    if dirs.len() <= keep {
        return;
    }
    dirs.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|meta| meta.modified())
            .unwrap_or(UNIX_EPOCH)
    });
    let remove_count = dirs.len().saturating_sub(keep);
    for entry in dirs.into_iter().take(remove_count) {
        let _ = std::fs::remove_dir_all(entry.path());
    }
}
