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

## Web build

`VITE_LU_BUILD_TARGET=web` produces a static bundle for any web host (`bun run build:web` in this directory, `bun run dev:web` to serve the same mode locally without the dev server's `/local-api` routes and proxies). The switch is read in `vite.config.ts` and in the code through `import.meta.env` (`src/lib/build-target.ts`, `isWebBuild()`).

- Native-only features are off: shell, computer control, native file dialogs, process spawning (built-in engine, Whisper, Piper, installers, MLX) and ComfyUI process control (`src/lib/capabilities.ts`). Commands without a browser path reject with `NativeAppRequiredError` (`code: "native_required"`, `src/api/native-required.ts`) instead of fetching `/local-api`.
- Local model servers are reached from the page by absolute address. Ollama defaults to `http://127.0.0.1:11434` and follows the address in Settings, Providers; ComfyUI defaults to `127.0.0.1:8188` and follows Settings, ComfyUI.
- Ollama must allow the site's origin, or the browser blocks every request: start it with `OLLAMA_ORIGINS` set to the origin the bundle is served from, for example `OLLAMA_ORIGINS=https://lu.example.org ollama serve` (several origins are comma separated). ComfyUI needs the same through `--enable-cors-header https://lu.example.org`.
- A page served over https from a public host may not reach `http://127.0.0.1` in every browser: Chromium and Firefox treat loopback as a secure origin, but recent Chromium versions ask the user for local network access first, and Safari can block the request as mixed content. Serving the bundle from `http://localhost` avoids both.
- API keys are sealed with WebCrypto AES-GCM under a non-extractable key kept in IndexedDB (`src/lib/web-key-vault.ts`). Base64 values from older builds move into the vault on first read and leave localStorage.
- The store backup goes to the origin private file system (`store_backup.json`, `src/lib/web-backup.ts`). Without one, the explicit backup paths and the footer's "Download backup" save it as a JSON file instead.
- Source offer (AGPL-3.0 section 13): the footer and Settings, Updates link to `VITE_LU_SOURCE_URL` (default `https://github.com/yolo-rs/locally-uncensored`), pinned to `VITE_LU_SOURCE_COMMIT` when set. Whoever deploys a modified build must point these at the modified source.
