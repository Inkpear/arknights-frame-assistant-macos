use mado::WindowBounds;
use serde::{Deserialize, Serialize};

use crate::touch_core::{
    definition::ActionDef,
    mouse,
    position::{UIPosition, UIRatio},
    vision::{ButtonKind, VisionDecision, VisionRuntime},
};

const ANIMATION_WAIT: f64 = 150.0; // Default wait time for animations in milliseconds

/// Context for executing a sequence of mouse actions within the game window.
pub struct ActionContext {
    cursor_position: (f64, f64),
    ui_position: UIPosition,
    /// When true, skill/retreat come only from white-diamond recognition.
    /// When false, the calibrated UI ratios are used.
    vision_mode: bool,
    vision: Option<VisionRuntime>,
}

impl ActionContext {
    /// Create an ActionContext that uses the configured UI ratios.
    pub fn new(
        frame_bounds: &WindowBounds,
        ui_ratio: &UIRatio,
        cursor_position: (f64, f64),
    ) -> Self {
        Self::new_with_vision(frame_bounds, ui_ratio, cursor_position, false, None)
    }

    /// Create an ActionContext for the selected mode.
    ///
    /// `vision_mode` is the compatibility switch. When enabled, the action uses
    /// only white-diamond recognition: if the runtime is unavailable or no
    /// diamond is found, the button click is skipped (there is no fallback).
    pub fn new_with_vision(
        frame_bounds: &WindowBounds,
        ui_ratio: &UIRatio,
        cursor_position: (f64, f64),
        vision_mode: bool,
        vision: Option<VisionRuntime>,
    ) -> Self {
        ActionContext {
            cursor_position,
            ui_position: UIPosition::new(ui_ratio, frame_bounds),
            vision_mode,
            vision,
        }
    }

    /// Resolve a button from the active mode.
    fn resolve_button(&self, kind: ButtonKind) -> Option<(f64, f64)> {
        if !self.vision_mode {
            return Some(match kind {
                ButtonKind::Skill => self.ui_position.skill,
                ButtonKind::Retreat => self.ui_position.retreat,
            });
        }
        let Some(vision) = self.vision.as_ref() else {
            log::warn!("Vision mode is enabled but unavailable; skipping {kind:?} click");
            return None;
        };
        match vision.resolve(kind) {
            VisionDecision::Click(point) => Some(point),
            VisionDecision::Skip => {
                log::info!("Vision mode: {kind:?} click skipped");
                None
            }
        }
    }

    /// Execute all steps in an action definition.
    pub fn execute_action(&self, action: &'static ActionDef) -> anyhow::Result<()> {
        for step in action.steps {
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
                    if let Some(point) = self.resolve_button(ButtonKind::Skill) {
                        mouse::left_click(point.0, point.1)?;
                    }
                }
                StepType::ClickRetreat => {
                    if let Some(point) = self.resolve_button(ButtonKind::Retreat) {
                        mouse::left_click(point.0, point.1)?;
                    }
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
        if action.restore_cursor {
            std::thread::sleep(std::time::Duration::from_millis(15));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> WindowBounds {
        WindowBounds {
            x: 100.0,
            y: 200.0,
            width: 800.0,
            height: 600.0,
        }
    }

    #[test]
    fn ratio_is_used_when_vision_mode_is_off() {
        let ctx = ActionContext::new(&bounds(), &UIRatio::default(), (500.0, 500.0));
        let expected = UIPosition::new(&UIRatio::default(), &bounds());
        assert_eq!(ctx.resolve_button(ButtonKind::Skill), Some(expected.skill));
        assert_eq!(
            ctx.resolve_button(ButtonKind::Retreat),
            Some(expected.retreat)
        );
    }

    #[test]
    fn vision_mode_without_runtime_skips_the_click() {
        let ctx = ActionContext::new_with_vision(
            &bounds(),
            &UIRatio::default(),
            (500.0, 500.0),
            true,
            None,
        );
        assert!(ctx.resolve_button(ButtonKind::Skill).is_none());
        assert!(ctx.resolve_button(ButtonKind::Retreat).is_none());
    }
}
