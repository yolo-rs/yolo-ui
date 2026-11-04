// SPDX-License-Identifier: MIT OR Apache-2.0
// Server side of TanStack Query: a QueryClient per request, prefetch helpers and the `HydrateClient`
// boundary that ships the dehydrated cache to the browser (the TanStack "advanced SSR" pattern).
import {
  type FetchInfiniteQueryOptions,
  type FetchQueryOptions,
  type QueryClient,
  type QueryKey,
  dehydrate,
} from "@tanstack/react-query";
import type { ReactNode } from "react";
import { requestStorage } from "../app/runtime/store";
import { HydrationBoundary } from "./hydration-boundary";
import { makeQueryClient, type QueryClientOptions } from "./make-query-client";

export { DEFAULT_STALE_TIME, makeQueryClient, type QueryClientOptions } from "./make-query-client";

/** Clients keyed on the request store: released with the store once the response is done. */
const clients = new WeakMap<object, QueryClient>();
let fallback: QueryClient | undefined;

/**
 * The QueryClient of the current request. Outside a request (scripts, tests, prerendering without a
 * store) a module singleton is returned instead. `options` only apply when the client is created.
 */
export function getQueryClient(options?: QueryClientOptions): QueryClient {
  const store = requestStorage.getStore();
  if (!store) return (fallback ??= makeQueryClient(options));
  let client = clients.get(store);
  if (!client) {
    client = makeQueryClient(options);
    clients.set(store, client);
  }
  return client;
}

/** Drop the module singleton (tests). */
export function resetQueryClient(): void {
  fallback = undefined;
}

/** `client.prefetchQuery` on the request client. */
export function prefetchQuery<
  TQueryFnData = unknown,
  TError = Error,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
>(options: FetchQueryOptions<TQueryFnData, TError, TData, TQueryKey>, client: QueryClient = getQueryClient()) {
  return client.prefetchQuery(options);
}

/** `client.prefetchInfiniteQuery` on the request client. */
export function prefetchInfiniteQuery<
  TQueryFnData = unknown,
  TError = Error,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
  TPageParam = unknown,
>(
  options: FetchInfiniteQueryOptions<TQueryFnData, TError, TData, TQueryKey, TPageParam>,
  client: QueryClient = getQueryClient(),
) {
  return client.prefetchInfiniteQuery(options);
}

export interface HydrateClientProps {
  children?: ReactNode;
  /** Defaults to the request client. */
  client?: QueryClient;
}

/**
 * Server component: dehydrates the request client (including pending prefetches) into a
 * `HydrationBoundary` so client components below it read the prefetched data without refetching.
 */
export function HydrateClient({ children, client }: HydrateClientProps) {
  return <HydrationBoundary state={dehydrate(client ?? getQueryClient())}>{children}</HydrationBoundary>;
}
