use serde::Serialize;
use std::{fmt, io};
use thiserror::Error;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("文件系统错误: {0}")]
    Io(#[from] io::Error),
    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("SQLite 错误: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("配置错误: {0}")]
    Config(String),
    #[error("进程错误: {0}")]
    Process(String),
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "io_error",
            Self::Json(_) => "json_error",
            Self::Sqlite(_) => "sqlite_error",
            Self::Config(_) => "config_error",
            Self::Process(_) => "process_error",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CommandError {
            code: self.code().to_string(),
            message: self.to_string(),
            detail: Some(format!("{self:?}")),
        }
        .serialize(serializer)
    }
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        Self::Config(value)
    }
}

impl From<&str> for AppError {
    fn from(value: &str) -> Self {
        Self::Config(value.to_string())
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

pub type AppResult<T> = Result<T, AppError>;
