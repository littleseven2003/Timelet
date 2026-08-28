use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

// 数据文件结构版本，字段演进时递增并编写迁移逻辑
const SCHEMA_VERSION: u32 = 1;
const ENTRIES_FILE: &str = "entries.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub name: String,
    // countdown：距目标日；elapsed：自起始日已过
    pub entry_type: String,
    // ISO 日期（YYYY-MM-DD）
    pub date: String,
    // 可选时刻（HH:mm），缺失表示纯日期条目
    pub time: Option<String>,
    // 展示单位（day/week/month/year），缺失按天
    pub display_unit: Option<String>,
    // 选填备注，面板悬停时展示
    pub note: Option<String>,
    pub color: String,
    pub pinned: bool,
    // 手动排序值（拖拽后生成），缺失表示按自动规则排序
    pub sort_index: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Store {
    schema_version: u32,
    entries: Vec<Entry>,
}

// 全局条目状态：启动时加载，变更后立即落盘
pub struct EntryStore(Mutex<Vec<Entry>>);

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let entries = load(app).unwrap_or_default();
    app.manage(EntryStore(Mutex::new(entries)));
    Ok(())
}

fn storage_path(app: &AppHandle) -> PathBuf {
    let mut path = app.path().app_data_dir().expect("无法定位应用数据目录");
    fs::create_dir_all(&path).ok();
    path.push(ENTRIES_FILE);
    path
}

fn load(app: &AppHandle) -> Option<Vec<Entry>> {
    let text = fs::read_to_string(storage_path(app)).ok()?;
    let store: Store = serde_json::from_str(&text).ok()?;
    // 后续版本按 schema_version 在此编写迁移
    Some(store.entries)
}

// 写临时文件后原子替换，避免写入中断导致数据损坏
fn save(app: &AppHandle, entries: &[Entry]) -> Result<(), String> {
    let store = Store {
        schema_version: SCHEMA_VERSION,
        entries: entries.to_vec(),
    };
    let text = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;

    let path = storage_path(app);
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn entry_list(state: State<'_, EntryStore>) -> Vec<Entry> {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn entry_save(
    app: AppHandle,
    state: State<'_, EntryStore>,
    entry: Entry,
) -> Result<(), String> {
    let mut entries = state.0.lock().unwrap();
    match entries.iter_mut().find(|e| e.id == entry.id) {
        Some(existing) => *existing = entry,
        None => entries.push(entry),
    }
    save(&app, &entries)?;
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn entry_delete(
    app: AppHandle,
    state: State<'_, EntryStore>,
    id: String,
) -> Result<(), String> {
    let mut entries = state.0.lock().unwrap();
    entries.retain(|e| e.id != id);
    save(&app, &entries)?;
    notify_changed(&app);
    Ok(())
}

// 数据变更后广播事件，让隐藏中的面板窗口刷新列表
fn notify_changed(app: &AppHandle) {
    app.emit("entries-changed", ()).ok();
}

// 按给定 id 顺序写入手动排序值，一次落盘并广播
#[tauri::command]
pub fn entry_reorder(
    app: AppHandle,
    state: State<'_, EntryStore>,
    ids: Vec<String>,
) -> Result<(), String> {
    let mut entries = state.0.lock().unwrap();
    for (index, id) in ids.iter().enumerate() {
        if let Some(entry) = entries.iter_mut().find(|e| e.id == *id) {
            entry.sort_index = Some(index as i64);
        }
    }
    save(&app, &entries)?;
    notify_changed(&app);
    Ok(())
}
