// SPDX-License-Identifier: Apache-2.0
// next/router shim (Pages Router compatibility) adapted over the App Router context.

import { createElement, useMemo, type ComponentType, type ReactElement } from "react";
import { resolveHref, type UrlObject } from "../link.ts";
import { dispatchNavigate, useYoloRouter } from "../navigation.ts";

/** Router event names of `Router.events`. */
export type RouterEvent =
  | "routeChangeStart"
  | "beforeHistoryChange"
  | "routeChangeComplete"
  | "routeChangeError"
  | "hashChangeStart"
  | "hashChangeComplete";

/** Handler of a router event. */
export type RouterEventHandler = (...args: unknown[]) => void;

/** Events emitter of `next/router` (`Router.events.on("routeChangeStart", fn)`). */
export interface RouterEvents {
  on(event: RouterEvent, handler: RouterEventHandler): void;
  off(event: RouterEvent, handler: RouterEventHandler): void;
  emit(event: RouterEvent, ...args: unknown[]): void;
}

const listeners = new Map<RouterEvent, Set<RouterEventHandler>>();

/** Shared router events emitter. */
export const routerEvents: RouterEvents = {
  on: (event, handler) => {
    const set = listeners.get(event) ?? new Set<RouterEventHandler>();
    set.add(handler);
    listeners.set(event, set);
  },
  off: (event, handler) => {
    listeners.get(event)?.delete(handler);
  },
  emit: (event, ...args) => {
    for (const handler of listeners.get(event) ?? []) {
      handler(...args);
    }
  },
};

/** Transition options of `push` / `replace`. */
export interface TransitionOptions {
  readonly shallow?: boolean;
  readonly locale?: string | false;
  readonly scroll?: boolean;
  readonly unstable_skipClientCache?: boolean;
}

/** Query object of the Pages Router (`router.query`). */
export type ParsedUrlQuery = Record<string, string | string[]>;

/** `next/router` instance shape. */
export interface NextRouter {
  readonly route: string;
  readonly pathname: string;
  readonly query: ParsedUrlQuery;
  readonly asPath: string;
  readonly basePath: string;
  readonly locale?: string;
  readonly locales?: ReadonlyArray<string>;
  readonly defaultLocale?: string;
  readonly isReady: boolean;
  readonly isFallback: boolean;
  readonly isPreview: boolean;
  readonly isLocaleDomain: boolean;
  readonly events: RouterEvents;
  push(url: string | UrlObject, as?: string | UrlObject, options?: TransitionOptions): Promise<boolean>;
  replace(url: string | UrlObject, as?: string | UrlObject, options?: TransitionOptions): Promise<boolean>;
  reload(): void;
  back(): void;
  forward(): void;
  prefetch(url: string, as?: string, options?: { priority?: boolean; locale?: string | false }): Promise<void>;
  beforePopState(callback: (state: unknown) => boolean): void;
}

function queryFrom(search: string, params: Readonly<Record<string, string | ReadonlyArray<string>>>): ParsedUrlQuery {
  const query: ParsedUrlQuery = {};
  for (const [key, value] of new URLSearchParams(search)) {
    const existing = query[key];
    if (existing === undefined) {
      query[key] = value;
    } else {
      query[key] = Array.isArray(existing) ? [...existing, value] : [existing, value];
    }
  }
  for (const [key, value] of Object.entries(params)) {
    query[key] = Array.isArray(value) ? [...(value as ReadonlyArray<string>)] : (value as string);
  }
  return query;
}

/** `next/router` `useRouter()` adapted over the App Router context. */
export function useRouter(): NextRouter {
  const router = useYoloRouter();
  const { pathname, search, hash, params, navigate, prefetch, back, forward, refresh } = router;
  return useMemo<NextRouter>(() => {
    const asPath = `${pathname}${search}${hash}`;
    const go = async (url: string | UrlObject, as: string | UrlObject | undefined, replace: boolean, options?: TransitionOptions) => {
      const href = resolveHref(as ?? url);
      routerEvents.emit("routeChangeStart", href, { shallow: options?.shallow === true });
      try {
        await navigate(href, { replace, scroll: options?.scroll });
        routerEvents.emit("routeChangeComplete", href, { shallow: options?.shallow === true });
        return true;
      } catch (error) {
        routerEvents.emit("routeChangeError", error, href, { shallow: options?.shallow === true });
        return false;
      }
    };
    return {
      route: pathname,
      pathname,
      query: queryFrom(search, params),
      asPath,
      basePath: "",
      isReady: true,
      isFallback: false,
      isPreview: false,
      isLocaleDomain: false,
      events: routerEvents,
      push: (url, as, options) => go(url, as, false, options),
      replace: (url, as, options) => go(url, as, true, options),
      reload: () => {
        void refresh();
      },
      back,
      forward,
      prefetch: async (url) => {
        prefetch(url);
      },
      beforePopState: () => undefined,
    };
  }, [pathname, search, hash, params, navigate, prefetch, back, forward, refresh]);
}

/** Props injected by `withRouter`. */
export interface WithRouterProps {
  readonly router: NextRouter;
}

/** HOC injecting the `router` prop (`withRouter(MyComponent)`). */
export function withRouter<P extends WithRouterProps>(Component: ComponentType<P>): ComponentType<Omit<P, "router">> {
  function WithRouter(props: Omit<P, "router">): ReactElement {
    const router = useRouter();
    return createElement(Component, { ...(props as P), router });
  }
  WithRouter.displayName = `withRouter(${Component.displayName ?? Component.name ?? "Component"})`;
  return WithRouter;
}

function currentHref(): { pathname: string; search: string; hash: string } {
  if (typeof window === "undefined") {
    return { pathname: "/", search: "", hash: "" };
  }
  return { pathname: window.location.pathname, search: window.location.search, hash: window.location.hash };
}

/** Singleton router for imperative use outside components (`Router.push("/x")`). */
const Router = {
  events: routerEvents,
  get pathname(): string {
    return currentHref().pathname;
  },
  get asPath(): string {
    const { pathname, search, hash } = currentHref();
    return `${pathname}${search}${hash}`;
  },
  get query(): ParsedUrlQuery {
    return queryFrom(currentHref().search, {});
  },
  get isReady(): boolean {
    return typeof window !== "undefined";
  },
  async push(url: string | UrlObject, as?: string | UrlObject, options?: TransitionOptions): Promise<boolean> {
    dispatchNavigate(resolveHref(as ?? url), { replace: false, scroll: options?.scroll });
    return true;
  },
  async replace(url: string | UrlObject, as?: string | UrlObject, options?: TransitionOptions): Promise<boolean> {
    dispatchNavigate(resolveHref(as ?? url), { replace: true, scroll: options?.scroll });
    return true;
  },
  reload(): void {
    if (typeof window !== "undefined") {
      window.location.reload();
    }
  },
  back(): void {
    if (typeof window !== "undefined") {
      window.history.back();
    }
  },
  forward(): void {
    if (typeof window !== "undefined") {
      window.history.forward();
    }
  },
  async prefetch(): Promise<void> {
    return undefined;
  },
  beforePopState(): void {
    return undefined;
  },
};

/** `next/router` default export (singleton router). */
export default Router;
