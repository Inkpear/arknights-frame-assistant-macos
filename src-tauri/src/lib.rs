pub mod config;
pub mod ipc;
pub mod startup;
pub mod state;
pub mod touch_core;

use std::sync::Arc;

use tauri::Manager;

use crate::{config::AppConfig, state::AppState};

/// Launch the Tauri application with all services.
pub fn run() {
    env_logger::init();
    let data_dir = app_data_dir();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ipc::commands::set_hotkey_enabled,
            ipc::commands::set_calibrating_mode_enabled,
            ipc::commands::set_calibrating_target,
            ipc::commands::switch_profile,
            ipc::commands::update_custom_keycode,
            ipc::commands::update_ui_ratio,
            ipc::commands::shutdown,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window.hide().ok();
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            let config = AppConfig::load(&data_dir).unwrap_or_default();
            let app_state = Arc::new(AppState::new(config, data_dir.clone(), handle));
            app_state.emit_status();
            app.manage(app_state.clone());

            tauri::async_runtime::spawn(async move {
                if let Err(e) = startup::run(app_state).await {
                    log::error!("Startup failed: {e}");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Failed to start application");
}

fn app_data_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
    format!("{home}/Library/Application Support/arknights-frame-assistant-macos")
}
