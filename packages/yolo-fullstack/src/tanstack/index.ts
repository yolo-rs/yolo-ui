// SPDX-License-Identifier: MIT OR Apache-2.0
// TanStack Router and Query on the `app/` scan: generated route tree, Bun.serve route handlers,
// per-request QueryClient and hydration. The `"use client"` pieces live in `./query-client`.
export {
  HTTP_METHODS,
  bunPatternFromRoute,
  bunPatternsFromRoute,
  bunRouteHandler,
  bunRoutesFromTree,
  serveSpa,
  type BunRouteHandler,
  type BunRoutesOptions,
  type HttpMethod,
  type ModuleLoader,
  type RouteContext,
  type RouteHandler,
  type RouteModule,
  type ServeSpaOptions,
} from "./bun-routes";
export { HydrationBoundary, type HydrationBoundaryProps } from "./hydration-boundary";
export { DEFAULT_STALE_TIME, makeQueryClient, type QueryClientOptions } from "./make-query-client";
export {
  HydrateClient,
  getQueryClient,
  prefetchInfiniteQuery,
  prefetchQuery,
  resetQueryClient,
  type HydrateClientProps,
} from "./query";
export { QueryProvider, getBrowserQueryClient, type QueryProviderProps } from "./query-client";
export {
  ROUTE_TREE_MODULE,
  renderRouteTree,
  routeTreePlugin,
  tanstackRoutePath,
  type RenderRouteTreeOptions,
} from "./router";
