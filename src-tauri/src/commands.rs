use crate::{
    codex_config::{
        create_codex_config_backup as create_config_backup,
        delete_codex_config_backup as delete_config_backup,
        list_codex_config_backups as list_config_backups,
        restore_codex_config_backup as restore_config_backup, sync_codex_config,
    },
    error::{AppError, AppResult},
    knowledge_board::{self, KnowledgeBoard, KnowledgeOverview},
    local_db::read_task_board,
    models::{
        ApiSpeedMode, AppSettings, CodexAccessMode, CodexConfigBackup, DetectionPaths,
        ReasoningEffort, TaskBoard, UsageSnapshot,
    },
    paths::{app_log_dir, detect_codex_data_dir, detect_state_db},
    settings::{detection_paths, normalize_refresh_interval, read_settings, write_settings},
    skills_board::{get_skill_board as load_skill_board, SkillBoard},
    snapshot::load_usage_snapshot,
};
use chrono::NaiveDate;
use std::{fs, process::Command};
use tauri::Manager;

#[tauri::command]
pub async fn get_usage_snapshot() -> AppResult<UsageSnapshot> {
    tauri::async_runtime::spawn_blocking(load_usage_snapshot)
        .await
        .map_err(|err| AppError::Process(format!("后台读取用量失败: {err}")))
}

#[tauri::command]
pub async fn refresh_task_board() -> AppResult<TaskBoard> {
    tauri::async_runtime::spawn_blocking(|| {
        let settings = read_settings().unwrap_or_default();
        let codex_dir = detect_codex_data_dir(&settings);
        let state_db = codex_dir.as_ref().and_then(|dir| detect_state_db(dir));
        read_task_board(state_db.as_deref(), codex_dir.as_deref())
    })
    .await
    .map_err(|err| AppError::Process(format!("后台刷新任务看板失败: {err}")))
}

#[tauri::command]
pub async fn get_skill_board() -> AppResult<SkillBoard> {
    tauri::async_runtime::spawn_blocking(load_skill_board)
        .await
        .map_err(|err| AppError::Process(format!("后台读取 Skills 看板失败: {err}")))?
}

#[tauri::command]
pub async fn disable_skill(skill_id: String) -> AppResult<SkillBoard> {
    tauri::async_runtime::spawn_blocking(move || crate::skills_board::disable_skill(&skill_id))
        .await
        .map_err(|err| AppError::Process(format!("后台禁用技能失败: {err}")))?
}

#[tauri::command]
pub async fn enable_skill(skill_id: String) -> AppResult<SkillBoard> {
    tauri::async_runtime::spawn_blocking(move || crate::skills_board::enable_skill(&skill_id))
        .await
        .map_err(|err| AppError::Process(format!("后台启用技能失败: {err}")))?
}

#[tauri::command]
pub async fn archive_skill(skill_id: String) -> AppResult<SkillBoard> {
    tauri::async_runtime::spawn_blocking(move || crate::skills_board::archive_skill(&skill_id))
        .await
        .map_err(|err| AppError::Process(format!("后台删除技能失败: {err}")))?
}

#[tauri::command]
pub async fn open_skill_folder(skill_id: String) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || crate::skills_board::open_skill_folder(&skill_id))
        .await
        .map_err(|err| AppError::Process(format!("后台打开技能文件夹失败: {err}")))?
}

#[tauri::command]
pub async fn get_knowledge_board() -> AppResult<KnowledgeBoard> {
    tauri::async_runtime::spawn_blocking(knowledge_board::get_knowledge_board)
        .await
        .map_err(|err| AppError::Process(format!("后台读取知识库看板失败: {err}")))?
}

#[tauri::command]
pub async fn get_knowledge_overview(document_id: String) -> AppResult<KnowledgeOverview> {
    tauri::async_runtime::spawn_blocking(move || {
        knowledge_board::get_knowledge_overview(&document_id)
    })
    .await
    .map_err(|err| AppError::Process(format!("后台读取知识概述失败: {err}")))?
}

#[tauri::command]
pub async fn set_knowledge_enabled(
    document_id: String,
    enabled: bool,
) -> AppResult<KnowledgeBoard> {
    tauri::async_runtime::spawn_blocking(move || {
        knowledge_board::set_knowledge_enabled(&document_id, enabled)
    })
    .await
    .map_err(|err| AppError::Process(format!("后台更新知识状态失败: {err}")))?
}

#[tauri::command]
pub async fn get_app_settings() -> AppResult<AppSettings> {
    read_settings()
}

#[tauri::command]
pub async fn save_app_settings(settings: AppSettings) -> AppResult<AppSettings> {
    let mut settings = normalize_settings_for_save(settings);
    sync_codex_config(&settings)?;
    settings.api_key = None;
    write_settings(&settings)
}

fn normalize_settings_for_save(mut settings: AppSettings) -> AppSettings {
    settings.refresh_interval_secs = normalize_refresh_interval(settings.refresh_interval_secs);
    settings.codex_binary_path = normalize_optional_string(settings.codex_binary_path);
    settings.codex_data_dir = normalize_optional_string(settings.codex_data_dir);
    if settings.access_mode == CodexAccessMode::Official {
        settings.api_endpoint = None;
        settings.api_key = None;
        settings.api_model = "gpt-5".to_string();
        settings.reasoning_effort = ReasoningEffort::Medium;
        settings.speed_mode = ApiSpeedMode::Balanced;
    } else {
        settings.api_endpoint = normalize_api_endpoint(settings.api_endpoint);
        settings.api_key = normalize_optional_string(settings.api_key);
        settings.api_model = normalize_required_string(&settings.api_model, "gpt-5");
    }
    settings.membership_started_on = normalize_date_string(settings.membership_started_on);
    settings
}

#[tauri::command]
pub async fn set_always_on_top(app: tauri::AppHandle, enabled: bool) -> AppResult<bool> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_always_on_top(enabled)
            .map_err(|err| AppError::Config(format!("设置窗口置顶失败: {err}")))?;
    }
    Ok(enabled)
}

#[tauri::command]
pub async fn get_detection_paths() -> AppResult<DetectionPaths> {
    Ok(detection_paths())
}

#[tauri::command]
pub async fn list_codex_config_backups() -> AppResult<Vec<CodexConfigBackup>> {
    tauri::async_runtime::spawn_blocking(list_config_backups)
        .await
        .map_err(|err| AppError::Process(format!("后台读取配置备份失败: {err}")))?
}

#[tauri::command]
pub async fn create_codex_config_backup(
    label: Option<String>,
) -> AppResult<Vec<CodexConfigBackup>> {
    tauri::async_runtime::spawn_blocking(move || create_config_backup(label))
        .await
        .map_err(|err| AppError::Process(format!("后台保存配置备份失败: {err}")))?
}

#[tauri::command]
pub async fn restore_codex_config_backup(id: String) -> AppResult<Vec<CodexConfigBackup>> {
    tauri::async_runtime::spawn_blocking(move || restore_config_backup(&id))
        .await
        .map_err(|err| AppError::Process(format!("后台恢复配置备份失败: {err}")))?
}

#[tauri::command]
pub async fn delete_codex_config_backup(id: String) -> AppResult<Vec<CodexConfigBackup>> {
    tauri::async_runtime::spawn_blocking(move || delete_config_backup(&id))
        .await
        .map_err(|err| AppError::Process(format!("后台删除配置备份失败: {err}")))?
}

#[tauri::command]
pub async fn open_log_folder() -> AppResult<String> {
    let dir = app_log_dir()?;
    fs::create_dir_all(&dir)?;
    open_path(&dir)?;
    Ok(dir.to_string_lossy().to_string())
}

fn open_path(path: &std::path::Path) -> AppResult<()> {
    #[cfg(windows)]
    {
        Command::new("explorer").arg(path).spawn()?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(path).spawn()?;
        return Ok(());
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn normalize_required_string(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_date_string(value: Option<String>) -> Option<String> {
    let trimmed = normalize_optional_string(value)?;
    NaiveDate::parse_from_str(&trimmed, "%Y-%m-%d")
        .ok()
        .map(|date| date.format("%Y-%m-%d").to_string())
}

fn normalize_api_endpoint(value: Option<String>) -> Option<String> {
    let mut endpoint = normalize_optional_string(value)?
        .trim_end_matches('/')
        .to_string();
    if !endpoint.contains("://") {
        endpoint = format!("https://{endpoint}");
    }
    while endpoint.to_ascii_lowercase().ends_with("/v1/v1") {
        let next_len = endpoint.len().saturating_sub(3);
        endpoint.truncate(next_len);
    }
    if !endpoint.to_ascii_lowercase().ends_with("/v1") {
        endpoint.push_str("/v1");
    }
    Some(endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_endpoint_normalization_adds_single_v1() {
        assert_eq!(
            normalize_api_endpoint(Some("https://api.example.com".to_string())).as_deref(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(
            normalize_api_endpoint(Some("https://api.example.com/v1".to_string())).as_deref(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(
            normalize_api_endpoint(Some("api.example.com/v1/v1/".to_string())).as_deref(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(normalize_api_endpoint(Some("   ".to_string())), None);
    }

    #[test]
    fn official_settings_clear_relay_fields_before_save() {
        let settings = AppSettings {
            access_mode: CodexAccessMode::Official,
            api_endpoint: Some("https://api.example.com/v1".to_string()),
            api_key: Some("sk-test".to_string()),
            api_model: "relay-model".to_string(),
            reasoning_effort: ReasoningEffort::Extreme,
            speed_mode: ApiSpeedMode::Fast,
            ..AppSettings::default()
        };

        let normalized = normalize_settings_for_save(settings);

        assert_eq!(normalized.access_mode, CodexAccessMode::Official);
        assert_eq!(normalized.api_endpoint, None);
        assert_eq!(normalized.api_key, None);
        assert_eq!(normalized.api_model, "gpt-5");
        assert_eq!(normalized.reasoning_effort, ReasoningEffort::Medium);
        assert_eq!(normalized.speed_mode, ApiSpeedMode::Balanced);
    }

    #[test]
    fn refresh_interval_is_limited_to_two_hundred_through_three_hundred_seconds() {
        let too_fast = normalize_settings_for_save(AppSettings {
            refresh_interval_secs: 1,
            ..AppSettings::default()
        });
        let too_slow = normalize_settings_for_save(AppSettings {
            refresh_interval_secs: 3_600,
            ..AppSettings::default()
        });

        assert_eq!(too_fast.refresh_interval_secs, 200);
        assert_eq!(too_slow.refresh_interval_secs, 300);
    }
}
