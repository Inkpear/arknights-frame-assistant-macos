use cgevents::Keycode;

use crate::touch_core::{action::StepType, definition::ActionDefinition};

const ACTION_STEPS: [StepType; 3] = [
    StepType::ClickLeftPause,
    StepType::WaitMillis(33.0),
    StepType::ClickRightPause,
];

/// Click left pause → wait 33ms → click right pause.
pub struct Advance33msAction;

impl ActionDefinition for Advance33msAction {
    fn get_steps(&self) -> &'static [StepType] {
        &ACTION_STEPS
    }

    fn get_action_id(&self) -> &'static str {
        "advance_33ms"
    }

    fn get_default_keycode(&self) -> u16 {
        Keycode::T
    }
}
