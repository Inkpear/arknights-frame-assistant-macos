use cgevents::Keycode;

use crate::touch_core::{action::StepType, definition::ActionDefinition};

const ACTION_STEPS: [StepType; 1] = [StepType::ClickLeftPause];

/// Click left pause.
pub struct SwitchPauseAction;

impl ActionDefinition for SwitchPauseAction {
    fn get_steps(&self) -> &'static [StepType] {
        &ACTION_STEPS
    }

    fn get_action_id(&self) -> &'static str {
        "switch_pause"
    }

    fn get_default_keycode(&self) -> u16 {
        Keycode::SPACE
    }
}
