mod persistence;
mod settings;
mod storage;
mod tray;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            app.manage(tray::PendingEntryAction::default());
            settings::init(app.handle())?;
            storage::init(app.handle())?;
            tray::init(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            storage::entry_list,
            storage::entry_save,
            storage::entry_delete,
            storage::entry_reorder,
            settings::settings_get,
            settings::settings_set,
            tray::show_panel_menu,
            tray::take_pending_action,
            tray::open_main_create,
            tray::open_main_window,
            tray::open_entry_editor
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
