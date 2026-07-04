# Development Log

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
- 保存设置现在会自动同步 Codex `config.toml`：官方原生模式恢复官方 ChatGPT 配置形态，API 中转模式写入 `paishu_relay` provider、模型、推理强度和速度服务层；同步前会创建恢复快照和时间戳备份。
- 官方原生模式保存会清理 app 设置中的中转端点、一次性 API Key、模型、推理强度和速度策略，并基于当前 Codex `config.toml` 原地移除中转 provider 与 `service_tier`，不会再用旧 restore 快照覆盖当前项目路径、信任记录或 MCP 配置。
- 无边框窗口标题栏新增最小化和关闭按钮；默认开发窗口宽度从 `920` 增加到 `930`。

## 2026-07-04

- Produced unsigned/ad-hoc signed macOS arm64 tester artifacts: `codex-PAISHU_1.2.0_aarch64-test.dmg` and `.zip`. Native Tauri styled DMG failed in `bundle_dmg.sh`, so tester DMG was generated with plain `hdiutil create` from the signed `.app`.
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
