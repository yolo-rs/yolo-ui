// SPDX-License-Identifier: Apache-2.0
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildYoloNext, collectRouteModules, findServerActionModules, hasBuild } from "../src/build.ts";
import { defineYoloConfig, resolveYoloConfig } from "../src/config.ts";
import { scanAppDir } from "../src/router.ts";
import type { BuildResult } from "../src/build.ts";
import { FIXTURE, removeDir, scratchDir } from "./helpers.ts";

let root: string;
let result: BuildResult;

beforeAll(async () => {
  root = scratchDir("build");
  mkdirSync(join(root, "app", "components"), { recursive: true });
  mkdirSync(join(root, "public"), { recursive: true });
  writeFileSync(join(root, "app", "layout.tsx"), 'export const metadata = { title: "Smoke", description: "smoke test" };\nexport default function RootLayout({ children }: { children: React.ReactNode }) { return <html lang="en"><body>{children}</body></html>; }\n');
  writeFileSync(join(root, "app", "page.tsx"), 'import { Counter } from "./components/Counter";\nexport default function Page() { return <main><h1>Hello smoke</h1><Counter initial={2} /></main>; }\n');
  writeFileSync(join(root, "app", "components", "Counter.tsx"), '"use client";\nimport { useState } from "react";\nexport function Counter({ initial }: { initial: number }) { const [n, setN] = useState(initial); return <button onClick={() => setN(n + 1)}>{n}</button>; }\n');
  writeFileSync(join(root, "app", "actions.ts"), '"use server";\nexport async function ping(x: number) { return x + 1; }\n');
  writeFileSync(join(root, "public", "robots.txt"), "User-agent: *\n");
  writeFileSync(join(root, "tsconfig.json"), JSON.stringify({ compilerOptions: { jsx: "react-jsx", paths: { "@/*": ["./*"] } } }));
  const config = await resolveYoloConfig(defineYoloConfig({ reactStrictMode: true }), { root });
  result = await buildYoloNext({ root, config });
}, 120_000);

afterAll(() => {
  if (root) removeDir(root);
});

describe("buildYoloNext", () => {
  test("produces the client bundle, BUILD_ID and manifests", () => {
    expect(result.warnings.filter((w) => w.severity === "error")).toEqual([]);
    expect(result.success).toBe(true);
    const chunks = readdirSync(join(root, ".yolo", "static", "chunks"));
    expect(chunks.some((f) => /^entry-client-[a-z0-9]+\.js$/.test(f))).toBe(true);
    expect(result.rootMainFiles.some((f) => f.startsWith("chunks/entry-client-"))).toBe(true);
    expect(result.clientEntry && existsSync(result.clientEntry)).toBe(true);
    const buildId = readFileSync(join(root, ".yolo", "BUILD_ID"), "utf8");
    expect(buildId).toMatch(/^[0-9a-f]{16}$/);
    expect(result.buildId).toBe(buildId);
    for (const name of ["build-manifest.json", "app-build-manifest.json", "client-reference-manifest.json", "server-reference-manifest.json", "routes-manifest.json", "app-paths-manifest.json", "prerender-manifest.json", "route-manifest.json"]) {
      expect(existsSync(join(root, ".yolo", name))).toBe(true);
    }
    expect(existsSync(join(root, ".yolo", "public", "robots.txt"))).toBe(true);
    expect(result.durationMs).toBeGreaterThan(0);
  });

  test("rsc pass collects client modules and server actions", () => {
    expect(result.clientModules).toEqual(["app/components/Counter.tsx"]);
    expect(Object.values(result.serverActions)).toEqual([{ module: "app/actions.ts", exportName: "ping" }]);
    const clientManifest = JSON.parse(readFileSync(join(root, ".yolo", "client-reference-manifest.json"), "utf8")) as { clientModules: Record<string, { id: string; name: string; chunks: string[]; async: boolean }> };
    const counter = clientManifest.clientModules["app/components/Counter.tsx"];
    expect(counter?.name).toBe("*");
    expect(counter?.async).toBe(false);
    expect(counter?.chunks.length).toBeGreaterThan(0);
    const prerender = JSON.parse(readFileSync(join(root, ".yolo", "prerender-manifest.json"), "utf8")) as { version: number; preview: { previewModeId: string } };
    expect(prerender.version).toBe(4);
    expect(prerender.preview.previewModeId).toMatch(/^[0-9a-f]{32}$/);
    expect(result.rscEntry && existsSync(result.rscEntry)).toBe(true);
    expect(result.ssrEntry && existsSync(result.ssrEntry)).toBe(true);
    expect(hasBuild(join(root, ".yolo"))).toBe(true);
  });

  test("the rsc bundle renders Flight and the ssr bundle renders HTML", async () => {
    const rsc = (await import(Bun.pathToFileURL(result.rscEntry ?? "").href)) as { renderFlight: (input: Record<string, unknown>) => Promise<ReadableStream<Uint8Array>>; modules: Record<string, unknown> };
    const manifest = scanAppDir(root);
    const entry = manifest.entries.find((e) => e.urlPattern === "/");
    // Client components are stubbed as client references in the rsc realm, so only route and action modules are mapped.
    expect(Object.keys(rsc.modules).sort()).toEqual(["app/actions.ts", "app/layout.tsx", "app/page.tsx"]);
    const flight = await new Response(await rsc.renderFlight({ pathname: "/", search: "", params: {}, entry })).text();
    expect(flight).toContain("Hello smoke");
    expect(flight).toContain("app/components/Counter.tsx");
    const ssr = (await import(Bun.pathToFileURL(result.ssrEntry ?? "").href)) as { renderHtml: (flight: ReadableStream<Uint8Array>, options?: Record<string, unknown>) => Promise<ReadableStream<Uint8Array>> };
    const html = await new Response(await ssr.renderHtml(await rsc.renderFlight({ pathname: "/", search: "", params: {}, entry }), { bootstrapModules: ["/_yolo/static/chunks/x.js"] })).text();
    expect(html).toContain("<title>Smoke</title>");
    expect(html).toContain("<h1>Hello smoke</h1>");
    expect(html).toContain("<button>2</button>");
    expect(html).toContain("self.__yolo_f");
    expect(html.trimEnd().endsWith("</body></html>")).toBe(true);
  }, 30_000);
});

describe("webpack-worker engine on the build output", () => {
  test("resolveRscEngine picks the bundles and serves HTML, Flight and actions", async () => {
    const { resolveRscEngine } = await import("../src/rsc-engine.ts");
    const { serverReferenceId } = await import("../src/plugin.ts");
    const config = await resolveYoloConfig(defineYoloConfig({}), { root });
    const engine = await resolveRscEngine({ root, config, packageResolve: false, candidates: [] });
    expect(engine.name).toBe("webpack-worker");
    const html = await new Response(await engine.renderHtml({ request: new Request("http://localhost/?q=1") })).text();
    expect(html).toContain("<h1>Hello smoke</h1>");
    expect(html).toContain('<script type="module" src="/_yolo/static/chunks/entry-client-');
    expect(html).toContain("self.__yolo_f");
    const flight = await new Response(await engine.renderFlight({ request: new Request("http://localhost/") })).text();
    expect(flight).toContain("Hello smoke");
    const missing = await new Response(await engine.renderHtml({ request: new Request("http://localhost/missing") })).text();
    expect(missing).toContain("404");
    const actionId = serverReferenceId("app/actions.ts", "ping");
    const response = await engine.handleAction!(new Request("http://localhost/", { method: "POST", headers: { "next-action": actionId, "content-type": "text/plain" }, body: "[41]" }), { actionId });
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("text/x-component");
    const payload = await response.text();
    expect(payload).toContain('"ok":true');
    expect(payload).toContain("42");
    const unknown = await engine.handleAction!(new Request("http://localhost/", { method: "POST", headers: { "next-action": "00dead" }, body: "[]" }), {});
    expect(unknown.status).toBe(404);
  }, 30_000);
});

describe("module collection on the fixture", () => {
  test("collectRouteModules and findServerActionModules", () => {
    const manifest = scanAppDir(FIXTURE);
    const modules = collectRouteModules(manifest);
    expect(modules).toContain("app/layout.tsx");
    expect(modules).toContain("app/@modal/default.tsx");
    expect(modules).toContain("app/global-error.tsx");
    expect(modules).toContain("app/sitemap.ts");
    expect(modules).toContain("pages/legacy.tsx");
    expect(modules).not.toContain("app/components/Counter.tsx");
    expect(findServerActionModules(FIXTURE, "app")).toEqual(["app/actions.ts"]);
  });
});
