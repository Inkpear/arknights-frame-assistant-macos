use std::sync::Arc;

use mado::{WindowBounds, WindowEvent, WindowInfo, WindowListener};

use crate::state::AppState;

/// Tracks the currently focused window and its game-matching status.
#[derive(Debug, Clone, Default)]
pub struct WindowContext {
    /// Application name
    pub app_name: Option<String>,
    /// Window title
    pub window_title: Option<String>,
    /// Window position and size in pixels
    pub window_bounds: Option<WindowBounds>,
    /// Whether the current window matches Arknights
    pub is_arknights: bool,
}

/// Filter rules: app_name or window_title (lowercased) containing any of these → match.
const WINDOW_FILTER_RULES: [&str; 2] = ["arknights", "明日方舟"];

/// Check if app_name or window_title matches any filter rule.
pub fn matches_rules(name: &Option<String>, title: &Option<String>) -> bool {
    let name_lower = name.as_ref().map(|s| s.to_lowercase());
    let title_lower = title.as_ref().map(|s| s.to_lowercase());
    WINDOW_FILTER_RULES.iter().any(|rule| {
        let rule = rule.to_lowercase();
        name_lower.as_ref().is_some_and(|n| n.contains(&rule))
            || title_lower.as_ref().is_some_and(|t| t.contains(&rule))
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

    /// Refresh all fields from mado::WindowInfo and re-evaluate is_arknights.
    pub fn update_from_window_info(&mut self, info: &WindowInfo) {
        self.app_name = info.app.name.clone();
        self.window_title = info.title.clone();
        self.window_bounds = info.bounds.clone();
        self.is_arknights = matches_rules(&self.app_name, &self.window_title);
    }

    /// Mark as unavailable when window is destroyed or minimized.
    pub fn mark_unavailable(&mut self) {
        self.window_bounds = None;
        self.is_arknights = false;
    }
}

/// Bridges mado WindowEvents to AppState for status push.
pub struct ArkWindowListener {
    pub app_state: Arc<AppState>,
}

impl WindowListener for ArkWindowListener {
    fn on_focus_change(&self, event: WindowEvent) {
        let mut ctx = self.app_state.window_ctx.write().unwrap();
        match event {
            WindowEvent::WindowChanged { window } => {
                ctx.update_from_window_info(&window);
            }
            WindowEvent::WindowBoundsChanged { window } => {
                if let Some(bounds) = window.bounds {
                    ctx.window_bounds = Some(bounds);
                }
            }
            WindowEvent::WindowMinimized { .. } | WindowEvent::WindowDestroyed { .. } => {
                ctx.mark_unavailable();
            }
            _ => {}
        }
        drop(ctx);
        self.app_state.emit_status();
    }
}
