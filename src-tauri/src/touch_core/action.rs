use mado::WindowBounds;
use serde::{Deserialize, Serialize};

use crate::touch_core::{
    definition::ActionDefinition,
    mouse,
    position::{UIPosition, UIRatio},
};

const ANIMATION_WAIT: f64 = 100.0; // Default wait time for animations in milliseconds

/// Context for executing a sequence of mouse actions within the game window.
pub struct ActionContext {
    cursor_position: (f64, f64),
    ui_position: UIPosition,
}

impl ActionContext {
    /// Create an ActionContext, validating the cursor is within the given window bounds.
    pub fn new(bounds: &WindowBounds, ui_ratio: &UIRatio, cursor_position: (f64, f64)) -> Self {
        let ui_position = UIPosition::new(ui_ratio, bounds);
        ActionContext {
            cursor_position,
            ui_position,
        }
    }

    /// Execute all steps in an ActionDefinition.
    pub fn execute_action(&self, action: &'static dyn ActionDefinition) -> anyhow::Result<()> {
        for step in action.get_steps() {
            match step {
                StepType::ClickCursor => {
                    mouse::left_click(self.cursor_position.0, self.cursor_position.1)?;
                }
                StepType::ClickLeftPause => {
                    mouse::left_click(
                        self.ui_position.left_pause.0,
                        self.ui_position.left_pause.1,
                    )?;
                }
                StepType::ClickRightPause => {
                    mouse::left_click(
                        self.ui_position.right_pause.0,
                        self.ui_position.right_pause.1,
                    )?;
                }
                StepType::ClickSkill => {
                    mouse::left_click(self.ui_position.skill.0, self.ui_position.skill.1)?;
                }
                StepType::ClickRetreat => {
                    mouse::left_click(self.ui_position.retreat.0, self.ui_position.retreat.1)?;
                }
                StepType::ClickSpeed => {
                    mouse::left_click(self.ui_position.speed.0, self.ui_position.speed.1)?;
                }
                StepType::WaitAnimation => {
                    spin_sleep::sleep(std::time::Duration::from_secs_f64(ANIMATION_WAIT / 1000.0));
                }
                StepType::WaitMillis(ms) => {
                    spin_sleep::sleep(std::time::Duration::from_secs_f64(*ms / 1000.0));
                }
            }
        }
        if action.get_restore_cursor() {
            mouse::move_to(self.cursor_position.0, self.cursor_position.1)?;
        }
        Ok(())
    }
}

/// A single step in an action sequence.
#[derive(Deserialize, Serialize, Debug)]
pub enum StepType {
    ClickCursor,
    ClickLeftPause,
    ClickRightPause,
    ClickSkill,
    ClickRetreat,
    ClickSpeed,
    WaitAnimation,
    WaitMillis(f64),
}
