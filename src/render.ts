/**
 * render — All DOM rendering functions.
 *
 * These read directly from the shared state (status, keybindOverrides, ratios)
 * and replace innerHTML of their respective containers. No virtual DOM —
 * straightforward imperative rendering for a small UI surface.
 */
import { logUI } from "./debug";
import { t } from "./i18n";
import { $ } from "./dom";
import {
  DEFAULT_VISION,
  KEYBIND_DEFS,
  keybindOverrides,
  ratios,
  status,
  keybindsVersion,
  ratiosVersion,
} from "./state";
import { keyLabel } from "./keybind";

// Last-rendered versions; when unchanged, the rebuild is skipped (P1).
let lastKeybindsVersion = -1;
let lastRatiosVersion = -1;

/** Full re-render of dynamic sections (incremental where data is unchanged). */
export function refreshUI() {
  renderFooter();
  renderHeader();
  renderKeybinds();
  renderRatios();
  updateVisionStatus();
  renderTabs();
  logUI("refresh complete");
}

/** Footer: dot + window info. */
export function renderFooter() {
  const dot = $(".status-dot");
  const text = $("#status-text");
  dot.classList.toggle("active", status.hotkey_active);

  if (status.window.is_available && status.window.bounds) {
    const b = status.window.bounds as { x: number; y: number; width: number; height: number };
    const app = status.window.app_name ?? "???";
    const pos = `${Math.round(b.x)},${Math.round(b.y)}-${Math.round(b.width)}×${Math.round(b.height)}`;
    const title = status.window.window_title ? `: ${status.window.window_title}` : "";
    text.textContent = `${app}(${pos})${title}`;
  } else {
    text.textContent = status.window.is_arknights ? "…" : t("status.noWindow");
  }
}

/** Header: profile badge + hotkey toggle. */
export function renderHeader() {
  const badge = $("#profile-badge");
  const toggle = $("#hotkey-toggle") as HTMLInputElement;
  badge.textContent = t(`profile.${status.current_profile}`);
  toggle.checked = status.hotkey_enabled;
}

/** Keybind list: rebuilt only when keybind data changed. Preserves focus. */
export function renderKeybinds() {
  if (lastKeybindsVersion === keybindsVersion) return;
  lastKeybindsVersion = keybindsVersion;
  const list = $("#keybind-list");
  list.innerHTML = KEYBIND_DEFS.map((d) => {
    const custom = keybindOverrides.get(d.actionId);
    return `
    <li class="keybind-item">
      <span class="keybind-item__name">${t(`action.${d.actionId}`)}</span>
      <span class="keybind-item__key">${custom != null ? keyLabel(custom) : d.defaultKey}</span>
      <span class="keybind-item__actions">
        <button class="btn btn--sm no-drag" data-action="rebind" data-id="${d.actionId}">
          ${custom != null ? t("btn.rebindAgain") : t("btn.rebind")}
        </button>
        ${custom != null ? `<button class="btn btn--sm btn--secondary no-drag" data-action="reset-key" data-id="${d.actionId}">${t("btn.restore")}</button>` : ""}
      </span>
    </li>`;
  }).join("");
}

/** Ratio grid: rebuilt only when ratio data changed. */
export function renderRatios() {
  if (lastRatiosVersion === ratiosVersion) return;
  lastRatiosVersion = ratiosVersion;
  const grid = $("#ratio-grid");
  grid.innerHTML = ratios
    .map(
      (r) => `
    <div class="ratio-item">
      <span class="ratio-item__label">${t(`ratio.${r.ratioType}`)}</span>
      <span class="ratio-item__inputs no-drag">
        <div class="ratio-item__input-wrapper">
          <span class="ratio-item__axis-label">X</span>
          <input class="ratio-item__input" data-ratio="${r.ratioType}" data-axis="0"
                 value="${r.ratio[0].toFixed(3)}" readonly />
        </div>
        <div class="ratio-item__input-wrapper">
          <span class="ratio-item__axis-label">Y</span>
          <input class="ratio-item__input" data-ratio="${r.ratioType}" data-axis="1"
                 value="${r.ratio[1].toFixed(3)}" readonly />
        </div>
        <button class="btn btn--sm no-drag" data-action="calibrate-item" data-id="${r.ratioType}">${t("btn.calibrateItem")}</button>
        <button class="btn btn--sm btn--secondary no-drag" data-action="reset-ratio" data-id="${r.ratioType}" title="⟲">⟲</button>
      </span>
    </div>`
    )
    .join("");
}

/** Update ratio input values in-place without recreating DOM. */
export function updateRatioInputs() {
  for (const r of ratios) {
    const xInp = document.querySelector<HTMLInputElement>(`.ratio-item__input[data-ratio="${r.ratioType}"][data-axis="0"]`);
    const yInp = document.querySelector<HTMLInputElement>(`.ratio-item__input[data-ratio="${r.ratioType}"][data-axis="1"]`);
    if (xInp) xInp.value = r.ratio[0].toFixed(3);
    if (yInp) yInp.value = r.ratio[1].toFixed(3);
  }
}

/** Null-safe status text update. */
function setStatusText(selector: string, text: string) {
  const el = document.querySelector<HTMLElement>(selector);
  if (el) el.textContent = text;
}

/** White-diamond recognition: update the static card in place. */
export function updateVisionStatus() {
  const v = status.vision ?? DEFAULT_VISION;
  const permission = document.querySelector<HTMLElement>("#vision-permission");
  if (permission) {
    permission.textContent = v.permission ? t("vision.permissionGranted") : t("vision.permissionDenied");
    permission.classList.toggle("vision-permission--ok", v.permission);
    permission.classList.toggle("vision-permission--denied", !v.permission);
  }
  const request = document.querySelector<HTMLButtonElement>("#vision-request-permission");
  if (request) request.disabled = v.permission;

  const enabled = document.querySelector<HTMLInputElement>("#vision-enabled");
  if (enabled) enabled.checked = v.enabled;
  const dryRun = document.querySelector<HTMLInputElement>("#vision-dry-run");
  if (dryRun) dryRun.checked = v.dry_run;
  const debug = document.querySelector<HTMLInputElement>("#vision-debug");
  if (debug) debug.checked = v.debug_capture;

  let result = "—";
  if (v.last_ok === true) {
    result = t(
      "vision.lastOk",
      String(v.last_latency_ms ?? 0),
      (v.last_confidence ?? 0).toFixed(2)
    );
  } else if (v.last_ok === false) {
    result = t("vision.lastFail", localizeVisionReason(v.last_reason));
  }
  if (v.dry_run && v.last_ok != null) {
    result = `${t("vision.dryRunPrefix")} ${result}`;
  }
  setStatusText("#vision-last-result", result);

  const error = document.querySelector<HTMLElement>("#vision-error");
  if (error) {
    error.textContent = v.enabled && !v.permission ? t("vision.needPermission") : "";
  }
}

function localizeVisionReason(raw: string | null): string {
  switch (raw) {
    case "permission_denied":
      return t("vision.errPermission");
    case "window_not_found":
      return t("vision.errWindow");
    case "capture_failed":
      return t("vision.errCapture");
    case "low_confidence":
      return t("vision.errLowConfidence");
    case "button_out_of_bounds":
      return t("vision.errBounds");
    case "no_diamond":
    default:
      return t("vision.errNoDiamond");
  }
}

/** Tab bar active state — garrison follows profile, calibrate/regular are frontend-driven. */
export function renderTabs() {
  const calTab = document.querySelector<HTMLButtonElement>('.tab[data-tab="calibrate"]');
  // Don't override calibrate tab if user is actively using it
  if (calTab?.classList.contains("active")) return;

  const regTab = document.querySelector<HTMLButtonElement>('.tab[data-tab="regular"]');
  const garTab = document.querySelector<HTMLButtonElement>('.tab[data-tab="garrison"]');
  const regPanel = document.querySelector("#panel-regular");
  const garPanel = document.querySelector("#panel-garrison");

  const isRegular = status.current_profile === "RegularOperations";
  regTab?.classList.toggle("active", isRegular);
  garTab?.classList.toggle("active", !isRegular);

  // Keep panels in sync — also hide calibrate panel
  const calPanel = document.querySelector("#panel-calibrate");
  if (regPanel) regPanel.classList.toggle("active", isRegular);
  if (garPanel) garPanel.classList.toggle("active", !isRegular);
  if (calPanel) calPanel.classList.remove("active");
}
