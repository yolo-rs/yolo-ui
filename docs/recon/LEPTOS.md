# Leptos ecosystem recon for our public sites

Checked 2026-11-05. Stars, licenses and push dates come from `gh api repos/<o>/<r>`. Versions, licenses and the `leptos` requirement come from `https://crates.io/api/v1/crates/<name>` and its `/dependencies` endpoint. Docs come from Context7 (Leptos book, leptos, cargo-leptos, leptos_i18n) and the local `leptos_router-0.8.17` source. Items marked **[unverified]** are inference.

Scope: ADR `C:\src\yolo-stack\docs\decisions\0003-verdict-leptos-dioxus-tanstack.md` makes Leptos 0.8 on Axum the stack for public sites (dragonballfr.com, rosegriffon.fr). `SITE` below means `C:\src\yolo-stack\apps\shenron\crates\shenron-site`.

## 0. Baseline

- `leptos-rs/leptos`: 21,463 stars, MIT, pushed 2026-11-04. Stable 0.8.23 and 0.9.0-rc1, both 2026-10-31. cargo-leptos 0.3.11 (548 stars).
- `SITE/Cargo.lock` pins leptos 0.8.23, leptos_router 0.8.17, leptos_meta 0.8.7, leptos_axum 0.8.10, server_fn 0.8.13, axum 0.8.9 and wasm-bindgen 0.2.130.

## 1. Leptos 0.8 in depth: what SITE uses or misses

| Feature | Upstream 0.8 | SITE today | Action |
|---|---|---|---|
| SSR modes | `OutOfOrder` (default, inline JS), `InOrder`, `Async`, `PartiallyBlocked`, `Static`. Nested routes take the strictest mode. | `SsrMode::Static` on all 5 routes (`src/app.rs` 85-111) | Keep. Dynamic routes setting `<Title>` after data: `Async`/`PartiallyBlocked`. |
| Static params | `StaticRoute::prerender_params` builds the **cartesian product** of the per-key value lists (`static_routes.rs`, `StaticPath::into_paths`). `.regenerate(\|params\| Stream<()>)` gives on-demand regeneration (ISR). | `manga_params` (`src/app.rs` 64-74) inserts a flattened volume list and a flattened chapter list separately. With C chapters this renders 3 x C x C paths, with duplicates and invalid pairs. No `regenerate`. | **Bug.** Use a splat (`/:lang/manga/*vc`, values `1/3`). Add `regenerate` from API change events. |
| Islands | `leptos = { features = ["islands"] }`, `#[island]`, `hydrate_islands()`, `<HydrationScripts islands=true/>`. Floor of about 24 kB wasm (ADR). | Full hydration: `hydrate_body(App)` in `src/lib.rs` 23-27. `ReaderIsland` (`src/reader.rs`) is a plain `#[component]`. | Islands: reader and theme picker only. |
| Wasm splitting | `cargo leptos build --split`, `#[lazy]`, `#[lazy_route]` with `LazyRoute`. Book: "ironing out bugs". | Not used | Reader painter chunk only. **[unverified]** with islands. |
| Server functions | `#[server]`, default prefix `/api` (`server-fn-prefix`), forms through `ActionForm`. | None | Comments, contact form. Read API stays plain Axum (ADR point 6). |
| Router | `path!()` macro, `ParamsMap`, `<Form method="GET">` progressive enhancement | `path!` and `use_params_map` used; locale prefix parsed by hand (`src/pages/mod.rs`) | Fine. Use `<Form>` for search. |
| leptos_meta | `Title`, `Meta`, `Link`, `Html attr:*`, `Stylesheet`, `MetaTags` | All used (`src/seo.rs`, `src/app.rs`) | Add JSON-LD; `<Html attr:data-theme>` (section 4). |
| Axum integration | `generate_route_list_with_ssg`, `leptos_routes(_with_context)`, `file_and_error_handler`, `ResponseOptions` | All used (`src/main.rs`, `src/pages/not_found.rs`) | Pass API client and theme pack via `leptos_routes_with_context`. |
| cargo-leptos | `hash-files`, `js-minify`, `style-bundle` (Lightning CSS), `wasm-opt-features`, `end2end-cmd`, `front-target-dir` | `wasm-release` profile, `style-file`, no hashing | Enable `hash-files`, `js-minify`. `end2end-cmd` via Bun, not the template's `npx`. |
| i18n | `leptos_i18n` 0.6.2 (MIT, 168 stars, leptos ^0.8): compile-time key checks, `leptos_i18n_build` codegen, `I18nContextProvider`, ICU plurals, localized paths. Alt: `leptos-fluent` 0.3.1. | Own catalog: `src/i18n.rs` (`include_str!` JSON, `t`/`tf`), with a key-parity test | Keep ours; adopt `leptos_i18n` for plurals or localized slugs. |

## 2. Templates

- `leptos-rs/start-axum` (89 stars, Unlicense, last push 2026-03-06). SITE's `Cargo.toml` already mirrors it. Copy only `end2end/` and CI.
- `leptos-rs/start-axum-workspace` (37 stars, Unlicense, last push 2025-12-23): `app`/`frontend`/`server`. **Copy this split for the shared kit** (step 2). The frontend crate must reference `app` or islands exports break (book).
- No official islands starter. Read `examples/islands`, `islands_router`, `hackernews_islands_axum`, `static_routing` (regenerate with `notify`) and `lazy_routes` in `leptos-rs/leptos`.

## 3. awesome-leptos picks

`leptos-rs/awesome-leptos`: 966 stars, last push 2026-09-18.

- **leptos-use** 0.20.0 (`Synphonyte/leptos-use`, 496 stars, MIT OR Apache-2.0, leptos ^0.8, pushed 2026-10-28). `use_cookie`, `use_color_mode`, `use_intersection_observer`, in islands.
- **Icons.** `phosphor-leptos` 0.8.0 (44 stars, MIT, leptos ^0.8, last push 2025-05-09, maintenance is slow) matches our Phosphor directive. `leptos_icons` 0.7.1 plus `icondata` 0.7.0 (MIT, leptos >=0.8.3) cover many icon sets. Both render inline SVG server-side: zero wasm.
- **Component kits.** `thaw` 0.4.8 still targets leptos ^0.7.7 (0.5.0-beta dates from 2025-05). `leptonic` 0.5.0 targets ^0.6. Skip both. `leptodon` 1.7.0 (Apache-2.0, ^0.8.21, pushed 2026-11-04) is active but Flowbite/Tailwind-styled. `rust-ui/ui` (787 stars, MIT) is a shadcn-style copy-paste registry built on Tailwind. `RustForWeb/shadcn-ui` and `RustForWeb/radix` are **archived**. `leptos-shadcn-ui` 0.9.0 and `radix-leptos-primitives` 0.9.0 have under 500 recent downloads. Verdict: none fits token-driven, no-Tailwind CSS; use rust-ui and leptodon as markup/ARIA references, write our own components.
- **Positioning.** `floating-ui-leptos` 0.6.0 (MIT, ^0.8, repo pushed 2026-11-04), popovers in islands.
- **Tables and charts.** `leptos-struct-table` 0.19.1 (MIT OR Apache-2.0, ^0.8, 261 stars) for databook indexes. `leptos-chartistry` 0.2.3 (**MPL-2.0**, ^0.8, 148 stars) (copyleft per file, fine unmodified), or server-rendered SVG.
- **Forms and auth.** No maintained form crate; use `ActionForm` plus native validation. Auth: `axum-login` 0.18.0 (MIT, 1,022 stars); `leptos_oidc` 0.9.0 (MIT, GitLab, stars unchecked).
- **Markdown.** `pulldown-cmark` 0.13.6 (MIT, 2,757 stars, pushed 2026-11-03) at build time, `inner_html` server-side; `markdown` 1.0.0 for MDX. `rambip/leptos-markdown` is archived.
- **Styling.** `stylance` 0.8.4 (MIT OR Apache-2.0) scoped CSS modules, no runtime.
- **Skip:** `leptos_image` 0.2.0 and `leptos_query` 0.5.3 (both stuck on leptos 0.6), and `leptos-captcha` (archived).
- **Real apps to read:** `benwis/benwis_leptos` (MIT, pushed 2026-10-27), `itehax/rust-blog` (Apache-2.0), `leptos-rs/leptos-website` (stale since 2025-01), `examples/hackernews_islands_axum`. `khuedoan/blog` is GPL-3.0: read only.

## 4. Theming with yolo-tokens and no client JS

- `SITE/build.rs` already turns `src/tokens.rs` into `style/main.css` (`--color-*`, `--space-*`, `--radius-*`). The palette is hard-coded hex.
- `C:\src\yolo-ui\crates\yolo-emit\src\lib.rs` has `emit_preset(preset, Target::Css)` (line 219). It writes `--md-sys-color-*` (49 roles) plus `--yolo-*` preset variables, but only under `:root` and `prefers-color-scheme` (line 314).
- `yolo-tokens` is `no_std` and `no_alloc` without its `std` feature (its `Cargo.toml` 18-27), so it is safe in a build script and on wasm.

Design:

1. **Build time.** `SITE/build.rs` depends on `yolo-emit` (build-dependency, path `C:/src/yolo-ui/crates/yolo-emit`) and writes `[data-theme="<pack>"]` blocks with light/dark variants, via a new `css_scoped(selector, light, dark)` beside `css()`. Alias site names (`--color-primary`) to `--md-sys-color-*` roles.
2. **Theme pack** (a `ThemePack` struct in the shared kit): `preset: YoloThemePreset`, `seed`, `wallpaper` (exposed as `--yolo-wallpaper: url(...)`), `logo` (an SVG sprite id), `icon_set` (Phosphor weight), `title` and `site_name`. Colors, wallpaper, logo and icons are pure CSS (`var(--yolo-wallpaper)`, `mask-image`, `<use href="/sprite.svg#logo-{pack}">`). Title and site name are server-rendered from the pack.
3. **One switch, no JS.** Site default at start time (`SITE_THEME=rosegriffon`). Visitor switch: `<form method="get" action="/theme">` hits an Axum handler that sets a `theme` cookie and 303s back; middleware rewrites `<html data-theme>` in the prerendered HTML. **[unverified]** rewrite cost; prerendering per pack would multiply ~55,000 pages.
4. Optional: a theme-picker `#[island]` on `use_cookie`; the form remains the fallback.

## 5. Reuse table

| Crate | Version | License | wasm / SSR | What we take | App |
|---|---|---|---|---|---|
| leptos / leptos_router / leptos_meta / leptos_axum | 0.8.23 / 0.8.17 / 0.8.7 / 0.8.10 | MIT | both | islands, Static plus regenerate, meta | both |
| cargo-leptos | 0.3.11 | MIT | tool | hash-files, js-minify, split | both |
| leptos-use | 0.20.0 | MIT OR Apache-2.0 | both (browser APIs only in islands) | use_cookie, use_color_mode, intersection | both |
| phosphor-leptos | 0.8.0 | MIT | SSR SVG | icon set | both |
| leptos_icons + icondata | 0.7.1 / 0.7.0 | MIT | SSR SVG | fallback icon sets | both |
| leptos_i18n | 0.6.2 | MIT | both | plurals, localized paths (later) | rosegriffon [option] |
| leptos-struct-table | 0.19.1 | MIT OR Apache-2.0 | both | databook and wiki indexes | shenron |
| leptos-chartistry | 0.2.3 | MPL-2.0 | both | power-level and timeline charts | shenron [option] |
| floating-ui-leptos | 0.6.0 | MIT | island | popovers, tooltips | both |
| stylance | 0.8.4 | MIT OR Apache-2.0 | build | scoped CSS modules | both |
| pulldown-cmark | 0.13.6 | MIT | SSR/build | markdown articles | both |
| axum-login | 0.18.0 | MIT | server | admin and member sessions | rosegriffon [if needed] |
| yolo-emit / yolo-tokens | 0.8.0-alpha.3 (workspace) | MIT OR Apache-2.0 | build / no_std | scoped CSS per theme pack | both |

## 6. Ranked next steps

1. **Fix the manga prerender product** in `SITE/src/app.rs` (`manga_params`). Use a splat route and parse it in `SITE/src/pages/manga.rs`. Have invalid volume/chapter pairs render `NotFound`.
2. **Extract a shared kit**, `C:\src\yolo-stack\crates\yolo-site-kit` (proposed), following the start-axum-workspace split. Move `seo.rs`, `i18n.rs` (catalog-agnostic), `server/{sitemap,robots,redirects}.rs`, the `Lang` routing helpers and `ThemePack` there. Shenron and rosegriffon then share one SEO, sitemap and redirect implementation.
3. **Switch to islands.** Enable the `islands` feature in `SITE/Cargo.toml`, call `hydrate_islands()` in `SITE/src/lib.rs`, pass `islands=true` in `shell()` in `SITE/src/app.rs`, and make `ReaderIsland` an `#[island]` in `SITE/src/reader.rs`. Text pages then load no wasm.
4. **Generate the theme from yolo-tokens.** Add `css_scoped` to `C:\src\yolo-ui\crates\yolo-emit\src\lib.rs`. Make `SITE/build.rs` call it in place of the hard-coded hex in `SITE/src/tokens.rs` (keep spacing and radii there). Set `<Html attr:data-theme>` in the `Layout` in `SITE/src/app.rs`.
5. **Scaffold `C:\src\yolo-stack\apps\rosegriffon`** from SITE's `Cargo.toml` (standalone `[workspace]`, ssr/hydrate features, `hash-files`, `js-minify`) on top of `yolo-site-kit`, with `ThemePack::rosegriffon()`, `messages/fr.json` and a contact `#[server]` function with `ActionForm`.
6. Add `regenerate` streams, `phosphor-leptos` nav icons, JSON-LD in `seo.rs`, and a Bun-run `end2end/`.
7. Re-check Leptos 0.9 at stable (ADR reopening trigger).
