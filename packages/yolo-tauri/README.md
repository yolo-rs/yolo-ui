# `yolo-tauri`: Tauri v3 Alpha (`dev`) and Wry/Tao/Muda/Tray-Icon integration for Yolo UI

`yolo-tauri` connects the Tauri v3 Alpha stack (`tauri-apps` `dev` branch) to Aphrody Labs Yolo UI (`yolo-ui`) and Dioxus 0.8 (`dioxus-desktop` / `dioxus-fullstack`).

## Overview

Dioxus Desktop/Mobile and Tauri v3 Alpha are built on the same low-level crates for native windows, webviews, menus, system tray and hotkeys, maintained by the Tauri Programme (`submodules/tauri`, `submodules/wry`, `submodules/tao`, `submodules/muda`, `submodules/tray-icon`, `submodules/global-hotkey`, `submodules/window-vibrancy`, `submodules/cargo-mobile2`, `submodules/plugins-workspace`):

| Crate               | Workspace version     | Submodule path                 | Role in `yolo-ui` & `yolo-tauri`                                                           |
| :------------------ | :-------------------- | :----------------------------- | :----------------------------------------------------------------------------------------- |
| `tauri`             | `3.0.0-alpha` (`dev`) | `submodules/tauri`             | Tauri v3 runtime with the Wry host, ACL capabilities, and mobile plugins                   |
| `wry`               | `0.57.0+`             | `submodules/wry`               | Cross-platform WebView rendering engine (WebView2, WKWebView, WebKitGTK, Android/iOS)      |
| `tao`               | `0.37.1+`             | `submodules/tao`               | Cross-platform window creation and event loop (`rwh_05` / `rwh_06`)                        |
| `muda`              | `0.21.2+`             | `submodules/muda`              | Native application menu bar and context menus (Windows, macOS, GTK3)                       |
| `tray-icon`         | `0.26.1+`             | `submodules/tray-icon`         | System tray / status bar icons with `libappindicator` & Muda menus                         |
| `global-hotkey`     | `0.9.0+`              | `submodules/global-hotkey`     | OS-wide global keyboard shortcuts                                                          |
| `window-vibrancy`   | `0.7.1+`              | `submodules/window-vibrancy`   | Acrylic, Mica, Vibrancy, and blurred glass window effects                                  |
| `cargo-mobile2`     | `0.20.1+`             | `submodules/cargo-mobile2`     | Android (JNI/Gradle) and iOS (Xcode) build & device orchestration                          |
| `plugins-workspace` | `v3` (`dev`)          | `submodules/plugins-workspace` | Official Tauri v3 plugins (`fs`, `shell`, `dialog`, `http`, `notification`, `store`, etc.) |

## Capabilities

1. Tauri v3 access control list (ACL) and capability manifest builder:
   - Builds strongly typed Tauri v3 `capabilities/*.json` manifests (`CapabilityManifest`, `PermissionEntry`, `WindowAclScope`) with scoped filesystem/network permissions and multi-webview targets.
2. Tauri v3 Wry shell configuration:
   - Configures the Wry native OS webview with window vibrancy (`VibrancyEffect`), a system tray (`SystemTrayConfig`) and global hotkeys (`GlobalHotkeyBinding`). Chromium and CEF browser integrations belong to `yolo-web`. A CEF Tauri runtime adapter is planned there; this package does not integrate one yet.
3. Unified IPC command and streaming channel bridge:
   - JSON/binary IPC envelopes (`IpcRequest`, `IpcResponse`, `TauriChannelMsg`) that need no boilerplate and work across `tauri::ipc::Channel`, `dioxus-desktop` custom protocols (`dioxus://`) and `dioxus-fullstack` `#[server]` functions.
4. Tauri v2 to v3 Alpha migration helpers:
   - Programmatic migration diagnostics and configuration upgraders based on the official [Tauri v3 Migration Guide (`https://v3.tauri.app/start/migrate/from-tauri-2/`)](https://v3.tauri.app/start/migrate/from-tauri-2/).
