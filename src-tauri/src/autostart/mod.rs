use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub fn is_enabled(app: &AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("读取系统自启动状态失败：{error}"))
}

pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|error| format!("修改系统自启动状态失败：{error}"))
}
