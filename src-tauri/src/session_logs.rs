use crate::{
    models::{DetailedUsage, PricedTokenUsage, TokenBreakdown},
    pricing::{estimated_cost_usd, model_token_price},
};
use chrono::{DateTime, Datelike, Local, TimeZone};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

#[derive(Debug, Clone)]
pub struct SessionUsageSource {
    pub rollout_path: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
struct SessionUsageDelta {
    epoch_secs: i64,
    tokens: TokenBreakdown,
}

#[derive(Debug, Default)]
struct DetailedUsageAccumulator {
    today: PricedTokenUsage,
    seven_day: PricedTokenUsage,
    month: PricedTokenUsage,
    value_period: Option<PricedTokenUsage>,
    lifetime: PricedTokenUsage,
    parsed_file_count: usize,
    token_event_count: usize,
}

impl DetailedUsageAccumulator {
    fn add(
        &mut self,
        tokens: TokenBreakdown,
        epoch_secs: i64,
        model: Option<&str>,
        windows: &TimeWindows,
    ) {
        let price = model_token_price(model);
        let cost = estimated_cost_usd(tokens, &price);
        self.lifetime.add(tokens, cost);
        if epoch_secs >= windows.month_start {
            self.month.add(tokens, cost);
        }
        if windows
            .value_period_start
            .is_some_and(|start| epoch_secs >= start)
        {
            self.value_period
                .get_or_insert_with(PricedTokenUsage::default)
                .add(tokens, cost);
        }
        if epoch_secs >= windows.seven_day_start {
            self.seven_day.add(tokens, cost);
        }
        if epoch_secs >= windows.day_start {
            self.today.add(tokens, cost);
        }
    }

    fn into_usage(self) -> DetailedUsage {
        DetailedUsage {
            today: self.today,
            seven_day: self.seven_day,
            month: self.month,
            value_period: self.value_period,
            lifetime: self.lifetime,
            parsed_file_count: self.parsed_file_count,
            token_event_count: self.token_event_count,
        }
    }
}

struct TimeWindows {
    day_start: i64,
    seven_day_start: i64,
    month_start: i64,
    value_period_start: Option<i64>,
}

pub fn read_detailed_usage(
    sources: &[SessionUsageSource],
    value_period_start: Option<i64>,
) -> Option<DetailedUsage> {
    if sources.is_empty() {
        return None;
    }
    let windows = current_time_windows(value_period_start);
    let mut accumulator = DetailedUsageAccumulator::default();
    if windows.value_period_start.is_some() {
        accumulator.value_period = Some(PricedTokenUsage::default());
    }

    for source in sources {
        let deltas = read_session_deltas(Path::new(&source.rollout_path));
        if deltas.is_empty() {
            continue;
        }
        accumulator.parsed_file_count += 1;
        accumulator.token_event_count += deltas.len();
        for delta in deltas {
            accumulator.add(
                delta.tokens,
                delta.epoch_secs,
                source.model.as_deref(),
                &windows,
            );
        }
    }

    if accumulator.parsed_file_count == 0 || accumulator.token_event_count == 0 {
        return None;
    }
    Some(accumulator.into_usage())
}

fn current_time_windows(value_period_start: Option<i64>) -> TimeWindows {
    let now = Local::now();
    let today = now.date_naive();
    let day_start = today
        .and_hms_opt(0, 0, 0)
        .and_then(|naive| Local.from_local_datetime(&naive).earliest())
        .unwrap_or(now)
        .timestamp();
    let seven_day_start = day_start - 6 * 24 * 60 * 60;
    let month_start = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .and_then(|naive| Local.from_local_datetime(&naive).earliest())
        .unwrap_or(now)
        .timestamp();
    TimeWindows {
        day_start,
        seven_day_start,
        month_start,
        value_period_start,
    }
}

fn read_session_deltas(path: &Path) -> Vec<SessionUsageDelta> {
    let Ok(file) = File::open(path) else {
        return Vec::new();
    };
    let reader = BufReader::new(file);
    let mut previous = TokenBreakdown::default();
    let mut deltas = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        if !line.contains("token_count") {
            continue;
        }
        let Some((epoch_secs, current)) = parse_token_count_line(&line) else {
            continue;
        };
        let mut delta = current.delta(previous);
        if delta.has_negative_value() {
            delta = current;
        }
        previous = current;
        if !delta.is_zero() {
            deltas.push(SessionUsageDelta {
                epoch_secs,
                tokens: delta,
            });
        }
    }
    deltas
}

fn parse_token_count_line(line: &str) -> Option<(i64, TokenBreakdown)> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    let timestamp = value.get("timestamp")?.as_str()?;
    let epoch_secs = DateTime::parse_from_rfc3339(timestamp).ok()?.timestamp();
    let usage = value.pointer("/payload/info/total_token_usage")?;
    Some((
        epoch_secs,
        TokenBreakdown {
            input_tokens: value_to_i64(usage.get("input_tokens")?)?,
            cached_input_tokens: value_to_i64(
                usage.get("cached_input_tokens").unwrap_or(&Value::Null),
            )
            .unwrap_or(0),
            output_tokens: value_to_i64(usage.get("output_tokens")?)?,
            reasoning_output_tokens: value_to_i64(
                usage.get("reasoning_output_tokens").unwrap_or(&Value::Null),
            )
            .unwrap_or(0),
            total_tokens: value_to_i64(usage.get("total_tokens")?)?,
        },
    ))
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
    fn parses_token_count_delta() {
        let line = r#"{"timestamp":"2026-07-03T10:00:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":25,"output_tokens":40,"reasoning_output_tokens":0,"total_tokens":140}}}}"#;
        let (_, tokens) = parse_token_count_line(line).unwrap();
        assert_eq!(tokens.input_tokens, 100);
        assert_eq!(tokens.cached_input_tokens, 25);
        assert_eq!(tokens.output_tokens, 40);
    }

    #[test]
    fn value_period_uses_token_split_pricing_from_period_start() {
        let mut accumulator = DetailedUsageAccumulator {
            value_period: Some(PricedTokenUsage::default()),
            ..Default::default()
        };
        let windows = TimeWindows {
            day_start: 0,
            seven_day_start: 0,
            month_start: 0,
            value_period_start: Some(1_000),
        };

        accumulator.add(
            TokenBreakdown {
                input_tokens: 1_000_000,
                cached_input_tokens: 400_000,
                output_tokens: 100_000,
                reasoning_output_tokens: 0,
                total_tokens: 1_100_000,
            },
            1_000,
            Some("chat-latest"),
            &windows,
        );
        let usage = accumulator.into_usage();

        assert!((usage.value_period.unwrap().estimated_cost_usd - 6.2).abs() < 0.001);
    }
}
