use std::{
    collections::HashMap,
    sync::{RwLock, atomic::AtomicBool},
};

use tauri::Emitter;
use tokio::sync::broadcast;

use crate::{
    config::{AppConfig, AppConfigType},
    ipc::protocol::{AppStatusPayload, UIRatioPayload, WindowInfoPayload},
    touch_core::{
        definition::{self, ActionDefinition},
        position::{UIRatio, UIRationType},
        window::WindowContext,
    },
};

/// Central application state shared across all services.
pub struct AppState {
    pub hotkey_enabled: AtomicBool,
    pub calibrating_mode: AtomicBool,
    pub config: RwLock<AppConfig>,
    pub keycode_map: RwLock<HashMap<u16, &'static dyn ActionDefinition>>,
    pub window_ctx: RwLock<WindowContext>,
    pub calibrating_target: RwLock<Option<UIRationType>>,
    pub shutdown_channel: (broadcast::Sender<()>, broadcast::Receiver<()>),
    pub app_handle: tauri::AppHandle,
    data_dir: String,
}

impl AppState {
    /// Create AppState with the given config and data directory.
    pub fn new(config: AppConfig, data_dir: String, app_handle: tauri::AppHandle) -> Self {
        let hotkey_enabled = AtomicBool::new(config.hotkey_enabled);
        let keycode_map = RwLock::new(definition::build_keycode_map(
            definition::static_mapping_for(&config.current_profile),
            config.current_keycode(),
        ));
        let shutdown_channel = broadcast::channel(1);
        Self {
            hotkey_enabled,
            calibrating_mode: AtomicBool::new(false),
            config: RwLock::new(config),
            keycode_map,
            window_ctx: RwLock::new(WindowContext::default()),
            calibrating_target: RwLock::new(None),
            app_handle,
            data_dir,
            shutdown_channel,
        }
    }

    /// Switch to a different profile.
    pub fn switch_config(&self, new_profile: AppConfigType) -> anyhow::Result<()> {
        let keycode_override: Option<HashMap<String, u16>>;
        {
            let guard = self.config.read().unwrap();
            if guard.current_profile == new_profile {
                return Ok(());
            }
            keycode_override = guard.profile_keycode(&new_profile).cloned();
        }
        self.config.write().unwrap().current_profile = new_profile.clone();

        let new_map = definition::build_keycode_map(
            definition::static_mapping_for(&new_profile),
            keycode_override.as_ref(),
        );
        *self.keycode_map.write().unwrap() = new_map;
        self.emit_status();
        self.save()
    }

    /// Toggle hotkey enabled state.
    pub fn switch_hotkey_enabled(&self, enabled: bool) -> anyhow::Result<()> {
        self.hotkey_enabled
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
        self.config.write().unwrap().hotkey_enabled = enabled;
        self.emit_status();
        self.save()
    }

    /// Toggle calibrating mode.
    pub fn switch_calibrating_mode_enabled(&self, enabled: bool) {
        self.calibrating_mode
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
        self.emit_status();
    }

    /// Set the UI element currently being calibrated.
    pub fn set_calibrating_target(&self, target: UIRationType) {
        *self.calibrating_target.write().unwrap() = Some(target);
    }

    /// Update the custom keycode for actions in the current profile.
    pub fn update_custom_keycode(&self, custom_actions: &[(String, u16)]) -> anyhow::Result<()> {
        let updated_keycode: Option<HashMap<String, u16>>;
        let profile: AppConfigType;
        {
            let mut guard = self.config.write().unwrap();
            profile = guard.current_profile.clone();
            let keycode_field = match guard.current_profile {
                AppConfigType::RegularOperations => &mut guard.regular_operations_keycode,
                AppConfigType::GarrisonProtocol => &mut guard.garrison_protocol_keycode,
            };
            keycode_field.get_or_insert_with(HashMap::new).extend(
                custom_actions
                    .iter()
                    .map(|(id, keycode)| (id.clone(), *keycode)),
            );
            updated_keycode = keycode_field.clone();
        }
        let new_map = definition::build_keycode_map(
            definition::static_mapping_for(&profile),
            updated_keycode.as_ref(),
        );
        *self.keycode_map.write().unwrap() = new_map;
        self.save()
    }

    /// Update the UI ratio in the config.
    pub fn update_ui_ratio(&self, new_ratios: &[UIRatioPayload]) -> anyhow::Result<()> {
        let mut guard = self.config.write().unwrap();
        let ratio = guard.ui_ratio.get_or_insert_with(UIRatio::default);
        for item in new_ratios {
            match item.ratio_type {
                UIRationType::LeftPause => ratio.left_pause = item.ratio,
                UIRationType::RightPause => ratio.right_pause = item.ratio,
                UIRationType::Skill => ratio.skill = item.ratio,
                UIRationType::Retreat => ratio.retreat = item.ratio,
                UIRationType::Speed => ratio.speed = item.ratio,
            }
        }
        drop(guard);
        self.save()
    }

    /// Whether hotkey event handling is enabled.
    pub fn is_hotkey_enabled(&self) -> bool {
        self.hotkey_enabled
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Whether the current tracked window belongs to Arknights and has bounds.
    pub fn is_window_available(&self) -> bool {
        self.window_ctx.read().unwrap().is_available()
    }

    /// Whether calibrating mode is active.
    pub fn is_calibrating_mode_enabled(&self) -> bool {
        self.calibrating_mode
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Look up the ActionDefinition mapped to a keycode.
    pub fn get_action_for_keycode(&self, keycode: u16) -> Option<&'static dyn ActionDefinition> {
        self.keycode_map.read().unwrap().get(&keycode).copied()
    }

    /// Send shutdown signal to all background tasks.
    pub fn shutdown(&self) -> anyhow::Result<usize> {
        Ok(self.shutdown_channel.0.send(())?)
    }

    /// Emit a status-changed event to the frontend.
    pub fn emit_status(&self) {
        let window_ctx = self.window_ctx.read().unwrap();
        let config = self.config.read().unwrap();
        let window_available = window_ctx.is_available();
        let hotkey_enabled = self.is_hotkey_enabled();
        let _ = self.app_handle.emit(
            "status-changed",
            AppStatusPayload {
                hotkey_enabled,
                calibrating_mode_enabled: self.is_calibrating_mode_enabled(),
                current_profile: config.current_profile.clone(),
                hotkey_active: hotkey_enabled && window_available,
                window: WindowInfoPayload {
                    app_name: window_ctx.app_name.clone(),
                    window_title: window_ctx.window_title.clone(),
                    bounds: window_ctx.window_bounds.clone(),
                    is_arknights: window_ctx.is_arknights,
                    is_available: window_available,
                },
            },
        );
    }
    /// Emit a ratio-updated event for calibrating mode.
    pub fn emit_ratio_updated(&self, ratio_type: &UIRationType, ratio: (f64, f64)) {
        let _ = self.app_handle.emit(
            "ratio-updated",
            UIRatioPayload {
                ratio_type: ratio_type.clone(),
                ratio,
            },
        );
    }

    fn save(&self) -> anyhow::Result<()> {
        self.config.read().unwrap().save(&self.data_dir)
    }
}
