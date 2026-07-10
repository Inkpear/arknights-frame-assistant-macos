/**
 * keybind — Keyboard shortcut capture and rebinding.
 *
 * Capture flow:
 *   1. User clicks "修改" / "重绑" button → openKeyOverlay shows modal
 *   2. User presses a key → keyCodeToU16 maps KeyboardEvent.code to CGKeyCode
 *   3. Supported key → saved to keybindOverrides Map → invoke("update_custom_keycode")
 *   4. Unsupported key → toast with specific reason
 *
 * Keycode mapping: KeyboardEvent.code → macOS CGKeyCode (u16).
 * Full map covers A-Z, 0-9, F1-F12, arrows, modifiers, punctuation.
 */
import { invoke } from "@tauri-apps/api/core";
import { logIpc } from "./debug";
import { t } from "./i18n";
import { $, displayToast } from "./dom";
import { keybindOverrides, notifyKeybindsChanged, KEYBIND_DEFS } from "./state";
import { renderKeybinds } from "./render";

// ── Single source of truth: label + KeyboardEvent.code for each CGKeyCode ──
const KEY_TABLE: { code: number; label: string; eventCode?: string }[] = [
  { code: 0x00, label: "A", eventCode: "KeyA" },
  { code: 0x01, label: "S", eventCode: "KeyS" },
  { code: 0x02, label: "D", eventCode: "KeyD" },
  { code: 0x03, label: "F", eventCode: "KeyF" },
  { code: 0x04, label: "H", eventCode: "KeyH" },
  { code: 0x05, label: "G", eventCode: "KeyG" },
  { code: 0x06, label: "Z", eventCode: "KeyZ" },
  { code: 0x07, label: "X", eventCode: "KeyX" },
  { code: 0x08, label: "C", eventCode: "KeyC" },
  { code: 0x09, label: "V", eventCode: "KeyV" },
  { code: 0x0B, label: "B", eventCode: "KeyB" },
  { code: 0x0C, label: "Q", eventCode: "KeyQ" },
  { code: 0x0D, label: "W", eventCode: "KeyW" },
  { code: 0x0E, label: "E", eventCode: "KeyE" },
  { code: 0x0F, label: "R", eventCode: "KeyR" },
  { code: 0x10, label: "Y", eventCode: "KeyY" },
  { code: 0x11, label: "T", eventCode: "KeyT" },
  { code: 0x12, label: "1", eventCode: "Digit1" },
  { code: 0x13, label: "2", eventCode: "Digit2" },
  { code: 0x14, label: "3", eventCode: "Digit3" },
  { code: 0x15, label: "4", eventCode: "Digit4" },
  { code: 0x16, label: "6", eventCode: "Digit6" },
  { code: 0x17, label: "5", eventCode: "Digit5" },
  { code: 0x18, label: "=", eventCode: "Equal" },
  { code: 0x19, label: "9", eventCode: "Digit9" },
  { code: 0x1A, label: "7", eventCode: "Digit7" },
  { code: 0x1B, label: "-", eventCode: "Minus" },
  { code: 0x1C, label: "8", eventCode: "Digit8" },
  { code: 0x1D, label: "0", eventCode: "Digit0" },
  { code: 0x1E, label: "]", eventCode: "BracketRight" },
  { code: 0x1F, label: "O", eventCode: "KeyO" },
  { code: 0x20, label: "U", eventCode: "KeyU" },
  { code: 0x21, label: "[", eventCode: "BracketLeft" },
  { code: 0x22, label: "I", eventCode: "KeyI" },
  { code: 0x23, label: "P", eventCode: "KeyP" },
  { code: 0x24, label: "Return", eventCode: "Enter" },
  { code: 0x25, label: "L", eventCode: "KeyL" },
  { code: 0x26, label: "J", eventCode: "KeyJ" },
  { code: 0x27, label: "'", eventCode: "Quote" },
  { code: 0x28, label: "K", eventCode: "KeyK" },
  { code: 0x29, label: ";", eventCode: "Semicolon" },
  { code: 0x2A, label: "\\", eventCode: "Backslash" },
  { code: 0x2B, label: ",", eventCode: "Comma" },
  { code: 0x2C, label: "/", eventCode: "Slash" },
  { code: 0x2D, label: "N", eventCode: "KeyN" },
  { code: 0x2E, label: "M", eventCode: "KeyM" },
  { code: 0x2F, label: ".", eventCode: "Period" },
  { code: 0x30, label: "Tab", eventCode: "Tab" },
  { code: 0x31, label: "Space", eventCode: "Space" },
  { code: 0x32, label: "`", eventCode: "Backquote" },
  { code: 0x33, label: "Delete", eventCode: "Backspace" },
  { code: 0x35, label: "Escape", eventCode: "Escape" },
  { code: 0x36, label: "R-Cmd", eventCode: "MetaRight" },
  { code: 0x37, label: "Cmd", eventCode: "MetaLeft" },
  { code: 0x38, label: "Shift", eventCode: "ShiftLeft" },
  { code: 0x39, label: "CapsLock" },
  { code: 0x3A, label: "Option", eventCode: "AltLeft" },
  { code: 0x3B, label: "Ctrl", eventCode: "ControlLeft" },
  { code: 0x3C, label: "R-Shift", eventCode: "ShiftRight" },
  { code: 0x3D, label: "R-Option", eventCode: "AltRight" },
  { code: 0x3E, label: "R-Ctrl", eventCode: "ControlRight" },
  { code: 0x7A, label: "F1", eventCode: "F1" },
  { code: 0x78, label: "F2", eventCode: "F2" },
  { code: 0x63, label: "F3", eventCode: "F3" },
  { code: 0x76, label: "F4", eventCode: "F4" },
  { code: 0x60, label: "F5", eventCode: "F5" },
  { code: 0x61, label: "F6", eventCode: "F6" },
  { code: 0x62, label: "F7", eventCode: "F7" },
  { code: 0x64, label: "F8", eventCode: "F8" },
  { code: 0x65, label: "F9", eventCode: "F9" },
  { code: 0x6D, label: "F10", eventCode: "F10" },
  { code: 0x67, label: "F11", eventCode: "F11" },
  { code: 0x6F, label: "F12", eventCode: "F12" },
  { code: 0x7B, label: "←", eventCode: "ArrowLeft" },
  { code: 0x7C, label: "→", eventCode: "ArrowRight" },
  { code: 0x7D, label: "↓", eventCode: "ArrowDown" },
  { code: 0x7E, label: "↑", eventCode: "ArrowUp" },
];

// Derived: CGKeyCode → display label
const KEY_LABELS: Record<number, string> = Object.fromEntries(
  KEY_TABLE.map((k) => [k.code, k.label])
);
// Derived: KeyboardEvent.code → CGKeyCode
const KEYCODE_MAP: Record<string, number> = Object.fromEntries(
  KEY_TABLE.filter((k) => k.eventCode).map((k) => [k.eventCode!, k.code])
);

export function keyLabel(code: number): string {
  return KEY_LABELS[code] ?? String.fromCharCode(code);
}

// Derived: display label → CGKeyCode (for resolving default-key conflicts).
const LABEL_TO_CODE: Record<string, number> = Object.fromEntries(
  KEY_TABLE.map((k) => [k.label, k.code])
);

/** CGKeyCode for an action's default key label, or -1 if unknown. */
function defaultKeycodeFor(actionId: string): number {
  const def = KEYBIND_DEFS.find((d) => d.actionId === actionId);
  return def ? (LABEL_TO_CODE[def.defaultKey] ?? -1) : -1;
}

/** Convert a KeyboardEvent into a CGKeyCode, or -1 if unsupported. */
function keyCodeToU16(e: KeyboardEvent): number {
  return KEYCODE_MAP[e.code] ?? -1;
}

// ── Capture overlay state ──
let pendingKeybind: { actionId: string } | null = null;

function openKeyOverlay(actionId: string) {
  pendingKeybind = { actionId };
  $("#key-overlay").classList.remove("hidden");
  logIpc("→", `rebind:${actionId}`, "overlay open");
}

function closeKeyOverlay() {
  pendingKeybind = null;
  $("#key-overlay").classList.add("hidden");
}

/** Handle a key press while the capture overlay is open. */
function handleKeyCapture(e: KeyboardEvent) {
  if (!pendingKeybind) return;
  const kc = keyCodeToU16(e);
  if (kc < 0) {
    displayToast(t("toast.unsupportedKey"), true);
    return;
  }
  // Check for duplicate binding against custom overrides AND other actions' defaults.
  for (const def of KEYBIND_DEFS) {
    if (def.actionId === pendingKeybind.actionId) continue;
    const otherCustom = keybindOverrides.get(def.actionId);
    const otherDefault = defaultKeycodeFor(def.actionId);
    const otherKc = otherCustom ?? (otherDefault >= 0 ? otherDefault : -1);
    if (otherKc === kc) {
      if (
        !confirm(t("toast.keyConflict", t(`action.${def.actionId}`), t(`action.${pendingKeybind.actionId}`)))
      ) {
        closeKeyOverlay();
        e.preventDefault();
        return;
      }
      // If the conflicting action had a custom override, clear it so the new
      // binding wins; otherwise the conflict is with its default — the backend's
      // build_keycode_map resolves it deterministically (custom wins, sorted).
      if (otherCustom != null) keybindOverrides.delete(def.actionId);
      break;
    }
  }
  keybindOverrides.set(pendingKeybind.actionId, kc);
  notifyKeybindsChanged();
  saveKeybinds();
  renderKeybinds();
  displayToast(t("toast.bound", t(`action.${pendingKeybind.actionId}`), keyLabel(kc)));
  closeKeyOverlay();
  e.preventDefault();
}

/** Block mouse side buttons during capture. */
function handleMouseSideButton(e: MouseEvent) {
  if ((e.button === 3 || e.button === 4) && pendingKeybind) {
    displayToast(t("toast.noSideButton"), true);
    closeKeyOverlay();
    e.preventDefault();
  }
}

/** Send current keybind overrides to backend (full replace, not delta). */
function saveKeybinds() {
  const actions: { actionId: string; keycode: number }[] = [];
  keybindOverrides.forEach((keycode, actionId) => actions.push({ actionId, keycode }));
  logIpc("→", "update_custom_keycode", actions);
  invoke("update_custom_keycode", { actions }).catch((e) =>
    console.error("update_custom_keycode failed:", e)
  );
}

export function initKeybindModule() {
  // Delegate click events within the keybind list
  $("#keybind-list").addEventListener("click", (e) => {
    const btn = (e.target as HTMLElement).closest("button") as HTMLButtonElement | null;
    if (!btn) return;
    const action = btn.dataset.action;
    const id = btn.dataset.id!;
    if (action === "rebind") openKeyOverlay(id);
    if (action === "reset-key") {
      keybindOverrides.delete(id);
      notifyKeybindsChanged();
      saveKeybinds();
      renderKeybinds();
      displayToast(t("toast.restored", t(`action.${id}`)));
    }
  });

  document.addEventListener("keydown", (e) => {
    if (pendingKeybind) handleKeyCapture(e);
  });
  document.addEventListener("mousedown", handleMouseSideButton);
  $("#key-overlay-cancel").addEventListener("click", closeKeyOverlay);
}
