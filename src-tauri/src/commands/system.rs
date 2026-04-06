use tauri::Manager;

#[tauri::command]
pub fn hide_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("spotlight") {
        let _ = window.hide();
    }
}
