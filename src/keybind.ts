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
import { isTauri, $, displayToast, keybindOverrides } from "./state";
import { renderKeybinds } from "./render";

// ── CGKeyCode → display label ──
const KEY_LABELS: Record<number, string> = {
  0x00: "A", 0x01: "S", 0x02: "D", 0x03: "F", 0x04: "H",
  0x05: "G", 0x06: "Z", 0x07: "X", 0x08: "C", 0x09: "V",
  0x0B: "B", 0x0C: "Q", 0x0D: "W", 0x0E: "E", 0x0F: "R",
  0x10: "Y", 0x11: "T",
  0x12: "1", 0x13: "2", 0x14: "3", 0x15: "4", 0x16: "6",
  0x17: "5", 0x18: "=", 0x19: "9", 0x1A: "7", 0x1B: "-",
  0x1C: "8", 0x1D: "0", 0x1E: "]", 0x1F: "O", 0x20: "U",
  0x21: "[", 0x22: "I", 0x23: "P", 0x24: "Return", 0x25: "L",
  0x26: "J", 0x27: "'", 0x28: "K", 0x29: ";", 0x2A: "\\",
  0x2B: ",", 0x2C: "/", 0x2D: "N", 0x2E: "M", 0x2F: ".",
  0x30: "Tab", 0x31: "Space", 0x32: "`", 0x33: "Delete",
  0x35: "Escape", 0x36: "R-Cmd", 0x37: "Cmd", 0x38: "Shift",
  0x39: "CapsLock", 0x3A: "Option", 0x3B: "Ctrl",
  0x7A: "F1", 0x78: "F2", 0x63: "F3", 0x76: "F4",
  0x60: "F5", 0x61: "F6", 0x62: "F7", 0x64: "F8",
  0x65: "F9", 0x6D: "F10", 0x67: "F11", 0x6F: "F12",
  0x7B: "←", 0x7C: "→", 0x7D: "↓", 0x7E: "↑",
};

export function keyLabel(code: number): string {
  return KEY_LABELS[code] ?? String.fromCharCode(code);
}

// ── KeyboardEvent.code → CGKeyCode ──
const KEYCODE_MAP: Record<string, number> = {
  KeyA: 0x00, KeyS: 0x01, KeyD: 0x02, KeyF: 0x03, KeyH: 0x04,
  KeyG: 0x05, KeyZ: 0x06, KeyX: 0x07, KeyC: 0x08, KeyV: 0x09,
  KeyB: 0x0B, KeyQ: 0x0C, KeyW: 0x0D, KeyE: 0x0E, KeyR: 0x0F,
  KeyY: 0x10, KeyT: 0x11,
  Digit1: 0x12, Digit2: 0x13, Digit3: 0x14, Digit4: 0x15, Digit6: 0x16,
  Digit5: 0x17, Equal: 0x18, Digit9: 0x19, Digit7: 0x1A, Minus: 0x1B,
  Digit8: 0x1C, Digit0: 0x1D, BracketRight: 0x1E, KeyO: 0x1F, KeyU: 0x20,
  BracketLeft: 0x21, KeyI: 0x22, KeyP: 0x23, Enter: 0x24, KeyL: 0x25,
  KeyJ: 0x26, Quote: 0x27, KeyK: 0x28, Semicolon: 0x29, Backslash: 0x2A,
  Comma: 0x2B, Slash: 0x2C, KeyN: 0x2D, KeyM: 0x2E, Period: 0x2F,
  Tab: 0x30, Space: 0x31, Backquote: 0x32, Backspace: 0x33,
  Escape: 0x35, ShiftLeft: 0x38, ShiftRight: 0x3C,
  ControlLeft: 0x3B, ControlRight: 0x3E,
  AltLeft: 0x3A, AltRight: 0x3D,
  MetaLeft: 0x37, MetaRight: 0x36,
  F1: 0x7A, F2: 0x78, F3: 0x63, F4: 0x76,
  F5: 0x60, F6: 0x61, F7: 0x62, F8: 0x64,
  F9: 0x65, F10: 0x6D, F11: 0x67, F12: 0x6F,
  ArrowLeft: 0x7B, ArrowRight: 0x7C, ArrowDown: 0x7D, ArrowUp: 0x7E,
};

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
  // Check for duplicate binding
  for (const [actionId, existing] of keybindOverrides) {
    if (existing === kc && actionId !== pendingKeybind.actionId) {
      if (!confirm(t("toast.keyConflict", t(`action.${actionId}`), t(`action.${pendingKeybind.actionId}`)))) {
        closeKeyOverlay();
        e.preventDefault();
        return;
      }
      keybindOverrides.delete(actionId);
      break;
    }
  }
  keybindOverrides.set(pendingKeybind.actionId, kc);
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
  if (!isTauri()) return;
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
