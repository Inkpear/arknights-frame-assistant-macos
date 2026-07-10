/**
 * events — Wires all event listeners and IPC invoke calls.
 *
 * Architecture: User action → invoke(command) → backend → emit("status-changed")
 *               listen("status-changed") → applyRemoteStatus → refreshUI
 *
 * ALL invoke() and listen() calls are guarded by isTauri().
 * In a non-Tauri context (plain browser), the UI renders but IPC is no-op.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { logIpc, logUI } from "./debug";
import { setLang, applyI18n, t } from "./i18n";
import { $, displayToast, isTauri } from "./dom";
import { status, ratios, applyRemoteStatus, AppStatus } from "./state";
import { refreshUI, updateRatioInputs } from "./render";
import { initKeybindModule } from "./keybind";
import { initRatioModule } from "./ratio";
import { initCalibrateModule, reapplyCalibratingState, handleStatusChanged } from "./calibrate";

export async function init() {
  logUI(`init — tauri:${isTauri()}`);

  if (!isTauri()) {
    logUI("non-Tauri mode — IPC disabled, UI is static");
    return;
  }

  // ── Subscribe to push events BEFORE pulling initial state (B3) ──
  // A status-changed fired between get_status and listen would otherwise be lost.
  let initialApplied = false;

  try {
    await listen("status-changed", (event) => {
      const payload = event.payload as AppStatus;
      // Dedup: the first status-changed may duplicate the get_status payload.
      if (initialApplied) {
        const langChanged = applyRemoteStatus(payload);
        if (langChanged) {
          setLang(status.language === "English" ? "zh" : "en");
          applyI18n();
        }
        refreshUI();
        reapplyCalibratingState();
        handleStatusChanged(payload.calibrating_mode_enabled);
        logUI("status-changed → refresh");
      }
    });
  } catch (e) {
    console.error("listen status-changed failed", e);
  }

  try {
    listen("ratio-updated", (event) => {
      const p = event.payload as { ratio_type: string; ratio: [number, number] };
      const r = ratios.find((r) => r.ratioType === p.ratio_type);
      if (r) r.ratio = p.ratio;
      updateRatioInputs();
      logIpc("←", "ratio-updated", p);
    });
  } catch (e) {
    console.error("listen ratio-updated failed", e);
  }

  // ── Pull initial state (listeners are already armed) ──
  try {
    const payload = await invoke<AppStatus>("get_status");
    applyRemoteStatus(payload);
    setLang(status.language === "English" ? "zh" : "en");
    applyI18n();
    refreshUI();
    initialApplied = true;
    logUI("init → get_status applied");
  } catch (e) {
    console.error("get_status failed:", e);
  }

  // ── User actions → backend invoke ──

  // Hotkey toggle
  $("#hotkey-toggle").addEventListener("change", (e) => {
    const enabled = (e.target as HTMLInputElement).checked;
    logIpc("→", "set_hotkey_enabled", { enabled });
    invoke("set_hotkey_enabled", { enabled }).catch((err) =>
      console.error("set_hotkey_enabled failed:", err)
    );
  });

  // Tabs — frontend-only switch (calibrate / regular) or disabled (garrison)
  document.querySelectorAll(".tab").forEach((tabEl) => {
    tabEl.addEventListener("click", () => {
      const tab = tabEl as HTMLButtonElement;
      const tabId = tab.dataset.tab!;

      // Frontend tab switch
      document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
      tab.classList.add("active");
      document.querySelectorAll(".tab-panel").forEach((p) => p.classList.remove("active"));
      const panel = document.querySelector(`#panel-${tabId}`);
      if (panel) panel.classList.add("active");
      // Notify backend of profile change
      if (tabId === "regular") {
        logIpc("→", "switch_profile", { newProfile: "RegularOperations" });
        invoke("switch_profile", { newProfile: "RegularOperations" }).catch((err) =>
          console.error("switch_profile failed:", err)
        );
      } else if (tabId === "garrison") {
        // backend GarrisonProtocol not yet implemented
        displayToast(t("toast.garrisonWip"), true);
      }
      logUI(`tab → ${tabId}`);
    });
  });

  // ── Window drag (manual startDragging for Tauri #15623) ──
  // Only drag when clicking the header area, not buttons/inputs.
  document.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    // Never drag on interactive elements
    if (target.closest("button, input, textarea, select, .no-drag")) return;
    // Only drag on the header drag region
    if (!target.closest(".drag-region")) return;
    e.stopPropagation();
    try {
      e.detail === 2
        ? getCurrentWindow().toggleMaximize()
        : getCurrentWindow().startDragging();
    } catch {
      // not in Tauri — ignore
    }
  });

  // ── Sub-modules ──
  initKeybindModule();
  initRatioModule();
  initCalibrateModule();
}
