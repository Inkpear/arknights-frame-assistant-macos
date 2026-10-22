use mado::WindowBounds;
use serde::{Deserialize, Serialize};

/// Ratios (0.0–1.0) mapping UI element positions relative to the window.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct UIRatio {
    pub left_pause: (f64, f64),
    pub right_pause: (f64, f64),
    pub skill: (f64, f64),
    pub retreat: (f64, f64),
    pub speed: (f64, f64),
}

/// Default UI ratios for a 1920x1080 window.
impl Default for UIRatio {
    fn default() -> Self {
        UIRatio {
            left_pause: (0.92, 0.1),
            right_pause: (0.96, 0.1),
            skill: (0.7, 0.65),
            retreat: (0.47, 0.38),
            speed: (0.85, 0.1),
        }
    }
}

/// Identifies which UI element a ratio value corresponds to.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum UIRatioType {
    LeftPause,
    RightPause,
    Skill,
    Retreat,
    Speed,
}

/// Absolute pixel positions calculated from a `UIRatio` + `WindowBounds`.
pub struct UIPosition {
    pub left_pause: (f64, f64),
    pub right_pause: (f64, f64),
    pub skill: (f64, f64),
    pub retreat: (f64, f64),
    pub speed: (f64, f64),
}

impl UIPosition {
    /// Compute absolute pixel positions from a UIRatio and window bounds.
    pub fn new(ui_ratio: &UIRatio, window: &WindowBounds) -> Self {
        let left_pause = compute_position(ui_ratio.left_pause.0, ui_ratio.left_pause.1, window);
        let right_pause = compute_position(ui_ratio.right_pause.0, ui_ratio.right_pause.1, window);
        let skill = compute_position(ui_ratio.skill.0, ui_ratio.skill.1, window);
        let retreat = compute_position(ui_ratio.retreat.0, ui_ratio.retreat.1, window);
        let speed = compute_position(ui_ratio.speed.0, ui_ratio.speed.1, window);
        UIPosition {
            left_pause,
            right_pause,
            skill,
            retreat,
            speed,
        }
    }
}

/// Map a ratio (0.0–1.0) to absolute pixel coordinates within a window.
/// Formula: `window.origin + ratio * window.size`
pub fn compute_position(ratio_x: f64, ratio_y: f64, window: &WindowBounds) -> (f64, f64) {
    let x = window.x + (ratio_x * window.width);
    let y = window.y + (ratio_y * window.height);
    (x, y)
}

/// Candidate game content aspect ratios (width / height).
const CONTENT_ASPECTS: [f64; 4] = [16.0 / 9.0, 16.0 / 10.0, 4.0 / 3.0, 3.0 / 2.0];

/// Derive the game content rect by removing the window title bar.
///
/// `mado` reports the whole window frame, including the title bar. The game
/// renders at a standard aspect ratio, so the title bar height can be
/// recovered by solving `width / (height - title_bar) = aspect`. The candidate
/// whose implied title bar is plausible and closest to a typical macOS title
/// bar wins.
pub fn content_bounds(frame: &WindowBounds) -> WindowBounds {
    if frame.width <= 0.0 || frame.height <= 0.0 {
        return frame.clone();
    }
    let mut best: Option<(f64, f64)> = None; // (title_bar_height, score)
    for aspect in CONTENT_ASPECTS {
        let title_bar = frame.height - frame.width / aspect;
        if (0.0..=120.0).contains(&title_bar) {
            let score = (title_bar - 32.0).abs();
            if best
                .map(|(_, best_score)| score < best_score)
                .unwrap_or(true)
            {
                best = Some((title_bar, score));
            }
        }
    }
    let title_bar = best.map(|(height, _)| height).unwrap_or(0.0);
    WindowBounds {
        x: frame.x,
        y: frame.y + title_bar,
        width: frame.width,
        height: frame.height - title_bar,
    }
}

/// Check whether a point `(x, y)` falls inside the given window bounds.
pub fn check_in_window(x: f64, y: f64, window: &WindowBounds) -> bool {
    x >= window.x && x <= window.x + window.width && y >= window.y && y <= window.y + window.height
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_window() -> WindowBounds {
        WindowBounds {
            x: 100.0,
            y: 200.0,
            width: 800.0,
            height: 600.0,
        }
    }

    #[test]
    fn compute_position_corner() {
        let w = test_window();
        assert_eq!(compute_position(0.0, 0.0, &w), (100.0, 200.0));
        assert_eq!(compute_position(1.0, 1.0, &w), (900.0, 800.0));
    }

    #[test]
    fn compute_position_center() {
        let w = test_window();
        assert_eq!(compute_position(0.5, 0.5, &w), (500.0, 500.0));
    }

    #[test]
    fn check_in_window_inside() {
        let w = test_window();
        assert!(check_in_window(100.0, 200.0, &w));
        assert!(check_in_window(900.0, 800.0, &w));
        assert!(check_in_window(500.0, 500.0, &w));
    }

    #[test]
    fn check_in_window_outside() {
        let w = test_window();
        assert!(!check_in_window(99.0, 200.0, &w));
        assert!(!check_in_window(100.0, 199.0, &w));
        assert!(!check_in_window(901.0, 800.0, &w));
    }

    #[test]
    fn content_bounds_strips_16_9_title_bar() {
        let frame = WindowBounds {
            x: 100.0,
            y: 200.0,
            width: 1283.0,
            height: 758.0,
        };
        let content = content_bounds(&frame);
        assert!((content.x - 100.0).abs() < 1e-9);
        assert!((content.y - 236.3).abs() < 0.1);
        assert!((content.width - 1283.0).abs() < 1e-9);
        assert!((content.height - 721.7).abs() < 0.1);
        assert!((content.width / content.height - 16.0 / 9.0).abs() < 1e-3);
    }

    #[test]
    fn content_bounds_keeps_fullscreen_16_9() {
        let frame = WindowBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        };
        let content = content_bounds(&frame);
        assert!((content.y - 0.0).abs() < 1e-9);
        assert!((content.height - 1080.0).abs() < 1e-9);
    }

    #[test]
    fn content_bounds_handles_16_10() {
        let frame = WindowBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1230.0,
        };
        let content = content_bounds(&frame);
        assert!((content.y - 30.0).abs() < 1e-9);
        assert!((content.height - 1200.0).abs() < 1e-9);
    }

    #[test]
    fn content_bounds_handles_4_3() {
        let frame = WindowBounds {
            x: 0.0,
            y: 0.0,
            width: 2048.0,
            height: 1570.0,
        };
        let content = content_bounds(&frame);
        assert!((content.y - 34.0).abs() < 1e-9);
        assert!((content.height - 1536.0).abs() < 1e-9);
    }
}
