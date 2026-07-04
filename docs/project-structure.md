# Project Structure

## Read When

- Before adding, moving, or renaming project directories or major modules.

## Owner

- Project Assistant

## Update Trigger

- New source directories, docs sections, runtime entrypoints, or ownership boundaries.

## Validation

- `rg --files` reflects the documented paths and stale entries are removed.

## Root

- `package.json` - frontend scripts and Tauri CLI entry.
- `vite.config.ts` - Vite dev/build config.
- `README.md` - development start and overview.
- `.ai_project.md` - compact project memory.

## Frontend

- `src/App.tsx` - dashboard composition and settings drawer orchestration.
- `src/components/` - dashboard panels, settings drawer, status card, tests.
- `src/features/skills-board/` - isolated Skills board UI, feature-local IPC wrapper, types, styles, and component test.
- `src/lib/` - IPC wrappers, formatting helpers, mock fallback data.
- `src/styles/` - design tokens and app CSS.
- `src/types/usage.ts` - TypeScript mirror of Rust data contracts.

## Tauri / Rust

- `src-tauri/src/models.rs` - Rust source of truth for shared data models.
- `src-tauri/src/commands.rs` - IPC command handlers and settings normalization.
- `src-tauri/src/codex_config.rs` - Codex `config.toml` synchronization for official native and API relay modes.
- `src-tauri/src/codex_process.rs` - Codex app-server JSON-RPC integration.
- `src-tauri/src/local_db.rs` - local SQLite access.
- `src-tauri/src/session_logs.rs` - session JSONL parsing.
- `src-tauri/src/snapshot.rs` - usage snapshot aggregation.
- `src-tauri/src/settings.rs` - persisted settings.
- `src-tauri/src/skills_board/` - local Codex skills scanner, metadata extraction, safe disable/archive/open-folder actions.

## Docs

- `docs/data-contract.md` - update when IPC/models/settings semantics change.
- `docs/ui-design.md` - update when UI layout or behavior changes.
- `docs/maintenance/development-log.md` - append reusable implementation notes.
