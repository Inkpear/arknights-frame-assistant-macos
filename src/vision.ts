import { invoke } from "@tauri-apps/api/core";
import { logIpc } from "./debug";
import { t } from "./i18n";
import { displayToast } from "./dom";
import { status } from "./state";

/** Wire the white-diamond recognition card. */
export function initVisionModule() {
  const enabled = document.querySelector<HTMLInputElement>("#vision-enabled");
  enabled?.addEventListener("change", () => {
    const value = enabled.checked;
    logIpc("→", "set_vision_enabled", { enabled: value });
    invoke("set_vision_enabled", { enabled: value }).catch((err) =>
      console.error("set_vision_enabled failed:", err)
    );
    if (value && !status.vision?.permission) {
      displayToast(t("vision.needPermission"), true);
    } else if (value) {
      displayToast(t("toast.visionOn"));
    }
  });

  const dryRun = document.querySelector<HTMLInputElement>("#vision-dry-run");
  dryRun?.addEventListener("change", () => {
    const value = dryRun.checked;
    logIpc("→", "set_vision_dry_run", { enabled: value });
    invoke("set_vision_dry_run", { enabled: value }).catch((err) =>
      console.error("set_vision_dry_run failed:", err)
    );
  });

  const debug = document.querySelector<HTMLInputElement>("#vision-debug");
  debug?.addEventListener("change", () => {
    const value = debug.checked;
    logIpc("→", "set_vision_debug", { enabled: value });
    invoke("set_vision_debug", { enabled: value }).catch((err) =>
      console.error("set_vision_debug failed:", err)
    );
  });

  const request = document.querySelector<HTMLButtonElement>("#vision-request-permission");
  request?.addEventListener("click", () => {
    logIpc("→", "request_screen_capture_permission");
    invoke<boolean>("request_screen_capture_permission")
      .then((granted) => {
        displayToast(
          granted ? t("toast.visionPermissionGranted") : t("toast.visionPermissionDenied"),
          !granted
        );
      })
      .catch((err) => console.error("request_screen_capture_permission failed:", err));
  });
}
