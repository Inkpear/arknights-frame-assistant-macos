use cgevents::Keycode;

use crate::touch_core::{action::StepType, definition::ActionDefinition};

const ACTION_STEPS: [StepType; 1] = [StepType::ClickSpeed];

/// Click speed toggle.
pub struct SwitchSpeedAction;

impl ActionDefinition for SwitchSpeedAction {
    fn get_steps(&self) -> &'static [StepType] {
        &ACTION_STEPS
    }

    fn get_action_id(&self) -> &'static str {
        "switch_speed"
    }

    fn get_default_keycode(&self) -> u16 {
        Keycode::D
    }
}
