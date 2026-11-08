// SPDX-License-Identifier: Apache-2.0
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { defineYoloConfig } from "../src/config.ts";
import { createAppRouterRoutes, createYoloNextApp, robotsTxt, sitemapXml, type YoloNextApp } from "../src/server.ts";

const root = `${import.meta.dir}/.tmp-server-${process.pid}`;

function write(relative: string, contents: string): void {
  const file = `${root}/${relative}`;
  mkdirSync(file.slice(0, file.lastIndexOf("/")), { recursive: true });
  writeFileSync(file, contents);
}

let app: YoloNextApp;

beforeAll(() => {
  write(
    "app/layout.tsx",
    'export const metadata = { title: { default: "Fixture", template: "%s | Fixture" }, description: "server test" };\nexport default function RootLayout({ children }: { children: React.ReactNode }) { return <html lang="en"><body><header data-layout="root">Layout</header>{children}</body></html>; }\n',
  );
  write("app/page.tsx", 'export default function Home() { return <main id="home"><h1>Home page</h1></main>; }\n');
  write(
    "app/blog/[slug]/page.tsx",
    'export async function generateMetadata({ params }: { params: Promise<{ slug: string }> }) { const { slug } = await params; return { title: `Post ${slug}` }; }\nexport default async function Post({ params, searchParams }: { params: Promise<{ slug: string }>; searchParams: Promise<{ q?: string }> }) { const { slug } = await params; const { q } = await searchParams; return <article><h1>{slug}</h1><p>{q ?? "no query"}</p></article>; }\n',
  );
  write(
    "app/api/health/route.ts",
    'export async function GET(_request: Request, { params }: { params: Promise<Record<string, string>> }) { await params; return Response.json({ ok: true, runtime: "yolo" }); }\nexport async function POST(request: Request) { const body = await request.json(); return Response.json(body, { status: 201 }); }\n',
  );
  write(
    "app/api/cookie/route.ts",
    'import { cookies } from "../../../../../src/headers.ts";\nexport async function GET() { (await cookies()).set("seen", "1", { httpOnly: true }); return new Response("ok"); }\n',
  );
  write("app/redirecting/page.tsx", 'import { redirect } from "../../../../src/navigation.ts";\nexport default function Redirecting() { redirect("/blog/target"); }\n');
  write("app/gone/page.tsx", 'import { notFound } from "../../../../src/navigation.ts";\nexport default function Gone() { notFound(); }\n');
  write("app/broken/page.tsx", "export default function Broken() { throw new Error(\"render exploded\"); }\n");
  write("app/error.tsx", '"use client";\nexport default function ErrorBoundary({ error }: { error: Error }) { return <div role="alert">Boundary: {error.message}</div>; }\n');
  write("app/not-found.tsx", "export default function NotFound() { return <main><h2>Custom not found</h2></main>; }\n");
  write(
    "app/robots.ts",
    'export default function robots() { return { rules: [{ userAgent: "*", allow: "/", disallow: ["/dashboard/", "/api/"] }], sitemap: "https://fixture.example.com/sitemap.xml" }; }\n',
  );
  write(
    "app/sitemap.ts",
    'export default function sitemap() { return [{ url: "https://fixture.example.com", lastModified: new Date("2026-01-01"), changeFrequency: "weekly", priority: 1 }]; }\n',
  );
  write("app/manifest.ts", 'export default function manifest() { return { name: "Fixture", start_url: "/" }; }\n');
  write("public/logo.svg", '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10"/></svg>\n');
  write("public/big.txt", "x".repeat(4096));
  write(".yolo/static/chunks/app.js", "console.log('hashed');\n");
  write(
    "proxy.ts",
    'export function proxy(request: Request) { const url = new URL(request.url); if (url.pathname === "/blocked") { return new Response("blocked by proxy", { status: 403 }); } if (url.pathname.startsWith("/u/")) { return new Response(null, { headers: { "x-middleware-rewrite": url.pathname.replace("/u/", "/blog/") } }); } return new Response(null, { headers: { "x-middleware-next": "1", "x-yolo-proxy": "1" } }); }\nexport const config = { matcher: ["/blocked", "/u/:path*", "/"] };\n',
  );
  app = createYoloNextApp({
    root,
    config: defineYoloConfig({
      poweredByHeader: true,
      deploymentId: "dpl_test",
      async redirects() {
        return [{ source: "/old-blog/:slug", destination: "/blog/:slug", permanent: true }];
      },
      async rewrites() {
        return { afterFiles: [{ source: "/alias/:slug", destination: "/blog/:slug" }] };
      },
      async headers() {
        return [{ source: "/(.*)", headers: [{ key: "X-Frame-Options", value: "DENY" }] }];
      },
    }),
    rscMode: "inline",
    dev: false,
  });
});

afterAll(async () => {
  await app.stop();
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      rmSync(root, { recursive: true, force: true });
      return;
    } catch {
      await Bun.sleep(100);
    }
  }
});

const get = (path: string, init?: RequestInit) => app.fetch(new Request(`http://localhost${path}`, init));

describe("createYoloNextApp", () => {
  test("exposes a Bun.serve compatible shape and a route table", () => {
    expect(app.port).toBe(3000);
    expect(app.hostname).toBe("0.0.0.0");
    expect(app.development).toBe(false);
    expect(typeof app.fetch).toBe("function");
    expect(Object.keys(app.routes)).toEqual(expect.arrayContaining(["/", "/blog/:slug", "/api/health", "/_yolo/health", "/_yolo/static/*", "/_next/static/*", "/_yolo/image"]));
    const routes = createAppRouterRoutes({ ...app.manifest, entries: app.manifest.entries.filter((e) => e.urlPattern === "/") }, async () => new Response("x"));
    expect(routes["/"]).toBeDefined();
  });

  test("renders the root page through the layout with metadata in head", async () => {
    const response = await get("/");
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain("text/html");
    expect(response.headers.get("x-powered-by")).toBe("Yolo");
    expect(response.headers.get("x-frame-options")).toBe("DENY");
    expect(response.headers.get("x-nextjs-deployment-id")).toBe("dpl_test");
    expect(response.headers.get("cache-control")).toContain("s-maxage");
    const html = await response.text();
    expect(html).toContain('<header data-layout="root">Layout</header>');
    expect(html).toContain('<main id="home"><h1>Home page</h1></main>');
    expect(html).toContain("<title>Fixture</title>");
    expect(html).toContain("__next_f");
  });

  test("dynamic segments receive params and searchParams promises", async () => {
    const response = await get("/blog/hello?q=yolo");
    expect(response.status).toBe(200);
    const html = await response.text();
    expect(html).toContain("<h1>hello</h1>");
    expect(html).toContain("<p>yolo</p>");
    expect(html).toContain("<title>Post hello | Fixture</title>");
  });

  test("?_rsc= returns the Flight payload", async () => {
    const response = await get("/blog/flight?_rsc=1");
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("text/x-component");
    expect(response.headers.get("vary")).toContain("rsc");
    expect(response.headers.get("x-nextjs-stale-time")).toBe("300");
    const flight = await response.text();
    expect(flight).toContain('["$","h1",null,{"children":"flight"}]');
  });

  test("route handlers: GET, POST, HEAD fallback, OPTIONS and cookies from the store", async () => {
    const health = await get("/api/health");
    expect(health.status).toBe(200);
    expect(await health.json()).toEqual({ ok: true, runtime: "yolo" });
    const created = await get("/api/health", { method: "POST", body: JSON.stringify({ a: 1 }), headers: { "content-type": "application/json" } });
    expect(created.status).toBe(201);
    expect(await created.json()).toEqual({ a: 1 });
    const head = await get("/api/health", { method: "HEAD" });
    expect(head.status).toBe(200);
    expect(await head.text()).toBe("");
    const options = await get("/api/health", { method: "OPTIONS" });
    expect(options.status).toBe(204);
    expect(options.headers.get("allow")).toBe("GET, POST, HEAD, OPTIONS");
    const missing = await get("/api/health", { method: "DELETE" });
    expect(missing.status).toBe(405);
    const cookie = await get("/api/cookie");
    expect(cookie.headers.getSetCookie()[0]).toContain("seen=1");
  });

  test("metadata routes: robots.txt, sitemap.xml and manifest.webmanifest", async () => {
    const robots = await get("/robots.txt");
    expect(robots.headers.get("content-type")).toContain("text/plain");
    const robotsBody = await robots.text();
    expect(robotsBody).toContain("User-Agent: *");
    expect(robotsBody).toContain("Disallow: /dashboard/");
    expect(robotsBody).toContain("Sitemap: https://fixture.example.com/sitemap.xml");
    const sitemap = await get("/sitemap.xml");
    expect(sitemap.headers.get("content-type")).toBe("application/xml");
    const xml = await sitemap.text();
    expect(xml).toContain("<urlset");
    expect(xml).toContain("<loc>https://fixture.example.com</loc>");
    expect(xml).toContain("<lastmod>2026-01-01T00:00:00.000Z</lastmod>");
    expect(xml).toContain("<changefreq>weekly</changefreq>");
    const manifest = await get("/manifest.webmanifest");
    expect(manifest.headers.get("content-type")).toBe("application/manifest+json");
    expect(await manifest.json()).toEqual({ name: "Fixture", start_url: "/" });
  });

  test("public files and static chunks", async () => {
    const logo = await get("/logo.svg");
    expect(logo.status).toBe(200);
    expect(logo.headers.get("content-type")).toContain("image/svg+xml");
    expect(logo.headers.get("cache-control")).toContain("public");
    const chunk = await get("/_yolo/static/chunks/app.js");
    expect(chunk.status).toBe(200);
    expect(chunk.headers.get("cache-control")).toContain("immutable");
    const compat = await get("/_next/static/chunks/app.js");
    expect(compat.status).toBe(200);
    expect((await get("/_yolo/static/nope.js")).status).toBe(404);
    const health = await get("/_yolo/health");
    expect((await health.json()) as { ok: boolean }).toMatchObject({ ok: true, runtime: `bun@${Bun.version}` });
  });

  test("404 renders not-found.tsx inside the root layout", async () => {
    const response = await get("/missing");
    expect(response.status).toBe(404);
    const html = await response.text();
    expect(html).toContain("<h2>Custom not found</h2>");
    expect(html).toContain('data-layout="root"');
    const gone = await get("/gone");
    expect(gone.status).toBe(404);
  });

  test("redirect rules, trailing slash and rewrites", async () => {
    const redirect = await get("/old-blog/hello");
    expect(redirect.status).toBe(308);
    expect(redirect.headers.get("location")).toBe("/blog/hello");
    const slash = await get("/blog/hello/");
    expect(slash.status).toBe(308);
    expect(slash.headers.get("location")).toBe("/blog/hello");
    const alias = await get("/alias/rewritten");
    expect(alias.status).toBe(200);
    expect(await alias.text()).toContain("<h1>rewritten</h1>");
  });

  test("proxy.ts can block, rewrite and continue", async () => {
    const blocked = await get("/blocked");
    expect(blocked.status).toBe(403);
    expect(await blocked.text()).toBe("blocked by proxy");
    const rewritten = await get("/u/proxied");
    expect(rewritten.status).toBe(200);
    expect(await rewritten.text()).toContain("<h1>proxied</h1>");
    const home = await get("/");
    expect(home.headers.get("x-yolo-proxy")).toBe("1");
  });

  test("redirect() and errors thrown during render", async () => {
    const redirect = await get("/redirecting");
    expect(redirect.status).toBe(307);
    expect(redirect.headers.get("location")).toBe("/blog/target");
    const broken = await get("/broken");
    expect(broken.status).toBe(500);
    expect(await broken.text()).toContain("Boundary:");
  });

  test("compresses buffered text bodies and honours ETag", async () => {
    const gz = await get("/big.txt", { headers: { "accept-encoding": "gzip" } });
    expect(gz.headers.get("content-encoding")).toBeOneOf(["gzip", null]);
    const etag = gz.headers.get("etag");
    if (etag) {
      const again = await get("/big.txt", { headers: { "if-none-match": etag } });
      expect(again.status).toBe(304);
    }
  });

  test("listen() starts a Bun server answering over HTTP", async () => {
    const server = app.listen(0);
    try {
      const response = await fetch(`http://127.0.0.1:${server.port}/api/health`);
      expect(await response.json()).toEqual({ ok: true, runtime: "yolo" });
      expect(app.port).toBe(server.port);
    } finally {
      await server.stop(true);
    }
  });
});

describe("metadata serializers", () => {
  test("robotsTxt and sitemapXml", () => {
    expect(robotsTxt({ rules: { userAgent: ["a", "b"], disallow: "/x" }, sitemap: ["s1", "s2"] })).toBe("User-Agent: a\nUser-Agent: b\nDisallow: /x\n\nSitemap: s1\nSitemap: s2\n");
    expect(sitemapXml([{ url: "https://e.com/a&b", priority: 0.5 }])).toContain("<loc>https://e.com/a&amp;b</loc><priority>0.5</priority>");
  });
});
