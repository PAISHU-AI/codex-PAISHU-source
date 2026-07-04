use crate::{
    automations::read_automation_tasks,
    error::AppResult,
    models::{
        DailyTokenBucket, LocalThread, LocalUsage, TaskBoard, TaskColumn, TaskColumnKind, TaskItem,
    },
    session_logs::{read_detailed_usage, SessionUsageSource},
};
use chrono::{Datelike, Local, TimeZone};
use rusqlite::{Connection, OpenFlags, Row};
use std::{collections::HashMap, path::Path};

pub fn read_local_usage(
    db_path: &Path,
    include_detailed_usage: bool,
    value_period_start: Option<i64>,
) -> AppResult<LocalUsage> {
    let conn = open_readonly(db_path)?;
    let windows = local_windows();

    let totals = conn.query_row(
        "SELECT
          COALESCE(SUM(tokens_used), 0) AS lifetime_tokens,
          COALESCE(SUM(CASE WHEN updated_at >= ?1 THEN tokens_used ELSE 0 END), 0) AS today_tokens,
          COALESCE(SUM(CASE WHEN updated_at >= ?2 THEN tokens_used ELSE 0 END), 0) AS seven_day_tokens,
          COUNT(*) AS thread_count,
          COALESCE(MAX(updated_at), 0) AS last_updated_at
        FROM threads",
        [windows.day_start, windows.seven_day_start],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        },
    )?;

    let recent_threads = read_recent_threads(&conn)?;
    let daily_buckets = read_daily_buckets(&conn, &windows)?;
    let detailed_usage = if include_detailed_usage {
        let sources = read_session_sources(&conn)?;
        read_detailed_usage(&sources, value_period_start)
    } else {
        None
    };

    Ok(LocalUsage {
        lifetime_tokens: totals.0,
        today_tokens: totals.1,
        seven_day_tokens: totals.2,
        thread_count: totals.3,
        last_updated_at: nonzero_epoch(totals.4),
        daily_buckets,
        recent_threads,
        detailed_usage,
    })
}

pub fn read_task_board(db_path: Option<&Path>, codex_dir: Option<&Path>) -> TaskBoard {
    let now = Local::now().timestamp();
    let windows = local_windows();
    let active_cutoff = now - 2 * 60 * 60;
    let mut active_items = Vec::new();
    let mut pending_items = Vec::new();
    let mut done_items = Vec::new();

    if let Some(db_path) = db_path {
        if let Ok(conn) = open_readonly(db_path) {
            if let Ok(items) = read_today_threads(&conn, windows.day_start, active_cutoff) {
                for item in items {
                    if item.kind == TaskColumnKind::Active {
                        active_items.push(item);
                    } else {
                        pending_items.push(item);
                    }
                }
            }
            if let Ok(items) = read_done_threads(&conn, windows.day_start) {
                done_items = items;
            }
        }
    }

    let scheduled_items = read_automation_tasks(codex_dir);
    let total_count =
        active_items.len() + pending_items.len() + scheduled_items.len() + done_items.len();
    TaskBoard {
        refreshed_at: now,
        total_count,
        columns: vec![
            make_column(TaskColumnKind::Active, "进行中", active_items),
            make_column(TaskColumnKind::Pending, "待处理", pending_items),
            make_column(TaskColumnKind::Scheduled, "定时", scheduled_items),
            make_column(TaskColumnKind::Done, "完成", done_items),
        ],
    }
}

fn open_readonly(path: &Path) -> AppResult<Connection> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)
}

fn read_recent_threads(conn: &Connection) -> AppResult<Vec<LocalThread>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, tokens_used, updated_at, model, cwd, archived
         FROM threads
         ORDER BY updated_at DESC
         LIMIT 5",
    )?;
    let threads = stmt
        .query_map([], thread_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(threads)
}

fn thread_from_row(row: &Row<'_>) -> rusqlite::Result<LocalThread> {
    Ok(LocalThread {
        id: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
        title: row
            .get::<_, Option<String>>(1)?
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "未命名会话".to_string()),
        tokens: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
        updated_at: row.get::<_, Option<i64>>(3)?,
        model: row.get::<_, Option<String>>(4)?,
        cwd: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
        archived: row.get::<_, Option<i64>>(6)?.unwrap_or(0) != 0,
    })
}

fn read_daily_buckets(
    conn: &Connection,
    windows: &LocalWindows,
) -> AppResult<Vec<DailyTokenBucket>> {
    let mut stmt = conn.prepare(
        "SELECT date(updated_at, 'unixepoch', 'localtime') AS day,
                COALESCE(SUM(tokens_used), 0) AS tokens
         FROM threads
         WHERE updated_at >= ?1
         GROUP BY day
         ORDER BY day ASC",
    )?;
    let rows = stmt.query_map([windows.seven_day_start], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let tokens_by_day = rows.collect::<Result<HashMap<_, _>, _>>()?;
    let mut buckets = Vec::new();
    for offset in (0..7).rev() {
        let date = Local::now().date_naive() - chrono::Duration::days(offset);
        let key = date.format("%Y-%m-%d").to_string();
        buckets.push(DailyTokenBucket {
            id: key.clone(),
            label: format!("{}/{}", date.month(), date.day()),
            tokens: *tokens_by_day.get(&key).unwrap_or(&0),
        });
    }
    Ok(buckets)
}

fn read_session_sources(conn: &Connection) -> AppResult<Vec<SessionUsageSource>> {
    let mut stmt = conn.prepare(
        "SELECT rollout_path, model
         FROM threads
         WHERE rollout_path IS NOT NULL
           AND rollout_path <> ''
           AND tokens_used > 0
         ORDER BY updated_at ASC",
    )?;
    let mut seen = std::collections::HashSet::new();
    let mut sources = Vec::new();
    for row in stmt.query_map([], |row| {
        Ok(SessionUsageSource {
            rollout_path: row.get::<_, String>(0)?,
            model: row.get::<_, Option<String>>(1)?,
        })
    })? {
        let source = row?;
        if seen.insert(source.rollout_path.clone()) {
            sources.push(source);
        }
    }
    Ok(sources)
}

fn read_today_threads(
    conn: &Connection,
    day_start: i64,
    active_cutoff: i64,
) -> AppResult<Vec<TaskItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, preview, cwd, tokens_used, updated_at, recency_at, model
         FROM threads
         WHERE archived = 0
           AND preview <> ''
           AND (updated_at >= ?1 OR recency_at >= ?1 OR created_at >= ?1)
         ORDER BY recency_at DESC, updated_at DESC
         LIMIT 24",
    )?;
    let items = stmt
        .query_map([day_start], |row| {
            let recency_at = row.get::<_, Option<i64>>(6)?;
            let updated_at = recency_at.or(row.get::<_, Option<i64>>(5)?);
            let kind = if updated_at.unwrap_or(0) >= active_cutoff {
                TaskColumnKind::Active
            } else {
                TaskColumnKind::Pending
            };
            make_thread_task_item(row, updated_at, kind)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

fn read_done_threads(conn: &Connection, day_start: i64) -> AppResult<Vec<TaskItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, preview, cwd, tokens_used, COALESCE(archived_at, updated_at), model
         FROM threads
         WHERE archived = 1
           AND COALESCE(archived_at, updated_at) >= ?1
         ORDER BY COALESCE(archived_at, updated_at) DESC
         LIMIT 12",
    )?;
    let items = stmt
        .query_map([day_start], |row| {
            make_thread_task_item(row, row.get::<_, Option<i64>>(5)?, TaskColumnKind::Done)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

fn make_thread_task_item(
    row: &Row<'_>,
    updated_at: Option<i64>,
    kind: TaskColumnKind,
) -> rusqlite::Result<TaskItem> {
    let raw_id = row.get::<_, Option<String>>(0)?.unwrap_or_default();
    let title = normalize_title(
        row.get::<_, Option<String>>(1)?.as_deref(),
        row.get::<_, Option<String>>(2)?.as_deref(),
    );
    let cwd = row.get::<_, Option<String>>(3)?.unwrap_or_default();
    let tokens = row.get::<_, Option<i64>>(4)?.unwrap_or(0);
    let code_suffix = raw_id.replace('-', "");
    let code_tail = code_suffix
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>()
        .to_ascii_uppercase();
    let chip = match kind {
        TaskColumnKind::Active => {
            if tokens >= 5_000_000 {
                "高耗"
            } else {
                "活跃"
            }
        }
        TaskColumnKind::Pending => {
            if tokens >= 2_000_000 {
                "中等"
            } else {
                "待机"
            }
        }
        TaskColumnKind::Scheduled => "定时",
        TaskColumnKind::Done => "完成",
    };
    let workspace = short_workspace_name(&cwd);
    let detail = [
        workspace,
        if tokens > 0 {
            Some(format_tokens(tokens))
        } else {
            None
        },
    ]
    .into_iter()
    .flatten()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" · ");

    Ok(TaskItem {
        id: format!("{raw_id}{kind:?}"),
        code: format!("COD-{code_tail}"),
        title,
        detail,
        chip: chip.to_string(),
        updated_at,
        tokens: Some(tokens),
        kind,
    })
}

fn make_column(kind: TaskColumnKind, title: &str, items: Vec<TaskItem>) -> TaskColumn {
    TaskColumn {
        id: kind,
        title: title.to_string(),
        count: items.len(),
        items: items.into_iter().take(3).collect(),
    }
}

fn normalize_title(title: Option<&str>, fallback: Option<&str>) -> String {
    title
        .or(fallback)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("未命名会话")
        .chars()
        .take(80)
        .collect()
}

fn short_workspace_name(cwd: &str) -> Option<String> {
    if cwd.is_empty() {
        return None;
    }
    std::path::Path::new(cwd)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .or_else(|| Some(cwd.to_string()))
}

fn format_tokens(value: i64) -> String {
    let abs_value = (value as f64).abs();
    if abs_value >= 100_000_000.0 {
        trim_decimal(format!("{:.1}", value as f64 / 100_000_000.0)) + "亿"
    } else if abs_value >= 10_000.0 {
        trim_decimal(format!("{:.1}", value as f64 / 10_000.0)) + "万"
    } else if abs_value >= 1_000.0 {
        trim_decimal(format!("{:.1}", value as f64 / 1_000.0)) + "千"
    } else {
        value.to_string()
    }
}

fn trim_decimal(value: String) -> String {
    value
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn nonzero_epoch(value: i64) -> Option<i64> {
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

struct LocalWindows {
    day_start: i64,
    seven_day_start: i64,
}

fn local_windows() -> LocalWindows {
    let now = Local::now();
    let day_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|naive| Local.from_local_datetime(&naive).earliest())
        .unwrap_or(now)
        .timestamp();
    LocalWindows {
        day_start,
        seven_day_start: day_start - 6 * 24 * 60 * 60,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_large_tokens() {
        assert_eq!(format_tokens(1_250_000), "125万");
        assert_eq!(format_tokens(12_300), "1.2万");
    }
}
