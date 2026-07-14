use crate::{
    codex_config::sync_codex_config,
    codex_process::read_app_server,
    local_db::{read_local_usage, read_task_board},
    models::{CodexAccessMode, DiagnosticItem, DiagnosticStatus, UsageSnapshot},
    paths::{detect_codex_binary, detect_codex_data_dir, detect_state_db, read_codex_auth_status},
    settings::read_settings,
};
use chrono::{Datelike, Local, NaiveDate, TimeZone};

pub fn load_usage_snapshot() -> UsageSnapshot {
    let settings = read_settings().unwrap_or_default();
    let codex_binary = detect_codex_binary(&settings);
    let codex_dir = detect_codex_data_dir(&settings);
    let state_db = codex_dir.as_ref().and_then(|dir| detect_state_db(dir));
    let auth_status = read_codex_auth_status(codex_dir.as_deref());
    let mut messages = Vec::new();
    let mut diagnostics = Vec::new();

    diagnostics.push(DiagnosticItem {
        id: "codex-cli".to_string(),
        title: "Codex CLI".to_string(),
        detail: codex_binary
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| "未找到 codex 可执行文件".to_string()),
        status: if codex_binary.is_some() {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Error
        },
    });

    diagnostics.push(DiagnosticItem {
        id: "codex-login".to_string(),
        title: "ChatGPT 登录状态".to_string(),
        detail: if auth_status.is_logged_in {
            "已检测到本机 ChatGPT 登录凭据".to_string()
        } else if auth_status.mode.as_deref() == Some("apikey") {
            "当前为 API Key 模式，不使用 ChatGPT 登录".to_string()
        } else {
            "未检测到可用的 ChatGPT 登录凭据".to_string()
        },
        status: if auth_status.is_logged_in {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Warning
        },
    });

    diagnostics.push(DiagnosticItem {
        id: "codex-data".to_string(),
        title: "Codex 数据目录".to_string(),
        detail: codex_dir
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| "未找到 .codex 数据目录".to_string()),
        status: if codex_dir.is_some() {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Warning
        },
    });

    diagnostics.push(DiagnosticItem {
        id: "sqlite".to_string(),
        title: "SQLite state_5".to_string(),
        detail: state_db
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| "未找到 state_5.sqlite".to_string()),
        status: if state_db.is_some() {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Warning
        },
    });

    let use_official_account = settings.access_mode == CodexAccessMode::Official;
    if use_official_account {
        if let Err(err) = sync_codex_config(&settings) {
            messages.push(format!("官方配置修复失败: {err}"));
        }
    }

    let app_server = if use_official_account {
        match read_app_server(
            codex_binary.as_deref(),
            settings.membership_started_on.as_deref(),
        ) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                messages.push(err.to_string());
                Default::default()
            }
        }
    } else {
        Default::default()
    };
    if use_official_account {
        messages.extend(app_server.messages.clone());
    }
    diagnostics.push(DiagnosticItem {
        id: "app-server".to_string(),
        title: "Codex app-server".to_string(),
        detail: if !use_official_account {
            "API 模式使用本地统计，已跳过官方 app-server".to_string()
        } else if app_server.primary.is_some() || app_server.account.is_some() {
            "通过 Codex CLI 子命令读取官方实时数据".to_string()
        } else {
            app_server
                .messages
                .first()
                .cloned()
                .unwrap_or_else(|| "app-server 无可用数据".to_string())
        },
        status: if !use_official_account
            || app_server.primary.is_some()
            || app_server.account.is_some()
        {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Warning
        },
    });

    let settings_value_period_start = settings
        .membership_started_on
        .as_deref()
        .and_then(current_membership_cycle_start_string);
    let value_period_start = if use_official_account {
        app_server
            .official_usage
            .as_ref()
            .and_then(|usage| usage.value_period_start.as_deref())
            .or(settings_value_period_start.as_deref())
            .and_then(value_period_start_epoch)
    } else {
        settings_value_period_start
            .as_deref()
            .and_then(value_period_start_epoch)
    };
    let include_local_detailed_usage = should_include_local_detailed_usage_for_value();
    let local = state_db.as_ref().and_then(|path| {
        match read_local_usage(path, include_local_detailed_usage, value_period_start) {
            Ok(local) => Some(local),
            Err(err) => {
                messages.push(format!("SQLite 查询失败: {err}"));
                None
            }
        }
    });
    let task_board = if settings.show_task_board {
        Some(read_task_board(state_db.as_deref(), codex_dir.as_deref()))
    } else {
        None
    };

    let detailed_usage = local
        .as_ref()
        .and_then(|usage| usage.detailed_usage.as_ref());
    let session_log_detail = detailed_usage
        .map(|detailed| {
            format!(
                "解析 {} 个文件 / {} 个 token_count 事件，金额使用本地精细 token 价格",
                detailed.parsed_file_count, detailed.token_event_count
            )
        })
        .unwrap_or_else(|| "未解析到 token_count 事件，金额将降级为官方总量粗估".to_string());
    diagnostics.push(DiagnosticItem {
        id: "session-logs".to_string(),
        title: "会话令牌日志".to_string(),
        detail: session_log_detail,
        status: if detailed_usage.is_some() {
            DiagnosticStatus::Ok
        } else {
            DiagnosticStatus::Warning
        },
    });

    UsageSnapshot {
        refreshed_at: Local::now().timestamp(),
        account: app_server.account,
        auth_status,
        limit_id: app_server.limit_id,
        limit_name: app_server.limit_name,
        primary: app_server.primary,
        secondary: app_server.secondary,
        credits: app_server.credits,
        cloud_lifetime_tokens: app_server.cloud_lifetime_tokens,
        official_usage: app_server.official_usage,
        local,
        task_board,
        diagnostics,
        messages,
    }
}

fn value_period_start_epoch(value: &str) -> Option<i64> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .and_then(|naive| Local.from_local_datetime(&naive).earliest())
        .map(|date_time| date_time.timestamp())
}

fn should_include_local_detailed_usage_for_value() -> bool {
    true
}

fn current_membership_cycle_start_string(value: &str) -> Option<String> {
    let membership_started_on = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    let today = Local::now().date_naive();
    Some(
        current_membership_cycle_start(membership_started_on, today)
            .format("%Y-%m-%d")
            .to_string(),
    )
}

fn current_membership_cycle_start(membership_started_on: NaiveDate, today: NaiveDate) -> NaiveDate {
    if membership_started_on > today {
        return membership_started_on;
    }

    let anchor_day = membership_started_on.day();
    let this_month = clamped_month_date(today.year(), today.month(), anchor_day);
    if this_month <= today {
        return this_month;
    }

    let (year, month) = previous_month(today.year(), today.month());
    clamped_month_date(year, month, anchor_day)
}

fn previous_month(year: i32, month: u32) -> (i32, u32) {
    if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

fn clamped_month_date(year: i32, month: u32, day: u32) -> NaiveDate {
    let last_day = (28..=31)
        .rev()
        .find(|candidate| NaiveDate::from_ymd_opt(year, month, *candidate).is_some())
        .unwrap_or(28);
    NaiveDate::from_ymd_opt(year, month, day.min(last_day)).unwrap_or_else(|| {
        NaiveDate::from_ymd_opt(year, month, last_day)
            .expect("valid month and clamped day should create a date")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_value_period_uses_current_membership_cycle_start() {
        let membership_started_on = NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();

        assert_eq!(
            current_membership_cycle_start(membership_started_on, today),
            NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()
        );
    }

    #[test]
    fn local_detailed_usage_is_always_requested_for_value_estimates() {
        assert!(should_include_local_detailed_usage_for_value());
    }
}
