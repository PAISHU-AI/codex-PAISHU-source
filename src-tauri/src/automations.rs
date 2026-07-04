use crate::models::{TaskColumnKind, TaskItem};
use serde_json::Value;
use std::{fs, path::Path};

pub fn read_automation_tasks(codex_dir: Option<&Path>) -> Vec<TaskItem> {
    let Some(codex_dir) = codex_dir else {
        return Vec::new();
    };
    let root = codex_dir.join("automations");
    let mut items = Vec::new();
    visit_automation_files(&root, &mut |path| {
        if let Some(item) = parse_automation_file(path) {
            items.push(item);
        }
    });
    items.sort_by(|a, b| a.title.cmp(&b.title));
    items
}

fn visit_automation_files(path: &Path, visitor: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_automation_files(&path, visitor);
        } else if path
            .file_name()
            .is_some_and(|name| name == "automation.toml")
        {
            visitor(&path);
        }
    }
}

fn parse_automation_file(path: &Path) -> Option<TaskItem> {
    let text = fs::read_to_string(path).ok()?;
    let value = toml::from_str::<toml::Value>(&text).ok()?;
    let status = get_string(&value, "status").unwrap_or_default();
    if status.to_ascii_uppercase() != "ACTIVE" {
        return None;
    }
    let id = get_string(&value, "id")
        .or_else(|| path.parent()?.file_name()?.to_str().map(str::to_string))
        .unwrap_or_else(|| "automation".to_string());
    let name = get_string(&value, "name").unwrap_or_else(|| "自动任务".to_string());
    let kind = get_string(&value, "kind").unwrap_or_else(|| "cron".to_string());
    let schedule = get_string(&value, "rrule")
        .map(|rrule| summarize_schedule(&rrule))
        .unwrap_or_default();
    let detail = [automation_kind_label(&kind), schedule]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");

    Some(TaskItem {
        id: format!("automation-{id}"),
        code: format!(
            "AUTO-{}",
            id.chars().take(4).collect::<String>().to_ascii_uppercase()
        ),
        title: name,
        detail,
        chip: if kind == "heartbeat" {
            "唤醒"
        } else {
            "定时"
        }
        .to_string(),
        updated_at: get_string(&value, "updated_at").and_then(|value| value.parse().ok()),
        tokens: None,
        kind: TaskColumnKind::Scheduled,
    })
}

fn automation_kind_label(kind: &str) -> String {
    match kind {
        "heartbeat" => "心跳".to_string(),
        "cron" => "定时".to_string(),
        "followup" => "跟进".to_string(),
        _ => kind.chars().take(18).collect(),
    }
}

fn get_string(value: &toml::Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| match v {
        toml::Value::String(value) => Some(value.clone()),
        toml::Value::Integer(value) => Some(value.to_string()),
        toml::Value::Boolean(value) => Some(value.to_string()),
        _ => None,
    })
}

fn summarize_schedule(rrule: &str) -> String {
    let lower = rrule.to_ascii_lowercase();
    if lower.contains("freq=daily") {
        "每天".to_string()
    } else if lower.contains("freq=weekly") {
        "每周".to_string()
    } else if lower.contains("freq=hourly") {
        "每小时".to_string()
    } else {
        rrule.chars().take(28).collect()
    }
}

#[allow(dead_code)]
fn json_field_as_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| v.as_str().map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_daily_rrule() {
        assert_eq!(summarize_schedule("FREQ=DAILY;INTERVAL=1"), "每天");
    }
}
