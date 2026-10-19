#[cfg(target_os = "macos")]
fn main() {
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    };
    use tauri::{
        menu::{Menu, MenuItem, Submenu},
        Manager,
    };
    use timelet_lib::notifications::{self, NotificationAction, ReminderNotification};

    fn status(app: &tauri::AppHandle, message: impl Into<String>) {
        let message = message.into();
        println!("{message}");
        if let Some(window) = app.get_webview_window("config") {
            let text = serde_json::to_string(&message).unwrap();
            let _ = window.eval(format!(
                "document.body.querySelector('pre').textContent += {text} + '\\n'"
            ));
        }
    }

    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "com.littleseven.timelet.notification-check".into();
    context.config_mut().product_name = Some("Timelet Notification Check".into());
    context.config_mut().app.windows.clear();

    let sequence = Arc::new(AtomicU64::new(0));
    let ids = Arc::new(Mutex::new(Vec::<String>::new()));
    let action_ids = ids.clone();
    tauri::Builder::default()
        .register_uri_scheme_protocol("notification-check", |_context, _request| {
            tauri::http::Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body("<meta charset='utf-8'><title>Timelet 通知验证</title><body style='font:16px system-ui;padding:24px'><h2>Timelet 原生通知验证</h2><p>通过菜单栏“通知验证”执行操作。此副本不读取真实条目，不执行完成或稍后写盘。</p><pre style='white-space:pre-wrap'></pre></body>".as_bytes().to_vec())
                .unwrap()
        })
        .setup(move |app| {
            let app_handle = app.handle().clone();
            notifications::init(move |response| {
                status(&app_handle, format!("动作回调：{}", serde_json::to_string(&response).unwrap()));
                if response.action == NotificationAction::Open {
                    let handle = app_handle.clone();
                    let _ = app_handle.run_on_main_thread(move || {
                        if let Some(window) = handle.get_webview_window("config") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    });
                }
            }).map_err(std::io::Error::other)?;

            let permission = MenuItem::with_id(app, "permission", "读取通知权限", true, None::<&str>)?;
            let request = MenuItem::with_id(app, "request", "请求通知授权", true, None::<&str>)?;
            let send = MenuItem::with_id(app, "send", "发送测试通知", true, None::<&str>)?;
            let snapshot = MenuItem::with_id(app, "snapshot", "查看系统通知记录", true, None::<&str>)?;
            let remove = MenuItem::with_id(app, "remove", "撤回本次测试通知", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "隐藏测试窗口", true, None::<&str>)?;
            let dock = MenuItem::with_id(app, "dock", "隐藏测试 Dock 图标", true, None::<&str>)?;
            let restore = MenuItem::with_id(app, "restore", "恢复测试窗口与 Dock 图标", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出通知验证", true, None::<&str>)?;
            let submenu = Submenu::with_items(app, "通知验证", true, &[&permission, &request, &send, &snapshot, &remove, &hide, &dock, &restore, &quit])?;
            app.set_menu(Menu::with_items(app, &[&submenu])?)?;
            let tray_menu = Menu::with_items(app, &[&permission, &request, &send, &snapshot, &remove, &hide, &dock, &restore, &quit])?;
            tauri::tray::TrayIconBuilder::new()
                .icon(tauri::include_image!("icons/tray.png"))
                .icon_as_template(true)
                .tooltip("Timelet 通知验证")
                .menu(&tray_menu)
                .build(app)?;

            tauri::WebviewWindowBuilder::new(app, "config", tauri::WebviewUrl::CustomProtocol("notification-check://localhost/".parse()?))
                .title("Timelet 通知验证").inner_size(660.0, 440.0).build()?;
            let handle = app.handle().clone();
            notifications::permission(move |permission| status(&handle, format!("初始权限：{}", serde_json::to_string(&permission).unwrap())));
            Ok(())
        })
        .on_menu_event(move |app, event| {
            match event.id().as_ref() {
                "permission" => {
                    let app = app.clone();
                    notifications::permission(move |permission| status(&app, format!("权限：{}", serde_json::to_string(&permission).unwrap())));
                }
                "request" => {
                    let app = app.clone();
                    notifications::request_permission(move |result| status(&app, format!("授权结果：{result:?}")));
                }
                "send" => {
                    let id = format!("timelet-check-{}-{}", std::process::id(), sequence.fetch_add(1, Ordering::SeqCst));
                    action_ids.lock().unwrap().push(id.clone());
                    let handle = app.clone();
                    let result = notifications::send(ReminderNotification {
                        occurrence_id: id, entry_id: "notification-check-entry".into(), revision: "check-revision-1".into(),
                        title: "时屿通知测试".into(), body: "现在 · 原生通知验证，请测试完成、10分钟后提醒和点击打开。".into(),
                    }, move |result| status(&handle, format!("系统接受请求：{result:?}")));
                    if let Err(error) = result { status(app, error); }
                }
                "snapshot" => {
                    let app = app.clone();
                    notifications::snapshot(move |snapshot| status(&app, format!("系统记录：{}", serde_json::to_string(&snapshot).unwrap())));
                }
                "remove" => {
                    notifications::remove(&action_ids.lock().unwrap());
                    status(app, "已请求撤回本次测试的通知");
                }
                "hide" => { if let Some(window) = app.get_webview_window("config") { let _ = window.hide(); } }
                "dock" => { let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory); }
                "restore" => {
                    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
                    if let Some(window) = app.get_webview_window("config") { let _ = window.show(); let _ = window.set_focus(); }
                }
                "quit" => { notifications::remove(&action_ids.lock().unwrap()); app.exit(0); }
                _ => {}
            }
        })
        .run(context)
        .expect("无法启动通知验证应用");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("原生通知验证当前仅支持 macOS");
}
