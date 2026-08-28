use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Position, Size, WebviewWindow, WindowEvent};

// 面板逻辑尺寸（物理像素 = 逻辑像素 × 缩放比），与前端面板视图保持一致
const PANEL_LOGICAL_WIDTH: f64 = 320.0;
// 仅 Windows 定位需要面板高度（面板贴任务栏上方）；窗口实际高度由窗口配置提供
#[cfg(target_os = "windows")]
const PANEL_LOGICAL_HEIGHT: f64 = 420.0;
// 面板与托盘图标的间距（逻辑像素）
const PANEL_GAP: f64 = 5.0;
// 失焦收起后短时间内的托盘点击视为"收起"动作，避免收起后又被立刻弹出
const BLUR_RACE_WINDOW: Duration = Duration::from_millis(250);

// 记录面板最近一次因失焦而收起的时刻，用于托盘点击的竞态判断
#[derive(Default)]
struct PanelBlurState(Mutex<Option<Instant>>);

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(PanelBlurState::default());
    watch_panel_blur(app);

    let quit = MenuItem::with_id(app, "quit", "退出 Timelet", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .icon(
            app.default_window_icon()
                .expect("应用图标缺失，无法创建托盘图标")
                .clone(),
        )
        .tooltip("Timelet（时屿）")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id() == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position: _,
                rect,
                ..
            } = event
            {
                toggle_panel(tray.app_handle(), rect.position, rect.size);
            }
        });

    // macOS 菜单栏要求图标为模板样式以自适应深浅色
    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    builder.build(app)?;
    Ok(())
}

// 左键点击托盘图标时切换面板显隐，并在显示前按图标位置重新定位
fn toggle_panel(app: &AppHandle, icon_pos: Position, icon_size: Size) {
    let Some(panel) = app.get_webview_window("panel") else {
        return;
    };
    let state = app.state::<PanelBlurState>();

    if panel.is_visible().unwrap_or(false) {
        hide_panel(&panel, &state);
    } else {
        let recently_hidden = state
            .0
            .lock()
            .unwrap()
            .map(|t| t.elapsed() < BLUR_RACE_WINDOW)
            .unwrap_or(false);
        if recently_hidden {
            return;
        }
        if position_panel(&panel, icon_pos, icon_size).is_ok() {
            let _ = panel.show();
            let _ = panel.set_focus();
        }
    }
}

// 失焦时收起面板；macOS 点击菜单栏与 Windows 点击任务栏均会触发
fn watch_panel_blur(app: &AppHandle) {
    let Some(panel) = app.get_webview_window("panel") else {
        return;
    };
    let handle = app.clone();
    let blur_panel = panel.clone();
    panel.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            let state = handle.state::<PanelBlurState>();
            hide_panel(&blur_panel, &state);
        }
    });
}

fn hide_panel(panel: &WebviewWindow, state: &PanelBlurState) {
    *state.0.lock().unwrap() = Some(Instant::now());
    let _ = panel.hide();
}

// 双平台定位：macOS 面板出现在菜单栏图标正下方，Windows 出现在任务栏图标上方
fn position_panel(panel: &WebviewWindow, icon_pos: Position, icon_size: Size) -> tauri::Result<()> {
    let scale = panel.scale_factor()?;
    let width = PANEL_LOGICAL_WIDTH * scale;
    #[cfg(target_os = "windows")]
    let height = PANEL_LOGICAL_HEIGHT * scale;

    // 托盘事件坐标可能是逻辑值，统一转成物理坐标参与计算
    let icon_pos: tauri::PhysicalPosition<f64> = icon_pos.to_physical(scale);
    let icon_size: tauri::PhysicalSize<f64> = icon_size.to_physical(scale);

    let icon_center_x = icon_pos.x + icon_size.width / 2.0;
    let icon_edge_y = icon_pos.y + icon_size.height;

    let mut x = icon_center_x - width / 2.0;
    #[cfg(target_os = "macos")]
    let mut y = icon_edge_y + PANEL_GAP * scale;
    #[cfg(target_os = "windows")]
    let mut y = icon_pos.y - height - PANEL_GAP * scale;

    // 防止水平越界：限制在托盘图标所在显示器的范围内
    if let Ok(Some(monitor)) = panel.monitor_from_point(icon_center_x, icon_edge_y) {
        let origin = monitor.position();
        let size = monitor.size();
        let max_x = (origin.x + size.width as i32) as f64 - width;
        x = x.clamp(origin.x as f64, max_x);

        #[cfg(target_os = "macos")]
        {
            let min_y = origin.y as f64;
            if y < min_y {
                y = min_y;
            }
        }
        #[cfg(target_os = "windows")]
        {
            let max_y = (origin.y + size.height as i32) as f64 - height;
            if y > max_y {
                y = max_y;
            }
        }
    }

    panel.set_position(tauri::PhysicalPosition::new(x, y))
}
