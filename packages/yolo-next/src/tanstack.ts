// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/tanstack: TanStack Router / Query bridge (query hydration, server functions, page loaders).

import { createElement, type ReactElement, type ReactNode } from "react";
import {
  HydrationBoundary,
  QueryClient,
  QueryClientProvider,
  dehydrate,
  type DehydratedState,
} from "@tanstack/react-query";
import { callServer, serverReferenceId } from "./actions.ts";

const QUERY_STATE_GLOBAL = "__YOLO_QUERY_STATE__";
const TANSTACK_ROUTER_GLOBAL = "__TSR_ROUTER__";
const SSR_QUERY_PACKAGE = "@tanstack/react-router-ssr-query";

function isServer(): boolean {
  return typeof window === "undefined" || typeof document === "undefined";
}

/** Creates a per-request `QueryClient` (queries stay fresh for 60 s so SSR data is reused on the client). */
export function createRequestQueryClient(): QueryClient {
  return new QueryClient({ defaultOptions: { queries: { staleTime: 60_000 } } });
}

/** Escapes `<`, `>`, `&`, U+2028 and U+2029 so JSON can be embedded inside a `<script>` element. */
export function escapeInlineJson(json: string): string {
  return json
    .replace(/</g, "\\u003c")
    .replace(/>/g, "\\u003e")
    .replace(/&/g, "\\u0026")
    .replace(new RegExp(String.fromCharCode(0x2028), "g"), "\\u2028")
    .replace(new RegExp(String.fromCharCode(0x2029), "g"), "\\u2029");
}

/** Serializes the dehydrated query cache as JSON safe for an inline script. */
export function dehydrateForHtml(queryClient: QueryClient): string {
  return escapeInlineJson(JSON.stringify(dehydrate(queryClient)));
}

/** Inline script assigning the dehydrated state to `window.__YOLO_QUERY_STATE__`. */
export function HydrationScript(props: { readonly state: string; readonly nonce?: string }): ReactElement {
  return createElement("script", {
    nonce: props.nonce,
    dangerouslySetInnerHTML: { __html: `window.${QUERY_STATE_GLOBAL}=${props.state};` },
  });
}

/** Reads the dehydrated state injected by `HydrationScript` (client only; `undefined` on the server). */
export function readHydratedQueryState(): DehydratedState | undefined {
  if (isServer()) {
    return undefined;
  }
  const state = (globalThis as Record<string, unknown>)[QUERY_STATE_GLOBAL];
  return typeof state === "object" && state !== null ? (state as DehydratedState) : undefined;
}

/** `QueryClientProvider` plus a `HydrationBoundary` fed by `window.__YOLO_QUERY_STATE__` on the client. */
export function YoloQueryProvider(props: {
  readonly client: QueryClient;
  readonly state?: DehydratedState;
  readonly children?: ReactNode;
}): ReactElement {
  const state = props.state ?? readHydratedQueryState();
  return createElement(
    QueryClientProvider,
    { client: props.client },
    createElement(HydrationBoundary, { state }, props.children),
  );
}

/** Registers the mounted TanStack router so `next/navigation` hooks delegate to it. */
export function registerTanStackRouter(router: unknown): void {
  (globalThis as Record<string, unknown>)[TANSTACK_ROUTER_GLOBAL] = router;
}

/** Removes the registered TanStack router. */
export function unregisterTanStackRouter(): void {
  delete (globalThis as Record<string, unknown>)[TANSTACK_ROUTER_GLOBAL];
}

/** The registered TanStack router, or `undefined` when none is mounted. */
export function useTanStackRouter<T = unknown>(): T | undefined {
  const router = (globalThis as Record<string, unknown>)[TANSTACK_ROUTER_GLOBAL];
  return router === undefined || router === null ? undefined : (router as T);
}

/** Options forwarded to `setupRouterSsrQueryIntegration`. */
export interface RouterSsrQueryOptions {
  readonly router: unknown;
  readonly queryClient: QueryClient;
  readonly handleRedirects?: boolean;
  readonly wrapQueryClient?: boolean;
}

/** Calls `setupRouterSsrQueryIntegration` from `@tanstack/react-router-ssr-query` when installed; returns false otherwise. */
export async function setupRouterSsrQueryIntegration(options: RouterSsrQueryOptions): Promise<boolean> {
  try {
    const specifier: string = SSR_QUERY_PACKAGE;
    const module = (await import(specifier)) as {
      setupRouterSsrQueryIntegration?: (options: RouterSsrQueryOptions) => unknown;
    };
    if (typeof module.setupRouterSsrQueryIntegration !== "function") {
      return false;
    }
    module.setupRouterSsrQueryIntegration(options);
    return true;
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Server functions (TanStack Start-like builder over Server Actions)
// ---------------------------------------------------------------------------

/** HTTP method of a server function. */
export type ServerFnMethod = "GET" | "POST";

/** Context passed to a server function handler. */
export interface ServerFnContext<TInput> {
  readonly data: TInput;
  readonly method: ServerFnMethod;
  readonly id: string;
}

/** Options of `createServerFn` (`id` is explicit because Bun has no compile step to inject one). */
export interface ServerFnOptions {
  readonly method?: ServerFnMethod;
  readonly id: string;
}

/** Callable server function: direct call on the server, `callServer` RPC on the client. */
export type ServerFn<TInput, TOutput> = ((call?: { readonly data?: TInput }) => Promise<TOutput>) & {
  readonly id: string;
  readonly method: ServerFnMethod;
};

/** Builder returned by `createServerFn`. */
export interface ServerFnBuilder<TInput> {
  validator<TNext>(fn: (input: unknown) => TNext): ServerFnBuilder<TNext>;
  handler<TOutput>(fn: (context: ServerFnContext<TInput>) => Promise<TOutput> | TOutput): ServerFn<TInput, TOutput>;
}

type AnyHandler = (context: ServerFnContext<unknown>) => unknown;
const serverFns = new Map<string, { readonly method: ServerFnMethod; readonly validate: (input: unknown) => unknown; readonly handler: AnyHandler }>();

function buildServerFn<TInput>(
  options: ServerFnOptions,
  validate: (input: unknown) => TInput,
): ServerFnBuilder<TInput> {
  const method = options.method ?? "POST";
  return {
    validator: <TNext>(fn: (input: unknown) => TNext) => buildServerFn<TNext>(options, fn),
    handler: <TOutput>(fn: (context: ServerFnContext<TInput>) => Promise<TOutput> | TOutput) => {
      serverFns.set(options.id, { method, validate, handler: fn as AnyHandler });
      const serverFn = async (call?: { readonly data?: TInput }): Promise<TOutput> => {
        const data = validate(call?.data);
        if (isServer()) {
          return fn({ data, method, id: options.id });
        }
        return (await callServer(options.id, [data])) as TOutput;
      };
      return Object.assign(serverFn, { id: options.id, method }) as ServerFn<TInput, TOutput>;
    },
  };
}

/** Creates a server function builder (`createServerFn({ id, method }).validator(fn).handler(fn)`). */
export function createServerFn<TInput = undefined>(options: ServerFnOptions): ServerFnBuilder<TInput> {
  return buildServerFn<TInput>(options, (input) => input as TInput);
}

/** Invokes a registered server function by id (used by the Server Actions endpoint). */
export async function invokeServerFn(id: string, data: unknown): Promise<unknown> {
  const entry = serverFns.get(id);
  if (!entry) {
    throw new Error(`Unknown server function "${id}"`);
  }
  return entry.handler({ data: entry.validate(data), method: entry.method, id });
}

/** Stable id for a server function from its file and export name (`serverReferenceId`). */
export function serverFnId(filename: string, name: string): string {
  return serverReferenceId(filename, name);
}

// ---------------------------------------------------------------------------
// Page loaders
// ---------------------------------------------------------------------------

/** Route params / search params record (Next.js 16 shape). */
export type ParamsRecord = Record<string, string | string[]>;

/** Props passed to an App Router page (async `params` and `searchParams`). */
export interface PageLoaderProps {
  readonly params: Promise<ParamsRecord>;
  readonly searchParams: Promise<ParamsRecord>;
}

/** Minimal shape of an App Router page module. */
export interface PageModuleLike {
  readonly default: (props: PageLoaderProps) => unknown;
  readonly generateMetadata?: (props: PageLoaderProps, parent?: Promise<unknown>) => unknown;
}

/** Loader input (subset of the TanStack `loader` context). */
export interface PageLoaderContext {
  readonly params?: Readonly<Record<string, string | string[] | undefined>>;
  readonly location?: {
    readonly search?: Readonly<Record<string, unknown>> | string;
    readonly searchStr?: string;
    readonly href?: string;
  };
}

/** Result of a page loader. */
export interface PageLoaderResult {
  readonly props: PageLoaderProps;
  readonly params: ParamsRecord;
  readonly searchParams: ParamsRecord;
  readonly metadata?: unknown;
}

function searchParamsFromContext(context: PageLoaderContext): ParamsRecord {
  const out: ParamsRecord = {};
  const location = context.location;
  const serialized =
    location?.searchStr ??
    (typeof location?.search === "string" ? location.search : undefined) ??
    (location?.href ? new URL(location.href, "http://yolo.local").search : undefined);
  if (typeof serialized === "string") {
    for (const [key, value] of new URLSearchParams(serialized)) {
      const existing = out[key];
      if (existing === undefined) {
        out[key] = value;
      } else {
        out[key] = Array.isArray(existing) ? [...existing, value] : [existing, value];
      }
    }
    return out;
  }
  if (location?.search && typeof location.search === "object") {
    for (const [key, value] of Object.entries(location.search)) {
      if (Array.isArray(value)) {
        out[key] = value.map(String);
      } else if (value !== undefined && value !== null) {
        out[key] = typeof value === "string" ? value : JSON.stringify(value);
      }
    }
  }
  return out;
}

/** Adapts an async server component module into a TanStack `loader` resolving the props the page expects. */
export function loaderFromPage(pageModule: PageModuleLike): (context: PageLoaderContext) => Promise<PageLoaderResult> {
  return async (context) => {
    const params: ParamsRecord = {};
    for (const [key, value] of Object.entries(context.params ?? {})) {
      if (value !== undefined) {
        params[key] = value;
      }
    }
    const searchParams = searchParamsFromContext(context);
    const props: PageLoaderProps = {
      params: Promise.resolve(params),
      searchParams: Promise.resolve(searchParams),
    };
    const metadata = pageModule.generateMetadata ? await pageModule.generateMetadata(props) : undefined;
    return metadata === undefined ? { props, params, searchParams } : { props, params, searchParams, metadata };
  };
}
