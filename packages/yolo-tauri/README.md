# `yolo-tauri` — Tauri v3 Alpha (`dev`) & Wry/Tao/Muda/Tray-Icon Integration for Yolo UI

`yolo-tauri` bridges the **Tauri v3 Alpha (`tauri-apps` `dev` branch)** stack with **Aphrody Labs Yolo UI (`yolo-ui`)** and **Dioxus 0.8 (`dioxus-desktop` / `dioxus-fullstack`)**.

## Overview

Both **Dioxus Desktop/Mobile** and **Tauri v3 Alpha** are built on top of the exact same low-level native windowing, webview, menu, system tray, and hotkey crates maintained by the Tauri Programme (`submodules/tauri`, `submodules/wry`, `submodules/tao`, `submodules/muda`, `submodules/tray-icon`, `submodules/global-hotkey`, `submodules/window-vibrancy`, `submodules/cargo-mobile2`, `submodules/plugins-workspace`):

| Crate               | Workspace Version     | Submodule Path                 | Role in `yolo-ui` & `yolo-tauri`                                                           |
| :------------------ | :-------------------- | :----------------------------- | :----------------------------------------------------------------------------------------- |
| `tauri`             | `3.0.0-alpha` (`dev`) | `submodules/tauri`             | Full Tauri v3 runtime, ACL capabilities, CEF & Wry engines, mobile plugins                 |
| `wry`               | `0.57.0+`             | `submodules/wry`               | Cross-platform WebView rendering engine (WebView2, WKWebView, WebKitGTK, Android/iOS)      |
| `tao`               | `0.37.1+`             | `submodules/tao`               | Cross-platform window creation and event loop (`rwh_05` / `rwh_06`)                        |
| `muda`              | `0.21.2+`             | `submodules/muda`              | Native application menu bar and context menus (Windows, macOS, GTK3)                       |
| `tray-icon`         | `0.26.1+`             | `submodules/tray-icon`         | System tray / status bar icons with `libappindicator` & Muda menus                         |
| `global-hotkey`     | `0.9.0+`              | `submodules/global-hotkey`     | OS-wide global keyboard shortcuts                                                          |
| `window-vibrancy`   | `0.7.1+`              | `submodules/window-vibrancy`   | Acrylic, Mica, Vibrancy, and blurred glass window effects                                  |
| `cargo-mobile2`     | `0.20.1+`             | `submodules/cargo-mobile2`     | Android (JNI/Gradle) and iOS (Xcode) build & device orchestration                          |
| `plugins-workspace` | `v3` (`dev`)          | `submodules/plugins-workspace` | Official Tauri v3 plugins (`fs`, `shell`, `dialog`, `http`, `notification`, `store`, etc.) |

## Key Capabilities

1. **Tauri v3 Access Control List (ACL) & Capability Manifest Builder**:
   - Construct strongly-typed Tauri v3 `capabilities/*.json` manifests (`CapabilityManifest`, `PermissionEntry`, `WindowAclScope`) with scoped filesystem/network permissions and multi-webview targets.
2. **Tauri v3 Multi-Engine Shell Configuration (`Wry` + `CEF`)**:
   - Configure Tauri v3's new **Chromium Embedded Framework (`WebviewEngine::Cef`)** alongside native OS WebViews (`WebviewEngine::Wry`), native window vibrancy (`VibrancyEffect`), system tray (`SystemTrayConfig`), and global hotkeys (`GlobalHotkeyBinding`).
3. **Unified IPC Command & Streaming Channel Bridge**:
   - Zero-boilerplate JSON/binary IPC envelopes (`IpcRequest`, `IpcResponse`, `TauriChannelMsg`) that interoperate cleanly between `tauri::ipc::Channel`, `dioxus-desktop` custom protocols (`dioxus://`), and `dioxus-fullstack` `#[server]` functions.
4. **Tauri v2 -> v3 Alpha Migration Helpers**:
   - Programmatic migration diagnostics and configuration upgraders based on the official [Tauri v3 Migration Guide (`https://v3.tauri.app/start/migrate/from-tauri-2/`)](https://v3.tauri.app/start/migrate/from-tauri-2/).
