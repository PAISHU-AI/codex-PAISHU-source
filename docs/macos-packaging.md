# macOS Packaging

## Read When

- Before changing macOS bundle, signing, notarization, shortcut behavior, or Codex path detection.

## Owner

- Desktop / Release

## Update Trigger

- Bundle metadata, signing, notarization, app permissions, or macOS runtime behavior changes.

## Validation

- `npm run tauri build` succeeds on macOS and the generated `.app` launches.
- `npm run dev` starts without the transparent-window private API warning.

## Requirements

- macOS 13+
- Xcode Command Line Tools
- Node.js 24+
- Rust 1.92+

## Commands

```sh
npm install
npm run build
npm run rust:check
npm run tauri build
```

When macOS proc-macro dylibs fail with `E0463`, build with an official rustup `1.92.0` toolchain and keep `CARGO_HOME`/`CARGO_TARGET_DIR` outside protected Documents paths, for example under `/tmp`. Build `tauri/custom-protocol` once before `tauri build` so the bundle path uses a loadable `tauri_macros` dylib.

## Runtime Behavior

- Global shortcut: `Command+U`
- The main window is transparent and frameless; macOS requires `app.macOSPrivateApi: true` in `src-tauri/tauri.conf.json` and the `tauri/macos-private-api` feature in `src-tauri/Cargo.toml`.
- Codex CLI detection checks:
  - `/Applications/Codex.app/Contents/Resources/codex`
  - `/opt/homebrew/bin/codex`
  - `/usr/local/bin/codex`
  - `/usr/bin/codex`
  - `PATH`
- `.codex` data detection defaults to `~/.codex`.

## Release Notes

- `.icns` is generated from `Resources/codexU-icon.png`.
- Homebrew Rust `1.96.0` on macOS 27 reproduced intermittent Tauri release bundling failures with Rust `E0463` proc-macro crate lookup errors. Prefer the official rustup stable toolchain for release packaging if this appears.
- On macOS 27, proc-macro dylibs created under Documents can inherit `com.apple.provenance` and fail to load. The verified workaround is to use a temporary official rustup toolchain plus `/tmp` Cargo home/target for release builds.
- If Tauri's styled DMG script fails after the `.app` is produced, a simple tester DMG can be created with `hdiutil create -volname codex-PAISHU -srcfolder <app> -format UDZO <output.dmg>`. Re-sign the `.app` with `codesign --force --deep --sign - <app>` before packaging.
- Developer ID signing and notarization are intentionally not hardcoded.
- Add signing and notarization through release environment variables or a dedicated release workflow.
