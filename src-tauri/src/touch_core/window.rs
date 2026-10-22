use std::sync::LazyLock;

use mado::{WindowBounds, WindowEvent, WindowInfo, WindowListener};
use tauri::{AppHandle, Manager};

use crate::state::AppState;
use crate::touch_core::vision::capture::warm_game_window;

/// Tracks the currently focused window and its game-matching status.
#[derive(Debug, Clone, Default)]
pub struct WindowContext {
    /// Application name
    pub app_name: Option<String>,
    /// Window title
    pub window_title: Option<String>,
    /// Platform-specific window identifier
    pub window_id: Option<u32>,
    /// Window position and size in pixels
    pub window_bounds: Option<WindowBounds>,
    /// Whether the current window matches Arknights
    pub is_arknights: bool,
    /// Last bounds seen for an Arknights window. Kept when focus moves to the
    /// settings window so calibration can still normalize against the game.
    pub last_arknights_bounds: Option<WindowBounds>,
    /// Last window id/title seen for an Arknights window, used to capture the
    /// game window even when the settings window briefly takes focus.
    pub last_arknights_window_id: Option<u32>,
    pub last_arknights_title: Option<String>,
}

/// Filter rules (pre-lowercased) for matching the Arknights window.
static WINDOW_FILTER_RULES: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| vec!["arknights", "明日方舟"]);

/// Check if app_name or window_title matches any (pre-lowercased) filter.
pub fn matches_rules(name: &Option<String>, title: &Option<String>, filters: &[&str]) -> bool {
    let name_lower = name.as_ref().map(|s| s.trim().to_lowercase());
    let title_lower = title.as_ref().map(|s| s.trim().to_lowercase());
    filters.iter().any(|rule| {
        name_lower.as_ref().is_some_and(|n| n.eq(rule))
            || title_lower.as_ref().is_some_and(|t| t.eq(rule))
    })
}

impl WindowContext {
    /// Window is usable when it matches Arknights and has bounds.
    pub fn is_available(&self) -> bool {
        self.is_arknights && self.window_bounds.is_some()
    }

    /// Get the current window bounds.
    pub fn bounds(&self) -> Option<&WindowBounds> {
        self.window_bounds.as_ref()
    }

    /// Bounds to use for calibration: the current Arknights window when it is
    /// focused, otherwise the last known Arknights bounds.
    pub fn calibration_bounds(&self) -> Option<&WindowBounds> {
        self.window_bounds
            .as_ref()
            .filter(|_| self.is_arknights)
            .or(self.last_arknights_bounds.as_ref())
    }

    /// Game content rect, with the window title bar removed. Vision capture
    /// and coordinate mapping use this rect.
    pub fn content_bounds(&self) -> Option<WindowBounds> {
        self.calibration_bounds()
            .map(crate::touch_core::position::content_bounds)
    }

    /// Window id for vision capture: the current game window when focused,
    /// otherwise the last game window seen.
    pub fn vision_window_id(&self) -> Option<u32> {
        self.window_id
            .filter(|_| self.is_arknights)
            .or(self.last_arknights_window_id)
    }

    /// Window title for vision capture fallback.
    pub fn vision_window_title(&self) -> Option<String> {
        if self.is_arknights {
            self.window_title.clone()
        } else {
            self.last_arknights_title.clone()
        }
    }

    /// Refresh all fields from mado::WindowInfo and re-evaluate is_arknights.
    pub fn update_from_window_info(&mut self, info: &WindowInfo) {
        self.app_name = info.app.name.clone();
        self.window_title = info.title.clone();
        self.window_id = info.window_id;
        self.window_bounds = info.bounds.clone();
        self.is_arknights = matches_rules(&self.app_name, &self.window_title, &WINDOW_FILTER_RULES);
        if self.is_arknights {
            self.last_arknights_bounds = info.bounds.clone();
            self.last_arknights_window_id = info.window_id;
            self.last_arknights_title = info.title.clone();
        }
    }

    /// Mark as unavailable when window is destroyed or minimized.
    pub fn mark_unavailable(&mut self) {
        self.window_bounds = None;
        self.window_id = None;
        self.is_arknights = false;
    }
}

/// Bridges mado WindowEvents to AppState for status push.
pub struct ArkWindowListener {
    pub app_handle: AppHandle,
}

impl WindowListener for ArkWindowListener {
    fn on_focus_change(&self, event: WindowEvent) {
        let app_state = self.app_handle.state::<AppState>();
        let warm_target = {
            let mut ctx = app_state.window_ctx.blocking_write();
            let mut focus_changed = false;
            match event {
                WindowEvent::WindowChanged { window } => {
                    focus_changed = true;
                    ctx.update_from_window_info(&window);
                }
                WindowEvent::WindowBoundsChanged { window } => {
                    if let Some(bounds) = window.bounds {
                        ctx.window_bounds = Some(bounds.clone());
                        ctx.window_id = window.window_id;
                        if ctx.is_arknights {
                            ctx.last_arknights_bounds = Some(bounds);
                            ctx.last_arknights_window_id = window.window_id;
                            ctx.last_arknights_title = ctx.window_title.clone();
                        }
                    }
                }
                WindowEvent::WindowMinimized { .. } | WindowEvent::WindowDestroyed { .. } => {
                    ctx.mark_unavailable();
                }
                _ => {}
            }
            // Start the capture stream as soon as the game gains focus, so
            // the first hotkey does not pay stream setup inside its wait.
            // Bounds-only events are skipped: a drag would restart the stream
            // repeatedly, and the next action refreshes it once.
            if focus_changed && ctx.is_arknights {
                ctx.window_id
                    .zip(ctx.bounds().cloned())
                    .map(|(window_id, bounds)| (window_id, ctx.window_title.clone(), bounds))
            } else {
                None
            }
        };
        if let Some((window_id, title, bounds)) = warm_target {
            let vision_enabled = app_state.config.blocking_read().effective_vision().enabled;
            if vision_enabled && app_state.vision_permission_granted() {
                warm_game_window(window_id, title.as_deref(), &bounds);
            }
        }
        app_state.emit_status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_arknights_by_name() {
        assert!(matches_rules(
            &Some("Arknights".into()),
            &None,
            &WINDOW_FILTER_RULES
        ));
        assert!(matches_rules(
            &Some("arknights".into()),
            &None,
            &WINDOW_FILTER_RULES
        ));
        assert!(matches_rules(
            &None,
            &Some("明日方舟".into()),
            &WINDOW_FILTER_RULES
        ));
    }

    #[test]
    fn matches_no_match() {
        assert!(!matches_rules(
            &Some("Safari".into()),
            &Some("Google".into()),
            &WINDOW_FILTER_RULES
        ));
        assert!(!matches_rules(&None, &None, &WINDOW_FILTER_RULES));
    }

    #[test]
    fn window_context_default_unavailable() {
        let ctx = WindowContext::default();
        assert!(!ctx.is_available());
        assert!(ctx.bounds().is_none());
    }

    #[test]
    fn window_context_available_when_matching_with_bounds() {
        let mut ctx = WindowContext::default();
        ctx.is_arknights = true;
        ctx.window_bounds = Some(WindowBounds {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        });
        assert!(ctx.is_available());
        assert!(ctx.bounds().is_some());
    }

    #[test]
    fn mark_unavailable_clears_state() {
        let mut ctx = WindowContext::default();
        ctx.is_arknights = true;
        ctx.window_bounds = Some(WindowBounds {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        });
        ctx.mark_unavailable();
        assert!(!ctx.is_available());
        assert!(ctx.window_bounds.is_none());
        assert!(!ctx.is_arknights);
    }

    #[test]
    fn calibration_bounds_falls_back_to_last_game_window() {
        let game = WindowBounds {
            x: 10.0,
            y: 20.0,
            width: 800.0,
            height: 600.0,
        };
        let mut ctx = WindowContext::default();
        ctx.is_arknights = true;
        ctx.window_bounds = Some(game.clone());
        ctx.last_arknights_bounds = Some(game);

        ctx.is_arknights = false;
        ctx.window_bounds = Some(WindowBounds {
            x: 100.0,
            y: 100.0,
            width: 720.0,
            height: 560.0,
        });

        let bounds = ctx.calibration_bounds().expect("last game bounds");
        assert_eq!(bounds.x, 10.0);
        assert_eq!(bounds.y, 20.0);
        assert_eq!(bounds.width, 800.0);
        assert_eq!(bounds.height, 600.0);
    }

    #[test]
    fn content_bounds_removes_title_bar() {
        let mut ctx = WindowContext::default();
        ctx.is_arknights = true;
        ctx.window_bounds = Some(WindowBounds {
            x: 100.0,
            y: 200.0,
            width: 1283.0,
            height: 758.0,
        });
        let content = ctx.content_bounds().expect("content bounds");
        assert!((content.y - 236.3).abs() < 0.1);
        assert!((content.height - 721.7).abs() < 0.1);
    }
}
