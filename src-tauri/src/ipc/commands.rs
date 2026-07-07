use std::sync::Arc;

use tauri::State;

use crate::{
    config::AppConfigType,
    ipc::protocol::{CustomKeycodePayload, UIRatioPayload},
    state::AppState,
    touch_core::position::UIRationType,
};

/// Enable or disable hotkey event handling.
#[tauri::command]
pub fn set_hotkey_enabled(
    app_state: State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<(), String> {
    app_state
        .switch_hotkey_enabled(enabled)
        .map_err(|e| e.to_string())
}

/// Enable or disable calibrating mode.
#[tauri::command]
pub fn set_calibrating_mode_enabled(app_state: State<'_, Arc<AppState>>, enabled: bool) {
    app_state.switch_calibrating_mode_enabled(enabled);
}

/// Set which UI element is being calibrated.
#[tauri::command]
pub fn set_calibrating_target(app_state: State<'_, Arc<AppState>>, target: UIRationType) {
    app_state.set_calibrating_target(target);
}

/// Switch to a different profile (RegularOperations / GarrisonProtocol).
#[tauri::command]
pub fn switch_profile(
    app_state: State<'_, Arc<AppState>>,
    new_profile: AppConfigType,
) -> Result<(), String> {
    app_state
        .switch_config(new_profile)
        .map_err(|e| e.to_string())
}

/// Update custom keycode bindings for the current profile.
#[tauri::command]
pub fn update_custom_keycode(
    app_state: State<'_, Arc<AppState>>,
    actions: Vec<CustomKeycodePayload>,
) -> Result<(), String> {
    let tuples: Vec<(String, u16)> = actions
        .into_iter()
        .map(|a| (a.action_id, a.keycode))
        .collect();
    app_state
        .update_custom_keycode(&tuples)
        .map_err(|e| e.to_string())
}

/// Update UI ratio values in the config.
#[tauri::command]
pub fn update_ui_ratio(
    app_state: State<'_, Arc<AppState>>,
    ratios: Vec<UIRatioPayload>,
) -> Result<(), String> {
    app_state
        .update_ui_ratio(&ratios)
        .map_err(|e| e.to_string())
}

/// Send shutdown signal to all background services.
#[tauri::command]
pub fn shutdown(app_state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    app_state.shutdown().map_err(|e| e.to_string())
}
