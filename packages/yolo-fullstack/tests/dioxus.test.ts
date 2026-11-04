// SPDX-License-Identifier: MIT OR Apache-2.0
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  callServerFn,
  dioxusBridge,
  dioxusProxy,
  dioxusWebRoutes,
  DioxusServerFnError,
  findDxPublicDir,
  isUnsafeRelativePath,
  serverFnPath,
  xxh64,
  xxh64Decimal,
} from "../src/dioxus/index";

const servers: Array<{ stop(force?: boolean): Promise<void> | void }> = [];
const tempDirs: string[] = [];

afterAll(async () => {
  for (const server of servers) await server.stop(true);
  for (const dir of tempDirs) rmSync(dir, { recursive: true, force: true });
});

describe("xxh64", () => {
  test("matches reference vectors with seed 0", () => {
    expect(xxh64("", 0n)).toBe(0xef46db3751d8e999n);
    expect(xxh64("a", 0n)).toBe(0xd24ec4f1a98c6e5bn);
    expect(xxh64("abc", 0n)).toBe(0x44bc2cf5ad770999n);
  });

  test("decimal rendering is a plain u64", () => {
    expect(xxh64Decimal("")).toBe(0xef46db3751d8e999n.toString(10));
    expect(xxh64Decimal("a")).toMatch(/^\d+$/);
  });
});

describe("serverFnPath", () => {
  test("explicit endpoint skips the hash", () => {
    expect(serverFnPath({ name: "get_user", endpoint: "/users" })).toBe(
      "/api/users",
    );
    expect(
      serverFnPath({ name: "get_user", endpoint: "users", prefix: "/rpc" }),
    ).toBe("/rpc/users");
  });

  test("implicit endpoint appends xxh64(manifestDir:modulePath)", () => {
    const manifestDir = "/home/dev/my_app";
    const modulePath = "my_app::api";
    const expected = `/api/get_user${xxh64(`${manifestDir}:${modulePath}`, 0n).toString(10)}`;
    expect(serverFnPath({ name: "get_user", manifestDir, modulePath })).toBe(
      expected,
    );
  });

  test("missing manifest or module path yields no hash", () => {
    expect(serverFnPath({ name: "get_user" })).toBe("/api/get_user");
    expect(serverFnPath({ name: "get_user", prefix: "/api/" })).toBe(
      "/api/get_user",
    );
    expect(serverFnPath({ name: "get_user", prefix: "" })).toBe("/get_user");
  });
});

describe("callServerFn", () => {
  let baseUrl = "";
  const seen: Array<{
    path: string;
    contentType: string | null;
    accept: string | null;
    body: string;
  }> = [];

  beforeAll(() => {
    const server = Bun.serve({
      port: 0,
      async fetch(request) {
        const url = new URL(request.url);
        const body = await request.text();
        seen.push({
          path: url.pathname,
          contentType: request.headers.get("content-type"),
          accept: request.headers.get("accept"),
          body,
        });
        if (url.pathname.startsWith("/api/fail")) {
          return Response.json(
            { ServerError: { message: "boom" } },
            { status: 500 },
          );
        }
        if (url.pathname.startsWith("/api/echo")) {
          return Response.json({
            method: request.method,
            args:
              request.headers.get("content-type") === "application/json" &&
              body.length > 0
                ? JSON.parse(body)
                : body,
          });
        }
        return new Response("Not Found", { status: 404 });
      },
    });
    servers.push(server);
    baseUrl = `http://127.0.0.1:${server.port}`;
  });

  test("posts JSON and parses JSON", async () => {
    const result = await callServerFn<{ method: string; args: { id: number } }>(
      {
        baseUrl,
        name: "echo",
        args: { id: 7 },
      },
    );
    expect(result.method).toBe("POST");
    expect(result.args).toEqual({ id: 7 });
    const last = seen.at(-1)!;
    expect(last.path).toBe("/api/echo");
    expect(last.contentType).toBe("application/json");
    expect(last.accept).toBe("application/json");
  });

  test("url input encodes as form", async () => {
    await callServerFn({
      baseUrl,
      path: "/api/echo",
      args: { q: "x y" },
      input: "url",
    }).catch(() => undefined);
    const last = seen.at(-1)!;
    expect(last.contentType).toBe("application/x-www-form-urlencoded");
    expect(last.body).toBe("q=x+y");
  });

  test("non-2xx throws DioxusServerFnError with status and body", async () => {
    const error = await callServerFn({ baseUrl, name: "fail", args: {} }).catch(
      (e: unknown) => e,
    );
    expect(error).toBeInstanceOf(DioxusServerFnError);
    const typed = error as DioxusServerFnError;
    expect(typed.status).toBe(500);
    expect(typed.body).toEqual({ ServerError: { message: "boom" } });
    expect(typed.path).toBe("/api/fail");
  });
});

describe("dioxusProxy", () => {
  let upstreamPort = 0;
  let proxyPort = 0;
  let lastUpstream: {
    headers: Record<string, string>;
    body: string;
    method: string;
    path: string;
  } | null = null;

  beforeAll(() => {
    const upstream = Bun.serve({
      port: 0,
      async fetch(request) {
        const headers: Record<string, string> = {};
        request.headers.forEach((value, key) => {
          headers[key] = value;
        });
        const url = new URL(request.url);
        lastUpstream = {
          headers,
          body: await request.text(),
          method: request.method,
          path: url.pathname + url.search,
        };
        if (url.pathname === "/api/teapot")
          return new Response("short and stout", {
            status: 418,
            headers: { "x-up": "1" },
          });
        return Response.json({ ok: true });
      },
    });
    servers.push(upstream);
    upstreamPort = upstream.port!;

    const proxy = dioxusProxy({
      target: `http://127.0.0.1:${upstreamPort}`,
      prefix: "/api",
    });
    const front = Bun.serve({
      port: 0,
      fetch(request) {
        if (proxy.matches(new URL(request.url).pathname)) return proxy(request);
        return new Response("local", { status: 200 });
      },
    });
    servers.push(front);
    proxyPort = front.port!;
  });

  test("matches only the prefix", () => {
    const proxy = dioxusProxy({ target: "http://localhost:1" });
    expect(proxy.matches("/api")).toBe(true);
    expect(proxy.matches("/api/x")).toBe(true);
    expect(proxy.matches("/apix")).toBe(false);
    expect(proxy.matches("/")).toBe(false);
  });

  test("forwards method, headers, body and query", async () => {
    const response = await fetch(
      `http://127.0.0.1:${proxyPort}/api/thing?x=1`,
      {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "x-custom": "yes",
          connection: "keep-alive",
        },
        body: JSON.stringify({ a: 1 }),
      },
    );
    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ ok: true });
    expect(lastUpstream).not.toBeNull();
    const up = lastUpstream!;
    expect(up.method).toBe("POST");
    expect(up.path).toBe("/api/thing?x=1");
    expect(up.body).toBe(JSON.stringify({ a: 1 }));
    expect(up.headers["x-custom"]).toBe("yes");
    expect(up.headers["content-type"]).toBe("application/json");
    expect(up.headers["x-forwarded-host"]).toBe(`127.0.0.1:${proxyPort}`);
    expect(up.headers["x-forwarded-proto"]).toBe("http");
    expect(up.headers["host"]).toBe(`127.0.0.1:${upstreamPort}`);
  });

  test("preserves upstream status and headers", async () => {
    const response = await fetch(`http://127.0.0.1:${proxyPort}/api/teapot`);
    expect(response.status).toBe(418);
    expect(response.headers.get("x-up")).toBe("1");
    expect(await response.text()).toBe("short and stout");
  });

  test("non-matching paths stay local", async () => {
    const response = await fetch(`http://127.0.0.1:${proxyPort}/other`);
    expect(await response.text()).toBe("local");
  });
});

function makeWebDir(): string {
  const dir = mkdtempSync(join(tmpdir(), "yolo-dioxus-web-"));
  tempDirs.push(dir);
  mkdirSync(join(dir, "assets"));
  mkdirSync(join(dir, "wasm"));
  writeFileSync(join(dir, "index.html"), "<!doctype html><title>dx</title>");
  writeFileSync(join(dir, "assets", "app.js"), "console.log('dx');");
  writeFileSync(join(dir, "assets", "style.css"), "body{}");
  writeFileSync(
    join(dir, "wasm", "app.wasm"),
    new Uint8Array([0x00, 0x61, 0x73, 0x6d, 1, 0, 0, 0]),
  );
  writeFileSync(join(dir, "secret.txt"), "nope");
  return dir;
}

describe("dioxusWebRoutes", () => {
  let base = "";

  beforeAll(() => {
    const dir = makeWebDir();
    const server = Bun.serve({
      port: 0,
      routes: dioxusWebRoutes({ dir, spa: true }),
      fetch: () => new Response("fallback", { status: 404 }),
    });
    servers.push(server);
    base = `http://127.0.0.1:${server.port}`;
  });

  test("rejects a missing directory", () => {
    expect(() =>
      dioxusWebRoutes({ dir: join(tmpdir(), "does-not-exist-yolo") }),
    ).toThrow(/not found/);
  });

  test("serves index.html at the prefix", async () => {
    const response = await fetch(`${base}/`);
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain("text/html");
    expect(await response.text()).toContain("<title>dx</title>");
  });

  test("serves assets with content type and immutable cache", async () => {
    const js = await fetch(`${base}/assets/app.js`);
    expect(js.status).toBe(200);
    expect(js.headers.get("content-type")).toContain("text/javascript");
    expect(js.headers.get("cache-control")).toBe(
      "public, max-age=31536000, immutable",
    );
    expect(await js.text()).toBe("console.log('dx');");

    const css = await fetch(`${base}/assets/style.css`);
    expect(css.headers.get("content-type")).toContain("text/css");

    const wasm = await fetch(`${base}/wasm/app.wasm`);
    expect(wasm.status).toBe(200);
    expect(wasm.headers.get("content-type")).toBe("application/wasm");
    expect(wasm.headers.get("cache-control")).toBe(
      "public, max-age=31536000, immutable",
    );
    expect((await wasm.arrayBuffer()).byteLength).toBe(8);
  });

  test("missing asset is 404", async () => {
    const response = await fetch(`${base}/assets/missing.js`);
    expect(response.status).toBe(404);
  });

  test("rejects traversal", async () => {
    expect(isUnsafeRelativePath("assets/../secret.txt")).toBe(true);
    expect(isUnsafeRelativePath("assets\\secret.txt")).toBe(true);
    expect(isUnsafeRelativePath("assets/a\0b")).toBe(true);
    expect(isUnsafeRelativePath("assets/app.js")).toBe(false);

    // The URL parser folds "%2e%2e" into "..", so this resolves to "/secret.txt" and falls to the SPA index.
    const encodedDots = await fetch(`${base}/assets/%2e%2e/secret.txt`);
    expect(encodedDots.status).toBe(200);
    const slashDots = await fetch(`${base}/assets/..%2fsecret.txt`);
    expect(slashDots.status).toBe(403);
    const backslash = await fetch(`${base}/assets/%5c..%5csecret.txt`);
    expect(backslash.status).toBe(403);
    const nul = await fetch(`${base}/wasm/app.wasm%00`);
    expect(nul.status).toBe(403);
    for (const response of [encodedDots, slashDots, backslash, nul])
      expect(await response.text()).not.toContain("nope");
  });

  test("SPA fallback serves index.html for client routes", async () => {
    const response = await fetch(`${base}/dashboard/settings`);
    expect(response.status).toBe(200);
    expect(await response.text()).toContain("<title>dx</title>");
  });

  test("without spa unknown routes hit the server fetch", async () => {
    const dir = makeWebDir();
    const server = Bun.serve({
      port: 0,
      routes: dioxusWebRoutes({ dir, spa: false, prefix: "/app" }),
      fetch: () => new Response("fallback", { status: 404 }),
    });
    servers.push(server);
    const index = await fetch(`http://127.0.0.1:${server.port}/app/`);
    expect(index.status).toBe(200);
    const asset = await fetch(
      `http://127.0.0.1:${server.port}/app/assets/app.js`,
    );
    expect(asset.status).toBe(200);
    const unknown = await fetch(`http://127.0.0.1:${server.port}/app/unknown`);
    expect(unknown.status).toBe(404);
    expect(await unknown.text()).toBe("fallback");
  });
});

describe("dioxusBridge", () => {
  test("routes API to the proxy and everything else to the web build", async () => {
    const upstream = Bun.serve({
      port: 0,
      fetch: (request) =>
        Response.json({ path: new URL(request.url).pathname }),
    });
    servers.push(upstream);
    const dir = makeWebDir();
    const bridge = dioxusBridge({
      target: `http://127.0.0.1:${upstream.port}`,
      dir,
    });
    expect(Object.keys(bridge.routes)).toContain("/api/*");
    const server = Bun.serve({
      port: 0,
      routes: bridge.routes,
      fetch: bridge.fetch,
    });
    servers.push(server);
    const base = `http://127.0.0.1:${server.port}`;

    const api = await fetch(`${base}/api/get_user123`, {
      method: "POST",
      body: "{}",
    });
    expect(await api.json()).toEqual({ path: "/api/get_user123" });
    const index = await fetch(`${base}/`);
    expect(await index.text()).toContain("<title>dx</title>");
    const spa = await fetch(`${base}/some/route`);
    expect(await spa.text()).toContain("<title>dx</title>");
    const js = await fetch(`${base}/assets/app.js`);
    expect(js.headers.get("content-type")).toContain("text/javascript");
  });
});

describe("findDxPublicDir", () => {
  test("locates target/dx/<crate>/<profile>/web/public", () => {
    const crate = mkdtempSync(join(tmpdir(), "yolo-dx-crate-"));
    tempDirs.push(crate);
    expect(findDxPublicDir(crate)).toBeNull();
    const pub = join(crate, "target", "dx", "my_app", "debug", "web", "public");
    mkdirSync(pub, { recursive: true });
    writeFileSync(join(pub, "index.html"), "<!doctype html>");
    expect(findDxPublicDir(crate)).toBe(pub);
    expect(findDxPublicDir(crate, true)).toBeNull();
  });
});
