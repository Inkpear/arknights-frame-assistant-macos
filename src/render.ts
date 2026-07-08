/**
 * render — All DOM rendering functions.
 *
 * These read directly from the shared state (status, keybindOverrides, ratios)
 * and replace innerHTML of their respective containers. No virtual DOM —
 * straightforward imperative rendering for a small UI surface.
 */
import { logUI } from "./debug";
import { t } from "./i18n";
import { $, KEYBIND_DEFS, keybindOverrides, ratios, status } from "./state";
import { keyLabel } from "./keybind";

/** Full re-render of all dynamic sections. */
export function refreshUI() {
  renderFooter();
  renderHeader();
  renderKeybinds();
  renderRatios();
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
  logUI("render footer");
}

/** Header: profile badge + hotkey toggle. */
export function renderHeader() {
  const badge = $("#profile-badge");
  const toggle = $("#hotkey-toggle") as HTMLInputElement;
  badge.textContent = t(`profile.${status.current_profile}`);
  toggle.checked = status.hotkey_enabled;
}

/** Keybind list: each action shows its label + current key + rebind/reset buttons. */
export function renderKeybinds() {
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
  logUI("render keybinds");
}

/** Ratio grid: X/Y readonly inputs + per-element calibrate + reset buttons. */
export function renderRatios() {
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
  logUI("render ratios");
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

  logUI("render tabs");
}
