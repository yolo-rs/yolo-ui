// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/navigation: next/navigation compatible control-flow errors, router context and client hooks.

import {
  createContext,
  createElement,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactElement,
  type ReactNode,
} from "react";
import { peekRequestStore } from "./headers.ts";

// ---------------------------------------------------------------------------
// Control-flow errors (digest formats identical to Next.js 16)
// ---------------------------------------------------------------------------

/** Redirect kinds accepted by `redirect()` (Next.js `RedirectType`). */
export const RedirectType = { push: "push", replace: "replace" } as const;
/** Union of the `RedirectType` values. */
export type RedirectType = (typeof RedirectType)[keyof typeof RedirectType];
/** Error code carried by redirect control-flow errors. */
export const REDIRECT_ERROR_CODE = "NEXT_REDIRECT";
/** Error code carried by 401/403/404 control-flow errors. */
export const HTTP_ERROR_FALLBACK_ERROR_CODE = "NEXT_HTTP_ERROR_FALLBACK";
/** HTTP statuses of redirect errors. */
export type RedirectStatusCode = 307 | 308;
/** HTTP statuses of access fallback errors. */
export type HTTPAccessFallbackStatus = 401 | 403 | 404;

/** Error thrown by `redirect()` and `permanentRedirect()`. */
export interface RedirectError extends Error {
  readonly digest: `${typeof REDIRECT_ERROR_CODE};${RedirectType};${string};${RedirectStatusCode};`;
}

/** Error thrown by `notFound()`, `forbidden()` and `unauthorized()`. */
export interface HTTPAccessFallbackError extends Error {
  readonly digest: `${typeof HTTP_ERROR_FALLBACK_ERROR_CODE};${HTTPAccessFallbackStatus}`;
}

const INTERNAL_DIGESTS: ReadonlySet<string> = new Set([
  "DYNAMIC_SERVER_USAGE",
  "BAILOUT_TO_CLIENT_SIDE_RENDERING",
  "NEXT_PRERENDER_INTERRUPTED",
  "NEXT_STATIC_GEN_BAILOUT",
  "next-request-in-use-cache",
]);

function inActionPhase(): boolean {
  try {
    return peekRequestStore()?.phase === "action";
  } catch {
    return false;
  }
}

function digestOf(error: unknown): string | undefined {
  if (typeof error !== "object" || error === null) {
    return undefined;
  }
  const digest = (error as { digest?: unknown }).digest;
  return typeof digest === "string" ? digest : undefined;
}

function createRedirectError(url: string, type: RedirectType, status: RedirectStatusCode): RedirectError {
  const error = new Error(REDIRECT_ERROR_CODE) as Error & { digest: string };
  error.digest = `${REDIRECT_ERROR_CODE};${type};${url};${status};`;
  return error as unknown as RedirectError;
}

function createAccessFallbackError(status: HTTPAccessFallbackStatus): HTTPAccessFallbackError {
  const error = new Error(HTTP_ERROR_FALLBACK_ERROR_CODE) as Error & { digest: string };
  error.digest = `${HTTP_ERROR_FALLBACK_ERROR_CODE};${status}`;
  return error as unknown as HTTPAccessFallbackError;
}

/** Throws a 307 redirect to `url` (`replace` while rendering, `push` inside Server Actions). */
export function redirect(url: string, type?: RedirectType): never {
  throw createRedirectError(url, type ?? (inActionPhase() ? RedirectType.push : RedirectType.replace), 307);
}

/** Throws a 308 permanent redirect to `url`. */
export function permanentRedirect(url: string, type?: RedirectType): never {
  throw createRedirectError(url, type ?? (inActionPhase() ? RedirectType.push : RedirectType.replace), 308);
}

/** Throws the 404 control-flow error rendered by the nearest `not-found` boundary. */
export function notFound(): never {
  throw createAccessFallbackError(404);
}

/** Throws the 403 control-flow error rendered by the nearest `forbidden` boundary. */
export function forbidden(): never {
  throw createAccessFallbackError(403);
}

/** Throws the 401 control-flow error rendered by the nearest `unauthorized` boundary. */
export function unauthorized(): never {
  throw createAccessFallbackError(401);
}

function redirectParts(digest: string): { type: string; destination: string; status: number } | undefined {
  const parts = digest.split(";");
  if (parts.length < 5 || parts[0] !== REDIRECT_ERROR_CODE) {
    return undefined;
  }
  return {
    type: parts[1] ?? "",
    destination: parts.slice(2, -2).join(";"),
    status: Number(parts[parts.length - 2]),
  };
}

/** True when `error` is a redirect control-flow error. */
export function isRedirectError(error: unknown): error is RedirectError {
  const digest = digestOf(error);
  if (digest === undefined) {
    return false;
  }
  const parts = redirectParts(digest);
  return (
    parts !== undefined &&
    (parts.type === "push" || parts.type === "replace") &&
    parts.destination.length > 0 &&
    (parts.status === 307 || parts.status === 308)
  );
}

/** Destination URL of a redirect error. */
export function getRedirectUrl(error: RedirectError): string {
  return redirectParts(error.digest)?.destination ?? "";
}

/** HTTP status (307 or 308) of a redirect error. */
export function getRedirectStatus(error: RedirectError): RedirectStatusCode {
  return redirectParts(error.digest)?.status === 308 ? 308 : 307;
}

/** Navigation kind (`push` or `replace`) of a redirect error. */
export function getRedirectType(error: RedirectError): RedirectType {
  return redirectParts(error.digest)?.type === "push" ? RedirectType.push : RedirectType.replace;
}

/** True when `error` is a 401/403/404 control-flow error. */
export function isHttpAccessFallbackError(error: unknown): error is HTTPAccessFallbackError {
  const digest = digestOf(error);
  if (digest === undefined) {
    return false;
  }
  const [code, status] = digest.split(";");
  return code === HTTP_ERROR_FALLBACK_ERROR_CODE && (status === "401" || status === "403" || status === "404");
}

/** HTTP status of an access fallback error. */
export function getAccessFallbackStatus(error: HTTPAccessFallbackError): HTTPAccessFallbackStatus {
  const status = Number(error.digest.split(";")[1]);
  return status === 401 || status === 403 ? status : 404;
}

/** True for any Next.js router control-flow error (redirect or access fallback). */
export function isNextRouterError(error: unknown): error is RedirectError | HTTPAccessFallbackError {
  return isRedirectError(error) || isHttpAccessFallbackError(error);
}

function isInternalFrameworkError(error: unknown): boolean {
  const digest = digestOf(error);
  if (digest !== undefined && INTERNAL_DIGESTS.has(digest)) {
    return true;
  }
  return (
    typeof error === "object" &&
    error !== null &&
    (error as { $$typeof?: unknown }).$$typeof === Symbol.for("react.postpone")
  );
}

/** Rethrows Next.js control-flow errors swallowed by a `try/catch`; returns for ordinary errors. */
export function unstable_rethrow(error: unknown): void {
  if (isNextRouterError(error) || isInternalFrameworkError(error)) {
    throw error;
  }
  if (error instanceof Error && error.cause !== undefined && error.cause !== error) {
    unstable_rethrow(error.cause);
  }
}

/** True when `error` is the "unrecognized Server Action" error raised for a stale client. */
export function unstable_isUnrecognizedActionError(error: unknown): boolean {
  return digestOf(error) === "UNRECOGNIZED_ACTION";
}

const READONLY_SEARCH_PARAMS_MESSAGE =
  "Method unavailable on `ReadonlyURLSearchParams`. Read more: https://nextjs.org/docs/app/api-reference/functions/use-search-params#updating-searchparams";

/** `URLSearchParams` whose mutating methods throw (returned by `useSearchParams()`). */
export class ReadonlyURLSearchParams extends URLSearchParams {
  /** Unavailable on read-only params: throws. */
  override append(): never {
    throw new Error(READONLY_SEARCH_PARAMS_MESSAGE);
  }

  /** Unavailable on read-only params: throws. */
  override delete(): never {
    throw new Error(READONLY_SEARCH_PARAMS_MESSAGE);
  }

  /** Unavailable on read-only params: throws. */
  override set(): never {
    throw new Error(READONLY_SEARCH_PARAMS_MESSAGE);
  }

  /** Unavailable on read-only params: throws. */
  override sort(): never {
    throw new Error(READONLY_SEARCH_PARAMS_MESSAGE);
  }
}

// ---------------------------------------------------------------------------
// Router context
// ---------------------------------------------------------------------------

/** Options of a client-side navigation. */
export interface YoloNavigateOptions {
  readonly replace?: boolean;
  readonly scroll?: boolean;
}

/** Dynamic route params exposed by `useParams()`. */
export type RouteParams = Readonly<Record<string, string | ReadonlyArray<string>>>;

/** Active layout segments: a flat list (the `children` slot) or a record keyed by parallel route. */
export type LayoutSegments = ReadonlyArray<string> | Readonly<Record<string, ReadonlyArray<string>>>;

/** Location tracked by the router context. */
export interface YoloRouterLocation {
  readonly pathname: string;
  readonly search: string;
  readonly hash: string;
  readonly params: RouteParams;
  readonly segments: LayoutSegments;
}

/** Pluggable navigation backend (RSC runtime, TanStack Router, or the HTML swap baseline). */
export interface YoloNavigator {
  navigate(href: string, options?: YoloNavigateOptions): Promise<void> | void;
  prefetch?(href: string): Promise<void> | void;
  refresh?(): Promise<void> | void;
}

/** Value carried by `YoloRouterContext`. */
export interface YoloRouterContextValue extends YoloRouterLocation {
  navigate(href: string, options?: YoloNavigateOptions): Promise<void>;
  refresh(): Promise<void>;
  prefetch(href: string): void;
  back(): void;
  forward(): void;
}

/** React context carrying the current location and the navigation functions. */
export const YoloRouterContext = createContext<YoloRouterContextValue | null>(null);
/** Context carrying the pending state of the nearest `Link` navigation. */
export const LinkStatusContext = createContext<{ readonly pending: boolean }>({ pending: false });
/** Depth of the current layout relative to the root (`useSelectedLayoutSegments` is relative to it). */
export const LayoutDepthContext = createContext(0);
/** Context used by `useServerInsertedHTML` to register HTML inserted during SSR. */
export const ServerInsertedHTMLContext = createContext<((callback: () => ReactNode) => void) | null>(null);

/** Name of the `CustomEvent` that asks the mounted provider to navigate. */
export const NAVIGATE_EVENT = "yolo:navigate";
/** Name of the `CustomEvent` dispatched after a baseline HTML navigation completed. */
export const NAVIGATED_EVENT = "yolo:navigated";
const TANSTACK_ROUTER_GLOBAL = "__TSR_ROUTER__";
const DUMMY_ORIGIN = "http://yolo.local";

function isBrowser(): boolean {
  return typeof window !== "undefined" && typeof document !== "undefined";
}

interface TanStackRouterLike {
  navigate(options: { to: string; replace?: boolean; resetScroll?: boolean }): Promise<unknown> | unknown;
  preloadRoute?(options: { to: string }): Promise<unknown>;
  invalidate?(): Promise<unknown> | unknown;
  history?: { back(): void; forward(): void };
}

function registeredTanStackRouter(): TanStackRouterLike | undefined {
  const candidate = (globalThis as Record<string, unknown>)[TANSTACK_ROUTER_GLOBAL];
  if (typeof candidate === "object" && candidate !== null) {
    const router = candidate as Partial<TanStackRouterLike>;
    if (typeof router.navigate === "function") {
      return router as TanStackRouterLike;
    }
  }
  return undefined;
}

/** Splits a pathname into decoded segments (`/blog/hello` -> `["blog", "hello"]`). */
export function segmentsFromPathname(pathname: string): string[] {
  return pathname
    .split("/")
    .filter((part) => part.length > 0)
    .map((part) => {
      try {
        return decodeURIComponent(part);
      } catch {
        return part;
      }
    });
}

/** Parses `href` into a router location (params and segments are supplied by the caller). */
export function parseLocation(href: string, params: RouteParams = {}, segments?: LayoutSegments): YoloRouterLocation {
  const url = new URL(href, DUMMY_ORIGIN);
  return {
    pathname: url.pathname,
    search: url.search,
    hash: url.hash,
    params,
    segments: segments ?? segmentsFromPathname(url.pathname),
  };
}

function windowHref(): string {
  return isBrowser() ? `${window.location.pathname}${window.location.search}${window.location.hash}` : "/";
}

const prefetched = new Set<string>();

function connectionAllowsPrefetch(): boolean {
  if (!isBrowser()) {
    return false;
  }
  const connection = (navigator as Navigator & { connection?: { saveData?: boolean; effectiveType?: string } })
    .connection;
  if (connection?.saveData === true) {
    return false;
  }
  return !/2g/.test(connection?.effectiveType ?? "");
}

function reviveScripts(root: HTMLElement): void {
  for (const script of Array.from(root.querySelectorAll("script"))) {
    const fresh = document.createElement("script");
    for (const attribute of Array.from(script.attributes)) {
      fresh.setAttribute(attribute.name, attribute.value);
    }
    fresh.textContent = script.textContent;
    script.replaceWith(fresh);
  }
}

async function htmlNavigate(href: string, options: YoloNavigateOptions): Promise<boolean> {
  const url = new URL(href, window.location.href);
  if (url.origin !== window.location.origin) {
    window.location.assign(url.href);
    return false;
  }
  let html: string;
  try {
    const response = await fetch(url.href, { headers: { accept: "text/html" }, credentials: "same-origin" });
    if (new URL(response.url, window.location.href).origin !== window.location.origin) {
      window.location.assign(response.url);
      return false;
    }
    if (!response.ok && response.status !== 404) {
      throw new Error(`navigation failed with status ${response.status}`);
    }
    html = await response.text();
  } catch {
    window.location.assign(url.href);
    return false;
  }
  const next = new DOMParser().parseFromString(html, "text/html");
  const swap = (): void => {
    document.title = next.title;
    document.body.innerHTML = next.body.innerHTML;
    reviveScripts(document.body);
  };
  const withTransition = (
    document as Document & { startViewTransition?: (callback: () => void) => { finished: Promise<void> } }
  ).startViewTransition;
  if (typeof withTransition === "function") {
    await withTransition.call(document, swap).finished.catch(() => undefined);
  } else {
    swap();
  }
  if (options.replace === true) {
    window.history.replaceState({ yolo: true }, "", url.href);
  } else {
    window.history.pushState({ yolo: true }, "", url.href);
  }
  if (options.scroll !== false) {
    const target = url.hash ? document.getElementById(decodeURIComponent(url.hash.slice(1))) : null;
    if (target) {
      target.scrollIntoView();
    } else {
      window.scrollTo(0, 0);
    }
  }
  window.dispatchEvent(new CustomEvent(NAVIGATED_EVENT, { detail: { href: url.href } }));
  return true;
}

function baselinePrefetch(href: string): void {
  if (!connectionAllowsPrefetch()) {
    return;
  }
  const url = new URL(href, window.location.href);
  if (url.origin !== window.location.origin || prefetched.has(url.href)) {
    return;
  }
  prefetched.add(url.href);
  const init: RequestInit = {
    headers: { accept: "text/html", purpose: "prefetch" },
    credentials: "same-origin",
    priority: "low",
  };
  void fetch(url.href, init).catch(() => prefetched.delete(url.href));
}

/** Asks the mounted `YoloRouterProvider` to navigate (no-op on the server). */
export function dispatchNavigate(href: string, options: YoloNavigateOptions = {}): void {
  if (isBrowser()) {
    window.dispatchEvent(new CustomEvent(NAVIGATE_EVENT, { detail: { href, ...options } }));
  }
}

/** Props of `YoloRouterProvider`. */
export interface YoloRouterProviderProps {
  readonly initialUrl?: string;
  readonly params?: RouteParams;
  readonly segments?: LayoutSegments;
  readonly navigator?: YoloNavigator;
  readonly children?: ReactNode;
}

/** Provides the router context; on the client it performs soft navigations and listens to `popstate` and `yolo:navigate`. */
export function YoloRouterProvider(props: YoloRouterProviderProps): ReactElement {
  const { initialUrl, params, segments, navigator: customNavigator, children } = props;
  const [location, setLocation] = useState<YoloRouterLocation>(() =>
    parseLocation(initialUrl ?? windowHref(), params ?? {}, segments),
  );
  const navigatorRef = useRef<YoloNavigator | undefined>(customNavigator);
  const paramsRef = useRef<RouteParams>(params ?? {});
  useEffect(() => {
    navigatorRef.current = customNavigator;
    paramsRef.current = params ?? {};
  }, [customNavigator, params]);

  const navigate = useCallback(async (href: string, options: YoloNavigateOptions = {}): Promise<void> => {
    if (!isBrowser()) {
      return;
    }
    const custom = navigatorRef.current;
    if (custom) {
      await custom.navigate(href, options);
      setLocation(parseLocation(href, paramsRef.current));
      return;
    }
    const tanstack = registeredTanStackRouter();
    if (tanstack) {
      await tanstack.navigate({ to: href, replace: options.replace === true, resetScroll: options.scroll !== false });
      setLocation(parseLocation(href, paramsRef.current));
      return;
    }
    if (await htmlNavigate(href, options)) {
      setLocation(parseLocation(href, paramsRef.current));
    }
  }, []);

  const refresh = useCallback(async (): Promise<void> => {
    if (!isBrowser()) {
      return;
    }
    const custom = navigatorRef.current;
    if (custom?.refresh) {
      await custom.refresh();
      return;
    }
    const tanstack = registeredTanStackRouter();
    if (tanstack?.invalidate) {
      await tanstack.invalidate();
      return;
    }
    await htmlNavigate(windowHref(), { replace: true, scroll: false });
  }, []);

  const prefetch = useCallback((href: string): void => {
    if (!isBrowser()) {
      return;
    }
    const custom = navigatorRef.current;
    if (custom?.prefetch) {
      void custom.prefetch(href);
      return;
    }
    const tanstack = registeredTanStackRouter();
    if (tanstack?.preloadRoute) {
      void tanstack.preloadRoute({ to: href }).catch(() => undefined);
      return;
    }
    baselinePrefetch(href);
  }, []);

  const back = useCallback((): void => {
    if (isBrowser()) {
      window.history.back();
    }
  }, []);
  const forward = useCallback((): void => {
    if (isBrowser()) {
      window.history.forward();
    }
  }, []);

  useEffect(() => {
    if (!isBrowser()) {
      return undefined;
    }
    const onPopState = (): void => {
      setLocation(parseLocation(windowHref(), paramsRef.current));
      if (!navigatorRef.current && !registeredTanStackRouter()) {
        void htmlNavigate(windowHref(), { replace: true, scroll: false });
      }
    };
    const onNavigateEvent = (event: Event): void => {
      const detail = (event as CustomEvent<{ href?: string } & YoloNavigateOptions>).detail;
      if (detail && typeof detail.href === "string") {
        void navigate(detail.href, { replace: detail.replace, scroll: detail.scroll });
      }
    };
    window.addEventListener("popstate", onPopState);
    window.addEventListener(NAVIGATE_EVENT, onNavigateEvent);
    return () => {
      window.removeEventListener("popstate", onPopState);
      window.removeEventListener(NAVIGATE_EVENT, onNavigateEvent);
    };
  }, [navigate]);

  const value = useMemo<YoloRouterContextValue>(
    () => ({ ...location, navigate, refresh, prefetch, back, forward }),
    [location, navigate, refresh, prefetch, back, forward],
  );
  return createElement(YoloRouterContext.Provider, { value }, children);
}

function detachedRouter(): YoloRouterContextValue {
  const location = parseLocation(windowHref());
  return {
    ...location,
    navigate: async (href, options = {}) => {
      if (isBrowser()) {
        if (options.replace === true) {
          window.location.replace(href);
        } else {
          window.location.assign(href);
        }
      }
    },
    refresh: async () => {
      if (isBrowser()) {
        window.location.reload();
      }
    },
    prefetch: (href) => {
      if (isBrowser()) {
        baselinePrefetch(href);
      }
    },
    back: () => {
      if (isBrowser()) {
        window.history.back();
      }
    },
    forward: () => {
      if (isBrowser()) {
        window.history.forward();
      }
    },
  };
}

/** Router context value, falling back to a `window.location` based router outside a provider. */
export function useYoloRouter(): YoloRouterContextValue {
  const context = useContext(YoloRouterContext);
  return context ?? detachedRouter();
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

/** Options accepted by `router.push` and `router.replace`. */
export interface NavigateOptions {
  readonly scroll?: boolean;
}

/** Options accepted by `router.prefetch`. */
export interface PrefetchOptions {
  readonly kind?: "auto" | "full" | "temporary";
  readonly onInvalidate?: () => void;
}

/** App Router instance returned by `useRouter()`. */
export interface AppRouterInstance {
  push(href: string, options?: NavigateOptions): void;
  replace(href: string, options?: NavigateOptions): void;
  refresh(): void;
  prefetch(href: string, options?: PrefetchOptions): void;
  back(): void;
  forward(): void;
  readonly bfcacheId: string;
}

/** `next/navigation` `useRouter()`: push, replace, refresh, prefetch, back and forward. */
export function useRouter(): AppRouterInstance {
  const router = useYoloRouter();
  const { navigate, refresh, prefetch, back, forward } = router;
  return useMemo<AppRouterInstance>(
    () => ({
      push: (href, options) => {
        void navigate(href, { replace: false, scroll: options?.scroll });
      },
      replace: (href, options) => {
        void navigate(href, { replace: true, scroll: options?.scroll });
      },
      refresh: () => {
        void refresh();
      },
      prefetch: (href) => {
        prefetch(href);
      },
      back,
      forward,
      bfcacheId: "",
    }),
    [navigate, refresh, prefetch, back, forward],
  );
}

/** Current pathname (`/blog/hello`). */
export function usePathname(): string {
  return useYoloRouter().pathname;
}

/** Current query string as a read-only `URLSearchParams`. */
export function useSearchParams(): ReadonlyURLSearchParams {
  const { search } = useYoloRouter();
  return useMemo(() => new ReadonlyURLSearchParams(search), [search]);
}

/** Dynamic route params of the current route. */
export function useParams<T extends RouteParams = RouteParams>(): T {
  return useYoloRouter().params as T;
}

function segmentsForSlot(segments: LayoutSegments, parallelRouteKey: string): ReadonlyArray<string> {
  if (Array.isArray(segments)) {
    return parallelRouteKey === "children" ? (segments as ReadonlyArray<string>) : [];
  }
  return (segments as Readonly<Record<string, ReadonlyArray<string>>>)[parallelRouteKey] ?? [];
}

/** Active child segments below the current layout for `parallelRouteKey`. */
export function useSelectedLayoutSegments(parallelRouteKey = "children"): string[] {
  const { segments } = useYoloRouter();
  const depth = useContext(LayoutDepthContext);
  return segmentsForSlot(segments, parallelRouteKey)
    .slice(depth)
    .filter((segment) => segment !== "__PAGE__" && segment !== "__DEFAULT__");
}

/** First active child segment below the current layout, or `null`. */
export function useSelectedLayoutSegment(parallelRouteKey = "children"): string | null {
  const segments = useSelectedLayoutSegments(parallelRouteKey);
  return segments[0] ?? null;
}

/** Wraps a layout's children so `useSelectedLayoutSegments` becomes relative to that layout. */
export function YoloLayoutBoundary(props: { readonly children?: ReactNode }): ReactElement {
  const depth = useContext(LayoutDepthContext);
  return createElement(LayoutDepthContext.Provider, { value: depth + 1 }, props.children);
}

/** Pending state of the nearest `Link` navigation. */
export function useLinkStatus(): { readonly pending: boolean } {
  return useContext(LinkStatusContext);
}

/** Registers HTML (styles) to insert into the SSR stream; no-op without a collector. */
export function useServerInsertedHTML(callback: () => ReactNode): void {
  const insert = useContext(ServerInsertedHTMLContext);
  if (insert) {
    insert(callback);
  }
}

/** Web Vitals metric reported by `useReportWebVitals` (web-vitals `Metric` shape). */
export interface WebVitalsMetric {
  readonly id: string;
  readonly name: "LCP" | "CLS" | "INP" | "FCP" | "TTFB";
  readonly value: number;
  readonly rating: "good" | "needs-improvement" | "poor";
  readonly delta: number;
  readonly entries: ReadonlyArray<PerformanceEntry>;
  readonly navigationType: string;
}

const VITALS_THRESHOLDS: Readonly<Record<WebVitalsMetric["name"], readonly [number, number]>> = {
  LCP: [2500, 4000],
  CLS: [0.1, 0.25],
  INP: [200, 500],
  FCP: [1800, 3000],
  TTFB: [800, 1800],
};

/** Rates a Web Vital value against the standard good / poor thresholds. */
export function rateWebVital(name: WebVitalsMetric["name"], value: number): WebVitalsMetric["rating"] {
  const [good, poor] = VITALS_THRESHOLDS[name];
  if (value <= good) {
    return "good";
  }
  return value <= poor ? "needs-improvement" : "poor";
}

/** Reports LCP, CLS, INP, FCP and TTFB through `PerformanceObserver` on the client; no-op on the server. */
export function useReportWebVitals(reportWebVitalsFn: (metric: WebVitalsMetric) => void): void {
  const callbackRef = useRef(reportWebVitalsFn);
  useEffect(() => {
    callbackRef.current = reportWebVitalsFn;
  }, [reportWebVitalsFn]);
  useEffect(() => {
    if (!isBrowser() || typeof PerformanceObserver === "undefined") {
      return undefined;
    }
    const navigationEntry = performance.getEntriesByType("navigation")[0] as PerformanceNavigationTiming | undefined;
    const navigationType = navigationEntry?.type ?? "navigate";
    const previous: Partial<Record<WebVitalsMetric["name"], number>> = {};
    const report = (name: WebVitalsMetric["name"], value: number, entries: PerformanceEntry[]): void => {
      const last = previous[name] ?? 0;
      previous[name] = value;
      callbackRef.current({
        id: `v4-${name}-${Math.floor(performance.timeOrigin)}`,
        name,
        value,
        rating: rateWebVital(name, value),
        delta: value - last,
        entries,
        navigationType,
      });
    };
    if (navigationEntry) {
      report("TTFB", navigationEntry.responseStart, [navigationEntry]);
    }
    const observers: PerformanceObserver[] = [];
    const observe = (
      type: string,
      options: Record<string, unknown>,
      onEntries: (entries: PerformanceEntry[]) => void,
    ): void => {
      try {
        const observer = new PerformanceObserver((list) => onEntries(list.getEntries()));
        observer.observe({ type, buffered: true, ...options } as PerformanceObserverInit);
        observers.push(observer);
      } catch {
        // Entry type unsupported by this browser.
      }
    };
    observe("paint", {}, (entries) => {
      const fcp = entries.find((entry) => entry.name === "first-contentful-paint");
      if (fcp) {
        report("FCP", fcp.startTime, [fcp]);
      }
    });
    observe("largest-contentful-paint", {}, (entries) => {
      const last = entries[entries.length - 1];
      if (last) {
        report("LCP", last.startTime, [last]);
      }
    });
    let cls = 0;
    const clsEntries: PerformanceEntry[] = [];
    observe("layout-shift", {}, (entries) => {
      for (const entry of entries) {
        const shift = entry as PerformanceEntry & { hadRecentInput?: boolean; value?: number };
        if (shift.hadRecentInput !== true) {
          cls += shift.value ?? 0;
          clsEntries.push(entry);
        }
      }
      report("CLS", cls, clsEntries);
    });
    let inp = 0;
    observe("event", { durationThreshold: 40 }, (entries) => {
      for (const entry of entries) {
        if (entry.duration > inp) {
          inp = entry.duration;
          report("INP", inp, [entry]);
        }
      }
    });
    return () => {
      for (const observer of observers) {
        observer.disconnect();
      }
    };
  }, []);
}
