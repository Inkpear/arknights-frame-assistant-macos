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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_arknights_by_name() {
        assert!(matches_rules(&Some("Arknights".into()), &None));
        assert!(matches_rules(&Some("arknights".into()), &None));
        assert!(matches_rules(&None, &Some("明日方舟".into())));
    }

    #[test]
    fn matches_arknights_by_title() {
        assert!(matches_rules(
            &Some("Terminal".into()),
            &Some("Arknights - Stage".into())
        ));
        assert!(matches_rules(&None, &Some("明日方舟 - 作战".into())));
    }

    #[test]
    fn matches_no_match() {
        assert!(!matches_rules(
            &Some("Safari".into()),
            &Some("Google".into())
        ));
        assert!(!matches_rules(&None, &None));
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
}
