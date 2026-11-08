// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/server: the Bun.serve App Router. Request pipeline in Next.js order (static files,
// headers, redirects, proxy, rewrites, filesystem routes, 404), route handlers, metadata routes,
// Server Actions, Flight/HTML page rendering (RSC worker or inline fallback), response finishing.

import { existsSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { createElement, type ReactElement, type ReactNode } from "react";
import { handleServerAction, isActionRequest, loadActionManifest, registerServerAction, type ActionRegistry } from "./actions.ts";
import { cached, configureCacheLife, pathTags } from "./cache.ts";
import { cacheControlFor, contentTypeFor, serveStaticAsset } from "./cdn.ts";
import { defineYoloConfig, type YoloConfigInput } from "./config.ts";
import { createFontsHandler } from "./fonts.ts";
import {
  createRequestStore,
  mergeStoreHeaders,
  runAfterCallbacks,
  runWithRequestStore,
  type RequestStore,
} from "./headers.ts";
import { createImageOptimizerHandler } from "./image.ts";
import { headElements, renderHeadTags, resolveMetadataChain } from "./metadata.ts";
import {
  getAccessFallbackStatus,
  getRedirectStatus,
  getRedirectUrl,
  isHttpAccessFallbackError,
  isRedirectError,
} from "./navigation.ts";
import { yoloNextPlugin } from "./plugin.ts";
import { matchRoute, scanAppDir } from "./router.ts";
import {
  buildRouteElement,
  loadReactServerDomClient,
  renderFlightToHtml,
  renderToFlight,
  startRscWorker,
  type ClientModuleMap,
  type RouteModule,
  type RscWorker,
} from "./rsc.ts";
import { applyRules, matchRule, requestFacts } from "./rules.ts";
import {
  RSC_HEADERS,
  type Metadata,
  type NextCompatConfig,
  type ResolvedYoloConfig,
  type RouteEntry,
  type RouteManifest,
  type RouteMatch,
  type Viewport,
} from "./types.ts";

/** Options of `createYoloNextApp`. */
export interface CreateYoloNextAppOptions {
  readonly root: string;
  readonly config: ResolvedYoloConfig | NextCompatConfig;
  readonly routes?: RouteManifest;
  readonly dev?: boolean;
  readonly fetchImpl?: typeof fetch;
  /** `auto` (worker when react-server-dom-webpack resolves and `YOLO_RSC_MODE !== "inline"`), `worker` or `inline`. */
  readonly rscMode?: "auto" | "worker" | "inline";
}

/** Route table accepted by `Bun.serve({ routes })`. */
export type AppRoutes = Record<string, (request: Request, server?: Bun.Server<unknown>) => Promise<Response>>;

/** The App Router application (also usable as a `Bun.serve` config object through `export default app`). */
export interface YoloNextApp {
  readonly config: ResolvedYoloConfig;
  readonly manifest: RouteManifest;
  readonly routes: AppRoutes;
  readonly hostname: string;
  readonly development: boolean | { hmr: boolean; console: boolean };
  port?: number;
  fetch(request: Request, server?: Bun.Server<unknown>): Promise<Response>;
  listen(port?: number): Bun.Server<unknown>;
  stop(): Promise<void>;
  /** Registers an in-process Server Action (inline registration for imported modules). */
  registerAction(module: string, exportName: string, fn: (...args: never[]) => unknown): string;
}

const BUFFERED = "x-yolo-buffered";
const COMPRESSIBLE = /^(text\/|application\/(json|javascript|xml|manifest\+json|x-component)|image\/svg\+xml)/;

function isResolved(config: ResolvedYoloConfig | NextCompatConfig): config is ResolvedYoloConfig {
  return "cdn" in config && "server" in config && "bundler" in config;
}

function toPosix(path: string): string {
  return path.replace(/\\/g, "/");
}

function textResponse(body: string, status: number, contentType: string, headers: Record<string, string> = {}): Response {
  return new Response(body, { status, headers: { "content-type": contentType, [BUFFERED]: "1", ...headers } });
}

function escapeHtml(text: string): string {
  return Bun.escapeHTML(text);
}

// ---------------------------------------------------------------------------
// Metadata routes
// ---------------------------------------------------------------------------

interface SitemapItem {
  url: string;
  lastModified?: string | Date;
  changeFrequency?: string;
  priority?: number;
  alternates?: { languages?: Record<string, string> };
}

interface RobotsRule {
  userAgent?: string | string[];
  allow?: string | string[];
  disallow?: string | string[];
  crawlDelay?: number;
}

/** Serializes a `MetadataRoute.Sitemap` array to XML. */
export function sitemapXml(items: ReadonlyArray<SitemapItem>): string {
  const urls = items
    .map((item) => {
      let xml = `<url><loc>${escapeHtml(item.url)}</loc>`;
      if (item.lastModified) {
        const date = item.lastModified instanceof Date ? item.lastModified.toISOString() : String(item.lastModified);
        xml += `<lastmod>${escapeHtml(date)}</lastmod>`;
      }
      if (item.changeFrequency) {
        xml += `<changefreq>${escapeHtml(item.changeFrequency)}</changefreq>`;
      }
      if (item.priority !== undefined) {
        xml += `<priority>${item.priority}</priority>`;
      }
      for (const [lang, href] of Object.entries(item.alternates?.languages ?? {})) {
        xml += `<xhtml:link rel="alternate" hreflang="${escapeHtml(lang)}" href="${escapeHtml(href)}"/>`;
      }
      return `${xml}</url>`;
    })
    .join("");
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">${urls}</urlset>\n`;
}

/** Serializes a `MetadataRoute.Robots` object to `robots.txt`. */
export function robotsTxt(robots: { rules?: RobotsRule | RobotsRule[]; sitemap?: string | string[]; host?: string }): string {
  const rules = Array.isArray(robots.rules) ? robots.rules : robots.rules ? [robots.rules] : [];
  const lines: string[] = [];
  const list = (value: string | string[] | undefined): string[] => (value === undefined ? [] : Array.isArray(value) ? value : [value]);
  for (const rule of rules) {
    const agents = list(rule.userAgent);
    for (const agent of agents.length > 0 ? agents : ["*"]) {
      lines.push(`User-Agent: ${agent}`);
    }
    for (const allow of list(rule.allow)) {
      lines.push(`Allow: ${allow}`);
    }
    for (const disallow of list(rule.disallow)) {
      lines.push(`Disallow: ${disallow}`);
    }
    if (rule.crawlDelay !== undefined) {
      lines.push(`Crawl-delay: ${rule.crawlDelay}`);
    }
    lines.push("");
  }
  if (robots.host) {
    lines.push(`Host: ${robots.host}`, "");
  }
  for (const sitemap of list(robots.sitemap)) {
    lines.push(`Sitemap: ${sitemap}`);
  }
  return `${lines.join("\n").trim()}\n`;
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

interface ProxyModule {
  default?: unknown;
  proxy?: unknown;
  middleware?: unknown;
  config?: { matcher?: string | Array<string | { source: string; has?: unknown[]; missing?: unknown[] }> };
}

type NextRequestCtor = new (input: Request | string | URL, init?: RequestInit) => Request;

interface AppState {
  proxy?: Promise<{ handler?: (request: Request) => unknown; matchers: Array<{ source: string; has?: unknown[]; missing?: unknown[] }> }>;
  registry?: Promise<ActionRegistry>;
  image?: (request: Request) => Promise<Response>;
  fonts?: (request: Request) => Response;
  worker?: Promise<RscWorker | undefined>;
  nextRequest?: Promise<NextRequestCtor | undefined>;
  pluginInstalled: boolean;
  server?: Bun.Server<unknown>;
  inlineRegistry: ActionRegistry;
}

/** Wraps a `Request` in `NextRequest` from `shims/server.ts` when available. */
export async function nextRequestFromRequest(request: Request): Promise<Request> {
  try {
    const specifier = "./shims/server.ts";
    const mod = (await import(specifier)) as { NextRequest?: NextRequestCtor };
    if (typeof mod.NextRequest === "function") {
      return new mod.NextRequest(request);
    }
  } catch {
    // shim unavailable: plain Request
  }
  return request;
}

/** Builds the `Bun.serve` route table from a manifest; every pattern delegates to `handler`. */
export function createAppRouterRoutes(
  manifest: RouteManifest,
  handler: (request: Request, server?: Bun.Server<unknown>) => Promise<Response>,
  options: { staticRoute?: string; compatStaticRoutes?: ReadonlyArray<string>; fontsRoute?: string; imageRoute?: string } = {},
): AppRoutes {
  const routes: AppRoutes = {};
  const add = (pattern: string): void => {
    routes[pattern] = (request, server) => handler(request, server);
  };
  add("/_yolo/health");
  add(`${options.staticRoute ?? "/_yolo/static"}/*`);
  for (const compat of options.compatStaticRoutes ?? ["/_next/static"]) {
    add(`${compat}/*`);
  }
  add(`${options.fontsRoute ?? "/_yolo/fonts"}/*`);
  add(options.imageRoute ?? "/_yolo/image");
  for (const entry of manifest.entries) {
    if (entry.slot) {
      continue;
    }
    add(entry.bunPattern);
    const optional = entry.params.find((param) => param.optional);
    if (optional && entry.bunPattern.endsWith("/*")) {
      add(entry.bunPattern.slice(0, -2) || "/");
    }
  }
  return routes;
}

/** Creates the App Router application for `root`. */
export function createYoloNextApp(options: CreateYoloNextAppOptions): YoloNextApp {
  const root = toPosix(resolve(options.root));
  const config = isResolved(options.config) ? options.config : defineYoloConfig(options.config as YoloConfigInput);
  const dev = options.dev ?? Bun.env.NODE_ENV !== "production";
  const fetchImpl = options.fetchImpl ?? fetch;
  const manifest =
    options.routes ??
    scanAppDir(root, {
      appDir: config.appDir,
      ...(config.pagesDir ? { pagesDir: config.pagesDir } : {}),
      pageExtensions: config.pageExtensions,
    });
  configureCacheLife(config.cacheLife);
  const cdn = config.cdn;
  const basePath = cdn.basePath.replace(/\/$/, "");
  const distDir = join(root, config.distDir);
  const publicDir = join(root, cdn.publicDir);
  const appDir = join(root, config.appDir);
  const deploymentId = cdn.deploymentId;
  const rscMode = options.rscMode ?? (Bun.env.YOLO_RSC_MODE === "inline" ? "inline" : Bun.env.YOLO_RSC_MODE === "worker" ? "worker" : "auto");
  const state: AppState = { pluginInstalled: false, inlineRegistry: new Map() };
  const moduleCache = new Map<string, Promise<RouteModule>>();

  const installPlugin = (): void => {
    if (state.pluginInstalled) {
      return;
    }
    state.pluginInstalled = true;
    try {
      Bun.plugin(yoloNextPlugin({ root, config, env: "server" }));
    } catch {
      // Runtime plugins are best effort (next/* imports then need an explicit alias).
    }
  };

  const importModule = async (file: string): Promise<RouteModule> => {
    installPlugin();
    const absolute = toPosix(resolve(root, file));
    if (!dev) {
      let cached = moduleCache.get(absolute);
      if (!cached) {
        cached = import(absolute) as Promise<RouteModule>;
        moduleCache.set(absolute, cached);
      }
      return cached;
    }
    const mtime = Bun.file(absolute).lastModified;
    try {
      return (await import(`${absolute}?v=${mtime}`)) as RouteModule;
    } catch {
      return (await import(absolute)) as RouteModule;
    }
  };

  const getRegistry = (): Promise<ActionRegistry> => {
    state.registry ??= loadActionManifest(root).then((loaded) => {
      for (const [id, entry] of state.inlineRegistry) {
        loaded.set(id, entry);
      }
      state.inlineRegistry = loaded;
      return loaded;
    });
    return state.registry;
  };

  const getProxy = () => {
    state.proxy ??= (async () => {
      const file = config.proxyFile ?? manifest.proxyFile;
      if (!file) {
        return { matchers: [] };
      }
      const mod = (await importModule(file)) as ProxyModule;
      const candidate = mod.proxy ?? mod.middleware ?? mod.default;
      const handler = typeof candidate === "function" ? (candidate as (request: Request) => unknown) : undefined;
      const raw = mod.config?.matcher;
      const list = raw === undefined ? [] : Array.isArray(raw) ? raw : [raw];
      const matchers = list.map((item) => (typeof item === "string" ? { source: item } : item));
      return handler ? { handler, matchers } : { matchers };
    })();
    return state.proxy;
  };

  const getWorker = (): Promise<RscWorker | undefined> => {
    state.worker ??= (async () => {
      if (rscMode === "inline") {
        return undefined;
      }
      try {
        Bun.resolveSync("react-server-dom-webpack/server.edge", root);
      } catch {
        if (rscMode === "worker") {
          console.warn("[yolo-next] react-server-dom-webpack is not installed; rendering inline.");
        }
        return undefined;
      }
      try {
        return await startRscWorker({ root, env: { NODE_ENV: dev ? "development" : "production" } });
      } catch (error) {
        console.warn("[yolo-next] RSC worker unavailable, rendering inline:", error instanceof Error ? error.message : String(error));
        return undefined;
      }
    })();
    return state.worker;
  };

  const getNextRequest = async (request: Request): Promise<Request> => nextRequestFromRequest(request);

  // -------------------------------------------------------------------------
  // Static files
  // -------------------------------------------------------------------------

  const servePublic = (pathname: string, request: Request): Response | undefined => {
    const relative = decodeURIComponent(pathname).replace(/^\/+/, "");
    if (relative.length === 0 || relative.includes("..") || relative.startsWith("_next/")) {
      return undefined;
    }
    const file = join(publicDir, relative);
    if (!existsSync(file) || !statSync(file).isFile()) {
      return undefined;
    }
    return serveStaticAsset(cdn, file, request);
  };

  const serveStatic = (url: URL, request: Request): Response | undefined => {
    const pathname = url.pathname;
    if (pathname === "/_yolo/health") {
      return textResponse(
        JSON.stringify({ ok: true, runtime: `bun@${Bun.version}`, buildId: Bun.env.YOLO_BUILD_ID ?? "dev", routes: manifest.entries.length }),
        200,
        "application/json",
      );
    }
    for (const prefix of [cdn.staticRoute, ...cdn.compatStaticRoutes]) {
      if (pathname.startsWith(`${prefix}/`)) {
        const rest = pathname.slice(prefix.length + 1);
        if (rest.includes("..")) {
          return new Response("Not Found", { status: 404 });
        }
        const file = join(distDir, "static", rest);
        if (existsSync(file) && statSync(file).isFile()) {
          return serveStaticAsset({ ...cdn, publicCacheControl: cdn.immutableCacheControl }, file, request);
        }
        return new Response("Not Found", { status: 404 });
      }
    }
    if (pathname.startsWith(`${cdn.fontsRoute}/`)) {
      state.fonts ??= createFontsHandler({ root, fontsRoute: cdn.fontsRoute });
      return state.fonts(request);
    }
    if (pathname === "/favicon.ico") {
      const appIcon = join(appDir, "favicon.ico");
      if (existsSync(appIcon)) {
        return serveStaticAsset(cdn, appIcon, request);
      }
    }
    for (const file of manifest.staticMetadata) {
      if (file.generated) {
        continue;
      }
      const base = toPosix(file.path).split("/").pop() ?? "";
      if (pathname === `/${base}` && !existsSync(join(publicDir, base))) {
        const absolute = resolve(root, file.path);
        if (existsSync(absolute)) {
          return serveStaticAsset(cdn, absolute, request);
        }
      }
    }
    return servePublic(pathname, request);
  };

  // -------------------------------------------------------------------------
  // Page rendering
  // -------------------------------------------------------------------------

  interface PageModules {
    page: RouteModule;
    layouts: RouteModule[];
    templates: RouteModule[];
    loading?: RouteModule;
    metadata: Metadata;
    viewport?: Viewport;
  }

  const loadPageModules = async (entry: RouteEntry, props: { params: Promise<Record<string, string | string[]>>; searchParams: Promise<Record<string, string | string[]>> }): Promise<PageModules> => {
    const page = await importModule(entry.file);
    const layouts = await Promise.all(entry.layouts.map((file) => importModule(file)));
    const templates = await Promise.all(entry.templates.map((file) => importModule(file)));
    const loading = entry.loading ? await importModule(entry.loading) : undefined;
    const chain: Metadata[] = [];
    let viewport: Viewport | undefined;
    const collect = async (mod: RouteModule): Promise<void> => {
      if (mod.metadata && typeof mod.metadata === "object") {
        chain.push(mod.metadata as Metadata);
      }
      if (typeof mod.generateMetadata === "function") {
        const parent = Promise.resolve(resolveMetadataChain(chain));
        const generated = await (mod.generateMetadata as (p: unknown, parent: Promise<Metadata>) => Promise<Metadata> | Metadata)(props, parent);
        chain.push(generated ?? {});
      }
      if (mod.viewport && typeof mod.viewport === "object") {
        viewport = mod.viewport as Viewport;
      }
      if (typeof mod.generateViewport === "function") {
        viewport = (await (mod.generateViewport as (p: unknown) => Promise<Viewport> | Viewport)(props)) ?? viewport;
      }
    };
    for (const layout of layouts) {
      await collect(layout);
    }
    await collect(page);
    return { page, layouts, templates, ...(loading ? { loading } : {}), metadata: resolveMetadataChain(chain), ...(viewport ? { viewport } : {}) };
  };

  const headNodes = (metadata: Metadata, viewport?: Viewport): ReactNode[] =>
    headElements(metadata, viewport).map((el, index) => createElement(el.tag, { key: `yolo-head-${index}`, ...el.attrs }, el.children));

  const paramsFor = (entry: RouteEntry, params: Record<string, string>): Record<string, string | string[]> => {
    const out: Record<string, string | string[]> = { ...params };
    for (const param of entry.params) {
      const value = out[param.name];
      if (param.catchAll && typeof value === "string") {
        out[param.name] = value.split("/").filter(Boolean);
      }
    }
    return out;
  };

  const searchParamsFor = (url: URL): Record<string, string | string[]> => {
    const out: Record<string, string | string[]> = {};
    for (const [key, value] of url.searchParams) {
      const prev = out[key];
      out[key] = prev === undefined ? value : [...(Array.isArray(prev) ? prev : [prev]), value];
    }
    return out;
  };

  const wantsFlight = (request: Request, url: URL): boolean =>
    url.searchParams.has("_rsc") || request.headers.get(RSC_HEADERS.rsc) === "1" || request.headers.get("accept")?.includes(RSC_HEADERS.contentType) === true;

  const flightResponse = (body: BodyInit, status: number, extra: Record<string, string> = {}): Response =>
    new Response(body, {
      status,
      headers: {
        "content-type": RSC_HEADERS.contentType,
        vary: `${RSC_HEADERS.rsc}, ${RSC_HEADERS.routerStateTree}, ${RSC_HEADERS.routerPrefetch}`,
        [RSC_HEADERS.staleTime]: "300",
        ...extra,
      },
    });

  const renderInlineHtml = async (element: ReactElement, headHtml: string, status: number, store: RequestStore): Promise<Response> => {
    const flightStream = await runWithRequestStore(store, () => renderToFlight({ element }));
    const html = await renderFlightToHtml({ flightStream, headHtml, fallback: true });
    return new Response(html, { status, headers: { "content-type": "text/html; charset=utf-8" } });
  };

  const renderErrorPage = async (entry: RouteEntry | undefined, error: Error, status: number, store: RequestStore): Promise<Response> => {
    const digest = (error as { digest?: string }).digest;
    const boundaryFile = entry?.error ?? manifest.globalError;
    try {
      if (boundaryFile) {
        const boundary = await importModule(boundaryFile);
        const Boundary = boundary.default as ((props: Record<string, unknown>) => unknown) | undefined;
        if (typeof Boundary === "function") {
          const props = { error: Object.assign(new Error(dev ? error.message : "An error occurred in the Server Components render."), digest ? { digest } : {}), reset: () => undefined, retry: () => undefined };
          const isGlobal = boundaryFile === manifest.globalError && !entry?.error;
          let node: ReactElement = createElement(Boundary as never, props);
          if (!isGlobal && manifest.rootLayout) {
            const layout = await importModule(manifest.rootLayout);
            node = createElement(layout.default as never, { params: Promise.resolve({}) }, node);
          }
          return renderInlineHtml(node, "", status, store);
        }
      }
    } catch (nested) {
      console.error("[yolo-next] error boundary failed:", nested);
    }
    const body = `<!DOCTYPE html><html><head><title>${status}</title></head><body><h1>${status}</h1><p>${escapeHtml(dev ? error.message : "Internal Server Error")}</p></body></html>`;
    return textResponse(body, status, "text/html; charset=utf-8");
  };

  const renderNotFound = async (request: Request, url: URL, entry?: RouteEntry): Promise<Response> => {
    const store = createRequestStore(request, { phase: "render", ...(deploymentId ? { deploymentId } : {}) });
    const file = entry?.notFound ?? manifest.rootNotFound;
    if (!file) {
      return textResponse("<!DOCTYPE html><html><head><title>404: This page could not be found.</title></head><body><h1>404</h1><p>This page could not be found.</p></body></html>", 404, "text/html; charset=utf-8");
    }
    try {
      const notFound = await importModule(file);
      let node: ReactElement = createElement(notFound.default as never, {});
      const layouts = entry ? entry.layouts : manifest.rootLayout ? [manifest.rootLayout] : [];
      for (const layoutFile of [...layouts].reverse()) {
        const layout = await importModule(layoutFile);
        node = createElement(layout.default as never, { params: Promise.resolve({}) }, node);
      }
      if (wantsFlight(request, url)) {
        const flight = await runWithRequestStore(store, () => renderToFlight({ element: node }));
        return flightResponse(flight, 404);
      }
      return renderInlineHtml(node, "", 404, store);
    } catch (error) {
      return renderErrorPage(entry, error instanceof Error ? error : new Error(String(error)), 500, store);
    }
  };

  const renderPage = async (request: Request, url: URL, match: RouteMatch): Promise<Response> => {
    const entry = match.entry;
    const started = performance.now();
    const store = createRequestStore(request, { params: match.params, phase: "render", ...(deploymentId ? { deploymentId } : {}) });
    const params = paramsFor(entry, match.params);
    const searchParams = searchParamsFor(url);
    const props = { params: Promise.resolve(params), searchParams: Promise.resolve(searchParams) };
    const flight = wantsFlight(request, url);
    const segment = entry.segmentConfig;
    try {
      const modules = await runWithRequestStore(store, () => loadPageModules(entry, props));
      const headHtml = renderHeadTags(modules.metadata, modules.viewport);
      const worker = await getWorker();
      let response: Response;
      if (worker) {
        const headers: Record<string, string> = {};
        request.headers.forEach((value, key) => {
          headers[key] = value;
        });
        const workerResponse = await worker.render({
          pathname: url.pathname,
          search: url.search,
          headers,
          routeFile: toPosix(resolve(root, entry.file)),
          layoutFiles: entry.layouts.map((file) => toPosix(resolve(root, file))),
          templateFiles: entry.templates.map((file) => toPosix(resolve(root, file))),
          ...(entry.loading ? { loadingFile: toPosix(resolve(root, entry.loading)) } : {}),
          params: match.params,
        });
        const redirectTo = workerResponse.headers.get("x-yolo-redirect");
        const signalled = Number(workerResponse.headers.get("x-yolo-status") ?? "200");
        if (redirectTo) {
          return new Response(null, { status: signalled === 308 ? 308 : 307, headers: { location: redirectTo } });
        }
        if (signalled === 404) {
          return renderNotFound(request, url, entry);
        }
        if (workerResponse.status !== 200) {
          throw new Error(await workerResponse.text());
        }
        if (workerResponse.headers.get("x-yolo-dynamic") === "1") {
          store.usedDynamic = true;
        }
        if (flight) {
          response = flightResponse(workerResponse.body ?? "", 200);
        } else {
          const clientModuleMap = await loadClientModuleMap();
          const html = await renderFlightToHtml({ flightStream: workerResponse.body ?? new Response("").body!, headHtml, clientModuleMap });
          response = new Response(html, { status: 200, headers: { "content-type": "text/html; charset=utf-8" } });
        }
      } else {
        const element = buildRouteElement({
          page: modules.page,
          layouts: modules.layouts,
          templates: modules.templates,
          ...(modules.loading ? { loading: modules.loading } : {}),
          params: match.params,
          searchParams,
          head: headNodes(modules.metadata, modules.viewport),
          slots: Object.fromEntries(entry.parallelSlots.map((slot) => [slot, null])),
        });
        const flightStream = await runWithRequestStore(store, () => renderToFlight({ element }));
        if (flight) {
          response = flightResponse(flightStream, 200);
        } else {
          const html = await renderFlightToHtml({ flightStream, fallback: true });
          response = new Response(html, { status: 200, headers: { "content-type": "text/html; charset=utf-8" } });
        }
      }
      await runAfterCallbacks(store);
      const headers = new Headers(response.headers);
      mergeStoreHeaders(store, headers);
      const dynamic = store.usedDynamic || segment.dynamic === "force-dynamic" || flight;
      if (dynamic || segment.revalidate === 0) {
        headers.set("cache-control", cdn.dynamicHtmlCacheControl);
      } else if (typeof segment.revalidate === "number") {
        headers.set("cache-control", `s-maxage=${segment.revalidate}, stale-while-revalidate=${cdn.expireTime}`);
      } else {
        headers.set("cache-control", `public, s-maxage=${cdn.expireTime}, stale-while-revalidate`);
      }
      if (dev) {
        headers.set("server-timing", `render;dur=${(performance.now() - started).toFixed(1)}`);
      }
      return new Response(response.body, { status: response.status, headers });
    } catch (error) {
      if (isRedirectError(error)) {
        return new Response(null, { status: getRedirectStatus(error), headers: { location: getRedirectUrl(error) } });
      }
      if (isHttpAccessFallbackError(error)) {
        const status = getAccessFallbackStatus(error);
        if (status === 404) {
          return renderNotFound(request, url, entry);
        }
        return renderErrorPage(entry, Object.assign(new Error(status === 403 ? "Forbidden" : "Unauthorized"), { digest: (error as { digest?: string }).digest }), status, store);
      }
      console.error(`[yolo-next] render failed for ${url.pathname}:`, error);
      return renderErrorPage(entry, error instanceof Error ? error : new Error(String(error)), 500, store);
    }
  };

  let clientModuleMapPromise: Promise<ClientModuleMap> | undefined;
  const loadClientModuleMap = (): Promise<ClientModuleMap> => {
    clientModuleMapPromise ??= (async () => {
      const file = Bun.file(join(distDir, "client-manifest.json"));
      if (!(await file.exists())) {
        return {};
      }
      try {
        const json = (await file.json()) as { ssrModuleMapping?: ClientModuleMap; clientModules?: Record<string, { id: string; name: string; chunks: string[] }> };
        if (json.ssrModuleMapping) {
          return json.ssrModuleMapping;
        }
        const map: ClientModuleMap = {};
        for (const [key, entry] of Object.entries(json.clientModules ?? {})) {
          const [module, exportName = "default"] = key.split("#");
          (map[module ?? key] ??= {})[exportName] = { id: resolve(root, entry.id), name: entry.name, chunks: [] };
        }
        return map;
      } catch {
        return {};
      }
    })();
    return clientModuleMapPromise;
  };

  // -------------------------------------------------------------------------
  // Route handlers and metadata routes
  // -------------------------------------------------------------------------

  const runRouteHandler = async (request: Request, match: RouteMatch): Promise<Response> => {
    const entry = match.entry;
    const mod = await importModule(entry.file);
    const method = request.method.toUpperCase();
    const methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"].filter((m) => typeof mod[m] === "function");
    type RouteHandlerFn = (request: Request, ctx: { params: Promise<Record<string, string | string[]>> }) => Promise<Response> | Response;
    let handler: RouteHandlerFn | undefined = typeof mod[method] === "function" ? (mod[method] as RouteHandlerFn) : undefined;
    if (!handler && method === "HEAD" && typeof mod.GET === "function") {
      handler = mod.GET as RouteHandlerFn;
    }
    if (!handler && method === "OPTIONS") {
      const allow = [...new Set([...methods, ...(methods.includes("GET") ? ["HEAD"] : []), "OPTIONS"])].join(", ");
      return new Response(null, { status: 204, headers: { allow } });
    }
    if (!handler) {
      return new Response(null, { status: 405, headers: { allow: methods.join(", ") } });
    }
    const store = createRequestStore(request, { params: match.params, phase: "route", ...(deploymentId ? { deploymentId } : {}) });
    const nextRequest = await getNextRequest(request);
    try {
      const response = await runWithRequestStore(store, () => handler(nextRequest, { params: Promise.resolve(paramsFor(entry, match.params)) }));
      await runAfterCallbacks(store);
      const result = response instanceof Response ? response : new Response(String(response));
      const headers = mergeStoreHeaders(store, new Headers(result.headers));
      const body = method === "HEAD" ? null : result.body;
      return new Response(body, { status: result.status, statusText: result.statusText, headers });
    } catch (error) {
      if (isRedirectError(error)) {
        return new Response(null, { status: getRedirectStatus(error), headers: { location: getRedirectUrl(error) } });
      }
      if (isHttpAccessFallbackError(error)) {
        return new Response(null, { status: getAccessFallbackStatus(error) });
      }
      console.error(`[yolo-next] route handler failed for ${entry.file}:`, error);
      return textResponse(dev ? String(error instanceof Error ? error.stack ?? error.message : error) : "Internal Server Error", 500, "text/plain; charset=utf-8");
    }
  };

  const runMetadataRoute = async (request: Request, match: RouteMatch): Promise<Response> => {
    const entry = match.entry;
    const mod = await importModule(entry.file);
    const generate = mod.default as ((ctx: Record<string, unknown>) => unknown) | undefined;
    if (typeof generate !== "function") {
      return new Response("Not Found", { status: 404 });
    }
    const store = createRequestStore(request, { params: match.params, phase: "route", ...(deploymentId ? { deploymentId } : {}) });
    const ctx = { params: Promise.resolve(match.params), id: match.params.id ?? match.params.__metadata_id__ };
    const result = await runWithRequestStore(store, () => generate(ctx));
    switch (entry.metadataKind) {
      case "sitemap":
        return textResponse(sitemapXml((result as SitemapItem[]) ?? []), 200, "application/xml", { "cache-control": "public, max-age=0, must-revalidate" });
      case "robots":
        return textResponse(robotsTxt((result as Parameters<typeof robotsTxt>[0]) ?? {}), 200, "text/plain; charset=utf-8", { "cache-control": "public, max-age=0, must-revalidate" });
      case "manifest":
        return textResponse(JSON.stringify(result ?? {}), 200, "application/manifest+json", { "cache-control": "public, max-age=0, must-revalidate" });
      default: {
        const contentType = typeof mod.contentType === "string" ? mod.contentType : undefined;
        if (result instanceof Response) {
          const headers = new Headers(result.headers);
          if (contentType && !headers.has("content-type")) {
            headers.set("content-type", contentType);
          }
          headers.set("cache-control", "public, immutable, no-transform, max-age=31536000");
          return new Response(result.body, { status: result.status, headers });
        }
        if (result instanceof Blob || result instanceof Uint8Array || result instanceof ArrayBuffer) {
          return new Response(result as BodyInit, { headers: { "content-type": contentType ?? "application/octet-stream", "cache-control": "public, immutable, no-transform, max-age=31536000" } });
        }
        return new Response(String(result), { headers: { "content-type": contentType ?? "text/plain; charset=utf-8" } });
      }
    }
  };

  // -------------------------------------------------------------------------
  // Proxy
  // -------------------------------------------------------------------------

  const runProxy = async (request: Request, url: URL): Promise<{ response?: Response; request: Request; url: URL; headers?: Headers }> => {
    const proxy = await getProxy();
    if (!proxy.handler) {
      return { request, url };
    }
    if (proxy.matchers.length > 0) {
      const facts = requestFacts(request, url);
      const matched = proxy.matchers.some((matcher) => matchRule(matcher as never, url.pathname, facts, basePath) !== undefined);
      if (!matched) {
        return { request, url };
      }
    }
    const store = createRequestStore(request, { phase: "proxy", ...(deploymentId ? { deploymentId } : {}) });
    const nextRequest = await getNextRequest(request);
    const result = await runWithRequestStore(store, async () => proxy.handler?.(nextRequest));
    await runAfterCallbacks(store);
    if (!(result instanceof Response)) {
      return { request, url, headers: store.responseHeaders };
    }
    const headers = mergeStoreHeaders(store, new Headers(result.headers));
    const rewrite = headers.get("x-middleware-rewrite");
    if (rewrite) {
      const target = new URL(rewrite, url);
      headers.delete("x-middleware-rewrite");
      headers.delete("x-middleware-next");
      return { request, url: target, headers };
    }
    if (headers.get("x-middleware-next") === "1") {
      headers.delete("x-middleware-next");
      const overrides = headers.get("x-middleware-override-headers");
      let next = request;
      if (overrides) {
        const requestHeaders = new Headers(request.headers);
        for (const key of overrides.split(",")) {
          const value = headers.get(`x-middleware-request-${key.trim()}`);
          if (value !== null) {
            requestHeaders.set(key.trim(), value);
            headers.delete(`x-middleware-request-${key.trim()}`);
          }
        }
        headers.delete("x-middleware-override-headers");
        next = new Request(request, { headers: requestHeaders });
      }
      return { request: next, url, headers };
    }
    return { response: new Response(result.body, { status: result.status, statusText: result.statusText, headers }), request, url };
  };

  // -------------------------------------------------------------------------
  // Pipeline
  // -------------------------------------------------------------------------

  const externalRewrite = async (request: Request, target: URL): Promise<Response> => {
    const headers = new Headers(request.headers);
    headers.delete("host");
    return fetchImpl(target, { method: request.method, headers, body: request.method === "GET" || request.method === "HEAD" ? null : request.body, redirect: "manual" } as RequestInit);
  };

  const dispatch = async (request: Request, url: URL): Promise<Response | undefined> => {
    if (await isActionRequest(request)) {
      const registry = await getRegistry();
      const worker = await getWorker();
      if (worker && registry.size === 0) {
        return worker.action(request);
      }
      return handleServerAction(request, {
        registry,
        allowedOrigins: config.server.actionAllowedOrigins,
        bodySizeLimit: config.server.actionBodySizeLimit,
        store: createRequestStore(request, { phase: "action", ...(deploymentId ? { deploymentId } : {}) }),
        onRevalidate: (revalidated) => {
          for (const path of revalidated.paths) {
            void pathTags(path);
          }
        },
      });
    }
    if (config.images) {
      const imageRoutes = [config.images.route, ...config.images.compatRoutes];
      if (imageRoutes.includes(url.pathname)) {
        state.image ??= createImageOptimizerHandler(config.images, { root, publicDir: cdn.publicDir, fetchImpl, dev } as Parameters<typeof createImageOptimizerHandler>[1]);
        return state.image(request);
      }
    }
    let match = matchRoute(manifest, url.pathname);
    if (match?.entry.slot) {
      const primary = manifest.entries.find((entry) => entry.urlPattern === match?.entry.urlPattern && !entry.slot);
      match = primary ? { entry: primary, params: match.params } : match;
    }
    if (!match) {
      return undefined;
    }
    switch (match.entry.kind) {
      case "route-handler":
        if (match.entry.legacyPages) {
          return textResponse("Legacy pages/api handlers are not supported by the App Router runtime.", 501, "text/plain; charset=utf-8");
        }
        return runRouteHandler(request, match);
      case "metadata":
        return runMetadataRoute(request, match);
      default:
        if (request.method !== "GET" && request.method !== "HEAD") {
          return new Response(null, { status: 405, headers: { allow: "GET, HEAD" } });
        }
        return renderPage(request, url, match);
    }
  };

  const finish = (request: Request, response: Response, extraHeaders: Headers | undefined): Response => {
    const headers = new Headers(response.headers);
    if (extraHeaders) {
      for (const [key, value] of extraHeaders) {
        if (key === "set-cookie") {
          headers.append("set-cookie", value);
        } else if (!headers.has(key) || key.startsWith("x-")) {
          headers.set(key, value);
        }
      }
    }
    if (config.server.poweredByHeader) {
      headers.set("x-powered-by", "Yolo");
    }
    if (deploymentId) {
      headers.set("x-nextjs-deployment-id", deploymentId);
    }
    const buffered = headers.get(BUFFERED) === "1";
    headers.delete(BUFFERED);
    if (!buffered) {
      return new Response(response.body, { status: response.status, statusText: response.statusText, headers });
    }
    return response.arrayBuffer().then((buffer) => {
      let body: BodyInit = buffer;
      const type = headers.get("content-type") ?? "";
      if (config.server.generateEtags && !headers.has("etag")) {
        headers.set("etag", `W/"${Bun.hash(new Uint8Array(buffer)).toString(16)}"`);
        if (request.headers.get("if-none-match") === headers.get("etag")) {
          return new Response(null, { status: 304, headers });
        }
      }
      const accept = request.headers.get("accept-encoding") ?? "";
      if (config.server.compress && buffer.byteLength > 1024 && COMPRESSIBLE.test(type) && !headers.has("content-encoding")) {
        if (/\bzstd\b/.test(accept)) {
          body = new Uint8Array(Bun.zstdCompressSync(new Uint8Array(buffer)));
          headers.set("content-encoding", "zstd");
        } else if (/\bgzip\b/.test(accept)) {
          body = new Uint8Array(Bun.gzipSync(new Uint8Array(buffer)));
          headers.set("content-encoding", "gzip");
        }
        if (headers.has("content-encoding")) {
          headers.append("vary", "Accept-Encoding");
          headers.delete("content-length");
        }
      }
      return new Response(request.method === "HEAD" ? null : body, { status: response.status, statusText: response.statusText, headers });
    }) as unknown as Response;
  };

  // `defineYoloConfig` is synchronous: function-valued `redirects`/`rewrites`/`headers` of the
  // original next.config are awaited once here, on the first request.
  let rulesConfig: ResolvedYoloConfig = config;
  let rulesReady: Promise<void> | undefined;
  const ensureRules = (): Promise<void> => {
    rulesReady ??= (async () => {
      const next = config.next;
      const patch: { redirects?: ResolvedYoloConfig["redirects"]; rewrites?: ResolvedYoloConfig["rewrites"]; headers?: ResolvedYoloConfig["headers"] } = {};
      if (typeof next.redirects === "function" && config.redirects.length === 0) {
        patch.redirects = await next.redirects();
      }
      if (typeof next.headers === "function" && config.headers.length === 0) {
        patch.headers = await next.headers();
      }
      const rewrites = config.rewrites;
      if (typeof next.rewrites === "function" && !(rewrites.beforeFiles?.length || rewrites.afterFiles?.length || rewrites.fallback?.length)) {
        const value = await next.rewrites();
        patch.rewrites = Array.isArray(value) ? { afterFiles: value } : (value as ResolvedYoloConfig["rewrites"]);
      }
      rulesConfig = { ...config, ...patch };
    })();
    return rulesReady;
  };

  const handle = async (request: Request): Promise<Response> => {
    await ensureRules();
    let url = new URL(request.url);
    const extra = new Headers();
    // Skew protection: a stale client bundle reloads on x-nextjs-deployment-id mismatch.
    if (deploymentId) {
      const requested = request.headers.get(cdn.skewHeader) ?? url.searchParams.get(cdn.skewQuery);
      if (requested && requested !== deploymentId && request.headers.has(RSC_HEADERS.rsc)) {
        return new Response(null, { status: 200, headers: { "x-nextjs-deployment-id": deploymentId, "content-type": "text/plain" } });
      }
    }
    if (basePath) {
      if (url.pathname === basePath || url.pathname.startsWith(`${basePath}/`)) {
        url = new URL(url);
        url.pathname = url.pathname.slice(basePath.length) || "/";
        request = new Request(url, request);
      } else if (!url.pathname.startsWith("/_yolo/") && !url.pathname.startsWith("/_next/")) {
        return new Response("Not Found", { status: 404 });
      }
    }
    // Trailing slash normalization.
    if (!config.trailingSlash && url.pathname.length > 1 && url.pathname.endsWith("/") && !cdn.trailingSlash) {
      const target = `${url.pathname.replace(/\/+$/, "")}${url.search}`;
      return new Response(null, { status: 308, headers: { location: `${basePath}${target}` } });
    }
    if (config.trailingSlash && !url.pathname.endsWith("/") && !/\.[a-z0-9]+$/i.test(url.pathname) && !url.pathname.startsWith("/api/") && !url.pathname.startsWith("/_")) {
      return new Response(null, { status: 308, headers: { location: `${basePath}${url.pathname}/${url.search}` } });
    }
    // 1. static files.
    const asset = serveStatic(url, request);
    if (asset) {
      return finish(request, asset, undefined);
    }
    // 2. headers rules.
    const headerRules = applyRules(request, rulesConfig, "headers");
    if (headerRules.headers) {
      for (const [key, value] of headerRules.headers) {
        extra.set(key, value);
      }
    }
    // 3. redirects.
    const redirects = applyRules(request, rulesConfig, "redirects");
    if (redirects.response) {
      return finish(request, redirects.response, extra);
    }
    // 4. proxy.
    const proxied = await runProxy(request, url);
    if (proxied.response) {
      return finish(request, proxied.response, extra);
    }
    request = proxied.request;
    url = proxied.url;
    if (proxied.headers) {
      for (const [key, value] of proxied.headers) {
        if (key === "set-cookie") {
          extra.append("set-cookie", value);
        } else {
          extra.set(key, value);
        }
      }
    }
    if (url.origin !== new URL(request.url).origin) {
      return finish(request, await externalRewrite(request, url), extra);
    }
    if (url.href !== request.url) {
      request = new Request(url, request);
    }
    // 5. rewrites beforeFiles, 6. filesystem, 7. afterFiles, 9. fallback.
    for (const phase of ["beforeFiles", "afterFiles", "fallback"] as const) {
      const rewrite = applyRules(request, rulesConfig, phase);
      if (rewrite.rewrittenUrl) {
        if (rewrite.rewrittenUrl.origin !== url.origin) {
          return finish(request, await externalRewrite(request, rewrite.rewrittenUrl), extra);
        }
        url = rewrite.rewrittenUrl;
        request = new Request(url, request);
      }
      if (phase === "beforeFiles" || rewrite.rewrittenUrl) {
        const response = await dispatch(request, url);
        if (response) {
          return finish(request, response, extra);
        }
      }
    }
    // 10. 404.
    return finish(request, await renderNotFound(request, url), extra);
  };

  const fetchHandler = async (request: Request): Promise<Response> => {
    try {
      return await handle(request);
    } catch (error) {
      console.error("[yolo-next] unhandled error:", error);
      return textResponse(dev ? String(error instanceof Error ? error.stack ?? error.message : error) : "Internal Server Error", 500, "text/plain; charset=utf-8");
    }
  };

  const development = dev ? { hmr: config.server.hmr, console: config.server.browserConsole } : false;
  const app: YoloNextApp = {
    config,
    manifest,
    routes: createAppRouterRoutes(manifest, (request) => fetchHandler(request), {
      staticRoute: cdn.staticRoute,
      compatStaticRoutes: cdn.compatStaticRoutes,
      fontsRoute: cdn.fontsRoute,
      ...(config.images ? { imageRoute: config.images.route } : {}),
    }),
    hostname: config.server.hostname,
    development,
    port: config.server.port,
    fetch: (request) => fetchHandler(request),
    listen(port = Number(Bun.env.PORT ?? config.server.port)) {
      const server = Bun.serve({
        port,
        hostname: config.server.hostname,
        development,
        idleTimeout: 30,
        fetch: (request) => fetchHandler(request),
        error(error) {
          console.error("[yolo-next] server error:", error);
          return new Response(`<!DOCTYPE html><html><body><h1>500</h1><pre>${escapeHtml(dev ? String(error.stack ?? error.message) : "Internal Server Error")}</pre></body></html>`, {
            status: 500,
            headers: { "content-type": "text/html; charset=utf-8" },
          });
        },
      });
      state.server = server;
      app.port = server.port;
      return server;
    },
    async stop() {
      const worker = state.worker ? await state.worker : undefined;
      worker?.stop();
      await state.server?.stop(true);
      state.server = undefined;
    },
    registerAction(module, exportName, fn) {
      const id = registerServerAction(state.inlineRegistry, module, exportName, fn);
      if (state.registry) {
        void state.registry.then((registry) => {
          const entry = state.inlineRegistry.get(id);
          if (entry) {
            registry.set(id, entry);
          }
        });
      }
      return id;
    },
  };
  return app;
}

/** Content type helper re-exported for route tables (`cdn.ts`). */
export { cacheControlFor, contentTypeFor };
