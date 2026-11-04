# @yolo-rs/fullstack

A Bun-native replacement for Next.js. The Next.js `app/` conventions (page, layout, template, loading, error,
not-found, route handlers, route groups, dynamic and catch-all segments, metadata files, `proxy.ts`
middleware) with React Server Components, server actions, streaming SSR and static generation, built by three
`Bun.build` passes (rsc, client, ssr) and served by `Bun.serve`. No Node.js, no webpack, no Turbopack binary:
Bun 1.4.4+ is the whole toolchain.

Status 2026-11-04: `bun test` 141 tests green in this package, including an end-to-end build and serve of
`fixtures/app-basic` (next.config rules, Turbopack aliases and rules, `next/font/local`, `next/image`
through the optimizer with sharp, route handlers, prerendering). `bun check` and `bun lint` clean. Google
Fonts are tested against a stub; the live Google Fonts API was not called in CI.

## Commands

```sh
bun x yolo-fullstack migrate [--apply]   # report a Next.js project (and rewrite package.json, tsconfig, .gitignore)
bun x yolo-fullstack dev [--port 3000]   # rebuild on change, RSC refresh over WebSocket
bun x yolo-fullstack build [--no-prerender] [--offline]
bun x yolo-fullstack start
bun x yolo-fullstack compile [--target bun-linux-x64-musl]   # one executable
bun x yolo-fullstack docker                                   # Dockerfile + server.js on oven/bun
bun x yolo-fullstack config                                   # the resolved next.config and the ignored keys
```

`--turbopack` / `--turbo` are accepted on every command and reported as no-ops.

## What maps to what

| Next.js                                                                                                                                     | Here                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `next.config.{js,mjs,ts,mts,cjs}` (object, function, async)                                                                                 | loaded by `loadNextConfig`; `basePath`, `assetPrefix`, `trailingSlash`, `skipTrailingSlashRedirect`, `poweredByHeader`, `env`, `images`, `turbopack`, `rewrites` (beforeFiles, afterFiles, fallback, `has`/`missing`, external proxying), `redirects`, `headers`, `serverRuntimeConfig`/`publicRuntimeConfig` are honoured; every other key is listed under `unsupported` with the reason                             |
| Turbopack (`turbopack.resolveAlias`, `resolveExtensions`, `rules`)                                                                          | mapped onto `Bun.build` by `turbopackPlugin`: plain, list, conditional and `pkg/*` aliases; `raw-loader`, `file-loader`, `url-loader`, `json`, `toml`, `wasm`, `@svgr/webpack` rules                                                                                                                                                                                                                                  |
| `next/font/google`, `next/font/local`                                                                                                       | resolved at build time: Google CSS fetched once with a woff2 user agent, files self-hosted under `/_yolo/media/fonts/`, cached in `.yolo/fonts/`, `className` / `style` / `variable`, `<link rel="preload">` per page; dev falls back to the system stack offline, `build` fails unless `YOLO_FONT_FALLBACK=1`                                                                                                        |
| `next/image` + `/_next/image`                                                                                                               | `Image`, `getImageProps`, `srcSet` from `deviceSizes` / `imageSizes` / `sizes`, loaders `default`, `vercel`, `imgix`, `cloudinary`, `akamai`, `cloudflare`, `fastly`, `custom` (`images.loaderFile`); the optimizer validates `remotePatterns` / `localPatterns` / `w` / `q`, negotiates avif/webp, resizes with `sharp` when installed (pass-through otherwise), caches in memory and on disk with `minimumCacheTTL` |
| CDN for static assets                                                                                                                       | `assetPrefix` is baked into the client bundles, stylesheets and font URLs; the host keeps serving `/_yolo/*` as the CDN origin                                                                                                                                                                                                                                                                                        |
| `next/link`, `next/navigation`, `next/headers`, `next/server`, `next/cache`, `next/dynamic`, `next/script`, `next/constants`, `next/config` | module aliases (`NEXT_COMPAT`), `basePath` applied by `Link`                                                                                                                                                                                                                                                                                                                                                          |
| Server Components, server actions, `useActionState`, progressive forms                                                                      | `react-server-dom-parcel` 19                                                                                                                                                                                                                                                                                                                                                                                          |
| `generateStaticParams`, `dynamic = "force-dynamic"`, `cookies()`, `headers()`, `connection()`                                               | prerendering with dynamic detection                                                                                                                                                                                                                                                                                                                                                                                   |
| `robots`, `sitemap`, `manifest` files, `proxy.ts` / `middleware.ts` with `config.matcher`                                                   | supported                                                                                                                                                                                                                                                                                                                                                                                                             |
| Parallel routes, intercepting routes, `use cache`, Cache Components, `next/og`, edge runtime, Pages Router, `i18n`, `webpack()`             | not supported; `yolo-fullstack migrate` lists every occurrence with the file                                                                                                                                                                                                                                                                                                                                          |

## TanStack

`@yolo-rs/fullstack/tanstack`:

- `QueryProvider` (client) and `makeQueryClient`: one QueryClient per request on the server, one per browser.
- `getQueryClient()` keyed on the request store, `HydrateClient` server component wrapping `HydrationBoundary` with the dehydrated cache (pending queries included), the official App Router pattern.
- `renderRouteTree(tree)` / `routeTreePlugin(tree)`: a `@tanstack/react-router` tree generated from the `app/` scan (`[slug]` to `$slug`, catch-all to `$`, layouts, loading/error/not-found), served as the virtual module `yolo:route-tree`.
- `bunRoutesFromTree(tree, load)` and `serveSpa({ html, tree })`: `route.ts` handlers as `Bun.serve({ routes })` entries next to an HTML import, Bun's native fullstack dev server with HMR for the SPA mode.

## Dioxus 0.8

`@yolo-rs/fullstack/dioxus`:

- `dioxusWebRoutes({ dir })`: serve a `dx build --platform web` output (index.html, assets, wasm) from `Bun.serve` routes, with SPA fallback and traversal guards.
- `dioxusProxy({ target })` and `dioxusBridge(...)`: stream `/api/*` server-function calls to the Dioxus fullstack (axum) server.
- `serverFnPath` and `callServerFn`: call `#[server]` functions from TypeScript; the implicit endpoint hash (`xxh64(CARGO_MANIFEST_DIR:module_path)`) is reproduced with `Bun.hash.xxHash64`.
- `dxBuild`: run `dx build` through `Bun.spawn` and locate the public directory.

## Programmatic use

```ts
import { buildApp, prerenderApp, serveApp, loadNextConfig } from "@yolo-rs/fullstack";

const config = await loadNextConfig(root);
console.log(config.unsupported);
await buildApp({ root, outDir: "dist", config });
await prerenderApp("dist");
await serveApp({ outDir: "dist", port: 3000 });
```

Build output: `client/` (served at `/_yolo/`), `server/rsc`, `server/ssr`, `static/`, `prerender/` and
`manifest.json` (version 2: routes, client modules, stylesheets, font preloads, config rules, image config,
asset prefix, unsupported keys).

## Origin

`src/app/` is migrated from the `@aphrody/next-bun/app` App Router of Bun UI; see [NOTICE.md](NOTICE.md) for
the upstream commit, path and license. Everything else is new in `yolo-rs/yolo-ui`.
