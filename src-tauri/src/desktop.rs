use std::sync::LazyLock;

use tauri::{
    Manager,
    image::Image,
    menu::{MenuBuilder, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
};

use crate::state::AppState;

pub const TRAY_ID: &str = "main-tray";

pub static ICON_STOPPED: LazyLock<Image> = LazyLock::new(|| {
    Image::from_bytes(include_bytes!("../icons/icon_stopped.png"))
        .expect("Failed to load icon_stopped.png")
});

pub static ICON_RUNNING: LazyLock<Image> = LazyLock::new(|| {
    Image::from_bytes(include_bytes!("../icons/icon_running.png"))
        .expect("Failed to load icon_running.png")
});

pub struct TrayMenuItemHandles {
    toggle_hotkey_item: MenuItem<tauri::Wry>,
    lang_item: MenuItem<tauri::Wry>,
    show_hide_item: MenuItem<tauri::Wry>,
    quit_item: MenuItem<tauri::Wry>,
    tray_icon: TrayIcon,
}

impl TrayMenuItemHandles {
    pub fn new(app: &tauri::App) -> tauri::Result<Self> {
        let show_hide_item = MenuItem::with_id(app, "show_hide", "Show", true, None::<&str>)?;
        let toggle_hotkey_item =
            MenuItem::with_id(app, "toggle_hotkey", "Enable Hotkey", true, None::<&str>)?;
        let lang_item = MenuItem::with_id(app, "lang", "中文", true, None::<&str>)?;
        let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

        let separator = PredefinedMenuItem::separator(app)?;

        let menu = MenuBuilder::new(app)
            .items(&[
                &show_hide_item,
                &toggle_hotkey_item,
                &lang_item,
                &separator,
                &quit_item,
            ])
            .build()?;

        TrayIconBuilder::with_id(TRAY_ID)
            .icon(ICON_STOPPED.clone())
            .menu(&menu)
            .on_menu_event(move |app_handle, event| {
                let state = app_handle.state::<AppState>();
                match event.id.as_ref() {
                    "show_hide" => {
                        tauri::async_runtime::block_on(async { state.toggle_window().await })
                    }
                    "toggle_hotkey" => {
                        let enabled = !state.is_hotkey_enabled();
                        tauri::async_runtime::block_on(async {
                            state.switch_hotkey_enabled(enabled).await.ok();
                        });
                    }
                    "lang" => {
                        tauri::async_runtime::block_on(async {
                            state.toggle_language().await.ok();
                        });
                    }
                    "quit" => {
                        let app = app_handle.clone();
                        state.shutdown().ok();
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        app.exit(0);
                    }
                    _ => {}
                }
            })
            .build(app)?;

        let tray_icon = app
            .tray_by_id(TRAY_ID)
            .ok_or(tauri::Error::Anyhow(anyhow::anyhow!(
                "Failed to get tray icon by ID"
            )))?;

        Ok(Self {
            toggle_hotkey_item,
            lang_item,
            show_hide_item,
            quit_item,
            tray_icon,
        })
    }

    pub fn update_tray_status(
        &self,
        is_english: bool,
        is_hotkey_enabled: bool,
        is_window_available: bool,
        is_window_visible: bool,
    ) -> tauri::Result<()> {
        let toggle_hotkey_label = toggle_hotkey_label(is_hotkey_enabled, is_english);
        let show_hide_label = show_hide_label(is_window_visible, is_english);
        let lang_label = if is_english { "中文" } else { "English" };
        let quit_label = if is_english { "Quit" } else { "退出" };

        if is_hotkey_enabled && is_window_available {
            self.tray_icon.set_icon(Some(ICON_RUNNING.clone()))?;
        } else {
            self.tray_icon.set_icon(Some(ICON_STOPPED.clone()))?;
        }
        if let Ok(curr) = self.toggle_hotkey_item.text()
            && curr != toggle_hotkey_label
        {
            self.toggle_hotkey_item.set_text(toggle_hotkey_label)?;
        }
        if let Ok(curr) = self.lang_item.text()
            && curr != lang_label
        {
            self.lang_item.set_text(lang_label)?;
        }
        if let Ok(curr) = self.show_hide_item.text()
            && curr != show_hide_label
        {
            self.show_hide_item.set_text(show_hide_label)?;
        }
        if let Ok(curr) = self.quit_item.text()
            && curr != quit_label
        {
            self.quit_item.set_text(quit_label)?;
        }
        Ok(())
    }
}

fn toggle_hotkey_label(is_enabled: bool, is_english: bool) -> &'static str {
    if is_enabled {
        if is_english {
            "Disable Hotkey"
        } else {
            "禁用热键"
        }
    } else {
        if is_english {
            "Enable Hotkey"
        } else {
            "启用热键"
        }
    }
}

fn show_hide_label(is_window_visible: bool, is_english: bool) -> &'static str {
    if is_window_visible {
        if is_english {
            "Hide Window"
        } else {
            "隐藏窗口"
        }
    } else {
        if is_english {
            "Show Window"
        } else {
            "显示窗口"
        }
    }
}
