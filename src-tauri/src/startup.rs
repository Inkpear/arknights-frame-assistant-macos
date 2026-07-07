use std::sync::Arc;

use mado::{MonitorConfig, WindowMonitor};

use crate::touch_core::action::ActionContext;
use crate::{state::AppState, touch_core::window::ArkWindowListener};

use cgevents::async_api::{CGEventItem, CGEventTapStream};
use cgevents::{CG_EVENT_MASK_FOR_ALL_EVENTS, Keycode, TapLocation};

/// Start all services.
pub async fn run(app_state: Arc<AppState>) -> anyhow::Result<()> {
    tokio::try_join!(
        start_event_listener(app_state.clone()),
        start_monitor(app_state.clone()),
    )?;
    log::info!("All services have been shut down. Exiting.");
    Ok(())
}

/// Start event listener
async fn start_event_listener(app_state: Arc<AppState>) -> anyhow::Result<()> {
    let mut shutdown_signal = app_state.shutdown_channel.1.resubscribe();
    let stream =
        CGEventTapStream::subscribe(TapLocation::Session, CG_EVENT_MASK_FOR_ALL_EVENTS, 64)?;
    loop {
        tokio::select! {
            _ = shutdown_signal.recv() => {
                log::info!("Received shutdown signal, shutting down event listener...");
                break;
            }
            event = stream.next() => {
                if event.is_none() {
                    log::error!("Event stream ended unexpectedly.");
                    break;
                }
                let event = event.unwrap();
                log::debug!("Received event: {:?}", event);
                if app_state.is_calibrating_mode_enabled() {
                    if let Err(e) = handle_event_with_calibrating_mode(Arc::clone(&app_state), event) {
                        log::error!("Error handling event in calibrating mode: {e}");
                    }
                } else {
                    if let Err(e) = handle_event(Arc::clone(&app_state), event) {
                        log::error!("Error handling event: {e}");
                    }
                }
            }
        }
    }
    Ok(())
}

/// Start window monitor
async fn start_monitor(app_state: Arc<AppState>) -> anyhow::Result<()> {
    let mut shutdown_signal = app_state.shutdown_channel.1.resubscribe();
    let config = MonitorConfig {
        track_window_bounds_changes: true,
        ..Default::default()
    };
    let monitor = WindowMonitor::with_config(ArkWindowListener { app_state }, config);
    tokio::select! {
        _ = shutdown_signal.recv() => {
            log::info!("Received shutdown signal, shutting down process monitor...");
            WindowMonitor::stop()?;
        }
        res = tokio::task::spawn_blocking(move || monitor.run()) => {
            if let Err(e) = res {
                log::error!("Window monitor failed: {e}");
            }
        }
    }
    Ok(())
}

/// Handle a single CGEventItem, executing the corresponding action if applicable.
fn handle_event(app_state: Arc<AppState>, event: CGEventItem) -> anyhow::Result<()> {
    log::debug!("Received event: {:?}", event);
    if !app_state.is_hotkey_enabled() {
        log::debug!("Hotkey is disabled, ignoring event");
        return Ok(());
    }
    if !app_state.is_window_available() {
        log::debug!("Window is not available, ignoring event");
        return Ok(());
    }

    let action = app_state.get_action_for_keycode(event.keycode);
    if action.is_none() {
        log::debug!("No action mapped for keycode: {}", event.keycode);
        return Ok(());
    }
    let action = action.unwrap();
    let action_ctx: ActionContext;
    {
        let config_guard = app_state.config.read().unwrap();
        let window_ctx_guard = app_state.window_ctx.read().unwrap();
        action_ctx = ActionContext::new(
            window_ctx_guard.bounds().unwrap(),
            config_guard
                .ui_ratio
                .as_ref()
                .unwrap_or(&Default::default()),
            (event.location.x, event.location.y),
        )?;
    }

    log::info!("Executing action: {}", action.get_action_id());
    log::debug!("{:?}", action.get_steps());

    action_ctx.execute_action(action)?;

    log::info!("Action {} executed successfully", action.get_action_id());

    Ok(())
}

/// Handle a single CGEventItem in calibrating mode.
fn handle_event_with_calibrating_mode(
    app_state: Arc<AppState>,
    event: CGEventItem,
) -> anyhow::Result<()> {
    if !app_state.is_window_available() {
        log::debug!("Window is not available, ignoring event");
        return Ok(());
    }
    if event.keycode != Keycode::SPACE {
        log::debug!("Calibrating mode: ignoring non-space key event");
        return Ok(());
    }
    let (cursor_x, cursor_y) = (event.location.x, event.location.y);
    let (ratio_x, ratio_y) = {
        let window_ctx_guard = app_state.window_ctx.read().unwrap();
        let bounds = window_ctx_guard.bounds().unwrap();
        (
            (cursor_x - bounds.x) / bounds.width,
            (cursor_y - bounds.y) / bounds.height,
        )
    };
    if !(0.0..=1.0).contains(&ratio_x) || !(0.0..=1.0).contains(&ratio_y) {
        log::warn!(
            "Calibrating mode: cursor position ({}, {}) is outside the window bounds",
            cursor_x,
            cursor_y
        );
        return Ok(());
    }

    if let Some(target) = app_state.calibrating_target.read().unwrap().as_ref() {
        app_state.emit_ratio_updated(target, (ratio_x, ratio_y));
    }

    Ok(())
}
