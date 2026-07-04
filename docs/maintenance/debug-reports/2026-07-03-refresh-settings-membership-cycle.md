# Refresh, Settings Save, And Membership Cycle Fix

## Read When

- When refresh becomes slow, settings appear not to save, or membership value progress shows the wrong period.
- When app startup or manual refresh appears to freeze the window while usage data is loading.

## Owner

- Desktop / Debugging

## Update Trigger

- Snapshot refresh source order, settings persistence behavior, or membership-period calculation changes.

## Validation

- Rust tests, frontend tests, lint, and build pass after the behavior change.

## Symptoms

- Refresh felt slow because the snapshot path parsed local session JSONL files on every refresh even when official account usage was already available.
- Settings save appeared ineffective because the drawer did not close on success and the frontend did not use the normalized settings returned by the Tauri command.
- Membership value progress used the configured open date as a fixed start date, so an account opened on `2026-04-10` still counted from April on `2026-07-03`.
- On `2026-07-04`, the today token card could show zero in official mode even though local Codex SQLite had current-day usage.
- After adding current-day local supplementation, the 7-day trend could show an inflated current-day bar such as `16亿` because it used `LocalUsage.dailyBuckets`.
- Startup could appear frozen because `get_usage_snapshot` directly ran app-server startup, SQLite reads, and JSONL parsing on the Tauri command path.

## Root Cause

- `load_usage_snapshot` always called `read_local_usage` with detailed JSONL parsing.
- `App.updateSettings` set local state before persistence and `SettingsDrawer` treated save as a fire-and-forget callback.
- `build_official_usage` did not roll the original membership open date forward to the current billing-cycle start.
- Official `account/usage/read` daily buckets can lag the local day; the observed response ended at `2026-07-03` while the local current date was `2026-07-04`. The frontend treated official zero for the current day as authoritative and ignored local real-time usage.
- `LocalUsage.dailyBuckets` is based on SQLite `threads.tokens_used` grouped by thread `updated_at`, so it can count the full lifetime tokens of sessions updated today. It is not a current-day token delta source.
- `get_usage_snapshot` called `load_usage_snapshot()` directly, so a slow app-server response or large JSONL parse could block the command executor enough to make the desktop window feel stuck.

## Fix

- Skip detailed local JSONL parsing when official usage exists; keep it as fallback when official data is unavailable.
- Use the saved settings returned by `save_app_settings`, refresh in the background, close the settings drawer after successful save, and show inline errors on failure.
- Convert `membershipStartedOn` into the current billing-cycle start before summing `OfficialUsage.valuePeriod`.
- In official mode, keep official data as the primary source but use JSONL detailed `LocalUsage.detailedUsage.today` for the today card and today's trend bar when official current-day usage is missing or zero.
- Run `get_usage_snapshot` and `refresh_task_board` work inside `tauri::async_runtime::spawn_blocking`, so the window renders first and data refresh completes in the background worker pool.

## Regression Risk

- If official usage is unavailable, local JSONL parsing still runs as fallback and may remain slow on very large histories.
- The current cycle assumes monthly billing anchored to the day of the configured open date.
- Current-day local supplementation can make today's value more immediate than official account buckets; source labels must keep this visible.
- If JSONL detailed usage is unavailable, do not synthesize today's official trend bar from SQLite thread daily buckets.
- Any future expensive filesystem, SQLite, app-server, or JSONL work added to IPC commands must stay off the UI/main command path.
