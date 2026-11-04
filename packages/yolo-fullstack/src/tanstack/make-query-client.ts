// SPDX-License-Identifier: MIT OR Apache-2.0
// QueryClient factory shared by the server (`query.tsx`) and the client (`query-client.tsx`) modules.
// Neutral module (no directive) so a server component gets the real function, not a client reference.
import { QueryClient, type QueryClientConfig, defaultShouldDehydrateQuery } from "@tanstack/react-query";

export type QueryClientOptions = QueryClientConfig;

/** Default `staleTime` of every query: data prefetched on the server is not refetched right after hydration. */
export const DEFAULT_STALE_TIME = 60_000;

/**
 * A QueryClient with the TanStack "advanced SSR" defaults: one minute of staleness and dehydration of
 * pending queries, so a server prefetch that is still in flight streams to the browser as a promise.
 */
export function makeQueryClient(options: QueryClientOptions = {}): QueryClient {
  const { defaultOptions, ...rest } = options;
  return new QueryClient({
    ...rest,
    defaultOptions: {
      ...defaultOptions,
      queries: { staleTime: DEFAULT_STALE_TIME, ...defaultOptions?.queries },
      dehydrate: {
        shouldDehydrateQuery: query => defaultShouldDehydrateQuery(query) || query.state.status === "pending",
        ...defaultOptions?.dehydrate,
      },
    },
  });
}
