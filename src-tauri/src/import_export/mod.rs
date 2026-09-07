use std::{collections::HashSet, fs, io::Write, path::Path};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    config::{AppConfig, CommandItem},
    icon,
};

const EXPORT_FORMAT: &str = "tray-command-launcher";
const EXPORT_VERSION: u32 = 1;
const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
struct ExportFile {
    format: String,
    version: u32,
    items: Vec<CommandItem>,
}

pub fn export(path: &Path, config: &AppConfig) -> Result<(), String> {
    let document = ExportFile {
        format: EXPORT_FORMAT.to_string(),
        version: EXPORT_VERSION,
        items: config.items.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&document)
        .map_err(|error| format!("生成导出 JSON 失败：{error}"))?;
    let temporary = path.with_extension("json.tmp");
    let mut file =
        fs::File::create(&temporary).map_err(|error| format!("创建导出文件失败：{error}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("写入导出文件失败：{error}"))?;
    crate::file_replace::replace(&temporary, path)
        .map_err(|error| format!("完成导出文件失败：{error}"))
}

pub fn read(path: &Path) -> Result<Vec<CommandItem>, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("读取导入文件信息失败：{error}"))?;
    if metadata.len() > MAX_IMPORT_BYTES {
        return Err("导入文件不能超过 64 MiB".to_string());
    }
    let contents =
        fs::read_to_string(path).map_err(|error| format!("读取导入文件失败：{error}"))?;
    parse(&contents)
}

fn parse(contents: &str) -> Result<Vec<CommandItem>, String> {
    let document: ExportFile =
        serde_json::from_str(contents).map_err(|error| format!("JSON 解析失败：{error}"))?;
    if document.format != EXPORT_FORMAT {
        return Err(format!("不支持的导入格式：{}", document.format));
    }
    if document.version != EXPORT_VERSION {
        return Err(format!("不支持的导入版本：{}", document.version));
    }

    let mut ids = HashSet::new();
    let mut items = document.items;
    for item in &mut items {
        if item.title.trim().is_empty() || item.command.trim().is_empty() {
            return Err(format!("条目 {} 的标题和命令不能为空", item.id));
        }
        if !ids.insert(item.id) {
            return Err(format!("导入文件包含重复 UUID：{}", item.id));
        }
        if let Some(data_url) = &item.icon {
            item.icon = Some(icon::normalize_data_url(data_url)?);
        }
    }
    items.sort_by_key(|item| item.order);
    Ok(items)
}

pub fn merge(existing: &[CommandItem], imported: Vec<CommandItem>) -> Vec<CommandItem> {
    let mut result = existing.to_vec();
    let mut ids = result.iter().map(|item| item.id).collect::<HashSet<_>>();
    for mut item in imported {
        while !ids.insert(item.id) {
            item.id = Uuid::new_v4();
        }
        item.order = result.len() as u32;
        result.push(item);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: Uuid, order: u32) -> CommandItem {
        CommandItem {
            id,
            title: format!("条目 {order}"),
            command: format!("echo {order}"),
            enabled: true,
            order,
            icon: None,
        }
    }

    #[test]
    fn rejects_wrong_format_and_version() {
        assert!(parse(r#"{"format":"other","version":1,"items":[]}"#).is_err());
        assert!(parse(r#"{"format":"tray-command-launcher","version":2,"items":[]}"#).is_err());
    }

    #[test]
    fn append_regenerates_conflicting_uuid_and_preserves_existing() {
        let id = Uuid::new_v4();
        let merged = merge(&[item(id, 0)], vec![item(id, 8)]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id, id);
        assert_ne!(merged[1].id, id);
        assert_eq!(merged[1].order, 1);
    }

    #[test]
    fn parsing_never_executes_imported_command() {
        let marker = std::env::temp_dir().join(format!("multiboot-import-{}", Uuid::new_v4()));
        let id = Uuid::new_v4();
        let json = serde_json::json!({
            "format": "tray-command-launcher",
            "version": 1,
            "items": [{
                "id": id,
                "title": "安全测试",
                "command": format!("touch {}", marker.display()),
                "enabled": true,
                "order": 0
            }]
        });
        let parsed = parse(&json.to_string()).unwrap();
        assert_eq!(parsed.len(), 1);
        assert!(!marker.exists());
    }
}
