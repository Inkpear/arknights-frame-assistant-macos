use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::touch_core::position::UIRatio;

const CONFIG_FILENAME: &str = "config.json";

/// Persistent application config.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub hotkey_enabled: bool,
    pub current_profile: AppConfigType,
    pub ui_ratio: Option<UIRatio>,
    pub regular_operations_keycode: Option<HashMap<String, u16>>,
    pub garrison_protocol_keycode: Option<HashMap<String, u16>>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey_enabled: false,
            current_profile: AppConfigType::RegularOperations,
            ui_ratio: None,
            regular_operations_keycode: None,
            garrison_protocol_keycode: None,
        }
    }
}

impl AppConfig {
    /// Load from path/configurations/config.json, returns default if not found.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let config_path = std::path::Path::new(path)
            .join("configurations")
            .join(CONFIG_FILENAME);
        match std::fs::read_to_string(&config_path) {
            Ok(content) => serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", config_path.display(), e)),
            Err(_) => Ok(Self::default()),
        }
    }

    /// Save to path/configurations/config.json.
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let config_dir = std::path::Path::new(path).join("configurations");
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join(CONFIG_FILENAME);
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
        std::fs::write(&config_path, data)
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", config_path.display(), e))?;
        Ok(())
    }

    /// Current effective UIRatio.
    pub fn effective_ui_ratio(&self) -> UIRatio {
        self.ui_ratio.clone().unwrap_or_default()
    }

    /// Custom keycodes for the current profile.
    pub fn current_keycode(&self) -> Option<&HashMap<String, u16>> {
        self.profile_keycode(&self.current_profile)
    }

    /// Custom keycodes for the given profile.
    pub fn profile_keycode(&self, profile: &AppConfigType) -> Option<&HashMap<String, u16>> {
        match profile {
            AppConfigType::RegularOperations => self.regular_operations_keycode.as_ref(),
            AppConfigType::GarrisonProtocol => self.garrison_protocol_keycode.as_ref(),
        }
    }
}

/// The active profile / game mode.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum AppConfigType {
    RegularOperations,
    GarrisonProtocol,
}
