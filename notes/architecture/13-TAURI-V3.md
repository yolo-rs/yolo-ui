# 13: Tauri v3 Alpha (`dev` / `v3`) and the Wry/Tao native desktop and mobile stack

> Current integration boundary (2026-11-04): Yolo UI integrates Tauri with
> Wry. The selected Tauri source tree contains upstream CEF-related code, but
> Yolo UI does not expose or qualify a CEF runtime. Chromium, CEF, and
> browser-engine adapters belong to `yolo-web`, where a Tauri-to-CEF adapter
> is future work. The CEF examples below describe upstream architecture only;
> they are not available Yolo UI features.

`yolo-ui` integrates Tauri v3 Alpha (`3.0.0-alpha.5`) next to Dioxus 0.8 (`dioxus-desktop` / `dioxus-mobile`) through the `yolo-tauri` crate (`packages/yolo-tauri`) and 15 forked and synced `aphrody-labs/*` submodules under `submodules/*`.

## 1. `tauri-apps` -> `aphrody-labs` submodule fleet

`dioxus-desktop` and `tauri` v3 use the same windowing, webview, menu, system tray, hotkey, vibrancy, and mobile build crates:

| Submodule path | Upstream and fork (`aphrody-labs/*`) | Branch | Crate version | Role in `yolo-ui` |
| :--- | :--- | :--- | :--- | :--- |
| `submodules/tauri` | `aphrody-labs/tauri` | `v3` (`dev`) | `3.0.0-alpha.5` | Selected Tauri source (`tauri`, `tauri-runtime`, `tauri-runtime-wry`, bundler, CLI); CEF source is not integrated by Yolo UI |
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

## 2. How `dioxus-desktop` and `yolo-tauri` share the native stack

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

## 3. Tauri 2.0 -> 3.0 Alpha migration reference (`https://v3.tauri.app/start/migrate/from-tauri-2/`)

### 3.1 Decoupled webview runtimes (upstream reference)
Tauri 3.0 Alpha no longer selects the webview runtime through Cargo features (`wry` / `cef`) on the `tauri` crate. Applications depend directly on a runtime crate and pass its attributes to `tauri::Builder::runtime(...)`:

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
Omitting `.runtime(...)` fails at runtime with `tauri_runtime::Error::RuntimeNotConfigured`.

### 3.2 Type-erased `tauri::DynRuntime` by default
- `tauri::Builder::default()` and generic handles (`tauri::AppHandle`, `tauri::Window`, `tauri::Webview`, `tauri::WebviewWindow`) now default to the type-erased `tauri::DynRuntime` instead of `tauri::Wry`.
- `tauri::Wry` and `tauri::WryHandle` re-exports were removed from `tauri`.
- Static dispatch is still available through `tauri::Builder::<tauri_runtime_wry::WryRuntime>::new()`.

### 3.3 Runtime extension traits (`AppHandleWryExt`, `WebviewWryExt`, `WebviewCefExt`)
Runtime-specific APIs moved out of `tauri` into extension traits on the runtime crates:

| Tauri 2.0 API | Tauri 3.0 Alpha replacement |
| :--- | :--- |
| `tauri::tao`, `tauri::wry` | `tauri_runtime_wry::{tao, wry}` |
| `AppHandle::create_tao_window` | `tauri_runtime_wry::AppHandleWryExt::create_tao_window` |
| `App::wry_plugin` | `tauri_runtime_wry::AppWryExt::wry_plugin` |
| `Webview::with_webview` | `tauri_runtime_wry::WebviewWryExt::with_wry_webview` or `PlatformWebview::downcast_ref` |
| `tauri::webview_version()` | `app.webview_version()` / `app_handle.webview_version()` |
| `tauri::android_binding!` | `tauri_runtime_wry::android_binding!` |

### 3.4 Linux GTK3 / GTK4 selection and the `ksni` D-Bus tray backend
- `tauri` no longer links GTK unconditionally on Linux. `tauri-runtime-wry` enables `tauri/gtk3` (`gtk = "0.18"`) and `tauri-runtime-cef` enables `tauri/gtk4` (`gtk4 = "0.11.4"`). Accessing the wrong GTK version returns `tauri::Error::GtkVersionMismatch`.
- Linux system tray icons now default to `ksni` (pure D-Bus `StatusNotifierItem`), which removes the `libayatana-appindicator` build and runtime dependency. The legacy `libappindicator` backend is still available as an opt-in through `features = ["linux-libappindicator"]`.

### 3.5 On-demand ACL command permissions and `allow-*` / `deny-*` wildcards
- Tauri no longer writes the autogenerated `allow-$command` and `deny-$command` files into the source tree. `permissions/autogenerated/commands` is removed, and the files go to `OUT_DIR`.
- App capabilities can now use implicit `allow-*` and `deny-*` wildcard entries to allow or deny all commands registered via `tauri_build::AppManifest::commands`.
- `yolo-tauri::CapabilityManifest` supports `PermissionEntry::allow_all_app_commands()` (`"allow-*"`) and `PermissionEntry::deny_all_app_commands()` (`"deny-*"`).

### 3.6 Live development resources and macOS 26 Liquid Glass icons
- In `tauri dev` / `cargo run`, `bundle > resources` resolve directly from their source paths and are not copied into `target/`, so editing assets does not trigger a full rebuild.
- `tauri-bundler` supports Xcode 26 Icon Composer `.icon` directories and prebuilt `Assets.car` bundles in `bundle > icon`.
