// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { analyzeSegmentModule, evaluateLiteral, inlineDirectives, legacyDataFunctions, routeHandlerMethods, scanModuleExports } from "../src/segments.ts";
import { FIXTURE } from "./helpers.ts";

const read = (file: string): string => readFileSync(join(FIXTURE, file), "utf8");

describe("analyzeSegmentModule on the fixture", () => {
  test("blog page", () => {
    const config = analyzeSegmentModule(read("app/blog/[slug]/page.tsx"), "app/blog/[slug]/page.tsx");
    expect(config.hasGenerateStaticParams).toBe(true);
    expect(config.hasGenerateMetadata).toBe(true);
    expect(config.hasGenerateViewport).toBe(false);
    expect(config.revalidate).toBe(60);
    expect(config.dynamicParams).toBe(true);
    expect(config.directive).toBe("none");
    expect(config.defaultExport).toBe("BlogPost");
    expect(config.defaultExportAsync).toBe(true);
    expect(config.namedExports).toEqual(["dynamicParams", "revalidate", "generateStaticParams", "generateMetadata"]);
    expect(config.nextImports).toEqual(["next", "next/navigation"]);
    expect(config.diagnostics).toEqual([]);
  });

  test("pricing page: static metadata and the deprecated edge runtime", () => {
    const config = analyzeSegmentModule(read("app/(marketing)/pricing/page.tsx"), "app/(marketing)/pricing/page.tsx");
    expect(config.staticMetadata).toEqual({ title: "Pricing" });
    expect(config.runtime).toBe("edge");
    expect(config.diagnostics.map((d) => d.code)).toEqual(["segment-config-edge-runtime-deprecated"]);
    expect(config.diagnostics[0]?.file).toBe("app/(marketing)/pricing/page.tsx");
  });

  test("root layout: metadata literal with new URL, viewport, next imports", () => {
    const config = analyzeSegmentModule(read("app/layout.tsx"), "app/layout.tsx");
    expect(config.staticMetadata?.metadataBase).toBe("https://fixture.example.com/");
    expect(config.staticMetadata?.title).toEqual({ default: "Yolo Fixture", template: "%s | Yolo Fixture" });
    expect(config.staticMetadata?.keywords).toEqual(["yolo", "next", "bun"]);
    expect((config.staticMetadata?.openGraph as { images: unknown[] }).images).toEqual([{ url: "/opengraph-image", width: 1200, height: 630 }]);
    expect(config.staticViewport?.themeColor).toEqual([
      { media: "(prefers-color-scheme: dark)", color: "#09090b" },
      { media: "(prefers-color-scheme: light)", color: "#ffffff" },
    ]);
    expect(config.staticViewport?.colorScheme).toBe("dark light");
    expect(config.nextImports).toEqual(["next", "next/font/google", "next/font/local", "next/script"]);
    expect(config.defaultExport).toBe("RootLayout");
  });

  test("Counter: use client with hooks", () => {
    const config = analyzeSegmentModule(read("app/components/Counter.tsx"), "app/components/Counter.tsx");
    expect(config.directive).toBe("use-client");
    expect(config.hooksUsed).toEqual(["useState", "useActionState", "useOptimistic", "useTransition", "useRouter", "usePathname", "useSearchParams"]);
    expect(config.namedExports).toEqual(["Counter"]);
    expect(config.defaultExport).toBeUndefined();
    expect(config.nextImports).toEqual(["next/navigation"]);
  });

  test("data.ts: function-level use cache, actions.ts: file-level use server", () => {
    const data = read("app/lib/data.ts");
    expect(analyzeSegmentModule(data, "app/lib/data.ts").directive).toBe("none");
    expect(inlineDirectives(data)).toEqual({ useServer: [], useCache: ["getPosts", "getPost"] });
    expect(analyzeSegmentModule(read("app/actions.ts"), "app/actions.ts").directive).toBe("use-server");
    expect(inlineDirectives(read("app/actions.ts"))).toEqual({ useServer: [], useCache: [] });
    expect(inlineDirectives(read("app/dashboard/page.tsx"))).toEqual({ useServer: ["refreshNow"], useCache: [] });
    expect(analyzeSegmentModule(read("app/dashboard/page.tsx"), "app/dashboard/page.tsx").maxDuration).toBe(30);
  });

  test("other segment configs and diagnostics", () => {
    expect(analyzeSegmentModule(read("app/docs/[...slug]/page.tsx"), "x").dynamic).toBe("force-static");
    expect(analyzeSegmentModule(read("app/shop/[[...filters]]/page.tsx"), "x").fetchCache).toBe("force-cache");
    expect(analyzeSegmentModule(read("app/page.tsx"), "x").revalidate).toBe(3600);
    const ppr = analyzeSegmentModule('export const experimental_ppr = true;\nexport const preferredRegion = ["iad1"];\nexport default function P() {}', "p.tsx");
    expect(ppr.experimentalPpr).toBe(true);
    expect(ppr.preferredRegion).toEqual(["iad1"]);
    expect(ppr.diagnostics.map((d) => d.code)).toEqual(["segment-config-experimental-ppr-removed", "segment-config-preferred-region-deprecated"]);
  });
});

describe("route handlers and legacy pages", () => {
  test("methods in canonical order", () => {
    expect(routeHandlerMethods(read("app/api/posts/[id]/route.ts"))).toEqual(["GET", "POST", "DELETE"]);
    expect(routeHandlerMethods(read("app/api/health/route.ts"))).toEqual(["GET"]);
    expect(routeHandlerMethods("export const DELETE = () => {}; export { handler as GET }; const handler = () => {};")).toEqual(["GET", "DELETE"]);
  });

  test("legacy data functions", () => {
    expect(legacyDataFunctions(read("pages/legacy.tsx"))).toEqual(["getServerSideProps"]);
    expect(legacyDataFunctions("export async function getStaticProps() {}\nexport async function getStaticPaths() {}\nPage.getInitialProps = async () => ({});")).toEqual(["getStaticProps", "getStaticPaths", "getInitialProps"]);
  });
});

describe("export scanning and literal evaluation", () => {
  test("scanModuleExports", () => {
    const exports = scanModuleExports('export const a = 1, b = 2;\nexport function c() {}\nexport default async function D() {}\nexport { x as y, z };\nexport type T = string;\nexport * from "./w";\nconst x = 1, z = 2;');
    expect(exports.names).toEqual(expect.arrayContaining(["a", "c", "y", "z"]));
    expect(exports.names).not.toContain("T");
    expect(exports.hasDefault).toBe(true);
    expect(exports.defaultName).toBe("D");
    expect(exports.defaultAsync).toBe(true);
    expect(exports.reexportAll).toEqual(["./w"]);
  });

  test("evaluateLiteral", () => {
    expect(evaluateLiteral('{ a: "x", b: [1, -2.5, true, null], c: { d: `tpl` }, e: new URL("/p", "https://h.io"), f: undefined, }')).toEqual({ a: "x", b: [1, -2.5, true, null], c: { d: "tpl" }, e: "https://h.io/p", f: undefined });
    expect(evaluateLiteral("{ a: fn() }")).toBeUndefined();
    expect(evaluateLiteral("{ a: `${x}` }")).toBeUndefined();
    expect(evaluateLiteral("60 as const")).toBe(60);
    expect(evaluateLiteral("false")).toBe(false);
  });
});
