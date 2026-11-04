# Yolo Engines

`yolo-engines` is the single first-party Rust package for Yolo UI's cross-
platform toolkit adapters. Its `gpui`, `iced`, `egui`, `cosmic`, `gtk`, `qt`,
and `tauri` modules share the same theme tokens, color roles, and contracts.

Toolkit integrations are selected by Cargo features. The default build keeps
native system bindings disabled; `system-gtk4`, `system-qt6`, and
`tauri-desktop-shell` opt into those host-specific bindings. The old
`yolo-gpui`, `yolo-iced`, `yolo-egui`, `yolo-cosmic`, `yolo-gtk`, `yolo-qt`,
and `yolo-tauri` package names remain thin compatibility re-exports.
