use cgevents::Keycode;

use crate::touch_core::{action::StepType, definition::ActionDefinition};

const ACTION_STEPS: [StepType; 3] = [
    StepType::ClickCursor,
    StepType::WaitAnimation,
    StepType::ClickSkill,
];

/// Click cursor → wait → click skill.
pub struct QuickSkillAction;

impl ActionDefinition for QuickSkillAction {
    fn get_steps(&self) -> &'static [StepType] {
        &ACTION_STEPS
    }

    fn get_action_id(&self) -> &'static str {
        "quick_skill"
    }

    fn get_default_keycode(&self) -> u16 {
        Keycode::E
    }
}
