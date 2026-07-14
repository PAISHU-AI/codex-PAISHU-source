use crate::{
    error::AppResult,
    models::{AppSettings, DetectionPaths},
    paths,
};
use std::{fs, path::PathBuf};

pub const MIN_REFRESH_INTERVAL_SECS: u64 = 200;
pub const MAX_REFRESH_INTERVAL_SECS: u64 = 300;

fn settings_path() -> AppResult<PathBuf> {
    Ok(paths::app_log_dir()?
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("settings.json"))
}

pub fn read_settings() -> AppResult<AppSettings> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let text = fs::read_to_string(path)?;
    let mut settings = serde_json::from_str::<AppSettings>(&text).unwrap_or_default();
    settings.refresh_interval_secs = normalize_refresh_interval(settings.refresh_interval_secs);
    Ok(settings)
}

pub fn write_settings(settings: &AppSettings) -> AppResult<AppSettings> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut normalized = settings.clone();
    normalized.refresh_interval_secs = normalize_refresh_interval(normalized.refresh_interval_secs);
    let text = serde_json::to_string_pretty(&normalized)?;
    fs::write(path, text)?;
    Ok(normalized)
}

pub fn normalize_refresh_interval(value: u64) -> u64 {
    value.clamp(MIN_REFRESH_INTERVAL_SECS, MAX_REFRESH_INTERVAL_SECS)
}

pub fn detection_paths() -> DetectionPaths {
    let settings = read_settings().unwrap_or_default();
    paths::detect_paths(&settings)
}
