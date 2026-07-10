use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::touch_core::position::UIRatio;

const CONFIG_FILENAME: &str = "config.json";
const CONFIG_SUBDIR: &str = "configurations";

/// Persistent application config, resolved against an app-managed directory.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(default)]
pub struct AppConfig {
    pub hotkey_enabled: bool,
    pub language: AppLanguage,
    pub current_profile: AppConfigType,
    pub ui_ratio: Option<UIRatio>,
    pub regular_operations_keycode: Option<HashMap<String, u16>>,
    pub garrison_protocol_keycode: Option<HashMap<String, u16>>,
    /// Directory used for load/save, supplied by the app at startup.
    #[serde(skip)]
    pub config_dir: PathBuf,
}

impl AppConfig {
    fn config_path(&self) -> PathBuf {
        self.config_dir.join(CONFIG_SUBDIR).join(CONFIG_FILENAME)
    }

    /// Load from `<config_dir>/configurations/config.json`, returns default if missing.
    pub fn load(config_dir: PathBuf) -> anyhow::Result<Self> {
        let mut config: Self = if let Ok(content) =
            std::fs::read_to_string(config_dir.join(CONFIG_SUBDIR).join(CONFIG_FILENAME))
        {
            serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?
        } else {
            Self::default()
        };
        config.config_dir = config_dir;
        Ok(config)
    }

    /// Save to `<config_dir>/configurations/config.json`.
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = self.config_dir.join(CONFIG_SUBDIR);
        std::fs::create_dir_all(&dir)?;
        let path = self.config_path();
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
        std::fs::write(&path, data)
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", path.display(), e))?;
        log::debug!("Saved config to {}", path.display());
        Ok(())
    }

    /// Replace the custom keycode map for the current profile.
    /// An empty slice clears the map; a non-empty slice replaces it entirely.
    pub fn update_custom_keycode(&mut self, new_keycodes: &[(String, u16)]) {
        let map_ref = match self.current_profile {
            AppConfigType::RegularOperations => &mut self.regular_operations_keycode,
            AppConfigType::GarrisonProtocol => &mut self.garrison_protocol_keycode,
        };

        if new_keycodes.is_empty() {
            *map_ref = None;
            return;
        }
        let mut map = HashMap::new();
        for (action_id, keycode) in new_keycodes {
            map.insert(action_id.clone(), *keycode);
        }
        *map_ref = Some(map);
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
