# Architecture

## Read When

- Before changing Tauri commands, Rust data services, desktop shell behavior, or cross-platform boundaries.

## Owner

- Desktop / Architecture

## Update Trigger

- IPC commands, native capabilities, data source behavior, packaging, or platform support changes.

## Validation

- `npm run build`, `npm run rust:check`, and relevant Rust/frontend tests pass.

## Overview

`光核超级服务` is an independent Tauri 2 application under the original `codexU` repository. It does not extend the old Swift app. The old Swift implementation remains reference material for data semantics and UI information architecture.

## Layers

| Layer          | Owns                                                               | Must Not Own                                       |
| -------------- | ------------------------------------------------------------------ | -------------------------------------------------- |
| React UI       | Layout, visual state, loading/empty/error states, settings form    | Local filesystem reads, process execution, secrets |
| Frontend API   | Stable `invoke` wrappers and browser mock fallback                 | Business logic or privileged native behavior       |
| Tauri Commands | IPC boundary, typed request/response, command validation           | UI rendering                                       |
| Rust Services  | Codex app-server, SQLite, JSONL, knowledge-service proxy, settings | View-specific formatting                           |
| Desktop Shell  | Tray, global shortcut, window visibility/topmost behavior          | Data parsing logic                                 |

## Native Boundary

Stable commands:

- `get_usage_snapshot`
- `refresh_task_board`
- `get_app_settings`
- `save_app_settings`
- `list_codex_config_backups`
- `create_codex_config_backup`
- `restore_codex_config_backup`
- `delete_codex_config_backup`
- `get_detection_paths`
- `open_log_folder`
- `get_knowledge_board`
- `get_knowledge_overview`
- `set_knowledge_enabled`

Command return values are serializable Rust structs that mirror the TypeScript types in `src/types/usage.ts`.

## Platform Strategy

- Windows and macOS are first-class targets.
- Windows uses tray + topmost window rather than exact desktop-layer attachment.
- macOS keeps `Command+U` parity with the original app.
- macOS keeps a normal Dock presence and a top-menu-bar tray entry. Minimize uses the Dock; close is intercepted as hide-to-menu-bar; Dock reopen, tray click, and the global shortcut restore and focus the main window.
- Codex executable and data paths are auto-detected but can be overridden in settings.
- Account-level 7-day/30-day trends prefer Codex app-server `account/usage/read`.
- Token value cards and membership-period value progress prefer local JSONL `token_count` parsing because it exposes uncached input, cached input, and output token splits for official API-price estimation. Official aggregate usage is only a fallback for value when JSONL details are unavailable.
- Access mode settings control UI state, dashboard data-source selection, and Codex `config.toml` synchronization. Official native mode keeps official account/app-server reads; API relay mode uses local SQLite/JSONL statistics.

## Security Notes

- SQLite is opened read-only.
- Shell execution is limited to `codex app-server` and opening the app log folder.
- The UI receives diagnostics and sanitized status, not raw secrets.
- The vector knowledge API token is read only by `src-tauri/src/knowledge_board.rs`; React receives typed inventory/metrics and never receives the token or PostgreSQL credentials. The proxy accepts only a loopback service host and validates document UUIDs before write actions.
- Knowledge overviews are bounded extracts of the active persisted revision. The native proxy returns the overview but never returns the service token or direct database credentials; optional Google translation changes display state only.
- Global shortcut permissions are declared in Tauri capabilities.
- Borderless window dragging, minimize, and close use the Tauri window API and require explicit `core:window:*` permissions in `src-tauri/capabilities/default.json`.
- API relay settings store endpoint, model, reasoning effort, and speed only. Do not store API keys or secrets in the current plain JSON settings file or generated Codex config.
- Saving settings rewrites the user Codex config only through `src-tauri/src/codex_config.rs`, with a first-run default snapshot, restore snapshot, and timestamped backups. The settings UI can list, create, and restore named backup metadata, but it never receives raw config or auth contents.
