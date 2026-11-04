// SPDX-License-Identifier: MIT OR Apache-2.0
// End to end: the fixture app (next.config with rewrites, redirects, headers, env, turbopack aliases and
// rules, a local next/font, next/image, a route handler) built by the three Bun.build passes, prerendered
// and served by the host.
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { buildApp, type AppBuildResult } from "../src/app/build";
import { loadApp, serveApp, type AppHandler } from "../src/app/host";
import { prerenderApp } from "../src/app/prerender";

const root = join(import.meta.dir, "..", "fixtures", "app-basic");
let outDir: string;
let build: AppBuildResult;
let app: AppHandler;
let server: Bun.Server<undefined>;
const url = (path: string) => `${server.url.origin}${path}`;

beforeAll(async () => {
  outDir = mkdtempSync(join(tmpdir(), "yolo-e2e-"));
  rmSync(join(root, ".yolo"), { recursive: true, force: true });
  build = await buildApp({ root, outDir, buildId: "test", offline: true });
  await prerenderApp(outDir);
  app = await loadApp(outDir);
  server = await serveApp({ outDir, port: 0 });
}, 120_000);

afterAll(async () => {
  await server?.stop(true);
  rmSync(outDir, { recursive: true, force: true });
  rmSync(join(root, ".yolo"), { recursive: true, force: true });
});

describe("build", () => {
  test("manifest carries the config", () => {
    const { manifest } = build;
    expect(manifest.version).toBe(2);
    expect(manifest.rules.redirects).toHaveLength(1);
    expect(manifest.rules.rewrites.beforeFiles[0]?.destination).toBe("/about");
    expect(manifest.images.deviceSizes).toEqual([640, 1200]);
    expect(manifest.unsupported.map(u => u.key)).toContain("webpack");
    expect(manifest.routes.map(r => r.pattern)).toEqual(
      expect.arrayContaining(["/", "/about", "/blog/[slug]", "/api/hello"]),
    );
  });

  test("font stylesheet and file are emitted", () => {
    expect(build.manifest.css.length).toBeGreaterThan(0);
    expect(build.manifest.preloads.length).toBe(1);
    expect(existsSync(join(outDir, "client", build.manifest.preloads[0]!))).toBe(true);
  });
});

describe("serve", () => {
  test("home page: env define, turbopack alias and rule, font class, image srcset", async () => {
    const response = await fetch(url("/"));
    expect(response.status).toBe(200);
    expect(response.headers.get("x-frame-options")).toBe("DENY");
    expect(response.headers.get("x-powered-by")).toBe("@yolo-rs/fullstack");
    const html = await response.text();
    expect(html).toMatch(/Home (<!-- -->)?fixture/);
    expect(html).toContain('data-alias="alias-ok"');
    expect(html).toContain('data-widget="widget-ok"');
    expect(html).toContain('data-note="hello from raw"');
    expect(html).toMatch(/class="yf_[a-z0-9]+ yfv_[a-z0-9]+"/);
    expect(html).toContain('rel="preload"');
    expect(html).toContain("/_next/image?url=%2Fphoto.png&amp;w=640&amp;q=75");
    expect(html).toContain("/_next/image?url=%2Fphoto.png&amp;w=1200&amp;q=75");
  });

  test("redirect, before and after rewrites", async () => {
    const redirect = await fetch(url("/old"), { redirect: "manual" });
    expect(redirect.status).toBe(308);
    expect(redirect.headers.get("location")).toBe("/about");
    const before = await fetch(url("/rewritten"));
    expect(await before.text()).toContain("About page");
    const after = await fetch(url("/legacy/hello"));
    expect(after.status).toBe(200);
    expect(await after.text()).toMatch(/Post (<!-- -->)?hello/);
  });

  test("route handler, static file and the image endpoint", async () => {
    expect(await (await fetch(url("/api/hello"))).json()).toEqual({ hello: "world" });
    expect((await fetch(url("/photo.png"))).status).toBe(200);
    const image = await fetch(url("/_next/image?url=%2Fphoto.png&w=640&q=75"));
    expect(image.status).toBe(200);
    expect(image.headers.get("cache-control")).toContain("public");
    expect((await fetch(url("/_next/image?url=%2Fphoto.png&w=999&q=75"))).status).toBe(400);
  });

  test("404 and the stylesheet", async () => {
    expect((await fetch(url("/nope"))).status).toBe(404);
    const css = await fetch(url(`/_yolo/${build.manifest.css[0]}`));
    expect(css.status).toBe(200);
    expect(await css.text()).toContain("@font-face");
  });

  test("render() exposes the prerender decision", async () => {
    const rendered = await app.render(new Request(url("/about")), { prerender: true, skipMiddleware: true });
    expect(rendered.dynamic).toBe(false);
  });
});
