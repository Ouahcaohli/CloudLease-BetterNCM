# CloudLease BetterNCM V0.3.0 — Native side-panel prototype

This is the **source/build project**, not a precompiled installable plugin. It replaces the previous full-page panel concept with a native Windows side panel intended to sit next to the 64-bit NetEase Cloud Music window.

## Intended layout

- A native Windows side window positioned to the left of the visible NetEase Cloud Music main window (or to the right if there is no room on the left).
- A real Windows month-calendar control and live local time.
- Lease list with add/delete, account identifier, start date, expiry date, and note.
- Records are saved to `%APPDATA%\CloudLease\leases.json`.
- A button inside NetEase Cloud Music toggles the native panel; the panel also has a “收纳” button.

## Important status

- Source has been authored against the published BetterNCM Rust native-plugin API example and the official `native_plugin` manifest mechanism.
- **It has not been compiled or tested against the user's live NetEase Cloud Music process in this environment.** The current container has no Rust/MSVC toolchain, so this repository includes a Windows GitHub Actions workflow to compile the x64 DLL.
- Window-title detection, native-call namespace binding, and panel behavior must be verified on the target PC. If the native call does not appear, check the BetterNCM console and the generated DLL architecture.
- This first native prototype implements basic add/delete and JSON persistence. Editing existing records, status coloring/expiry reminders, import/export, calendar-to-record filtering, and automatic resizing of the music window are not yet implemented.

## Build the installable ZIP

1. Create a new **private** or public GitHub repository and upload the contents of this project, keeping `.github/workflows/build-native.yml`.
2. In the repository, open **Actions** and run **Build CloudLease BetterNCM Native Plugin** with **Run workflow**.
3. When it finishes, open the workflow run and download artifact `CloudLease-BetterNCM-V0.3.0-installable-x64`.
4. The artifact contains the installable plugin ZIP, including `native.dll`.
5. Fully exit NetEase Cloud Music before installing/updating a plugin that includes a native DLL. Back up the existing plugin folder first. Restart the client after installation.

## Build source reference

The native plugin uses the official BetterNCM native Rust API repository: https://github.com/BetterNCM/betterncm-native-plugin-rs
The `native_plugin` manifest field is documented by the BetterNCM plugin library: https://github.com/BetterNCM/BetterNCM-Plugins
