// SPDX-License-Identifier: MIT OR Apache-2.0
// Route handlers of an `app/` scan (`route.ts` modules exporting GET/POST/...) as `Bun.serve({ routes })`
// entries, and a SPA host that serves them next to an HTML bundle for the TanStack router.
import type { BunPlugin, BunRequest, HTMLBundle, Server } from "bun";
import type { AppRoute, AppTree, Params } from "../app/scan";

export const HTTP_METHODS = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"] as const;
export type HttpMethod = (typeof HTTP_METHODS)[number];

/** Next-style handler context: `params` is a promise, as in Next 15. */
export interface RouteContext {
  params: Promise<Params>;
}

export type RouteHandler = (request: Request, context: RouteContext) => Response | Promise<Response>;

/** A loaded `route.ts` module. */
export type RouteModule = Partial<Record<HttpMethod, RouteHandler>> & Record<string, unknown>;

export type ModuleLoader = (file: string) => Promise<Record<string, unknown>>;

export interface BunRoutesOptions {
  /** URL prefix of every entry (`/api`); the app patterns are appended to it. */
  prefix?: string;
}

/** One Bun.serve route entry per pattern. */
export type BunRouteHandler = (request: BunRequest) => Promise<Response>;

const normalizePrefix = (prefix: string | undefined): string => {
  if (!prefix) return "";
  const trimmed = prefix.replace(/\/+$/, "");
  return trimmed ? (trimmed.startsWith("/") ? trimmed : `/${trimmed}`) : "";
};

function bunSegment(part: AppRoute["parts"][number]): string {
  switch (part.kind) {
    case "static":
      return part.value;
    case "single":
      return `:${part.value}`;
    default:
      return "*";
  }
}

/**
 * Every Bun pattern of `route`: `[id]` becomes `:id`, `[...slug]` becomes `/*`, and `[[...slug]]`
 * yields two patterns (with and without the trailing `/*`), the wildcard one first.
 */
export function bunPatternsFromRoute(route: AppRoute, options: BunRoutesOptions = {}): string[] {
  const prefix = normalizePrefix(options.prefix);
  const parts = route.parts.map(bunSegment);
  const main = `${prefix}/${parts.join("/")}`.replace(/\/+/g, "/");
  const last = route.parts.at(-1);
  if (last?.kind !== "optional") return [main];
  const bare = `${prefix}/${parts.slice(0, -1).join("/")}`.replace(/\/+/g, "/");
  return [main, bare];
}

/** The primary Bun pattern of `route` (`/docs/*` for every catch-all). */
export function bunPatternFromRoute(route: AppRoute, options: BunRoutesOptions = {}): string {
  return bunPatternsFromRoute(route, options)[0]!;
}

const decode = (segment: string): string => {
  try {
    return decodeURIComponent(segment);
  } catch {
    return segment;
  }
};

/** Next-style params: `req.params` plus the catch-all segments (Bun exposes no wildcard param). */
function routeParams(route: AppRoute, request: BunRequest, prefixDepth: number): Params {
  const params: Params = {};
  const raw = (request.params ?? {}) as Record<string, string>;
  for (const part of route.parts) {
    if (part.kind === "single" && raw[part.value] !== undefined) params[part.value] = raw[part.value]!;
  }
  const splat = route.parts.findIndex(part => part.kind === "catchall" || part.kind === "optional");
  if (splat >= 0) {
    const segments = new URL(request.url).pathname.split("/").filter(Boolean).map(decode);
    const rest = segments.slice(prefixDepth + splat);
    if (rest.length) params[route.parts[splat]!.value] = rest;
  }
  return params;
}

/**
 * The handler of one route module: dispatches on the request method, `HEAD` falls back to `GET`
 * (body dropped), and any method the module does not export answers 405 with an `Allow` header.
 * A single function (rather than a `{ GET, POST }` object) because Bun sends unlisted methods to
 * `fetch`, which in a SPA host is the HTML page, and because the module loads lazily on first use.
 */
export function bunRouteHandler(route: AppRoute, load: ModuleLoader, options: BunRoutesOptions = {}): BunRouteHandler {
  const prefixDepth = normalizePrefix(options.prefix).split("/").filter(Boolean).length;
  let module: Promise<RouteModule> | undefined;
  return async request => {
    module ??= load(route.file) as Promise<RouteModule>;
    let mod: RouteModule;
    try {
      mod = await module;
    } catch (error) {
      module = undefined;
      throw error;
    }
    const method = request.method.toUpperCase() as HttpMethod;
    const handler = mod[method] ?? (method === "HEAD" ? mod.GET : undefined);
    if (typeof handler !== "function") {
      const allow = HTTP_METHODS.filter(m => typeof mod[m] === "function" || (m === "HEAD" && mod.GET));
      return new Response("Method Not Allowed", {
        status: 405,
        headers: { allow: allow.join(", ") },
      });
    }
    const params = Promise.resolve(routeParams(route, request, prefixDepth));
    const response = await handler(request, { params });
    if (method === "HEAD" && !mod.HEAD) return new Response(null, response);
    return response;
  };
}

/**
 * `Bun.serve({ routes })` entries for every `kind: "route"` handler of `tree`. `load` imports a
 * project-relative route module (for instance `file => import(join(root, file))`).
 */
export function bunRoutesFromTree(
  tree: AppTree,
  load: ModuleLoader,
  options: BunRoutesOptions = {},
): Record<string, BunRouteHandler> {
  const routes: Record<string, BunRouteHandler> = {};
  for (const route of tree.routes) {
    if (route.kind !== "route") continue;
    const handler = bunRouteHandler(route, load, options);
    for (const pattern of bunPatternsFromRoute(route, options)) routes[pattern] ??= handler;
  }
  return routes;
}

export interface ServeSpaOptions extends BunRoutesOptions {
  /** An HTML import (`import index from "./index.html"`) or a Response served for every page URL. */
  html: HTMLBundle | Response;
  tree: AppTree;
  load: ModuleLoader;
  port?: number;
  hostname?: string;
  development?: boolean | { hmr?: boolean; console?: boolean };
  /** Extra `routes` entries (static assets, health checks); the app routes win on conflicts. */
  routes?: Record<string, unknown>;
  /** Bundler plugins for the HTML import (for example `routeTreePlugin(tree)`), through `bunfig.toml` in dev. */
  plugins?: BunPlugin[];
}

/**
 * Serve the route handlers of `tree` next to a single-page app: every other URL gets `html`, so the
 * TanStack router owns client-side navigation. Bun bundles an `HTMLBundle` on the fly in development.
 */
export function serveSpa(options: ServeSpaOptions): Server<undefined> {
  const { html, tree, load, port, hostname, development, prefix } = options;
  const routes = {
    ...options.routes,
    ...bunRoutesFromTree(tree, load, { prefix }),
    "/*": html,
  };
  return Bun.serve({
    port: port ?? Number(process.env.PORT ?? 3000),
    hostname: hostname ?? process.env.HOSTNAME,
    development,
    routes,
    error(error) {
      console.error(error);
      return new Response("Internal Server Error", { status: 500 });
    },
  });
}
