use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    // 开机自启（开关状态与系统自启项同步，由前端调用自启插件维护）
    pub launch_at_login: bool,
    // 面板是否显示已过期的倒计时条目
    pub show_expired: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            show_expired: true,
        }
    }
}

// 全局设置状态：启动时加载，变更后立即落盘
pub struct SettingsStore(Mutex<Settings>);

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let settings = fs::read_to_string(path(app))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    app.manage(SettingsStore(Mutex::new(settings)));
    Ok(())
}

fn path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("无法定位应用数据目录")
        .join(SETTINGS_FILE)
}

#[tauri::command]
pub fn settings_get(state: State<'_, SettingsStore>) -> Settings {
    state.0.lock().unwrap().clone()
}

// 整体保存并原子落盘
#[tauri::command]
pub fn settings_set(
    app: AppHandle,
    state: State<'_, SettingsStore>,
    settings: Settings,
) -> Result<(), String> {
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    let file = path(&app);
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &file).map_err(|e| e.to_string())?;
    *state.0.lock().unwrap() = settings;
    Ok(())
}
