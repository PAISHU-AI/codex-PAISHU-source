use crate::{
    error::{AppError, AppResult},
    models::{AppSettings, AuthStatus, DetectionPaths},
};
use serde_json::Value;
use std::{
    env,
    path::{Path, PathBuf},
};

pub fn app_log_dir() -> AppResult<PathBuf> {
    let base = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .ok_or_else(|| AppError::Config("无法解析本地数据目录".to_string()))?;
    Ok(base.join("paishu-agi").join("logs"))
}

pub fn detect_paths(settings: &AppSettings) -> DetectionPaths {
    let codex_data_dir = detect_codex_data_dir(settings);
    let state_db_path = codex_data_dir.as_ref().and_then(|dir| detect_state_db(dir));
    let app_log_dir = app_log_dir()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| "logs".to_string());

    DetectionPaths {
        codex_binary_path: detect_codex_binary(settings)
            .map(|path| path.to_string_lossy().to_string()),
        codex_data_dir: codex_data_dir.map(|path| path.to_string_lossy().to_string()),
        state_db_path: state_db_path.map(|path| path.to_string_lossy().to_string()),
        app_log_dir,
    }
}

pub fn detect_codex_binary(settings: &AppSettings) -> Option<PathBuf> {
    if let Some(path) = settings.codex_binary_path.as_deref() {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let mut candidates = Vec::new();
    if cfg!(windows) {
        if let Some(home) = dirs::home_dir() {
            candidates.extend([
                home.join(".codex").join(".sandbox-bin").join("codex.exe"),
                home.join(".codex").join("bin").join("codex.exe"),
            ]);
        }
        if let Some(local) = dirs::data_local_dir() {
            candidates.push(local.join("Programs").join("Codex").join("codex.exe"));
        }
        if let Some(program_files) = env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(program_files).join("Codex").join("codex.exe"));
        }
    }
    if cfg!(target_os = "macos") {
        candidates.extend([
            PathBuf::from("/Applications/Codex.app/Contents/Resources/codex"),
            PathBuf::from("/Applications/ChatGPT.app/Contents/Resources/codex"),
            PathBuf::from("/opt/homebrew/bin/codex"),
            PathBuf::from("/usr/local/bin/codex"),
            PathBuf::from("/usr/bin/codex"),
        ]);
    }

    if let Some(path) = candidates
        .into_iter()
        .find(|path| is_usable_auto_candidate(path))
    {
        return Some(path);
    }

    let names: &[&str] = if cfg!(windows) {
        &["codex.exe", "codex.cmd", "codex.bat", "codex"]
    } else {
        &["codex"]
    };

    for name in names {
        if let Ok(path) = which::which(name) {
            if is_usable_auto_candidate(&path) {
                return Some(path);
            }
        }
    }

    None
}

pub fn read_codex_auth_status(codex_dir: Option<&Path>) -> AuthStatus {
    let Some(codex_dir) = codex_dir else {
        return AuthStatus::default();
    };
    let Ok(text) = std::fs::read_to_string(codex_dir.join("auth.json")) else {
        return AuthStatus::default();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return AuthStatus::default();
    };
    auth_status_from_value(&value)
}

fn auth_status_from_value(value: &Value) -> AuthStatus {
    let mode = value
        .get("auth_mode")
        .and_then(Value::as_str)
        .map(str::to_string);
    let has_chatgpt_tokens = value
        .pointer("/tokens/access_token")
        .and_then(Value::as_str)
        .is_some_and(|token| !token.trim().is_empty());
    AuthStatus {
        is_logged_in: mode.as_deref() == Some("chatgpt") && has_chatgpt_tokens,
        mode,
    }
}

fn is_usable_auto_candidate(path: &Path) -> bool {
    path.exists() && !is_windowsapps_path(path)
}

fn is_windowsapps_path(path: &Path) -> bool {
    let normalized = path
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    normalized.contains("\\program files\\windowsapps\\")
}

pub fn detect_codex_data_dir(settings: &AppSettings) -> Option<PathBuf> {
    if let Some(path) = settings.codex_data_dir.as_deref() {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    dirs::home_dir()
        .map(|home| home.join(".codex"))
        .filter(|path| path.exists())
}

pub fn detect_state_db(codex_dir: &Path) -> Option<PathBuf> {
    [
        codex_dir.join("state_5.sqlite"),
        codex_dir.join("sqlite").join("state_5.sqlite"),
    ]
    .into_iter()
    .find(|path| path.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_db_prefers_root_database() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("state_5.sqlite"), "").unwrap();
        assert_eq!(
            detect_state_db(temp.path()).unwrap(),
            temp.path().join("state_5.sqlite")
        );
    }

    #[test]
    fn skips_windowsapps_binary_for_auto_detection() {
        assert!(is_windowsapps_path(Path::new(
            r"C:\Program Files\WindowsApps\OpenAI.Codex_1\app\resources\codex.exe"
        )));
        assert!(!is_windowsapps_path(Path::new(
            r"C:\Users\me\.codex\.sandbox-bin\codex.exe"
        )));
    }

    #[test]
    fn recognizes_chatgpt_bundled_cli_path() {
        let path = Path::new("/Applications/ChatGPT.app/Contents/Resources/codex");
        assert!(path.to_string_lossy().contains("ChatGPT.app"));
    }

    #[test]
    fn chatgpt_auth_requires_mode_and_access_token() {
        let value = serde_json::json!({
            "auth_mode": "chatgpt",
            "tokens": { "access_token": "present" }
        });
        assert!(auth_status_from_value(&value).is_logged_in);

        let api_key_mode = serde_json::json!({
            "auth_mode": "apikey",
            "tokens": { "access_token": "present" }
        });
        assert!(!auth_status_from_value(&api_key_mode).is_logged_in);
    }

    #[cfg(windows)]
    #[test]
    fn prefers_sandbox_codex_binary_when_available() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let sandbox_binary = home.join(".codex").join(".sandbox-bin").join("codex.exe");
        if !sandbox_binary.exists() {
            return;
        }

        assert_eq!(
            detect_codex_binary(&AppSettings::default()).as_deref(),
            Some(sandbox_binary.as_path())
        );
    }
}
