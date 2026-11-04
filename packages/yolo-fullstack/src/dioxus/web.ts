// SPDX-License-Identifier: MIT OR Apache-2.0
// Serve a `dx build --platform web` output directory from Bun.serve routes.

import { existsSync, statSync } from "node:fs";
import { join, resolve, sep } from "node:path";

export type RouteHandler = (request: Request) => Promise<Response> | Response;

export interface DioxusWebRoutesOptions {
  /** Path to `target/dx/<crate>/{debug|release}/web/public`. */
  dir: string;
  /** URL prefix the app is mounted under; defaults to "/". */
  prefix?: string;
  /** Serve index.html for unknown paths so client-side routing works; defaults to true. */
  spa?: boolean;
}

const IMMUTABLE_CACHE = "public, max-age=31536000, immutable";
const CONTENT_TYPES: Record<string, string> = {
  ".wasm": "application/wasm",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".avif": "image/avif",
  ".ico": "image/x-icon",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".otf": "font/otf",
  ".txt": "text/plain; charset=utf-8",
};

/** Content type for a file path based on its extension. */
export function contentTypeFor(path: string): string {
  const dot = path.lastIndexOf(".");
  const ext = dot === -1 ? "" : path.slice(dot).toLowerCase();
  return CONTENT_TYPES[ext] ?? "application/octet-stream";
}

/** Normalize a mount prefix so it starts and ends with "/". */
export function normalizeWebPrefix(prefix: string | undefined): string {
  let value = prefix ?? "/";
  if (!value.startsWith("/")) value = `/${value}`;
  if (!value.endsWith("/")) value = `${value}/`;
  return value;
}

/** Reject path segments that could escape the public directory. */
export function isUnsafeRelativePath(relative: string): boolean {
  if (relative.length === 0) return true;
  if (relative.includes("\0") || relative.includes("\\")) return true;
  if (relative.startsWith("/")) return true;
  if (/^[a-zA-Z]:/.test(relative)) return true;
  for (const segment of relative.split("/")) {
    if (segment === ".." || segment === "." || segment.length === 0)
      return true;
  }
  return false;
}

function decodeSegment(raw: string): string | null {
  try {
    return decodeURIComponent(raw);
  } catch {
    return null;
  }
}

/** Resolve a request pathname (already stripped of the prefix) to a file under `root`. */
export function resolvePublicFile(
  root: string,
  relative: string,
): string | null {
  const decoded = decodeSegment(relative);
  if (decoded === null || isUnsafeRelativePath(decoded)) return null;
  const absolute = resolve(root, decoded);
  const rootWithSep = root.endsWith(sep) ? root : root + sep;
  if (!absolute.startsWith(rootWithSep)) return null;
  return absolute;
}

function fileResponse(path: string, immutable: boolean): Response {
  const file = Bun.file(path);
  const headers: Record<string, string> = {
    "content-type": contentTypeFor(path),
  };
  headers["cache-control"] = immutable ? IMMUTABLE_CACHE : "no-cache";
  return new Response(file, { headers });
}

async function serveFile(
  path: string | null,
  immutable: boolean,
): Promise<Response> {
  if (path === null) return new Response("Forbidden", { status: 403 });
  const file = Bun.file(path);
  if (!(await file.exists())) return new Response("Not Found", { status: 404 });
  return fileResponse(path, immutable);
}

/** Build Bun.serve `routes` serving index.html, assets/* and wasm/* from a dx web build. */
export function dioxusWebRoutes(
  options: DioxusWebRoutesOptions,
): Record<string, RouteHandler> {
  const root = resolve(options.dir);
  if (!existsSync(root) || !statSync(root).isDirectory()) {
    throw new Error(`dioxusWebRoutes: directory not found: ${root}`);
  }
  const indexPath = join(root, "index.html");
  if (!existsSync(indexPath)) {
    throw new Error(`dioxusWebRoutes: index.html not found in ${root}`);
  }
  const prefix = normalizeWebPrefix(options.prefix);
  const spa = options.spa ?? true;

  const serveIndex: RouteHandler = () =>
    new Response(Bun.file(indexPath), {
      headers: {
        "content-type": CONTENT_TYPES[".html"]!,
        "cache-control": "no-cache",
      },
    });

  const hashedDir =
    (folder: "assets" | "wasm"): RouteHandler =>
    (request) => {
      const pathname = new URL(request.url).pathname;
      const base = `${prefix}${folder}/`;
      if (!pathname.startsWith(base))
        return new Response("Not Found", { status: 404 });
      const relative = `${folder}/${pathname.slice(base.length)}`;
      return serveFile(resolvePublicFile(root, relative), true);
    };

  const routes: Record<string, RouteHandler> = {
    [prefix]: serveIndex,
    [`${prefix}index.html`]: serveIndex,
    [`${prefix}assets/*`]: hashedDir("assets"),
    [`${prefix}wasm/*`]: hashedDir("wasm"),
  };
  if (spa) routes[`${prefix}*`] = serveIndex;
  return routes;
}
