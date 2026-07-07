use cgevents::Keycode;

use crate::touch_core::{action::StepType, definition::ActionDefinition};

const ACTION_STEPS: [StepType; 5] = [
    StepType::ClickLeftPause,
    StepType::ClickCursor,
    StepType::ClickRightPause,
    StepType::WaitAnimation,
    StepType::ClickRetreat,
];

/// Click left pause → click cursor → click right pause → wait → click retreat.
pub struct PauseRetreatAction;

impl ActionDefinition for PauseRetreatAction {
    fn get_steps(&self) -> &'static [StepType] {
        &ACTION_STEPS
    }

    fn get_action_id(&self) -> &'static str {
        "pause_retreat"
    }

    fn get_default_keycode(&self) -> u16 {
        Keycode::A
    }
}
