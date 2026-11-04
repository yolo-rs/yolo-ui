#![doc = include_str!("../README.md")]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
pub use yolo_shared::{self, APHRODY_DARK_THEME, AphrodyThemeTokens, RgbaColor};

#[cfg(feature = "desktop-shell")]
pub use tao;
#[cfg(feature = "desktop-shell")]
pub use wry;

#[cfg(all(
    feature = "desktop-shell",
    any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )
))]
pub use global_hotkey;
#[cfg(all(
    feature = "desktop-shell",
    any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )
))]
pub use muda;

#[cfg(all(
    feature = "desktop-shell",
    any(target_os = "windows", target_os = "macos", target_os = "linux")
))]
pub use tray_icon;

/// Crate version constants synchronized with the `yolo-ui` Tauri v3 Alpha (`dev` / `v3`) submodule fleet.
pub mod versions {
    pub const TAURI_V3_VERSION: &str = "3.0.0-alpha.5";
    pub const TAURI_RUNTIME_WRY_VERSION: &str = "3.0.0-alpha.5";
    pub const TAURI_RUNTIME_CEF_VERSION: &str = "3.0.0-alpha.6";
    pub const WRY_VERSION: &str = "0.57.1";
    pub const TAO_VERSION: &str = "0.37.2";
    pub const MUDA_VERSION: &str = "0.21.2";
    pub const TRAY_ICON_VERSION: &str = "0.26.1";
    pub const GLOBAL_HOTKEY_VERSION: &str = "0.9.0";
    pub const WINDOW_VIBRANCY_VERSION: &str = "0.8.1";
}

/// Errors produced by `yolo-tauri` configuration, ACL validation, or IPC serialization.
#[derive(Debug, Error)]
pub enum YoloTauriError {
    #[error("Tauri v3 runtime not configured (`tauri_runtime::Error::RuntimeNotConfigured`)")]
    RuntimeNotConfigured,
    #[error(
        "Runtime type mismatch: expected `{expected}`, found `{actual}` (`tauri_runtime::Error::RuntimeTypeMismatch`)"
    )]
    RuntimeTypeMismatch {
        expected: &'static str,
        actual: &'static str,
    },
    #[error(
        "Linux GTK version mismatch between runtime (`{runtime_gtk}`) and compile-time selection (`{compiled_gtk}`) (`tauri::Error::GtkVersionMismatch`)"
    )]
    GtkVersionMismatch {
        runtime_gtk: &'static str,
        compiled_gtk: &'static str,
    },
    #[error("Invalid capability manifest `{identifier}`: {reason}")]
    InvalidCapability { identifier: String, reason: String },
    #[error("IPC command `{command}` denied by capability ACL for window `{window}`")]
    AclDenied { command: String, window: String },
    #[error("IPC serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid URL scheme or origin: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, YoloTauriError>;

// ============================================================================
// 1. Tauri v3 Alpha Multi-Engine Shell Configuration (`Wry` + `CEF`)
// ============================================================================

/// Webview runtime engine selector introduced in Tauri 3.0 Alpha.
///
/// In Tauri 3.0, the webview runtime is no longer selected by Cargo features on `tauri`;
/// instead, applications depend on `tauri-runtime-wry` or `tauri-runtime-cef` directly
/// and pass the runtime attributes to `tauri::Builder::runtime(...)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "engine", rename_all = "kebab-case")]
pub enum WebviewEngine {
    /// System WebView via `tauri-runtime-wry` (`wry` 0.57+ / `tao` 0.37+):
    /// WebView2 on Windows, WKWebView on macOS/iOS, WebKitGTK (GTK3) on Linux, Android WebView.
    Wry,
    /// Chromium Embedded Framework (`tauri-runtime-cef` 3.0.0-alpha.6+):
    /// Ships a self-contained Chromium runtime on Windows, macOS, and Linux (GTK4).
    Cef {
        /// Optional path or channel for the bundled CEF distribution.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        distribution: Option<String>,
        /// Optional custom helper binary name on macOS/Linux.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        helper: Option<String>,
        /// Optional remote debugging port for Chrome DevTools Protocol (CDP) inspection.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        remote_debugging_port: Option<u16>,
    },
}

impl Default for WebviewEngine {
    fn default() -> Self {
        Self::Wry
    }
}

impl WebviewEngine {
    /// Returns the runtime crate name required in `Cargo.toml` for Tauri 3.0 Alpha.
    pub const fn runtime_crate_name(&self) -> &'static str {
        match self {
            Self::Wry => "tauri-runtime-wry",
            Self::Cef { .. } => "tauri-runtime-cef",
        }
    }

    /// Returns the Linux GTK version automatically enabled by the runtime crate in Tauri 3.0.
    pub const fn linux_gtk_version(&self) -> LinuxGtkBackend {
        match self {
            Self::Wry => LinuxGtkBackend::Gtk3,
            Self::Cef { .. } => LinuxGtkBackend::Gtk4,
        }
    }

    /// Resolves the custom scheme origin URL (`tauri://localhost` or `http://tauri.localhost`)
    /// defined by the runtime in Tauri 3.0 Alpha.
    pub fn custom_scheme_url(
        &self,
        scheme: &str,
        is_windows_or_android: bool,
        is_mock_runtime: bool,
    ) -> String {
        if is_mock_runtime {
            return format!("{scheme}://localhost");
        }
        match self {
            Self::Wry if is_windows_or_android => format!("http://{scheme}.localhost"),
            Self::Wry | Self::Cef { .. } => format!("{scheme}://localhost"),
        }
    }
}

/// Linux GTK binding version in Tauri 3.0 Alpha (`gtk3` vs `gtk4` features on `tauri`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinuxGtkBackend {
    /// GTK 3 (`gtk = "0.18"`), enabled automatically by `tauri-runtime-wry`.
    Gtk3,
    /// GTK 4 (`gtk4 = "0.11.4"`), enabled automatically by `tauri-runtime-cef`.
    Gtk4,
}

/// Linux system tray icon backend in Tauri 3.0 Alpha (`tray-icon` 0.26+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinuxTrayBackend {
    /// Pure D-Bus `StatusNotifierItem` (`ksni` crate) — **new default in Tauri 3.0**,
    /// removing the `libayatana-appindicator` system dependency.
    Ksni,
    /// Legacy `libappindicator` backend (opt-in via `features = ["linux-libappindicator"]`).
    LibAppIndicator,
}

impl Default for LinuxTrayBackend {
    fn default() -> Self {
        Self::Ksni
    }
}

/// Native window background blur & vibrancy effects (`window-vibrancy` 0.8.1+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VibrancyEffect {
    /// No translucent window vibrancy.
    None,
    /// Windows 11 Mica effect (`DWMWA_SYSTEMBACKDROP_TYPE = DWMSBT_MAINWINDOW`).
    Mica,
    /// Windows 11 Mica Tabbed effect (`DWMSBT_TABBEDWINDOW`).
    MicaTabbed,
    /// Windows 10/11 Acrylic blur (`DWMSBT_TRANSIENTWINDOW`).
    Acrylic,
    /// macOS `NSVisualEffectMaterial::HudWindow` dark glass.
    MacOsHudWindow,
    /// macOS `NSVisualEffectMaterial::UnderWindowBackground`.
    MacOsUnderWindow,
    /// macOS 26+ `NSGlassEffectView` Liquid Glass surface (`window-vibrancy` 0.8.1+).
    MacOsLiquidGlass,
}

impl Default for VibrancyEffect {
    fn default() -> Self {
        Self::Mica
    }
}

/// System tray icon and menu configuration (`tray-icon` 0.26+ & `muda` 0.21+).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemTrayConfig {
    pub id: String,
    pub tooltip: Option<String>,
    pub icon_path: String,
    pub menu_on_left_click: bool,
    pub linux_backend: LinuxTrayBackend,
}

impl Default for SystemTrayConfig {
    fn default() -> Self {
        Self {
            id: String::from("yolo-main-tray"),
            tooltip: Some(String::from("Aphrody Yolo UI Desktop")),
            icon_path: String::from("icons/icon.png"),
            menu_on_left_click: false,
            linux_backend: LinuxTrayBackend::Ksni,
        }
    }
}

/// Global keyboard shortcut binding (`global-hotkey` 0.9.0+).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalHotkeyBinding {
    pub id: String,
    pub accelerator: String,
    pub command: String,
}

impl GlobalHotkeyBinding {
    pub fn new(
        id: impl Into<String>,
        accelerator: impl Into<String>,
        command: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            accelerator: accelerator.into(),
            command: command.into(),
        }
    }
}

/// Unified Desktop & Mobile shell configuration for Tauri v3 Alpha and Dioxus Desktop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TauriV3Config {
    pub product_name: String,
    pub identifier: String,
    pub version: String,
    pub webview_engine: WebviewEngine,
    pub vibrancy: VibrancyEffect,
    pub devtools_enabled: bool,
    pub dynamic_acl: bool,
    pub tray: Option<SystemTrayConfig>,
    pub global_hotkeys: Vec<GlobalHotkeyBinding>,
    /// macOS 26+ Liquid Glass `.icon` bundle or `Assets.car` path supported in Tauri 3.0.
    pub macos_liquid_glass_icon: Option<String>,
}

impl Default for TauriV3Config {
    fn default() -> Self {
        Self {
            product_name: String::from("Aphrody Yolo App"),
            identifier: String::from("com.aphrody.yolo.app"),
            version: String::from("0.8.0-alpha.3"),
            webview_engine: WebviewEngine::Wry,
            vibrancy: VibrancyEffect::Mica,
            devtools_enabled: true,
            dynamic_acl: true,
            tray: Some(SystemTrayConfig::default()),
            global_hotkeys: Vec::new(),
            macos_liquid_glass_icon: None,
        }
    }
}

impl TauriV3Config {
    /// Validates that the selected runtime and Linux GTK bindings are compatible.
    pub fn validate_linux_gtk(&self, compiled_gtk: LinuxGtkBackend) -> Result<()> {
        let runtime_gtk = self.webview_engine.linux_gtk_version();
        if runtime_gtk != compiled_gtk {
            return Err(YoloTauriError::GtkVersionMismatch {
                runtime_gtk: match runtime_gtk {
                    LinuxGtkBackend::Gtk3 => "gtk3",
                    LinuxGtkBackend::Gtk4 => "gtk4",
                },
                compiled_gtk: match compiled_gtk {
                    LinuxGtkBackend::Gtk3 => "gtk3",
                    LinuxGtkBackend::Gtk4 => "gtk4",
                },
            });
        }
        Ok(())
    }
}

// ============================================================================
// 2. Tauri v3 ACL & Capability Manifest Builder
// ============================================================================

/// Window & Webview target scope for a Tauri v3 Capability (`capabilities/*.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowAclScope {
    /// Window labels targeted by this capability (e.g., `["main", "settings-*"]`).
    pub windows: Vec<String>,
    /// Optional explicit webview labels for multi-webview windows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub webviews: Vec<String>,
    /// Target platforms (`["linux", "macOS", "windows", "android", "iOS"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<String>,
}

impl Default for WindowAclScope {
    fn default() -> Self {
        Self {
            windows: vec![String::from("main")],
            webviews: Vec::new(),
            platforms: Vec::new(),
        }
    }
}

impl WindowAclScope {
    /// Checks whether a given window label matches this ACL scope (supporting `*` prefix/wildcard).
    pub fn matches_window(&self, window_label: &str) -> bool {
        self.windows.iter().any(|pattern| {
            if pattern == "*" {
                true
            } else if let Some(prefix) = pattern.strip_suffix('*') {
                window_label.starts_with(prefix)
            } else {
                pattern == window_label
            }
        })
    }
}

/// Permission entry in a Tauri v3 Capability manifest.
///
/// Includes support for Tauri 3.0's new implicit `allow-*` and `deny-*` wildcard
/// permissions for an application's own commands (`tauri_build::AppManifest::commands`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PermissionEntry {
    /// Simple permission identifier (e.g., `"core:default"`, `"allow-*"`, `"deny-*"`, `"dialog:default"`).
    Simple(String),
    /// Scoped permission with explicit `allow` and `deny` rules (e.g., `fs:scope` or `http:default`).
    Scoped {
        identifier: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        allow: Vec<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        deny: Vec<serde_json::Value>,
    },
}

impl PermissionEntry {
    /// Tauri 3.0 implicit permission that allows **all** of the app's own commands
    /// registered via `tauri_build::AppManifest::commands`.
    pub fn allow_all_app_commands() -> Self {
        Self::Simple(String::from("allow-*"))
    }

    /// Tauri 3.0 implicit permission that denies **all** of the app's own commands.
    pub fn deny_all_app_commands() -> Self {
        Self::Simple(String::from("deny-*"))
    }

    /// Creates a standard named permission (e.g., `"core:default"`).
    pub fn named(identifier: impl Into<String>) -> Self {
        Self::Simple(identifier.into())
    }

    /// Returns the identifier string of this permission entry.
    pub fn identifier(&self) -> &str {
        match self {
            Self::Simple(id) => id.as_str(),
            Self::Scoped { identifier, .. } => identifier.as_str(),
        }
    }
}

/// Strongly-typed Tauri v3 Capability manifest (`src-tauri/capabilities/<id>.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityManifest {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub identifier: String,
    pub description: String,
    #[serde(flatten)]
    pub scope: WindowAclScope,
    pub permissions: Vec<PermissionEntry>,
}

impl CapabilityManifest {
    /// Creates a new Tauri v3 capability manifest targeting the `"main"` window with `"core:default"`.
    pub fn new(identifier: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            schema: Some(String::from("../gen/schemas/desktop-schema.json")),
            identifier: identifier.into(),
            description: description.into(),
            scope: WindowAclScope::default(),
            permissions: vec![PermissionEntry::named("core:default")],
        }
    }

    /// Adds a target window label to the capability scope.
    pub fn with_window(mut self, window: impl Into<String>) -> Self {
        let w = window.into();
        if !self.scope.windows.contains(&w) {
            self.scope.windows.push(w);
        }
        self
    }

    /// Adds a permission entry to the capability manifest.
    pub fn with_permission(mut self, entry: PermissionEntry) -> Self {
        self.permissions.push(entry);
        self
    }

    /// Validates the capability manifest according to Tauri v3 ACL rules.
    pub fn validate(&self) -> Result<()> {
        if self.identifier.trim().is_empty() {
            return Err(YoloTauriError::InvalidCapability {
                identifier: self.identifier.clone(),
                reason: String::from("capability identifier must not be empty"),
            });
        }
        if self.scope.windows.is_empty() && self.scope.webviews.is_empty() {
            return Err(YoloTauriError::InvalidCapability {
                identifier: self.identifier.clone(),
                reason: String::from("at least one target window or webview must be specified"),
            });
        }
        Ok(())
    }

    /// Evaluates whether an app command `cmd` is permitted for `window_label` under this capability.
    ///
    /// Respects Tauri 3.0's `allow-*`, `deny-*`, `allow-<cmd>`, and `deny-<cmd>` resolution order
    /// (explicit deny always wins over allow).
    pub fn is_app_command_allowed(&self, window_label: &str, cmd: &str) -> bool {
        if !self.scope.matches_window(window_label) {
            return false;
        }
        let deny_exact = format!("deny-{cmd}");
        let allow_exact = format!("allow-{cmd}");
        let mut allowed = false;
        for perm in &self.permissions {
            let id = perm.identifier();
            if id == "deny-*" || id == deny_exact {
                return false;
            }
            if id == "allow-*" || id == allow_exact {
                allowed = true;
            }
        }
        allowed
    }

    /// Serializes the capability manifest to pretty-printed JSON for `src-tauri/capabilities/<id>.json`.
    pub fn to_json_pretty(&self) -> Result<String> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }
}

// ============================================================================
// 3. Unified IPC Command & Streaming Channel Envelopes (Tauri v3 <-> Dioxus)
// ============================================================================

/// IPC Command Request envelope compatible with both Tauri v3 (`tauri::webview::InvokeRequest`)
/// and Dioxus Desktop / Fullstack RPC bridges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcRequest<T = serde_json::Value> {
    pub cmd: String,
    pub callback_id: u64,
    pub error_id: u64,
    pub origin_url: String,
    pub headers: BTreeMap<String, String>,
    pub payload: T,
}

impl<T> IpcRequest<T> {
    /// Creates a new IPC request using Tauri 3.0's canonical `tauri://localhost` origin.
    pub fn new(cmd: impl Into<String>, payload: T) -> Self {
        Self {
            cmd: cmd.into(),
            callback_id: 0,
            error_id: 1,
            origin_url: String::from("tauri://localhost"),
            headers: BTreeMap::new(),
            payload,
        }
    }

    /// Validates that the IPC request is permitted by the given `CapabilityManifest` for `window_label`.
    pub fn authorize(&self, capability: &CapabilityManifest, window_label: &str) -> Result<()> {
        if capability.is_app_command_allowed(window_label, &self.cmd) {
            Ok(())
        } else {
            Err(YoloTauriError::AclDenied {
                command: self.cmd.clone(),
                window: window_label.to_string(),
            })
        }
    }
}

/// IPC Command Response envelope returned from Tauri v3 commands or Dioxus desktop handlers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcResponse<T = serde_json::Value> {
    pub callback_id: u64,
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> IpcResponse<T> {
    pub fn ok(callback_id: u64, data: T) -> Self {
        Self {
            callback_id,
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error_id: u64, message: impl Into<String>) -> Self {
        Self {
            callback_id: error_id,
            ok: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Streaming message envelope for `tauri::ipc::Channel<T>` and `dioxus-desktop` / `dioxus-liveview` streams.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TauriChannelMsg<T = serde_json::Value> {
    /// Monotonically increasing sequence index in the channel stream.
    pub seq: u64,
    /// Channel identifier matching the frontend `Channel` callback ID.
    pub channel_id: u64,
    /// Stream frame payload (`Chunk`, `Done`, or `Error`).
    pub frame: ChannelFrame<T>,
}

/// Frame variants for a `TauriChannelMsg`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
pub enum ChannelFrame<T> {
    Chunk(T),
    Done,
    Error(String),
}

// ============================================================================
// 4. Tauri v2 -> v3 Alpha Migration Auditor (`https://v3.tauri.app/start/migrate/from-tauri-2/`)
// ============================================================================

/// Actionable finding produced when auditing a Tauri 2.x project for Tauri 3.0 Alpha compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationFinding {
    pub rule_id: &'static str,
    pub description: &'static str,
    pub replacement_hint: &'static str,
}

/// Audits a `Cargo.toml` content string for breaking Tauri 2.0 -> 3.0 Alpha dependency/feature changes.
pub fn audit_tauri_v2_cargo_toml(cargo_toml: &str) -> Vec<MigrationFinding> {
    let mut findings = Vec::new();

    if cargo_toml.contains("\"wry\"") && cargo_toml.contains("tauri") {
        findings.push(MigrationFinding {
            rule_id: "TAURI-V3-001",
            description: "The `wry` and `cef` Cargo features were removed from the `tauri` crate in 3.0 Alpha.",
            replacement_hint: "Add `tauri-runtime-wry = \"3.0.0-alpha.5\"` (or `tauri-runtime-cef`) to `[dependencies]` and call `.runtime(tauri_runtime_wry::Wry::default())`.",
        });
    }

    if !cargo_toml.contains("tauri-runtime-wry") && !cargo_toml.contains("tauri-runtime-cef") {
        findings.push(MigrationFinding {
            rule_id: "TAURI-V3-002",
            description: "No webview runtime crate (`tauri-runtime-wry` or `tauri-runtime-cef`) found in `Cargo.toml`.",
            replacement_hint: "Depend directly on `tauri-runtime-wry = \"3.0.0-alpha.5\"` or `tauri-runtime-cef = \"3.0.0-alpha.6\"`.",
        });
    }

    if cargo_toml.contains("macos-private-api") && !cargo_toml.contains("tauri-runtime-") {
        findings.push(MigrationFinding {
            rule_id: "TAURI-V3-003",
            description: "`devtools`, `macos-private-api`, and `unstable` must be enabled on the runtime crate (`tauri-runtime-wry` or `tauri-runtime-cef`).",
            replacement_hint: "Move `features = [\"devtools\", \"macos-private-api\"]` to `tauri-runtime-wry`.",
        });
    }

    findings
}

/// Audits Rust source code for Tauri 2.0 APIs that moved or changed in Tauri 3.0 Alpha.
pub fn audit_tauri_v2_rust_source(source: &str) -> Vec<MigrationFinding> {
    let mut findings = Vec::new();

    if source.contains("tauri::Wry") || source.contains("tauri::WryHandle") {
        findings.push(MigrationFinding {
            rule_id: "TAURI-V3-004",
            description: "`tauri::Wry` and `tauri::WryHandle` re-exports were removed; `AppHandle` now defaults to `tauri::DynRuntime`.",
            replacement_hint: "Omit the runtime generic (`&tauri::AppHandle`) for dynamic dispatch, or use `tauri_runtime_wry::WryRuntime` for static dispatch.",
        });
    }

    if source.contains("tauri::tao") || source.contains("tauri::wry") {
        findings.push(MigrationFinding {
            rule_id: "TAURI-V3-005",
            description: "`tauri::tao` and `tauri::wry` re-exports were removed from `tauri`.",
            replacement_hint: "Import `tauri_runtime_wry::{tao, wry}` and extension traits `AppHandleWryExt` / `WebviewWryExt`.",
        });
    }

    if source.contains("tauri::webview_version()") {
        findings.push(MigrationFinding {
            rule_id: "TAURI-V3-006",
            description: "`tauri::webview_version()` free function was removed because version depends on the active runtime.",
            replacement_hint: "Call `app.webview_version()?` on `tauri::App` or `tauri::AppHandle`.",
        });
    }

    if source.contains("http://tauri.localhost") && source.contains("get_ipc_response") {
        findings.push(MigrationFinding {
            rule_id: "TAURI-V3-007",
            description: "`tauri::test::MockRuntime` now uses `tauri://localhost` on every platform (including Windows and Android).",
            replacement_hint: "Replace `\"http://tauri.localhost\"` with `\"tauri://localhost\"` in `InvokeRequest` tests.",
        });
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webview_engine_gtk_and_custom_scheme() {
        let wry_engine = WebviewEngine::Wry;
        assert_eq!(wry_engine.runtime_crate_name(), "tauri-runtime-wry");
        assert_eq!(wry_engine.linux_gtk_version(), LinuxGtkBackend::Gtk3);
        assert_eq!(
            wry_engine.custom_scheme_url("tauri", true, false),
            "http://tauri.localhost"
        );
        assert_eq!(
            wry_engine.custom_scheme_url("tauri", false, false),
            "tauri://localhost"
        );
        assert_eq!(
            wry_engine.custom_scheme_url("tauri", true, true),
            "tauri://localhost"
        );

        let cef_engine = WebviewEngine::Cef {
            distribution: None,
            helper: None,
            remote_debugging_port: Some(9222),
        };
        assert_eq!(cef_engine.runtime_crate_name(), "tauri-runtime-cef");
        assert_eq!(cef_engine.linux_gtk_version(), LinuxGtkBackend::Gtk4);
        assert_eq!(
            cef_engine.custom_scheme_url("tauri", true, false),
            "tauri://localhost"
        );
    }

    #[test]
    fn test_capability_manifest_and_v3_wildcard_permissions() {
        let cap = CapabilityManifest::new("main-desktop", "Main desktop window capability")
            .with_window("settings-*")
            .with_permission(PermissionEntry::allow_all_app_commands())
            .with_permission(PermissionEntry::named("deny-dangerous_reset"));

        assert!(cap.validate().is_ok());
        assert!(cap.is_app_command_allowed("main", "greet_user"));
        assert!(cap.is_app_command_allowed("settings-audio", "save_profile"));
        assert!(!cap.is_app_command_allowed("main", "dangerous_reset"));
        assert!(!cap.is_app_command_allowed("untrusted-popup", "greet_user"));

        let json = cap.to_json_pretty().expect("serialize capability");
        assert!(json.contains("\"allow-*\""));
        assert!(json.contains("\"deny-dangerous_reset\""));
    }

    #[test]
    fn test_ipc_request_authorization_and_channel_frames() {
        let cap = CapabilityManifest::new("default", "Default ACL")
            .with_permission(PermissionEntry::named("allow-fetch_metrics"));

        let req = IpcRequest::new("fetch_metrics", serde_json::json!({ "window": "1h" }));
        assert!(req.authorize(&cap, "main").is_ok());
        assert!(req.authorize(&cap, "other").is_err());

        let denied_req = IpcRequest::new("drop_database", serde_json::Value::Null);
        assert!(matches!(
            denied_req.authorize(&cap, "main"),
            Err(YoloTauriError::AclDenied { .. })
        ));

        let msg = TauriChannelMsg {
            seq: 1,
            channel_id: 42,
            frame: ChannelFrame::Chunk(serde_json::json!({ "progress": 0.75 })),
        };
        let encoded = serde_json::to_string(&msg).unwrap();
        let decoded: TauriChannelMsg = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_tauri_v2_to_v3_migration_auditors() {
        let legacy_toml = r#"
[dependencies]
tauri = { version = "2.1.0", features = ["wry", "macos-private-api"] }
"#;
        let toml_findings = audit_tauri_v2_cargo_toml(legacy_toml);
        assert_eq!(toml_findings.len(), 3);
        assert_eq!(toml_findings[0].rule_id, "TAURI-V3-001");
        assert_eq!(toml_findings[1].rule_id, "TAURI-V3-002");
        assert_eq!(toml_findings[2].rule_id, "TAURI-V3-003");

        let legacy_rs = r#"
use tauri::wry;
fn setup(app: &tauri::AppHandle<tauri::Wry>) {
    let _v = tauri::webview_version();
}
"#;
        let rs_findings = audit_tauri_v2_rust_source(legacy_rs);
        assert_eq!(rs_findings.len(), 3);
    }
}
