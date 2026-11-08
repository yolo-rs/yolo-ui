// SPDX-License-Identifier: Apache-2.0
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { defineYoloConfig } from "../src/config.ts";
import { scanAppDir } from "../src/router.ts";
import {
  FULLSTACK_CANDIDATES,
  createWorkerEngine,
  forcedRscEngine,
  reactServerDomWebpackAvailable,
  resolveRscEngine,
} from "../src/rsc-engine.ts";

const root = `${import.meta.dir}/.tmp-engine-${process.pid}`;

beforeAll(() => {
  mkdirSync(`${root}/app/blog/[slug]`, { recursive: true });
  writeFileSync(`${root}/app/layout.tsx`, 'export default function Layout({ children }: { children: React.ReactNode }) { return <html lang="en"><body><nav>engine-nav</nav>{children}</body></html>; }\n');
  writeFileSync(`${root}/app/page.tsx`, "export default function Home() { return <main>engine-home</main>; }\n");
  writeFileSync(`${root}/app/blog/[slug]/page.tsx`, 'export default async function Post({ params }: { params: Promise<{ slug: string }> }) { const { slug } = await params; return <article>{slug}</article>; }\n');
});

afterAll(async () => {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      rmSync(root, { recursive: true, force: true });
      return;
    } catch {
      await Bun.sleep(100);
    }
  }
});

const config = defineYoloConfig({});

describe("resolveRscEngine", () => {
  test("forcedRscEngine reads env and experimental.rscEngine", () => {
    expect(forcedRscEngine(undefined, {})).toBeUndefined();
    expect(forcedRscEngine(undefined, { YOLO_RSC_ENGINE: "inline" })).toBe("inline");
    expect(forcedRscEngine(undefined, { YOLO_RSC_ENGINE: "worker" })).toBe("webpack-worker");
    expect(forcedRscEngine(defineYoloConfig({ experimental: { rscEngine: "fullstack" } }), {})).toBe("fullstack");
    expect(forcedRscEngine(undefined, { YOLO_RSC_ENGINE: "bogus" })).toBeUndefined();
  });

  test("prefer: inline always yields the inline engine", async () => {
    const engine = await resolveRscEngine({ root, config, prefer: "inline", log: () => undefined });
    expect(engine.name).toBe("inline");
    const html = await new Response(await engine.renderHtml({ request: new Request("http://localhost/") })).text();
    expect(html).toContain("<main>engine-home</main>");
    expect(html).toContain("<nav>engine-nav</nav>");
  });

  test("without fullstack candidates or bundles the resolver falls back", async () => {
    const logs: string[] = [];
    const engine = await resolveRscEngine({ root, config, candidates: [], packageResolve: false, bundles: false, log: (m) => logs.push(m) });
    expect(engine.name).toBe("inline");
    expect(logs.at(-1)).toBe("rsc engine: inline");
    const forced = await resolveRscEngine({ root, config, candidates: [], packageResolve: false, prefer: "fullstack", log: () => undefined });
    expect(forced.name).toBe("inline");
  });

  test("FULLSTACK_CANDIDATES lists node_modules first and the yolo-ui reference tree", () => {
    const candidates = FULLSTACK_CANDIDATES("C:/proj");
    expect(candidates[0]).toContain("node_modules");
    expect(candidates.some((c) => c.endsWith("yolo-ui/packages/yolo-fullstack"))).toBe(true);
  });

  test("webpack-worker engine renders through the react-server worker when react-server-dom-webpack resolves", async () => {
    expect(reactServerDomWebpackAvailable(root)).toBe(true);
    const manifest = scanAppDir(root);
    const engine = createWorkerEngine({ root, config, manifest, dev: false, log: () => undefined });
    expect(engine?.name).toBe("webpack-worker");
    expect(engine?.details.mode).toBe("worker");
    if (!engine) {
      return;
    }
    try {
      const flight = await new Response(await engine.renderFlight({ request: new Request("http://localhost/blog/worker") })).text();
      expect(flight).toContain('["$","article",null,{"children":"worker"}');
      const html = await new Response(await engine.renderHtml({ request: new Request("http://localhost/blog/worker"), metadata: { title: "Worker" } })).text();
      expect(html).toContain("<article>worker</article>");
      expect(html).toContain("<nav>engine-nav</nav>");
      expect(html).toContain("<title>Worker</title>");
      expect(html).toContain("__next_f");
      const selected = await resolveRscEngine({ root, config, manifest, candidates: [], packageResolve: false, prefer: "webpack-worker", log: () => undefined });
      expect(selected.name).toBe("webpack-worker");
    } finally {
      const html = await new Response(await engine.renderHtml({ request: new Request("http://localhost/nope") })).text();
      expect(html).toContain("404");
    }
  }, 40_000);
});

describe("adaptFullstackModule", () => {
  const text = (stream: ReadableStream<Uint8Array>): Promise<string> => new Response(stream).text();

  test("wraps a fake fullstack module: Flight via the rsc header, actions via yolo-action, lazy build", async () => {
    const { adaptFullstackModule } = await import("../src/rsc-engine.ts");
    const seen: Array<{ url: string; rsc: string | null; action: string | null }> = [];
    let built = 0;
    const fake = {
      buildApp: async () => {
        built += 1;
      },
      loadApp: async () => ({
        fetch: async (request: Request) => {
          seen.push({ url: request.url, rsc: request.headers.get("rsc"), action: request.headers.get("yolo-action") });
          const flight = request.headers.get("rsc") === "1";
          return new Response(flight ? "0:payload\n" : "<!doctype html><p>page</p>", { headers: { "content-type": flight ? "text/x-component" : "text/html" } });
        },
      }),
    };
    const engine = adaptFullstackModule(fake, { root, outDir: `${root}/fs-out` });
    expect(engine?.name).toBe("fullstack");
    expect(built).toBe(0);
    expect(await text(await engine!.renderFlight({ request: new Request("http://localhost/a") }))).toBe("0:payload\n");
    expect(await text(await engine!.renderHtml({ request: new Request("http://localhost/b") }))).toBe("<!doctype html><p>page</p>");
    const action = await engine!.handleAction!(new Request("http://localhost/c", { method: "POST", headers: { "next-action": "00abc" } }), {});
    expect(action.status).toBe(200);
    const whole = await engine!.fetch!(new Request("http://localhost/d"));
    expect(whole.headers.get("content-type")).toBe("text/html");
    expect(built).toBe(1);
    expect(seen.map((s) => s.rsc)).toEqual(["1", null, null, null]);
    expect(seen[2]?.action).toBe("00abc");
    expect(adaptFullstackModule({}, { root })).toBeUndefined();
    expect(adaptFullstackModule(null, { root })).toBeUndefined();
  });

  test("resolveRscEngine picks a candidate directory exposing loadApp", async () => {
    const candidate = `${root}/fake-fullstack`;
    mkdirSync(`${candidate}/src/app`, { recursive: true });
    writeFileSync(`${candidate}/src/app/index.ts`, 'export async function loadApp(outDir: string) { return { fetch: async (request: Request) => new Response(request.headers.get("rsc") ? "0:flight\\n" : "<html>fake " + outDir.length + "</html>") }; }\n');
    const engine = await resolveRscEngine({ root, config, candidates: [candidate], packageResolve: false, bundles: false, fullstackOutDir: `${root}/fs-candidate`, log: () => undefined });
    expect(engine.name).toBe("fullstack");
    expect(engine.details.source?.replace(/\\/g, "/")).toBe(`${candidate}/src/app/index.ts`.replace(/\\/g, "/"));
    expect(await text(await engine.renderFlight({ request: new Request("http://localhost/") }))).toBe("0:flight\n");
    expect(await text(await engine.renderHtml({ request: new Request("http://localhost/") }))).toContain("<html>fake");
  });

  test("render failures of the fullstack handler fall back to the inline engine", async () => {
    const { adaptFullstackModule } = await import("../src/rsc-engine.ts");
    const broken = {
      loadApp: async () => {
        throw new Error("no build");
      },
    };
    const logs: string[] = [];
    const engine = adaptFullstackModule(broken, { root, outDir: `${root}/fs-broken`, log: (m) => logs.push(m) });
    const html = await text(await engine!.renderHtml({ request: new Request("http://localhost/") }));
    expect(html).toContain("<main>engine-home</main>");
    expect(logs.some((m) => m.includes("falling back to inline"))).toBe(true);
  }, 30_000);
});
