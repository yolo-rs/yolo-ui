// SPDX-License-Identifier: Apache-2.0
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, utimesSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import {
  buildCdnManifest,
  cacheControlFor,
  compressVariants,
  contentTypeFor,
  contentTypeWithCharset,
  fileEtag,
  isHashedAsset,
  publicUrl,
  serveStaticAsset,
  uploadToCdn,
  withSkew,
} from "../src/cdn.ts";
import type { CdnPlan } from "../src/types.ts";
import { CDN_DEFAULTS } from "../src/types.ts";
import { removeDir, scratchDir } from "./helpers.ts";

const plan: CdnPlan = { ...CDN_DEFAULTS, assetPrefix: "https://cdn.example.com", deploymentId: "dpl_123" };

describe("urls and policy", () => {
  test("publicUrl with prefix and skew query", () => {
    expect(publicUrl(plan, "chunks/app-abc123.js")).toBe("https://cdn.example.com/_yolo/static/chunks/app-abc123.js?dpl=dpl_123");
    expect(publicUrl({ ...CDN_DEFAULTS }, "/chunks/app.js")).toBe("/_yolo/static/chunks/app.js");
    expect(publicUrl({ ...CDN_DEFAULTS, basePath: "/docs", staticRoute: "/docs/_yolo/static" }, "x.css")).toBe("/docs/_yolo/static/x.css");
    expect(withSkew(plan, "/a?b=1#h")).toBe("/a?b=1&dpl=dpl_123#h");
    expect(withSkew(CDN_DEFAULTS, "/a")).toBe("/a");
  });

  test("cacheControlFor table", () => {
    expect(cacheControlFor(plan, "chunks/entry-client-q1nve29d.js")).toBe("public, max-age=31536000, immutable");
    expect(cacheControlFor(plan, "chunks/chunk-9f8e7d6c.js")).toBe("public, max-age=31536000, immutable");
    expect(cacheControlFor(plan, "media/hero-0123456789ab.png")).toBe("public, max-age=31536000, immutable");
    expect(cacheControlFor(plan, "index.html")).toBe("private, no-cache, no-store, max-age=0, must-revalidate");
    expect(cacheControlFor(plan, "robots.txt")).toBe("public, max-age=0");
    expect(cacheControlFor(plan, "logo.svg")).toBe("public, max-age=0");
    expect(isHashedAsset("app.js")).toBe(false);
    expect(isHashedAsset("styles.1a2b3c4d.css")).toBe(true);
  });

  test("contentTypeFor table", () => {
    const table: Array<[string, string]> = [
      ["a.js", "text/javascript"],
      ["a.mjs", "text/javascript"],
      ["a.css", "text/css"],
      ["a.html", "text/html"],
      ["a.json", "application/json"],
      ["a.js.map", "application/json"],
      ["a.svg", "image/svg+xml"],
      ["a.png", "image/png"],
      ["a.jpg", "image/jpeg"],
      ["a.gif", "image/gif"],
      ["a.webp", "image/webp"],
      ["a.avif", "image/avif"],
      ["a.ico", "image/x-icon"],
      ["a.woff", "font/woff"],
      ["a.woff2", "font/woff2"],
      ["a.ttf", "font/ttf"],
      ["a.otf", "font/otf"],
      ["a.wasm", "application/wasm"],
      ["a.txt", "text/plain"],
      ["a.xml", "application/xml"],
      ["a.webmanifest", "application/manifest+json"],
      ["a.mp4", "video/mp4"],
      ["a.webm", "video/webm"],
      ["a.unknown", "application/octet-stream"],
      ["noext", "application/octet-stream"],
    ];
    for (const [path, type] of table) expect(contentTypeFor(path)).toBe(type);
    expect(contentTypeWithCharset("a.js")).toBe("text/javascript; charset=utf-8");
    expect(contentTypeWithCharset("a.png")).toBe("image/png");
  });
});

describe("files", () => {
  let dir: string;
  beforeAll(() => {
    dir = scratchDir("cdn");
    mkdirSync(join(dir, "chunks"), { recursive: true });
    writeFileSync(join(dir, "chunks", "app-abcdef12.js"), "console.log('hello world hello world hello world');\n".repeat(20));
    writeFileSync(join(dir, "index.html"), "<!doctype html><title>x</title>");
    writeFileSync(join(dir, "logo.png"), new Uint8Array([137, 80, 78, 71]));
  });
  afterAll(() => removeDir(dir));

  test("compressVariants writes .gz and .zst next to text assets", async () => {
    const js = join(dir, "chunks", "app-abcdef12.js");
    const written = await compressVariants(js);
    expect(written.map((p) => p.slice(js.length))).toEqual([".gz", ".zst"]);
    expect(existsSync(`${js}.gz`)).toBe(true);
    expect(existsSync(`${js}.zst`)).toBe(true);
    expect(Bun.file(`${js}.gz`).size).toBeLessThan(Bun.file(js).size);
    expect(await compressVariants(join(dir, "logo.png"))).toEqual([]);
  });

  test("buildCdnManifest lists assets without the precompressed variants", async () => {
    const manifest = await buildCdnManifest(plan, dir);
    expect(manifest.map((e) => e.path)).toEqual(["chunks/app-abcdef12.js", "index.html", "logo.png"]);
    const js = manifest[0];
    expect(js?.publicUrl).toBe("https://cdn.example.com/_yolo/static/chunks/app-abcdef12.js?dpl=dpl_123");
    expect(js?.contentType).toBe("text/javascript");
    expect(js?.cacheControl).toBe("public, max-age=31536000, immutable");
    expect(js?.hashed).toBe(true);
    expect(js?.size).toBeGreaterThan(0);
    expect(manifest[1]?.cacheControl).toBe(plan.dynamicHtmlCacheControl);
  });

  test("uploadToCdn dry run and fake client", async () => {
    const manifest = await buildCdnManifest({ ...plan, upload: { provider: "r2", bucket: "assets", prefix: "_yolo/static", publicBaseUrl: "https://cdn.example.com" } }, dir);
    const dry = await uploadToCdn({ ...plan, upload: { provider: "r2", bucket: "assets", prefix: "_yolo/static", publicBaseUrl: "https://cdn.example.com" } }, manifest, { dryRun: true, outDir: dir });
    expect(dry.dryRun).toBe(true);
    expect(dry.keys).toEqual(["_yolo/static/chunks/app-abcdef12.js", "_yolo/static/index.html", "_yolo/static/logo.png"]);
    const writes: Array<[string, string | undefined]> = [];
    const client = {
      write: async (key: string, _data: unknown, options?: { type?: string }) => {
        writes.push([key, options?.type]);
        return 1;
      },
    };
    const result = await uploadToCdn(plan, manifest, { client: client as never, outDir: dir, concurrency: 2 });
    expect(result.dryRun).toBe(false);
    expect(writes.map(([key]) => key).sort()).toEqual(["_yolo/static/chunks/app-abcdef12.js", "_yolo/static/index.html", "_yolo/static/logo.png"]);
    expect(writes.find(([key]) => key.endsWith(".js"))?.[1]).toBe("text/javascript");
  });

  test("serveStaticAsset: headers, 304, encodings, HEAD and 404", async () => {
    const js = join(dir, "chunks", "app-abcdef12.js");
    utimesSync(js, new Date("2026-01-01T00:00:00Z"), new Date("2026-01-01T00:00:00Z"));
    const first = serveStaticAsset(plan, js, new Request("http://localhost/_yolo/static/chunks/app-abcdef12.js"));
    expect(first.status).toBe(200);
    expect(first.headers.get("content-type")).toBe("text/javascript; charset=utf-8");
    expect(first.headers.get("cache-control")).toBe("public, max-age=31536000, immutable");
    expect(first.headers.get("x-deployment-id")).toBe("dpl_123");
    expect(first.headers.get("vary")).toBe("Accept-Encoding");
    const etag = first.headers.get("etag") ?? "";
    expect(etag).toMatch(/^W\/"[0-9a-f]+-[0-9a-f]+"$/);
    expect(etag).toBe(fileEtag(Bun.file(js).size, new Date("2026-01-01T00:00:00Z").getTime()));
    expect((await first.text()).startsWith("console.log")).toBe(true);
    const cached = serveStaticAsset(plan, js, new Request("http://localhost/x", { headers: { "if-none-match": etag } }));
    expect(cached.status).toBe(304);
    expect(cached.headers.get("etag")).toBe(etag);
    const zstd = serveStaticAsset(plan, js, new Request("http://localhost/x", { headers: { "accept-encoding": "gzip, zstd, br" } }));
    expect(zstd.headers.get("content-encoding")).toBe("zstd");
    const gzip = serveStaticAsset(plan, js, new Request("http://localhost/x", { headers: { "accept-encoding": "gzip;q=1.0, br" } }));
    expect(gzip.headers.get("content-encoding")).toBe("gzip");
    expect(Number(gzip.headers.get("content-length"))).toBe(Bun.file(`${js}.gz`).size);
    const identity = serveStaticAsset(plan, js, new Request("http://localhost/x", { headers: { "accept-encoding": "br" } }));
    expect(identity.headers.get("content-encoding")).toBeNull();
    const head = serveStaticAsset(plan, js, new Request("http://localhost/x", { method: "HEAD" }));
    expect(head.status).toBe(200);
    expect(head.body).toBeNull();
    expect(serveStaticAsset(plan, join(dir, "missing.js"), new Request("http://localhost/x")).status).toBe(404);
    const html = serveStaticAsset(plan, join(dir, "index.html"), new Request("http://localhost/x"));
    expect(html.headers.get("cache-control")).toBe(plan.dynamicHtmlCacheControl);
    expect(html.headers.get("content-type")).toBe("text/html; charset=utf-8");
  });
});
