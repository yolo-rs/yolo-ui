# Bun and Tauri bridge provenance

This package imports the first-party Bun bridge from `aphrody-labs/bun-os`,
commit `ca9f2739629a8a421a8c18928a499889372ae25f`, source path
`packages/bun-tauri/{js,bin}`. The source snapshot is Apache-2.0 OR MIT; the
upstream license texts are retained in this directory. Original package name
was `@aphrody/tauri` version `1.4.7`; this consolidation publishes it as
`@yolo-rs/bun-tauri` and updates package metadata only. Source files retain
upstream SPDX headers.

The source tracks Tauri `v3` commit
`78f49ba3993fed5a1b921235c547731006ad2a74` (upstream manifest versions
`@tauri-apps/api` `3.0.0-alpha.3` and CLI `3.0.0-alpha.5`). The archive's
`catalog:` dependency aliases were replaced with those exact versions because
this workspace does not share Bun OS's package catalog.

This import contains the Bun/TypeScript guest bridge and sidecar builder only.
Its `plugin:aphrody|...` commands and `plugin:mcp-bridge|...` commands require
matching native Rust plugins. The Bun OS Rust fork is not bundled here, so
these functions are not usable until the corresponding native plugin crates
are integrated and registered by an app.
