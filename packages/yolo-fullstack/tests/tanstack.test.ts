// SPDX-License-Identifier: MIT OR Apache-2.0
import { afterAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { QueryClientProvider, useQuery } from "@tanstack/react-query";
import { createElement } from "react";
import { renderToString } from "react-dom/server";
import { type AppRoute, type AppTree, type Segment, parsePattern } from "../src/app/scan";
import { createStore, requestStorage } from "../src/app/runtime/store";
import {
  HydrateClient,
  QueryProvider,
  bunPatternFromRoute,
  bunPatternsFromRoute,
  bunRoutesFromTree,
  getQueryClient,
  makeQueryClient,
  prefetchQuery,
  renderRouteTree,
  resetQueryClient,
  routeTreePlugin,
  tanstackRoutePath,
} from "../src/tanstack";

const servers: Bun.Server<undefined>[] = [];
const tempDirs: string[] = [];
afterAll(() => {
  for (const server of servers) server.stop(true);
  for (const dir of tempDirs) rmSync(dir, { recursive: true, force: true });
});

const route = (kind: AppRoute["kind"], pattern: string, file: string, segments: Segment[] = []): AppRoute => ({
  kind,
  pattern,
  parts: parsePattern(pattern),
  file,
  segments,
});

describe("tanstackRoutePath", () => {
  const table: [string, string][] = [
    ["/", "/"],
    ["/about", "/about"],
    ["/blog/[slug]", "/blog/$slug"],
    ["/users/[id]/posts/[postId]", "/users/$id/posts/$postId"],
    ["/docs/[...slug]", "/docs/$"],
    ["/docs/[[...slug]]", "/docs/$"],
    ["/[...all]", "/$"],
  ];
  for (const [input, expected] of table)
    test(`${input} -> ${expected}`, () => expect(tanstackRoutePath(input)).toBe(expected));
});

describe("bunPatternsFromRoute", () => {
  const table: [string, string[]][] = [
    ["/", ["/"]],
    ["/api/health", ["/api/health"]],
    ["/api/users/[id]", ["/api/users/:id"]],
    ["/api/orgs/[org]/repos/[repo]", ["/api/orgs/:org/repos/:repo"]],
    ["/api/files/[...path]", ["/api/files/*"]],
    ["/api/docs/[[...slug]]", ["/api/docs/*", "/api/docs"]],
    ["/[[...all]]", ["/*", "/"]],
  ];
  for (const [input, expected] of table)
    test(`${input} -> ${expected.join(" | ")}`, () => {
      const r = route("route", input, "app/route.ts");
      expect(bunPatternsFromRoute(r)).toEqual(expected);
      expect(bunPatternFromRoute(r)).toBe(expected[0]!);
    });
  test("prefix", () => {
    expect(
      bunPatternsFromRoute(route("route", "/users/[id]", "x"), {
        prefix: "/api/",
      }),
    ).toEqual(["/api/users/:id"]);
    expect(bunPatternsFromRoute(route("route", "/", "x"), { prefix: "api" })).toEqual(["/api/"]);
  });
});

function sampleTree(): AppTree {
  const root: Segment = {
    path: "/",
    files: {
      layout: "app/layout.tsx",
      loading: "app/loading.tsx",
      "not-found": "app/not-found.tsx",
    },
  };
  const blog: Segment = {
    path: "/blog",
    files: { layout: "app/blog/layout.tsx", error: "app/blog/error.tsx" },
  };
  const marketing: Segment = {
    path: "/",
    files: { layout: "app/(marketing)/layout.tsx" },
  };
  const docs: Segment = {
    path: "/docs",
    files: { loading: "app/docs/loading.tsx" },
  };
  return {
    root: "C:/fake",
    appDir: "app",
    routes: [
      route("page", "/", "app/page.tsx", [root]),
      route("page", "/about", "app/(marketing)/about/page.tsx", [root, marketing, { path: "/about", files: {} }]),
      route("page", "/blog", "app/blog/page.tsx", [root, blog]),
      route("page", "/blog/[slug]", "app/blog/[slug]/page.tsx", [root, blog, { path: "/blog/[slug]", files: {} }]),
      route("page", "/docs/[[...slug]]", "app/docs/[[...slug]]/page.tsx", [
        root,
        docs,
        { path: "/docs/[[...slug]]", files: {} },
      ]),
      route("route", "/api/users/[id]", "app/api/users/[id]/route.ts"),
    ],
    metadata: [],
    notFound: "app/not-found.tsx",
    rootSegment: root,
  };
}

describe("renderRouteTree", () => {
  const code = renderRouteTree(sampleTree());

  test("root route from the root layout with boundaries", () => {
    expect(code).toContain(
      'const rootRoute = createRootRoute({ component: withOutlet(lazy(() => import("./app/layout.tsx"))), wrapInSuspense: true, pendingComponent: lazy(() => import("./app/loading.tsx")), notFoundComponent: lazy(() => import("./app/not-found.tsx")) });',
    );
  });

  test("index page under the root", () => {
    expect(code).toContain(
      'createRoute({ getParentRoute: () => rootRoute, path: "/", component: lazy(() => import("./app/page.tsx")), pendingComponent: lazy(() => import("./app/loading.tsx")), notFoundComponent: lazy(() => import("./app/not-found.tsx")) })',
    );
  });

  test("route group layout becomes a pathless parent", () => {
    expect(code).toMatch(
      /const layout\d+ = createRoute\(\{ getParentRoute: \(\) => rootRoute, id: "_layout\d+", component: withOutlet\(lazy\(\(\) => import\("\.\/app\/\(marketing\)\/layout\.tsx"\)\)\)/,
    );
    expect(code).toMatch(
      /getParentRoute: \(\) => layout\d+, path: "about", component: lazy\(\(\) => import\("\.\/app\/\(marketing\)\/about\/page\.tsx"\)\)/,
    );
  });

  test("nested layout with its index page, dynamic child and error boundary", () => {
    expect(code).toMatch(
      /const layout\d+ = createRoute\(\{ getParentRoute: \(\) => rootRoute, path: "blog", component: withOutlet\(lazy\(\(\) => import\("\.\/app\/blog\/layout\.tsx"\)\)\), pendingComponent: lazy\(\(\) => import\("\.\/app\/loading\.tsx"\)\), errorComponent: lazy\(\(\) => import\("\.\/app\/blog\/error\.tsx"\)\)/,
    );
    expect(code).toMatch(
      /getParentRoute: \(\) => layout\d+, path: "\/", component: lazy\(\(\) => import\("\.\/app\/blog\/page\.tsx"\)\)/,
    );
    expect(code).toMatch(
      /getParentRoute: \(\) => layout\d+, path: "\$slug", component: lazy\(\(\) => import\("\.\/app\/blog\/\[slug\]\/page\.tsx"\)\)/,
    );
  });

  test("catch-all page and nearest loading file", () => {
    expect(code).toMatch(
      /getParentRoute: \(\) => rootRoute, path: "docs\/\$", component: lazy\(\(\) => import\("\.\/app\/docs\/\[\[\.\.\.slug\]\]\/page\.tsx"\)\), pendingComponent: lazy\(\(\) => import\("\.\/app\/docs\/loading\.tsx"\)\)/,
    );
  });

  test("tree assembly, router factory and no handler routes", () => {
    expect(code).toMatch(
      /export const routeTree = rootRoute\.addChildren\(\[page\d+, layout\d+\.addChildren\(\[page\d+\]\), layout\d+\.addChildren\(\[page\d+, page\d+\]\), page\d+\]\);/,
    );
    expect(code).toContain("export function createAppRouter(");
    expect(code).toContain('createRouter({ routeTree, defaultPreload: "intent", ...options })');
    expect(code).not.toContain("route.ts");
  });

  test("importPrefix", () => {
    expect(renderRouteTree(sampleTree(), { importPrefix: "/src/" })).toContain('import("/src/app/page.tsx")');
  });

  test("root without a layout renders Outlet", () => {
    const tree = sampleTree();
    tree.rootSegment = { path: "/", files: {} };
    tree.notFound = undefined;
    tree.routes = [route("page", "/", "app/page.tsx", [tree.rootSegment])];
    expect(renderRouteTree(tree)).toContain("createRootRoute({ component: Outlet, wrapInSuspense: true })");
  });
});

describe("routeTreePlugin", () => {
  test("serves yolo:route-tree to Bun.build", async () => {
    const dir = mkdtempSync(join(tmpdir(), "yolo-tanstack-"));
    tempDirs.push(dir);
    mkdirSync(join(dir, "app"));
    writeFileSync(join(dir, "app", "layout.tsx"), "export default function Layout(p: any) { return p.children; }\n");
    writeFileSync(join(dir, "app", "page.tsx"), 'export default function Page() { return "home"; }\n');
    writeFileSync(join(dir, "entry.ts"), 'export { routeTree } from "yolo:route-tree";\n');
    const root: Segment = { path: "/", files: { layout: "app/layout.tsx" } };
    const tree: AppTree = {
      root: dir,
      appDir: "app",
      routes: [route("page", "/", "app/page.tsx", [root])],
      metadata: [],
      rootSegment: root,
    };
    const result = await Bun.build({
      entrypoints: [join(dir, "entry.ts")],
      plugins: [routeTreePlugin(tree)],
      external: ["react", "@tanstack/react-router"],
      target: "browser",
    });
    expect(result.success).toBe(true);
    const text = await result.outputs[0]!.text();
    expect(text).toContain("createRootRoute");
    expect(text).toContain("home");
  });
});

describe("bunRoutesFromTree", () => {
  const seen: unknown[] = [];
  const modules: Record<string, Record<string, unknown>> = {
    "app/api/users/[id]/route.ts": {
      GET: async (req: Request, ctx: { params: Promise<Record<string, unknown>> }) =>
        Response.json({ method: req.method, params: await ctx.params }),
      POST: async (req: Request) => Response.json({ body: await req.text() }, { status: 201 }),
    },
    "app/api/files/[...path]/route.ts": {
      GET: async (_req: Request, ctx: { params: Promise<Record<string, unknown>> }) => Response.json(await ctx.params),
    },
    "app/api/docs/[[...slug]]/route.ts": {
      GET: async (_req: Request, ctx: { params: Promise<Record<string, unknown>> }) => Response.json(await ctx.params),
      DELETE: async () => new Response(null, { status: 204 }),
    },
  };
  const tree: AppTree = {
    root: "C:/fake",
    appDir: "app",
    routes: [
      route("route", "/api/users/[id]", "app/api/users/[id]/route.ts"),
      route("route", "/api/files/[...path]", "app/api/files/[...path]/route.ts"),
      route("route", "/api/docs/[[...slug]]", "app/api/docs/[[...slug]]/route.ts"),
      route("page", "/", "app/page.tsx"),
    ],
    metadata: [],
    rootSegment: { path: "/", files: {} },
  };
  const load = async (file: string) => {
    seen.push(file);
    const mod = modules[file];
    if (!mod) throw new Error(`unknown ${file}`);
    return mod;
  };
  const routes = bunRoutesFromTree(tree, load);
  const server = Bun.serve({
    port: 0,
    routes,
    fetch: () => new Response("fallback", { status: 404 }),
  });
  servers.push(server);
  const url = (path: string) => new URL(path, server.url).href;

  test("entries", () => {
    expect(Object.keys(routes).sort()).toEqual(["/api/docs", "/api/docs/*", "/api/files/*", "/api/users/:id"]);
  });

  test("params from :id", async () => {
    const res = await fetch(url("/api/users/42"));
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ method: "GET", params: { id: "42" } });
  });

  test("POST and module loaded once", async () => {
    const res = await fetch(url("/api/users/42"), {
      method: "POST",
      body: "hi",
    });
    expect(res.status).toBe(201);
    expect(await res.json()).toEqual({ body: "hi" });
    expect(seen.filter(f => f === "app/api/users/[id]/route.ts")).toHaveLength(1);
  });

  test("405 with Allow for a missing method", async () => {
    const res = await fetch(url("/api/users/42"), { method: "PUT" });
    expect(res.status).toBe(405);
    expect(res.headers.get("allow")).toBe("GET, HEAD, POST");
  });

  test("HEAD falls back to GET without a body", async () => {
    const res = await fetch(url("/api/users/7"), { method: "HEAD" });
    expect(res.status).toBe(200);
    expect(await res.text()).toBe("");
  });

  test("catch-all splat as an array", async () => {
    const res = await fetch(url("/api/files/a/b/c%20d"));
    expect(await res.json()).toEqual({ path: ["a", "b", "c d"] });
  });

  test("optional catch-all with and without segments", async () => {
    expect(await (await fetch(url("/api/docs/x/y"))).json()).toEqual({
      slug: ["x", "y"],
    });
    expect(await (await fetch(url("/api/docs"))).json()).toEqual({});
    expect((await fetch(url("/api/docs"), { method: "DELETE" })).status).toBe(204);
  });

  test("unmatched paths reach fetch", async () => {
    expect((await fetch(url("/api/nope"))).status).toBe(404);
  });

  test("prefix option", async () => {
    const prefixed = bunRoutesFromTree(tree, load, { prefix: "/v1" });
    const s = Bun.serve({ port: 0, routes: prefixed });
    servers.push(s);
    const res = await fetch(new URL("/v1/api/files/p/q", s.url).href);
    expect(await res.json()).toEqual({ path: ["p", "q"] });
  });
});

describe("getQueryClient", () => {
  test("one client per request store, a singleton outside", async () => {
    resetQueryClient();
    const outside = getQueryClient();
    expect(getQueryClient()).toBe(outside);
    const storeA = createStore(new Request("http://localhost/a"));
    const storeB = createStore(new Request("http://localhost/b"));
    const a1 = requestStorage.run(storeA, () => getQueryClient());
    const a2 = requestStorage.run(storeA, () => getQueryClient());
    const b = requestStorage.run(storeB, () => getQueryClient());
    expect(a1).toBe(a2);
    expect(b).not.toBe(a1);
    expect(a1).not.toBe(outside);
    expect(getQueryClient()).toBe(outside);
    await requestStorage.run(storeA, async () => {
      await Promise.resolve();
      expect(getQueryClient()).toBe(a1);
    });
  });

  test("makeQueryClient defaults", async () => {
    const client = makeQueryClient();
    expect(client.getDefaultOptions().queries?.staleTime).toBe(60_000);
    const pending = client.prefetchQuery({
      queryKey: ["slow"],
      queryFn: () => new Promise<number>(r => setTimeout(() => r(1), 20)),
    });
    const { dehydrate } = await import("@tanstack/react-query");
    const state = dehydrate(client);
    expect(state.queries.map(q => q.queryHash)).toEqual(['["slow"]']);
    expect(state.queries[0]!.state.status).toBe("pending");
    await pending;
  });
});

describe("HydrateClient", () => {
  function Todos() {
    const { data, status } = useQuery({
      queryKey: ["todos"],
      queryFn: async () => ["never fetched"],
    });
    return createElement("ul", { "data-status": status }, ...(data ?? []).map(t => createElement("li", { key: t }, t)));
  }

  test("prefetched data renders from the hydrated cache", async () => {
    const store = createStore(new Request("http://localhost/todos"));
    const html = await requestStorage.run(store, async () => {
      await prefetchQuery({
        queryKey: ["todos"],
        queryFn: async () => ["write tests", "ship"],
      });
      // A fresh client on the consumer side: the data can only come from the dehydrated state.
      const browser = makeQueryClient();
      return renderToString(
        createElement(
          QueryClientProvider,
          { client: browser },
          createElement(HydrateClient, null, createElement(Todos)),
        ),
      );
    });
    expect(html).toContain('data-status="success"');
    expect(html).toContain("<li>write tests</li><li>ship</li>");
  });

  test("QueryProvider renders on the server", () => {
    const html = renderToString(
      createElement(QueryProvider, null, createElement(HydrateClient, { client: makeQueryClient() }, "ok")),
    );
    expect(html).toBe("ok");
  });
});
