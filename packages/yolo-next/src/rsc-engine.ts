// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/rsc-engine: picks the React Server Components engine of a project at runtime:
//   1. `fullstack`      — @yolo-rs/fullstack (react-server-dom-parcel, its own three-pass build), adapted
//                         through its `buildApp` / `loadApp` handler (Flight via the `rsc: 1` header);
//   2. `webpack-worker` — the react-server-dom-webpack bundles produced by `buildYoloNext` (the rsc
//                         bundle is an isolated `react-server` realm; the ssr bundle streams the HTML);
//   3. `inline`         — plain React SSR of the route modules (no Flight), or a static HTML shell.
// Every step is wrapped in try/catch: resolution never throws and render failures fall back to `inline`.

import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { publicUrl } from "./cdn.ts";
import { headElements, renderHeadTags, resolveMetadataChain } from "./metadata.ts";
import { yoloNextPlugin } from "./plugin.ts";
import { loadRouteManifest, matchRoute } from "./router.ts";
import { renderFlightToHtml, startRscWorker, type RscWorker } from "./rsc.ts";
import type { Metadata, ResolvedYoloConfig, RouteEntry, RouteManifest, RouteMatch, Viewport } from "./types.ts";

/** Engine forced through `YOLO_RSC_ENGINE` (env) or `experimental.rscEngine` (next.config). */
export function forcedRscEngine(config?: ResolvedYoloConfig, env: Record<string, string | undefined> = Bun.env): RscEngineName | undefined {
  const candidates = [env.YOLO_RSC_ENGINE, (config?.next.experimental as { rscEngine?: unknown } | undefined)?.rscEngine];
  for (const value of candidates) {
    if (value === "fullstack" || value === "webpack-worker" || value === "inline") return value;
    if (value === "worker" || value === "webpack") return "webpack-worker";
  }
  return undefined;
}

/** Engine identifiers in resolution order. */
export type RscEngineName = "fullstack" | "webpack-worker" | "inline";

/** Input of `renderFlight` / `renderHtml`. Only `request` is required; the rest is derived when absent. */
export interface RscRenderInput {
  readonly request: Request;
  readonly url?: URL;
  readonly match?: RouteMatch;
  readonly manifest?: RouteManifest;
  readonly config?: ResolvedYoloConfig;
  /** Metadata resolved by the caller (merged on top of the modules' static metadata). */
  readonly metadata?: Metadata;
  readonly viewport?: Viewport;
  readonly status?: number;
  readonly bootstrapModules?: ReadonlyArray<string>;
  readonly bootstrapScriptContent?: string;
  readonly stylesheets?: ReadonlyArray<string>;
  readonly formState?: unknown;
  readonly nonce?: string;
  readonly signal?: AbortSignal;
}

/** Context of `handleAction`. */
export interface RscActionContext {
  readonly actionId?: string;
  readonly match?: RouteMatch;
  readonly manifest?: RouteManifest;
  readonly config?: ResolvedYoloConfig;
}

/** A resolved RSC engine. */
export interface RscEngine {
  readonly name: RscEngineName;
  /** Human-readable facts (candidate path, version, bundle paths). */
  readonly details: Readonly<Record<string, string>>;
  renderFlight(input: RscRenderInput): Promise<ReadableStream<Uint8Array>>;
  renderHtml(input: RscRenderInput): Promise<ReadableStream<Uint8Array>>;
  handleAction?(request: Request, ctx: RscActionContext): Promise<Response>;
  /** Whole-response handler when the engine owns routing too (fullstack). */
  fetch?(request: Request): Promise<Response>;
}

/** Options of `resolveRscEngine`. */
export interface ResolveRscEngineOptions {
  readonly root: string;
  readonly config?: ResolvedYoloConfig;
  readonly manifest?: RouteManifest;
  /** Candidate directories of `@yolo-rs/fullstack` (default `FULLSTACK_CANDIDATES(root)`). */
  readonly candidates?: ReadonlyArray<string>;
  /** Also try `Bun.resolveSync("@yolo-rs/fullstack/app")` from the root (default true). */
  readonly packageResolve?: boolean;
  /** Try the built react-server-dom-webpack bundles under the dist dir (default true). */
  readonly bundles?: boolean;
  /** Force an engine (`inline` always succeeds). */
  readonly prefer?: RscEngineName;
  /** Output directory of the fullstack build (default `<root>/<distDir>/fullstack`). */
  readonly fullstackOutDir?: string;
  /** Rebuild the fullstack app even when a manifest exists. */
  readonly rebuild?: boolean;
  readonly dev?: boolean;
  readonly log?: (message: string) => void;
}

/** Default candidate directories of `@yolo-rs/fullstack`. */
export function FULLSTACK_CANDIDATES(root: string): string[] {
  return [
    join(root, "node_modules", "@yolo-rs", "fullstack"),
    "C:/src/yolo-stack/repos/yolo-ui/packages/yolo-fullstack",
    "C:/src/yolo-ui/packages/yolo-fullstack",
  ];
}

const encoder = new TextEncoder();

function textStream(text: string): ReadableStream<Uint8Array> {
  return new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(encoder.encode(text));
      controller.close();
    },
  });
}

function withHeaders(request: Request, headers: Record<string, string>): Request {
  const next = new Headers(request.headers);
  for (const [key, value] of Object.entries(headers)) next.set(key, value);
  return new Request(request, { headers: next });
}

// ---------------------------------------------------------------------------
// 1. @yolo-rs/fullstack adapter
// ---------------------------------------------------------------------------

interface FullstackHandler {
  fetch(request: Request): Promise<Response>;
  render?(request: Request, options?: unknown): Promise<{ response: Response; rsc?: Uint8Array }>;
}

interface FullstackModule {
  buildApp?: (options: { root: string; outDir?: string; dev?: boolean }) => Promise<unknown>;
  loadApp?: (outDir: string, options?: unknown) => Promise<FullstackHandler>;
  devApp?: (options: { root: string; outDir?: string }) => Promise<unknown>;
}

/** Options of `adaptFullstackModule`. */
export interface FullstackAdapterOptions {
  readonly root: string;
  readonly outDir?: string;
  readonly rebuild?: boolean;
  readonly dev?: boolean;
  readonly source?: string;
  readonly log?: (message: string) => void;
  /** Engine used when the fullstack handler cannot be built or loaded (default: inline). */
  readonly fallback?: RscEngine;
}

/**
 * Wraps an imported `@yolo-rs/fullstack/app` module (anything exposing `loadApp`, optionally
 * `buildApp`) as an RscEngine: the handler is built/loaded lazily on first render; Flight payloads
 * come from `fetch` with the `rsc: 1` header, actions forward the `next-action` id as `yolo-action`.
 */
export function adaptFullstackModule(mod: unknown, options: FullstackAdapterOptions): RscEngine | undefined {
  const fullstack = mod as FullstackModule | null | undefined;
  if (!fullstack || typeof fullstack !== "object" || typeof fullstack.loadApp !== "function") return undefined;
  const root = resolve(options.root);
  const outDir = options.outDir ?? join(root, ".yolo", "fullstack");
  const log = options.log ?? (() => {});
  let handlerPromise: Promise<FullstackHandler> | undefined;
  let fallback: RscEngine | undefined;
  const ensure = (): Promise<FullstackHandler> => {
    handlerPromise ??= (async () => {
      const manifestFile = join(outDir, "manifest.json");
      if ((options.rebuild || !existsSync(manifestFile)) && typeof fullstack.buildApp === "function") {
        log(`fullstack: building ${root} -> ${outDir}`);
        await fullstack.buildApp({ root, outDir, dev: options.dev });
      }
      const handler = await fullstack.loadApp!(outDir);
      if (!handler || typeof handler.fetch !== "function") throw new Error("@yolo-rs/fullstack loadApp returned no fetch handler");
      return handler;
    })();
    return handlerPromise;
  };
  const useFallback = (error: unknown): RscEngine => {
    if (!fallback) {
      log(`fullstack engine unavailable, falling back to inline: ${error instanceof Error ? error.message : String(error)}`);
      fallback = options.fallback ?? createInlineEngine({ root });
    }
    return fallback;
  };
  const details: Record<string, string> = { source: options.source ?? "module", outDir };
  return {
    name: "fullstack",
    details,
    async renderFlight(input) {
      try {
        const handler = await ensure();
        const response = await handler.fetch(withHeaders(input.request, { rsc: "1", accept: "text/x-component" }));
        return response.body ?? textStream("");
      } catch (error) {
        return useFallback(error).renderFlight(input);
      }
    },
    async renderHtml(input) {
      try {
        const handler = await ensure();
        const response = await handler.fetch(input.request);
        return response.body ?? textStream("");
      } catch (error) {
        return useFallback(error).renderHtml(input);
      }
    },
    async handleAction(request, ctx) {
      const handler = await ensure();
      const id = ctx.actionId ?? request.headers.get("next-action") ?? "";
      return handler.fetch(id ? withHeaders(request, { "yolo-action": id }) : request);
    },
    async fetch(request) {
      const handler = await ensure();
      return handler.fetch(request);
    },
  };
}

async function importFullstack(options: ResolveRscEngineOptions): Promise<{ mod: unknown; source: string } | undefined> {
  const root = resolve(options.root);
  if (options.packageResolve !== false) {
    for (const specifier of ["@yolo-rs/fullstack/app", "@yolo-rs/fullstack"]) {
      try {
        const file = Bun.resolveSync(specifier, root);
        const mod = await import(Bun.pathToFileURL(file).href);
        return { mod, source: file };
      } catch {
        // try the next candidate
      }
    }
  }
  for (const candidate of options.candidates ?? FULLSTACK_CANDIDATES(root)) {
    const file = join(candidate, "src", "app", "index.ts");
    if (!existsSync(file)) continue;
    try {
      const mod = await import(Bun.pathToFileURL(file).href);
      return { mod, source: file };
    } catch (error) {
      options.log?.(`fullstack candidate ${file} failed to import: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// 2. react-server-dom-webpack bundles
// ---------------------------------------------------------------------------

interface RscBundle {
  renderFlight(input: Record<string, unknown>, options?: Record<string, unknown>): Promise<ReadableStream<Uint8Array>>;
  handleAction?(request: Request, input: Record<string, unknown>): Promise<Response>;
}

interface SsrBundle {
  renderHtml(flight: ReadableStream<Uint8Array>, options?: Record<string, unknown>): Promise<ReadableStream<Uint8Array>>;
}

function flightInput(input: RscRenderInput, manifest: RouteManifest | undefined): Record<string, unknown> {
  const url = input.url ?? new URL(input.request.url);
  const match = input.match ?? (manifest ? matchRoute(manifest, url.pathname) : undefined);
  return {
    url: url.href,
    method: input.request.method,
    pathname: url.pathname,
    search: url.search,
    params: match?.params ?? {},
    entry: match?.entry,
    status: input.status ?? (match ? 200 : 404),
  };
}

/** Creates the engine backed by the bundles of `buildYoloNext` when they exist under the dist dir. */
export async function createWebpackEngine(options: ResolveRscEngineOptions): Promise<RscEngine | undefined> {
  const root = resolve(options.root);
  const distDir = options.config?.distDir ?? ".yolo";
  const dist = join(root, distDir);
  const rscFile = join(dist, "server", "rsc", "entry-rsc.js");
  const ssrFile = join(dist, "server", "ssr", "entry-ssr.js");
  if (!existsSync(rscFile) || !existsSync(ssrFile)) return createWorkerEngine(options);
  let rsc: RscBundle;
  let ssr: SsrBundle;
  try {
    rsc = (await import(Bun.pathToFileURL(rscFile).href)) as RscBundle;
    ssr = (await import(Bun.pathToFileURL(ssrFile).href)) as SsrBundle;
  } catch (error) {
    options.log?.(`webpack bundles failed to import: ${error instanceof Error ? error.message : String(error)}`);
    return undefined;
  }
  if (typeof rsc.renderFlight !== "function" || typeof ssr.renderHtml !== "function") return undefined;
  let rootMainFiles: string[] = [];
  try {
    const buildManifest = JSON.parse(readFileSync(join(dist, "build-manifest.json"), "utf8")) as { rootMainFiles?: string[] };
    rootMainFiles = buildManifest.rootMainFiles ?? [];
  } catch {
    rootMainFiles = [];
  }
  const cdn = options.config?.cdn;
  const toUrl = (file: string): string => (cdn ? publicUrl(cdn, file) : `/_yolo/static/${file}`);
  const bootstrapModules = rootMainFiles.filter((f) => f.endsWith(".js")).map(toUrl);
  const stylesheets = rootMainFiles.filter((f) => f.endsWith(".css")).map(toUrl);
  let manifest = options.manifest;
  const getManifest = (input: RscRenderInput): RouteManifest => {
    manifest ??= input.manifest ?? loadRouteManifest(root, { distDir });
    return manifest;
  };
  return {
    name: "webpack-worker",
    details: { rsc: rscFile, ssr: ssrFile, bootstrap: bootstrapModules.join(",") },
    renderFlight(input) {
      return rsc.renderFlight(flightInput(input, getManifest(input)), { signal: input.signal });
    },
    async renderHtml(input) {
      const flight = await rsc.renderFlight(flightInput(input, getManifest(input)), { signal: input.signal });
      return ssr.renderHtml(flight, {
        bootstrapModules: input.bootstrapModules ?? bootstrapModules,
        bootstrapScriptContent: input.bootstrapScriptContent,
        stylesheets: input.stylesheets ?? stylesheets,
        formState: input.formState,
        nonce: input.nonce,
        signal: input.signal,
      });
    },
    async handleAction(request, ctx) {
      if (typeof rsc.handleAction !== "function") return new Response("Server actions are not available", { status: 501 });
      const url = new URL(request.url);
      const m = ctx.manifest ?? manifest ?? loadRouteManifest(root, { distDir });
      const match = ctx.match ?? matchRoute(m, url.pathname);
      return rsc.handleAction(request, { actionId: ctx.actionId, url: url.href, pathname: url.pathname, search: url.search, params: match?.params ?? {}, entry: match?.entry });
    },
  };
}

// ---------------------------------------------------------------------------
// 2b. react-server-dom-webpack worker (no build: `bun --conditions=react-server rsc-worker.ts`)
// ---------------------------------------------------------------------------

/** Whether `react-server-dom-webpack/server.edge` resolves from `root`. */
export function reactServerDomWebpackAvailable(root: string): boolean {
  for (const from of [root, import.meta.dir]) {
    try {
      Bun.resolveSync("react-server-dom-webpack/server.edge", from);
      return true;
    } catch {
      // try the next base directory
    }
  }
  return false;
}

/**
 * Engine rendering the route modules in an RSC worker process (real Flight renderer) and streaming
 * the HTML through `renderFlightToHtml` in this process. Used when no production bundles exist.
 */
export function createWorkerEngine(options: ResolveRscEngineOptions): RscEngine | undefined {
  const root = resolve(options.root);
  if (!reactServerDomWebpackAvailable(root)) return undefined;
  const log = options.log ?? (() => {});
  const distDir = options.config?.distDir ?? ".yolo";
  let manifest = options.manifest;
  const getManifest = (input: RscRenderInput | RscActionContext): RouteManifest => {
    manifest ??= input.manifest ?? loadRouteManifest(root, { distDir });
    return manifest;
  };
  let workerPromise: Promise<RscWorker> | undefined;
  const worker = (): Promise<RscWorker> => {
    workerPromise ??= startRscWorker({ root, env: { NODE_ENV: options.dev === false ? "production" : (Bun.env.NODE_ENV ?? "development") } });
    return workerPromise;
  };
  let inline: RscEngine | undefined;
  const fallback = (error: unknown): RscEngine => {
    if (!inline) {
      log(`webpack-worker engine failed, falling back to inline: ${error instanceof Error ? error.message : String(error)}`);
      inline = createInlineEngine({ root, config: options.config, manifest, log });
    }
    return inline;
  };
  const renderFlight = async (input: RscRenderInput): Promise<ReadableStream<Uint8Array>> => {
    const url = input.url ?? new URL(input.request.url);
    const m = getManifest(input);
    const match = input.match ?? matchRoute(m, url.pathname);
    const entry = match?.entry.kind === "page" ? match.entry : undefined;
    if (!entry) throw new Error(`no page route for ${url.pathname}`);
    const headers: Record<string, string> = {};
    input.request.headers.forEach((value, key) => {
      headers[key] = value;
    });
    const response = await (await worker()).render({
      pathname: url.pathname,
      search: url.search,
      headers,
      routeFile: join(root, entry.file),
      layoutFiles: entry.layouts.map((file) => join(root, file)),
      templateFiles: entry.templates.map((file) => join(root, file)),
      ...(entry.loading ? { loadingFile: join(root, entry.loading) } : {}),
      params: match?.params ?? {},
    });
    if (response.status !== 200 || !response.body) throw new Error(`worker render failed (${response.status}): ${await response.text()}`);
    return response.body;
  };
  return {
    name: "webpack-worker",
    details: { mode: "worker", root },
    async renderFlight(input) {
      try {
        return await renderFlight(input);
      } catch (error) {
        return fallback(error).renderFlight(input);
      }
    },
    async renderHtml(input) {
      try {
        const flightStream = await renderFlight(input);
        return await renderFlightToHtml({
          flightStream,
          ...(input.bootstrapModules ? { bootstrapScripts: input.bootstrapModules } : {}),
          ...(input.bootstrapScriptContent ? { bootstrapScriptContent: input.bootstrapScriptContent } : {}),
          ...(input.nonce ? { nonce: input.nonce } : {}),
          ...(input.signal ? { signal: input.signal } : {}),
          ...(input.metadata ? { headHtml: renderHeadTags(input.metadata, input.viewport) } : {}),
        });
      } catch (error) {
        return fallback(error).renderHtml(input);
      }
    },
    async handleAction(request) {
      return (await worker()).action(request);
    },
  };
}

// ---------------------------------------------------------------------------
// 3. inline fallback (plain React SSR, or a static shell)
// ---------------------------------------------------------------------------

let runtimePluginInstalled = false;

function installRuntimePlugin(root: string, config: ResolvedYoloConfig | undefined): void {
  if (runtimePluginInstalled) return;
  runtimePluginInstalled = true;
  try {
    Bun.plugin(yoloNextPlugin({ root, config, env: "server" }));
  } catch {
    // runtime plugins are best effort
  }
}

type ReactLike = {
  createElement: (type: unknown, props: Record<string, unknown> | null, ...children: unknown[]) => unknown;
  Fragment: unknown;
  Suspense: unknown;
};

type ReactDomServerLike = {
  renderToReadableStream: (element: unknown, options?: Record<string, unknown>) => Promise<ReadableStream<Uint8Array>>;
};

function staticShell(title: string, body: string, metadata: Metadata, viewport?: Viewport): string {
  const head = renderHeadTags({ title, ...metadata }, viewport);
  return `<!DOCTYPE html><html lang="en"><head>${head}</head><body>${body}</body></html>`;
}

async function importModule(root: string, file: string): Promise<Record<string, unknown>> {
  return (await import(Bun.pathToFileURL(join(root, file)).href)) as Record<string, unknown>;
}

async function inlineElement(root: string, react: ReactLike, entry: RouteEntry | undefined, params: Record<string, string>, url: URL, manifest: RouteManifest, extraMetadata: Metadata | undefined, extraViewport: Viewport | undefined): Promise<unknown> {
  const { createElement, Fragment, Suspense } = react;
  const paramsObject: Record<string, string | string[]> = { ...params };
  for (const p of entry?.params ?? []) {
    const value = paramsObject[p.name];
    if (p.catchAll && typeof value === "string") paramsObject[p.name] = value.split("/").filter(Boolean);
  }
  const searchParams: Record<string, string | string[]> = {};
  for (const [key, value] of url.searchParams) {
    const prev = searchParams[key];
    searchParams[key] = prev === undefined ? value : Array.isArray(prev) ? [...prev, value] : [prev, value];
  }
  const props = { params: Promise.resolve(paramsObject), searchParams: Promise.resolve(searchParams) };
  const chain: Metadata[] = [];
  let viewport: Viewport | undefined;
  const layouts = entry ? entry.layouts : manifest.rootLayout ? [manifest.rootLayout] : [];
  const loaded = new Map<string, Record<string, unknown>>();
  const load = async (file: string): Promise<Record<string, unknown>> => {
    let mod = loaded.get(file);
    if (!mod) {
      mod = await importModule(root, file);
      loaded.set(file, mod);
    }
    return mod;
  };
  const collect = async (mod: Record<string, unknown>): Promise<void> => {
    if (mod.metadata && typeof mod.metadata === "object") chain.push(mod.metadata as Metadata);
    if (typeof mod.generateMetadata === "function") chain.push(((await (mod.generateMetadata as (p: unknown, parent: Promise<Metadata>) => Promise<Metadata>)(props, Promise.resolve(resolveMetadataChain(chain)))) ?? {}) as Metadata);
    if (mod.viewport && typeof mod.viewport === "object") viewport = mod.viewport as Viewport;
  };
  for (const layout of layouts) await collect(await load(layout));
  let node: unknown;
  if (entry) {
    const page = await load(entry.file);
    await collect(page);
    if (typeof page.default !== "function") throw new Error(`page module has no default export: ${entry.file}`);
    node = createElement(page.default, props);
    if (entry.loading) {
      const loading = await load(entry.loading);
      if (typeof loading.default === "function") node = createElement(Suspense, { fallback: createElement(loading.default, null) }, node);
    }
  } else {
    const notFound = manifest.rootNotFound ? await load(manifest.rootNotFound) : undefined;
    node = notFound && typeof notFound.default === "function" ? createElement(notFound.default, null) : createElement("main", null, createElement("h1", null, "404"), createElement("p", null, "This page could not be found."));
  }
  if (extraMetadata) chain.push(extraMetadata);
  const head = headElements(resolveMetadataChain(chain), extraViewport ?? viewport).map((el, index) => createElement(el.tag, { key: `yolo-head-${index}`, ...el.attrs }, el.children));
  node = createElement(Fragment, null, ...head, node);
  for (let i = layouts.length - 1; i >= 0; i -= 1) {
    const file = layouts[i];
    if (!file) continue;
    const layout = await load(file);
    if (typeof layout.default !== "function") continue;
    const slots: Record<string, null> = {};
    for (const slot of entry?.parallelSlots ?? []) slots[slot] = null;
    node = createElement(layout.default, { params: props.params, children: node, ...slots });
  }
  return node;
}

/** Creates the inline engine: React SSR of the route modules when React is importable, else a static shell. */
export function createInlineEngine(options: { root: string; config?: ResolvedYoloConfig; manifest?: RouteManifest; log?: (message: string) => void }): RscEngine {
  const root = resolve(options.root);
  let manifest = options.manifest;
  const getManifest = (input: RscRenderInput): RouteManifest => {
    manifest ??= input.manifest ?? loadRouteManifest(root, { distDir: options.config?.distDir });
    return manifest;
  };
  const renderHtml = async (input: RscRenderInput): Promise<ReadableStream<Uint8Array>> => {
    const url = input.url ?? new URL(input.request.url);
    const m = getManifest(input);
    const match = input.match ?? matchRoute(m, url.pathname);
    const entry = match?.entry.kind === "page" ? match.entry : undefined;
    try {
      installRuntimePlugin(root, input.config ?? options.config);
      const react = (await import("react")) as unknown as ReactLike;
      const dom = (await import("react-dom/server")) as unknown as ReactDomServerLike;
      const element = await inlineElement(root, react, entry, match?.params ?? {}, url, m, input.metadata, input.viewport);
      const stream = await dom.renderToReadableStream(element, {
        bootstrapModules: input.bootstrapModules,
        bootstrapScriptContent: input.bootstrapScriptContent,
        nonce: input.nonce,
        signal: input.signal,
        onError(error: unknown) {
          const digest = (error as { digest?: unknown } | null)?.digest;
          if (typeof digest === "string") return digest;
          options.log?.(`inline render error: ${error instanceof Error ? error.message : String(error)}`);
          return undefined;
        },
      });
      return stream;
    } catch (error) {
      options.log?.(`inline engine fell back to a static shell: ${error instanceof Error ? error.message : String(error)}`);
      const title = entry ? `${url.pathname}` : "404";
      const body = entry ? `<main id="__yolo"><p>${Bun.escapeHTML(entry.file)} could not be rendered: ${Bun.escapeHTML(error instanceof Error ? error.message : String(error))}</p></main>` : "<main><h1>404</h1><p>This page could not be found.</p></main>";
      return textStream(staticShell(title, body, input.metadata ?? {}, input.viewport));
    }
  };
  return {
    name: "inline",
    details: { root },
    renderHtml,
    async renderFlight(input) {
      const html = await new Response(await renderHtml(input)).text();
      const bodyMatch = /<body[^>]*>([\s\S]*)<\/body>/i.exec(html);
      const inner = bodyMatch?.[1] ?? html;
      const row = `0:${JSON.stringify(["$", "div", null, { "data-yolo-inline": "1", dangerouslySetInnerHTML: { __html: inner } }])}\n`;
      return textStream(row);
    },
  };
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/**
 * Resolves the RSC engine of a project: `@yolo-rs/fullstack` (package or candidate paths), then the
 * react-server-dom-webpack bundles of `buildYoloNext`, then the inline fallback. Never throws.
 */
export async function resolveRscEngine(options: ResolveRscEngineOptions): Promise<RscEngine> {
  const root = resolve(options.root);
  const log = options.log ?? (() => {});
  const inline = (): RscEngine => createInlineEngine({ root, config: options.config, manifest: options.manifest, log });
  const prefer = options.prefer ?? forcedRscEngine(options.config);
  options = { ...options, prefer };
  if (prefer === "inline") return inline();
  if (prefer === undefined || prefer === "fullstack") {
    try {
      const found = await importFullstack(options);
      if (found) {
        const engine = adaptFullstackModule(found.mod, {
          root,
          outDir: options.fullstackOutDir ?? join(root, options.config?.distDir ?? ".yolo", "fullstack"),
          rebuild: options.rebuild,
          dev: options.dev,
          source: found.source,
          log,
          fallback: inline(),
        });
        if (engine) {
          log(`rsc engine: fullstack (${found.source})`);
          return engine;
        }
      }
    } catch (error) {
      log(`fullstack resolution failed: ${error instanceof Error ? error.message : String(error)}`);
    }
    if (options.prefer === "fullstack") return inline();
  }
  if (options.bundles !== false) {
    try {
      const engine = await createWebpackEngine({ ...options, root, log });
      if (engine) {
        log(`rsc engine: webpack-worker (${engine.details.rsc ?? ""})`);
        return engine;
      }
    } catch (error) {
      log(`webpack engine resolution failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  log("rsc engine: inline");
  return inline();
}
