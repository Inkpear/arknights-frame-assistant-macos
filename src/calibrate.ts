import { invoke } from "@tauri-apps/api/core";
import { logIpc } from "./debug";
import { t } from "./i18n";
import { isTauri, $, displayToast, ratios, status } from "./state";

/** The ratioType currently being calibrated, or null. */
let activeTarget: string | null = null;

/** Start calibration for a specific ratio type. */
export function startCalibratingItem(ratioType: string) {
  if (activeTarget) {
    invoke("set_calibrating_mode_enabled", { enabled: false }).catch(() => {});
  }
  activeTarget = ratioType;
  logIpc("→", "set_calibrating_target", { target: ratioType });
  if (isTauri()) invoke("set_calibrating_target", { target: ratioType }).catch(() => {});
  logIpc("→", "set_calibrating_mode_enabled", { enabled: true });
  if (isTauri()) invoke("set_calibrating_mode_enabled", { enabled: true }).catch(() => {});
  displayToast(t("toast.calibrateOn"));
}

/** Re-apply .calibrating class after DOM rebuild (called from events.ts after refreshUI). */
export function reapplyCalibratingState() {
  if (!activeTarget || !status.calibrating_mode_enabled) return;
  const btn = document.querySelector<HTMLButtonElement>(
    `[data-action="calibrate-item"][data-id="${activeTarget}"]`
  );
  if (btn) btn.classList.add("calibrating");
}

/**
 * Called from events.ts after each status-changed.
 * Detects calibration completion and shows toast.
 */
export function handleStatusChanged(calibratingEnabled: boolean) {
  if (activeTarget && !calibratingEnabled) {
    // Calibration just completed — toast the updated value
    const r = ratios.find((r) => r.ratioType === activeTarget);
    if (r) {
      displayToast(
        t("toast.ratioUpdatedDetail",
          t(`ratio.${activeTarget}`),
          r.ratio[0].toFixed(3),
          r.ratio[1].toFixed(3)
        )
      );
    }
    activeTarget = null;
  }
}

export function initCalibrateModule() {
  $("#ratio-grid").addEventListener("click", (e) => {
    const btn = (e.target as HTMLElement).closest("button") as HTMLButtonElement | null;
    if (!btn) return;
    if (btn.dataset.action === "calibrate-item") {
      startCalibratingItem(btn.dataset.id!);
    }
  });
}
