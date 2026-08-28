mod storage;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            storage::init(app.handle())?;
            tray::init(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            storage::entry_list,
            storage::entry_save,
            storage::entry_delete
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
