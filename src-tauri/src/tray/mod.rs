use crate::{AppState, icon, run_entry_from_tray};
use tauri::{
    Manager,
    menu::{IconMenuItemBuilder, Menu, MenuItemBuilder, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

const ENTRY_PREFIX: &str = "entry:";
const SETTINGS_ID: &str = "settings";
const QUIT_ID: &str = "quit";

pub fn create(app: &mut tauri::App) -> tauri::Result<()> {
    let menu = build_menu(app.handle())?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("MultiBoot")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                SETTINGS_ID => show_settings(app),
                QUIT_ID => app.exit(0),
                _ if id.starts_with(ENTRY_PREFIX) => {
                    run_entry_from_tray(app.clone(), &id[ENTRY_PREFIX.len()..]);
                }
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

pub fn refresh_tray_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    replace_menu(app)
}

fn replace_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let tray = app
        .tray_by_id("main-tray")
        .ok_or_else(|| tauri::Error::AssetNotFound("main tray icon".into()))?;
    tray.set_menu(Some(menu))
}

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    let state = app.state::<AppState>();
    let config = state
        .config
        .read()
        .map_err(|_| tauri::Error::Anyhow(anyhow::anyhow!("配置状态锁已损坏")))?;
    let mut items = config
        .items
        .iter()
        .filter(|item| item.enabled)
        .collect::<Vec<_>>();
    items.sort_by_key(|item| item.order);

    for item in &items {
        let mut builder =
            IconMenuItemBuilder::with_id(format!("{ENTRY_PREFIX}{}", item.id), item.title.as_str());
        if let Some(data_url) = &item.icon {
            match icon::decode_menu_image(data_url) {
                Ok(image) => builder = builder.icon(image),
                Err(error) => log::warn!("忽略条目 {} 的无效图标：{error}", item.id),
            }
        }
        let menu_item = builder.build(app)?;
        menu.append(&menu_item)?;
    }

    if !items.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&MenuItemBuilder::with_id(SETTINGS_ID, "设置…").build(app)?)?;
    menu.append(&MenuItemBuilder::with_id(QUIT_ID, "退出").build(app)?)?;
    Ok(menu)
}

pub fn show_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
