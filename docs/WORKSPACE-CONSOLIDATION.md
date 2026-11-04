# Workspace consolidation

This repository has one Cargo workspace and one Bun workspace with a root
`bun.lock` pinning the JavaScript workspace graph. First-party
product crates are direct children of `crates/`, `packages/`, and `apps/`; the
root manifests are the only workspace entry points for those products. Nested
directories remain for tests, generated harnesses, and the selected fork
sources currently required by path dependencies. Source absorption into the
canonical product tree is ongoing; imported components retain their original
license notices and provenance.

## Ownership and layout

| Area                                                         | Canonical location                          | Entry point                                                                                                  |
| ------------------------------------------------------------ | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Shared runtime and platform-neutral engines                  | `crates/`                                   | `yolo-shared`, `yolo-tokens`, `yolo-layout`, `yolo-render`, `yolo-gpu`                                       |
| Cross-platform toolkit adapters                              | `crates/yolo-engines`                       | `gpui`, `iced`, `egui`, `cosmic`, `gtk`, `qt`, `tauri` modules                                               |
| Framework API and feature-gated engine access                | `packages/`                                 | `packages/yolo-ui`; old per-engine packages are compatibility re-exports                                     |
| Command-line, plugin, MCP, and JS tooling                    | `packages/`, `scripts/`                     | `bun run ui <command>`                                                                                       |
| Product applications kept separate from framework acceptance | `apps/`                                     | Product-specific commands; not migration evidence                                                            |
| Build, release, and repository automation                    | `scripts/`, `Justfile`, root `package.json` | `bun run build`, `bun run build:release`, `just`                                                             |
| Design and architecture documentation                        | `docs/`, `notes/architecture/`              | `docs/README.md`                                                                                             |
| Templates and runnable examples                              | framework package assets and `examples/`    | `dx new` and example manifests                                                                               |
| Distribution output                                          | `dist/`                                     | release build tooling                                                                                        |
| Browser engines and browser research tooling                 | sibling `yolo-web` repository               | Chromium, `bun:chromium`, Obscura/CDP, Servo/Stylo, CEF, browser-facing Google projects                      |
| OS portability and cross toolchains                          | sibling `yolo-cross` repository             | Windows/Linux platform layers, MSVC, cross-compilers, sysroots, and native package-manager adapters          |
| Bun runtime and YOLO OS package CLI                          | sibling `yolo-bun` repository               | Upstream-aligned Bun runtime changes plus one package-management interface that delegates to native backends |

These repositories have separate source trees and release ownership. The root
Cargo and Bun workspaces unify products **inside `yolo-ui`**; they do not claim
to turn sibling repositories into members or to vendor their full source. A
cross-repository integration is complete when the owning repository exposes a
versioned API/package and this workspace consumes it through an explicit
dependency or generated artifact with provenance recorded. Until that exists,
references to sibling projects are ownership boundaries, not build dependencies.

### Cross-repository ownership rules

- Browser implementations, Chromium-derived browser code, Obscura, CDP, CEF,
  and browser-specific Google integrations are maintained in `yolo-web`.
  `yolo-ui` may own reusable visual contracts, theme tokens, native UI, and
  client applications that call a `yolo-web` API. Its Chromium theme preset is
  a design contract, not a Chromium implementation.
- Windows/Linux portability, MSVC configuration, target triples, cross-build
  sysroots, and native package-manager adapters belong to `yolo-cross`. The
  `yolo-bun` package CLI is the unified user-facing package-manager interface
  for `yolo-os`; it delegates installs, verification, and platform-specific
  plans to the owning native backends rather than reimplementing them. UI and
  browser products consume published cross-platform contracts or build
  artifacts; they do not duplicate a second cross-compilation framework.
- Frameworks, React/Next-like UI APIs, M3 UI packages, Tauri integration,
  frontends, design plugins, and UI developer tools belong to `yolo-ui`.
  Browser-engine adapters used by a Tauri host remain in `yolo-web`; the
  `yolo-tauri` package owns the generic host/ACL bridge.
- `yolo-bun` follows upstream Bun for runtime behavior and performance. Keep
  runtime patches limited to Bun's own structure plus essential runtime
  support (for example environment/cookie APIs and minimal Windows/Linux
  integration). Its separate `yolo-os` package CLI may coordinate Bun/npm,
  Flatpak/Flathub, F-Droid, WinGet, uv, Cargo, vcpkg, and pacman sources through
  their native authorities; source discovery alone does not establish that a
  repository is configured, trusted, or ready to install. Move framework,
  UI, Tauri, plugin, browser, and cross-build product features to their owning
  repositories. Do not claim upstream-level runtime performance without a
  reproducible comparison against the same Bun release.
- First-party ownership means maintained by the Yolo product; it does not
  erase upstream licenses, attribution, or source provenance. Do not copy an
  upstream implementation into another repository just to satisfy a folder
  layout or workspace count.

`packages/yolo-ui` is the unified Rust-facing API, and `crates/yolo-engines`
owns the shared adapter implementations. Backend-specific code stays behind
optional features so a consumer can select web, desktop, native, or a toolkit
bridge without making every platform dependency mandatory. These are
first-party integration crates; an adapter may still depend on an upstream
platform library where that library supplies OS bindings or a native runtime.
The Shadcn-Bun inspector and migrator implementation is owned by the unified
`@yolo-rs/ui-plugin`; `@yolo-rs/shadcn-bun` remains a compatibility entry point.
The root CLI and MCP import the workspace package APIs instead of reaching into
each other's source trees.

The root Cargo manifest declares 237 workspace members: 134 packages under
`crates/`, `packages/`, and `apps/`, 61 selected package manifests under
`submodules/`, 39 Spacedrive packages, two Spacedrive extensions, and the root
`dioxus-examples` package. The absorbed
`apps/locally-uncensored` AGPL desktop/mobile backend, its native plugin,
microphone probe, and iOS fixture now use this workspace; their Bun dependencies
use the root Bun lockfile. The libFuzzer runner also uses the root workspace; its
`test = false` target stays out of unit-test runs while its build profile is
configured at the root.
Some imported manifests still inherit package metadata and dependency versions
from the root workspace, so source migration or manifest adaptation remains
before those submodule workspace entries can be removed. Standalone example
manifests remain outside the workspace.

Spacedrive's 39 Rust packages and two extensions now belong to the root Cargo
workspace; its standalone Cargo manifest and extension workspace declarations
were removed. Root dependency catalog entries were added for 33 dependencies
that were previously Spacedrive-only. A set of dependencies retains
Spacedrive-specific inline constraints because the existing root versions or
default-feature policies differ; these are version migrations still to do.
Spacedrive's 2021 editions and upstream license files remain preserved; package
license fields are retained where declared upstream, while missing metadata
must be normalized before publishing individual packages. The workspace Rust
minimum is now 1.95. The shared release
profile uses unwind to preserve Spacedrive's required panic behavior. Named
`spacedrive-dev`, `mobile-dev`, and `extension-release` profiles retain its
special build settings. Its Bun app and React packages remain in the root Bun
workspace, and `sd:*` commands now target the root Cargo workspace. The desktop
builder, local plugins, frontend API/CLI and capability generation have been
migrated to the local Tauri 3 Alpha stack and wired to `yolo-tauri`. The root
lock now resolves with Iroh 1.4.1 and the separate upstream mDNS address lookup
crate. `cargo check --locked -p sd-core`, the relay integration target, and
`cargo check --locked -p spacedrive` pass on the current host. The Spacedrive
React typecheck and production frontend bundle also pass. Full workspace,
cross-platform, and release-build qualification remain open. The app-local
Spacedrive docs site is in the root Bun workspace and uses the root `bun.lock`.

The `@yolo-rs/bun-tauri` package imports the Bun guest bridge and sidecar from
the clean `aphrody-labs/bun-os` commit recorded in its provenance file. The
matching Bun OS Rust fork remains a separate 14-crate Tauri workspace whose
`links = "Tauri"` native runtime conflicts with the selected upstream Tauri
runtime; its native plugins have not been merged. Bun OS does not contain a
complete Aphrody Drive application: its `aphrody-drive` crate contains only
permission/schema material. The imported Spacedrive app remains Spacedrive and
is not represented as Aphrody Drive.

The product policy is to absorb useful engine, framework, tooling, and template
sources into Yolo UI while preserving each component's upstream license and
provenance. “First party” describes the unified product and maintenance
ownership; it does not change copyright ownership or relicense imported code.
The same applies to repositories collected under `upstreams/`. A source is
considered consolidated only when it has a canonical product location, a
workspace/build integration, and retained attribution; mirroring a repository
alone does not complete that migration.

## Aphrody Drive archive reconciliation

The retired Aphrody Drive application is not restored as a second file manager.
The legacy tag `legacy/aphrody-drive-last` is retained in the Bun OS archive;
its provenance identifies the old Spacedrive-based copy as FSL-1.1-ALv2.
Yolo UI's absorbed Spacedrive source is separately Apache-2.0, and the two
licenses must not be conflated. The archive's current `crates/aphrody-drive`
contains only a permission schema. The following legacy features were checked
against the current Spacedrive tree:

| Legacy feature                                    | Current canonical implementation                                                                     | Reconciliation                                                                                                                                                          |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Bun.connect` daemon client over Unix/TCP sockets | `apps/spacedrive/packages/ts-client/src/transport.ts`                                                | Already present in the Spacedrive client; keep one transport implementation.                                                                                            |
| SVG thumbnail rasterization via `resvg`           | `apps/spacedrive/crates/images/src/svg.rs`                                                           | Already present in the image crate.                                                                                                                                     |
| Google Drive as an indexed cloud volume           | `apps/spacedrive/core/src/ops/volumes/add_cloud/` and `packages/interface/.../CloudStorageSteps.tsx` | Core operation, OpenDAL backend, and UI flow already exist. Verify OAuth refresh and folder selection when improving the flow; do not import the old DriveStudio route. |
| Tauri daemon sidecar/supervision                  | `apps/spacedrive/apps/tauri/src-tauri/src/main.rs` and `build.rs`                                    | Sidecar discovery, spawning, and lifecycle handling exist. Keep platform autostart/service work in this Tauri app and shared shell contracts.                           |
| Native drag and macOS Full Disk Access            | `apps/spacedrive/apps/tauri/src-tauri/src/drag/` and `apps/spacedrive/crates/fda/`                   | Existing Spacedrive implementations; do not duplicate their plugin crates.                                                                                              |
| Browser OPFS explorer protocol                    | Legacy Bun OS WebOS files                                                                            | Remains a browser-only WebOS concern; it is not part of the native Spacedrive daemon client.                                                                            |
| Local-model chat attachment builder               | `apps/locally-uncensored/src/lib/chat-attachments.ts`                                                | A related local AI attachment path exists in Locally Uncensored; keep it app-owned until a shared `yolo-ai` contract is defined.                                        |
| “Ask Aphrody” context prompt for selected files   | No current Spacedrive equivalent found                                                               | Unported legacy behavior. Rebuild only as a first-party Yolo UI/Aphrody AI feature with explicit selection, OCR, and media eligibility contracts.                       |

The Bun OS `packages/bun-tauri` nested workspace contains 14 crates, mostly
renamed Tauri v3 upstream sources. Its main runtime crate declares
`links = "Tauri"`, which conflicts with the already selected Tauri source in
`submodules/tauri`. The upstream base is the same commit already used by Yolo
UI, so import only clearly first-party plugin behavior after adapting it to
that selected runtime. CEF-specific code belongs to `yolo-web`; generic UI and
MCP bridge behavior belongs to Yolo UI. Preserve each source license and
commit provenance during any port.

## AI and UI product boundaries

`yolo-ai` owns the provider-neutral agent core and reusable runtime interfaces;
Aphrody AI owns its branded AI backends and libraries. Aphrody UI can present
Locally Uncensored as its GUI, OpenCode as its terminal interface, and Zed as
its editor integration. These products remain part of the architecture while
their source licenses and provenance stay attached. In particular, the
Locally Uncensored GUI remains AGPL-3.0-only. UI integrations should use the
shared agent engine/context interfaces instead of embedding another turn loop.

## Dependency policy

Use `[workspace.dependencies]` in the root `Cargo.toml` for shared versions,
then refer to them with `workspace = true` from member manifests. Platform
adapters should be optional features on the umbrella package when they can be
made optional safely. Canonical product code belongs to the root Cargo and Bun
workspaces. Imported projects that still declare nested workspaces are
temporary migration exceptions; `bun run optimize` lists their Cargo roots so
they cannot be mistaken for consolidated members.

This policy centralizes version selection. External platform runtimes and
libraries may remain dependencies where needed, while imported source code is
tracked with its own license and provenance. Dependency centralization does
not imply that upstream code has been rewritten or relicensed.

## Unified commands

| Task                                                           | Command                                                                         |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| UI search, theme output, migration, devtools, MCP, diagnostics | `bun run ui <command>`                                                          |
| Rust workspace build                                           | `bun run build`                                                                 |
| Release build                                                  | `bun run build:release`                                                         |
| Whole-workspace check, lint, or tests                          | `cargo check --workspace`, `cargo clippy --workspace`, `cargo test --workspace` |
| Full Rust and Bun validation                                   | `bun run test`                                                                  |

Scripts are thin entry points over the root workspace. Add new product
commands to the existing CLI or root scripts rather than creating a parallel
tooling package or workspace.

Release profile settings are owned by the root manifest so Tauri builds from
absorbed applications do not silently ignore package-local profile blocks.
