use std::{collections::HashMap, sync::LazyLock};

use cgevents::Keycode;

use crate::{config::AppConfigType, touch_core::action::StepType};

/// A declarative game action: id, steps, default keycode, cursor-restore flag.
pub struct ActionDef {
    pub action_id: &'static str,
    pub steps: &'static [StepType],
    pub default_keycode: u16,
    pub restore_cursor: bool,
}

/// All Regular Operations actions, indexed by action-id.
pub static REGULAR_OPERATIONS_ACTIONS: LazyLock<HashMap<&'static str, &'static ActionDef>> =
    LazyLock::new(|| {
        ACTIONS
            .iter()
            .filter(|a| a.profile == Profile::RegularOperations)
            .map(|a| (a.def.action_id, &a.def))
            .collect()
    });

/// All Garrison Protocol actions, indexed by action-id.
pub static GARRISON_PROTOCOL_ACTIONS: LazyLock<HashMap<&'static str, &'static ActionDef>> =
    LazyLock::new(|| {
        ACTIONS
            .iter()
            .filter(|a| a.profile == Profile::GarrisonProtocol)
            .map(|a| (a.def.action_id, &a.def))
            .collect()
    });

#[derive(Clone, Copy, PartialEq, Eq)]
enum Profile {
    RegularOperations,
    GarrisonProtocol,
}

/// The full action table. Add a new action by appending one entry.
static ACTIONS: &[ActionDefEntry] = &[
    ActionDefEntry {
        profile: Profile::RegularOperations,
        def: ActionDef {
            action_id: "advance_12ms",
            steps: &[
                StepType::ClickLeftPause,
                StepType::WaitMillis(12.0),
                StepType::ClickRightPause,
            ],
            default_keycode: Keycode::R,
            restore_cursor: true,
        },
    },
    ActionDefEntry {
        profile: Profile::RegularOperations,
        def: ActionDef {
            action_id: "advance_33ms",
            steps: &[
                StepType::ClickLeftPause,
                StepType::WaitMillis(31.0),
                StepType::ClickRightPause,
            ],
            default_keycode: Keycode::T,
            restore_cursor: true,
        },
    },
    ActionDefEntry {
        profile: Profile::RegularOperations,
        def: ActionDef {
            action_id: "advance_166ms",
            steps: &[
                StepType::ClickLeftPause,
                StepType::WaitMillis(164.0),
                StepType::ClickRightPause,
            ],
            default_keycode: Keycode::Y,
            restore_cursor: true,
        },
    },
    ActionDefEntry {
        profile: Profile::RegularOperations,
        def: ActionDef {
            action_id: "pause_retreat",
            steps: &[
                StepType::ClickLeftPause,
                StepType::ClickCursor,
                StepType::ClickRightPause,
                StepType::WaitAnimation,
                StepType::ClickRetreat,
            ],
            default_keycode: Keycode::A,
            restore_cursor: true,
        },
    },
    ActionDefEntry {
        profile: Profile::RegularOperations,
        def: ActionDef {
            action_id: "pause_selected",
            steps: &[
                StepType::ClickLeftPause,
                StepType::ClickCursor,
                StepType::ClickRightPause,
            ],
            default_keycode: Keycode::W,
            restore_cursor: true,
        },
    },
    ActionDefEntry {
        profile: Profile::RegularOperations,
        def: ActionDef {
            action_id: "pause_skill",
            steps: &[
                StepType::ClickLeftPause,
                StepType::ClickCursor,
                StepType::ClickRightPause,
                StepType::WaitAnimation,
                StepType::ClickSkill,
            ],
            default_keycode: Keycode::S,
            restore_cursor: true,
        },
    },
    ActionDefEntry {
        profile: Profile::RegularOperations,
        def: ActionDef {
            action_id: "quick_retreat",
            steps: &[
                StepType::ClickCursor,
                StepType::WaitAnimation,
                StepType::ClickRetreat,
            ],
            default_keycode: Keycode::Q,
            restore_cursor: true,
        },
    },
    ActionDefEntry {
        profile: Profile::RegularOperations,
        def: ActionDef {
            action_id: "quick_skill",
            steps: &[
                StepType::ClickCursor,
                StepType::WaitAnimation,
                StepType::ClickSkill,
            ],
            default_keycode: Keycode::E,
            restore_cursor: true,
        },
    },
    ActionDefEntry {
        profile: Profile::RegularOperations,
        def: ActionDef {
            action_id: "switch_pause",
            steps: &[StepType::ClickLeftPause],
            default_keycode: Keycode::SPACE,
            restore_cursor: true,
        },
    },
    ActionDefEntry {
        profile: Profile::RegularOperations,
        def: ActionDef {
            action_id: "switch_speed",
            steps: &[StepType::ClickSpeed],
            default_keycode: Keycode::D,
            restore_cursor: true,
        },
    },
];

/// Wrapper carrying the profile tag alongside the action definition.
struct ActionDefEntry {
    profile: Profile,
    def: ActionDef,
}

/// Get the static action mapping for a profile.
pub fn static_mapping_for(
    profile: &AppConfigType,
) -> &'static HashMap<&'static str, &'static ActionDef> {
    match profile {
        AppConfigType::RegularOperations => &REGULAR_OPERATIONS_ACTIONS,
        AppConfigType::GarrisonProtocol => &GARRISON_PROTOCOL_ACTIONS,
    }
}

/// Build a keycode → Action lookup table from a static mapping and custom keycodes.
///
/// Deterministic conflict resolution: each action resolves to its custom keycode
/// (if any) or its default. When two actions resolve to the same keycode, the
/// one that comes *later* in the sorted action-id order wins, and a warning is
/// logged naming the loser and winner. Sorting removes `HashMap` iteration
/// nondeterminism so the same inputs always produce the same winner.
pub fn build_keycode_map(
    static_mapping: &HashMap<&'static str, &'static ActionDef>,
    custom_keycode: Option<&HashMap<String, u16>>,
) -> HashMap<u16, &'static ActionDef> {
    // Collect (action_id, action, resolved_keycode) and sort by action_id for
    // deterministic conflict ordering.
    let mut entries: Vec<(&'static str, &'static ActionDef, u16)> = static_mapping
        .iter()
        .map(|(action_id, action)| {
            let keycode = custom_keycode
                .and_then(|c| c.get(*action_id))
                .copied()
                .unwrap_or(action.default_keycode);
            (*action_id, *action, keycode)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    let mut map = HashMap::new();
    for (action_id, action, keycode) in entries {
        if let Some(prev) = map.insert(keycode, action)
            && prev.action_id != action_id
        {
            log::warn!(
                "Keycode {} conflict: {} shadowed by {}",
                keycode,
                prev.action_id,
                action_id
            );
        }
    }
    map
}
