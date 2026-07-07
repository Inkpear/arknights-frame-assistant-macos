use cgevents::Keycode;

use crate::touch_core::{action::StepType, definition::ActionDefinition};

const ACTION_STEPS: [StepType; 3] = [
    StepType::ClickLeftPause,
    StepType::ClickCursor,
    StepType::ClickRightPause,
];

/// Click left pause → click cursor → click right pause.
pub struct PauseSelectedAction;

impl ActionDefinition for PauseSelectedAction {
    fn get_steps(&self) -> &'static [StepType] {
        &ACTION_STEPS
    }

    fn get_action_id(&self) -> &'static str {
        "pause_selected"
    }

    fn get_default_keycode(&self) -> u16 {
        Keycode::W
    }
}
