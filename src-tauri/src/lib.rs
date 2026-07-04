mod automations;
mod codex_config;
mod codex_process;
mod commands;
mod error;
mod local_db;
mod models;
mod paths;
mod pricing;
mod session_logs;
mod settings;
mod skills_board;
mod snapshot;

use commands::{
    archive_skill, create_codex_config_backup, delete_codex_config_backup, disable_skill,
    enable_skill, get_app_settings, get_detection_paths, get_skill_board, get_usage_snapshot,
    list_codex_config_backups, open_log_folder, open_skill_folder, refresh_task_board,
    restore_codex_config_backup, save_app_settings, set_always_on_top,
};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow,
};

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state() == ShortcutState::Pressed {
                        toggle_main_window(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_usage_snapshot,
            refresh_task_board,
            get_app_settings,
            save_app_settings,
            set_always_on_top,
            get_detection_paths,
            list_codex_config_backups,
            create_codex_config_backup,
            restore_codex_config_backup,
            delete_codex_config_backup,
            open_log_folder,
            get_skill_board,
            disable_skill,
            enable_skill,
            archive_skill,
            open_skill_folder
        ])
        .setup(|app| {
            if let Err(err) = crate::codex_config::ensure_default_codex_config_backup() {
                eprintln!("保存 Codex 默认配置备份失败: {err}");
            }
            setup_tray(app)?;
            setup_shortcut(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 codex-paishu 时出错");
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let toggle = MenuItemBuilder::with_id("toggle", "显示 / 隐藏").build(app)?;
    let topmost = MenuItemBuilder::with_id("topmost", "切换窗口置顶").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&toggle, &topmost, &quit])
        .build()?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => toggle_main_window(app),
            "topmost" => toggle_always_on_top(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn setup_shortcut(app: &mut tauri::App) -> tauri::Result<()> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let shortcut = if cfg!(target_os = "macos") {
        "Command+U"
    } else {
        "Ctrl+Alt+U"
    };
    if let Err(err) = app.global_shortcut().register(shortcut) {
        eprintln!("注册全局快捷键 {shortcut} 失败: {err}");
    }
    Ok(())
}

fn main_window(app: &tauri::AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = main_window(app) {
        let is_visible = window.is_visible().unwrap_or(false);
        if is_visible {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }
}

fn toggle_always_on_top(app: &tauri::AppHandle) {
    if let Some(window) = main_window(app) {
        let next = !window.is_always_on_top().unwrap_or(false);
        let _ = window.set_always_on_top(next);
    }
}
