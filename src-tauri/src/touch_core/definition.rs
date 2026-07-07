use std::{collections::HashMap, sync::LazyLock};

use crate::{config::AppConfigType, touch_core::action::StepType};

mod advance_12ms;
mod advance_166ms;
mod advance_33ms;
mod pause_retreat;
mod pause_selected;
mod pause_skill;
mod quick_retreat;
mod quick_skill;
mod switch_pause;
mod switch_speed;

/// A game action defined by a sequence of mouse steps and a default key binding.
pub trait ActionDefinition: Sync + 'static {
    fn get_steps(&self) -> &'static [StepType];
    fn get_action_id(&self) -> &'static str;
    fn get_default_keycode(&self) -> u16;
    fn get_restore_cursor(&self) -> bool {
        true
    }
}

/// Action-id → Action mapping for the Regular Operations profile.
pub static REGULAR_OPERATIONS_ACTION_ID_MAPPING: LazyLock<
    HashMap<&'static str, &'static dyn ActionDefinition>,
> = LazyLock::new(|| {
    let actions: Vec<&'static dyn ActionDefinition> = vec![
        &advance_12ms::Advance12msAction,
        &advance_33ms::Advance33msAction,
        &advance_166ms::Advance166msAction,
        &pause_retreat::PauseRetreatAction,
        &pause_selected::PauseSelectedAction,
        &pause_skill::PauseSkillAction,
        &quick_retreat::QuickRetreatAction,
        &quick_skill::QuickSkillAction,
        &switch_pause::SwitchPauseAction,
        &switch_speed::SwitchSpeedAction,
    ];
    HashMap::from_iter(
        actions
            .into_iter()
            .map(|action| (action.get_action_id(), action)),
    )
});

/// Action-id → Action mapping for the Garrison Protocol profile.
pub static GARRISON_PROTOCOL_ACTION_ID_MAPPING: LazyLock<
    HashMap<&'static str, &'static dyn ActionDefinition>,
> = LazyLock::new(|| todo!());

/// Get the static action mapping for a profile.
pub fn static_mapping_for(
    profile: &AppConfigType,
) -> &'static HashMap<&'static str, &'static dyn ActionDefinition> {
    match profile {
        AppConfigType::RegularOperations => &REGULAR_OPERATIONS_ACTION_ID_MAPPING,
        AppConfigType::GarrisonProtocol => &GARRISON_PROTOCOL_ACTION_ID_MAPPING,
    }
}

/// Build a keycode → Action lookup table from static mapping and custom keycodes.
pub fn build_keycode_map(
    static_mapping: &HashMap<&'static str, &'static dyn ActionDefinition>,
    custom_keycode: Option<&HashMap<String, u16>>,
) -> HashMap<u16, &'static dyn ActionDefinition> {
    let mut map = HashMap::new();
    for (action_id, action) in static_mapping {
        let keycode = custom_keycode
            .and_then(|c| c.get(*action_id))
            .copied()
            .unwrap_or_else(|| action.get_default_keycode());
        if let Some(prev) = map.insert(keycode, *action)
            && prev.get_action_id() != *action_id
        {
            log::warn!(
                "Keycode {} shadowed: {} overwritten by {}",
                keycode,
                prev.get_action_id(),
                action_id
            );
        }
    }
    map
}
