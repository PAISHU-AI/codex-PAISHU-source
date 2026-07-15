# Development Log

## 2026-07-15

- Released the Knowledge Board monitoring upgrade as version `2.0.0`: the board now performs a single-flight, read-only status refresh every 200 seconds, visibly reports automatic monitoring, and keeps initial/manual refreshes as the only governed source-sync path.
- Installed the signed local `2.0.0` app in `/Applications/光核超级服务.app` and copied the verified DMG to the default delivery folder with SHA-256 `4805b90dac1c4082e73a1b2922a5418b4db8d3144183635d3013015c68d16c41`. A real manual sync after the guarded data repair reported `0` changes and `39` skips; the UI and service remain at 39 enabled documents, 0 disabled documents, and 135 active chunks.
- During the v2.0 validation, the database still contained 39 pre-existing root-relative duplicate records alongside the 39 canonical records. A `pg_restore`-verified backup at `~/Library/Application Support/PAISHU/knowledge-service/backups/paishu_knowledge-20260715-2200-before-v2-dedupe.dump` preceded the guarded cleanup. It removed only those legacy records, re-enabled the canonical set, and retained 33 inactive historical revision chunks outside the dashboard's active chunk total.
- Released the Knowledge Board repair as desktop version `1.5.1`: the installed `1.5.0` app predated the governed auto-sync command, so its healthy but empty local service could not repopulate itself. The new build includes auto-sync plus exact source reveal and recoverable knowledge deletion.
- Replaced CLI-driven package ingest with native loopback API synchronization. Document IDs now include the package namespace, preventing repeated names such as `knowledge-map.md` and `cards/glossary.md` from overwriting documents in other packages; manifest hashes keep refresh idempotent and source URIs still point to the original governed files.
- Knowledge inventory rows now display and sort by the governed Chinese package name, so repeated file titles remain readable without changing persisted knowledge content.
- Knowledge deletion now mirrors the Skills board safety model: source files move to an app-local knowledge trash, tombstones keep deleted documents hidden, vector retrieval is disabled, and service-update failures roll the file operation back.
- Cleared the final legacy runtime residue after a verified database backup: an orphaned older ingest process had recreated 39 non-canonical records and bulk-disabled the canonical set. The old process is stopped, duplicates are removed, and the stable dashboard now reports 39 unique enabled documents, 0 disabled documents, and 135 chunks.
- Added Knowledge Board auto-sync. Refresh now calls a native `sync_knowledge_sources` command that discovers governed `knowledge-retrieval` packages, ingests them into `paishu-global-v2` through the local `paishu-kb` CLI, and reports per-package failures without hiding already indexed knowledge.
- Tightened Knowledge Board auto-sync to honor package `metadata.yml`: draft/deprecated/archived/quarantined packages and `ingestion.enabled: false` are skipped, while `ingestion.mode: kb_only` syncs only the clean `kb/` layer.
- Set the Knowledge Board's default inventory filter to enabled documents so disabled legacy projections do not dominate the first view.
- Set `PAISHU-AI/codex-PAISHU-source` as the canonical public publication target for the current source and verified root-level macOS DMG; ignored build caches, runtime data, credentials, and login state remain outside Git.
- Renamed the previous user-visible product name to `光核超级服务` across Tauri bundle metadata, window/header/browser titles, runtime client metadata, README, and packaging docs while retaining `paishu-agi`, `paishu_agi`, and `com.paishu.agi` as compatibility identifiers. Replaced the desktop icon and in-app logo from the user-provided 1254×1254 PNG sources and regenerated every platform icon derivative.
- Replaced the complete visual asset set in the user-specified order: desktop platform icon source, quota/dashboard screenshot, knowledge screenshot, Skills screenshot, API relay screenshot, and the in-app header logo. Platform icons are regenerated from `src-tauri/icons/source-icon.png`.
- Added dual macOS window presence: title-bar minimize to Dock, close/hide to the top menu bar, explicit tray actions, and Dock reopen restoration. The tray toggle now restores minimized windows instead of hiding them.
- Removed transparent-window private APIs and made the window/root surface fully opaque so idle or unfocused windows no longer reveal the desktop; internal card translucency remains.
- Upgraded the knowledge service and desktop contract to v1.2 with a bounded active-revision overview endpoint plus temporary Simplified Chinese translation in the detail pane.
- Added the full-width Knowledge Board with real PostgreSQL + pgvector metrics: database capacity, knowledge/document count, vector chunk count, average read latency, read success/failure totals, searchable inventory, document details, and reversible enabled/disabled state.
- Added a localhost-only Rust knowledge proxy. The API token remains native-side, remote hosts are rejected, and document IDs are validated before state writes.
- Upgraded the PAISHU vector knowledge service to v1.1.0 with migration `002_dashboard_and_document_state.sql`, `/v1/dashboard`, `/v1/documents/{document_id}/enabled`, read latency/outcome audit events, and enabled-document filtering in hybrid retrieval.

## Read When

- When recovering recent implementation decisions or preparing a follow-up change.

## Owner

- Project Assistant

## Update Trigger

- Reusable product behavior, validation evidence, or implementation constraints change.

## Validation

- Entries include concrete changed behavior and avoid transient command logs.

## 2026-07-03

- Settings drawer no longer shows the old explanatory subtitle `接入方式、路径、刷新频率、主题与任务看板行为`.
- Official native mode now keeps relay-only controls hidden. It uses Codex default official state and does not require an API endpoint.
- API relay mode shows endpoint/model/reasoning/speed presets. Endpoint normalization accepts a base URL and stores exactly one trailing `/v1`, avoiding duplicate `/v1/v1`.
- Refresh now always parses local session JSONL detailed usage for token-value cards, while official usage remains responsible for account-level trend data.
- Member value progress now uses the current billing cycle anchored to the configured membership open date. Example: `2026-04-10` with local date `2026-07-03` starts at `2026-06-10`.
- Settings drawer save now waits for persistence, closes on success, keeps inline errors on failure, and no longer shows the log button.
- Dashboard data source now follows access mode: official native mode uses official app-server account data and `account/usage/read` daily buckets; API relay mode uses local SQLite/JSONL statistics and hides official quota windows.
- 环境诊断卡片改为固定图标列 + 可截断文本列，长本地路径不会再压住相邻卡片。
- 保存设置现在会自动同步 Codex `config.toml`：官方原生模式恢复官方 ChatGPT 配置形态，API 中转模式写入 `paishu_agi_relay` provider、模型、推理强度和速度服务层；同步前会创建恢复快照和时间戳备份。
- 官方原生模式保存会清理 app 设置中的中转端点、一次性 API Key、模型、推理强度和速度策略，并基于当前 Codex `config.toml` 原地移除中转 provider 与 `service_tier`，不会再用旧 restore 快照覆盖当前项目路径、信任记录或 MCP 配置。
- 无边框窗口标题栏新增最小化和关闭按钮；默认开发窗口宽度从 `920` 增加到 `930`。

## 2026-07-04

- Produced unsigned/ad-hoc signed macOS arm64 tester artifacts for the previous product name. Native Tauri styled DMG failed in `bundle_dmg.sh`, so tester DMG was generated with plain `hdiutil create` from the signed `.app`.
- Release packaging required a temporary official rustup `1.92.0` toolchain and `/tmp` Cargo target/cache to avoid macOS `com.apple.provenance` / proc-macro `E0463` failures observed with Rust 1.96 and Documents-path build outputs.
- macOS dev startup now enables Tauri `macOSPrivateApi` for the transparent frameless window, removing the startup warning required by macOS transparent windows.
- Local macOS validation found Homebrew Rust `1.96.0` can fail Tauri release bundling with `E0463` proc-macro crate lookup errors; debug startup, frontend build, Rust check, tests, and lint pass on the same machine.
- Skills board macOS validation now covers the real disable -> enable round trip against temporary `~/.codex`-style roots, and UI labels use `已启用` instead of the ambiguous `已启动`.
- Skills board disable/enable actions no longer depend on `window.confirm`; disabling keeps the current filter in place, while enabling still returns to the enabled list. Skill state buttons are green for currently enabled skills and red for currently disabled skills.
- 今日任务看板的可见入口替换为独立 Skills 技能看板；旧任务看板代码和 `UsageSnapshot.task_board` 聚合暂时保留，降低对统计功能的影响。
- Skills 看板前端集中在 `src/features/skills-board/`，后端集中在 `src-tauri/src/skills_board/`；前端只传 `skillId`，后端重新扫描解析路径。
- 技能删除采用安全归档到 `skills-trash`，禁用采用移动到 `skills-disabled`；系统技能、插件技能和 `yonghu-preferences` 强制只读。
- Skills 看板新增 `全部` / `已启用` / `已禁用` 筛选，已禁用技能来自 `.codex/skills-disabled` 并显示在对应列表。
- Skills 看板新增 Google 翻译切换按钮，翻译只影响当前显示的技能描述，`取消翻译` 会恢复原文，不写回 `SKILL.md`。
- Skills 看板新增启用已禁用技能能力：后端 `enable_skill` 将 `.codex/skills-disabled` 条目移回 `.codex/skills`，前端成功启用后切回 `已启用` 列表。
- 发布版本号统一更新到 `1.2.0`，并生成 Windows MSI/NSIS 安装包。子项目 `.gitignore` 排除了 `.devlogs` 和 `.dev-logs`，避免本地验证截图和日志进入独立发布仓库。

## 2026-07-13

- macOS Codex CLI 发现新增 ChatGPT 内置路径 `/Applications/ChatGPT.app/Contents/Resources/codex`；新版环境将 `app-server` 作为 `codex app-server` 子命令，而不是独立二进制文件。
- 官方模式的 `account/rateLimits/read` 请求显式发送 `params: null`，匹配当前 app-server JSON schema。
- 登录卡片新增经脱敏的本地 `auth.json` 兜底，仅确认 `chatgpt` 模式与 access token 是否存在；不会把 token、邮箱、账户 ID 或 API Key 发送给前端。app-server 暂时不可用时仍可准确展示已登录状态。
- 产品显示名现为“光核超级服务”；crate、bundle ID、API provider、备份命名与应用数据目录继续使用兼容性技术标识 `paishu_agi`。
- ChatGPT 内置 Codex 当前会把唯一的 10,080 分钟（7 天）额度窗口放在 `rateLimits.primary`，并返回空 `secondary`。额度解析改为按 `windowDurationMins` 分类，避免将实际 7 天额度错误标为 5 小时。
- 额度面板仅保留 7 天官方剩余额度百分比及中心圆环；Codex 已取消 5 小时限制，因此移除 5 小时卡片与本地 token 用量占比，不以本地数据替代官方额度。
- 额度区域重构为单环玻璃状态卡：移除旧双环的灰色外轨，采用品牌渐变进度环，并在同一信息层展示官方 7 天额度、可用/已用比例、重置时间与 7 天滚动窗口说明。
- 自动刷新设置统一限制为 200-300 秒，读取旧设置与保存新设置都会归一化到该范围。
