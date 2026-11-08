# @yolo-rs/stack-tanstack (planned)

TanStack Router 1.170.x + Query 5 + Start 1.169.x on Bun's native fullstack server.

Why not the documented `bun` deployment target: TanStack Start's own hosting guide builds with Vite
and a Nitro `bun` preset, then runs a reference `server.ts` under Bun. This package removes Vite and
Nitro from the path: a `Bun.build` client pass plus an SSR pass, and a `Bun.serve` handler with HTML
imports in dev (Bun HMR) and `bun build --production` for release. Server functions
(`createServerFn`) stay as-is; they are plain `fetch` calls.

Planned modules:

- `src/routes.ts`: file-route tree generated from `@yolo-rs/stack-routes`.
- `src/server.ts`: `Bun.serve` request handler, streaming SSR via `renderToReadableStream`.
- `src/query.ts`: re-export of the TanStack Query/Router adapters in
  `C:/src/yolo-ui/packages/yolo-fullstack/src/tanstack` once that package publishes.
- `src/island.tsx`: `<Island>` component that calls the `@yolo-rs/fullstack` Dioxus bridge.

RSC: TanStack Start marks Server Components experimental (React 19+, Vite 8+ or Rsbuild 2+). RSC
pages therefore come from `@yolo-rs/stack-next-compat` until Start stabilizes them.

Removal condition: none; this is the primary app shell (see `docs/DECISION.md`).
