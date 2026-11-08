# showcase-tanstack (planned)

TanStack Router + Query + Start app on Bun's native fullstack server; the primary yolo shell.

Planned layout:

```text
src/
  routes/            # generated from @yolo-rs/stack-routes
  server.ts          # Bun.serve: HTML imports in dev (HMR), streaming SSR
  client.tsx         # hydrateRoot + RouterProvider
  islands/           # <Island name="counter"> mounts the Dioxus island from showcase-dioxus
scripts/build.ts     # Bun.build client + ssr passes, bun build --production
```

Measured when it exists: build time, `dist/client` JS size (raw/gzip/brotli), rps and latency per
route under `bench/`.
