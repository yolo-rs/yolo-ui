# Locally Uncensored in Yolo UI

This application is now a source project in the Yolo UI monorepo at `apps/locally-uncensored`.

- Upstream: https://github.com/PurpleDoubleD/locally-uncensored
- Yolo fork: https://github.com/yolo-rs/locally-uncensored
- Imported revision: `5c9c4441c9cfd7240ee05ae4bb597077e8d89ff8`
- Package version at import: `3.0.8`
- License: `AGPL-3.0-only`; see `LICENSE` in this directory.
- Stack: Tauri v3 alpha, Rust, React 19, TypeScript, Tailwind CSS 4, and Vite.

The source was copied with its original license and copyright notices. Its desktop Rust manifests and URL opener integration are migrated to the local Tauri v3 alpha sources; Tauri now invokes its frontend through the root Bun workspace. The root Yolo UI license does not replace the app's AGPL terms.

From the repository root, start the frontend with `bun run lu:dev`, run the desktop shell with `bun run lu:tauri:dev`, or package it with `bun run lu:tauri:build`.
