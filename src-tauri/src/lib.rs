mod commands;
mod window;

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::search::search_aliases,
            commands::alias::open_alias,
            commands::executor::execute_command,
            commands::system::hide_window,
        ])
        .setup(|app| {
            // 全局快捷键 Cmd+J
            let shortcut: Shortcut = "CommandOrControl+J".parse().unwrap();
            let app_handle = app.handle().clone();
            app.global_shortcut()
                .on_shortcut(shortcut, move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        window::spotlight::toggle_spotlight(&app_handle);
                    }
                })?;

            // 系统托盘
            let open_search = MenuItemBuilder::with_id("open_search", "🔍 打开搜索").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "❌ 退出").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&open_search)
                .separator()
                .item(&quit)
                .build()?;

            let app_handle2 = app.handle().clone();
            TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap())
                .icon_as_template(true)
                .menu(&menu)
                .on_menu_event(move |_app, event| match event.id().as_ref() {
                    "open_search" => {
                        window::spotlight::toggle_spotlight(&app_handle2);
                    }
                    "quit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
