// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/cdn: public URLs with skew protection, Cache-Control policy, MIME table, CDN asset
// manifest, S3/R2 upload with Bun.S3Client, precompressed variants and a static asset responder.

import type { S3Client } from "bun";
import { existsSync, statSync } from "node:fs";
import { basename, extname, join, resolve } from "node:path";

import type { CdnAssetManifestEntry, CdnPlan } from "./types.ts";

const toPosix = (path: string): string => path.replace(/\\/g, "/");

const CONTENT_TYPES: Readonly<Record<string, string>> = {
  js: "text/javascript",
  mjs: "text/javascript",
  cjs: "text/javascript",
  css: "text/css",
  html: "text/html",
  htm: "text/html",
  json: "application/json",
  map: "application/json",
  svg: "image/svg+xml",
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  avif: "image/avif",
  ico: "image/x-icon",
  woff: "font/woff",
  woff2: "font/woff2",
  ttf: "font/ttf",
  otf: "font/otf",
  wasm: "application/wasm",
  txt: "text/plain",
  xml: "application/xml",
  webmanifest: "application/manifest+json",
  mp4: "video/mp4",
  webm: "video/webm",
  mp3: "audio/mpeg",
  pdf: "application/pdf",
  rsc: "text/x-component",
};

const TEXT_TYPES = new Set(["js", "mjs", "cjs", "css", "html", "htm", "json", "map", "svg", "txt", "xml", "webmanifest", "rsc"]);
const COMPRESSIBLE = new Set(["js", "mjs", "cjs", "css", "html", "htm", "json", "map", "svg", "txt", "xml", "webmanifest", "wasm", "rsc"]);

function extensionOf(path: string): string {
  return extname(path).slice(1).toLowerCase();
}

/** MIME type from the extension (`application/octet-stream` when unknown). */
export function contentTypeFor(path: string): string {
  return CONTENT_TYPES[extensionOf(path)] ?? "application/octet-stream";
}

/** Content type with `; charset=utf-8` for text types. */
export function contentTypeWithCharset(path: string): string {
  const type = contentTypeFor(path);
  return TEXT_TYPES.has(extensionOf(path)) ? `${type}; charset=utf-8` : type;
}

/** True when the file name carries a content hash (`name-<hash>.ext`, `chunk-<hash>.js`, `name.<hash>.ext`). */
export function isHashedAsset(path: string): boolean {
  const name = basename(toPosix(path));
  // Bun hashes are 8+ base36 chars; require a digit so plain words (`my-component.js`) do not count.
  return /[-.](?=[0-9a-z]*\d)[0-9a-z]{8,}(?:\.[a-z0-9]+)+$/i.test(name) || /^chunk-[0-9a-z]{6,}\.[a-z0-9]+$/i.test(name);
}

/** Appends the skew protection query (`?dpl=<deploymentId>`) when the plan has a deployment id. */
export function withSkew(plan: CdnPlan, url: string): string {
  if (!plan.deploymentId) return url;
  const [base, hash] = url.split("#", 2);
  const separator = (base ?? "").includes("?") ? "&" : "?";
  return `${base}${separator}${plan.skewQuery}=${encodeURIComponent(plan.deploymentId)}${hash !== undefined ? `#${hash}` : ""}`;
}

/** Public URL of an output file relative to the static dir: asset prefix + static route + path + skew query. */
export function publicUrl(plan: CdnPlan, path: string): string {
  const rel = toPosix(path).replace(/^\/+/, "");
  const prefix = plan.assetPrefix.replace(/\/+$/, "");
  const route = plan.staticRoute.startsWith("/") ? plan.staticRoute : `/${plan.staticRoute}`;
  return withSkew(plan, `${prefix}${route}/${rel}`);
}

/** `Cache-Control` for a served path: hashed assets immutable, HTML dynamic, everything else public. */
export function cacheControlFor(plan: CdnPlan, path: string): string {
  const ext = extensionOf(path);
  if (ext === "html" || ext === "htm") return plan.dynamicHtmlCacheControl;
  if (isHashedAsset(path)) return plan.immutableCacheControl;
  return plan.publicCacheControl;
}

/** Builds the CDN asset manifest of an output directory (precompressed `.gz`/`.zst` variants excluded). */
export async function buildCdnManifest(plan: CdnPlan, outDir: string): Promise<CdnAssetManifestEntry[]> {
  const root = resolve(outDir);
  const entries: CdnAssetManifestEntry[] = [];
  const glob = new Bun.Glob("**/*");
  for await (const file of glob.scan({ cwd: root, onlyFiles: true, dot: false })) {
    const rel = toPosix(file);
    if (rel.endsWith(".gz") || rel.endsWith(".zst") || rel.endsWith(".br")) continue;
    const size = statSync(join(root, file)).size;
    entries.push({
      path: rel,
      publicUrl: publicUrl(plan, rel),
      contentType: contentTypeFor(rel),
      cacheControl: cacheControlFor(plan, rel),
      hashed: isHashedAsset(rel),
      size,
    });
  }
  entries.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
  return entries;
}

/** Options of `uploadToCdn`. */
export interface UploadOptions {
  /** Pre-configured client (default: `new Bun.S3Client({ bucket, endpoint, region })` from `plan.upload`). */
  readonly client?: S3Client;
  /** Local directory the manifest paths are relative to (default cwd). */
  readonly outDir?: string;
  /** Log the keys instead of uploading. */
  readonly dryRun?: boolean;
  /** Parallel uploads (default 8). */
  readonly concurrency?: number;
  readonly onUpload?: (key: string, entry: CdnAssetManifestEntry) => void;
}

/** Result of `uploadToCdn`. */
export interface UploadResult {
  readonly keys: string[];
  readonly bytes: number;
  readonly dryRun: boolean;
}

/** Uploads the manifest entries to the CDN bucket (`Bun.S3Client.write`) with bounded concurrency. */
export async function uploadToCdn(plan: CdnPlan, manifest: ReadonlyArray<CdnAssetManifestEntry>, options: UploadOptions = {}): Promise<UploadResult> {
  const upload = plan.upload;
  const prefix = (upload?.prefix ?? plan.staticRoute.replace(/^\/+/, "")).replace(/^\/+|\/+$/g, "");
  const keyOf = (entry: CdnAssetManifestEntry): string => (prefix ? `${prefix}/${entry.path}` : entry.path);
  const keys = manifest.map(keyOf);
  const bytes = manifest.reduce((sum, entry) => sum + entry.size, 0);
  if (options.dryRun) return { keys, bytes, dryRun: true };
  let client = options.client;
  if (!client) {
    if (!upload) throw new Error("uploadToCdn: the CDN plan has no upload target and no client was given");
    client = new Bun.S3Client({ bucket: upload.bucket, endpoint: upload.endpoint, region: upload.region });
  }
  const outDir = resolve(options.outDir ?? process.cwd());
  const concurrency = Math.max(1, options.concurrency ?? 8);
  let index = 0;
  const worker = async (): Promise<void> => {
    for (;;) {
      const i = index;
      index += 1;
      const entry = manifest[i];
      if (!entry) return;
      const key = keyOf(entry);
      await client.write(key, Bun.file(join(outDir, entry.path)), { type: entry.contentType });
      options.onUpload?.(key, entry);
    }
  };
  await Promise.all(Array.from({ length: Math.min(concurrency, manifest.length) }, worker));
  return { keys, bytes, dryRun: false };
}

/** Writes `.gz` (gzip level 9) and `.zst` (zstd level 19) variants next to a text asset; returns the written paths. */
export async function compressVariants(path: string, options: { minSize?: number } = {}): Promise<string[]> {
  const ext = extensionOf(path);
  if (!COMPRESSIBLE.has(ext)) return [];
  const file = Bun.file(path);
  const bytes = new Uint8Array(await file.arrayBuffer());
  if (bytes.byteLength < (options.minSize ?? 0)) return [];
  const written: string[] = [];
  const gz = Bun.gzipSync(bytes, { level: 9 });
  await Bun.write(`${path}.gz`, gz);
  written.push(`${path}.gz`);
  const zstd = (Bun as unknown as { zstdCompressSync?: (data: Uint8Array, options?: { level?: number }) => Uint8Array }).zstdCompressSync;
  if (typeof zstd === "function") {
    await Bun.write(`${path}.zst`, zstd(bytes, { level: 19 }));
    written.push(`${path}.zst`);
  }
  return written;
}

/** Weak ETag of a file from its size and mtime. */
export function fileEtag(size: number, mtimeMs: number): string {
  return `W/"${size.toString(16)}-${Math.floor(mtimeMs).toString(16)}"`;
}

function etagMatches(header: string | null, etag: string): boolean {
  if (!header) return false;
  if (header.trim() === "*") return true;
  const strip = (value: string): string => value.trim().replace(/^W\//, "");
  const wanted = strip(etag);
  return header.split(",").some((candidate) => strip(candidate) === wanted);
}

function acceptsEncoding(request: Request, encoding: string): boolean {
  const header = request.headers.get("accept-encoding");
  if (!header) return false;
  for (const part of header.split(",")) {
    const [name, ...params] = part.trim().split(";");
    if (name?.trim() !== encoding && name?.trim() !== "*") continue;
    const q = params.map((p) => p.trim()).find((p) => p.startsWith("q="));
    if (q && Number(q.slice(2)) === 0) return false;
    return true;
  }
  return false;
}

/**
 * Serves a static asset: `ETag` + `If-None-Match` 304, precompressed `.zst`/`.gz` negotiation,
 * `Cache-Control` from the plan, `Content-Type`, `Vary: Accept-Encoding`, `x-deployment-id`.
 */
export function serveStaticAsset(plan: CdnPlan, filePath: string, request: Request): Response {
  let stat;
  try {
    stat = statSync(filePath);
  } catch {
    return new Response("Not Found", { status: 404 });
  }
  if (!stat.isFile()) return new Response("Not Found", { status: 404 });
  const etag = fileEtag(stat.size, stat.mtimeMs);
  const headers = new Headers({
    "cache-control": cacheControlFor(plan, filePath),
    "content-type": contentTypeWithCharset(filePath),
    etag,
    "last-modified": new Date(stat.mtimeMs).toUTCString(),
    "accept-ranges": "bytes",
  });
  if (plan.deploymentId) headers.set(plan.skewHeader, plan.deploymentId);
  const compressible = COMPRESSIBLE.has(extensionOf(filePath));
  if (compressible) headers.set("vary", "Accept-Encoding");
  if (etagMatches(request.headers.get("if-none-match"), etag)) return new Response(null, { status: 304, headers });
  let body = filePath;
  if (compressible && plan.compress) {
    if (acceptsEncoding(request, "zstd") && existsSync(`${filePath}.zst`)) {
      body = `${filePath}.zst`;
      headers.set("content-encoding", "zstd");
    } else if (acceptsEncoding(request, "gzip") && existsSync(`${filePath}.gz`)) {
      body = `${filePath}.gz`;
      headers.set("content-encoding", "gzip");
    }
  }
  const file = Bun.file(body);
  headers.set("content-length", String(body === filePath ? stat.size : file.size));
  if (request.method === "HEAD") return new Response(null, { status: 200, headers });
  return new Response(file, { status: 200, headers });
}
