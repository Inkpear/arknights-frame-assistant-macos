/**
 * i18n — Chinese / English translation system.
 *
 * Usage: t("key") → localized string.
 * Language is toggled via setLang(), which returns true if the language changed.
 * applyI18n() scans the DOM for [data-i18n] attributes and replaces textContent.
 */
type Lang = "zh" | "en";
type Dict = Record<string, string>;

const zh: Dict = {
  "status.noWindow": "未跟踪窗口",
  "profile.RegularOperations": "常规作战",
  "profile.GarrisonProtocol": "卫戍协议",
  "label.hotkey": "热键",
  "section.keybinds": "快捷键",
  "section.calibrate": "UI校准",
  "section.calibration": "UI 校准",
  "garrison.placeholder": "卫戍协议尚未实现",
  "btn.calibrateItem": "校准",
  "btn.cancel": "取消",
  "btn.rebind": "修改",
  "btn.rebindAgain": "重绑",
  "btn.restore": "恢复",
  "overlay.prompt": "按下要绑定的按键…",
  "overlay.noSideButton": "不支持鼠标侧键",
  "toast.noSideButton": "不支持鼠标侧键",
  "toast.unsupportedKey": "不支持此按键",
  "toast.bound": '"{0}" 已绑定 {1}',
  "toast.keyConflict": '按键已被 "{0}" 使用，覆盖为 "{1}" 吗？',
  "toast.restored": '"{0}" 已恢复默认',
  "toast.ratioUpdatedDetail": '"{0}" 已更新为 ({1}, {2})',
  "toast.ratioReset": '"{0}" 已恢复默认',
  "toast.calibrateOn": "校准模式已开启，在游戏窗口按 Space 记录位置",
  "action.advance_12ms": "前进12ms",
  "action.advance_33ms": "前进33ms",
  "action.advance_166ms": "前进166ms",
  "action.pause_retreat": "暂停撤退",
  "action.pause_selected": "暂停选中",
  "action.pause_skill": "暂停技能",
  "action.quick_retreat": "快速撤退",
  "action.quick_skill": "快速技能",
  "action.switch_pause": "切换暂停",
  "action.switch_speed": "切换倍速",
  "ratio.LeftPause": "左暂停",
  "ratio.RightPause": "右暂停",
  "ratio.Skill": "技能",
  "ratio.Retreat": "撤退",
  "ratio.Speed": "速度",
  "section.vision": "白菱形识别（兼容模式）",
  "vision.hint": "仅对快速技能/快速撤退生效：启用后会保持游戏窗口的捕获流（仅该窗口）。按快捷键后等待约 150ms 让选中干员的镜头动画结束，再取一帧识别并点击；未识别到菱形时不点击（不回退）。关闭开关时使用 UI 比例校准。首次使用需要授予屏幕录制权限；60 秒无操作后停止捕获。执行快速操作时设置窗口会自动隐藏，避免挡住游戏点击。",
  "vision.permissionLabel": "屏幕录制权限",
  "vision.permissionGranted": "已授权",
  "vision.permissionDenied": "未授权",
  "vision.requestPermission": "请求权限",
  "vision.permissionHint": "授权后通常需要重启 App 才能生效。",
  "vision.lastResultLabel": "最近识别",
  "vision.lastOk": "成功 · {0}ms · 置信度 {1}",
  "vision.lastFail": "失败：{0}",
  "vision.dryRunPrefix": "[试运行]",
  "vision.enabled": "兼容模式（白菱形识别）",
  "vision.dryRun": "只识别不点击（试运行）",
  "vision.debug": "保存调试截图",
  "vision.needPermission": "需要屏幕录制权限；未授权时兼容模式不会点击技能/撤退。关闭开关可继续使用 UI 比例。",
  "vision.errPermission": "屏幕录制权限被拒绝",
  "vision.errWindow": "找不到游戏窗口",
  "vision.errCapture": "窗口截图失败",
  "vision.errNoDiamond": "未识别到白菱形",
  "vision.errLowConfidence": "识别置信度不足",
  "vision.errBounds": "按钮位置超出游戏区域",
  "toast.visionOn": "兼容模式已开启：快速技能/撤退使用白菱形识别，未识别时不点击",
  "toast.visionPermissionGranted": "已获得屏幕录制权限；如仍无法截图，请重启 App",
  "toast.visionPermissionDenied": "未获得屏幕录制权限",
};

const en: Dict = {
  "status.noWindow": "No window tracked",
  "profile.RegularOperations": "Regular Ops",
  "profile.GarrisonProtocol": "Garrison Protocol",
  "label.hotkey": "Hotkey",
  "section.keybinds": "Keybinds",
  "section.calibrate": "UI Calib",
  "section.calibration": "UI Calibration",
  "garrison.placeholder": "Garrison Protocol not yet implemented",
  "btn.calibrateItem": "Calibrate",
  "btn.cancel": "Cancel",
  "btn.rebind": "Change",
  "btn.rebindAgain": "Rebind",
  "btn.restore": "Restore",
  "overlay.prompt": "Press a key to bind…",
  "overlay.noSideButton": "Mouse side buttons not supported",
  "toast.noSideButton": "Mouse side buttons not supported",
  "toast.unsupportedKey": "Key not supported",
  "toast.bound": '"{0}" bound to {1}',
  "toast.keyConflict": 'Key already used by "{0}", overwrite with "{1}"?',
  "toast.restored": '"{0}" restored to default',
  "toast.ratioUpdatedDetail": '"{0}" updated to ({1}, {2})',
  "toast.ratioReset": '"{0}" restored to default',
  "toast.calibrateOn": "Calibration mode on — press Space in game window to capture",
  "action.advance_12ms": "Advance 12ms",
  "action.advance_33ms": "Advance 33ms",
  "action.advance_166ms": "Advance 166ms",
  "action.pause_retreat": "Pause Retreat",
  "action.pause_selected": "Pause Selected",
  "action.pause_skill": "Pause Skill",
  "action.quick_retreat": "Quick Retreat",
  "action.quick_skill": "Quick Skill",
  "action.switch_pause": "Toggle Pause",
  "action.switch_speed": "Toggle Speed",
  "ratio.LeftPause": "Left Pause",
  "ratio.RightPause": "Right Pause",
  "ratio.Skill": "Skill",
  "ratio.Retreat": "Retreat",
  "ratio.Speed": "Speed",
  "section.vision": "White Diamond (Compatibility)",
  "vision.hint": "Only affects quick skill/retreat: while enabled, keeps a capture stream of the game window (that window only). After the hotkey it waits ~150ms for the operator-selection camera animation to finish, then captures one frame, detects and clicks; if no diamond is found it does not click (no fallback). Turn the switch off to use the UI ratios. Screen-recording permission is required; the stream stops after 60s of inactivity. The settings window hides automatically during quick actions so it cannot intercept clicks.",
  "vision.permissionLabel": "Screen Recording",
  "vision.permissionGranted": "Granted",
  "vision.permissionDenied": "Not granted",
  "vision.requestPermission": "Request",
  "vision.permissionHint": "Restart the app after granting permission for it to take effect.",
  "vision.lastResultLabel": "Last result",
  "vision.lastOk": "OK · {0}ms · confidence {1}",
  "vision.lastFail": "Failed: {0}",
  "vision.dryRunPrefix": "[dry-run]",
  "vision.enabled": "Compatibility mode (white diamond)",
  "vision.dryRun": "Detect only (dry-run)",
  "vision.debug": "Save debug frames",
  "vision.needPermission": "Screen-recording permission is required; the compatibility mode will not click until it is granted. Turn the switch off to use the UI ratios.",
  "vision.errPermission": "screen-recording permission denied",
  "vision.errWindow": "game window not found",
  "vision.errCapture": "window capture failed",
  "vision.errNoDiamond": "white diamond not found",
  "vision.errLowConfidence": "confidence too low",
  "vision.errBounds": "button position outside the game area",
  "toast.visionOn": "Compatibility mode enabled: quick skill/retreat use white-diamond recognition; no click if it is not found",
  "toast.visionPermissionGranted": "Screen-recording permission granted; restart the app if capture still fails",
  "toast.visionPermissionDenied": "Screen-recording permission not granted",
};

const DICTS: Record<Lang, Dict> = { zh, en };
let currentLang: Lang = "zh";

export function t(key: string, ...args: string[]): string {
  let s = DICTS[currentLang][key] ?? key;
  args.forEach((a, i) => (s = s.replace(`{${i}}`, a)));
  return s;
}

export function setLang(lang: Lang) {
  if (lang !== currentLang) {
    currentLang = lang;
    return true;
  }
  return false;
}

export function applyI18n() {
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = (el as HTMLElement).dataset.i18n!;
    el.textContent = t(key);
  });
}
