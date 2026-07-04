use crate::{
    error::AppResult,
    models::{AppSettings, DetectionPaths},
    paths,
};
use std::{fs, path::PathBuf};

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
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

pub fn write_settings(settings: &AppSettings) -> AppResult<AppSettings> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(settings)?;
    fs::write(path, text)?;
    Ok(settings.clone())
}

pub fn detection_paths() -> DetectionPaths {
    let settings = read_settings().unwrap_or_default();
    paths::detect_paths(&settings)
}
