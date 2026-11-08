// SPDX-License-Identifier: Apache-2.0
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { brotliCompressSync } from "node:zlib";
import {
  GOOGLE_FONTS_USER_AGENT,
  computeFallbackOverrides,
  createFontsHandler,
  css2Url,
  fontPreloadLinks,
  googleFont,
  listFontRequests,
  localFont,
  parseFontFaces,
  parseSfntMetrics,
  pickFallbackFont,
  prepareFonts,
  presetFontStack,
  resetFontRegistry,
  yoloFont,
} from "../src/fonts.ts";
import { ARIAL_METRICS, TIMES_NEW_ROMAN_METRICS } from "../src/types.ts";

const encoder = new TextEncoder();

function concat(parts: ReadonlyArray<Uint8Array>): Uint8Array {
  const out = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}

function headTable(unitsPerEm: number): Uint8Array {
  const table = new Uint8Array(54);
  new DataView(table.buffer).setUint16(18, unitsPerEm);
  return table;
}

function hheaTable(ascender: number, descender: number, lineGap: number): Uint8Array {
  const table = new Uint8Array(36);
  const view = new DataView(table.buffer);
  view.setInt16(4, ascender);
  view.setInt16(6, descender);
  view.setInt16(8, lineGap);
  return table;
}

function os2Table(version: number, xAvgCharWidth: number, capHeight: number): Uint8Array {
  const table = new Uint8Array(version >= 2 ? 96 : 78);
  const view = new DataView(table.buffer);
  view.setUint16(0, version);
  view.setInt16(2, xAvgCharWidth);
  if (version >= 2) {
    view.setInt16(88, capHeight);
  }
  return table;
}

type Tables = ReadonlyArray<readonly [string, Uint8Array]>;

function buildSfnt(tables: Tables, prefix = new Uint8Array(0)): Uint8Array {
  const directorySize = 12 + tables.length * 16;
  let cursor = prefix.length + directorySize;
  const offsets: number[] = [];
  for (const [, data] of tables) {
    offsets.push(cursor);
    cursor += (data.length + 3) & ~3;
  }
  const out = new Uint8Array(cursor);
  out.set(prefix, 0);
  const view = new DataView(out.buffer);
  view.setUint32(prefix.length, 0x00010000);
  view.setUint16(prefix.length + 4, tables.length);
  tables.forEach(([name, data], index) => {
    const record = prefix.length + 12 + index * 16;
    out.set(encoder.encode(name), record);
    view.setUint32(record + 8, offsets[index] ?? 0);
    view.setUint32(record + 12, data.length);
    out.set(data, offsets[index] ?? 0);
  });
  return out;
}

const KNOWN_INDEX: Readonly<Record<string, number>> = { cmap: 0, head: 1, hhea: 2, hmtx: 3, maxp: 4, name: 5, "OS/2": 6, post: 7 };

function base128(value: number): number[] {
  const bytes: number[] = [];
  let rest = value;
  do {
    bytes.unshift(rest & 0x7f);
    rest = Math.floor(rest / 128);
  } while (rest > 0);
  for (let index = 0; index < bytes.length - 1; index += 1) {
    bytes[index] = (bytes[index] ?? 0) | 0x80;
  }
  return bytes;
}

function buildWoff2(tables: Tables, explicitTags = false): Uint8Array {
  const directory: number[] = [];
  for (const [name, data] of tables) {
    const known = KNOWN_INDEX[name];
    if (explicitTags || known === undefined) {
      directory.push(63, ...encoder.encode(name));
    } else {
      directory.push(known);
    }
    directory.push(...base128(data.length));
  }
  const compressed = new Uint8Array(brotliCompressSync(concat(tables.map(([, data]) => data))));
  const header = new Uint8Array(48);
  const view = new DataView(header.buffer);
  header.set(encoder.encode("wOF2"), 0);
  view.setUint32(4, 0x00010000);
  view.setUint32(8, 48 + directory.length + compressed.length);
  view.setUint16(12, tables.length);
  view.setUint32(16, 12 + tables.length * 16 + tables.reduce((total, [, data]) => total + ((data.length + 3) & ~3), 0));
  view.setUint32(20, compressed.length);
  return concat([header, new Uint8Array(directory), compressed]);
}

const arialLikeTables: Tables = [
  ["head", headTable(2048)],
  ["hhea", hheaTable(1854, -434, 67)],
  ["OS/2", os2Table(4, 904, 1467)],
];
const sfntBytes = buildSfnt(arialLikeTables);

describe("css2Url", () => {
  test("family, weights, italics, ranges and display", () => {
    expect(css2Url("Inter")).toBe("https://fonts.googleapis.com/css2?family=Inter&display=swap");
    expect(css2Url("Roboto Mono", { weight: ["400", "700"] })).toBe("https://fonts.googleapis.com/css2?family=Roboto+Mono:wght@400;700&display=swap");
    expect(css2Url("Inter", { weight: ["700", "400"], style: ["normal", "italic"] })).toBe(
      "https://fonts.googleapis.com/css2?family=Inter:ital,wght@0,400;0,700;1,400;1,700&display=swap",
    );
    expect(css2Url("Inter", { weight: "100 900" })).toBe("https://fonts.googleapis.com/css2?family=Inter:wght@100..900&display=swap");
    expect(css2Url("Inter", { weight: "variable", display: "optional" })).toBe("https://fonts.googleapis.com/css2?family=Inter&display=optional");
    expect(css2Url("Lora", { style: "italic", weight: "400" })).toBe("https://fonts.googleapis.com/css2?family=Lora:ital,wght@1,400&display=swap");
    expect(GOOGLE_FONTS_USER_AGENT).toContain("Chrome/104.0.0.0");
  });
});

describe("font objects", () => {
  test("googleFont returns classes, variable and style", () => {
    resetFontRegistry();
    const inter = googleFont("Inter", { subsets: ["latin"], display: "swap", variable: "--font-inter" });
    expect(inter.className).toMatch(/^yolo-font-[0-9a-f]{8}$/);
    expect(inter.variable).toBe(`yolo-font-var-${inter.className.slice("yolo-font-".length)}`);
    expect(inter.style.fontFamily).toBe("'Inter', 'Inter Fallback'");
    expect(inter.style.fontWeight).toBeUndefined();
    const mono = googleFont("Roboto Mono", { weight: ["400"], style: "normal", fallback: ["monospace"], adjustFontFallback: false });
    expect(mono.style).toEqual({ fontFamily: "'Roboto Mono', monospace", fontWeight: 400, fontStyle: "normal" });
    expect(mono.variable).toBe("");
    expect(listFontRequests().map((plan) => plan.family)).toEqual(["Inter", "Roboto Mono"]);
    expect(googleFont("Inter", { subsets: ["latin"], display: "swap", variable: "--font-inter" }).className).toBe(inter.className);
  });

  test("yoloFont builds the plain object", () => {
    expect(yoloFont({ className: "a", style: { fontFamily: "x" } })).toEqual({ className: "a", style: { fontFamily: "x" } });
    expect(yoloFont({ className: "a", variable: "v", style: { fontFamily: "x", fontWeight: 700 } })).toEqual({
      className: "a",
      variable: "v",
      style: { fontFamily: "x", fontWeight: 700 },
    });
  });
});

describe("SFNT metrics", () => {
  test("parses TrueType, collections and WOFF2", () => {
    const expected = { family: "Synthetic", unitsPerEm: 2048, ascent: 1854, descent: -434, lineGap: 67, xAvgCharWidth: 904, capHeight: 1467 };
    expect(parseSfntMetrics(sfntBytes, "Synthetic")).toEqual(expected);
    const ttcHeader = new Uint8Array(16);
    ttcHeader.set(encoder.encode("ttcf"), 0);
    const ttcView = new DataView(ttcHeader.buffer);
    ttcView.setUint16(4, 1);
    ttcView.setUint32(8, 1);
    ttcView.setUint32(12, 16);
    expect(parseSfntMetrics(buildSfnt(arialLikeTables, ttcHeader), "Synthetic")).toEqual(expected);
    expect(parseSfntMetrics(buildWoff2(arialLikeTables), "Synthetic")).toEqual(expected);
    expect(parseSfntMetrics(buildWoff2(arialLikeTables, true), "Synthetic")).toEqual(expected);
    expect(parseSfntMetrics(encoder.encode("woff2-placeholder"), "Fake")).toBeUndefined();
    expect(parseSfntMetrics(buildSfnt([["head", headTable(1000)]]), "NoHhea")).toBeUndefined();
    const legacy = parseSfntMetrics(buildSfnt([["head", headTable(1000)], ["hhea", hheaTable(800, -200, 0)], ["OS/2", os2Table(1, 500, 0)]]), "Old");
    expect(legacy).toEqual({ family: "Old", unitsPerEm: 1000, ascent: 800, descent: -200, lineGap: 0, xAvgCharWidth: 500 });
  });

  const arialPath = "C:/Windows/Fonts/arial.ttf";
  test.if(existsSync(arialPath))("matches the bundled Arial metrics on Windows", async () => {
    const metrics = parseSfntMetrics(new Uint8Array(await Bun.file(arialPath).arrayBuffer()), "Arial");
    expect(metrics?.unitsPerEm).toBe(ARIAL_METRICS.unitsPerEm);
    expect(metrics?.ascent).toBe(ARIAL_METRICS.ascent);
    expect(metrics?.descent).toBe(ARIAL_METRICS.descent);
    expect(metrics?.xAvgCharWidth).toBe(ARIAL_METRICS.xAvgCharWidth);
  });
});

describe("fallback metrics", () => {
  test("computeFallbackOverrides math", () => {
    expect(computeFallbackOverrides(ARIAL_METRICS, ARIAL_METRICS)).toEqual({
      fallbackFamily: "Arial",
      ascentOverride: "90.53%",
      descentOverride: "21.19%",
      lineGapOverride: "3.27%",
      sizeAdjust: "100.00%",
    });
    const main = { family: "Main", unitsPerEm: 1000, ascent: 900, descent: -200, lineGap: 0, xAvgCharWidth: 500 };
    expect(computeFallbackOverrides(main, ARIAL_METRICS)).toEqual({
      fallbackFamily: "Arial",
      ascentOverride: "79.45%",
      descentOverride: "17.66%",
      lineGapOverride: "0.00%",
      sizeAdjust: "113.27%",
    });
  });

  test("pickFallbackFont heuristics", () => {
    expect(pickFallbackFont("Inter")).toBe(ARIAL_METRICS);
    expect(pickFallbackFont("Playfair Display")).toBe(TIMES_NEW_ROMAN_METRICS);
    expect(pickFallbackFont("Noto Serif")).toBe(TIMES_NEW_ROMAN_METRICS);
    expect(pickFallbackFont("Noto Sans")).toBe(ARIAL_METRICS);
    expect(pickFallbackFont("EB Garamond")).toBe(TIMES_NEW_ROMAN_METRICS);
    expect(pickFallbackFont("Inter", "Times New Roman")).toBe(TIMES_NEW_ROMAN_METRICS);
    expect(pickFallbackFont("Lora", "Arial")).toBe(ARIAL_METRICS);
    expect(pickFallbackFont("Inter", false)).toBeUndefined();
  });

  test("presetFontStack table", () => {
    expect(presetFontStack("yolo-neutral").sans).toContain("Inter");
    expect(presetFontStack("yolo-chromium")).toEqual({ sans: "'Google Sans Flex', system-ui", mono: "'Google Sans Code', monospace" });
    expect(presetFontStack("yolo-apple").mono).toContain("SF Mono");
  });
});

describe("prepareFonts and the fonts handler", () => {
  let root = "";
  const googleCss =
    "/* latin-ext */\n@font-face {\n  font-family: 'Inter';\n  font-style: normal;\n  font-weight: 400;\n  font-display: swap;\n" +
    "  src: url(https://fonts.gstatic.com/s/inter/v18/latin-ext.woff2) format('woff2');\n  unicode-range: U+0100-02BA, U+02BD-02C5;\n}\n" +
    "/* latin */\n@font-face {\n  font-family: 'Inter';\n  font-style: normal;\n  font-weight: 400;\n  font-display: swap;\n" +
    "  src: url(https://fonts.gstatic.com/s/inter/v18/latin.woff2) format('woff2');\n  unicode-range: U+0000-00FF, U+0131;\n}\n";
  const legacyCss =
    "@font-face {\n  font-family: 'Inter';\n  font-style: normal;\n  font-weight: 400;\n" +
    "  src: url(https://fonts.gstatic.com/s/inter/v18/inter.ttf) format('truetype');\n}\n";
  const fetched: string[] = [];
  const fetchImpl: typeof fetch = (async (input: string | URL | Request, init?: RequestInit) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
    const userAgent = new Headers(init?.headers).get("user-agent") ?? "";
    fetched.push(url);
    if (url.startsWith("https://fonts.googleapis.com/css2")) {
      return new Response(userAgent.includes("Chrome") ? googleCss : legacyCss, { headers: { "content-type": "text/css" } });
    }
    if (url.endsWith(".woff2")) {
      return new Response(encoder.encode(`woff2:${url}`) as BodyInit);
    }
    if (url.endsWith(".ttf")) {
      return new Response(sfntBytes as BodyInit);
    }
    return new Response("not found", { status: 404 });
  }) as typeof fetch;

  beforeAll(async () => {
    root = mkdtempSync(join(tmpdir(), "yolo-next-fonts-"));
    mkdirSync(join(root, "public", "fonts"), { recursive: true });
    await Bun.write(join(root, "public", "fonts", "CalSans-SemiBold.ttf"), sfntBytes);
  });

  afterAll(() => {
    rmSync(root, { recursive: true, force: true });
  });

  test("parses Google css2 blocks", () => {
    const faces = parseFontFaces(googleCss);
    expect(faces).toHaveLength(2);
    expect(faces[0]?.subset).toBe("latin-ext");
    expect(faces[1]?.sources[0]).toEqual({ url: "https://fonts.gstatic.com/s/inter/v18/latin.woff2", format: "woff2" });
    expect(faces[1]?.unicodeRange).toBe("U+0000-00FF, U+0131");
  });

  test("downloads, rewrites and emits fallback faces", async () => {
    resetFontRegistry();
    const inter = googleFont("Inter", { subsets: ["latin"], display: "swap", variable: "--font-inter" });
    const calSans = localFont({
      src: [{ path: "./public/fonts/CalSans-SemiBold.ttf", weight: "600", style: "normal" }],
      variable: "--font-display",
      display: "swap",
      adjustFontFallback: "Arial",
      cwd: root,
    });
    const prepared = await prepareFonts({ root, fetchImpl, dev: false });
    const files = readdirSync(join(root, ".yolo", "fonts"));
    expect(files.filter((file) => /^inter-[0-9a-f]{12}\.woff2$/.test(file))).toHaveLength(2);
    expect(files.filter((file) => /^calsans-semibold-[0-9a-f]{12}\.ttf$/.test(file))).toHaveLength(1);
    expect(files).toContain("fonts.css");
    expect(prepared.css).toContain("/_yolo/fonts/inter-");
    expect(prepared.css).toContain("/* latin-ext */");
    expect(prepared.css).toContain("unicode-range: U+0000-00FF, U+0131;");
    expect(prepared.css).toContain("font-family: 'Inter Fallback';");
    expect(prepared.css).toContain("src: local('Arial');");
    expect(prepared.css).toContain("ascent-override: 90.53%;");
    expect(prepared.css).toContain(`.${inter.className} {\n  font-family: 'Inter', 'Inter Fallback';\n}`);
    expect(prepared.css).toContain(`.${inter.variable} {\n  --font-inter: 'Inter', 'Inter Fallback';\n}`);
    expect(prepared.css).toContain("font-weight: 600;");
    expect(prepared.css).toContain(`.${calSans.className} {`);
    expect(prepared.css).toContain("font-family: 'CalSans-SemiBold Fallback';");
    expect(prepared.assets).toHaveLength(3);
    expect(prepared.assets.find((asset) => asset.subset === "latin-ext")?.preload).toBe(false);
    expect(prepared.assets.find((asset) => asset.subset === "latin")?.preload).toBe(true);
    expect(prepared.assets.find((asset) => asset.family === "CalSans-SemiBold")?.preload).toBe(true);
    expect(prepared.diagnostics).toEqual([]);
    expect(fetched.filter((url) => url.endsWith(".ttf"))).toHaveLength(1);
    const links = fontPreloadLinks(prepared.assets);
    expect(links.split("\n")).toHaveLength(2);
    expect(links).toContain('<link rel="preload" as="font" type="font/woff2" crossorigin href="/_yolo/fonts/inter-');
    expect(links).toContain('type="font/ttf"');
    const again = await prepareFonts({ root, fetchImpl, dev: false });
    expect(again.css).toBe(prepared.css);
  });

  test("offline mode emits a plain local fallback", async () => {
    resetFontRegistry();
    googleFont("Inter", { subsets: ["latin"] });
    const offlineRoot = mkdtempSync(join(tmpdir(), "yolo-next-fonts-offline-"));
    try {
      const prepared = await prepareFonts({ root: offlineRoot, fetchImpl, offline: true });
      expect(prepared.assets).toHaveLength(0);
      expect(prepared.css).toContain("font-family: 'Inter Fallback';");
      expect(prepared.diagnostics[0]).toContain("unavailable");
    } finally {
      rmSync(offlineRoot, { recursive: true, force: true });
    }
  });

  test("createFontsHandler serves cached files immutably", async () => {
    const handler = createFontsHandler({ root });
    const woff2 = readdirSync(join(root, ".yolo", "fonts")).find((file) => file.endsWith(".woff2")) ?? "";
    const response = handler(new Request(`http://localhost/_yolo/fonts/${woff2}`));
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("font/woff2");
    expect(response.headers.get("cache-control")).toBe("public, max-age=31536000, immutable");
    expect(response.headers.get("access-control-allow-origin")).toBe("*");
    expect(await response.text()).toStartWith("woff2:https://fonts.gstatic.com/");
    expect(handler(new Request("http://localhost/_yolo/fonts/fonts.css")).headers.get("content-type")).toBe("text/css; charset=utf-8");
    expect(handler(new Request("http://localhost/_yolo/fonts/missing.woff2")).status).toBe(404);
    expect(handler(new Request("http://localhost/_yolo/fonts/..%2Ffonts.css")).status).toBe(404);
    expect(handler(new Request("http://localhost/other")).status).toBe(404);
  });
});
