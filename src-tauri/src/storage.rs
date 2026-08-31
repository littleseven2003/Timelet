use crate::persistence::{DataFile, Document};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

// 数据文件结构版本，字段演进时递增并编写迁移逻辑
const SCHEMA_VERSION: u32 = 2;
const ENTRIES_FILE: &str = "entries.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub name: String,
    // countdown：距目标日；elapsed：自起始日已过
    pub entry_type: String,
    // ISO 日期（YYYY-MM-DD）
    pub date: String,
    // 可选时刻（HH:mm），缺失表示纯日期条目
    pub time: Option<String>,
    // 循环规则（daily 每天 / workday 工作日），仅带时刻的条目生效
    pub repeat: Option<String>,
    // 展示单位（day/week/month/year），缺失按天
    pub display_unit: Option<String>,
    // 选填备注，面板悬停时展示
    pub note: Option<String>,
    pub pinned: bool,
    // 手动排序值（拖拽后生成），缺失表示按自动规则排序
    pub sort_index: Option<i64>,
    // 归档后离开活动视图，可恢复；缺失表示活动条目
    pub archived: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Store {
    schema_version: u32,
    entries: Vec<Entry>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

impl Document for Store {
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "不支持的数据版本 {}，请使用兼容版本打开",
                self.schema_version
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for entry in &self.entries {
            if entry.id.is_empty() || !ids.insert(&entry.id) {
                return Err("条目标识为空或重复".into());
            }
            if !valid_date(&entry.date)
                || !matches!(entry.entry_type.as_str(), "countdown" | "elapsed")
                || entry.time.as_deref().is_some_and(|time| !valid_time(time))
            {
                return Err(format!("条目「{}」的日期、时刻或类型无法识别", entry.name));
            }
        }
        Ok(())
    }
}

fn valid_date(date: &str) -> bool {
    if date.len() != 10
        || !date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
    {
        return false;
    }
    let parts: Vec<_> = date.split('-').map(str::parse::<u32>).collect();
    let [Ok(year), Ok(month), Ok(day)] = parts.as_slice() else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let limit = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return false,
    };
    *year > 0 && *day >= 1 && *day <= limit
}

fn valid_time(time: &str) -> bool {
    if time.len() != 5 || time.as_bytes()[2] != b':' {
        return false;
    }
    let parts: Vec<_> = time.split(':').map(str::parse::<u32>).collect();
    matches!(parts.as_slice(), [Ok(hour), Ok(minute)] if *hour < 24 && *minute < 60)
}

impl Store {
    fn upsert(&mut self, entry: Entry, expected: Option<&str>) -> Result<(), String> {
        match self.entries.iter_mut().find(|e| e.id == entry.id) {
            Some(existing) => {
                if expected != Some(existing.updated_at.as_str()) {
                    return Err("条目已在另一窗口变化，请保留草稿并重新打开最新条目".into());
                }
                *existing = entry;
            }
            None if expected.is_some() => return Err("条目已被删除，未重新创建旧记录".into()),
            None => self.entries.push(entry),
        }
        Ok(())
    }
}

pub struct EntryStore(DataFile<Store>);

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let path = app.path().app_data_dir()?.join(ENTRIES_FILE);
    app.manage(EntryStore(DataFile::new(path)));
    Ok(())
}

#[tauri::command]
pub fn entry_list(state: State<'_, EntryStore>) -> Result<Vec<Entry>, String> {
    Ok(state.0.read()?.entries)
}

#[tauri::command]
pub fn entry_save(
    app: AppHandle,
    state: State<'_, EntryStore>,
    entry: Entry,
    expected_updated_at: Option<String>,
) -> Result<(), String> {
    if entry.name.trim().is_empty() || !matches!(entry.entry_type.as_str(), "countdown" | "elapsed")
    {
        return Err("请填写名称并选择有效类型".into());
    }
    state
        .0
        .change(|store| store.upsert(entry, expected_updated_at.as_deref()))?;
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn entry_delete(
    app: AppHandle,
    state: State<'_, EntryStore>,
    id: String,
) -> Result<(), String> {
    state.0.change(|store| {
        store.entries.retain(|e| e.id != id);
        Ok(())
    })?;
    notify_changed(&app);
    Ok(())
}

fn notify_changed(app: &AppHandle) {
    let _ = app.emit("entries-changed", ());
}

#[tauri::command]
pub fn entry_reorder(
    app: AppHandle,
    state: State<'_, EntryStore>,
    ids: Vec<String>,
    reset: Option<bool>,
) -> Result<(), String> {
    state.0.change(|store| {
        let active: std::collections::HashSet<_> = store
            .entries
            .iter()
            .filter(|e| e.archived != Some(true))
            .map(|e| &e.id)
            .collect();
        let requested: std::collections::HashSet<_> = ids.iter().collect();
        if requested != active || ids.len() != active.len() {
            return Err("条目已变化，请刷新完整列表后重新排序".into());
        }
        for (index, id) in ids.iter().enumerate() {
            if let Some(entry) = store.entries.iter_mut().find(|e| e.id == *id) {
                entry.sort_index = if reset.unwrap_or(false) {
                    None
                } else {
                    Some(index as i64)
                };
            }
        }
        Ok(())
    })?;
    notify_changed(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_schema_keeps_optional_archive_field_and_rejects_other_versions() {
        let current = r##"{"schema_version":2,"entries":[{"id":"a","name":"生日","entryType":"countdown","date":"2026-09-01","pinned":false,"createdAt":"2026-08-01T00:00:00Z","updatedAt":"2026-08-01T00:00:00Z"}]}"##;
        let store: Store = serde_json::from_str(current).unwrap();
        assert!(store.validate().is_ok());
        assert_eq!(store.entries[0].archived, None);
        let legacy: Store =
            serde_json::from_str(&current.replace("schema_version\":2", "schema_version\":1"))
                .unwrap();
        assert!(legacy.validate().is_err());
        let future: Store =
            serde_json::from_str(&current.replace("schema_version\":2", "schema_version\":3"))
                .unwrap();
        assert!(future.validate().is_err());
        assert!(serde_json::from_str::<Store>(
            &current.replace("\"pinned\":false", "\"color\":\"#2a9cdb\",\"pinned\":false")
        )
        .is_err());
    }

    #[test]
    fn invalid_calendar_dates_and_times_are_rejected() {
        assert!(valid_date("2024-02-29"));
        assert!(!valid_date("2026-02-29"));
        assert!(!valid_date("2026-04-31"));
        assert!(!valid_date("broken"));
        assert!(valid_time("23:59"));
        assert!(!valid_time("24:00"));
    }

    #[test]
    fn stale_edits_cannot_overwrite_or_resurrect_an_entry() {
        let mut store: Store = serde_json::from_str(r##"{"schema_version":2,"entries":[{"id":"a","name":"日期","entryType":"countdown","date":"2026-09-01","pinned":false,"createdAt":"old","updatedAt":"new"}]}"##).unwrap();
        let mut stale = store.entries[0].clone();
        stale.name = "旧编辑".into();
        assert!(store.upsert(stale.clone(), Some("old")).is_err());
        assert_eq!(store.entries[0].name, "日期");
        assert!(store.upsert(stale.clone(), Some("new")).is_ok());
        store.entries.clear();
        assert!(store.upsert(stale, Some("new")).is_err());
        assert!(store.entries.is_empty());
    }
}
