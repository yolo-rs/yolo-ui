# @yolo-rs/stack-next-compat (planned)

Next.js App Router migration layer. It owns no rendering code; it wires `@yolo-rs/fullstack`
(`C:/src/yolo-ui/packages/yolo-fullstack`, migrated from `C:/src/bun-ui/packages/bun-next/app`)
into the yolo app shell.

What `@yolo-rs/fullstack` already provides (read-only reference, 2026-11-04): Next `app/` conventions
(page, layout, template, loading, error, not-found, route handlers, groups, dynamic and catch-all
segments, metadata routes, `proxy.ts`), React Server Components through `react-server-dom-parcel`,
server actions, streaming SSR, prerender, `next.config` loader with Turbopack config mapping,
`next/font`, a `next/image` CDN endpoint, TanStack Query/Router adapters and a Dioxus bridge.

This package adds:

- `src/index.ts`: `mountNextApp(appDir, shell)` that registers the Next routes on the same
  `Bun.serve` instance as the TanStack shell, so migration happens one route at a time.
- A dependency on `@yolo-rs/fullstack` as `file:../../../yolo-ui/packages/yolo-fullstack` until that
  package has a `package.json`, then `workspace:*` or a git dependency. The dependency is not
  declared yet so `bun install` stays green.

Removal condition: when no consumer has a Next `app/` tree left, this package is deleted and
`@yolo-rs/fullstack` is consumed directly by `@yolo-rs/stack-tanstack` for RSC.
