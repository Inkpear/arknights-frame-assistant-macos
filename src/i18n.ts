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
