use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

const SETTINGS_FILE: &str = "settings.json";
const WINDOW_STATE_FILE: &str = "window-state.json";

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

// 主界面窗口的边界（逻辑坐标），用于关闭后恢复
#[derive(Debug, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn save_window_bounds(app: &AppHandle, window: &tauri::WebviewWindow) -> Result<(), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let position = window.outer_position().map_err(|e| e.to_string())?;

    let bounds = WindowBounds {
        x: position.x as f64 / scale,
        y: position.y as f64 / scale,
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
    };
    let text = serde_json::to_string_pretty(&bounds).map_err(|e| e.to_string())?;

    let mut file = app.path().app_data_dir().expect("无法定位应用数据目录");
    fs::create_dir_all(&file).ok();
    file.push(WINDOW_STATE_FILE);
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &file).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_window_bounds(app: &AppHandle) -> Option<WindowBounds> {
    let mut file = app.path().app_data_dir().expect("无法定位应用数据目录");
    file.push(WINDOW_STATE_FILE);
    let text = fs::read_to_string(file).ok()?;
    serde_json::from_str(&text).ok()
}
