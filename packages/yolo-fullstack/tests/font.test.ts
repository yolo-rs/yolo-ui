// SPDX-License-Identifier: MIT OR Apache-2.0
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  familyFromExport,
  familyParam,
  filterSubsets,
  googleCssUrl,
  googleFont,
  parseFontFaces,
} from "../src/font/google";
import {
  balancedEnd,
  findFontCalls,
  findFontImports,
  fontVirtualModule,
  materializeFont,
  prologueEnd,
  rewriteFonts,
  type FontArtifact,
} from "../src/font/index";
import { localFont } from "../src/font/local";

let dir: string;
beforeAll(() => {
  dir = mkdtempSync(join(tmpdir(), "yolo-font-"));
});
afterAll(() => rmSync(dir, { recursive: true, force: true }));

const GOOGLE_CSS = `/* cyrillic */
@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 100 900;
  font-display: swap;
  src: url(https://fonts.gstatic.com/s/inter/v18/cyr.woff2) format('woff2');
  unicode-range: U+0301, U+0400-045F;
}
/* latin */
@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 100 900;
  font-display: swap;
  src: url(https://fonts.gstatic.com/s/inter/v18/latin.woff2) format('woff2');
  unicode-range: U+0000-00FF;
}
`;

describe("google", () => {
  test("family names and CSS API parameters", () => {
    expect(familyFromExport("Roboto_Mono")).toBe("Roboto Mono");
    expect(familyParam("Inter", {})).toBe("Inter:wght@100..900");
    expect(familyParam("Inter", { weight: ["400", "700"] })).toBe("Inter:wght@400;700");
    expect(familyParam("Roboto", { weight: "400", style: ["normal", "italic"] })).toBe("Roboto:ital,wght@0,400;1,400");
    expect(familyParam("Roboto Mono", { weight: "variable", style: "italic" })).toBe(
      "Roboto+Mono:ital,wght@1,100..900",
    );
    expect(googleCssUrl("Roboto Mono", { weight: "400", display: "optional" })).toBe(
      "https://fonts.googleapis.com/css2?family=Roboto+Mono:wght@400&display=optional",
    );
  });

  test("parses and filters subsets", () => {
    const faces = parseFontFaces(GOOGLE_CSS);
    expect(faces.map(f => f.subset)).toEqual(["cyrillic", "latin"]);
    expect(faces[1]?.urls).toEqual(["https://fonts.gstatic.com/s/inter/v18/latin.woff2"]);
    expect(filterSubsets(faces, ["latin"]).map(f => f.subset)).toEqual(["latin"]);
    expect(filterSubsets(faces, undefined)).toHaveLength(2);
  });

  test("downloads once into the cache and self-hosts", async () => {
    const calls: string[] = [];
    const fakeFetch: typeof fetch = (async (input: string | URL | Request) => {
      const url = String(input);
      calls.push(url);
      if (url.startsWith("https://fonts.googleapis.com/"))
        return new Response(GOOGLE_CSS, { headers: { "content-type": "text/css" } });
      return new Response(new Uint8Array([1, 2, 3]));
    }) as typeof fetch;
    const cacheDir = join(dir, "google");
    const artifact = await googleFont(
      "Inter",
      { subsets: ["latin"], variable: "--font-inter" },
      { cacheDir, fetch: fakeFetch },
    );
    expect(artifact.family).toBe("Inter");
    expect(artifact.preload).toHaveLength(1);
    expect(artifact.style.fontFamily).toBe("'Inter'");
    const css = readFileSync(artifact.cssFile, "utf8");
    expect(css).toContain("url(./files/");
    expect(css).not.toContain("gstatic");
    expect(css).toContain(`.${artifact.className}{font-family:'Inter';}`);
    expect(css).toContain(`.${artifact.variableClassName}{--font-inter:'Inter';}`);
    expect(calls).toHaveLength(2);
    const again = await googleFont(
      "Inter",
      { subsets: ["latin"], variable: "--font-inter" },
      { cacheDir, fetch: fakeFetch, offline: true },
    );
    expect(again.id).toBe(artifact.id);
    expect(calls).toHaveLength(2);
  });

  test("offline cache miss fails", async () => {
    await expect(googleFont("Nope", {}, { cacheDir: join(dir, "google"), offline: true })).rejects.toThrow("offline");
  });
});

describe("local", () => {
  test("generates @font-face per file with relative urls", async () => {
    const project = join(dir, "local");
    mkdirSync(join(project, "fonts"), { recursive: true });
    writeFileSync(join(project, "fonts", "Brand-Regular.woff2"), "x");
    writeFileSync(join(project, "fonts", "Brand-Bold.woff2"), "y");
    const artifact = await localFont(
      {
        src: [
          { path: "./fonts/Brand-Regular.woff2", weight: "400" },
          { path: "./fonts/Brand-Bold.woff2", weight: "700" },
        ],
        variable: "--font-brand",
        fallback: ["Georgia", "serif"],
      },
      { cacheDir: join(dir, "cache"), importerDir: project },
    );
    expect(artifact.family).toBe("Brand-Regular");
    expect(artifact.style.fontFamily).toBe("'Brand-Regular', 'Georgia', serif");
    const css = readFileSync(artifact.cssFile, "utf8");
    expect(css.match(/@font-face/g)).toHaveLength(2);
    expect(css).toContain("font-weight:700");
    expect(css).toMatch(/url\("\.\.\/.*Brand-Bold\.woff2"\)/);
    const built = materializeFont(artifact, join(dir, "client", "media"), "/_yolo/media");
    expect(built.publicPreloads).toHaveLength(2);
    expect(readFileSync(built.builtCssFile!, "utf8")).toMatch(
      /url\("\/_yolo\/media\/fonts\/Brand-Bold-[a-z0-9]+\.woff2"\)/,
    );
  });

  test("missing file is an error", async () => {
    await expect(localFont({ src: "./nope.woff2" }, { cacheDir: dir, importerDir: dir })).rejects.toThrow("not found");
  });
});

describe("rewrite", () => {
  const source = `"use client";
// comment
import { Inter, Roboto_Mono as Mono } from "next/font/google";
import localFont from 'next/font/local';
import { useState } from "react";

const inter = Inter({ subsets: ["latin"], display: "swap" });
const mono = Mono({ weight: ["400"] });
const brand = localFont({ src: "./fonts/Brand-Regular.woff2" });
export const style = inter.className + mono.className + brand.className;
`;

  test("finds imports, calls and the prologue", () => {
    const bindings = findFontImports(source);
    expect(bindings.map(b => [b.local, b.imported, b.kind])).toEqual([
      ["Inter", "Inter", "google"],
      ["Mono", "Roboto_Mono", "google"],
      ["localFont", "default", "local"],
    ]);
    const calls = findFontCalls(source, bindings, "test.tsx");
    expect(calls.map(c => c.options)).toEqual([
      { subsets: ["latin"], display: "swap" },
      { weight: ["400"] },
      { src: "./fonts/Brand-Regular.woff2" },
    ]);
    expect(prologueEnd(source)).toBe(source.indexOf("import { Inter"));
    expect(balancedEnd("f(a, (b), 'x)')", 1)).toBe(15);
    expect(() =>
      findFontCalls(
        "import { Inter } from 'next/font/google'; Inter(require('x'));",
        findFontImports("import { Inter } from 'next/font/google'; Inter(require('x'));"),
        "t",
      ),
    ).toThrow("object literal");
  });

  test("rewrites calls to virtual modules with a fallback when offline in dev", async () => {
    const project = join(dir, "rewrite");
    mkdirSync(join(project, "fonts"), { recursive: true });
    writeFileSync(join(project, "fonts", "Brand-Regular.woff2"), "x");
    const warnings: string[] = [];
    const registry = new Map<string, FontArtifact>();
    const result = await rewriteFonts(source, join(project, "layout.tsx"), {
      cacheDir: join(dir, "rw-cache"),
      mode: "dev",
      offline: true,
      registry,
      warn: m => warnings.push(m),
    });
    expect(result).toBeDefined();
    expect(result!.code).toMatch(/^"use client";\n\/\/ comment\n\s*import __yolo_font_0 from "yolo-font:/);
    expect(result!.code).not.toContain("next/font");
    expect(result!.code).toContain("const inter = __yolo_font_0;");
    expect(result!.code).toContain("const brand = __yolo_font_2;");
    expect(result!.code).toContain('import { useState } from "react";');
    expect(result!.artifacts).toHaveLength(3);
    expect(result!.artifacts[0]?.fallback).toBe(true);
    expect(warnings).toHaveLength(2);
    const client = fontVirtualModule(result!.artifacts[2]!, "client");
    expect(client).toContain('import "');
    expect(client).toContain('"className":"yf_');
    expect(fontVirtualModule(result!.artifacts[0]!, "rsc")).not.toContain("import");
  });

  test("build mode without the fallback flag fails offline", async () => {
    await expect(
      rewriteFonts(`import { Inter } from "next/font/google"; export const f = Inter({});`, join(dir, "x.ts"), {
        cacheDir: join(dir, "rw2"),
        mode: "build",
        offline: true,
        registry: new Map(),
        env: {},
      }),
    ).rejects.toThrow();
  });

  test("modules without fonts are untouched", async () => {
    expect(
      await rewriteFonts("export const a = 1;", "a.ts", { cacheDir: dir, mode: "dev", registry: new Map() }),
    ).toBeUndefined();
  });
});
