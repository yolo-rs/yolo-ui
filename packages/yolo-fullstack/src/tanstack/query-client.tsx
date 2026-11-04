// SPDX-License-Identifier: MIT OR Apache-2.0
"use client";
// Browser side of TanStack Query: one QueryClient for the lifetime of the page, a fresh one per
// server render (a module singleton would leak data between requests during SSR).
import { QueryClientProvider, isServer, type QueryClient } from "@tanstack/react-query";
import { type ReactNode, useState } from "react";
import { makeQueryClient, type QueryClientOptions } from "./make-query-client";

export { DEFAULT_STALE_TIME, makeQueryClient, type QueryClientOptions } from "./make-query-client";

let browserQueryClient: QueryClient | undefined;

/** The browser singleton (created on first use); a new client on the server. */
export function getBrowserQueryClient(options?: QueryClientOptions): QueryClient {
  if (isServer) return makeQueryClient(options);
  return (browserQueryClient ??= makeQueryClient(options));
}

export interface QueryProviderProps {
  children?: ReactNode;
  options?: QueryClientOptions;
}

/** Wraps the app in a `QueryClientProvider` (use it in the root layout). */
export function QueryProvider({ children, options }: QueryProviderProps) {
  // Lazy useState rather than a module variable: React may suspend before the first render finishes,
  // and a client created during render would otherwise be thrown away on retry.
  const [client] = useState(() => getBrowserQueryClient(options));
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
