use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::AppConfigType;
use crate::touch_core::position::UIRatio;
use mado::WindowBounds;

/// Snapshot of the tracked window state.
#[derive(Serialize, Clone)]
pub struct WindowInfoPayload {
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub bounds: Option<WindowBounds>,
    pub is_arknights: bool,
    pub is_available: bool,
}

/// Full application status pushed to the frontend on every change.
#[derive(Serialize, Clone)]
pub struct AppStatusPayload {
    pub hotkey_enabled: bool,
    pub calibrating_mode_enabled: bool,
    pub current_profile: AppConfigType,
    pub hotkey_active: bool,
    pub language: String,
    pub regular_operations_keycode: Option<HashMap<String, u16>>,
    pub garrison_protocol_keycode: Option<HashMap<String, u16>>,
    pub ui_ratio: Option<UIRatio>,
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
