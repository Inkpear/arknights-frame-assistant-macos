/**
 * state — Central reactive state and data synchronisation.
 *
 * All IPC responses write into the shared `status`, `keybindOverrides`,
 * and `ratios` objects. The render module reads from them directly.
 * No framework — just mutable module-level state.
 */
import { logState } from "./debug";

// ── Types matching backend AppStatusPayload ──

export interface AppStatus {
  hotkey_enabled: boolean;
  hotkey_active: boolean;
  calibrating_mode_enabled: boolean;
  current_profile: string;
  language: string;
  ui_ratio: Record<string, [number, number]> | null;
  regular_operations_keycode?: Record<string, number>;
  garrison_protocol_keycode?: Record<string, number>;
  window: {
    app_name: string | null;
    window_title: string | null;
    bounds: { x: number; y: number; width: number; height: number } | null;
    is_arknights: boolean;
    is_available: boolean;
  };
}

export interface RatioValue {
  ratioType: string; // PascalCase: "LeftPause", "RightPause", ...
  ratio: [number, number];
}

export interface KeybindDef {
  actionId: string;
  defaultKey: string;
}

// ── Static definitions (match backend ActionDefinition) ──

export const KEYBIND_DEFS: KeybindDef[] = [
  { actionId: "advance_12ms", defaultKey: "R" },
  { actionId: "advance_33ms", defaultKey: "T" },
  { actionId: "advance_166ms", defaultKey: "Y" },
  { actionId: "pause_retreat", defaultKey: "A" },
  { actionId: "pause_selected", defaultKey: "W" },
  { actionId: "pause_skill", defaultKey: "S" },
  { actionId: "quick_retreat", defaultKey: "Q" },
  { actionId: "quick_skill", defaultKey: "E" },
  { actionId: "switch_pause", defaultKey: "Space" },
  { actionId: "switch_speed", defaultKey: "D" },
];

export const DEFAULT_RATIOS: RatioValue[] = [
  { ratioType: "LeftPause", ratio: [0.92, 0.10] },
  { ratioType: "RightPause", ratio: [0.96, 0.10] },
  { ratioType: "Skill", ratio: [0.70, 0.65] },
  { ratioType: "Retreat", ratio: [0.47, 0.38] },
  { ratioType: "Speed", ratio: [0.85, 0.10] },
];

// ── Reactive state (mutated by applyRemoteStatus) ──

export const status: AppStatus = {
  hotkey_enabled: false,
  hotkey_active: false,
  calibrating_mode_enabled: false,
  current_profile: "RegularOperations",
  language: "中文",
  ui_ratio: null,
  window: { app_name: null, window_title: null, bounds: null, is_arknights: false, is_available: false },
};

/** actionId → CGKeyCode overrides (only entries that differ from default) */
export const keybindOverrides: Map<string, number> = new Map();

/** Current UI ratio values (synced from backend) */
export const ratios: RatioValue[] = structuredClone(DEFAULT_RATIOS);

// ── Backend key mapping (PascalCase → snake_case, precomputed for O(n) sync) ──
const RATIO_FRONT_TO_BACK: Map<string, string> = new Map([
  ["LeftPause", "left_pause"],
  ["RightPause", "right_pause"],
  ["Skill", "skill"],
  ["Retreat", "retreat"],
  ["Speed", "speed"],
]);

// ── Dirty tracking for incremental rendering (P1) ──
// Bumped whenever keybind/ratio data changes; render.ts compares to detect skips.
export let keybindsVersion = 0;
export let ratiosVersion = 0;

function bumpKeybinds() { keybindsVersion++; }
function bumpRatios() { ratiosVersion++; }

/** Notify that keybind data changed locally (so the next render rebuilds the list). */
export function notifyKeybindsChanged() { bumpKeybinds(); }
/** Notify that ratio data changed locally (so the next render rebuilds the grid). */
export function notifyRatiosChanged() { bumpRatios(); }

/**
 * Apply a full status payload from the backend.
 * Called by the "status-changed" event listener.
 * Returns true if the language changed (caller must re-apply i18n).
 */
export function applyRemoteStatus(payload: AppStatus): boolean {
  logState("status-changed received", {
    hotkey: payload.hotkey_enabled,
    profile: payload.current_profile,
    lang: payload.language,
    window: payload.window.is_available,
  });

  const langChanged = status.language !== payload.language;
  const profileChanged = status.current_profile !== payload.current_profile;
  Object.assign(status, payload);

  // Language change forces a rebuild of keybind/ratio panels: their labels are
  // baked into innerHTML via t(...), so applyI18n() (which only touches
  // [data-i18n]) cannot update them.
  if (langChanged) {
    bumpKeybinds();
    bumpRatios();
  }

  // ── Keybind overrides: pick current-profile map from payload ──
  const keycodes =
    payload.current_profile === "RegularOperations"
      ? payload.regular_operations_keycode
      : payload.garrison_protocol_keycode;
  keybindOverrides.clear();
  if (keycodes) {
    for (const [id, kc] of Object.entries(keycodes)) keybindOverrides.set(id, kc);
  }
  if (profileChanged || keycodes) bumpKeybinds();
  logState("keybinds synced", keybindOverrides.size);

  // ── UI ratios: backend uses snake_case keys, frontend uses PascalCase ──
  if (payload.ui_ratio) {
    const ur = payload.ui_ratio as Record<string, [number, number]>;
    for (const r of ratios) {
      const bk = RATIO_FRONT_TO_BACK.get(r.ratioType);
      if (bk && ur[bk]) r.ratio = ur[bk];
    }
    bumpRatios();
    logState("ratios synced", ratios.map((r) => `${r.ratioType}:${r.ratio}`));
  }

  return langChanged;
}
