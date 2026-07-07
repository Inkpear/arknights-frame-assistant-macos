use serde::{Deserialize, Serialize};

use crate::config::AppConfigType;
use mado::WindowBounds;

/// Snapshot of the tracked window state.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfoPayload {
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub bounds: Option<WindowBounds>,
    pub is_arknights: bool,
    pub is_available: bool,
}

/// Full application status pushed to the frontend on every change.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppStatusPayload {
    pub hotkey_enabled: bool,
    pub calibrating_mode_enabled: bool,
    pub current_profile: AppConfigType,
    pub hotkey_active: bool,
    pub window: WindowInfoPayload,
}

/// A single UI ratio element update from the frontend.
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UIRatioPayload {
    pub ratio_type: crate::touch_core::position::UIRationType,
    pub ratio: (f64, f64),
}

/// A custom keycode binding for one action.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomKeycodePayload {
    pub action_id: String,
    pub keycode: u16,
}
