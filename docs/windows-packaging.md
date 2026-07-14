# Windows Packaging

## Read When

- Before changing Windows installer, WebView2 assumptions, icons, signing, or release commands.

## Owner

- Desktop / Release

## Update Trigger

- Bundle targets, Tauri config, installer metadata, signing, or Windows shell behavior changes.

## Validation

- `npm run tauri build` succeeds on Windows and generated installer launches the app.

## Requirements

- Node.js 24+
- Rust 1.92+
- Microsoft WebView2 runtime
- Tauri CLI via local npm dev dependency

## Commands

```powershell
npm install
npm run build
npm run rust:check
npm run tauri build
```

macOS cross-build for an unsigned NSIS `.exe` installer is experimental but works on this machine with `cargo-xwin`, cached Windows SDK/CRT files, NSIS, and Homebrew LLVM on `PATH`:

```zsh
PATH="$HOME/.cargo/bin:/opt/homebrew/opt/llvm/bin:$PATH" npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
```

## Expected Artifacts

Tauri writes Windows artifacts under:

```text
src-tauri/target/release/bundle/
```

The concrete `.msi` / `.exe` shape depends on installed Tauri bundler support.

## Runtime Behavior

- Global shortcut: `Ctrl+Alt+U`
- Tray left click toggles the window.
- Tray menu can show/hide, toggle topmost, and quit.
- Window is transparent and borderless with custom UI controls.

## Release Notes

- `src-tauri/icons/icon.ico` and Windows logo sizes are generated from the authoritative 光核超级服务 icon source at `src-tauri/icons/source-icon.png`.
- Code signing is not configured in source; add signing through release secrets or local build environment.
