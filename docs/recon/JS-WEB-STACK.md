# JS web stack recon: Next.js, Tailwind, shadcn, Base UI, Better Auth, Supabase, Elysia

Checked 2026-11-05. GitHub stats: `gh api repos/<o>/<r>` (stars, SPDX, last push) and `/releases` (latest stable).
Docs: Context7 (`/vercel/next.js`, `/tailwindlabs/tailwindcss.com`, `/shadcn-ui/ui`, `/mui/base-ui`,
`/better-auth/better-auth`, `/elysiajs/documentation`). Local facts: `C:\src\shenron\package.json`,
`C:\src\yolo-stack\docs\decisions\0003-verdict-leptos-dioxus-tanstack.md`, `C:\src\yolo-ui\crates\*`.
`[unverified]` marks claims not confirmed by a source this session.

## 0. Constraint that frames everything

ADR 0003 (accepted 2026-11-05) settles the end state for Shenron: **Leptos 0.8 on Axum** for the SEO site, **Dioxus** for native, and Bun services for the bot, API and pipeline. Next.js, Tailwind, Base UI and shadcn are removed; `@yolo-rs/fullstack` and the Next 16.5 fork are a **migration bridge** only. So this stack matters for keeping current sites running on Bun and for feeding `yolo-migrator`, not for new work.

Shenron pins today (root `package.json`): `next 16.5.0-canary.5` (vendored fork tgz), `react ^19.3.0`, `tailwindcss ^4.3.3`,
`@tailwindcss/postcss`, `@base-ui/react ^1.8.0`, `shadcn ^4.21.0`, `better-auth 1.7.14` plus `@better-auth/{passkey,oauth-provider,cimd,mcp}`,
`drizzle-orm ^0.45.3`, `kysely`, `postgres` (porsager). Shenron auth references Discord, Google, `genericOAuth`, magic link,
passkey, twoFactor and username. I found no Steam or Epic code. Whether rosegriffon.fr uses the same stack is `[unverified]`: I did not open its repo.

## 1. Inventory

| Project | Latest stable | License | Stars | Last push | Bun |
|---|---|---|---|---|---|
| facebook/react | 19.3.1 (2026-10-14) | MIT | 251,976 | 2026-11-04 | Works (pure JS) |
| vercel/next.js | 16.4.5 (2026-11-03) | MIT | 143,335 | 2026-11-05 | Partial; owner runs it through the fork + `@yolo-rs/fullstack` |
| tailwindlabs/tailwindcss | 4.3.4 (2026-11-02) | MIT | 98,063 | 2026-11-04 | Works (`@tailwindcss/cli`, Oxide native binary) `[unverified on 1.4.4]` |
| shadcn-ui/ui | shadcn@4.23.0 (2026-10-27) | MIT | 127,393 | 2026-11-05 | `bunx --bun shadcn` `[unverified]` |
| mui/base-ui | 1.9.0 (2026-10-09) | MIT | 11,301 | 2026-11-04 | Works (React lib) |
| better-auth/better-auth | 1.7.17 (2026-11-04) | MIT | 30,598 | 2026-11-05 | Works; Shenron runs it on Bun |
| supabase/supabase | v1.26.08 self-host (2026-08-07) | Apache-2.0 | 114,546 | 2026-11-05 | supabase-js works `[unverified]`; CLI is a Go binary |
| elysiajs/elysia | 1.4.31 (2026-11-05) | MIT | 19,330 | 2026-11-05 | Native target |
| elysiajs/eden | n/a | MIT | 346 | 2026-10-28 | Native |
| oven-sh/bun | 1.4.4 (2026-10-22) | NOASSERTION (MIT + LGPL JSC) | 96,547 | 2026-11-05 | n/a |
| leptos-rs/leptos | 0.8.23, 0.9.0-rc1 (2026-10-31) | MIT | 21,463 | 2026-11-04 | Rust |
| DioxusLabs/dioxus | n/a | Apache-2.0 | 39,607 | 2026-11-03 | Rust |

Shenron is three patches behind on Better Auth (1.7.14 vs 1.7.17); its Next canary.5 predates the canary.40 fork named in ADR 0003.

## 2. React 19 / Next.js 16: what migration tooling must parse

Source: Next docs `upgrading/version-16.mdx`, `migrating-to-cache-components.mdx`, `16-proxy.mdx`, `cacheLife.mdx`.

- **Cache Components.** `cacheComponents: true` in `next.config.ts` replaces `experimental.dynamicIO` and `experimental.useCache`. The `'use cache'` directive goes in functions or components, together with `cacheLife('max'|profile)` and `cacheTag(tag)`. Profiles: `cacheLife: { name: { stale, revalidate, expire } }` (seconds). Mapping: `'use cache'` + `cacheLife` becomes Leptos `SsrMode::Static` + `RegenerationFn` (ISR). `cacheTag`/`revalidateTag` become a tag-to-route invalidation table. `yolo-next` exposes `./cache` (directive coverage `[unverified]`).
- **proxy.ts.** It replaces `middleware.ts`. The file exports a `proxy(request)` function (named or default) plus `config.matcher`, and it lives at the project root or in `src/`. Map it to an Axum `tower` layer, keyed by the matcher. `@yolo-rs/fullstack` already runs it (ADR 0003).
- **React Compiler.** `reactCompiler: true` is stable, off by default, Babel-based (slower builds). Treat it as a no-op flag in the OXC pipelines; memoization is irrelevant to Rust targets.
- **Server actions.** `'use server'` functions become Leptos `#[server]` fns. ADR 0003 sets the boundary contract as "HTTP + JSON server-fn". `yolo-next/src/actions.ts` exists.
- **React 19.x.** `use()`, `useActionState`, `useOptimistic`, `<form action>` and `<Activity>` each need an IR node or a `diag.rs` diagnostic.

## 3. Tailwind v4, shadcn/ui, Base UI

**Tailwind v4** (theme.mdx, colors.mdx). Configuration lives in CSS: `@import "tailwindcss"`, then `@theme { --color-*, --font-*, --text-*, --spacing-*, --radius-*, --shadow-*, --breakpoint-*, --ease-*, --animate-* ... }`.
`@theme inline` is required when a value is a runtime `var(--x)`, which is the pattern for per-theme HCT schemes. `@theme static` emits every variable even when unused (token dump). `@custom-variant dark (&:is(.dark *))` defines dark mode; `@source "<path>"` adds scan paths. With no `tailwind.config.js`, the migrator only needs `@theme` parsing, which exists as `TailwindV4Theme::parse_css` in `yolo-migrator/src/shadcn_registry.rs:469-536`.

**shadcn/ui CLI v4** (cli.mdx, changelog 2026-03-cli-v4). The command is `shadcn init|create` with `-b, --base <base|radix|aria>`. `-t` selects the template (next, vite, start, react-router, laravel, astro). `--defaults` gives `--template=next --preset=base-nova`. Other flags: `--monorepo`, `--rtl`, `--css-variables` (on by default). Docs show only `npx`; the Bun form `bunx --bun shadcn@latest init -b base` is `[unverified]`.
The theming contract is `:root`/`.dark` OKLCH variables: `--background --foreground --card(-foreground) --popover(-foreground) --primary(-foreground) --secondary(-foreground) --muted(-foreground) --accent(-foreground) --destructive --border --input --ring --chart-1..5 --sidebar(-foreground|-primary|-primary-foreground|-accent|-accent-foreground|-border|-ring) --radius`.
These are bridged through `@theme inline { --color-X: var(--X) }`, with radii as multiples of `--radius` (sm 0.6, md 0.8, lg 1, xl 1.4, up to 4xl 2.6). Registries use `registry.json`/`registry-item.json`, and `ShadcnRegistryItem` already parses them.

**Base UI 1.9** (`docs/.../react/components`). 38 component directories: accordion, alert-dialog, autocomplete, avatar, button, checkbox, checkbox-group, collapsible, combobox, context-menu, dialog, drawer, field, fieldset, form, input, menu, menubar, meter, navigation-menu, number-field, otp-field, popover, preview-card, progress, radio, radio-group, scroll-area, select, separator, slider, switch, tabs, toast, toggle, toggle-group, toolbar, tooltip. They are unstyled compound parts (`Dialog.Root/Trigger/Popup`). `className` and `style` accept either a value or `(state) => value`. A `render` prop replaces the element (Radix uses `asChild`). State is exposed as `data-*` attributes (`data-open`, `data-checked`, `data-starting-style`, `data-ending-style`, `data-popup-side`, Field validity `data-valid/invalid/dirty/touched`) and as CSS variables (`--nested-dialogs`). Migrator mapping: state-function className becomes a Rust `match` on state; `data-[open]:` variants become conditional classes.

## 4. Auth, data, server

**Better Auth 1.7.17.** The packages are `better-auth`, `core`, `cli`, `{drizzle,kysely,prisma,mongo,memory}-adapter`, `passkey`, `sso`, `scim`, `oauth-provider`, `mcp`, `cimd`, `api-key`, `stripe`, `expo`, `electron`, `i18n` and `redis-storage`.
Built-in social providers (`packages/core/src/social-providers`) include **google, microsoft-entra-id** (`tenantId` default `common`, `authority`, `clientAssertion`), **discord** (email may be `null`), **github**, twitch, apple, roblox, reddit, slack, gitlab, linear, kick and about 25 others.
- **Steam has no built-in provider.** Steam login is OpenID 2.0, not OAuth2/OIDC, so `genericOAuth` (which needs a `discoveryUrl` or OAuth2 endpoints) cannot drive it directly. It needs a custom plugin verifying `openid.*` assertions against `steamcommunity.com/openid/login` `[unverified: no maintained community plugin found]`.
- **Epic Games has no built-in provider.** Use `genericOAuth` with Epic Account Services OAuth2 `[unverified: Epic OIDC discovery URL]`.
- Adapters: a `pg` `Pool`, a Kysely `PostgresDialect` with `schemaName` (docs show `schemaName: "auth"`), or `drizzleAdapter(db,{provider:"pg"})`, which is what Shenron uses. Neon and Supabase are plain Postgres connection strings here. Native `Bun.sql` support is `[unverified]`; today's path is porsager `postgres` through Drizzle.
- Elysia integration (elysia docs `integrations/better-auth.md`): `.mount('/auth', auth.handler)`, plus the `openAPI()` plugin and `auth.api.generateOpenAPISchema()` merged into `@elysia/openapi`.

**Supabase** (Apache-2.0). It bundles GoTrue auth (`supabase/auth`, 2,593 stars, MIT), Postgres, Storage (`supabase/storage`, Apache-2.0, S3-compatible) and Realtime (`supabase/realtime`, Elixir, Apache-2.0). Self-hosting uses `docker/docker-compose.yml` (variants: pg15, pg17, pgbouncer, s3, rustfs, caddy, nginx, envoy, kong). Verdict: keep Better Auth for identity. Use Supabase or Neon only as Postgres (plus optional Storage), so sessions are not split across two auth systems.

**Elysia 1.4.31** (MIT). Bun-first and built on `Bun.serve`. **Eden Treaty** gives end-to-end typed clients from `typeof app`. `@elysia/openapi` supersedes `@elysiajs/swagger` `[unverified: package rename date]`. It fits ADR 0003's Bun services layer and can host Better Auth once Next is gone.

## 5. Templates, awesome lists, reference apps

| Resource | Stars | License | Last push | Note |
|---|---|---|---|---|
| birobirobiro/awesome-shadcn-ui | 20,769 | MIT | 2026-11-03 | Main shadcn index |
| ui.shadcn.com/blocks, /templates | n/a | MIT | n/a | Official blocks (dashboard, sidebar, login) |
| Kiranism/next-shadcn-dashboard-starter | 7,227 | MIT | 2026-11-03 | Good migrator fixture |
| vercel/examples | 5,162 | MIT | 2026-11-04 | Plus `next.js/examples/` in repo |
| elysiajs/awesome-elysia | 373 | CC0-1.0 | 2026-09-20 | Plugin index |
| better-auth/examples | 46 | none | 2026-02-06 | Stale, unlicensed: read only |
| calcom/cal.diy (was cal.com) | 49,700 | MIT | 2026-11-03 | Large Next+Tailwind app |
| dubinc/dub.link (was dub) | 25,107 | NOASSERTION | 2026-10-27 | Next+Tailwind; license mixed (check before reuse) |

Both renames are `gh` redirects; whether they are intentional upstream changes is `[unverified]`.

## 6. Mapping onto our stack

| Item | Keep as Bun library (bridge) | Migrator target | End state |
|---|---|---|---|
| Next 16 app/ | `@yolo-rs/fullstack`, `@yolo-rs/next` | `yolo-next` parses routes, `'use cache'`, `proxy.ts`, actions | Leptos routes |
| React 19 components | `@yolo-rs/react` | `yolo-migrator` TSX to IR | Leptos `view!` (web), Dioxus `rsx!` (native) |
| shadcn + Base UI | `@yolo-rs/shadcn-bun` (inspector) | IR contract components (`ir.rs:15`) | Leptos/Dioxus component set |
| Tailwind v4 | CSS build until port | `TailwindV4Theme` + class resolver (`css.rs`) | Tokens from `yolo-tokens` |
| Better Auth | Keep (Bun, MIT) | none | Bun service behind Axum |
| Elysia + Eden | Adopt for Bun API | none | Bun API layer |
| Supabase | Optional managed Postgres/Storage | none | Postgres via Drizzle |

**Gap.** `MigrationTarget` (`crates/yolo-migrator/src/lib.rs:35`) supports dioxus, wgpu, egui, cosmic, gtk and qml. **It has no Leptos target**, although ADR 0003 makes Leptos the web end state.

**Token bridge.** `yolo-emit` `Target::Css` writes only `--md-sys-color-<role>` (`crates/yolo-emit/src/lib.rs:308-323`), and no shadcn or `@theme` output exists. The fix is a `Target::Tailwind` that emits three layers:
1. `:root`/`.dark` shadcn variables from `DynamicScheme` roles: background=surface, foreground=on-surface, card=surface-container-low, popover=surface-container-high, primary/-foreground=primary/on-primary, secondary=secondary-container/on-secondary-container, muted=surface-container-highest/on-surface-variant, accent=tertiary-container/on-tertiary-container, destructive=error, border=outline-variant, input=outline, ring=primary, chart-1..5 from five tonal palettes at tone 60 (dark 70), sidebar-* from surface-container. Write the values in `oklch()` to match the shadcn defaults (convert from ARGB).
2. `@theme inline { --color-X: var(--X); --color-md-<role>: var(--md-sys-color-<role>) }`, so both naming schemes produce utilities.
3. `--radius` from `yolo-tokens/src/shape.rs`, plus `--font-*`, `--ease-*` and `--animate-*` from `typography.rs` and `motion.rs`.
The roles follow shadcn semantics (muted = low emphasis, accent = hover). Contrast must pass the `yolo-contracts` WCAG checks.

## 7. Ranked next steps

1. **Add `Target::Tailwind` (shadcn variables + `@theme inline`) to `C:\src\yolo-ui\crates\yolo-emit\src\lib.rs`**, with the role table in `C:\src\yolo-ui\catalog\conversion.toml`, so all 7 presets can theme existing Shenron and rosegriffon CSS today.
2. **Add `MigrationTarget::Leptos`** as `C:\src\yolo-ui\crates\yolo-migrator\src\leptos.rs`, registered in `lib.rs:35`, reusing the IR from `dioxus.rs`. This aligns the migrator with ADR 0003.
3. **Map Base UI semantics in the IR**: `render` prop, state-function `className`, and `data-*` variants, in `crates/yolo-migrator/src/ir.rs` and `tsx.rs`. Cover all 38 components above in `shadcn_registry.rs::resolve_primitive_import`, including drawer, otp-field and autocomplete.
4. **Add Next 16 directive coverage to `C:\src\yolo-stack\packages\yolo-next\src\{cache,migrate,rules}.ts`**: `cacheComponents`, `'use cache'`/`cacheLife`/`cacheTag`, the `proxy.ts` matcher and `reactCompiler`. Emit Leptos `SsrMode`/ISR hints and an Axum layer stub.
5. **Move auth to a Bun service**: bump `better-auth` 1.7.14 to 1.7.17 in `C:\src\shenron\package.json` (owner action) and prototype Elysia `.mount('/auth', auth.handler)` + `openAPI()`. Add a Steam OpenID 2.0 plugin in `C:\src\shenron\packages\auth\src\` and an Epic `genericOAuth` entry.
6. **Golden fixtures**: `Kiranism/next-shadcn-dashboard-starter` (MIT) and the shadcn `base-nova` blocks, under `C:\src\yolo-ui\crates\yolo-migrator\tests\`.
