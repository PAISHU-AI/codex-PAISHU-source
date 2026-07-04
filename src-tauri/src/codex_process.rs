use crate::{
    error::{AppError, AppResult},
    models::{
        AccountInfo, CreditsInfo, DailyTokenBucket, DetailedUsage, OfficialUsage, PricedTokenUsage,
        RateWindow, TokenBreakdown, ValuePeriodSource,
    },
    pricing::estimated_aggregate_cost_usd,
};
use chrono::{Datelike, Duration as ChronoDuration, Local, NaiveDate};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

const APP_SERVER_TIMEOUT_SECS: u64 = 6;

#[derive(Debug, Clone, Default)]
pub struct AppServerSnapshot {
    pub account: Option<AccountInfo>,
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub primary: Option<RateWindow>,
    pub secondary: Option<RateWindow>,
    pub credits: Option<CreditsInfo>,
    pub cloud_lifetime_tokens: Option<i64>,
    pub official_usage: Option<OfficialUsage>,
    pub messages: Vec<String>,
}

pub fn read_app_server(
    codex_path: Option<&std::path::Path>,
    membership_started_on: Option<&str>,
) -> AppResult<AppServerSnapshot> {
    let Some(codex_path) = codex_path else {
        return Ok(AppServerSnapshot {
            messages: vec!["未找到 codex 可执行文件".to_string()],
            ..Default::default()
        });
    };

    let mut command = Command::new(codex_path);
    command
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let mut child = command
        .spawn()
        .map_err(|err| AppError::Process(format!("app-server 启动失败: {err}")))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::Process("app-server 标准输入不可用".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Process("app-server 标准输出不可用".to_string()))?;

    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let _ = tx.send(line);
        }
    });

    write_json_line(
        &mut stdin,
        json!({
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": { "name": "codex-paishu", "title": "codex-PAISHU", "version": env!("CARGO_PKG_VERSION") },
                "capabilities": { "experimentalApi": true, "optOutNotificationMethods": [] }
            }
        }),
    )?;

    let deadline = Instant::now() + Duration::from_secs(APP_SERVER_TIMEOUT_SECS);
    let mut snapshot = AppServerSnapshot::default();
    let mut completed = std::collections::HashSet::new();
    let mut sent_account_requests = false;

    while completed.len() < 3 && Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let Ok(line) = rx.recv_timeout(remaining.min(Duration::from_millis(500))) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(id) = value.get("id").and_then(Value::as_i64) else {
            continue;
        };

        if id == 1 && !sent_account_requests {
            sent_account_requests = true;
            write_json_line(&mut stdin, json!({ "method": "initialized" }))?;
            write_json_line(
                &mut stdin,
                json!({ "id": 2, "method": "account/read", "params": { "refreshToken": false } }),
            )?;
            write_json_line(
                &mut stdin,
                json!({ "id": 3, "method": "account/rateLimits/read" }),
            )?;
            write_json_line(
                &mut stdin,
                json!({ "id": 4, "method": "account/usage/read" }),
            )?;
            continue;
        }

        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("未知错误")
                .to_string();
            snapshot
                .messages
                .push(format!("app-server {id}: {message}"));
            completed.insert(id);
            continue;
        }

        let Some(result) = value.get("result") else {
            completed.insert(id);
            continue;
        };
        match id {
            2 => snapshot.account = parse_account(result),
            3 => parse_rate_limits(result, &mut snapshot),
            4 => {
                snapshot.cloud_lifetime_tokens = parse_cloud_lifetime_tokens(result);
                snapshot.official_usage = parse_official_usage(result, membership_started_on);
            }
            _ => {}
        }
        completed.insert(id);
    }

    if completed.len() < 3 {
        snapshot.messages.push(format!(
            "app-server 响应超时（{APP_SERVER_TIMEOUT_SECS} 秒）"
        ));
    }

    let _ = stdin.flush();
    let _ = child.kill();
    Ok(snapshot)
}

fn write_json_line(stdin: &mut impl Write, value: Value) -> AppResult<()> {
    serde_json::to_writer(&mut *stdin, &value)?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;
    Ok(())
}

fn parse_account(result: &Value) -> Option<AccountInfo> {
    let account = result.get("account")?;
    Some(AccountInfo {
        account_type: account.get("type")?.as_str()?.to_string(),
        plan_type: account
            .get("planType")
            .and_then(Value::as_str)
            .map(str::to_string),
        email_present: account.get("email").is_some_and(|value| !value.is_null()),
    })
}

fn parse_rate_limits(result: &Value, snapshot: &mut AppServerSnapshot) {
    let limits = result
        .pointer("/rateLimitsByLimitId/codex")
        .or_else(|| result.get("rateLimits"));
    let Some(limits) = limits else {
        return;
    };

    snapshot.limit_id = limits
        .get("limitId")
        .and_then(Value::as_str)
        .map(str::to_string);
    snapshot.limit_name = limits
        .get("limitName")
        .and_then(Value::as_str)
        .map(str::to_string);
    snapshot.primary = parse_rate_window(limits.get("primary"));
    snapshot.secondary = parse_rate_window(limits.get("secondary"));

    let reset_credits = result
        .pointer("/rateLimitResetCredits/availableCount")
        .and_then(value_to_i64);

    if let Some(credits) = limits.get("credits") {
        snapshot.credits = Some(CreditsInfo {
            has_credits: credits
                .get("hasCredits")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            unlimited: credits
                .get("unlimited")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            balance: credits.get("balance").map(value_to_string),
            reset_credits,
        });
    } else if reset_credits.is_some() {
        snapshot.credits = Some(CreditsInfo {
            has_credits: false,
            unlimited: false,
            balance: None,
            reset_credits,
        });
    }
}

fn parse_rate_window(value: Option<&Value>) -> Option<RateWindow> {
    let value = value?;
    Some(RateWindow {
        used_percent: value.get("usedPercent").and_then(value_to_f64)?,
        window_duration_mins: value.get("windowDurationMins").and_then(value_to_i64),
        resets_at: value.get("resetsAt").and_then(value_to_i64),
    })
}

fn parse_cloud_lifetime_tokens(result: &Value) -> Option<i64> {
    result
        .pointer("/summary/lifetimeTokens")
        .and_then(value_to_i64)
}

fn parse_official_usage(
    result: &Value,
    membership_started_on: Option<&str>,
) -> Option<OfficialUsage> {
    let lifetime_tokens = parse_cloud_lifetime_tokens(result);
    let mut buckets = Vec::new();
    for value in result.get("dailyUsageBuckets")?.as_array()? {
        let Some(date) = value
            .get("startDate")
            .and_then(Value::as_str)
            .and_then(parse_usage_date)
        else {
            continue;
        };
        let tokens = value.get("tokens").and_then(value_to_i64).unwrap_or(0);
        buckets.push((date, tokens.max(0)));
    }
    Some(build_official_usage(
        &buckets,
        lifetime_tokens,
        membership_started_on.and_then(parse_usage_date),
        Local::now().date_naive(),
    ))
}

fn parse_usage_date(value: &str) -> Option<NaiveDate> {
    let date = value.get(..10).unwrap_or(value);
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}

fn build_official_usage(
    all_buckets: &[(NaiveDate, i64)],
    lifetime_tokens: Option<i64>,
    membership_started_on: Option<NaiveDate>,
    today: NaiveDate,
) -> OfficialUsage {
    let month_start = today.with_day(1).unwrap_or(today);
    let first_bucket_date = all_buckets.iter().map(|(date, _)| *date).min();
    let (value_period_start, value_period_source) = if let Some(start) = membership_started_on {
        (
            current_membership_cycle_start(start, today),
            ValuePeriodSource::MembershipStart,
        )
    } else if let Some(start) = first_bucket_date {
        (start, ValuePeriodSource::OfficialRecord)
    } else {
        (month_start, ValuePeriodSource::CalendarMonth)
    };
    let daily_buckets = build_daily_buckets(all_buckets, today, 7);
    let recent_daily_buckets = build_daily_buckets(all_buckets, today, 30);

    let today_tokens = daily_buckets
        .last()
        .map(|bucket| bucket.tokens)
        .unwrap_or(0);
    let seven_day_tokens = daily_buckets
        .iter()
        .map(|bucket| bucket.tokens)
        .sum::<i64>();
    let month_tokens = all_buckets
        .iter()
        .filter(|(date, _)| *date >= month_start && *date <= today)
        .map(|(_, tokens)| *tokens)
        .sum::<i64>();
    let value_period_tokens = all_buckets
        .iter()
        .filter(|(date, _)| *date >= value_period_start && *date <= today)
        .map(|(_, tokens)| *tokens)
        .sum::<i64>();
    let lifetime_tokens = lifetime_tokens
        .unwrap_or_else(|| all_buckets.iter().map(|(_, tokens)| *tokens).sum::<i64>());

    OfficialUsage {
        lifetime_tokens,
        today_tokens,
        seven_day_tokens,
        month_tokens,
        value_period_start: Some(value_period_start.format("%Y-%m-%d").to_string()),
        value_period_source,
        value_period_tokens,
        value_period: priced_total_usage(value_period_tokens),
        daily_buckets,
        recent_daily_buckets,
        detailed_usage: DetailedUsage {
            today: priced_total_usage(today_tokens),
            seven_day: priced_total_usage(seven_day_tokens),
            month: priced_total_usage(month_tokens),
            value_period: None,
            lifetime: priced_total_usage(lifetime_tokens),
            parsed_file_count: all_buckets.len(),
            token_event_count: all_buckets.len(),
        },
    }
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

fn build_daily_buckets(
    all_buckets: &[(NaiveDate, i64)],
    today: NaiveDate,
    days: i64,
) -> Vec<DailyTokenBucket> {
    (0..days)
        .rev()
        .map(|offset| {
            let date = today - ChronoDuration::days(offset);
            let tokens = all_buckets
                .iter()
                .find_map(|(bucket_date, tokens)| (*bucket_date == date).then_some(*tokens))
                .unwrap_or(0);
            DailyTokenBucket {
                id: date.format("%Y-%m-%d").to_string(),
                label: format!("{}/{}", date.month(), date.day()),
                tokens,
            }
        })
        .collect()
}

fn priced_total_usage(total_tokens: i64) -> PricedTokenUsage {
    PricedTokenUsage {
        tokens: TokenBreakdown {
            input_tokens: total_tokens.max(0),
            cached_input_tokens: 0,
            output_tokens: 0,
            reasoning_output_tokens: 0,
            total_tokens: total_tokens.max(0),
        },
        estimated_cost_usd: estimated_aggregate_cost_usd(total_tokens),
    }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|v| i64::try_from(v).ok()))
        .or_else(|| value.as_f64().map(|v| v as i64))
        .or_else(|| value.as_str()?.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_usage_uses_last_seven_calendar_days() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let usage = build_official_usage(
            &[
                (NaiveDate::from_ymd_opt(2026, 6, 27).unwrap(), 10),
                (NaiveDate::from_ymd_opt(2026, 6, 28).unwrap(), 20),
                (NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(), 40),
                (NaiveDate::from_ymd_opt(2026, 7, 3).unwrap(), 80),
            ],
            Some(200),
            None,
            today,
        );

        assert_eq!(usage.today_tokens, 80);
        assert_eq!(usage.seven_day_tokens, 150);
        assert_eq!(usage.month_tokens, 120);
        assert_eq!(usage.lifetime_tokens, 200);
        assert_eq!(usage.value_period_tokens, 150);
        assert_eq!(usage.value_period_source, ValuePeriodSource::OfficialRecord);
        assert_eq!(usage.daily_buckets.len(), 7);
        assert_eq!(usage.daily_buckets.last().unwrap().label, "7/3");
        assert_eq!(usage.recent_daily_buckets.len(), 30);
        assert_eq!(usage.recent_daily_buckets.last().unwrap().label, "7/3");
    }

    #[test]
    fn official_value_period_uses_membership_start_when_configured() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let usage = build_official_usage(
            &[
                (NaiveDate::from_ymd_opt(2026, 6, 27).unwrap(), 10),
                (NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(), 40),
                (NaiveDate::from_ymd_opt(2026, 7, 3).unwrap(), 80),
            ],
            Some(200),
            Some(NaiveDate::from_ymd_opt(2026, 7, 1).unwrap()),
            today,
        );

        assert_eq!(usage.month_tokens, 120);
        assert_eq!(usage.value_period_tokens, 120);
        assert_eq!(usage.value_period_start.as_deref(), Some("2026-07-01"));
        assert_eq!(
            usage.value_period_source,
            ValuePeriodSource::MembershipStart
        );
    }

    #[test]
    fn official_value_period_uses_current_membership_cycle() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let usage = build_official_usage(
            &[
                (NaiveDate::from_ymd_opt(2026, 4, 10).unwrap(), 10),
                (NaiveDate::from_ymd_opt(2026, 6, 9).unwrap(), 20),
                (NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(), 40),
                (NaiveDate::from_ymd_opt(2026, 7, 3).unwrap(), 80),
            ],
            Some(200),
            Some(NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()),
            today,
        );

        assert_eq!(usage.value_period_start.as_deref(), Some("2026-06-10"));
        assert_eq!(usage.value_period_tokens, 120);
        assert_eq!(
            usage.value_period_source,
            ValuePeriodSource::MembershipStart
        );
    }
}

fn value_to_f64(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

fn value_to_string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}
