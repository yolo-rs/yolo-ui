# next-fixture

Static Next.js 16 App Router project used as a migration fixture by the Rust crate
`crates/yolo-next` and the Bun package `packages/yolo-next`. Its only purpose is to
exercise the whole Next.js public surface so the analyzers and migrators can be tested
against realistic input.

Do not install or run this project; it is a static fixture.

## Next.js features covered

- `next.config.ts`: Turbopack rules and aliases, images (`remotePatterns`, `localPatterns`,
  `qualities`), redirects, rewrites (`beforeFiles`/`afterFiles`/`fallback`), headers, compiler
  options, `cacheLife` profiles, `cacheComponents`, `reactCompiler`, `typedRoutes`,
  `serverExternalPackages`, `deploymentId`, experimental flags, and the removed `eslint` key.
- `proxy.ts` (replacement for `middleware.ts`) with redirect, rewrite, header mutation and a
  mixed `matcher`.
- App Router conventions: `layout`, `template`, `loading`, `error`, `global-error`,
  `not-found`, `default`, route groups, dynamic, catch-all and optional catch-all segments,
  parallel routes (`@analytics`, `@modal`), intercepting routes (`(.)photos`), private folders.
- Route segment config: `revalidate`, `dynamic`, `dynamicParams`, `fetchCache`, `maxDuration`,
  and the deprecated `runtime = "edge"`.
- Data and caching: `"use cache"`, `cacheLife`, `cacheTag`, `updateTag`, `revalidateTag`,
  `revalidatePath`, `refresh`, `unstable_cache`, Server Actions with `useActionState`.
- Metadata: static and generated metadata, `viewport`, `sitemap`, `robots`, `manifest`,
  `opengraph-image`, `icon.svg`.
- Built-ins: `next/image` (local, remote, unmatched remote), `next/link`, `next/script`,
  `next/font/google`, `next/font/local`, `next/dynamic`, `next/navigation`, `next/headers`.
- Route Handlers (`app/api/**/route.ts`) and legacy Pages Router files (`pages/legacy.tsx`,
  `pages/_app.tsx`, `pages/api/legacy.ts`).
- Instrumentation: `instrumentation.ts` and `instrumentation-client.ts`.

## Note on raw files

`app/globals.css`, `app/icon.svg`, `public/*.svg` and the font placeholder exist only because a Next.js 16
project contains them: they are migration input data consumed by `crates/yolo-next` and `packages/yolo-next`,
never framework source. The yolo-stack rule "no raw .html/.css/.js source" applies to emitted code, which
takes its tokens from `yolo-tokens` presets.
