# Data Contract

## Read When

- Before changing Rust models, TypeScript types, IPC commands, or data parsing.

## Owner

- Desktop / Data

## Update Trigger

- Codex schema, app-server method, token pricing, task grouping, or settings shape changes.

## Validation

- Rust and TypeScript builds pass; tests cover affected parsing or rendering behavior.

## Core Types

Rust source of truth: `src-tauri/src/models.rs`

TypeScript mirror: `src/types/usage.ts`

Important models:

- `UsageSnapshot`
- `RateWindow`
- `TokenBreakdown`
- `PricedTokenUsage`
- `DetailedUsage`
- `OfficialUsage`
- `LocalUsage`
- `TaskBoard`
- `TaskItem`
- `DiagnosticItem`
- `AppSettings`

## IPC Commands

| Command                       | Purpose                                                                                             |
| ----------------------------- | --------------------------------------------------------------------------------------------------- |
| `get_usage_snapshot`          | Full quota, local usage, task board, diagnostics, messages                                          |
| `refresh_task_board`          | Lightweight task board refresh                                                                      |
| `get_app_settings`            | Read persisted app settings                                                                         |
| `save_app_settings`           | Persist settings, sync Codex config, clamp refresh interval to 200-300 seconds                      |
| `list_codex_config_backups`   | Return metadata for managed Codex config/auth backup snapshots                                      |
| `create_codex_config_backup`  | Save the current Codex `config.toml` and `auth.json` snapshot, returning the refreshed backup list  |
| `restore_codex_config_backup` | Restore a selected managed snapshot after timestamp-backing up the current Codex files              |
| `delete_codex_config_backup`  | Delete a selected non-default managed backup directory and return the refreshed backup list         |
| `get_detection_paths`         | Return detected Codex executable, data dir, DB, and log dir                                         |
| `open_log_folder`             | Open app log folder using OS shell                                                                  |
| `get_skill_board`             | Return local Codex Skills metadata for the isolated Skills board                                    |
| `disable_skill`               | Move an allowed user skill to the local `skills-disabled` folder                                    |
| `enable_skill`                | Move a disabled skill from local `skills-disabled` back to `skills`                                 |
| `archive_skill`               | Move an allowed user skill to the local `skills-trash` folder                                       |
| `open_skill_folder`           | Open a resolved skill folder using the OS file manager                                              |
| `get_knowledge_board`         | Return sanitized vector knowledge metrics and governed document inventory                           |
| `sync_knowledge_sources`      | Ingest eligible governed local knowledge packages before returning the dashboard                    |
| `get_knowledge_overview`      | Return a bounded overview from one validated document's active persisted revision                   |
| `set_knowledge_enabled`       | Enable or disable one validated knowledge document through the localhost service                    |
| `open_knowledge_source`       | Resolve a validated document again and reveal its exact governed source file in the OS file manager |
| `delete_knowledge`            | Archive a governed source file, persist a tombstone, and disable vector retrieval                   |

## Data Semantics

- `get_usage_snapshot` must run snapshot aggregation on a blocking worker thread so app-server startup, SQLite reads, and JSONL parsing do not freeze the Tauri window during startup or manual refresh.

- `RateWindow.usedPercent` comes from app-server and UI calculates remaining percent. It is shown only in official native mode. The Rust boundary classifies quota windows by `windowDurationMins`, not the unstable `primary` / `secondary` field names: windows at least 10,080 minutes populate the 7-day slot, while shorter windows populate the 5-hour slot.
- `DetailedUsage.fiveHourLocal` remains available to local analytics but is not rendered in `QuotaPanel`. The quota panel displays only the official 7-day remaining percentage; it never fabricates or substitutes a 5-hour quota.
- Refresh intervals are normalized at the Rust read/write boundary and in the UI to 200-300 seconds.
- `OfficialUsage` comes from Codex app-server `account/usage/read`, including `dailyUsageBuckets`. It is the required source for official native mode token cards, account-level daily token buckets, and 7-day/30-day trend charts.
- `AuthStatus` is a redacted local read of `.codex/auth.json`: it reports only whether ChatGPT credentials are present and the auth mode. It never returns tokens, email, account ID, or API keys, and keeps the login card accurate when app-server is temporarily unavailable.
- `OfficialUsage.valuePeriod` is the fallback source for the membership-period value estimate card because app-server daily buckets expose aggregate token totals only.
- `LocalUsage.detailedUsage.valuePeriod` is the preferred source for membership-period value estimates when JSONL `token_count` events are available. It starts at the current billing-cycle date derived from `AppSettings.membershipStartedOn`, uses uncached input, cached input, and output token splits with model-specific official API prices, and remains zero when the current cycle has no parsed token events.
- `get_usage_snapshot` follows `AppSettings.accessMode`: official native mode reads official app-server data for quota/trend/account status and still parses local JSONL details for value estimates; API relay mode skips official app-server and parses local SQLite/JSONL as the primary dashboard data source.
- On macOS, CLI discovery includes the current ChatGPT-bundled Codex binary at `/Applications/ChatGPT.app/Contents/Resources/codex`; `app-server` is invoked as the `codex app-server` subcommand rather than searched as a separate executable.
- In official native mode, frontend token value and trend cards prefer `OfficialUsage`; if official usage is unavailable but `LocalUsage` exists, they fall back to local SQLite/JSONL data instead of showing zero. If official daily usage has not yet produced the current local day bucket and official today is zero, the UI uses JSONL detailed `LocalUsage.detailedUsage.today` for the today card and today's trend bar. It must not use `LocalUsage.dailyBuckets` for this supplement because those buckets aggregate full thread `tokens_used` for sessions updated that day.
- `TokenBreakdown.cachedInputTokens` is capped by UI formatting when displaying split bars.
- JSONL `token_count` events are cumulative per session; Rust stores deltas between events and resets on negative deltas.
- Task board groups active threads if updated in the last 2 hours, pending if touched today, done if archived today, scheduled if active automation TOML is found.
- Official 7-day trend windows are calendar-day buckets ending on the local current date. Missing dates are rendered as zero-token buckets so the chart remains stable.
- Official token value is an account-level estimate using aggregate token totals and the configured GPT-5 input token rate.
- `AppSettings.accessMode` records the selected Codex access display mode: official native login or API relay. Official native mode uses the default Codex app-server/account state and does not require or display an API endpoint.
- API relay fields are `apiEndpoint`, one-time `apiKey`, `apiModel`, `reasoningEffort`, and `speedMode`. They are shown only for relay mode in the settings UI. Empty endpoint/path fields are normalized to `null`, an empty model name is normalized to `gpt-5` on save, relay endpoints are normalized to exactly one trailing `/v1`, and the dashboard uses local usage data because API users may not have official login data.
- `AppSettings.apiKey` is accepted by `save_app_settings` for writing Codex `auth.json` and is not serialized into the app `settings.json` response/storage.
- Saving API relay mode updates the user Codex `config.toml` with `model_provider = "paishu_agi_relay"`, `[model_providers.paishu_agi_relay]`, `base_url`, `wire_api = "responses"`, `preferred_auth_method = "apikey"`, and the selected model/reasoning/speed fields. It also updates Codex `auth.json` with `auth_mode = "apikey"` and `OPENAI_API_KEY`; if the UI leaves API Key empty, an existing non-empty `OPENAI_API_KEY` is preserved, otherwise save fails.
- Saving official native mode edits the current Codex `config.toml` in place: it removes the 光核超级服务 relay provider shape, clears relay-only settings (`apiEndpoint`, one-time `apiKey`, relay model/reasoning/speed choices), and restores official ChatGPT auth defaults while preserving unrelated current config sections such as project paths/trust records and MCP servers. It must not restore from an old whole-file snapshot because that can drop newer Codex-managed records. It also sets Codex `auth.json` to `auth_mode = "chatgpt"` and clears `OPENAI_API_KEY`.
- The desktop app creates one `default-initial` managed backup of Codex `config.toml` and `auth.json` on first startup before later access-mode synchronization can rewrite those files. The settings drawer can create manual managed backups, select them from a dropdown, restore them, and delete non-default backups. Restore first creates timestamped backups of the current files, then copies backed-up files back; if a file did not exist in the selected snapshot, restoring that snapshot removes the current file to match the original state. `default-initial` is protected and cannot be deleted.
- `AppSettings.membershipStartedOn` is an optional `YYYY-MM-DD` original membership open date used only for current billing-cycle value calculation. Invalid or empty dates are normalized to `null`.
- `ReasoningEffort.extreme` maps to Codex `model_reasoning_effort = "xhigh"`. `ApiSpeedMode.fast` maps to `service_tier = "priority"`; stable/balanced remove the forced service tier.
- Skills board IPC returns `SkillBoard` / `SkillSummary` from `src-tauri/src/skills_board/`. The frontend passes only `skillId`; Rust rescans and resolves the path before any filesystem operation.
- Skills board metadata reads only bounded `SKILL.md` header/frontmatter content for `name` and `description`; full skill bodies are not sent to the frontend.
- Only user skills under the local Codex `~/.codex/skills` directory are disable/delete manageable on macOS. Disabled skills under `~/.codex/skills-disabled` are enable manageable. System skills, plugin cache skills, and `yonghu-preferences` are read-only. Delete is implemented as archive to `~/.codex/skills-trash`, not permanent removal.
- Knowledge board IPC returns `KnowledgeBoard` / `KnowledgeDocumentSummary` from `src-tauri/src/knowledge_board.rs`. Rust reads the service environment from `~/Library/Application Support/PAISHU/knowledge-service/config/.env`, enforces a loopback host, keeps the API token native-side, and proxies only bounded dashboard and enable-state operations.
- `sync_knowledge_sources` runs before the React knowledge board refreshes. Rust detects governed source roots from `PAISHU_KNOWLEDGE_RETRIEVAL_DIR`, `PAISHU_KNOWLEDGE_RETRIEVAL_DIRS`, the current development `knowledge-retrieval/`, and `~/Desktop/GUANGHE-PAISHU/knowledge-retrieval`, then reads each package `metadata.yml` before ingesting. Packages with `ingestion.enabled: false`, `ingestion.mode: quarantine`, or draft/deprecated/archived status are skipped. Packages with `ingestion.mode: kb_only` send only UTF-8 text files under `kb/` to the loopback service API, carrying package `status`, `access_tier`, and `owner`. Manifest SHA-256 values skip unchanged files, and external IDs use `<package>/<relative-path>` so same-named files from different packages remain distinct while `sourceUri` remains the exact governed file path. Individual package failures are returned as dashboard messages instead of blocking the full board.
- The Knowledge Board starts with a full governed source sync and retains that path for the manual refresh button. Its v2.0 automatic monitor calls only `get_knowledge_board` every 200 seconds, uses a frontend single-flight guard to avoid overlap, and never re-ingests sources or changes document enable/delete state.
- `KnowledgeBoard.chunkCount` reports only active vector chunks. Superseded revision chunks remain in PostgreSQL for audit/history but do not contribute to the visible search-ready chunk count or retrieval index total.
- `get_knowledge_overview` validates the document UUID and returns `KnowledgeOverview` from service v1.2. The overview is extracted from the active revision, capped at 1,200 characters, and translated only in temporary React display state.
- `open_knowledge_source` accepts only a validated document UUID. Rust reloads the visible document, resolves local paths and `file:` URIs, rejects sources outside detected `knowledge-retrieval` roots, and asks the OS file manager to select the exact source file.
- `delete_knowledge` is recoverable deletion, not a database hard-delete. Rust moves an existing governed source file under the app-local `PAISHU/knowledge-service/trash` directory, writes a tombstone, disables the service document, and excludes tombstoned IDs from returned documents and dashboard counts. If disabling fails, the source move and tombstone are rolled back. A confirmation gate remains in React.
- `databaseBytes` is PostgreSQL `pg_database_size`; `averageReadMs`, `readSuccessCount`, and `readFailureCount` come from knowledge-service audit events. Legacy `knowledge.searched` events count as successful reads, while duration starts with v1.1 `knowledge.read.*` events.
- Disabling knowledge sets the service-owned `kb_documents.enabled` flag. It excludes the document from hybrid retrieval without deleting vectors, revisions, source metadata, or audit history; re-enabling is reversible.

## Error Policy

`get_usage_snapshot` prefers partial data over hard failure. Missing Codex CLI, missing SQLite, missing session logs, and app-server timeout are returned as diagnostics/messages.
