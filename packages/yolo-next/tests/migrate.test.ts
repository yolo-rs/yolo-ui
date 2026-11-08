// SPDX-License-Identifier: Apache-2.0
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { analyzeProject, migratePackageJson, migrateProject, migrateScript, renameMiddlewareExport, rewriteNextImports } from "../src/migrate.ts";
import { FIXTURE, removeDir, scratchDir } from "./helpers.ts";

let copy: string;

beforeAll(() => {
  copy = scratchDir("migrate");
  for (const name of readdirSync(FIXTURE)) {
    if (name === "node_modules" || name === ".next" || name === ".yolo") continue;
    cpSync(join(FIXTURE, name), join(copy, name), { recursive: true });
  }
  writeFileSync(join(copy, "package-lock.json"), "{}");
});

afterAll(() => {
  if (copy) removeDir(copy);
});

describe("unit helpers", () => {
  test("migrateScript", () => {
    expect(migrateScript("next dev")).toBe("bun --hot src/server.ts");
    expect(migrateScript("next dev --turbopack -p 4000")).toBe("bun --hot src/server.ts");
    expect(migrateScript("next build")).toBe("bun run src/build.ts");
    expect(migrateScript("next start -p 8080")).toBe("NODE_ENV=production bun src/server.ts");
    expect(migrateScript("next lint")).toBe("bun lint");
    expect(migrateScript("next build && next export")).toBe("bun run src/build.ts && bun run src/build.ts --export");
    expect(migrateScript("eslint .")).toBe("eslint .");
  });

  test("migratePackageJson", () => {
    const pkg: Record<string, unknown> = {
      scripts: { dev: "next dev", build: "next build", start: "next start", lint: "eslint ." },
      dependencies: { next: "16.4.5", react: "19.3.1", "react-dom": "18.3.1", "@tanstack/react-query": "^5.90.0" },
      devDependencies: { "eslint-config-next": "16.4.5", typescript: "^5.9.0" },
      packageManager: "npm@11.0.0",
      engines: { node: ">=20" },
    };
    const notes = migratePackageJson(pkg, { workspace: false });
    expect(pkg.scripts).toEqual({ dev: "bun --hot src/server.ts", build: "bun run src/build.ts", start: "NODE_ENV=production bun src/server.ts", lint: "eslint ." });
    expect(pkg.dependencies).toEqual({ "@tanstack/react-query": "^5.90.0", "@tanstack/react-router": "^1.170.0", "@yolo-rs/next": "^0.1.0", react: "19.3.1", "react-dom": "^19.3.1" });
    expect(pkg.devDependencies).toEqual({ typescript: "^5.9.0" });
    expect(pkg.packageManager).toBe("bun@1.4.4");
    expect(pkg.engines).toEqual({ bun: ">=1.4.4" });
    expect(notes.some((n) => n.startsWith("removed next "))).toBe(true);
    const workspacePkg: Record<string, unknown> = { dependencies: {} };
    migratePackageJson(workspacePkg, { workspace: true });
    expect((workspacePkg.dependencies as Record<string, string>)["@yolo-rs/next"]).toBe("workspace:*");
  });

  test("renameMiddlewareExport and rewriteNextImports", () => {
    expect(renameMiddlewareExport("export async function middleware(req) {}\nexport const config = {};")).toBe("export async function proxy(req) {}\nexport const config = {};");
    expect(renameMiddlewareExport("export default function middleware() {}")).toBe("export default function proxy() {}");
    expect(renameMiddlewareExport("const middleware = () => {};\nexport { middleware };")).toBe("const middleware = () => {};\nexport { proxy };");
    expect(rewriteNextImports('import Link from "next/link";\nimport { headers } from \'next/headers\';\nimport "server-only";\nimport x from "next/unknown";')).toBe('import Link from "@yolo-rs/next/shims/link";\nimport { headers } from \'@yolo-rs/next/shims/headers\';\nimport "@yolo-rs/next/shims/server-only";\nimport x from "next/unknown";');
  });
});

describe("analyzeProject on the fixture", () => {
  test("counts", () => {
    const analysis = analyzeProject(FIXTURE);
    expect(analysis.name).toBe("next-fixture");
    expect(analysis.nextVersion).toBe("16.4.5");
    expect(analysis.configFile).toBe("next.config.ts");
    const c = analysis.counts;
    expect(c.pages).toBe(13);
    expect(c.routes).toBe(20);
    expect(c.routeHandlers).toBe(3);
    expect(c.legacyApiRoutes).toBe(1);
    expect(c.legacyPages).toBe(1);
    expect(c.metadataRoutes).toBe(4);
    expect(c.serverActionFiles).toBe(1);
    expect(c.inlineServerActions).toBe(1);
    expect(c.useClientFiles).toBeGreaterThanOrEqual(5);
    expect(c.useCacheFiles).toBe(1);
    expect(c.forceDynamic).toBe(0);
    expect(c.isrPages).toBe(2);
    expect(c.generateStaticParams).toBe(1);
    expect(c.nextOgUsage).toBe(1);
    expect(c.nextThirdPartiesUsage).toBe(0);
    expect(c.nextFontUsage).toBe(1);
    expect(c.nextImageUsage).toBe(2);
    expect(c.parallelSlots).toBe(2);
    expect(c.interceptingRoutes).toBe(1);
    expect(c.dynamicRoutes).toBe(7);
    expect(c.i18nMessageDirs).toBe(0);
    expect(c.sourceFiles).toBeGreaterThan(40);
    expect(c.sourceLines).toBeGreaterThan(500);
  });
});

describe("migrateProject", () => {
  test("dry run plans every change without writing", async () => {
    const result = await migrateProject({ root: copy, dryRun: true });
    expect(result.dryRun).toBe(true);
    expect(result.written).toEqual([]);
    const paths = result.changes.map((c) => c.path);
    for (const expected of ["package.json", "package-lock.json", "yolo.config.ts", "src/server.ts", "src/build.ts", "bunfig.toml", "yolo.bundler.ts", ".yolo/route-manifest.json", "tsconfig.json", ".gitignore", "MIGRATION-REPORT.md"]) {
      expect(paths).toContain(expected);
    }
    const pkgChange = result.changes.find((c) => c.path === "package.json");
    const pkg = JSON.parse(pkgChange?.contents ?? "{}") as { scripts: Record<string, string>; dependencies: Record<string, string>; devDependencies: Record<string, string>; packageManager: string };
    expect(pkg.scripts.dev).toBe("bun --hot src/server.ts");
    expect(pkg.scripts.build).toBe("bun run src/build.ts");
    expect(pkg.scripts.start).toBe("NODE_ENV=production bun src/server.ts");
    expect(pkg.dependencies.next).toBeUndefined();
    expect(pkg.devDependencies["eslint-config-next"]).toBeUndefined();
    // The scratch copy is not a workspace member of yolo-stack, so the published range is used.
    expect(pkg.dependencies["@yolo-rs/next"]).toBe("^0.1.0");
    expect(pkg.dependencies["@tanstack/react-router"]).toBe("^1.170.0");
    expect(pkg.packageManager).toBe("bun@1.4.4");
    expect(result.changes.find((c) => c.path === "package-lock.json")?.kind).toBe("delete");
    expect(existsSync(join(copy, "yolo.config.ts"))).toBe(false);
    expect(existsSync(join(copy, "package-lock.json"))).toBe(true);
    expect(result.counts.pages).toBe(13);
    expect(result.counts.routes).toBeGreaterThanOrEqual(15);
    expect(result.counts.routeHandlers).toBe(3);
    expect(result.counts.serverActionFiles).toBe(1);
    expect(result.counts.useClientFiles).toBeGreaterThanOrEqual(5);
    expect(result.counts.generateStaticParams).toBe(1);
    expect(result.report).toContain("# Migration report: next-fixture");
    expect(result.report).toContain("| Pages | 13 |");
    expect(result.report).toContain("| Server action files (`use server`) | 1 |");
    expect(result.config.output).toBe("standalone");
    expect(result.config.proxyFile).toBe("proxy.ts");
    const tsconfig = JSON.parse(result.changes.find((c) => c.path === "tsconfig.json")?.contents ?? "{}") as { compilerOptions: { plugins?: unknown; types: string[]; paths: Record<string, string[]> }; exclude: string[] };
    expect(tsconfig.compilerOptions.plugins).toBeUndefined();
    expect(tsconfig.compilerOptions.types).toEqual(["bun-types", "react", "react-dom"]);
    expect(tsconfig.compilerOptions.paths).toEqual({ "@/*": ["./*"] });
    expect(tsconfig.exclude).toContain(".yolo");
  });

  test("in-place migration writes the files", async () => {
    const result = await migrateProject({ root: copy, mode: "in-place" });
    expect(result.written.length).toBeGreaterThanOrEqual(10);
    expect(existsSync(join(copy, "yolo.config.ts"))).toBe(true);
    expect(existsSync(join(copy, "src", "server.ts"))).toBe(true);
    expect(existsSync(join(copy, "src", "build.ts"))).toBe(true);
    expect(existsSync(join(copy, "bunfig.toml"))).toBe(true);
    expect(existsSync(join(copy, "yolo.bundler.ts"))).toBe(true);
    expect(existsSync(join(copy, ".yolo", "route-manifest.json"))).toBe(true);
    expect(existsSync(join(copy, "MIGRATION-REPORT.md"))).toBe(true);
    expect(existsSync(join(copy, "package-lock.json"))).toBe(false);
    const yoloConfig = readFileSync(join(copy, "yolo.config.ts"), "utf8");
    expect(yoloConfig).toContain("export default defineYoloConfig({");
    expect(yoloConfig).toContain('output: "standalone"');
    expect(yoloConfig).toContain("redirects: async () => [");
    const server = readFileSync(join(copy, "src", "server.ts"), "utf8");
    expect(server).toContain('import createYoloNextApp from "@yolo-rs/next/server";');
    expect(server).toContain("if (import.meta.main) app.listen();");
    expect(readFileSync(join(copy, ".gitignore"), "utf8")).toContain(".yolo/");
    expect(readFileSync(join(copy, "app", "page.tsx"), "utf8")).toContain('from "next/image"');
  });

  test("middleware.ts is renamed to proxy.ts and imports can be rewritten", async () => {
    const dir = scratchDir("migrate-mw");
    try {
      mkdirSync(join(dir, "app"), { recursive: true });
      writeFileSync(join(dir, "package.json"), JSON.stringify({ name: "mw", scripts: { dev: "next dev" }, dependencies: { next: "16.4.5" } }));
      writeFileSync(join(dir, "app", "layout.tsx"), 'import Link from "next/link";\nexport default function L({ children }) { return <html><body><Link href="/">home</Link>{children}</body></html>; }');
      writeFileSync(join(dir, "app", "page.tsx"), "export default function P() { return null }");
      writeFileSync(join(dir, "middleware.ts"), 'import { NextResponse } from "next/server";\nexport function middleware() { return NextResponse.next(); }\nexport const config = { matcher: ["/x"] };\n');
      const result = await migrateProject({ root: dir, rewriteImports: true });
      expect(existsSync(join(dir, "proxy.ts"))).toBe(true);
      expect(existsSync(join(dir, "middleware.ts"))).toBe(false);
      const proxy = readFileSync(join(dir, "proxy.ts"), "utf8");
      expect(proxy).toContain("export function proxy()");
      expect(proxy).toContain('from "@yolo-rs/next/shims/server"');
      expect(readFileSync(join(dir, "app", "layout.tsx"), "utf8")).toContain('from "@yolo-rs/next/shims/link"');
      expect(result.changes.find((c) => c.kind === "rename")?.from).toBe("middleware.ts");
      expect(result.config.proxyFile).toBe("proxy.ts");
      expect(result.diagnostics.some((d) => d.code === "next-config-load-failed")).toBe(false);
    } finally {
      removeDir(dir);
    }
  });
});
