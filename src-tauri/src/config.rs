use std::{collections::HashMap, sync::LazyLock};

use serde::{Deserialize, Serialize};

use crate::touch_core::position::UIRatio;

const CONFIG_FILENAME: &str = "config.json";
static APP_DIR: LazyLock<String> = LazyLock::new(|| {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
    format!("{home}/Library/Application Support/arknights-frame-assistant-macos")
});

/// Persistent application config.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(default)]
pub struct AppConfig {
    pub hotkey_enabled: bool,
    pub language: AppLanguage,
    pub current_profile: AppConfigType,
    pub ui_ratio: Option<UIRatio>,
    pub regular_operations_keycode: Option<HashMap<String, u16>>,
    pub garrison_protocol_keycode: Option<HashMap<String, u16>>,
}

impl AppConfig {
    /// Load from path/configurations/config.json, returns default if not found.
    pub fn load() -> anyhow::Result<Self> {
        let config_path = std::path::Path::new(&*APP_DIR)
            .join("configurations")
            .join(CONFIG_FILENAME);
        match std::fs::read_to_string(&config_path) {
            Ok(content) => serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", config_path.display(), e)),
            Err(_) => Ok(Self::default()),
        }
    }

    /// Save to path/configurations/config.json.
    pub fn save(&self) -> anyhow::Result<()> {
        let config_dir = std::path::Path::new(&*APP_DIR).join("configurations");
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join(CONFIG_FILENAME);
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
        std::fs::write(&config_path, data)
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", config_path.display(), e))?;
        Ok(())
    }

    /// Update the custom keycode map for the current profile.
    pub fn update_custom_keycode(&mut self, new_keycodes: &[(String, u16)]) {
        if new_keycodes.is_empty() {
            match self.current_profile {
                AppConfigType::RegularOperations => self.regular_operations_keycode = None,
                AppConfigType::GarrisonProtocol => self.garrison_protocol_keycode = None,
            }
            return;
        }
        let map_ref = match self.current_profile {
            AppConfigType::RegularOperations => &mut self.regular_operations_keycode,
            AppConfigType::GarrisonProtocol => &mut self.garrison_protocol_keycode,
        };

        let map = map_ref.get_or_insert_with(HashMap::new);
        for (action_id, keycode) in new_keycodes {
            map.insert(action_id.clone(), *keycode);
        }
    }

    /// Get the current profile and its custom keycode map.
    pub fn get_current_keycode_map(&self) -> (AppConfigType, Option<HashMap<String, u16>>) {
        let map = match self.current_profile {
            AppConfigType::RegularOperations => self.regular_operations_keycode.clone(),
            AppConfigType::GarrisonProtocol => self.garrison_protocol_keycode.clone(),
        };
        (self.current_profile.clone(), map)
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
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default)]
pub enum AppConfigType {
    #[default]
    RegularOperations,
    GarrisonProtocol,
}

/// Application language.
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub enum AppLanguage {
    English,
    #[default]
    Chinese,
}
