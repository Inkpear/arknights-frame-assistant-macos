use std::{collections::HashMap, sync::atomic::AtomicBool};

use tauri::{Emitter, Manager, WebviewWindowBuilder, WebviewUrl};
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tokio::sync::{RwLock, broadcast};

use crate::{
    config::{AppConfig, AppConfigType, AppLanguage},
    desktop::TrayMenuItemHandles,
    ipc::protocol::{AppStatusPayload, UIRatioPayload, WindowInfoPayload},
    touch_core::{
        definition::{self, ActionDef},
        position::{UIRatio, UIRatioType},
        window::WindowContext,
    },
};

/// Central application state shared across all services.
///
/// Lock invariant: the `config` and `window_ctx` `RwLock`s must never be held
/// across an `.await`. Holders drop guards before awaiting to avoid blocking
/// the mado monitor thread (which uses `blocking_read`/`blocking_write`).
pub struct AppState {
    pub calibrating_mode: AtomicBool,
    /// Lock-free mirror of `config.hotkey_enabled`, read on the hot KeyUp path.
    pub hotkey_enabled: AtomicBool,
    pub config: RwLock<AppConfig>,
    pub keycode_map: RwLock<HashMap<u16, &'static ActionDef>>,
    pub window_ctx: RwLock<WindowContext>,
    pub calibrating_target: RwLock<Option<UIRatioType>>,
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
        let hotkey_enabled = AtomicBool::new(config.hotkey_enabled);
        Self {
            calibrating_mode: AtomicBool::new(false),
            hotkey_enabled,
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
        self.hotkey_enabled
            .store(enabled, std::sync::atomic::Ordering::SeqCst);

        self.emit_status_async().await;
        self.save().await?;
        Ok(())
    }

    pub async fn switch_calibrating_mode_enabled(&self, enabled: bool) {
        self.calibrating_mode
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
        self.emit_status_async().await;
    }

    pub async fn set_calibrating_target(&self, target: UIRatioType) {
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
        self.emit_status_async().await;
        self.save().await?;
        Ok(())
    }

    pub async fn update_ui_ratio(&self, new_ratio: &UIRatioPayload) -> anyhow::Result<()> {
        let mut guard = self.config.write().await;
        let ratio = guard.ui_ratio.get_or_insert_with(UIRatio::default);
        match new_ratio.ratio_type {
            UIRatioType::LeftPause => ratio.left_pause = new_ratio.ratio,
            UIRatioType::RightPause => ratio.right_pause = new_ratio.ratio,
            UIRatioType::Skill => ratio.skill = new_ratio.ratio,
            UIRatioType::Retreat => ratio.retreat = new_ratio.ratio,
            UIRatioType::Speed => ratio.speed = new_ratio.ratio,
        }
        drop(guard);
        self.switch_calibrating_mode_enabled(false).await;
        self.save().await
    }

    pub async fn reset_ui_ratio(&self, ratio_type: &UIRatioType) -> anyhow::Result<()> {
        let default_ratio = UIRatio::default();
        let mut guard = self.config.write().await;
        let ratio = guard.ui_ratio.get_or_insert_with(UIRatio::default);
        match ratio_type {
            UIRatioType::LeftPause => ratio.left_pause = default_ratio.left_pause,
            UIRatioType::RightPause => ratio.right_pause = default_ratio.right_pause,
            UIRatioType::Skill => ratio.skill = default_ratio.skill,
            UIRatioType::Retreat => ratio.retreat = default_ratio.retreat,
            UIRatioType::Speed => ratio.speed = default_ratio.speed,
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

    /// Toggle the settings window. On first open, creates the WebView
    /// (lazy init); hides instead of closing to keep the app alive.
    pub async fn toggle_window(&self) {
        if let Some(window) = self.app_handle.get_webview_window("main") {
            if window.is_visible().unwrap_or(false) {
                window.hide().ok();
            } else {
                window.show().ok();
                window.set_focus().ok();
            }
        } else {
            // First open: create the window (WebView + frontend).
            match WebviewWindowBuilder::new(
                &self.app_handle,
                "main",
                WebviewUrl::App("index.html".into()),
            )
            .title("AFA")
            .inner_size(720.0, 560.0)
            .min_inner_size(580.0, 420.0)
            .transparent(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true)
            .traffic_light_position(tauri::LogicalPosition::new(16.0, 18.0))
            .effects(
                EffectsBuilder::default()
                    .effect(Effect::HudWindow)
                    .radius(14.0)
                    .state(EffectState::Active)
                    .build(),
            )
            .build()
            {
                Ok(window) => {
                    window.set_focus().ok();
                }
                Err(e) => {
                    log::error!("Failed to create main window: {e}");
                }
            }
        }
        self.emit_status_async().await;
    }

    pub fn is_hotkey_enabled(&self) -> bool {
        self.hotkey_enabled
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn is_calibrating_mode_enabled(&self) -> bool {
        self.calibrating_mode
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub async fn get_action_for_keycode(&self, keycode: u16) -> Option<&'static ActionDef> {
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
        self.build_status_payload(config, window_ctx)
    }

    /// Build a status payload from a config + window snapshot.
    fn build_status_payload(
        &self,
        config: AppConfig,
        window_ctx: WindowContext,
    ) -> AppStatusPayload {
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
        let is_english = config.language == AppLanguage::English;
        let hotkey_enabled = config.hotkey_enabled;
        let is_window_visible = self.is_window_visible();
        let window_available = window_ctx.is_available();
        let payload = self.build_status_payload(config, window_ctx);

        // Tray/menu + emit must run on the main thread (called from the mado
        // monitor thread or a tokio worker). Schedule a single main-thread job.
        let outer = self.app_handle.clone();
        let inner = outer.clone();
        if let Err(e) = outer.run_on_main_thread(move || {
            let tray = inner.state::<TrayMenuItemHandles>();
            if let Err(e) = tray.update_tray_status(
                is_english,
                hotkey_enabled,
                window_available,
                is_window_visible,
            ) {
                log::error!("Failed to update tray: {e}");
            }
            if let Err(e) = inner.emit("status-changed", payload) {
                log::error!("Failed to emit status-changed event: {e}");
            }
        }) {
            log::error!("Failed to schedule main-thread status emit: {e}");
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
