use mado::{MonitorConfig, WindowBounds, WindowMonitor};
use tauri::path::BaseDirectory;
use tauri::{Manager, State};

use crate::config::AppConfig;
use crate::desktop::TrayMenuItemHandles;
use crate::ipc;
use crate::ipc::protocol::UIRatioPayload;
use crate::touch_core::action::ActionContext;
use crate::touch_core::position;
use crate::touch_core::position::UIRatioType;
use crate::touch_core::vision::VisionRuntime;
use crate::{state::AppState, touch_core::window::ArkWindowListener};

use cgevents::async_api::{CGEventItem, CGEventTapStream};
use cgevents::cg_event_type::CGEventType;
use cgevents::{Keycode, TapLocation};

/// Start application
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ipc::commands::get_status,
            ipc::commands::set_hotkey_enabled,
            ipc::commands::set_calibrating_mode_enabled,
            ipc::commands::set_calibrating_target,
            ipc::commands::switch_profile,
            ipc::commands::update_custom_keycode,
            ipc::commands::reset_ui_ratio,
            ipc::commands::reset_config,
            ipc::commands::set_vision_enabled,
            ipc::commands::set_vision_dry_run,
            ipc::commands::set_vision_debug,
            ipc::commands::request_screen_capture_permission,
            ipc::commands::shutdown,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window.hide().ok();
            }
        })
        .setup(move |app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();

            let config_dir = app
                .path()
                .resolve("", BaseDirectory::AppConfig)
                .unwrap_or_else(|_| std::env::temp_dir());
            let config = AppConfig::load(config_dir).unwrap_or_default();
            let app_state = AppState::new(config, handle.clone());
            let tray_menu_handles =
                TrayMenuItemHandles::new(app).expect("Failed to build tray menu");

            app.manage(app_state);
            app.manage(tray_menu_handles);

            tauri::async_runtime::block_on(async {
                app.handle().state::<AppState>().emit_status_async().await;
            });

            let listener_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = start_event_listener(listener_handle).await {
                    log::error!("Event listener failed: {e}");
                }
            });

            let window_monitor_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                start_window_monitor(window_monitor_handle).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Failed to start application");
}

/// Start event listener
async fn start_event_listener(app_handle: tauri::AppHandle) -> anyhow::Result<()> {
    let app_state = app_handle.state::<AppState>();
    let mut shutdown_signal = app_state.shutdown_channel.1.resubscribe();
    let mask = CGEventType::KeyUp.mask_bit();
    let stream = CGEventTapStream::subscribe(TapLocation::Session, mask, 64)?;
    loop {
        tokio::select! {
            _ = shutdown_signal.recv() => {
                log::info!("Event listener shutting down");
                break;
            }
            event = stream.next() => {
                if event.is_none() {
                    log::error!("Event stream ended unexpectedly");
                    break;
                }
                let event = event.unwrap();
                log::debug!("Received event: {:?}", event);
                if app_state.is_calibrating_mode_enabled() {
                    if let Err(e) = handle_event_with_calibrating_mode(app_state.clone(), event).await {
                        log::error!("Calibrating event handler failed: {e}");
                    }
                } else {
                    if let Err(e) = handle_event(app_state.clone(), event).await {
                        log::error!("Event handler failed: {e}");
                    }
                }
            }
        }
    }
    Ok(())
}

/// Start window monitor
async fn start_window_monitor(app_handle: tauri::AppHandle) {
    let mut shutdown_rx = app_handle
        .state::<AppState>()
        .shutdown_channel
        .1
        .resubscribe();
    let monitor_handle = app_handle.clone();
    std::thread::spawn(move || {
        let config = MonitorConfig {
            track_window_bounds_changes: true,
            ..Default::default()
        };
        let monitor = WindowMonitor::with_config(
            ArkWindowListener {
                app_handle: monitor_handle,
            },
            config,
        );
        if let Err(e) = monitor.run() {
            log::error!("Window monitor failed: {e}");
        }
    });
    let _ = shutdown_rx.recv().await;
    log::info!("Window monitor shutting down");
    WindowMonitor::stop().ok();
}

/// Handle a single CGEventItem, executing the corresponding action if applicable.
async fn handle_event(app_state: State<'_, AppState>, event: CGEventItem) -> anyhow::Result<()> {
    if !app_state.is_hotkey_enabled() {
        log::debug!("Hotkey is disabled, ignoring event");
        return Ok(());
    }
    let (window_bounds, content_bounds, window_id, window_title) = {
        let window_ctx = app_state.window_ctx.read().await;
        if !window_ctx.is_available() {
            log::debug!("Window is not available, ignoring event");
            return Ok(());
        }
        let frame = window_ctx.bounds().unwrap().clone();
        let content = window_ctx.content_bounds().unwrap_or_else(|| frame.clone());
        (
            frame,
            content,
            window_ctx.vision_window_id(),
            window_ctx.vision_window_title(),
        )
    };
    if !position::check_in_window(event.location.x, event.location.y, &window_bounds) {
        log::debug!("Event is outside the window bounds, ignoring");
        return Ok(());
    }

    let action = app_state.get_action_for_keycode(event.keycode).await;
    if action.is_none() {
        log::debug!("No action mapped for keycode: {}", event.keycode);
        return Ok(());
    }
    let action = action.unwrap();
    let (ui_ratio, vision_config) = {
        let config = app_state.config.read().await;
        (config.effective_ui_ratio(), config.effective_vision())
    };
    let quick_action = matches!(action.action_id, "quick_skill" | "quick_retreat");
    let vision_mode = quick_action && vision_config.enabled;
    // A visible settings window can sit above the game and swallow the
    // synthetic clicks. Hide it before the action; the tray can reopen it.
    if quick_action && app_state.is_window_visible() {
        if let Some(window) = app_state.app_handle.get_webview_window("main") {
            log::info!("Hiding settings window before quick action");
            if let Err(error) = window.hide() {
                log::warn!("Failed to hide settings window: {error}");
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    let vision = if vision_mode && app_state.vision_permission_granted() {
        window_id.map(|window_id| {
            VisionRuntime::new(
                window_id,
                window_title,
                window_bounds.clone(),
                content_bounds.clone(),
                vision_config.clone(),
                app_state.vision_debug_dir(),
                app_state.vision_status.clone(),
            )
        })
    } else {
        None
    };
    if let Some(runtime) = &vision {
        // Overlap stream setup with the action's ClickCursor + WaitAnimation.
        runtime.warm();
    }
    let action_ctx = ActionContext::new_with_vision(
        &window_bounds,
        &ui_ratio,
        (event.location.x, event.location.y),
        vision_mode,
        vision,
    );

    log::info!("Executing action: {}", action.action_id);
    log::debug!("{:?}", action.steps);

    action_ctx.execute_action(action)?;

    log::info!("Action {} executed successfully", action.action_id);
    if vision_mode {
        app_state.emit_status_async().await;
    }

    Ok(())
}

/// Handle a single CGEventItem in calibrating mode.
async fn handle_event_with_calibrating_mode(
    app_state: State<'_, AppState>,
    event: CGEventItem,
) -> anyhow::Result<()> {
    let Some(ratio_type) = app_state.calibrating_target.read().await.clone() else {
        app_state.switch_calibrating_mode_enabled(false).await;
        log::warn!("Calibrating mode: no target selected");
        return Ok(());
    };
    let (frame_bounds, focused_bounds, is_arknights) = {
        let window_ctx = app_state.window_ctx.read().await;
        (
            window_ctx.calibration_bounds().cloned(),
            window_ctx.window_bounds.clone(),
            window_ctx.is_arknights,
        )
    };
    let Some(frame_bounds) = frame_bounds else {
        log::debug!("Calibrating mode: no game window bounds, ignoring event");
        return Ok(());
    };
    if !is_arknights
        && let Some(focused_bounds) = focused_bounds.as_ref()
        && position::check_in_window(event.location.x, event.location.y, focused_bounds)
    {
        log::debug!("Calibrating mode: ignoring event inside the settings window");
        return Ok(());
    }

    handle_ratio_calibration(app_state.inner(), event, &frame_bounds, ratio_type).await
}

/// Existing ratio calibration: Space records the cursor position.
async fn handle_ratio_calibration(
    app_state: &AppState,
    event: CGEventItem,
    window_bounds: &WindowBounds,
    ratio_type: UIRatioType,
) -> anyhow::Result<()> {
    if event.event_type != Some(CGEventType::KeyUp) || event.keycode != Keycode::SPACE {
        log::debug!("Calibrating mode: ignoring non-space key event");
        return Ok(());
    }
    let (cursor_x, cursor_y) = (event.location.x, event.location.y);
    if !position::check_in_window(cursor_x, cursor_y, window_bounds) {
        log::warn!(
            "Calibrating mode: cursor position ({}, {}) is outside the window bounds",
            cursor_x,
            cursor_y
        );
        return Ok(());
    }
    let (ratio_x, ratio_y) = (
        (cursor_x - window_bounds.x) / window_bounds.width,
        (cursor_y - window_bounds.y) / window_bounds.height,
    );
    if !(0.0..=1.0).contains(&ratio_x) || !(0.0..=1.0).contains(&ratio_y) {
        log::warn!(
            "Calibrating mode: cursor position ({}, {}) is outside the window bounds",
            cursor_x,
            cursor_y
        );
        return Ok(());
    }
    let ratio_payload = UIRatioPayload {
        ratio_type,
        ratio: (ratio_x, ratio_y),
    };
    app_state.update_ui_ratio(&ratio_payload).await?;
    app_state.emit_ratio_updated(&ratio_payload);

    Ok(())
}
