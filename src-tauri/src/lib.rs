mod autostart;
mod command;
mod config;
mod file_replace;
mod icon;
mod import_export;
#[cfg(target_os = "linux")]
mod linux_window;
mod tray;

use std::sync::RwLock;

use command::CommandResult;
use config::{AppConfig, CommandItem, ConfigSnapshot};
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use uuid::Uuid;

struct AppState {
    config: RwLock<AppConfig>,
    config_path: std::path::PathBuf,
    startup_warnings: Vec<String>,
}

#[tauri::command]
fn get_config(state: tauri::State<'_, AppState>) -> Result<ConfigSnapshot, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "配置状态锁已损坏".to_string())?
        .clone();

    Ok(ConfigSnapshot {
        config,
        path: state.config_path.display().to_string(),
        warnings: state.startup_warnings.clone(),
    })
}

#[tauri::command]
fn execute_entry(id: String, state: tauri::State<'_, AppState>) -> Result<CommandResult, String> {
    let entry = state
        .config
        .read()
        .map_err(|_| "配置状态锁已损坏".to_string())?
        .items
        .iter()
        .find(|item| item.id.to_string() == id)
        .cloned()
        .ok_or_else(|| format!("未找到命令条目：{id}"))?;

    log::info!("执行命令条目 {} ({})", entry.id, entry.title);
    let result = command::execute_command(&entry.command);
    if result.success {
        log::info!(
            "命令条目 {} 执行成功，退出码 {:?}",
            entry.id,
            result.exit_code
        );
    } else {
        log::error!(
            "命令条目 {} 执行失败，退出码 {:?}",
            entry.id,
            result.exit_code
        );
    }
    Ok(result)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemInput {
    id: Option<Uuid>,
    title: String,
    command: String,
    enabled: bool,
    icon: Option<String>,
}

fn update_config<F>(app: &tauri::AppHandle, update: F) -> Result<ConfigSnapshot, String>
where
    F: FnOnce(&mut AppConfig) -> Result<(), String>,
{
    let state = app.state::<AppState>();
    let snapshot = {
        let mut config = state
            .config
            .write()
            .map_err(|_| "配置状态锁已损坏".to_string())?;
        let previous = config.clone();
        update(&mut config)?;
        config.items.sort_by_key(|item| item.order);
        for (index, item) in config.items.iter_mut().enumerate() {
            item.order = index as u32;
        }
        config::validate(&config)?;
        if let Err(error) = config::write_config(&state.config_path, &config) {
            *config = previous;
            return Err(format!("保存配置失败：{error}"));
        }
        config.clone()
    };

    tray::refresh_tray_menu(app).map_err(|error| format!("刷新托盘菜单失败：{error}"))?;
    Ok(ConfigSnapshot {
        config: snapshot,
        path: state.config_path.display().to_string(),
        warnings: state.startup_warnings.clone(),
    })
}

#[tauri::command]
fn save_entry(app: tauri::AppHandle, input: ItemInput) -> Result<ConfigSnapshot, String> {
    let title = input.title.trim().to_string();
    let command = input.command.trim().to_string();
    let icon = input
        .icon
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(icon::normalize_data_url)
        .transpose()?;
    update_config(&app, move |config| {
        if let Some(id) = input.id {
            let item = config
                .items
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or_else(|| format!("未找到命令条目：{id}"))?;
            item.title = title;
            item.command = command;
            item.enabled = input.enabled;
            item.icon = icon;
        } else {
            config.items.push(CommandItem {
                id: Uuid::new_v4(),
                title,
                command,
                enabled: input.enabled,
                order: config.items.len() as u32,
                icon,
            });
        }
        Ok(())
    })
}

#[tauri::command]
fn delete_entry(app: tauri::AppHandle, id: Uuid) -> Result<ConfigSnapshot, String> {
    update_config(&app, move |config| {
        let before = config.items.len();
        config.items.retain(|item| item.id != id);
        if config.items.len() == before {
            return Err(format!("未找到命令条目：{id}"));
        }
        Ok(())
    })
}

#[tauri::command]
fn move_entry(
    app: tauri::AppHandle,
    id: Uuid,
    direction: String,
) -> Result<ConfigSnapshot, String> {
    update_config(&app, move |config| {
        config.items.sort_by_key(|item| item.order);
        let index = config
            .items
            .iter()
            .position(|item| item.id == id)
            .ok_or_else(|| format!("未找到命令条目：{id}"))?;
        let target = match direction.as_str() {
            "up" if index > 0 => index - 1,
            "down" if index + 1 < config.items.len() => index + 1,
            "up" | "down" => return Ok(()),
            _ => return Err("排序方向必须是 up 或 down".to_string()),
        };
        config.items.swap(index, target);
        for (position, item) in config.items.iter_mut().enumerate() {
            item.order = position as u32;
        }
        Ok(())
    })
}

#[tauri::command]
fn set_entry_enabled(
    app: tauri::AppHandle,
    id: Uuid,
    enabled: bool,
) -> Result<ConfigSnapshot, String> {
    update_config(&app, move |config| {
        let item = config
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or_else(|| format!("未找到命令条目：{id}"))?;
        item.enabled = enabled;
        Ok(())
    })
}

#[tauri::command]
fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    autostart::is_enabled(&app)
}

#[tauri::command]
fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    autostart::set_enabled(&app, enabled)?;
    let state = app.state::<AppState>();
    let mut config = state
        .config
        .write()
        .map_err(|_| "配置状态锁已损坏".to_string())?;
    let previous = config.settings.autostart;
    config.settings.autostart = enabled;
    if let Err(error) = config::write_config(&state.config_path, &config) {
        config.settings.autostart = previous;
        let _ = autostart::set_enabled(&app, previous);
        return Err(format!("保存自启动设置失败：{error}"));
    }
    let actual = autostart::is_enabled(&app)?;
    log::info!("自启动状态已设置为 {actual}");
    Ok(actual)
}

#[tauri::command]
fn export_config(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let config = state
        .config
        .read()
        .map_err(|_| "配置状态锁已损坏".to_string())?;
    import_export::export(std::path::Path::new(&path), &config)?;
    log::info!("已导出 {} 个命令条目", config.items.len());
    Ok(())
}

#[tauri::command]
fn import_config(
    app: tauri::AppHandle,
    path: String,
    mode: String,
) -> Result<ConfigSnapshot, String> {
    let imported = import_export::read(std::path::Path::new(&path))?;
    let imported_count = imported.len();
    let mode_for_log = mode.clone();
    update_config(&app, move |config| {
        config.items = match mode.as_str() {
            "append" => import_export::merge(&config.items, imported),
            "replace" => imported,
            _ => return Err("导入模式必须是 append 或 replace".to_string()),
        };
        Ok(())
    })
    .inspect(|_| {
        log::info!("已通过 {mode_for_log} 模式导入 {imported_count} 个命令条目");
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log::info!("检测到重复启动，转到现有设置窗口");
            tray::show_settings(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            log::info!("MultiBoot 启动");
            let (config, config_path, startup_warnings) = config::load_or_create(app.handle())?;
            if config.settings.autostart
                && let Err(error) = autostart::set_enabled(app.handle(), true)
            {
                log::error!("默认启用自启动失败：{error}");
            }
            app.manage(AppState {
                config: RwLock::new(config),
                config_path,
                startup_warnings,
            });
            tray::create(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(target_os = "linux")]
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::Focused(true))
                && let Err(error) = linux_window::refresh_decorations(window)
            {
                log::warn!("刷新 Linux 设置窗口标题栏失败：{error}");
            }

            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            execute_entry,
            save_entry,
            delete_entry,
            move_entry,
            set_entry_enabled,
            get_autostart_enabled,
            set_autostart_enabled,
            export_config,
            import_config
        ])
        .run(tauri::generate_context!())
        .expect("运行 MultiBoot 时发生致命错误");
}

fn run_entry_from_tray(app: tauri::AppHandle, id: &str) {
    let entry = {
        let state = app.state::<AppState>();
        let Ok(config) = state.config.read() else {
            return;
        };
        config
            .items
            .iter()
            .find(|item| item.id.to_string() == id)
            .cloned()
    };

    let Some(entry) = entry else {
        return;
    };

    std::thread::spawn(move || {
        log::info!("从托盘执行命令条目 {} ({})", entry.id, entry.title);
        let result = command::execute_command(&entry.command);
        if result.success {
            log::info!(
                "命令条目 {} 执行成功，退出码 {:?}",
                entry.id,
                result.exit_code
            );
        } else {
            log::error!(
                "命令条目 {} 执行失败，退出码 {:?}",
                entry.id,
                result.exit_code
            );
        }
        let _ = app.emit("command-finished", &result);

        let notification_title = if result.success {
            "命令执行成功"
        } else {
            "命令执行失败"
        };
        if let Err(error) = app
            .notification()
            .builder()
            .title(notification_title)
            .body(&entry.title)
            .show()
        {
            log::error!("发送命令执行结果通知失败：{error}");
        }
    });
}
