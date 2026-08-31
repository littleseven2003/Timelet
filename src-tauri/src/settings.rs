use crate::persistence::{DataFile, Document};
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

const SETTINGS_FILE: &str = "settings.json";
const WINDOW_STATE_FILE: &str = "window-state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Settings {
    // 开机自启（开关状态与系统自启项同步，由前端调用自启插件维护）
    pub launch_at_login: bool,
    // 面板是否显示已过期的倒计时条目
    pub show_expired: bool,
    // 外观主题（system/light/dark），缺失跟随系统
    pub theme: Option<String>,
    pub panel_limit: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            show_expired: true,
            theme: None,
            panel_limit: 6,
        }
    }
}

impl Document for Settings {
    fn validate(&self) -> Result<(), String> {
        if !matches!(
            self.theme.as_deref(),
            None | Some("system" | "light" | "dark")
        ) || !(5..=8).contains(&self.panel_limit)
        {
            return Err("不支持的外观或面板条数设置，请使用兼容版本打开".into());
        }
        Ok(())
    }
}

pub struct SettingsStore(DataFile<Settings>);

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(SettingsStore(DataFile::new(path(app)?)));
    Ok(())
}

fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_data_dir()?.join(SETTINGS_FILE))
}

#[tauri::command]
pub fn settings_get(state: State<'_, SettingsStore>) -> Result<Settings, String> {
    state.0.read()
}

#[tauri::command]
pub fn settings_set(
    app: AppHandle,
    state: State<'_, SettingsStore>,
    settings: Settings,
) -> Result<(), String> {
    state.0.change(|current| {
        *current = settings.clone();
        Ok(())
    })?;
    // 数据已经保存，广播失败不能被误报为写入失败。
    let _ = app.emit("settings-changed", settings);
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
