# ChatGPT-Bundled Codex Runtime Compatibility

## Read When

- Codex CLI or app-server is reported as missing on macOS after a Codex / ChatGPT desktop update.
- The dashboard has local SQLite data but shows no official quota or logged-out status.

## Owner

Desktop / Debugging

## Update Trigger

- Codex desktop distribution path, app-server protocol schema, or auth-file shape changes.

## Validation

- `codex --version`, `codex login status`, and `codex doctor` confirm the current environment.
- `npm run lint`, `npm run test -- --run`, `npm run build`, `npm run rust:test`, and `npm run rust:check` pass.
- The packaged `.app` passes `codesign --verify --deep --strict` and the DMG passes `hdiutil verify`.

## Symptoms

- Environment diagnostics reported both Codex CLI and Codex app-server as absent.
- The login card showed pending confirmation even with an active ChatGPT login.
- Official quota and daily usage were unavailable, although local SQLite and session logs were present.

## Root Cause

- Current macOS Codex is bundled in ChatGPT at `/Applications/ChatGPT.app/Contents/Resources/codex`; the detector only checked the retired standalone Codex.app and shell locations.
- The app-server remains a `codex app-server` subcommand. The current request schema requires `params: null` for `account/rateLimits/read`.
- The login card depended exclusively on the app-server `account/read` response, so any transient app-server startup/read failure was displayed as a logged-out state.

## Fix

- Added the ChatGPT-bundled CLI to macOS path candidates.
- Sent `params: null` for `account/rateLimits/read`.
- Added a redacted `AuthStatus` fallback from local `auth.json`: only `auth_mode` and the presence of a ChatGPT access token are evaluated; no token, account ID, email, or API key leaves Rust.
- Kept app-server as the sole real-time official quota / account-usage source and local SQLite/JSONL as the existing usage fallback.

## Regression Risk And Prevention

- A local credential presence check establishes local login state, not server-side token freshness. The app-server response remains the authority for account plan and real-time quota.
- Treat `codex app-server` as ephemeral by default; a daemon is not required for dashboard refresh.
- Recheck the generated app-server schema after a major CLI upgrade before changing request fields.

## 7-Day Window Field Migration

Current ChatGPT-bundled Codex returned a single quota window at `rateLimits.primary` with `windowDurationMins: 10080` and `secondary: null`. The dashboard formerly treated `primary` as a fixed 5-hour slot, which mislabeled the real 7-day quota and left the 7-day card empty. The parser now classifies by duration.

The app-server schema exposes no alternate short-window query. Since Codex no longer has a 5-hour limit, the dashboard intentionally omits a 5-hour card and any local substitute. The 7-day card and central ring remain the only displayed official remaining quota percentage.
