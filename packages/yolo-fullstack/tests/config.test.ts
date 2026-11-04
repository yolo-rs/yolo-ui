// SPDX-License-Identifier: MIT OR Apache-2.0
import { describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { PHASE_DEVELOPMENT_SERVER, envDefines, loadNextConfig, resolveNextConfig } from "../src/config/next-config";
import {
  afterRoutes,
  beforeRoutes,
  compileRoutes,
  compileSource,
  fillDestination,
  matchSource,
  normalizePath,
  responseHeaders,
} from "../src/config/routes";
import {
  aliasConditions,
  pickAliasTarget,
  ruleGlobToRegExp,
  ruleLoader,
  stripBundlerFlags,
  svgrModule,
  turbopackPlugin,
} from "../src/config/turbopack";

describe("next.config", () => {
  test("defaults and normalization", async () => {
    const config = await resolveNextConfig({}, PHASE_DEVELOPMENT_SERVER);
    expect(config.basePath).toBe("");
    expect(config.images.path).toBe("/_next/image");
    expect(config.images.deviceSizes).toContain(1920);
    expect(config.rewrites).toEqual({ beforeFiles: [], afterFiles: [], fallback: [] });
    expect(config.unsupported).toEqual([]);
  });

  test("functions, images.domains, unsupported keys and env defines", async () => {
    const config = await resolveNextConfig(
      {
        basePath: "/docs",
        assetPrefix: "https://cdn.example.com/",
        env: { FOO: "bar" },
        images: { domains: ["img.example.com"], remotePatterns: [new URL("https://cdn.example.com/media/**")] },
        rewrites: async () => [{ source: "/a", destination: "/b" }],
        redirects: () => [{ source: "/old", destination: "/new", permanent: true }],
        webpack: () => undefined,
        experimental: { ppr: true },
      },
      PHASE_DEVELOPMENT_SERVER,
      undefined,
      { NEXT_PUBLIC_API: "x", SECRET: "y" },
    );
    expect(config.assetPrefix).toBe("https://cdn.example.com");
    expect(config.rewrites.afterFiles).toHaveLength(1);
    expect(config.redirects[0]?.permanent).toBe(true);
    expect(config.images.remotePatterns).toEqual([
      { protocol: "https", hostname: "cdn.example.com", port: undefined, pathname: "/media/**", search: undefined },
      { protocol: "https", hostname: "img.example.com" },
    ]);
    expect(config.unsupported.map(u => u.key)).toEqual(
      expect.arrayContaining(["webpack", "experimental.ppr", "images.domains"]),
    );
    expect(config.define).toEqual({ "process.env.NEXT_PUBLIC_API": '"x"', "process.env.FOO": '"bar"' });
    expect(envDefines({ env: {} }, { SECRET: "y" })).toEqual({});
  });

  test("loads next.config.ts (function form) from disk", async () => {
    const dir = mkdtempSync(join(tmpdir(), "yolo-config-"));
    try {
      writeFileSync(
        join(dir, "next.config.ts"),
        `export default (phase: string) => ({ basePath: "/" + phase.split("-")[1] });`,
      );
      const config = await loadNextConfig(dir, { phase: PHASE_DEVELOPMENT_SERVER });
      expect(config.file).toBe(join(dir, "next.config.ts"));
      expect(config.basePath).toBe("/development");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});

describe("routes", () => {
  test("source patterns", () => {
    expect(matchSource(compileSource("/blog/:slug"), "/blog/hello")).toEqual({ slug: "hello" });
    expect(matchSource(compileSource("/blog/:slug"), "/blog/a/b")).toBeUndefined();
    expect(matchSource(compileSource("/docs/:path*"), "/docs")).toEqual({ path: undefined });
    expect(matchSource(compileSource("/docs/:path*"), "/docs/a/b")).toEqual({ path: "a/b" });
    expect(matchSource(compileSource("/post/:id(\\d+)"), "/post/12")).toEqual({ id: "12" });
    expect(matchSource(compileSource("/post/:id(\\d+)"), "/post/ab")).toBeUndefined();
    expect(matchSource(compileSource("/(.*)"), "/anything/here")).toEqual({});
    expect(fillDestination("/new/:slug/:path*", { slug: "x", path: "a/b" })).toBe("/new/x/a/b");
  });

  const rules = compileRoutes({
    basePath: "",
    trailingSlash: false,
    skipTrailingSlashRedirect: false,
    poweredByHeader: true,
    redirects: [
      { source: "/old/:slug", destination: "/new/:slug", permanent: false },
      { source: "/gated", destination: "/login", has: [{ type: "cookie", key: "session" }], permanent: true },
      { source: "/ext", destination: "https://example.com/:q", has: [{ type: "query", key: "q" }] },
    ],
    rewrites: {
      beforeFiles: [{ source: "/first", destination: "/about" }],
      afterFiles: [{ source: "/legacy/:slug", destination: "/blog/:slug?from=legacy" }],
      fallback: [{ source: "/proxy/:path*", destination: "https://upstream.test/:path*" }],
    },
    headers: [
      {
        source: "/api/:path*",
        headers: [{ key: "x-api", value: "v1-:path*" }],
        missing: [{ type: "header", key: "x-internal" }],
      },
    ],
  });
  const req = (path: string, init?: RequestInit) => new Request(`http://h${path}`, init);

  test("redirects with params, conditions and external targets", () => {
    const a = beforeRoutes(rules, req("/old/x"), new URL("http://h/old/x"));
    expect(a.kind === "response" && a.response.status).toBe(307);
    expect(a.kind === "response" && a.response.headers.get("location")).toBe("/new/x");
    const none = beforeRoutes(rules, req("/gated"), new URL("http://h/gated"));
    expect(none.kind).toBe("next");
    const gated = beforeRoutes(rules, req("/gated", { headers: { cookie: "session=1" } }), new URL("http://h/gated"));
    expect(gated.kind === "response" && gated.response.status).toBe(308);
    const ext = beforeRoutes(rules, req("/ext?q=z"), new URL("http://h/ext?q=z"));
    expect(ext.kind === "response" && ext.response.headers.get("location")).toBe("https://example.com/z");
  });

  test("rewrites keep unconsumed query parameters", () => {
    const before = beforeRoutes(rules, req("/first?x=1"), new URL("http://h/first?x=1"));
    expect(before.kind === "rewrite" && before.url.pathname + before.url.search).toBe("/about?x=1");
    const after = afterRoutes(rules, req("/legacy/p"), new URL("http://h/legacy/p"), "afterFiles");
    expect(after.kind === "rewrite" && after.url.pathname + after.url.search).toBe("/blog/p?from=legacy");
    const fallback = afterRoutes(rules, req("/proxy/a/b"), new URL("http://h/proxy/a/b"), "fallback");
    expect(fallback.kind === "rewrite" && fallback.external).toBe(true);
    expect(fallback.kind === "rewrite" && fallback.url.href).toBe("https://upstream.test/a/b");
  });

  test("headers with captures and missing conditions, x-powered-by", () => {
    expect(responseHeaders(rules, req("/api/users"), new URL("http://h/api/users"))).toEqual([
      ["x-api", "v1-users"],
      ["x-powered-by", "@yolo-rs/fullstack"],
    ]);
    expect(
      responseHeaders(rules, req("/api/users", { headers: { "x-internal": "1" } }), new URL("http://h/api/users")),
    ).toEqual([["x-powered-by", "@yolo-rs/fullstack"]]);
  });

  test("basePath and trailingSlash normalization", () => {
    const withBase = { ...rules, basePath: "/docs", trailingSlash: true };
    expect(normalizePath(withBase, new URL("http://h/docs/guide/")).pathname).toBe("/guide");
    expect(normalizePath(withBase, new URL("http://h/docs/guide")).redirect?.headers.get("location")).toBe(
      "/docs/guide/",
    );
    expect(normalizePath(withBase, new URL("http://h/docs")).pathname).toBe("/");
    expect(normalizePath(withBase, new URL("http://h/other")).pathname).toBe("/other");
    expect(normalizePath(rules, new URL("http://h/a/")).redirect?.headers.get("location")).toBe("/a");
    expect(normalizePath(rules, new URL("http://h/file.txt/")).redirect?.headers.get("location")).toBe("/file.txt");
  });
});

describe("turbopack", () => {
  test("alias targets by layer", () => {
    expect(aliasConditions("client")[0]).toBe("browser");
    expect(pickAliasTarget({ browser: "./b.ts", default: "./d.ts" }, "client")).toBe("./b.ts");
    expect(pickAliasTarget({ browser: "./b.ts", default: "./d.ts" }, "rsc")).toBe("./d.ts");
    expect(pickAliasTarget({ browser: "./b.ts" }, "ssr")).toBeUndefined();
    expect(pickAliasTarget(["a", "b"], "client")).toEqual(["a", "b"]);
  });

  test("rule globs and loaders", () => {
    expect(ruleGlobToRegExp("*.svg").test("src/icon.svg")).toBe(true);
    expect(ruleGlobToRegExp("*.svg").test("src/icon.svgx")).toBe(false);
    expect(ruleGlobToRegExp("./src/**/*.{yml,yaml}").test("src/a/b/c.yaml")).toBe(true);
    expect(ruleGlobToRegExp("**/*.md").test("docs/x.md")).toBe(true);
    expect(ruleLoader({ loaders: ["raw-loader"] })).toEqual({ loader: "text" });
    expect(ruleLoader({ loaders: [{ loader: "@svgr/webpack" }, "url-loader"] })).toEqual({
      loader: "svgr",
      reason: "only the first loader (@svgr/webpack) is applied",
    });
    expect(ruleLoader({ loaders: ["yaml-loader"] }).reason).toContain("no Bun equivalent");
  });

  test("svgr module and plugin unsupported report", () => {
    const code = svgrModule('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 0"/></svg>');
    expect(code).toContain('"viewBox":"0 0 10 10"');
    expect(code).not.toContain("xmlns");
    expect(code).toContain("export const ReactComponent");
    const mapping = turbopackPlugin(
      { rules: { "*.yaml": { loaders: ["yaml-loader"] } }, moduleIds: "named" },
      { root: tmpdir(), layer: "client" },
    );
    expect(mapping.unsupported.map(u => u.key)).toEqual(["turbopack.rules[*.yaml]", "turbopack.moduleIds"]);
  });

  test("bundler flags are stripped", () => {
    expect(stripBundlerFlags(["dev", "--turbopack", "-p", "4000"])).toEqual({
      argv: ["dev", "-p", "4000"],
      turbopack: true,
      webpack: false,
    });
  });
});
