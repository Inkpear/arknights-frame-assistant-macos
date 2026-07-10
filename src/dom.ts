/**
 * dom — DOM utilities and Tauri-runtime detection.
 */

/** Shortcut for document.querySelector — throws if element not found. */
export function $(sel: string): HTMLElement {
  const el = document.querySelector(sel);
  if (!el) throw new Error(`Element not found: ${sel}`);
  return el as HTMLElement;
}

/** Show a transient toast. Auto-dismisses after 2.5s; keeps at most 3 visible. */
export function displayToast(msg: string, warning = false) {
  const container = document.querySelector("#toast-container")!;
  const el = document.createElement("div");
  el.className = `toast${warning ? " toast--warning" : ""}`;
  el.textContent = msg;
  container.appendChild(el);
  while (container.children.length > 3) container.firstChild?.remove();
  setTimeout(() => el.remove(), 2500);
}

/** True when running inside a Tauri webview. */
export function isTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}
