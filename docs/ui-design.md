# UI Design

## Read When

- Before changing dashboard layout, visual tokens, component hierarchy, or interaction states.

## Owner

- Frontend / UI

## Update Trigger

- Design tokens, dashboard sections, responsive behavior, or accessibility expectations change.

## Validation

- Frontend tests pass, production build succeeds, and 760x560 / 930x760 / 1200x860 layouts avoid overlap.

## Product Screen

The first screen is the product. There is no landing page. The dashboard is optimized for repeated desktop use.

## Locale

- The desktop UI defaults to Simplified Chinese with `html lang="zh-CN"`.
- User-facing labels, empty states, settings, diagnostics, tray menu items, task chips, and token units should remain Chinese.
- Product names and technical identifiers such as `Codex`, `CLI`, `SQLite`, `PATH`, model names, and file names may stay unchanged when translation would reduce clarity.
- Dates, numbers, currency, and compact token counts use Chinese locale-aware formatting where practical.

## Component Hierarchy

- `App`
- `HeaderBar`
  - `QuotaPanel`
  - `TokenValuePanel`
  - `TrendPanel`
- `LoginStatusCard`
- `SkillsBoard`
- `EnvironmentPanel`
  - `ErrorDisclosure`
  - `SettingsDrawer`

## Current Dashboard Behavior

- `TokenValuePanel` shows the current membership-cycle value as `本期价值估算` with a single amount. It must not show a reference cap, remaining-to-cap amount, or progress bar.
- The membership-cycle value uses the configured membership open date when available, with JSONL detailed token pricing preferred for amount accuracy.
- Dashboard data source follows the selected access mode. Official native mode shows official account data and official daily usage buckets. API relay mode shows local SQLite/JSONL usage and hides official quota windows.
- If official usage is temporarily unavailable in official native mode, token value and trend cards show local usage fallback with a clear source label instead of rendering zero-value statistics. If only the official current-day bucket is missing or zero, the today card and today's trend bar use local real-time usage and label the source as a local supplement.
- `TrendPanel` occupies the left side of the second row at desktop widths.
- `LoginStatusCard` occupies the right side of the 7-day trend row and summarizes official native login vs API relay mode, endpoint, model, reasoning effort, and speed.
- The login card settings button opens the same settings drawer; access settings are grouped near the top of the drawer.
- The settings drawer header should stay compact. Do not show the old explanatory subtitle `接入方式、路径、刷新频率、主题与任务看板行为`.
- The access settings section shows managed Codex config backup controls above `当前模式`: a backup dropdown, then `保存备份`, `恢复备份`, and `删除备份` in one row. The first app startup automatically creates the default backup so users can recover the original Codex config/auth state after later access-mode edits; deleting is disabled for the default backup.
- Official native mode shows only the mode selector and official-mode hint. Relay-only fields are hidden and reset on save because official mode uses Codex defaults.
- API relay mode shows API address, API Key, model name, reasoning effort, and speed strategy. The API address input accepts a base URL and normalizes it to exactly one trailing `/v1` when edited or saved. API Key is a password input; it is used for Codex `auth.json` sync and is not read back into the UI. API relay dashboard statistics are local because some users work without official login.
- The settings drawer save button must persist settings, close the drawer on success, and keep the drawer open with an inline error on failure. The drawer must not show a log button.
- The borderless title bar includes icon-only refresh, settings, minimize, and close controls.
- The visible full-width board under the trend/login row is the Skills board, not the old task board. It is implemented under `src/features/skills-board/`, uses a left skill list and right description pane, and all text/actions remain Chinese.
- Skills board shows only skill names, source/status, paths, and descriptions. It must not render full `SKILL.md` bodies. Delete/disable actions are disabled for system, plugin, protected, and already-disabled skills.
- Skills board search area includes status filters: `全部`, `已启用`, and `已禁用`. Disabled skills from `.codex/skills-disabled` must appear under `已禁用`, expose an enable action, and return to the enabled list after enabling. Disable and enable actions are reversible and run without a blocking browser confirmation dialog. Disabling keeps the current filter in place. Skill state buttons use green for currently enabled skills and red for currently disabled skills so mixed lists are easy to scan.
- Skills board header includes a Google Translate toggle left of refresh. `翻译` translates the selected skill description to Chinese; `取消翻译` restores the original description without changing files.
- Settings dropdown options must stay readable in dark and light themes; native option backgrounds should not become white text on white background.
- Reasoning/speed controls must show their current effective scope. Relay mode save synchronizes model, endpoint, reasoning, and speed to Codex config.

## Visual Direction

- Professional desktop dashboard.
- Dense but not cramped.
- Blue/purple Codex brand accents.
- Neutral surfaces and restrained glass.
- State colors are small-area signals only.

## Required States

Every data section must support:

- Loading
- Refreshing
- Empty
- Partial data
- Error disclosure

## Tokens

Tokens live in `src/styles/tokens.css`:

- Brand: `--brand-primary`, `--brand-strong`, `--brand-secondary`, `--brand-highlight`
- Status: success/info/warning/danger/neutral
- Data: input/cached/output
- Surfaces: window/section/card/elevated/track
- Text: primary/secondary/tertiary

## Responsive Rules

- Default window: `930x760`
- Minimum window: `760x560`
- Below `860px`, dashboard regions collapse to one column.
- Text inside task cards clamps instead of overlapping.
- Diagnostic paths truncate with ellipsis.
- The borderless window drag region is limited to the header. Interactive cards and drawer controls must remain clickable.
