// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

import {
  defineYoloConfig,
  loadEnvFiles,
  loadNextConfig,
  nextEnvFiles,
  normalizeNextConfig,
  parseBodySizeLimit,
  parseEnvFile,
  publicEnvDefine,
  resolveYoloConfig,
  serializeYoloConfig,
} from "../src/config.ts";

const FIXTURE = join(import.meta.dir, "..", "..", "..", "apps", "next-fixture");

describe("defineYoloConfig", () => {
  test("neutral defaults", () => {
    const c = defineYoloConfig({});
    expect(c.appDir).toBe("app");
    expect(c.distDir).toBe(".yolo");
    expect(c.output).toBe("server");
    expect(c.cdn.staticRoute).toBe("/_yolo/static");
    expect(c.cdn.compatStaticRoutes).toEqual(["/_next/static"]);
    expect(c.cdn.fontsRoute).toBe("/_yolo/fonts");
    expect(c.cdn.immutableCacheControl).toBe("public, max-age=31536000, immutable");
    expect(c.cdn.publicCacheControl).toBe("public, max-age=0");
    expect(c.cdn.dynamicHtmlCacheControl).toBe("private, no-cache, no-store, max-age=0, must-revalidate");
    expect(c.cdn.skewQuery).toBe("dpl");
    expect(c.cdn.skewHeader).toBe("x-deployment-id");
    expect(c.server.port).toBe(Number(Bun.env.PORT ?? 3000));
    expect(c.server.hostname).toBe("0.0.0.0");
    expect(c.server.hmr).toBe(true);
    expect(c.server.browserConsole).toBe(true);
    expect(c.server.compress).toBe(true);
    expect(c.server.poweredByHeader).toBe(true);
    expect(c.server.generateEtags).toBe(true);
    expect(c.server.actionBodySizeLimit).toBe(1_048_576);
    expect(c.server.actionAllowedOrigins).toEqual([]);
    expect(c.react).toEqual({ strictMode: true, compiler: false, fastRefresh: true, cacheComponents: false, typedRoutes: false });
    expect(c.theme).toEqual({ preset: "yolo-neutral", dark: true });
    expect(c.publicEnvPrefixes).toEqual(["NEXT_PUBLIC_", "YOLO_PUBLIC_"]);
    expect(c.images?.route).toBe("/_yolo/image");
    expect(c.images?.compatRoutes).toEqual(["/_next/image"]);
    expect(c.images?.loader).toBe("default");
    expect(c.images?.qualities).toEqual([75]);
    expect(c.images?.deviceSizes).toEqual([640, 750, 828, 1080, 1200, 1920, 2048, 3840]);
    expect(c.cacheLife.default).toEqual({ stale: 300, revalidate: 900, expire: 0xfffffffe });
    expect(c.cacheLife.hours).toEqual({ stale: 300, revalidate: 3600, expire: 86_400 });
    expect(c.bundler.extensions).toEqual([".tsx", ".ts", ".jsx", ".js", ".mjs", ".json"]);
    expect(c.pageExtensions).toEqual(["tsx", "ts", "jsx", "js"]);
    expect(c.trailingSlash).toBe(false);
    expect(c.diagnostics).toEqual([]);
    expect(c.next).toEqual({ experimental: {} });
  });

  test("basePath, assetPrefix, output, removed keys and renames", () => {
    const c = defineYoloConfig({
      basePath: "/docs",
      assetPrefix: "https://cdn.example.com/",
      output: "export",
      deploymentId: "dpl_1",
      poweredByHeader: false,
      generateEtags: false,
      reactStrictMode: false,
      eslint: { ignoreDuringBuilds: true },
      webpack: () => undefined,
      skipMiddlewareUrlNormalize: true,
      experimental: { serverComponentsExternalPackages: ["pg"], ppr: true, typedRoutes: true, turbo: { resolveAlias: { lodash: "underscore" } }, serverActions: { bodySizeLimit: "4mb" } },
      images: { domains: ["img.example.com"], loaderFile: "./loader.ts" },
      cacheLife: { blog: { stale: 10 } },
    });
    expect(c.cdn.basePath).toBe("/docs");
    expect(c.cdn.staticRoute).toBe("/docs/_yolo/static");
    expect(c.cdn.compatStaticRoutes).toEqual(["/docs/_next/static"]);
    expect(c.cdn.assetPrefix).toBe("https://cdn.example.com");
    expect(c.cdn.deploymentId).toBe("dpl_1");
    expect(c.cdn.poweredByHeader).toBe(false);
    expect(c.cdn.generateEtags).toBe(false);
    expect(c.cdn.output).toBe("export");
    expect(c.output).toBe("export");
    expect(c.react.strictMode).toBe(false);
    expect(c.react.cacheComponents).toBe(true);
    expect(c.react.typedRoutes).toBe(true);
    expect(c.server.actionBodySizeLimit).toBe(4 * 1024 * 1024);
    expect(c.images?.route).toBe("/docs/_yolo/image");
    expect(c.images?.loader).toBe("custom");
    expect(c.images?.loaderFile).toBe("./loader.ts");
    expect(c.images?.remotePatterns).toEqual([{ hostname: "img.example.com" }]);
    expect(c.serverExternalPackages).toEqual(["pg"]);
    expect(c.bundler.aliases).toEqual({ lodash: "underscore" });
    expect(c.next.skipProxyUrlNormalize).toBe(true);
    expect(c.cacheLife.blog).toEqual({ stale: 10, revalidate: 900, expire: 0xfffffffe });
    const codes = c.diagnostics.map((d) => d.code);
    expect(codes).toContain("next-config-removed-eslint");
    expect(codes).toContain("next-config-webpack-unsupported");
    expect(codes).toContain("next-config-images-domains-deprecated");
    expect(codes).toContain("next-config-renamed-key");
  });

  test("yolo overrides at the top level and under `yolo`", () => {
    const c = defineYoloConfig({ theme: { preset: "yolo-aphrody", dark: false }, yolo: { appDir: "src/app", publicEnvPrefixes: ["PUBLIC_"] } });
    expect(c.theme).toEqual({ preset: "yolo-aphrody", dark: false });
    expect(c.appDir).toBe("src/app");
    expect(c.publicEnvPrefixes).toEqual(["PUBLIC_"]);
    expect("yolo" in c).toBe(false);
  });

  test("a resolved config passes through", () => {
    const c = defineYoloConfig({});
    expect(defineYoloConfig(c)).toBe(c);
  });

  test("normalizeNextConfig reports every removed key once", () => {
    const { diagnostics, removed, renamed } = normalizeNextConfig({ amp: {}, serverRuntimeConfig: {}, publicRuntimeConfig: {}, experimental: { dynamicIO: true, useCache: true } });
    expect(removed).toEqual(["amp", "serverRuntimeConfig", "publicRuntimeConfig"]);
    expect(renamed.map(([from]) => from)).toEqual(["experimental.dynamicIO", "experimental.useCache"]);
    expect(diagnostics.filter((d) => d.code.startsWith("next-config-removed-"))).toHaveLength(3);
  });
});

describe("parseBodySizeLimit", () => {
  test("units and defaults", () => {
    expect(parseBodySizeLimit("1mb")).toBe(1_048_576);
    expect(parseBodySizeLimit("2mb")).toBe(2_097_152);
    expect(parseBodySizeLimit("500kb")).toBe(512_000);
    expect(parseBodySizeLimit("1gb")).toBe(1_073_741_824);
    expect(parseBodySizeLimit("64b")).toBe(64);
    expect(parseBodySizeLimit(123)).toBe(123);
    expect(parseBodySizeLimit("bogus")).toBe(1_048_576);
    expect(parseBodySizeLimit(undefined)).toBe(1_048_576);
  });
});

describe("fixture next.config.ts", () => {
  test("loadNextConfig evaluates the default export without a shell", async () => {
    const loaded = await loadNextConfig(FIXTURE);
    expect(loaded.file).toBe("next.config.ts");
    expect(loaded.wrappers).toEqual([]);
    expect(typeof loaded.config.redirects).toBe("function");
    expect(loaded.config.output).toBe("standalone");
  });

  test("resolveYoloConfig awaits the rule functions and detects the layout", async () => {
    const loaded = await loadNextConfig(FIXTURE);
    const c = await resolveYoloConfig(loaded.config, { root: FIXTURE });
    expect(c.redirects).toHaveLength(3);
    expect(c.redirects[0]).toEqual({ source: "/old-blog/:slug", destination: "/blog/:slug", permanent: true });
    expect(c.rewrites.beforeFiles).toHaveLength(1);
    expect(c.rewrites.afterFiles).toHaveLength(1);
    expect(c.rewrites.fallback).toHaveLength(1);
    expect(c.headers).toHaveLength(2);
    expect(c.server.actionBodySizeLimit).toBe(2_097_152);
    expect(c.server.actionAllowedOrigins).toEqual(["app.example.com"]);
    expect(c.cacheLife.blog).toEqual({ stale: 3600, revalidate: 900, expire: 86_400 });
    expect(c.cacheLife.seconds).toEqual({ stale: 30, revalidate: 1, expire: 60 });
    expect(c.diagnostics.some((d) => d.code === "next-config-removed-eslint")).toBe(true);
    expect(c.bundler.plugins.map((p) => p.kind)).toEqual(["svg-react", "text", "text", "loader-override"]);
    expect(c.bundler.plugins[3]?.loader).toBe("yaml");
    expect(c.bundler.aliases.underscore).toBe("lodash");
    expect(c.bundler.browserAliases).toEqual({ mocha: "mocha/browser-entry.js", fs: "./empty.ts" });
    expect(c.bundler.define.__BUILD_FLAG__).toBe("true");
    expect(c.bundler.define["process.env.APP_NAME"]).toBe('"Yolo Fixture"');
    expect(c.bundler.external).toEqual(["sharp", "pino"]);
    expect(c.images?.qualities).toEqual([50, 75, 90]);
    expect(c.images?.formats).toEqual(["image/avif", "image/webp"]);
    expect(c.images?.minimumCacheTtl).toBe(86_400);
    expect(c.images?.remotePatterns).toHaveLength(3);
    expect(c.images?.localPatterns).toEqual([{ pathname: "/assets/images/**", search: "" }]);
    expect(c.output).toBe("standalone");
    expect(c.cdn.output).toBe("standalone");
    expect(c.env).toEqual({ APP_NAME: "Yolo Fixture", API_BASE: "https://api.example.com" });
    expect(c.react).toEqual({ strictMode: true, compiler: true, fastRefresh: true, cacheComponents: true, typedRoutes: true });
    expect(c.transpilePackages).toEqual(["@acme/ui"]);
    expect(c.serverExternalPackages).toEqual(["sharp", "pino"]);
    expect(c.pageExtensions).toEqual(["tsx", "ts", "mdx"]);
    expect(c.appDir).toBe("app");
    expect(c.pagesDir).toBe("pages");
    expect(c.proxyFile).toBe("proxy.ts");
    expect(c.cdn.poweredByHeader).toBe(false);
    expect(c.server.poweredByHeader).toBe(false);
    expect(typeof c.next.redirects).toBe("function");
  });

  test("serializeYoloConfig re-emits resolved rules as async functions", async () => {
    const loaded = await loadNextConfig(FIXTURE);
    const c = await resolveYoloConfig(loaded.config, { root: FIXTURE });
    const ts = serializeYoloConfig(c, { root: FIXTURE });
    expect(ts).toContain('import { defineYoloConfig } from "@yolo-rs/next/config";');
    expect(ts).toContain("export default defineYoloConfig({");
    expect(ts).toContain("redirects: async () => [");
    expect(ts).toContain("rewrites: async () => {");
    expect(ts).toContain("headers: async () => [");
    expect(ts).toContain('"/old-blog/:slug"');
    expect(ts).toContain('preset: "yolo-neutral"');
    expect(ts).not.toContain("eslint");
    expect(ts).not.toContain("root:");
  });
});

describe("env files", () => {
  test("nextEnvFiles order, test mode and parsing", () => {
    const dir = mkdtempSync(join(tmpdir(), "yolo-env-"));
    try {
      writeFileSync(join(dir, ".env"), "A=base\nB=base\n# comment\nC='single quoted'\nD=\"escaped\\nvalue\"\nexport E=exported # trailing\n");
      writeFileSync(join(dir, ".env.local"), "A=local\n");
      writeFileSync(join(dir, ".env.production"), "B=prod\n");
      writeFileSync(join(dir, ".env.production.local"), "B=prodlocal\n");
      expect(nextEnvFiles(dir, "production").map((f) => basename(f))).toEqual([".env.production.local", ".env.local", ".env.production", ".env"]);
      expect(nextEnvFiles(dir, "test").map((f) => basename(f))).toEqual([".env"]);
      expect(nextEnvFiles(dir, "development").map((f) => basename(f))).toEqual([".env.local", ".env"]);
      const env = loadEnvFiles(dir, "production");
      expect(env.A).toBe("local");
      expect(env.B).toBe("prodlocal");
      expect(env.C).toBe("single quoted");
      expect(env.D).toBe("escaped\nvalue");
      expect(env.E).toBe("exported");
      expect(parseEnvFile("X=1\nY = two\n")).toEqual({ X: "1", Y: "two" });
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("publicEnvDefine only inlines the public prefixes", () => {
    expect(publicEnvDefine(["NEXT_PUBLIC_", "YOLO_PUBLIC_"], { NEXT_PUBLIC_A: "1", SECRET: "x", YOLO_PUBLIC_B: "two", NEXT_PUBLIC_C: undefined })).toEqual({
      "process.env.NEXT_PUBLIC_A": '"1"',
      "process.env.YOLO_PUBLIC_B": '"two"',
    });
  });
});
