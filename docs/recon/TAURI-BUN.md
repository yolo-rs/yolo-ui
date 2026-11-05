# Tauri v3 fork + Bun: stability, AOT/LTO, zero-copy IPC, fewer dependencies

Recon checked 2026-11-05. Read-only. Paths are relative to the yolo-ui repo root (evidence read from `C:\src\yolo-ui`). Nothing was built or run, so every "verify" step below is still to be done on a remote host.

## 0. Baseline

- Fork: `submodules/tauri` is `3.0.0-alpha.5` (`crates/tauri/Cargo.toml:3`). It is hooked in through `[patch.crates-io] tauri = { path = "submodules/tauri/crates/tauri" }` (`Cargo.toml:1685-1687`).
- `wry` and `tray-icon` resolve from crates.io, not from the submodule forks (`Cargo.lock`: `wry 0.57.1 registry`, `tray-icon 0.25.1 registry` + `0.26.1 path`, `muda 0.20.0` + `0.21.2`). Patches to `submodules/wry` or `submodules/tao` therefore never reach either app.
- `packages/yolo-tauri` re-exports config/ACL/JSON-envelope models (`src/lib.rs:3`, `crates/yolo-engines/src/tauri.rs:491,528`); `packages/bun-tauri` lacks its native plugins (`UPSTREAM-PROVENANCE.md:17-21`).
- Both apps share one workspace lock with 2968 packages (`Cargo.toml:177,190`). Getting a per-app crate count needs `cargo tree -p <pkg>`, which was not run.

## 1. Dependency footprint

| | Spacedrive (`apps/spacedrive/apps/tauri`) | Locally Uncensored (`apps/locally-uncensored`) |
|---|---|---|
| Rust direct deps | 23 common + 4 target + 3 build + 1 dev (30 unique) | 51 common + 32 target-specific + 1 build (78 unique) |
| tauri-plugin-* | 6: dialog, fs, opener, clipboard-manager, os, updater (`src-tauri/Cargo.toml:19-24`) | 7: shell, opener, lu-native, updater, single-instance, global-shortcut, deep-link (`src-tauri/Cargo.toml:67-68,175,184-185,201,234`) |
| JS deps / devDeps | 13 / 14 | 27 / 20 |

Lock duplicates: `axum 0.7.9` (Spacedrive `Cargo.toml:42`) vs `0.8.9` (LU `:101`), `tray-icon`, `muda`, `reqwest` x3, `windows-sys` x6.

**Redundant Tauri plugins (Spacedrive).** `tauri_plugin_fs`, `tauri_plugin_os` and `tauri_plugin_clipboard_manager` are registered (`src-tauri/src/main.rs:1911-1914`), but nothing calls them in `apps/tauri/src`, `packages/interface/src`, `packages/ts-client/src` or `src-tauri/src`. The frontend uses `navigator.clipboard` instead (7 hits). The capabilities still grant them (`capabilities/default.json`). LU's plugins all have call sites (`src/api/mcp/external-client.ts:218`, `src/stores/updateStore.ts:306`).

**JS that Bun or Web APIs replace.** The frontend runs in WebView2/WKWebView/WebKitGTK, not in Bun, so Bun APIs only help in the dev server, the build, tests and sidecars.

| Dep | Replacement | Where it applies |
|---|---|---|
| LU `uuid` (`package.json:69`, 31 files) | `crypto.randomUUID()`, already used 21 times | webview |
| LU `dotenv` (`:53`, used only in `vite.config.ts:6,34`) | Bun reads `.env` on its own (bun-types `docs/runtime/environment-variables.mdx:6`) | build |
| LU `vitest`, `jsdom`, `node --test` (`:92,87,36`) | `bun test` | tests |
| LU `@playwright/test` (`:74`) | `Bun.WebView`, experimental and headless (`bun.d.ts:9925`, `docs/runtime/webview.mdx`). Partial: no Tauri IPC, so it can only drive the browser dev surface | e2e |
| LU `@types/node` (`:79`), `engines.node >=22` (`:24`) | bun-types, `engines.bun` | tooling |
| LU `eslint` + plugins | `bun lint`, **canary only** (`docs/lint/index.mdx`) | tooling |
| Spacedrive `react-scan` (`package.json:36`) | drop it; the only use is commented out (`src/App.tsx:55-60`) | webview |
| Spacedrive `sass` (`:48`) | plain CSS. Only two `?url` SCSS imports (`packages/interface/src/components/QuickPreview/prism.tsx:4,6`) | build |
| `vite`, `@vitejs/plugin-react*`, `@tailwindcss/vite` | `bun ./index.html` / `bun build ./index.html` + `bun-plugin-tailwind` (`docs/bundler/html-static.mdx:24,218-255`) | build |
| LU `jszip` (1 file) | keep: `Bun.Archive` is tar only (`docs/runtime/archive.mdx:6`) | webview |

`Bun.SQL`, `bun:sqlite`, `Bun.password`, `Bun.CSRF`, `Bun.serve` have no webview consumer; they matter once LU's `dev-server/` (3230 lines) moves to Bun. `Bun.markdown` (`:1595`) and `Bun.Image` (`:9543`) cannot run in the webview. `URLPattern` on WebKitGTK/WKWebView is **unverified**.

## 2. Release profile

Current root profile (`Cargo.toml:931-937`): `incremental = true`, `panic = "unwind"`, `codegen-units = 1`, `lto = true` (fat), `opt-level = "s"`, `strip = true`. `.cargo/config.toml:20` sets `CARGO_INCREMENTAL=0`, which overrides the `incremental` key, so that key has no effect.

**Defect.** LU's crash reporter assumes `panic = "abort"` ("the release profile builds with `panic = "abort"` (Cargo.toml)", `src-tauri/src/crash_report.rs:4-6`). LU is a workspace member, so only the root profile applies, and that profile unwinds. Cargo also cannot set `lto` or `panic` per package, so per-app settings need named profiles.

Recommended, added next to the existing profiles:

```toml
[profile.tauri-release]
inherits = "release"
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
debug = false
incremental = false

[profile.tauri-ci]
inherits = "tauri-release"
lto = "thin"
codegen-units = 16
```

Trade-offs:
- Fat LTO with `cgu = 1` gives the slowest links, and sccache cannot cache the LTO step.
- `panic = "abort"` turns any panic inside a tokio task into a process exit. The fork itself relies on `catch_unwind` in `manager/scoped_listener.rs:333`, and LU uses it in `cancel_registry.rs:357,377`. Spacedrive should either stay on `unwind` or audit its commands first.
- `opt-level = 3` grows binaries versus `"s"` (not measured).
- Wiring: pass the profile through `tauri build -- --profile tauri-release`. Whether the v3 CLI handles this flag is **unverified**; check it in `submodules/tauri/crates/tauri-cli`.

**What AOT means.** Rust is already native AOT. For JS, `Bun.build({ bytecode: true })` must be `target: "bun"`, and ESM requires `compile: true` (`bun.d.ts:3414-3428`). That makes it apply to **sidecars only** (`packages/bun-tauri/js/sidecar.ts:48-52`), not to the webview bundle, which system JS engines parse. For the webview, "AOT" means a minified, tree-shaken `bun build` output plus the React Compiler. Neither app configures the React Compiler, and bun-types shows no built-in equivalent.

## 3. Zero-copy IPC

What the fork does (`crates/tauri/src/ipc`):
- `Response::new(impl Into<InvokeResponseBody>)` (`mod.rs:189-203`) and `InvokeResponseBody::Raw(Vec<u8>)` (`mod.rs:98-104`) arrive in JS as an `ArrayBuffer`.
- `Channel<Vec<u8>>` "is sent as a JSON array of numbers" (`channel.rs:146-149`). Use `Channel<InvokeResponseBody>` with `Raw` instead.
- Raw payloads under 1024 bytes are still inlined as a JSON array through `eval` (`channel.rs:38,302`). Larger ones go through the fetch protocol.
- On Android the request body is never `Raw` (`mod.rs:54-56`).

The webview boundary still costs one copy. "Zero-copy" here means no serde, no base64 and no 4x number-array inflation.

Tauri docs (checked 2026-11-05):
- https://v2.tauri.app/develop/calling-rust/ recommends `tauri::ipc::Response` for large data and `tauri::ipc::Request` with `InvokeBody::Raw`.
- https://v2.tauri.app/develop/calling-frontend/ states that events "are always JSON strings ... not suitable for bigger messages" and recommends channels for streaming.

Our call sites that move bytes as JSON:

| Site | Now | Change |
|---|---|---|
| LU `commands/proxy.rs:1379` + `src/api/backend.ts:379,415` | `Channel<Vec<u8>>` -> `number[]` per chat token chunk | `Channel<InvokeResponseBody>` + `Raw`; JS `Channel<ArrayBuffer>` |
| LU `proxy.rs:1327` (`proxy_localhost_stream`) <- `backend.ts:461,933,1050` | `Result<Vec<u8>>` -> `number[]` (images, downloads) | return `tauri::ipc::Response` |
| LU `proxy.rs:196` (`fetch_external_bytes`) <- `backend.ts:1032` | same, ZIPs and model files | `Response`, or better a stream to disk |
| LU `proxy.rs:1639` (`comfy_upload_image`) <- `src/api/comfyui.ts:2455,2532` | `Array.from(Uint8Array)` upload | `tauri::ipc::Request` raw body, metadata in headers; keep base64 on Android |
| LU `filesystem.rs:960`, `media_cmds.rs:430`, `image_mflux.rs:1443,1614`, `mlx_cutout.rs:155`, `computer.rs:463` | base64 inside JSON | `Response` raw, or an async URI scheme for media |
| Spacedrive `main.rs:594-640` (`daemon_request`) | parses the daemon's JSON line into `Value`, then reserializes | `Response::new(response_line)` passes the JSON string through |
| Spacedrive `core-event` via `app.emit` (`packages/ts-client/src/transport.ts`) | JSON events | `Channel`, which is ordered and avoids `eval` per event |
| Spacedrive `src-tauri/src/server.rs:1-5,300-345` | axum on a random port, because "custom URI protocols can't be async" | that claim is stale: `register_asynchronous_uri_scheme_protocol` exists (`app.rs:2346`). An `sd://` scheme removes axum 0.7, the auth token and the port |

## 4. Stability

**Alpha-only APIs in use:** `Builder::runtime` + `DynRuntime` (`crates/tauri/src/lib.rs:182`), the `allow-*` wildcard (Spacedrive `capabilities/default.json`; `notes/architecture/13-TAURI-V3.md:116-119`), `@tauri-apps/api 3.0.0-alpha.3` and plugin alphas. Upstream says 3.0 is alpha and production should stay on 2.x (https://v3.tauri.app/start/migrate/from-tauri-2/, checked).

**LU never selects a runtime.** It calls `tauri::Builder::default()` (`src-tauri/src/lib.rs:615`) and then `.build(...)` (`:1485`). It has no `tauri-runtime-wry` dependency and names the removed `tauri::Wry` (`commands/mobile_native.rs:45`). According to `tauri-runtime/src/dynamic.rs:2741`, `Error::RuntimeNotConfigured` follows. Spacedrive does select one (`main.rs:1910`). From reading the code, LU looks broken against the fork. Confirm with `cargo check -p locally-uncensored`.

**Devtools.** LU enables `tauri/devtools` in desktop release builds (`Cargo.toml:183`). The fork notes that the runtime crate needs the feature as well (`crates/tauri/Cargo.toml:205`).

**Pins to keep:** LU `webkit2gtk =2.0.2` (`:313`), `llama-cpp-2/sys-2 =0.1.154` (`:224-225`), `libc` lockstep (`:274-280`); Spacedrive `react-router-dom =6.20.1`.

**Not checked:** I did not look up known upstream GitHub issues. Treat that as an open item.

**Patch in the fork:** `tray-icon` 0.25 to 0.26.1 (`crates/tauri/Cargo.toml:99`); `[patch.crates-io]` for `wry`/`tao`/`muda`/`tray-icon`; a `tauri-macros` warning for `Channel<Vec<u8>>`.

**Upstream:** the `Channel<Vec<u8>>` diagnostic and a configurable `MAX_RAW_DIRECT_EXECUTE_THRESHOLD`.

## 5. Bun-friendliness

- **Spacedrive:** hooks already call Bun (`tauri.conf.json:6-7`), but `dev`/`build` run Vite (`package.json:10,12`). Blockers: aliases and `optimizeDeps` (`vite.config.ts:13-102`, move to tsconfig `paths`), SCSS `?url` (`prism.tsx:4,6`), 5 `import.meta.env` uses (`Bun.build` `env: "VITE_*"`, `bun.d.ts:3351`), `envPrefix TAURI_ENV_*` (`:118`). Fast Refresh: `reactFastRefresh` (`bun.d.ts:3580`). **Feasible now.**
- **LU:** `beforeDevCommand: "bun run dev"` (`tauri.conf.json:7-8`) runs Vite 8 with custom plugins and `devServerPlugin` (`vite.config.ts:199`). Blockers: the 3230-line `dev-server/` is Vite middleware and the user's runtime (`:209-217`), 4 proxies (`:219-249`), `import.meta.glob` (`src/mobile/platform/bridge.ts:157`), 20 `import.meta.env` uses, Vitest + jsdom. Order: dev-server to `Bun.serve` + HTML import, then build, then tests.

## 6. Bun 1.4.4 and canary features

Bun 1.4.4, 2026-10-22 (https://bun.com/blog/bun-v1.4.4, checked): `bun fmt`, `bun check -b`, `Bun.SQL` cancel/COPY, HTML-import dev overlay, `--compile` hardening, WebView/Image/CSRF/markdown fixes.

| Feature | Verified in bun-types 1.4.4 | Use |
|---|---|---|
| `Bun.build` `bytecode`/`compile` | `bun.d.ts:3428` | sidecars |
| `reactFastRefresh` | `:3580` | Bun dev server |
| `env: "PREFIX_*"` | `:3351` | replaces Vite `envPrefix` |
| `Bun.JSONL` | `:1398` | NDJSON chat streams in a Bun sidecar or dev-server |
| `Bun.WebView` (experimental) | `:9925` | e2e on the browser surface |
| `Bun.CSRF`, `Bun.secrets` | `:2849`, `:4129` | dev-server `/local-api` |
| `bun lint` | canary only (`docs/lint/index.mdx`) | replaces eslint after qualification |
| `bun upgrade --canary` | `docs/installation.mdx:193` | canary channel |

I verified no other canary-only features.

## 7. Patch plan, ranked by value

Format: change; remote verification; risk.

1. **LU runtime selection.** Add `tauri-runtime-wry` (path dep) to `src-tauri/Cargo.toml`, `.runtime(tauri_runtime_wry::Wry::default())` at `lib.rs:615`, `tauri_runtime_wry::WryRuntime` at `mobile_native.rs:45`. Verify: `cargo check -p locally-uncensored`, window opens on Windows/Linux. Risk: low.
2. **LU raw IPC.** `proxy.rs:1379` to `Channel<InvokeResponseBody>`; `proxy.rs:196,1327` return `Response`; `comfy_upload_image` takes `Request`; update readers in `backend.ts`/`comfyui.ts`; keep base64 on Android. Verify: extend the test at `proxy.rs:2396`; time a 20 MB image and token latency before/after. Risk: medium (JS shapes change).
3. **Profiles.** Add `tauri-release`/`tauri-ci`; resolve the `panic` mismatch with `crash_report.rs:4`; drop dead `incremental = true` (`Cargo.toml:932`). Verify: `cargo build --profile tauri-release -p locally-uncensored -p spacedrive`, compare size and cold start, force a panic and check the crash report. Risk: medium.
4. **Make fork patches effective.** Patch `wry`, `tao`, `muda`, `tray-icon` in `Cargo.toml:1685`; `tray-icon` 0.26 at `submodules/tauri/crates/tauri/Cargo.toml:99`. Verify: `cargo tree -d -p spacedrive` shows one `tray-icon`/`muda`. Risk: medium (0.25 to 0.26 API drift).
5. **Spacedrive IPC and dependency cuts.** `daemon_request` returns the JSON string via `Response` (`main.rs:594`); remove fs/os/clipboard plugins (`Cargo.toml:20,22-23`, `main.rs:1911-1914`, capabilities); drop `@tauri-apps/plugin-fs`, `react-scan`, `sass`. Verify: `tauri dev` smoke test of explorer, dialogs, updater. Risk: low.
6. **Spacedrive server to async URI scheme.** Replace `server.rs` axum routes with `register_asynchronous_uri_scheme_protocol("sd", ...)`; update `get_server_url` consumers and CSP (`tauri.conf.json:37-42`). Verify: thumbnails and sidecars on WebView2 and WebKitGTK. Risk: medium-high (origin differs per platform, `app.rs:2340-2345`).
7. **Spacedrive Vite to Bun.** `package.json:10,12` to `bun ./index.html` / `bun build ./index.html --outdir dist`, `bunfig.toml` Tailwind plugin, aliases to tsconfig. Verify: `tauri build`, `dist` size diff, HMR under `tauri dev`. Risk: medium.
8. **bun-tauri.** Default `bytecode: true, minify: true` in `sidecar.ts:48`; binary-safe `BunResponse.body` (`bun.ts:20-24`). Verify: `bun test`, sidecar start time. Risk: low.
9. **LU dev-server to `Bun.serve`**, then drop Vite, Vitest and Node. Verify: LU e2e suites against the Bun server. Risk: high (it is also the user runtime).
