# Spacedrive in Yolo UI

This directory contains the source snapshot of [`yolo-rs/spacedrive`](https://github.com/yolo-rs/spacedrive), forked from `spacedriveapp/spacedrive` at commit `26b57e1be3332f6781d1d6e4d29a10a7599e990a` on 2026-11-04. The upstream `LICENSE` and `NOTICE` are kept here; the source is Apache-2.0 and remains separately licensed from Yolo UI's MIT OR Apache-2.0 code.

## Integrated entry points

- Bun discovers Spacedrive's `apps/*`, `packages/*`, and `docs/` manifests through the root workspace. Install dependencies from `C:\src\yolo-ui` so Bun uses the root lockfile.
- Root scripts `sd:dev`, `sd:build`, `sd:typecheck`, `sd:docs:dev`, `sd:docs:build`, `sd:tauri:dev`, `sd:tauri:build`, `sd:daemon`, `sd:cli`, and `sd:test:rust` expose its app, docs, and backend workflows.
- `sd-tauri-core` depends on `packages/yolo-tauri` and publishes the Spacedrive shell identity as `yolo_shell_config()`. The Tauri entry point loads that policy at startup.
- The desktop app uses the local Tauri 3 Alpha Wry runtime and local v3 plugin crates. Its Bun manifest pins the matching API, plugin, and CLI alpha versions.
- The Tauri build manifest enumerates application commands for generated ACL permissions and validates `capabilities/default.json` through `yolo-tauri` before generating the ACL.
- `bun run sd:test:rust` delegates to `scripts/cargo.ts` and selects only Cargo packages beneath `apps/spacedrive/`. Root `just test` runs the complete Yolo UI workspace suite.
- Upstream React components, backend crates, CLI, daemon, scripts, tests, and documentation are retained in place under this source tree.

## Cargo workspace integration

Spacedrive's 39 backend/app packages and two extension crates are members of the root Cargo workspace; the former nested root manifest was removed. The root graph now contains 237 packages and declares Rust 1.95. Spacedrive's upstream `LICENSE` and `NOTICE` are preserved; package license fields are retained where declared upstream, and missing metadata must be normalized before publishing individual packages. A subset of version-conflicting dependencies remains package-scoped until API migrations can safely unify them. The root lockfile resolves with Iroh 1.4.1 and its separate mDNS address-lookup crate. `cargo check --locked -p sd-core`, the relay integration target, and `cargo check --locked -p spacedrive` pass on the current host. The Spacedrive React typecheck and production frontend bundle also pass.

Use the root `Cargo.toml` and root `Cargo.lock` for builds and dependency resolution. The docs site is part of the root Bun workspace and uses the root `bun.lock`.

## Tauri migration status

The desktop app manifest and builder are migrated to the local Tauri 3 Alpha Wry runtime and available v3 plugins. `yolo-tauri` supplies the shell policy and typed capability/IPC contracts; `sd-tauri-core` loads the policy, and the build script validates app-command permissions before Tauri ACL generation. Locally Uncensored also points at the same local Tauri v3 and plugin sources. On Windows, the Tauri crate check, React typecheck, production frontend bundle, and docs-site build pass; full workspace, cross-platform, and release qualification remain open.

## Root commands

```sh
bun run sd:dev
bun run sd:typecheck
bun run sd:tauri:dev
bun run sd:tauri:build
bun run sd:daemon
bun run sd:cli -- --help
bun run sd:test:rust
```

The Spacedrive-filtered Rust test command is available; tests were not run during this source integration.
