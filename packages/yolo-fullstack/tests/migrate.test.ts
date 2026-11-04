// SPDX-License-Identifier: MIT OR Apache-2.0
// `migrateNextProject` against a synthetic Next.js project.
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  formatMigrationReport,
  migrateNextProject,
  rewriteScript,
  type MigrationReport,
} from "../src/migrate";

let root: string;

function write(path: string, text: string): void {
  const file = join(root, path);
  mkdirSync(join(file, ".."), { recursive: true });
  writeFileSync(file, text);
}

const PACKAGE_JSON = {
  name: "demo",
  private: true,
  scripts: {
    dev: "next dev --turbopack -p 4000",
    build: "next build",
    start: "next start -p 3000 && echo ok",
    lint: "next lint --fix",
    test: "bun test",
  },
  dependencies: { next: "16.0.0", react: "^19.2.0", "react-dom": "^19.2.0" },
  devDependencies: { "eslint-config-next": "16.0.0", typescript: "^5.9.0" },
};

const TSCONFIG = `{
  // Next.js defaults
  "compilerOptions": {
    "strict": true,
    "plugins": [{ "name": "next" }],
    "paths": { "@/*": ["./src/*"] }, /* alias */
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx"],
  "exclude": ["node_modules"],
}
`;

beforeAll(() => {
  root = mkdtempSync(join(tmpdir(), "yolo-migrate-"));
  write("package.json", `${JSON.stringify(PACKAGE_JSON, null, 2)}\n`);
  write("tsconfig.json", TSCONFIG);
  write(".gitignore", "node_modules\n.next/\n");
  write(
    "next.config.mjs",
    `export default {
  output: "export",
  i18n: { locales: ["en", "fr"], defaultLocale: "en" },
  images: { loaderFile: "./image-loader.ts" },
  webpack(config) { return config; },
};
`,
  );
  write("image-loader.ts", "export default ({ src }) => src;\n");
  write(
    "app/layout.tsx",
    "export default function Layout({ children }) { return children; }\n",
  );
  write(
    "app/page.tsx",
    'export const runtime = "edge";\nexport default function Page() { return null; }\n',
  );
  write(
    "app/@modal/default.tsx",
    "export default function Modal() { return null; }\n",
  );
  write(
    "app/@modal/(.)photo/page.tsx",
    "export default function Photo() { return null; }\n",
  );
  write(
    "app/photo/page.tsx",
    "export default function Photo() { return null; }\n",
  );
  write(
    "app/og/route.tsx",
    'import { ImageResponse } from "next/og";\nimport { Inter } from "next/font/google";\nexport function GET() { return new ImageResponse(null); }\n',
  );
  write(
    "app/client.tsx",
    '"use client";\nimport dynamic from "next/dynamic";\nimport Script from "next/script";\nexport default dynamic(() => import("./x"));\n',
  );
  write(
    "app/cached.ts",
    '"use cache";\nimport { unstable_cache } from "next/cache";\nexport const x = unstable_cache(async () => 1);\n',
  );
  write(
    "app/draft.ts",
    'import { draftMode } from "next/headers";\nexport const on = async () => (await draftMode()).isEnabled;\n',
  );
  write("instrumentation.ts", "export function register() {}\n");
  write(
    "middleware.ts",
    'export function middleware() {}\nexport const config = { matcher: ["/((?!api|_next).*)"] };\n',
  );
  write(
    "pages/old.tsx",
    'export async function getServerSideProps() { return { props: {} }; }\nimport { useRouter } from "next/router";\nexport default function Old() { return null; }\n',
  );
  // Skipped directories must not leak into the report.
  write("node_modules/next/og.js", 'import "next/og";\n');
  write(".next/server/app/page.js", 'import "next/og";\n');
});

afterAll(() => {
  if (root) rmSync(root, { recursive: true, force: true });
});

const kinds = (findings: MigrationReport["unsupported"], kind: string) =>
  findings.filter((f) => f.kind === kind);

describe("migrateNextProject", () => {
  let report: MigrationReport;

  test("builds the report", async () => {
    report = await migrateNextProject(root);
    expect(report.errors).toEqual([]);
    expect(report.router).toBe("both");
  });

  test("reports unsupported routes and APIs with project-relative files", () => {
    expect(
      kinds(report.unsupported, "parallel-route").map((f) => f.file),
    ).toEqual(["app/@modal"]);
    expect(
      kinds(report.unsupported, "intercepting-route").map((f) => f.file),
    ).toEqual(["app/@modal/(.)photo"]);
    expect(kinds(report.unsupported, "next/og").map((f) => f.file)).toEqual([
      "app/og/route.tsx",
    ]);
    expect(
      kinds(report.unsupported, "edge-runtime").map((f) => f.file),
    ).toEqual(["app/page.tsx"]);
    expect(kinds(report.unsupported, "use-cache").map((f) => f.file)).toEqual([
      "app/cached.ts",
    ]);
    expect(
      kinds(report.unsupported, "unstable_cache").map((f) => f.file),
    ).toEqual(["app/cached.ts"]);
    expect(
      kinds(report.unsupported, "instrumentation").map((f) => f.file),
    ).toEqual(["instrumentation.ts"]);
    expect(
      kinds(report.unsupported, "pages-data-fetching").map((f) => f.file),
    ).toEqual(["pages/old.tsx"]);
    expect(kinds(report.unsupported, "next/router").map((f) => f.file)).toEqual(
      ["pages/old.tsx"],
    );
    expect(
      kinds(report.unsupported, "pages-router").map((f) => f.file),
    ).toEqual(["pages"]);
    const draft = kinds(report.unsupported, "draftMode");
    expect(draft.map((f) => f.file)).toEqual(["app/draft.ts"]);
    expect(draft[0]!.level).toBe("limited");
    for (const finding of report.unsupported) {
      expect(finding.file ?? "").not.toContain("node_modules");
      expect(finding.file ?? "").not.toContain(".next");
      expect(finding.file ?? "").not.toContain("\\");
    }
  });

  test("reports next.config keys", () => {
    const keys = report.unsupported
      .filter((f) => f.kind.startsWith("next.config:"))
      .map((f) => f.kind);
    expect(keys).toContain("next.config:webpack");
    expect(keys).toContain("next.config:i18n");
    expect(
      report.unsupported.find((f) => f.kind === "next.config:i18n")!.file,
    ).toBe("next.config.mjs");
    expect(kinds(report.supported, "next.config:output")[0]!.detail).toContain(
      "--prerender",
    );
    expect(
      kinds(report.supported, "next.config:images.loaderFile"),
    ).toHaveLength(1);
  });

  test("lists supported mappings", () => {
    expect(kinds(report.supported, "next/font").map((f) => f.file)).toEqual([
      "app/og/route.tsx",
    ]);
    expect(kinds(report.supported, "next/dynamic").map((f) => f.file)).toEqual([
      "app/client.tsx",
    ]);
    expect(kinds(report.supported, "next/script").map((f) => f.file)).toEqual([
      "app/client.tsx",
    ]);
    expect(
      kinds(report.supported, "middleware.matcher").map((f) => f.file),
    ).toEqual(["middleware.ts"]);
  });

  test("plans scripts, dependencies, tsconfig and .gitignore", () => {
    expect(
      Object.fromEntries(report.scripts.map((s) => [s.name, s.to])),
    ).toEqual({
      dev: "yolo-fullstack dev --port 4000",
      build: "yolo-fullstack build",
      start: "yolo-fullstack start --port 3000 && echo ok",
      lint: "bun lint",
    });
    expect(report.dependencies.remove).toEqual(["next", "eslint-config-next"]);
    expect(Object.keys(report.dependencies.add).sort()).toEqual([
      "@yolo-rs/fullstack",
      "react-server-dom-parcel",
    ]);
    expect(report.dependencies.warnings).toEqual([]);
    expect(report.tsconfig).toEqual({
      file: "tsconfig.json",
      removeNextPlugin: true,
      removeInclude: ["next-env.d.ts"],
    });
    expect(report.gitignore).toEqual({
      file: ".gitignore",
      add: [".yolo/", "dist/"],
    });
  });

  test("formats as plain text", () => {
    const text = formatMigrationReport(report);
    expect(text).toContain("router: both");
    expect(text).toContain("parallel-route");
    expect(text).toMatch(
      /dev\s+next dev --turbopack -p 4000\s+->\s+yolo-fullstack dev --port 4000/,
    );
    expect(text).toContain("remove next");
    expect(/[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/u.test(text)).toBe(false);
  });

  test("apply rewrites package.json, tsconfig.json and .gitignore without touching sources", async () => {
    const applied = await migrateNextProject(root, { apply: true });
    expect(applied.applied).toEqual([
      "package.json",
      "tsconfig.json",
      ".gitignore",
      "yolo-migration-report.json",
    ]);

    const text = readFileSync(join(root, "package.json"), "utf8");
    expect(text.endsWith("}\n")).toBe(true);
    expect(text).toContain(
      '\n  "scripts": {\n    "dev": "yolo-fullstack dev --port 4000",',
    );
    const pkg = JSON.parse(text);
    expect(pkg.scripts.lint).toBe("bun lint");
    expect(pkg.scripts.test).toBe("bun test");
    expect(pkg.dependencies.next).toBeUndefined();
    expect(pkg.devDependencies["eslint-config-next"]).toBeUndefined();
    expect(pkg.devDependencies.typescript).toBe("^5.9.0");
    expect(pkg.dependencies["@yolo-rs/fullstack"]).toMatch(/^\^\d/);
    expect(pkg.dependencies["react-server-dom-parcel"]).toBeDefined();

    const tsconfig = JSON.parse(
      readFileSync(join(root, "tsconfig.json"), "utf8"),
    );
    expect(tsconfig.compilerOptions.plugins).toBeUndefined();
    expect(tsconfig.compilerOptions.strict).toBe(true);
    expect(tsconfig.compilerOptions.paths).toEqual({ "@/*": ["./src/*"] });
    expect(tsconfig.include).toEqual(["**/*.ts", "**/*.tsx"]);

    expect(readFileSync(join(root, ".gitignore"), "utf8")).toBe(
      "node_modules\n.next/\n.yolo/\ndist/\n",
    );
    const saved = JSON.parse(
      readFileSync(join(root, "yolo-migration-report.json"), "utf8"),
    );
    expect(saved.router).toBe("both");

    for (const file of [
      "app/page.tsx",
      "app/@modal/(.)photo/page.tsx",
      "pages/old.tsx",
      "next.config.mjs",
      "instrumentation.ts",
    ])
      expect(existsSync(join(root, file))).toBe(true);

    // A second run has nothing left to rewrite but the report.
    const again = await migrateNextProject(root, { apply: true });
    expect(again.scripts).toEqual([]);
    expect(again.dependencies.remove).toEqual([]);
    expect(again.tsconfig.removeNextPlugin).toBe(false);
    expect(again.gitignore.add).toEqual([]);
    expect(again.applied).toEqual(["yolo-migration-report.json"]);
  });

  test("reports a broken next.config instead of throwing", async () => {
    const broken = mkdtempSync(join(tmpdir(), "yolo-migrate-broken-"));
    try {
      writeFileSync(join(broken, "next.config.js"), "module.exports = 42;\n");
      const result = await migrateNextProject(broken);
      expect(result.router).toBe("none");
      expect(result.errors.some((e) => e.startsWith("next.config:"))).toBe(
        true,
      );
      expect(result.errors.some((e) => e.startsWith("package.json:"))).toBe(
        true,
      );
    } finally {
      rmSync(broken, { recursive: true, force: true });
    }
  });
});

describe("rewriteScript", () => {
  test("maps commands and flags", () => {
    expect(rewriteScript("next dev")).toBe("yolo-fullstack dev");
    expect(rewriteScript("next dev --turbo -H 0.0.0.0 --port=5000")).toBe(
      "yolo-fullstack dev --hostname 0.0.0.0 --port=5000",
    );
    expect(rewriteScript("next build --debug && next start")).toBe(
      "yolo-fullstack build --debug && yolo-fullstack start",
    );
    expect(rewriteScript("next lint --dir src; next build")).toBe(
      "bun lint; yolo-fullstack build",
    );
    expect(rewriteScript("vitest run")).toBe("vitest run");
  });
});
