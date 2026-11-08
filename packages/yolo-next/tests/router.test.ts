// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  appPathsManifestJson,
  classifySegment,
  compareUrlPatterns,
  generateRouteTree,
  loadRouteManifest,
  matchRoute,
  routeRegexes,
  routesManifestJson,
  scanAppDir,
  sortEntries,
  toBunPattern,
  toDioxusRoute,
  toDioxusVariant,
  toLeptosPath,
  toTanStackPath,
} from "../src/router.ts";
import type { RouteEntry } from "../src/types.ts";

const FIXTURE = join(import.meta.dir, "..", "..", "..", "apps", "next-fixture");
const manifest = scanAppDir(FIXTURE);
const byFile = (file: string): RouteEntry => {
  const entry = manifest.entries.find((e) => e.file === file);
  if (!entry) throw new Error(`missing entry ${file}`);
  return entry;
};

describe("scanAppDir on the fixture", () => {
  test("url patterns, kinds and counts", () => {
    const patterns = manifest.entries.map((e) => e.urlPattern);
    for (const expected of ["/", "/pricing", "/photos/[id]", "/sitemap.xml", "/robots.txt", "/manifest.webmanifest", "/opengraph-image", "/legacy", "/api/legacy", "/api/health", "/api/posts/[id]", "/blog/[slug]", "/docs/[...slug]", "/shop/[[...filters]]", "/users/[username]", "/dashboard", "/fr", "/login"]) {
      expect(patterns).toContain(expected);
    }
    expect(manifest.entries).toHaveLength(20);
    expect(manifest.entries.filter((e) => e.kind === "page")).toHaveLength(13);
    expect(manifest.entries.filter((e) => e.kind === "route-handler")).toHaveLength(3);
    expect(manifest.entries.filter((e) => e.kind === "metadata")).toHaveLength(4);
    expect(manifest.appDir).toBe("app");
    expect(manifest.pagesDir).toBe("pages");
    expect(manifest.rootLayout).toBe("app/layout.tsx");
    expect(manifest.rootNotFound).toBe("app/not-found.tsx");
    expect(manifest.globalError).toBe("app/global-error.tsx");
    expect(manifest.proxyFile).toBe("proxy.ts");
    expect(manifest.instrumentation).toBe("instrumentation.ts");
    expect(manifest.instrumentationClient).toBe("instrumentation-client.ts");
    expect(manifest.staticMetadata).toEqual([{ kind: "icon", path: "app/icon.svg", generated: false }]);
    expect(manifest.diagnostics).toEqual([]);
    expect(patterns).not.toContain("/_private");
    expect(manifest.entries.some((e) => e.file.includes("_private"))).toBe(false);
  });

  test("groups, slots, intercepts and private folders", () => {
    const pricing = byFile("app/(marketing)/pricing/page.tsx");
    expect(pricing.urlPattern).toBe("/pricing");
    expect(pricing.appPath).toBe("/(marketing)/pricing");
    expect(pricing.layouts).toEqual(["app/layout.tsx", "app/(marketing)/layout.tsx"]);
    const modal = byFile("app/@modal/(.)photos/[id]/page.tsx");
    expect(modal.urlPattern).toBe("/photos/[id]");
    expect(modal.slot).toBe("modal");
    expect(modal.intercepting).toEqual({ level: 0, target: "/photos/[id]" });
    expect(modal.params).toEqual([{ name: "id", catchAll: false, optional: false }]);
    const analytics = byFile("app/dashboard/@analytics/page.tsx");
    expect(analytics.urlPattern).toBe("/dashboard");
    expect(analytics.slot).toBe("analytics");
    expect(analytics.loading).toBe("app/dashboard/@analytics/loading.tsx");
    const tree = manifest.tree;
    expect(tree?.name).toBe("app");
    expect(tree?.kind).toBe("root");
    expect(tree?.files.default).toBeUndefined();
    const modalSegment = tree?.children.find((c) => c.name === "@modal");
    expect(modalSegment?.kind).toBe("parallel-slot");
    expect(modalSegment?.files.default).toBe("app/@modal/default.tsx");
    const intercept = modalSegment?.children.find((c) => c.name === "(.)photos");
    expect(intercept?.kind).toBe("intercept");
    expect(intercept?.interceptLevel).toBe(0);
    expect(intercept?.interceptTarget).toBe("photos");
  });

  test("layouts chain, loading/error/not-found and parallel slots", () => {
    const blog = byFile("app/blog/[slug]/page.tsx");
    expect(blog.layouts).toEqual(["app/layout.tsx", "app/blog/layout.tsx"]);
    expect(blog.templates).toEqual(["app/template.tsx"]);
    expect(blog.loading).toBe("app/blog/[slug]/loading.tsx");
    expect(blog.error).toBe("app/error.tsx");
    expect(blog.notFound).toBe("app/not-found.tsx");
    expect(blog.parallelSlots).toEqual([]);
    expect(blog.params).toEqual([{ name: "slug", catchAll: false, optional: false }]);
    expect(byFile("app/dashboard/page.tsx").parallelSlots).toEqual(["analytics"]);
    expect(byFile("app/page.tsx").parallelSlots).toEqual(["modal"]);
    expect(byFile("app/docs/[...slug]/page.tsx").params).toEqual([{ name: "slug", catchAll: true, optional: false }]);
    expect(byFile("app/shop/[[...filters]]/page.tsx").params).toEqual([{ name: "filters", catchAll: true, optional: true }]);
  });

  test("route handlers, metadata routes and the pages router", () => {
    expect(byFile("app/api/posts/[id]/route.ts").methods).toEqual(["GET", "POST", "DELETE"]);
    expect(byFile("app/api/health/route.ts").methods).toEqual(["GET"]);
    expect(byFile("app/api/health/route.ts").segmentConfig.dynamic).toBe("force-static");
    const sitemap = byFile("app/sitemap.ts");
    expect(sitemap.kind).toBe("metadata");
    expect(sitemap.metadataKind).toBe("sitemap");
    expect(sitemap.urlPattern).toBe("/sitemap.xml");
    expect(byFile("app/opengraph-image.tsx").urlPattern).toBe("/opengraph-image");
    expect(byFile("app/manifest.ts").urlPattern).toBe("/manifest.webmanifest");
    const legacy = byFile("pages/legacy.tsx");
    expect(legacy.legacyPages).toBe(true);
    expect(legacy.legacyDataFunctions).toEqual(["getServerSideProps"]);
    expect(legacy.layouts).toEqual([]);
    const api = byFile("pages/api/legacy.ts");
    expect(api.kind).toBe("route-handler");
    expect(api.methods).toEqual(["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]);
    expect(manifest.entries.some((e) => e.file === "pages/_app.tsx")).toBe(false);
  });

  test("segment config is attached to entries", () => {
    const blog = byFile("app/blog/[slug]/page.tsx").segmentConfig;
    expect(blog.hasGenerateStaticParams).toBe(true);
    expect(blog.hasGenerateMetadata).toBe(true);
    expect(blog.revalidate).toBe(60);
    expect(blog.dynamicParams).toBe(true);
    expect(byFile("app/(marketing)/pricing/page.tsx").segmentConfig.diagnostics.map((d) => d.code)).toEqual(["segment-config-edge-runtime-deprecated"]);
    expect(byFile("app/@modal/(.)photos/[id]/page.tsx").segmentConfig.directive).toBe("use-client");
  });
});

describe("pattern conversions", () => {
  test("bun / tanstack / dioxus / leptos", () => {
    expect(toBunPattern("/blog/[slug]")).toBe("/blog/:slug");
    expect(toBunPattern("/docs/[...slug]")).toBe("/docs/*");
    expect(toBunPattern("/shop/[[...filters]]")).toBe("/shop/*");
    expect(toBunPattern("/")).toBe("/");
    expect(toTanStackPath("/blog/[slug]")).toBe("/blog/$slug");
    expect(toTanStackPath("/docs/[...slug]")).toBe("/docs/$");
    expect(toTanStackPath("/shop/[[...filters]]")).toBe("/shop/$");
    expect(toDioxusRoute("/blog/[slug]")).toBe("/blog/:slug");
    expect(toDioxusRoute("/docs/[...slug]")).toBe("/docs/:..slug");
    expect(toDioxusRoute("/shop/[[...filters]]")).toBe("/shop/:..filters");
    expect(toDioxusVariant("/")).toBe("Home");
    expect(toDioxusVariant("/blog/[slug]")).toBe("BlogSlug");
    expect(toDioxusVariant("/docs/[...slug]")).toBe("DocsSlugCatchAll");
    expect(toDioxusVariant("/shop/[[...filters]]")).toBe("ShopFiltersOptional");
    expect(toDioxusVariant("/api/posts/[id]")).toBe("ApiPostsId");
    expect(toDioxusVariant("/sitemap.xml")).toBe("SitemapXml");
    expect(toLeptosPath("/blog/[slug]")).toBe("/blog/:slug");
    expect(toLeptosPath("/docs/[...slug]")).toBe("/docs/*slug");
    expect(toLeptosPath("/shop/[[...filters]]")).toBe("/shop/*filters");
  });

  test("classifySegment", () => {
    expect(classifySegment("[slug]")).toEqual({ kind: "dynamic", param: "slug" });
    expect(classifySegment("[...slug]")).toEqual({ kind: "catch-all", param: "slug" });
    expect(classifySegment("[[...slug]]")).toEqual({ kind: "optional-catch-all", param: "slug" });
    expect(classifySegment("(marketing)")).toEqual({ kind: "group" });
    expect(classifySegment("@modal")).toEqual({ kind: "parallel-slot" });
    expect(classifySegment("_private")).toEqual({ kind: "private" });
    expect(classifySegment("(.)photos").interceptLevel).toBe(0);
    expect(classifySegment("(..)photos").interceptLevel).toBe(1);
    expect(classifySegment("(..)(..)photos").interceptLevel).toBe(2);
    expect(classifySegment("(...)photos")).toMatchObject({ kind: "intercept", interceptLevel: 3, interceptTarget: "photos" });
    expect(classifySegment("blog")).toEqual({ kind: "static" });
  });
});

describe("matchRoute", () => {
  test("fixture cases", () => {
    const cases: Array<[string, string | undefined, Record<string, string>]> = [
      ["/blog/hello", "app/blog/[slug]/page.tsx", { slug: "hello" }],
      ["/blog/hello/", "app/blog/[slug]/page.tsx", { slug: "hello" }],
      ["/docs/a/b", "app/docs/[...slug]/page.tsx", { slug: "a/b" }],
      ["/shop", "app/shop/[[...filters]]/page.tsx", {}],
      ["/shop/x/y", "app/shop/[[...filters]]/page.tsx", { filters: "x/y" }],
      ["/photos/9", "app/photos/[id]/page.tsx", { id: "9" }],
      ["/api/posts/42", "app/api/posts/[id]/route.ts", { id: "42" }],
      ["/pricing", "app/(marketing)/pricing/page.tsx", {}],
      ["/dashboard", "app/dashboard/page.tsx", {}],
      ["/sitemap.xml", "app/sitemap.ts", {}],
      ["/legacy", "pages/legacy.tsx", {}],
      ["/api/legacy", "pages/api/legacy.ts", {}],
      ["/users/caf%C3%A9", "app/users/[username]/page.tsx", { username: "café" }],
      ["/", "app/page.tsx", {}],
      ["/nope", undefined, {}],
      ["/docs", undefined, {}],
    ];
    for (const [pathname, file, params] of cases) {
      const match = matchRoute(manifest, pathname);
      expect(match?.entry.file).toBe(file);
      if (file) expect(match?.params).toEqual(params);
    }
    expect(matchRoute(manifest, "/photos/9")?.entry.slot).toBeUndefined();
  });
});

describe("sortEntries", () => {
  test("static < dynamic < catch-all < optional, shorter first, then alphabetical", () => {
    const order = ["/shop/[[...filters]]", "/docs/[...slug]", "/blog/[slug]", "/blog/new", "/b", "/a", "/", "/a/b"];
    expect([...order].sort(compareUrlPatterns)).toEqual(["/", "/a", "/b", "/a/b", "/blog/new", "/blog/[slug]", "/docs/[...slug]", "/shop/[[...filters]]"]);
    const sorted = sortEntries(manifest.entries).map((e) => e.urlPattern);
    expect(sorted.indexOf("/photos/[id]")).toBeGreaterThan(sorted.indexOf("/pricing"));
    expect(sorted.indexOf("/docs/[...slug]")).toBeGreaterThan(sorted.indexOf("/blog/[slug]"));
    expect(sorted.indexOf("/shop/[[...filters]]")).toBeGreaterThan(sorted.indexOf("/docs/[...slug]"));
  });
});

describe("manifests", () => {
  test("routes-manifest.json shape", () => {
    const routes = routesManifestJson(manifest, "") as { version: number; staticRoutes: Array<{ page: string; regex: string }>; dynamicRoutes: Array<{ page: string; namedRegex: string; routeKeys: Record<string, string> }>; rsc: { header: string } };
    expect(routes.version).toBe(3);
    expect(routes.rsc.header).toBe("RSC");
    expect(routes.staticRoutes.find((r) => r.page === "/")?.regex).toBe("^/(?:/)?$");
    expect(routes.staticRoutes.find((r) => r.page === "/sitemap.xml")?.regex).toBe("^/sitemap\\.xml(?:/)?$");
    const blog = routes.dynamicRoutes.find((r) => r.page === "/blog/[slug]");
    expect(blog?.namedRegex).toBe("^/blog/(?<nxtPslug>[^/]+?)(?:/)?$");
    expect(blog?.routeKeys).toEqual({ nxtPslug: "nxtPslug" });
    expect(routeRegexes("/docs/[...slug]").regex).toBe("^/docs/(.+?)(?:/)?$");
    expect(routeRegexes("/shop/[[...filters]]").namedRegex).toBe("^/shop(?:/(?<nxtPfilters>.+?))?(?:/)?$");
    expect(routes.dynamicRoutes.some((r) => r.page === "/dashboard")).toBe(false);
  });

  test("app-paths-manifest.json shape", () => {
    const paths = appPathsManifestJson(manifest);
    expect(paths["/page"]).toBe("app/page.tsx");
    expect(paths["/(marketing)/pricing/page"]).toBe("app/(marketing)/pricing/page.tsx");
    expect(paths["/api/health/route"]).toBe("app/api/health/route.ts");
    expect(paths["/sitemap.xml/route"]).toBe("app/sitemap.ts");
    expect(paths["/@modal/(.)photos/[id]/page"]).toBe("app/@modal/(.)photos/[id]/page.tsx");
    expect(Object.values(paths)).not.toContain("pages/legacy.tsx");
  });

  test("loadRouteManifest prefers .yolo/route-manifest.json", () => {
    const dir = mkdtempSync(join(tmpdir(), "yolo-router-"));
    try {
      mkdirSync(join(dir, ".yolo"), { recursive: true });
      mkdirSync(join(dir, "app"), { recursive: true });
      writeFileSync(join(dir, "app", "page.tsx"), "export default function P() { return null }");
      writeFileSync(join(dir, ".yolo", "route-manifest.json"), JSON.stringify({ ...manifest, root: "cached" }));
      expect(loadRouteManifest(dir).root).toBe("cached");
      expect(scanAppDir(dir).entries.map((e) => e.urlPattern)).toEqual(["/"]);
      expect(scanAppDir(dir).diagnostics.map((d) => d.code)).toEqual(["root-layout-missing"]);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("route-page-conflict diagnostic", () => {
    const dir = mkdtempSync(join(tmpdir(), "yolo-conflict-"));
    try {
      mkdirSync(join(dir, "app", "x"), { recursive: true });
      writeFileSync(join(dir, "app", "layout.tsx"), "export default function L({ children }) { return children }");
      writeFileSync(join(dir, "app", "x", "page.tsx"), "export default function P() { return null }");
      writeFileSync(join(dir, "app", "x", "route.ts"), "export function GET() { return new Response('x') }");
      const m = scanAppDir(dir);
      expect(m.diagnostics.map((d) => d.code)).toEqual(["route-page-conflict"]);
      expect(m.entries.filter((e) => e.urlPattern === "/x")).toHaveLength(1);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});

describe("generateRouteTree", () => {
  test("TanStack code-based tree", () => {
    const tree = generateRouteTree(manifest, { appDirImportPrefix: "../app" });
    expect(tree).toContain("createRootRoute(");
    expect(tree).toContain('path: "/blog/$slug"');
    expect(tree).toContain('path: "/docs/$"');
    expect(tree).toContain("createRouter({ routeTree");
    expect(tree).toContain('declare module "@tanstack/react-router"');
    expect(tree).toContain('import Mod_app_layout from "../app/layout.tsx";');
    expect(tree).toContain('import Mod_pages_legacy from "../pages/legacy.tsx";');
    expect(tree).toContain("component: withLayout(Mod_app_layout, Mod_app_template)");
    expect(tree).toContain("pendingComponent: Mod_app_loading");
    expect(tree).toContain("notFoundComponent: Mod_app_not_found");
    expect(tree).toContain("layout_blog.addChildren([");
    expect(tree).not.toContain("(.)photos");
    expect(tree).not.toContain("@analytics");
    expect(tree).not.toContain("route.ts");
  });
});
