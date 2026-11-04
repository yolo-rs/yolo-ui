# 13 — Tauri v3 Alpha (`dev` / `v3`) & Wry/Tao Native Desktop/Mobile Stack

`yolo-ui` integrates the **Tauri v3 Alpha (`3.0.0-alpha.5`)** ecosystem alongside **Dioxus 0.8 (`dioxus-desktop` / `dioxus-mobile`)** via the `yolo-tauri` crate (`packages/yolo-tauri`) and 15 forked/synced `aphrody-labs/*` submodules under `submodules/*`.

---

## 1. Unified `tauri-apps` -> `aphrody-labs` Submodule Fleet

Both `dioxus-desktop` and `tauri` v3 share the exact same underlying windowing, webview, menu, system tray, hotkey, vibrancy, and mobile build crates:

| Submodule Path | Upstream & Fork (`aphrody-labs/*`) | Branch | Crate Version | Role in `yolo-ui` |
| :--- | :--- | :--- | :--- | :--- |
| `submodules/tauri` | `aphrody-labs/tauri` | `v3` (`dev`) | `3.0.0-alpha.5` | Tauri v3 core (`tauri`, `tauri-runtime`, `tauri-runtime-wry` `3.0.0-alpha.5`, `tauri-runtime-cef` `3.0.0-alpha.6`, `tauri-bundler`, `tauri-cli`) |
| `submodules/wry` | `aphrody-labs/wry` | `dev` | `0.57.1` | Cross-platform WebView rendering library (WebView2, WKWebView, WebKitGTK, Android/iOS) |
| `submodules/tao` | `aphrody-labs/tao` | `dev` | `0.37.2` | Cross-platform application windowing and event loop (`rwh_05` / `rwh_06`) |
| `submodules/muda` | `aphrody-labs/muda` | `dev` | `0.21.2` | Native desktop application menu bars and context menus (Win32, AppKit, GTK3/GTK4) |
| `submodules/tray-icon` | `aphrody-labs/tray-icon` | `dev` | `0.26.1` | System tray / status bar icons (`ksni` D-Bus StatusNotifierItem & `libappindicator`) |
| `submodules/window-vibrancy` | `aphrody-labs/window-vibrancy` | `dev` | `0.8.1` | Windows 11 Mica/Acrylic and macOS `NSVisualEffectView` / macOS 26 `NSGlassEffectView` Liquid Glass |
| `submodules/global-hotkey` | `aphrody-labs/global-hotkey` | `dev` | `0.9.0` | OS-wide global keyboard shortcut registration |
| `submodules/cargo-mobile2` | `aphrody-labs/cargo-mobile2` | `dev` | `0.20.1` | Android (Gradle/JNI/NDK) & iOS (Xcode/UIKit) project generation and device tooling |
| `submodules/plugins-workspace` | `aphrody-labs/plugins-workspace` | `v3` | `3.0.0-alpha` | Official Tauri v3 plugins (`fs`, `dialog`, `shell`, `http`, `notification`, `store`, etc.) |
| `submodules/tauri-bindgen` | `aphrody-labs/tauri-bindgen` | `main` | `0.1.0` | Guest/host WIT bindgen for Tauri IPC |
| `submodules/create-tauri-app` | `aphrody-labs/create-tauri-app` | `dev` | `4.x / v3` | Project scaffolding templates (including Dioxus + Tauri templates) |
| `submodules/tauri-action` | `aphrody-labs/tauri-action` | `dev` | `v1` | GitHub Actions release & cross-compilation bundler workflow |
| `submodules/tauri-docs` | `aphrody-labs/tauri-docs` | `v3` | `v3` | Official Tauri 3.0 Alpha documentation & migration reference |
| `submodules/tauri-vscode` | `aphrody-labs/tauri-vscode` | `dev` | `v2/v3` | VS Code JSON schema & command extension for Tauri |
| `submodules/awesome-tauri` | `aphrody-labs/awesome-tauri` | `main` | — | Curated catalog of Tauri plugins, apps, and guides |

---

## 2. How `dioxus-desktop` and `yolo-tauri` Share the Native Stack

```
┌───────────────────────────────────────────────────────────────────────────┐
│                         Aphrody Labs Yolo UI App                          │
│                  (RSX Components, Signals, Router, Theme)                 │
└───────────────────┬───────────────────────────────────┬───────────────────┘
                    │                                   │
      ┌─────────────▼─────────────┐       ┌─────────────▼─────────────┐
      │      dioxus-desktop       │       │        yolo-tauri         │
      │  (In-Process VirtualDOM   │       │ (Tauri v3 ACL Manifests,  │
      │   + Sledgehammer Binary   │       │  Wry / CEF Multi-Engine,  │
      │   Mutations over Wry)     │       │  IPC Envelope & Channels) │
      └─────────────┬─────────────┘       └─────────────┬─────────────┘
                    │                                   │
      ┌─────────────┴───────────────────────────────────┴─────────────┐
      │       Shared Tauri Programme Native Crate Foundation          │
      │  • wry 0.57.1 (WebView2 / WKWebView / WebKitGTK)              │
      │  • tauri-runtime-cef 3.0.0-alpha.6 (Bundled Chromium / GTK4)  │
      │  • tao 0.37.2 (Windowing & EventLoop)                         │
      │  • muda 0.21.2 (Native Menus) + tray-icon 0.26.1 (KSNI/Tray)  │
      │  • window-vibrancy 0.8.1 (Mica, Acrylic, macOS Liquid Glass)  │
      │  • global-hotkey 0.9.0 (System-wide Accelerators)             │
      └───────────────────────────────────────────────────────────────┘
```

---

## 3. Tauri 2.0 -> 3.0 Alpha Migration Reference (`https://v3.tauri.app/start/migrate/from-tauri-2/`)

### 3.1 Decoupled Webview Runtimes (`tauri-runtime-wry` & `tauri-runtime-cef`)
In Tauri 3.0 Alpha, the webview runtime is **no longer selected via Cargo features (`wry` / `cef`) on the `tauri` crate**. Instead, applications depend directly on a runtime crate and pass its attributes to `tauri::Builder::runtime(...)`:

```toml
# Cargo.toml (Tauri 3.0 Alpha)
[build-dependencies]
tauri-build = "3.0.0-alpha.4"

[dependencies]
tauri = "3.0.0-alpha.5"
# Choose System WebView (Wry 0.57):
tauri-runtime-wry = { version = "3.0.0-alpha.5", features = ["devtools"] }
# Or choose Bundled Chromium (CEF):
# tauri-runtime-cef = { version = "3.0.0-alpha.6", features = ["devtools"] }
```

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .runtime(tauri_runtime_wry::Wry::default())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```
 omitting `.runtime(...)` fails at runtime with `tauri_runtime::Error::RuntimeNotConfigured`.

### 3.2 Type-Erased `tauri::DynRuntime` by Default
- `tauri::Builder::default()` and generic handles (`tauri::AppHandle`, `tauri::Window`, `tauri::Webview`, `tauri::WebviewWindow`) now default to the type-erased `tauri::DynRuntime` instead of `tauri::Wry`.
- `tauri::Wry` and `tauri::WryHandle` re-exports were removed from `tauri`.
- Static dispatch remains available via `tauri::Builder::<tauri_runtime_wry::WryRuntime>::new()`.

### 3.3 Runtime Extension Traits (`AppHandleWryExt`, `WebviewWryExt`, `WebviewCefExt`)
Runtime-specific APIs moved out of `tauri` into extension traits on the runtime crates:

| Tauri 2.0 API | Tauri 3.0 Alpha Replacement |
| :--- | :--- |
| `tauri::tao`, `tauri::wry` | `tauri_runtime_wry::{tao, wry}` |
| `AppHandle::create_tao_window` | `tauri_runtime_wry::AppHandleWryExt::create_tao_window` |
| `App::wry_plugin` | `tauri_runtime_wry::AppWryExt::wry_plugin` |
| `Webview::with_webview` | `tauri_runtime_wry::WebviewWryExt::with_wry_webview` or `PlatformWebview::downcast_ref` |
| `tauri::webview_version()` | `app.webview_version()` / `app_handle.webview_version()` |
| `tauri::android_binding!` | `tauri_runtime_wry::android_binding!` |

### 3.4 Linux GTK3 / GTK4 Selection & `ksni` D-Bus Tray Backend
- **GTK Version Isolation**: `tauri` no longer links GTK unconditionally on Linux. `tauri-runtime-wry` enables `tauri/gtk3` (`gtk = "0.18"`), whereas `tauri-runtime-cef` enables `tauri/gtk4` (`gtk4 = "0.11.4"`). Mismatched GTK access returns `tauri::Error::GtkVersionMismatch`.
- **D-Bus StatusNotifierItem (`ksni`) Default Tray**: Linux system tray icons now default to `ksni` (pure D-Bus `StatusNotifierItem`), eliminating the `libayatana-appindicator` build/runtime dependency. Legacy `libappindicator` remains opt-in via `features = ["linux-libappindicator"]`.

### 3.5 On-Demand ACL Command Permissions & `allow-*` / `deny-*` Wildcards
- Autogenerated `allow-$command` and `deny-$command` files are no longer written into the source tree (`permissions/autogenerated/commands` is removed; files are emitted to `OUT_DIR`).
- App capabilities can now use implicit `allow-*` and `deny-*` wildcard entries to allow or deny all commands registered via `tauri_build::AppManifest::commands`.
- `yolo-tauri::CapabilityManifest` supports `PermissionEntry::allow_all_app_commands()` (`"allow-*"`) and `PermissionEntry::deny_all_app_commands()` (`"deny-*"`) out of the box.

### 3.6 Live Development Resources & macOS 26 Liquid Glass Icons
- **Live Dev Resources**: In `tauri dev` / `cargo run`, `bundle > resources` are resolved directly from their source paths without copying into `target/`, preventing full rebuilds when editing assets.
- **macOS Liquid Glass Icons**: `tauri-bundler` supports Xcode 26 Icon Composer `.icon` directories and prebuilt `Assets.car` bundles in `bundle > icon`.
