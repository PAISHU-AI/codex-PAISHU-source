mod automations;
mod codex_config;
mod codex_process;
mod commands;
mod error;
mod knowledge_board;
mod local_db;
mod models;
mod paths;
mod pricing;
mod session_logs;
mod settings;
mod skills_board;
mod snapshot;

use commands::{
    archive_skill, create_codex_config_backup, delete_codex_config_backup, delete_knowledge,
    disable_skill, enable_skill, get_app_settings, get_detection_paths, get_knowledge_board,
    get_knowledge_overview, get_skill_board, get_usage_snapshot, list_codex_config_backups,
    open_knowledge_source, open_log_folder, open_skill_folder, refresh_task_board,
    restore_codex_config_backup, save_app_settings, set_always_on_top, set_knowledge_enabled,
    sync_knowledge_sources,
};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow,
};

pub fn run() {
    let app = tauri::Builder::default()
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
            open_skill_folder,
            sync_knowledge_sources,
            get_knowledge_board,
            get_knowledge_overview,
            set_knowledge_enabled,
            open_knowledge_source,
            delete_knowledge
        ])
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            if let Err(err) = crate::codex_config::ensure_default_codex_config_backup() {
                eprintln!("保存 Codex 默认配置备份失败: {err}");
            }
            setup_tray(app)?;
            setup_shortcut(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("构建 paishu-agi 运行时失败");

    app.run(|app_handle, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            restore_main_window(app_handle);
        }
    });
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "显示主窗口").build(app)?;
    let minimize_label = if cfg!(target_os = "macos") {
        "最小化到 Dock"
    } else {
        "最小化窗口"
    };
    let minimize = MenuItemBuilder::with_id("minimize", minimize_label).build(app)?;
    let hide = MenuItemBuilder::with_id("hide", "隐藏到顶部菜单栏").build(app)?;
    let topmost = MenuItemBuilder::with_id("topmost", "切换窗口置顶").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&show, &minimize, &hide, &topmost, &quit])
        .build()?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => restore_main_window(app),
            "minimize" => minimize_main_window(app),
            "hide" => hide_main_window(app),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowToggleAction {
    Restore,
    HideToMenuBar,
}

fn next_window_toggle_action(is_visible: bool, is_minimized: bool) -> WindowToggleAction {
    if !is_visible || is_minimized {
        WindowToggleAction::Restore
    } else {
        WindowToggleAction::HideToMenuBar
    }
}

fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = main_window(app) {
        let is_visible = window.is_visible().unwrap_or(false);
        let is_minimized = window.is_minimized().unwrap_or(false);
        match next_window_toggle_action(is_visible, is_minimized) {
            WindowToggleAction::Restore => restore_window(&window),
            WindowToggleAction::HideToMenuBar => {
                let _ = window.hide();
            }
        }
    }
}

fn restore_main_window(app: &tauri::AppHandle) {
    if let Some(window) = main_window(app) {
        restore_window(&window);
    }
}

fn restore_window(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn minimize_main_window(app: &tauri::AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.show();
        let _ = window.minimize();
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.hide();
    }
}

fn toggle_always_on_top(app: &tauri::AppHandle) {
    if let Some(window) = main_window(app) {
        let next = !window.is_always_on_top().unwrap_or(false);
        let _ = window.set_always_on_top(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_toggle_restores_hidden_or_minimized_windows_and_hides_visible_windows() {
        assert_eq!(
            next_window_toggle_action(false, false),
            WindowToggleAction::Restore
        );
        assert_eq!(
            next_window_toggle_action(true, true),
            WindowToggleAction::Restore
        );
        assert_eq!(
            next_window_toggle_action(true, false),
            WindowToggleAction::HideToMenuBar
        );
    }
}
