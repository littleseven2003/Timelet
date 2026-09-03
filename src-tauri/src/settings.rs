use crate::persistence::{DataFile, Document};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

const SETTINGS_FILE: &str = "settings.json";
const WINDOW_STATE_FILE: &str = "window-state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Settings {
    // 开机自启（开关状态与系统自启项同步，由前端调用自启插件维护）
    pub launch_at_login: bool,
    pub hide_app_icon: bool,
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
            hide_app_icon: false,
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

pub struct SettingsStore {
    data: DataFile<Settings>,
    // 系统显隐与磁盘保存作为一个操作串行执行，避免失败回滚覆盖后一次切换。
    update_lock: Mutex<()>,
}

impl SettingsStore {
    fn new(path: PathBuf) -> Self {
        Self {
            data: DataFile::new(path),
            update_lock: Mutex::new(()),
        }
    }

    fn save(
        &self,
        settings: &Settings,
        mut apply: impl FnMut(bool) -> Result<(), String>,
    ) -> Result<(), String> {
        let _guard = self
            .update_lock
            .lock()
            .map_err(|_| "设置正在恢复，请重启应用")?;
        settings.validate()?;
        let previous = self.data.read()?;
        let changed = previous.hide_app_icon != settings.hide_app_icon;
        if changed {
            apply(settings.hide_app_icon)?;
        }
        if let Err(error) = self.data.change(|current| {
            *current = settings.clone();
            Ok(())
        }) {
            if changed {
                apply(previous.hide_app_icon).map_err(|rollback| {
                    format!("{error}；图标状态恢复失败，请重启应用：{rollback}")
                })?;
            }
            return Err(error);
        }
        Ok(())
    }
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(SettingsStore::new(path(app)?));
    Ok(())
}

fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_data_dir()?.join(SETTINGS_FILE))
}

#[tauri::command]
pub fn settings_get(state: State<'_, SettingsStore>) -> Result<Settings, String> {
    state.data.read()
}

pub fn apply_startup_visibility(app: &AppHandle) {
    let result = app
        .state::<SettingsStore>()
        .data
        .read()
        .and_then(|settings| {
            if settings.hide_app_icon {
                apply_icon_visibility(app, true)?;
            }
            Ok(())
        });
    if let Err(error) = result {
        // 保留默认图标和托盘入口，让用户仍能打开设置处理错误。
        eprintln!("恢复应用图标设置失败：{error}");
    }
}

#[cfg(target_os = "windows")]
pub fn hide_app_icon(app: &AppHandle) -> bool {
    app.state::<SettingsStore>()
        .data
        .read()
        .map(|settings| settings.hide_app_icon)
        .unwrap_or(false)
}

fn apply_icon_visibility(app: &AppHandle, hidden: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let focused = app.webview_windows().into_values().find(|window| {
            window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false)
        });
        app.set_activation_policy(if hidden {
            tauri::ActivationPolicy::Accessory
        } else {
            tauri::ActivationPolicy::Regular
        })
        .map_err(|error| format!("无法切换 Dock 图标：{error}"))?;
        // 切换应用策略可能移走焦点，不应打断用户正在操作的设置窗口。
        if let Some(window) = focused {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
    #[cfg(target_os = "windows")]
    if let Some(window) = app.get_webview_window("config") {
        window
            .set_skip_taskbar(hidden)
            .map_err(|error| format!("无法切换任务栏图标：{error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn settings_set(
    app: AppHandle,
    state: State<'_, SettingsStore>,
    settings: Settings,
) -> Result<(), String> {
    state.save(&settings, |hidden| apply_icon_visibility(&app, hidden))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "timelet-settings-test-{}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn store(&self) -> SettingsStore {
            SettingsStore::new(self.0.join(SETTINGS_FILE))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_icon_setting_defaults_to_visible_and_choice_survives_reload() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join(SETTINGS_FILE), r#"{"theme":"dark"}"#).unwrap();
        let store = fixture.store();
        let mut settings = store.data.read().unwrap();
        assert!(!settings.hide_app_icon);
        settings.hide_app_icon = true;
        store.save(&settings, |_| Ok(())).unwrap();
        let reloaded = fixture.store().data.read().unwrap();
        assert!(reloaded.hide_app_icon);
        assert_eq!(reloaded.theme.as_deref(), Some("dark"));
    }

    #[test]
    fn system_failure_keeps_saved_settings_and_other_saves_do_not_toggle_icon() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let mut settings = Settings {
            hide_app_icon: true,
            ..Settings::default()
        };
        assert!(store
            .save(&settings, |_| Err("系统拒绝切换".into()))
            .is_err());
        assert!(!store.data.read().unwrap().hide_app_icon);
        settings.hide_app_icon = false;
        settings.theme = Some("dark".into());
        store
            .save(&settings, |_| panic!("主题保存不应重复切换图标"))
            .unwrap();
    }

    #[test]
    fn save_failure_restores_icon_and_rollback_failure_is_reported() {
        let fixture = Fixture::new();
        let store = fixture.store();
        store.save(&Settings::default(), |_| Ok(())).unwrap();
        fs::create_dir(fixture.0.join("settings.json.bak.tmp")).unwrap();
        let settings = Settings {
            hide_app_icon: true,
            ..Settings::default()
        };
        let mut applied = Vec::new();
        assert!(store
            .save(&settings, |hidden| {
                applied.push(hidden);
                Ok(())
            })
            .is_err());
        assert_eq!(applied, vec![true, false]);
        assert!(!fixture.store().data.read().unwrap().hide_app_icon);
        let error = store
            .save(&settings, |hidden| {
                if hidden {
                    Ok(())
                } else {
                    Err("系统拒绝恢复".into())
                }
            })
            .unwrap_err();
        assert!(error.contains("图标状态恢复失败，请重启应用"));
    }

    #[test]
    fn unreadable_or_invalid_settings_do_not_change_system_icon() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let mut settings = Settings {
            hide_app_icon: true,
            ..Settings::default()
        };
        settings.panel_limit = 0;
        assert!(store
            .save(&settings, |_| panic!("无效设置不应改变图标"))
            .is_err());
        settings.panel_limit = 6;
        fs::write(fixture.0.join(SETTINGS_FILE), "{broken").unwrap();
        assert!(store
            .save(&settings, |_| panic!("读取失败不应改变图标"))
            .is_err());
        assert_eq!(
            fs::read_to_string(fixture.0.join(SETTINGS_FILE)).unwrap(),
            "{broken"
        );
    }
}
