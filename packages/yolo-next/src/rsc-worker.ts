// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/rsc-worker: React Server Components render worker. Started by `startRscWorker` as
// `bun --conditions=react-server rsc-worker.ts`; serves `POST /render` (Flight) and `POST /action`.

import { createRequestStore, mergeStoreHeaders, runAfterCallbacks, runWithRequestStore } from "./headers.ts";
import {
  buildRouteElement,
  isReactServerEnvironment,
  renderToFlight,
  type ClientManifest,
  type RouteModule,
  type RscRenderRequest,
} from "./rsc.ts";
import { RSC_HEADERS } from "./types.ts";

type ActionsModule = typeof import("./actions.ts");

/** Router control-flow signal extracted from an error digest (`NEXT_REDIRECT;...` / `NEXT_HTTP_ERROR_FALLBACK;...`). */
export interface RouterSignal {
  readonly kind: "redirect" | "status";
  readonly status: number;
  readonly url?: string;
  readonly type?: string;
}

/** Parses Next.js router digests without importing `navigation.ts` (which needs client React). */
export function parseRouterSignal(error: unknown): RouterSignal | undefined {
  if (!error || typeof error !== "object") {
    return undefined;
  }
  const digest = (error as { digest?: unknown }).digest;
  if (typeof digest !== "string") {
    return undefined;
  }
  if (digest.startsWith("NEXT_REDIRECT;")) {
    const parts = digest.split(";");
    const status = Number(parts[parts.length - 2]);
    return { kind: "redirect", status: status === 308 ? 308 : 307, url: parts.slice(2, -2).join(";"), type: parts[1] ?? "push" };
  }
  if (digest.startsWith("NEXT_HTTP_ERROR_FALLBACK;")) {
    const status = Number(digest.split(";")[1]);
    return { kind: "status", status: status === 401 || status === 403 ? status : 404 };
  }
  return undefined;
}

/** Loads `.yolo/client-manifest.json` into a `ClientManifest` (accepts `clientModules` or flat maps). */
export async function loadClientManifestFile(root: string): Promise<ClientManifest> {
  const file = Bun.file(`${root.replace(/[\\/]+$/, "")}/.yolo/client-manifest.json`);
  if (!(await file.exists())) {
    return {};
  }
  try {
    const json = (await file.json()) as Record<string, unknown> | Array<Record<string, unknown>>;
    const manifest: ClientManifest = {};
    const source = Array.isArray(json)
      ? Object.fromEntries(
          json.map((entry) => [
            `${String(entry.module)}#${String(entry.exportName ?? "default")}`,
            { id: entry.id ?? entry.module, name: entry.exportName ?? "default", chunks: entry.chunks ?? [], async: entry.async ?? false },
          ]),
        )
      : ((json.clientModules as Record<string, unknown> | undefined) ?? json);
    for (const [key, value] of Object.entries(source)) {
      if (value && typeof value === "object") {
        const entry = value as { id?: unknown; name?: unknown; chunks?: unknown; async?: unknown };
        manifest[key] = {
          id: String(entry.id ?? key.split("#")[0]),
          name: String(entry.name ?? key.split("#")[1] ?? "default"),
          chunks: Array.isArray(entry.chunks) ? entry.chunks.map(String) : [],
          async: entry.async === true,
        };
      }
    }
    return manifest;
  } catch (error) {
    console.error("[rsc-worker] invalid client-manifest.json:", error);
    return {};
  }
}

const moduleCache = new Map<string, Promise<RouteModule>>();

/** Imports a route module by absolute path (dev: cache-busted by mtime). */
export async function importRouteModule(file: string, dev: boolean): Promise<RouteModule> {
  if (!dev) {
    let cached = moduleCache.get(file);
    if (!cached) {
      cached = import(file) as Promise<RouteModule>;
      moduleCache.set(file, cached);
    }
    return cached;
  }
  const mtime = Bun.file(file).lastModified;
  try {
    return (await import(`${file}?v=${mtime}`)) as RouteModule;
  } catch {
    return (await import(file)) as RouteModule;
  }
}

/** Renders one route request to a buffered Flight response with `x-yolo-*` status headers. */
export async function renderRequest(req: RscRenderRequest, manifest: ClientManifest, dev: boolean): Promise<Response> {
  const host = req.headers.host ?? req.headers["x-forwarded-host"] ?? "localhost";
  const protocol = req.headers["x-forwarded-proto"] ?? "http";
  const url = new URL(`${req.pathname}${req.search}`, `${protocol}://${host}`);
  const request = new Request(url, { headers: req.headers });
  const store = createRequestStore(request, { params: req.params, phase: "render", isPrerender: req.isPrerender ?? false });
  const page = await importRouteModule(req.routeFile, dev);
  const layouts = await Promise.all(req.layoutFiles.map((file) => importRouteModule(file, dev)));
  const templates = await Promise.all((req.templateFiles ?? []).map((file) => importRouteModule(file, dev)));
  const loading = req.loadingFile ? await importRouteModule(req.loadingFile, dev) : undefined;
  const searchParams: Record<string, string | string[]> = {};
  for (const [key, value] of url.searchParams) {
    const existing = searchParams[key];
    searchParams[key] = existing === undefined ? value : [...(Array.isArray(existing) ? existing : [existing]), value];
  }
  const element = buildRouteElement({
    page,
    layouts,
    templates,
    ...(loading ? { loading } : {}),
    params: req.params,
    searchParams,
  });
  let signal: RouterSignal | undefined;
  const text = await runWithRequestStore(store, async () => {
    const stream = await renderToFlight({
      element,
      clientManifest: manifest,
      onError(error) {
        const parsed = parseRouterSignal(error);
        if (parsed) {
          signal ??= parsed;
        } else {
          console.error("[rsc-worker] render error:", error);
        }
        const digest = (error as { digest?: unknown })?.digest;
        return typeof digest === "string" ? digest : undefined;
      },
    });
    return new Response(stream).text();
  });
  await runAfterCallbacks(store);
  const headers = new Headers({ "content-type": RSC_HEADERS.contentType });
  if (store.usedDynamic) {
    headers.set("x-yolo-dynamic", "1");
  }
  if (signal?.kind === "redirect" && signal.url) {
    headers.set("x-yolo-redirect", signal.url);
    headers.set("x-yolo-status", String(signal.status));
    headers.set("x-yolo-redirect-type", signal.type ?? "push");
  } else if (signal) {
    headers.set("x-yolo-status", String(signal.status));
  }
  mergeStoreHeaders(store, headers);
  return new Response(text, { status: 200, headers });
}

async function main(): Promise<void> {
  const root = Bun.env.YOLO_RSC_ROOT ?? process.cwd();
  const dev = Bun.env.NODE_ENV !== "production";
  if (!isReactServerEnvironment()) {
    console.error("[rsc-worker] started without --conditions=react-server; falling back to the inline encoder.");
  }
  const manifest = await loadClientManifestFile(root);
  let actions: ActionsModule | undefined;
  let actionsError: unknown;
  try {
    actions = (await import("./actions.ts")) as ActionsModule;
  } catch (error) {
    actionsError = error;
  }
  const registry = actions ? await actions.loadActionManifest(root) : undefined;
  const allowedOrigins = (Bun.env.YOLO_ACTION_ALLOWED_ORIGINS ?? "").split(",").map((s) => s.trim()).filter(Boolean);
  const bodySizeLimit = Number(Bun.env.YOLO_ACTION_BODY_LIMIT ?? "1048576");
  const server = Bun.serve({
    port: Number(Bun.env.YOLO_RSC_PORT ?? "0"),
    hostname: "127.0.0.1",
    idleTimeout: 60,
    async fetch(request) {
      const url = new URL(request.url);
      if (url.pathname === "/health") {
        return Response.json({ ok: true, reactServer: isReactServerEnvironment(), actions: actions !== undefined });
      }
      if (url.pathname === "/render" && request.method === "POST") {
        try {
          const body = (await request.json()) as RscRenderRequest;
          return await renderRequest(body, manifest, dev);
        } catch (error) {
          console.error("[rsc-worker] render failed:", error);
          const message = error instanceof Error ? error.message : String(error);
          return new Response(message, { status: 500, headers: { "x-yolo-status": "500" } });
        }
      }
      if (url.pathname === "/action" && request.method === "POST") {
        if (!actions || !registry) {
          const reason = actionsError instanceof Error ? actionsError.message : "actions module unavailable";
          return new Response(`Server Actions unavailable in the react-server worker: ${reason}`, { status: 501 });
        }
        return actions.handleServerAction(request, { registry, allowedOrigins, bodySizeLimit });
      }
      return new Response("Not Found", { status: 404 });
    },
  });
  const port = server.port ?? 0;
  const send = (process as { send?: (message: unknown) => void }).send;
  if (typeof send === "function") {
    send.call(process, { port });
  } else {
    console.log(`YOLO_RSC_PORT=${port}`);
  }
  process.on("disconnect", () => {
    server.stop(true);
    process.exit(0);
  });
}

if (import.meta.main) {
  await main();
}
