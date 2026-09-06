use std::{fs, io::Write, path::PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

const CONFIG_FILE_NAME: &str = "config.json";
const CONFIG_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandItem {
    pub id: Uuid,
    pub title: String,
    pub command: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub order: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

const fn default_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppConfig {
    pub version: u32,
    #[serde(default)]
    pub settings: Settings,
    pub items: Vec<CommandItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(default = "default_enabled")]
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { autostart: true }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            settings: Settings::default(),
            items: Vec::new(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSnapshot {
    pub config: AppConfig,
    pub path: String,
    pub warnings: Vec<String>,
}

pub fn load_or_create(
    app: &AppHandle,
) -> Result<(AppConfig, PathBuf, Vec<String>), Box<dyn std::error::Error>> {
    let directory = app.path().app_config_dir()?;
    fs::create_dir_all(&directory)?;
    let path = directory.join(CONFIG_FILE_NAME);

    if !path.exists() {
        let config = AppConfig::default();
        write_config(&path, &config)?;
        return Ok((config, path, Vec::new()));
    }

    let contents = fs::read_to_string(&path)?;
    match parse_resilient(&contents) {
        Ok((config, warnings)) => Ok((config, path, warnings)),
        Err(reason) => {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let backup = path.with_file_name(format!("config.corrupt-{timestamp}.json"));
            fs::rename(&path, &backup)?;
            let config = AppConfig::default();
            write_config(&path, &config)?;
            let warning = format!(
                "配置无法读取（{reason}），已备份到 {} 并恢复默认配置。",
                backup.display()
            );
            log::error!("{warning}");
            Ok((config, path, vec![warning]))
        }
    }
}

fn parse_resilient(contents: &str) -> Result<(AppConfig, Vec<String>), String> {
    let value: serde_json::Value =
        serde_json::from_str(contents).map_err(|error| format!("JSON 解析失败：{error}"))?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "缺少有效的 version".to_string())? as u32;
    if version != CONFIG_VERSION {
        return Err(format!("不支持的配置版本 {version}"));
    }

    let mut warnings = Vec::new();
    let settings = value
        .get("settings")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("settings 字段无效：{error}"))?
        .unwrap_or_default();
    let raw_items = value
        .get("items")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "缺少有效的 items 数组".to_string())?;
    let mut items = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for (index, raw) in raw_items.iter().enumerate() {
        match serde_json::from_value::<CommandItem>(raw.clone()) {
            Ok(item)
                if !item.title.trim().is_empty()
                    && !item.command.trim().is_empty()
                    && ids.insert(item.id) =>
            {
                items.push(item);
            }
            Ok(item) => warnings.push(format!("已跳过无效或重复条目：{}", item.id)),
            Err(error) => warnings.push(format!("已跳过第 {} 个无效条目：{error}", index + 1)),
        }
    }
    items.sort_by_key(|item| item.order);
    for (index, item) in items.iter_mut().enumerate() {
        item.order = index as u32;
    }
    Ok((
        AppConfig {
            version,
            settings,
            items,
        },
        warnings,
    ))
}

pub fn validate(config: &AppConfig) -> Result<(), String> {
    if config.version != CONFIG_VERSION {
        return Err(format!(
            "不支持的配置版本 {}，当前仅支持版本 {CONFIG_VERSION}",
            config.version
        ));
    }

    let mut ids = std::collections::HashSet::new();
    for item in &config.items {
        if item.title.trim().is_empty() {
            return Err(format!("条目 {} 的标题不能为空", item.id));
        }
        if item.command.trim().is_empty() {
            return Err(format!("条目 {} 的命令不能为空", item.id));
        }
        if !ids.insert(item.id) {
            return Err(format!("条目 ID 重复：{}", item.id));
        }
    }
    Ok(())
}

pub fn write_config(
    path: &std::path::Path,
    config: &AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary_path = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(config)?;
    let mut file = fs::File::create(&temporary_path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    crate::file_replace::replace(&temporary_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: Uuid, title: &str, command: &str) -> CommandItem {
        CommandItem {
            id,
            title: title.to_string(),
            command: command.to_string(),
            enabled: true,
            order: 0,
            icon: None,
        }
    }

    #[test]
    fn default_config_has_no_business_items() {
        assert_eq!(AppConfig::default().items, Vec::new());
    }

    #[test]
    fn rejects_duplicate_ids() {
        let id = Uuid::new_v4();
        let config = AppConfig {
            version: CONFIG_VERSION,
            settings: Settings::default(),
            items: vec![item(id, "一", "echo 1"), item(id, "二", "echo 2")],
        };
        assert!(validate(&config).unwrap_err().contains("ID 重复"));
    }

    #[test]
    fn rejects_blank_title_or_command() {
        let blank_title = AppConfig {
            version: CONFIG_VERSION,
            settings: Settings::default(),
            items: vec![item(Uuid::new_v4(), " ", "echo ok")],
        };
        assert!(validate(&blank_title).is_err());

        let blank_command = AppConfig {
            version: CONFIG_VERSION,
            settings: Settings::default(),
            items: vec![item(Uuid::new_v4(), "测试", "\n")],
        };
        assert!(validate(&blank_command).is_err());
    }

    #[test]
    fn older_items_receive_phase_two_defaults() {
        let id = Uuid::new_v4();
        let json = format!(
            r#"{{"version":1,"items":[{{"id":"{id}","title":"测试","command":"echo ok"}}]}}"#
        );
        let config: AppConfig = serde_json::from_str(&json).unwrap();
        assert!(config.settings.autostart);
        assert!(config.items[0].enabled);
        assert_eq!(config.items[0].order, 0);
    }

    #[test]
    fn resilient_parser_skips_bad_items_and_keeps_good_items() {
        let id = Uuid::new_v4();
        let json = format!(
            r#"{{"version":1,"items":[{{"id":"bad","title":"坏","command":"x"}},{{"id":"{id}","title":"好","command":"echo ok"}}]}}"#
        );
        let (config, warnings) = parse_resilient(&json).unwrap();
        assert_eq!(config.items.len(), 1);
        assert_eq!(config.items[0].id, id);
        assert_eq!(warnings.len(), 1);
    }
}
