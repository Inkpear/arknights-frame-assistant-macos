use tauri::State;

use crate::{
    config::AppConfigType,
    ipc::protocol::{AppStatusPayload, CustomKeycodePayload},
    state::AppState,
    touch_core::position::UIRationType,
};

/// Get the full current application status (for initial frontend sync).
#[tauri::command]
pub async fn get_status(app_state: State<'_, AppState>) -> Result<AppStatusPayload, String> {
    Ok(app_state.get_status_payload().await)
}

/// Enable or disable hotkey event handling.
#[tauri::command]
pub async fn set_hotkey_enabled(
    app_state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    app_state
        .switch_hotkey_enabled(enabled)
        .await
        .map_err(|e| e.to_string())
}

/// Enable or disable calibrating mode.
#[tauri::command]
pub async fn set_calibrating_mode_enabled(
    app_state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    app_state.switch_calibrating_mode_enabled(enabled).await;
    Ok(())
}

/// Set which UI element is being calibrated.
#[tauri::command]
pub async fn set_calibrating_target(
    app_state: State<'_, AppState>,
    target: UIRationType,
) -> Result<(), String> {
    app_state.set_calibrating_target(target).await;
    Ok(())
}

/// Switch to a different profile (RegularOperations / GarrisonProtocol).
#[tauri::command]
pub async fn switch_profile(
    app_state: State<'_, AppState>,
    new_profile: AppConfigType,
) -> Result<(), String> {
    app_state
        .switch_config(new_profile)
        .await
        .map_err(|e| e.to_string())
}

/// Update custom keycode bindings for the current profile.
#[tauri::command]
pub async fn update_custom_keycode(
    app_state: State<'_, AppState>,
    actions: Vec<CustomKeycodePayload>,
) -> Result<(), String> {
    let tuples: Vec<(String, u16)> = actions
        .into_iter()
        .map(|a| (a.action_id, a.keycode))
        .collect();
    app_state
        .update_custom_keycode(&tuples)
        .await
        .map_err(|e| e.to_string())
}

/// Send shutdown signal to all background services.
#[tauri::command]
pub fn shutdown(app_state: State<'_, AppState>) -> Result<usize, String> {
    app_state.shutdown().map_err(|e| e.to_string())
}

/// Reset a single UI ratio element to its default.
#[tauri::command]
pub async fn reset_ui_ratio(
    app_state: State<'_, AppState>,
    ratio_type: UIRationType,
) -> Result<(), String> {
    app_state
        .reset_ui_ratio(&ratio_type)
        .await
        .map_err(|e| e.to_string())
}

/// Reset all config (keybinds + UI ratios) to defaults.
#[tauri::command]
pub async fn reset_config(app_state: State<'_, AppState>) -> Result<(), String> {
    app_state.reset_config().await.map_err(|e| e.to_string())
}
