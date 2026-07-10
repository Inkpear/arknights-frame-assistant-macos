/**
 * ratio — Per-ratio reset button handler.
 *
 * Ratio inputs are readonly and updated via calibration or backend push.
 */
import { invoke } from "@tauri-apps/api/core";
import { logIpc } from "./debug";
import { t } from "./i18n";
import { $, displayToast } from "./dom";

export function initRatioModule() {
  // Reset a single ratio element to its default
  $("#ratio-grid").addEventListener("click", (e) => {
    const btn = (e.target as HTMLElement).closest("button") as HTMLButtonElement | null;
    if (!btn) return;
    if (btn.dataset.action === "reset-ratio") {
      const type = btn.dataset.id!;
      logIpc("→", "reset_ui_ratio", { ratioType: type });
      displayToast(t("toast.ratioReset", t(`ratio.${type}`)));
      invoke("reset_ui_ratio", { ratioType: type }).catch((err) =>
        console.error("reset_ui_ratio failed:", err)
      );
    }
  });
}
