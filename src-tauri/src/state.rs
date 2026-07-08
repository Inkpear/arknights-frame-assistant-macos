use std::{collections::HashMap, sync::atomic::AtomicBool};

use tauri::{Emitter, Manager};
use tokio::sync::{RwLock, broadcast};

use crate::{
    config::{AppConfig, AppConfigType, AppLanguage},
    desktop::tray::TrayMenuItemHandles,
    ipc::protocol::{AppStatusPayload, UIRatioPayload, WindowInfoPayload},
    touch_core::{
        definition::{self, ActionDefinition},
        position::{UIRatio, UIRationType},
        window::WindowContext,
    },
};

/// Central application state shared across all services.
pub struct AppState {
    pub calibrating_mode: AtomicBool,
    pub config: RwLock<AppConfig>,
    pub keycode_map: RwLock<HashMap<u16, &'static dyn ActionDefinition>>,
    pub window_ctx: RwLock<WindowContext>,
    pub calibrating_target: RwLock<Option<UIRationType>>,
    pub shutdown_channel: (broadcast::Sender<()>, broadcast::Receiver<()>),
    pub app_handle: tauri::AppHandle,
}

impl AppState {
    pub fn new(config: AppConfig, app_handle: tauri::AppHandle) -> Self {
        let keycode_map = RwLock::new(definition::build_keycode_map(
            definition::static_mapping_for(&config.current_profile),
            config.current_keycode(),
        ));
        let shutdown_channel = broadcast::channel(1);
        Self {
            calibrating_mode: AtomicBool::new(false),
            config: RwLock::new(config),
            keycode_map,
            window_ctx: RwLock::new(WindowContext::default()),
            calibrating_target: RwLock::new(None),
            shutdown_channel,
            app_handle,
        }
    }

    pub async fn switch_config(&self, new_profile: AppConfigType) -> anyhow::Result<()> {
        let keycode_override: Option<HashMap<String, u16>>;
        {
            let mut config_guard = self.config.write().await;
            if config_guard.current_profile == new_profile {
                return Ok(());
            }
            keycode_override = config_guard.profile_keycode(&new_profile).cloned();
            config_guard.current_profile = new_profile.clone();
        }

        let new_map = definition::build_keycode_map(
            definition::static_mapping_for(&new_profile),
            keycode_override.as_ref(),
        );
        *self.keycode_map.write().await = new_map;
        self.emit_status_async().await;
        self.save().await?;

        Ok(())
    }

    pub async fn switch_hotkey_enabled(&self, enabled: bool) -> anyhow::Result<()> {
        {
            let mut config_guard = self.config.write().await;
            if config_guard.hotkey_enabled == enabled {
                return Ok(());
            }
            config_guard.hotkey_enabled = enabled;
        }

        self.emit_status_async().await;
        self.save().await?;
        Ok(())
    }

    pub async fn switch_calibrating_mode_enabled(&self, enabled: bool) {
        self.calibrating_mode
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
        self.emit_status_async().await;
    }

    pub async fn set_calibrating_target(&self, target: UIRationType) {
        *self.calibrating_target.write().await = Some(target);
    }

    pub async fn update_custom_keycode(
        &self,
        custom_actions: &[(String, u16)],
    ) -> anyhow::Result<()> {
        let (profile, updated_map);
        {
            let mut config_guard = self.config.write().await;
            config_guard.update_custom_keycode(custom_actions);
            (profile, updated_map) = config_guard.get_current_keycode_map();
        }

        let new_map = definition::build_keycode_map(
            definition::static_mapping_for(&profile),
            updated_map.as_ref(),
        );
        *self.keycode_map.write().await = new_map;
        self.save().await?;
        Ok(())
    }

    pub async fn update_ui_ratio(&self, new_ratio: &UIRatioPayload) -> anyhow::Result<()> {
        let mut guard = self.config.write().await;
        let ratio = guard.ui_ratio.get_or_insert_with(UIRatio::default);
        match new_ratio.ratio_type {
            UIRationType::LeftPause => ratio.left_pause = new_ratio.ratio,
            UIRationType::RightPause => ratio.right_pause = new_ratio.ratio,
            UIRationType::Skill => ratio.skill = new_ratio.ratio,
            UIRationType::Retreat => ratio.retreat = new_ratio.ratio,
            UIRationType::Speed => ratio.speed = new_ratio.ratio,
        }
        drop(guard);
        self.switch_calibrating_mode_enabled(false).await;
        self.save().await
    }

    pub async fn reset_ui_ratio(&self, ratio_type: &UIRationType) -> anyhow::Result<()> {
        let default_ratio = UIRatio::default();
        let mut guard = self.config.write().await;
        let ratio = guard.ui_ratio.get_or_insert_with(UIRatio::default);
        match ratio_type {
            UIRationType::LeftPause => ratio.left_pause = default_ratio.left_pause,
            UIRationType::RightPause => ratio.right_pause = default_ratio.right_pause,
            UIRationType::Skill => ratio.skill = default_ratio.skill,
            UIRationType::Retreat => ratio.retreat = default_ratio.retreat,
            UIRationType::Speed => ratio.speed = default_ratio.speed,
        }
        drop(guard);
        self.emit_status_async().await;
        self.save().await
    }

    pub async fn reset_config(&self) -> anyhow::Result<()> {
        {
            let mut guard = self.config.write().await;
            guard.regular_operations_keycode = None;
            guard.garrison_protocol_keycode = None;
            guard.ui_ratio = None;
        }
        let profile = {
            let guard = self.config.read().await;
            guard.current_profile.clone()
        };
        let new_map = definition::build_keycode_map(definition::static_mapping_for(&profile), None);
        *self.keycode_map.write().await = new_map;
        self.emit_status_async().await;
        self.save().await
    }

    pub async fn toggle_language(&self) -> anyhow::Result<()> {
        {
            let mut config = self.config.write().await;
            config.language = match config.language {
                AppLanguage::English => AppLanguage::Chinese,
                AppLanguage::Chinese => AppLanguage::English,
            };
        }
        self.emit_status_async().await;
        self.save().await?;
        Ok(())
    }

    pub fn is_hotkey_enabled(&self) -> bool {
        tokio::task::block_in_place(|| self.config.blocking_read()).hotkey_enabled
    }

    pub fn is_calibrating_mode_enabled(&self) -> bool {
        self.calibrating_mode
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub async fn get_action_for_keycode(
        &self,
        keycode: u16,
    ) -> Option<&'static dyn ActionDefinition> {
        self.keycode_map.read().await.get(&keycode).copied()
    }

    pub fn is_window_visible(&self) -> bool {
        self.app_handle
            .get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false)
    }

    pub fn shutdown(&self) -> anyhow::Result<usize> {
        Ok(self.shutdown_channel.0.send(())?)
    }

    /// Push full state to frontend + sync tray icon and menu labels (sync, for non-runtime threads).
    pub fn emit_status(&self) {
        let config = self.config.blocking_read().clone();
        let window_ctx = self.window_ctx.blocking_read().clone();
        self.emit_status_data(config, window_ctx);
    }

    /// Push full state to frontend + sync tray icon and menu labels (async).
    pub async fn emit_status_async(&self) {
        let config = self.config.read().await.clone();
        let window_ctx = self.window_ctx.read().await.clone();
        self.emit_status_data(config, window_ctx);
    }

    /// Build a status payload for the frontend without emitting it.
    pub async fn get_status_payload(&self) -> AppStatusPayload {
        let config = self.config.read().await.clone();
        let window_ctx = self.window_ctx.read().await.clone();
        let hotkey_enabled = config.hotkey_enabled;
        let is_english = config.language == AppLanguage::English;
        let window_available = window_ctx.is_available();
        AppStatusPayload {
            hotkey_enabled,
            calibrating_mode_enabled: self.is_calibrating_mode_enabled(),
            current_profile: config.current_profile,
            hotkey_active: hotkey_enabled && window_available,
            language: if is_english {
                "中文".to_string()
            } else {
                "English".to_string()
            },
            regular_operations_keycode: config.regular_operations_keycode,
            garrison_protocol_keycode: config.garrison_protocol_keycode,
            ui_ratio: config.ui_ratio,
            window: WindowInfoPayload {
                app_name: window_ctx.app_name,
                window_title: window_ctx.window_title,
                bounds: window_ctx.window_bounds,
                is_arknights: window_ctx.is_arknights,
                is_available: window_available,
            },
        }
    }

    fn emit_status_data(&self, config: AppConfig, window_ctx: WindowContext) {
        let hotkey_enabled = config.hotkey_enabled;
        let is_english = config.language == AppLanguage::English;
        let language_label = if is_english { "中文" } else { "English" };
        let is_window_visible = self.is_window_visible();
        let window_available = window_ctx.is_available();

        let tray_menu_handles = self.app_handle.state::<TrayMenuItemHandles>();
        tray_menu_handles
            .update_tray_status(is_english, hotkey_enabled, window_available, is_window_visible)
            .expect("Failed to update tray menu labels, Must register TrayMenuItemHandles in app state before calling emit_status()");

        if let Err(e) = self.app_handle.emit(
            "status-changed",
            AppStatusPayload {
                hotkey_enabled,
                calibrating_mode_enabled: self.is_calibrating_mode_enabled(),
                current_profile: config.current_profile,
                hotkey_active: hotkey_enabled && window_available,
                language: language_label.to_string(),
                regular_operations_keycode: config.regular_operations_keycode,
                garrison_protocol_keycode: config.garrison_protocol_keycode,
                ui_ratio: config.ui_ratio.clone(),
                window: WindowInfoPayload {
                    app_name: window_ctx.app_name,
                    window_title: window_ctx.window_title,
                    bounds: window_ctx.window_bounds,
                    is_arknights: window_ctx.is_arknights,
                    is_available: window_available,
                },
            },
        ) {
            log::error!("Failed to emit status-changed event: {e}");
        }
    }

    pub fn emit_ratio_updated(&self, ratio_payload: &UIRatioPayload) {
        let _ = self.app_handle.emit(
            "ratio-updated",
            UIRatioPayload {
                ratio_type: ratio_payload.ratio_type.clone(),
                ratio: ratio_payload.ratio,
            },
        );
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let config_guard = self.config.read().await.clone();
        tokio::task::spawn_blocking(move || config_guard.save()).await?
    }
}
