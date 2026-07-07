use cgevents::Keycode;

use crate::touch_core::{action::StepType, definition::ActionDefinition};

const ACTION_STEPS: [StepType; 5] = [
    StepType::ClickLeftPause,
    StepType::ClickCursor,
    StepType::ClickRightPause,
    StepType::WaitAnimation,
    StepType::ClickSkill,
];

/// Click left pause → click cursor → click right pause → wait → click skill.
pub struct PauseSkillAction;

impl ActionDefinition for PauseSkillAction {
    fn get_steps(&self) -> &'static [StepType] {
        &ACTION_STEPS
    }

    fn get_action_id(&self) -> &'static str {
        "pause_skill"
    }

    fn get_default_keycode(&self) -> u16 {
        Keycode::S
    }
}
