// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";

import { defineYoloConfig } from "../src/config.ts";
import { applyDestination, applyRules, compileSource, conditionMatches, parseCookies, redirectStatus, rulesManifestJson } from "../src/rules.ts";
import type { ResolvedYoloConfig } from "../src/types.ts";

const facts = (init: { headers?: Record<string, string>; cookies?: Record<string, string>; query?: string; host?: string } = {}) => ({
  headers: new Headers(init.headers ?? {}),
  cookies: init.cookies ?? {},
  query: new URLSearchParams(init.query ?? ""),
  host: init.host ?? "example.com",
});

describe("compileSource", () => {
  test("/old/:slug", () => {
    const rule = compileSource("/old/:slug");
    expect(rule.match("/old/hello")).toEqual({ slug: "hello" });
    expect(rule.match("/old/hello/")).toEqual({ slug: "hello" });
    expect(rule.match("/old/a/b")).toBeUndefined();
    expect(rule.match("/old")).toBeUndefined();
    expect(rule.bunPattern).toBe("/old/:slug");
    expect(rule.axumPattern).toBe("/old/{slug}");
    expect(rule.regexSource).toBe("^/old(?:/(?<slug>[^/]+?))(?:/)?$");
    expect(rule.keys).toEqual(["slug"]);
  });

  test("/docs/:path*", () => {
    const rule = compileSource("/docs/:path*");
    expect(rule.match("/docs/a/b")).toEqual({ path: "a/b" });
    expect(rule.match("/docs")).toEqual({});
    expect(rule.match("/other")).toBeUndefined();
    expect(rule.bunPattern).toBe("/docs/*");
    expect(rule.axumPattern).toBe("/docs/{*path}");
  });

  test("/(.*) wildcard and /:id(\\d+) custom regex", () => {
    const any = compileSource("/(.*)");
    expect(any.match("/")).toEqual({ "0": "" });
    expect(any.match("/anything/here")).toEqual({ "0": "anything/here" });
    expect(any.bunPattern).toBe("/*");
    expect(() => new RegExp(any.regexSource)).not.toThrow();
    const digits = compileSource("/:id(\\d+)");
    expect(digits.match("/42")).toEqual({ id: "42" });
    expect(digits.match("/abc")).toBeUndefined();
    expect(digits.regexSource).toBe("^(?:/(?<id>\\d+))(?:/)?$");
    const plus = compileSource("/u/:parts+");
    expect(plus.match("/u")).toBeUndefined();
    expect(plus.match("/u/a/b")).toEqual({ parts: "a/b" });
    const optional = compileSource("/x/:maybe?");
    expect(optional.match("/x")).toEqual({});
    expect(optional.match("/x/y")).toEqual({ maybe: "y" });
  });

  test("percent-decoding of params", () => {
    expect(compileSource("/u/:name").match("/u/caf%C3%A9")).toEqual({ name: "café" });
  });
});

describe("destinations and statuses", () => {
  test("applyDestination", () => {
    expect(applyDestination("/blog/:slug", { slug: "x" })).toBe("/blog/x");
    expect(applyDestination("https://legacy.example.com/:path*", { path: "a/b" })).toBe("https://legacy.example.com/a/b");
    expect(applyDestination("https://api.example.com/:path*", {})).toBe("https://api.example.com/");
    expect(applyDestination("/x/:0", { "0": "wild" })).toBe("/x/wild");
  });

  test("redirectStatus", () => {
    expect(redirectStatus({ source: "/a", destination: "/b" })).toBe(307);
    expect(redirectStatus({ source: "/a", destination: "/b", permanent: true })).toBe(308);
    expect(redirectStatus({ source: "/a", destination: "/b", permanent: true, statusCode: 301 })).toBe(301);
  });
});

describe("conditionMatches", () => {
  test("header, cookie, query, host", () => {
    expect(conditionMatches({ type: "header", key: "x-legacy", value: "1" }, facts({ headers: { "x-legacy": "1" } }))).toBe(true);
    expect(conditionMatches({ type: "header", key: "x-legacy", value: "1" }, facts({ headers: { "x-legacy": "2" } }))).toBe(false);
    expect(conditionMatches({ type: "header", key: "x-legacy" }, facts({ headers: { "x-legacy": "anything" } }))).toBe(true);
    expect(conditionMatches({ type: "header", key: "x-legacy" }, facts())).toBe(false);
    expect(conditionMatches({ type: "cookie", key: "session", value: "s_.*" }, facts({ cookies: { session: "s_123" } }))).toBe(true);
    expect(conditionMatches({ type: "cookie", key: "session" }, facts())).toBe(false);
    expect(conditionMatches({ type: "query", key: "page", value: "\\d+" }, facts({ query: "page=12" }))).toBe(true);
    expect(conditionMatches({ type: "query", key: "page", value: "\\d+" }, facts({ query: "page=x" }))).toBe(false);
    expect(conditionMatches({ type: "host", value: "example.com" }, facts({ host: "example.com:3000" }))).toBe(true);
    expect(conditionMatches({ type: "host", value: "other.com" }, facts())).toBe(false);
    const params: Record<string, string> = {};
    expect(conditionMatches({ type: "header", key: "x-tenant", value: "first-(?<tenant>.*)" }, facts({ headers: { "x-tenant": "first-acme" } }), params)).toBe(true);
    expect(params).toEqual({ tenant: "acme" });
  });

  test("parseCookies", () => {
    expect(parseCookies("a=1; b=two%20words; a=dup; broken")).toEqual({ a: "1", b: "two words" });
    expect(parseCookies(null)).toEqual({});
  });
});

function fixtureConfig(): ResolvedYoloConfig {
  const base = defineYoloConfig({});
  return {
    ...base,
    redirects: [
      { source: "/old-blog/:slug", destination: "/blog/:slug", permanent: true },
      { source: "/docs", destination: "/docs/getting-started", permanent: false },
      { source: "/legacy/:path*", destination: "https://legacy.example.com/:path*", permanent: false, has: [{ type: "header", key: "x-legacy", value: "1" }] },
    ],
    rewrites: {
      beforeFiles: [{ source: "/api/proxy/:path*", destination: "https://api.example.com/:path*" }],
      afterFiles: [{ source: "/u/:username", destination: "/users/:username" }],
      fallback: [{ source: "/:path*", destination: "https://fallback.example.com/:path*" }],
    },
    headers: [
      { source: "/(.*)", headers: [{ key: "X-Frame-Options", value: "DENY" }, { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains; preload" }] },
      { source: "/_next/static/:path*", headers: [{ key: "Cache-Control", value: "public, max-age=31536000, immutable" }] },
    ],
  };
}

describe("applyRules", () => {
  const config = fixtureConfig();

  test("redirects produce a 308/307 response with Location", () => {
    const permanent = applyRules(new Request("http://localhost/old-blog/hello?x=1"), config, "redirects");
    expect(permanent.response?.status).toBe(308);
    expect(permanent.response?.headers.get("location")).toBe("/blog/hello?x=1");
    const temporary = applyRules(new Request("http://localhost/docs"), config, "redirects");
    expect(temporary.response?.status).toBe(307);
    expect(temporary.response?.headers.get("location")).toBe("/docs/getting-started");
    expect(applyRules(new Request("http://localhost/legacy/a/b"), config, "redirects").response).toBeUndefined();
    const external = applyRules(new Request("http://localhost/legacy/a/b", { headers: { "x-legacy": "1" } }), config, "redirects");
    expect(external.response?.headers.get("location")).toBe("https://legacy.example.com/a/b");
    expect(applyRules(new Request("http://localhost/blog/x"), config, "redirects")).toEqual({});
  });

  test("rewrites produce a rewritten URL", () => {
    const before = applyRules(new Request("http://localhost/api/proxy/v1/users?page=2"), config, "beforeFiles");
    expect(before.rewrittenUrl?.href).toBe("https://api.example.com/v1/users?page=2");
    const after = applyRules(new Request("http://localhost/u/alice"), config, "afterFiles");
    expect(after.rewrittenUrl?.pathname).toBe("/users/alice");
    expect(after.rewrittenUrl?.origin).toBe("http://localhost");
    const fallback = applyRules(new Request("http://localhost/missing/page"), config, "fallback");
    expect(fallback.rewrittenUrl?.href).toBe("https://fallback.example.com/missing/page");
    expect(applyRules(new Request("http://localhost/u/alice"), config, "beforeFiles")).toEqual({});
  });

  test("headers merge every matching rule", () => {
    const result = applyRules(new Request("http://localhost/_next/static/chunks/a.js"), config, "headers");
    expect(result.headers?.get("x-frame-options")).toBe("DENY");
    expect(result.headers?.get("cache-control")).toBe("public, max-age=31536000, immutable");
    const page = applyRules(new Request("http://localhost/blog/x"), config, "headers");
    expect(page.headers?.get("cache-control")).toBeNull();
    expect(page.headers?.get("strict-transport-security")).toContain("max-age=63072000");
  });

  test("basePath prefixes the sources unless basePath: false", () => {
    const withBase: ResolvedYoloConfig = { ...config, cdn: { ...config.cdn, basePath: "/app" }, redirects: [{ source: "/a", destination: "/b", permanent: true }, { source: "/raw", destination: "/r", basePath: false }] };
    expect(applyRules(new Request("http://localhost/app/a"), withBase, "redirects").response?.headers.get("location")).toBe("/b");
    expect(applyRules(new Request("http://localhost/a"), withBase, "redirects").response).toBeUndefined();
    expect(applyRules(new Request("http://localhost/raw"), withBase, "redirects").response?.status).toBe(307);
  });

  test("rulesManifestJson carries regexes", () => {
    const json = rulesManifestJson(config) as { redirects: Array<{ regex: string }>; headers: Array<{ regex: string }> };
    expect(json.redirects[0]?.regex).toBe("^/old-blog(?:/(?<slug>[^/]+?))(?:/)?$");
    expect(json.headers).toHaveLength(2);
  });
});
