// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/fonts: next/font runtime (Google css2 planning, self-hosting, fallback metrics, SFNT/WOFF/WOFF2 parsing).

import { existsSync, statSync } from "node:fs";
import { mkdir } from "node:fs/promises";
import { basename, dirname, extname, resolve } from "node:path";
import { brotliDecompressSync, inflateSync } from "node:zlib";
import {
  ARIAL_METRICS,
  TIMES_NEW_ROMAN_METRICS,
  type FallbackOverrides,
  type FontDisplay,
  type FontFaceAsset,
  type FontMetrics,
  type GoogleFontOptions,
  type LocalFontOptions,
  type LocalFontSrc,
  type NextFont,
  type NextFontWithVariable,
  type YoloPresetId,
} from "./types.ts";

/** Google Fonts user agent (Chrome 104) for which css2 returns woff2 files split by unicode-range. */
export const GOOGLE_FONTS_USER_AGENT =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36";
/** Legacy user agent for which css2 returns TrueType files (used to read fallback metrics). */
export const GOOGLE_FONTS_METRICS_USER_AGENT = "Mozilla/5.0 (compatible; YoloFontMetrics/1.0)";
/** Default route serving self-hosted font files. */
export const DEFAULT_FONTS_ROUTE = "/_yolo/fonts";
/** Default cache directory (relative to the project root) of self-hosted font files. */
export const DEFAULT_FONTS_CACHE_DIR = ".yolo/fonts";

const FONT_MIME: Readonly<Record<string, string>> = {
  woff2: "font/woff2",
  woff: "font/woff",
  ttf: "font/ttf",
  otf: "font/otf",
  css: "text/css; charset=utf-8",
};
const FORMAT_BY_EXTENSION: Readonly<Record<string, string>> = {
  woff2: "woff2",
  woff: "woff",
  ttf: "truetype",
  otf: "opentype",
};
const SERIF_HINTS: ReadonlyArray<string> = [
  "serif",
  "garamond",
  "playfair",
  "merriweather",
  "lora",
  "crimson",
  "baskerville",
  "times",
  "georgia",
  "cormorant",
  "spectral",
  "literata",
  "fraunces",
  "newsreader",
];

function sha256(input: string | Uint8Array): string {
  return new Bun.CryptoHasher("sha256").update(input).digest("hex");
}

function normalizeList(value: string | ReadonlyArray<string> | undefined): string[] {
  if (value === undefined) {
    return [];
  }
  return (typeof value === "string" ? [value] : [...value]).map((item) => item.trim()).filter((item) => item.length > 0);
}

/** URL/file-safe slug of a family name (`Roboto Mono` -> `roboto-mono`). */
export function fontSlug(family: string): string {
  return family
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function quoteFamily(family: string): string {
  return `'${family.replace(/'/g, "\\'")}'`;
}

// ---------------------------------------------------------------------------
// Font objects and registry
// ---------------------------------------------------------------------------

/** Definition accepted by `yoloFont` (emitted in the generated `src/fonts.ts` of a migrated app). */
export interface YoloFontDefinition {
  readonly className: string;
  readonly variable?: string;
  readonly style: { readonly fontFamily: string; readonly fontWeight?: number; readonly fontStyle?: string };
}

/** Builds a `next/font` compatible object from a precomputed definition. */
export function yoloFont(def: YoloFontDefinition & { readonly variable: string }): NextFontWithVariable;
export function yoloFont(def: YoloFontDefinition): NextFont;
export function yoloFont(def: YoloFontDefinition): NextFont | NextFontWithVariable {
  const style: NextFont["style"] = { fontFamily: def.style.fontFamily };
  if (def.style.fontWeight !== undefined) {
    style.fontWeight = def.style.fontWeight;
  }
  if (def.style.fontStyle !== undefined) {
    style.fontStyle = def.style.fontStyle;
  }
  return def.variable === undefined
    ? { className: def.className, style }
    : { className: def.className, variable: def.variable, style };
}

/** Resolved local font source. */
export interface ResolvedLocalSrc {
  readonly path: string;
  readonly weight?: string;
  readonly style?: string;
}

/** Lightweight plan of one font request (what `prepareFonts` needs to self-host it). */
export interface FontPlanLite {
  readonly kind: "google" | "local";
  readonly hash: string;
  readonly family: string;
  readonly className: string;
  readonly variable?: string;
  readonly variableClass?: string;
  readonly css2Url?: string;
  readonly familyCss: string;
  readonly display: FontDisplay;
  readonly preload: boolean;
  readonly subsets: ReadonlyArray<string>;
  readonly weights: ReadonlyArray<string>;
  readonly styles: ReadonlyArray<string>;
  readonly fallback: ReadonlyArray<string>;
  readonly adjustFontFallback: "Arial" | "Times New Roman" | false;
  readonly fontWeight?: number;
  readonly fontStyle?: string;
  readonly sources: ReadonlyArray<ResolvedLocalSrc>;
  readonly declarations: ReadonlyArray<{ readonly prop: string; readonly value: string }>;
}

/** `next/font` object extended with its plan. */
export type PlannedFont = NextFontWithVariable & { readonly plan: FontPlanLite };

const registry = new Map<string, FontPlanLite>();

/** Font requests registered by `googleFont` / `localFont` since startup (or the last reset). */
export function listFontRequests(): FontPlanLite[] {
  return [...registry.values()];
}

/** Clears the font registry (tests and dev reloads). */
export function resetFontRegistry(): void {
  registry.clear();
}

function singleNumericWeight(weights: ReadonlyArray<string>): number | undefined {
  if (weights.length !== 1) {
    return undefined;
  }
  const weight = weights[0] ?? "";
  return /^\d+$/.test(weight) ? Number(weight) : undefined;
}

function familyCssOf(family: string, adjust: boolean, fallback: ReadonlyArray<string>): string {
  const parts = [quoteFamily(family)];
  if (adjust) {
    parts.push(quoteFamily(`${family} Fallback`));
  }
  parts.push(...fallback);
  return parts.join(", ");
}

function toFontObject(plan: FontPlanLite): PlannedFont {
  const style: NextFont["style"] = { fontFamily: plan.familyCss };
  if (plan.fontWeight !== undefined) {
    style.fontWeight = plan.fontWeight;
  }
  if (plan.fontStyle !== undefined) {
    style.fontStyle = plan.fontStyle;
  }
  return { className: plan.className, variable: plan.variableClass ?? "", style, plan };
}

function weightAxisValues(weights: ReadonlyArray<string>): string[] {
  return weights
    .filter((weight) => weight !== "variable")
    .map((weight) => weight.replace(/\s+/g, "..").replace(/\.{3,}/g, ".."))
    .sort((a, b) => Number.parseInt(a, 10) - Number.parseInt(b, 10));
}

/** Google Fonts css2 URL for `family` (`family=Roboto+Mono:wght@400;700&display=swap`). */
export function css2Url(family: string, options: GoogleFontOptions = {}): string {
  const weights = weightAxisValues(normalizeList(options.weight));
  const styles = normalizeList(options.style);
  const hasItalic = styles.includes("italic");
  const hasNormal = styles.includes("normal") || !hasItalic;
  let url = `https://fonts.googleapis.com/css2?family=${family.trim().replace(/\s+/g, "+")}`;
  if (weights.length > 0) {
    if (hasItalic) {
      const tuples: string[] = [];
      for (const ital of hasNormal ? ["0", "1"] : ["1"]) {
        for (const weight of weights) {
          tuples.push(`${ital},${weight}`);
        }
      }
      url += `:ital,wght@${tuples.join(";")}`;
    } else {
      url += `:wght@${weights.join(";")}`;
    }
  } else if (hasItalic) {
    url += hasNormal ? ":ital@0;1" : ":ital@1";
  }
  url += `&display=${options.display ?? "swap"}`;
  return url;
}

/** Declares a Google font (synchronous): returns its classes and registers the request for `prepareFonts()`. */
export function googleFont(family: string, options: GoogleFontOptions = {}): PlannedFont {
  const hash = sha256(`${family}${JSON.stringify(options)}`).slice(0, 8);
  const weights = normalizeList(options.weight);
  const styles = normalizeList(options.style);
  const adjust = options.adjustFontFallback !== false;
  const fallback = options.fallback ?? [];
  const plan: FontPlanLite = {
    kind: "google",
    hash,
    family,
    className: `yolo-font-${hash}`,
    variable: options.variable,
    variableClass: options.variable ? `yolo-font-var-${hash}` : undefined,
    css2Url: css2Url(family, options),
    familyCss: familyCssOf(family, adjust, fallback),
    display: options.display ?? "swap",
    preload: options.preload !== false,
    subsets: options.subsets ?? [],
    weights,
    styles,
    fallback,
    adjustFontFallback: adjust ? (pickFallbackFont(family)?.family === "Times New Roman" ? "Times New Roman" : "Arial") : false,
    fontWeight: singleNumericWeight(weights),
    fontStyle: styles.length === 1 ? styles[0] : undefined,
    sources: [],
    declarations: [],
  };
  registry.set(hash, plan);
  return toFontObject(plan);
}

/** `localFont` options plus the hidden importer path injected by the Bun plugin. */
export type LocalFontRuntimeOptions = LocalFontOptions & { readonly cwd?: string; readonly __importer?: string };

/** Declares a self-hosted font (synchronous): returns its classes and registers its files for `prepareFonts()`. */
export function localFont(options: LocalFontRuntimeOptions): PlannedFont {
  const { __importer, cwd, ...publicOptions } = options;
  const baseDir = __importer ? dirname(__importer) : (cwd ?? process.cwd());
  const srcList: ReadonlyArray<LocalFontSrc> =
    typeof options.src === "string"
      ? [{ path: options.src, weight: options.weight, style: options.style }]
      : options.src.map((src) => ({ path: src.path, weight: src.weight ?? options.weight, style: src.style ?? options.style }));
  const sources: ResolvedLocalSrc[] = srcList.map((src) => ({
    path: resolve(baseDir, src.path),
    weight: src.weight,
    style: src.style,
  }));
  const hash = sha256(`local:${JSON.stringify({ ...publicOptions, src: sources })}`).slice(0, 8);
  const first = sources[0];
  const family = first ? basename(first.path, extname(first.path)) : `local-${hash}`;
  const adjustOption = options.adjustFontFallback ?? "Arial";
  const fallback = options.fallback ?? [];
  const sourceWeights = [...new Set(sources.map((src) => src.weight).filter((w): w is string => w !== undefined))];
  const sourceStyles = [...new Set(sources.map((src) => src.style).filter((s): s is string => s !== undefined))];
  const plan: FontPlanLite = {
    kind: "local",
    hash,
    family,
    className: `yolo-font-${hash}`,
    variable: options.variable,
    variableClass: options.variable ? `yolo-font-var-${hash}` : undefined,
    familyCss: familyCssOf(family, adjustOption !== false, fallback),
    display: options.display ?? "swap",
    preload: options.preload !== false,
    subsets: [],
    weights: sourceWeights,
    styles: sourceStyles,
    fallback,
    adjustFontFallback: adjustOption,
    fontWeight: singleNumericWeight(options.weight ? [options.weight] : sourceWeights),
    fontStyle: options.style ?? (sourceStyles.length === 1 ? sourceStyles[0] : undefined),
    sources,
    declarations: options.declarations ?? [],
  };
  registry.set(hash, plan);
  return toFontObject(plan);
}

// ---------------------------------------------------------------------------
// SFNT / WOFF / WOFF2 parsing
// ---------------------------------------------------------------------------

const WOFF2_KNOWN_TAGS: ReadonlyArray<string> = [
  "cmap", "head", "hhea", "hmtx", "maxp", "name", "OS/2", "post", "cvt ", "fpgm", "glyf", "loca", "prep", "CFF ",
  "VORG", "EBDT", "EBLC", "gasp", "hdmx", "kern", "LTSH", "PCLT", "VDMX", "vhea", "vmtx", "BASE", "GDEF", "GPOS",
  "GSUB", "EBSC", "JSTF", "MATH", "CBDT", "CBLC", "COLR", "CPAL", "SVG ", "sbix", "acnt", "avar", "bdat", "bloc",
  "bsln", "cvar", "fdsc", "feat", "fmtx", "fvar", "gvar", "hsty", "just", "lcar", "mort", "morx", "opbd", "prop",
  "trak", "Zapf", "Silf", "Glat", "Gloc", "Feat", "Sill",
];

type TableMap = Map<string, Uint8Array>;

function view(bytes: Uint8Array): DataView {
  return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
}

function tag(bytes: Uint8Array, offset: number): string {
  return String.fromCharCode(bytes[offset] ?? 0, bytes[offset + 1] ?? 0, bytes[offset + 2] ?? 0, bytes[offset + 3] ?? 0);
}

function readBase128(bytes: Uint8Array, offset: number): { value: number; next: number } {
  let value = 0;
  for (let index = 0; index < 5; index += 1) {
    const byte = bytes[offset + index];
    if (byte === undefined || (index === 0 && byte === 0x80) || value >= 0x2000000) {
      throw new Error("invalid UIntBase128");
    }
    value = value * 128 + (byte & 0x7f);
    if ((byte & 0x80) === 0) {
      return { value, next: offset + index + 1 };
    }
  }
  throw new Error("invalid UIntBase128");
}

function sfntTables(bytes: Uint8Array, directoryOffset = 0): TableMap | undefined {
  if (bytes.byteLength < directoryOffset + 12) {
    return undefined;
  }
  const dv = view(bytes);
  const version = dv.getUint32(directoryOffset);
  const signature = tag(bytes, directoryOffset);
  if (signature === "ttcf" && directoryOffset === 0) {
    // Collections: table record offsets are absolute, so keep the whole buffer and move the directory.
    const first = dv.getUint32(12);
    return first < bytes.byteLength ? sfntTables(bytes, first) : undefined;
  }
  if (version !== 0x00010000 && signature !== "OTTO" && signature !== "true") {
    return undefined;
  }
  const numTables = dv.getUint16(directoryOffset + 4);
  const tables: TableMap = new Map();
  for (let index = 0; index < numTables; index += 1) {
    const record = directoryOffset + 12 + index * 16;
    if (record + 16 > bytes.byteLength) {
      return undefined;
    }
    const offset = dv.getUint32(record + 8);
    const length = dv.getUint32(record + 12);
    if (offset + length <= bytes.byteLength) {
      tables.set(tag(bytes, record), bytes.subarray(offset, offset + length));
    }
  }
  return tables;
}

function woffTables(bytes: Uint8Array): TableMap | undefined {
  const dv = view(bytes);
  const numTables = dv.getUint16(12);
  const tables: TableMap = new Map();
  for (let index = 0; index < numTables; index += 1) {
    const record = 44 + index * 20;
    if (record + 20 > bytes.byteLength) {
      return undefined;
    }
    const offset = dv.getUint32(record + 4);
    const compLength = dv.getUint32(record + 8);
    const origLength = dv.getUint32(record + 12);
    const raw = bytes.subarray(offset, offset + compLength);
    tables.set(tag(bytes, record), compLength < origLength ? new Uint8Array(inflateSync(raw)) : raw);
  }
  return tables;
}

/** Decodes a WOFF2 container into its (untransformed) SFNT tables; `undefined` when malformed. */
export function woff2Tables(bytes: Uint8Array): TableMap | undefined {
  try {
    if (bytes.byteLength < 48 || tag(bytes, 0) !== "wOF2") {
      return undefined;
    }
    const dv = view(bytes);
    const flavor = tag(bytes, 4);
    const numTables = dv.getUint16(12);
    const totalCompressedSize = dv.getUint32(20);
    const directory: Array<{ tag: string; size: number }> = [];
    let cursor = 48;
    for (let index = 0; index < numTables; index += 1) {
      const flags = bytes[cursor] ?? 0;
      cursor += 1;
      const tagIndex = flags & 0x3f;
      let name: string;
      if (tagIndex === 63) {
        name = tag(bytes, cursor);
        cursor += 4;
      } else {
        name = WOFF2_KNOWN_TAGS[tagIndex] ?? "";
      }
      const transformVersion = (flags >> 6) & 0x03;
      const orig = readBase128(bytes, cursor);
      cursor = orig.next;
      let size = orig.value;
      const glyfOrLoca = name === "glyf" || name === "loca";
      const transformed = glyfOrLoca ? transformVersion === 0 : transformVersion !== 0;
      if (transformed) {
        const transform = readBase128(bytes, cursor);
        cursor = transform.next;
        size = transform.value;
      }
      directory.push({ tag: name, size });
    }
    if (flavor === "ttcf") {
      cursor += 4;
      const fonts = readBase128(bytes, cursor);
      cursor = fonts.next;
      for (let font = 0; font < fonts.value; font += 1) {
        const count = readBase128(bytes, cursor);
        cursor = count.next + 4;
        for (let index = 0; index < count.value; index += 1) {
          cursor = readBase128(bytes, cursor).next;
        }
      }
    }
    const compressed = bytes.subarray(cursor, cursor + totalCompressedSize);
    const data = new Uint8Array(brotliDecompressSync(compressed));
    const tables: TableMap = new Map();
    let offset = 0;
    for (const entry of directory) {
      tables.set(entry.tag, data.subarray(offset, offset + entry.size));
      offset += entry.size;
    }
    return tables;
  } catch {
    return undefined;
  }
}

/** Splits font bytes (TrueType, OpenType, collection, WOFF or WOFF2) into SFNT tables. */
export function fontTables(bytes: Uint8Array): TableMap | undefined {
  if (bytes.byteLength < 12) {
    return undefined;
  }
  const signature = tag(bytes, 0);
  try {
    if (signature === "wOF2") {
      return woff2Tables(bytes);
    }
    if (signature === "wOFF") {
      return woffTables(bytes);
    }
    return sfntTables(bytes);
  } catch {
    return undefined;
  }
}

/** Reads unitsPerEm, ascent, descent, lineGap, capHeight and xAvgCharWidth from font bytes (first face of a collection). */
export function parseSfntMetrics(bytes: Uint8Array, family: string): FontMetrics | undefined {
  const tables = fontTables(bytes);
  const head = tables?.get("head");
  const hhea = tables?.get("hhea");
  const os2 = tables?.get("OS/2");
  if (!tables || !head || !hhea || !os2 || head.byteLength < 20 || hhea.byteLength < 10 || os2.byteLength < 4) {
    return undefined;
  }
  const headView = view(head);
  const hheaView = view(hhea);
  const os2View = view(os2);
  const version = os2View.getUint16(0);
  const metrics: FontMetrics = {
    family,
    unitsPerEm: headView.getUint16(18),
    ascent: hheaView.getInt16(4),
    descent: hheaView.getInt16(6),
    lineGap: hheaView.getInt16(8),
    xAvgCharWidth: os2View.getInt16(2),
  };
  if (version >= 2 && os2.byteLength >= 90) {
    return { ...metrics, capHeight: os2View.getInt16(88) };
  }
  return metrics;
}

// ---------------------------------------------------------------------------
// Fallback metrics
// ---------------------------------------------------------------------------

/** Size-adjust and ascent/descent/line-gap overrides making `fallback` mimic `main` (next/font formulas). */
export function computeFallbackOverrides(main: FontMetrics, fallback: FontMetrics): FallbackOverrides {
  const useAz = main.azAvgWidth !== undefined && fallback.azAvgWidth !== undefined;
  const mainAvg = useAz ? (main.azAvgWidth ?? main.xAvgCharWidth) : main.xAvgCharWidth;
  const fallbackAvg = useAz ? (fallback.azAvgWidth ?? fallback.xAvgCharWidth) : fallback.xAvgCharWidth;
  const mainRatio = mainAvg / main.unitsPerEm;
  const fallbackRatio = fallbackAvg / fallback.unitsPerEm;
  const sizeAdjust = mainRatio > 0 && fallbackRatio > 0 ? mainRatio / fallbackRatio : 1;
  const percent = (value: number): string => `${Math.abs(value * 100).toFixed(2)}%`;
  return {
    fallbackFamily: fallback.family,
    ascentOverride: percent(main.ascent / (main.unitsPerEm * sizeAdjust)),
    descentOverride: percent(main.descent / (main.unitsPerEm * sizeAdjust)),
    lineGapOverride: percent(main.lineGap / (main.unitsPerEm * sizeAdjust)),
    sizeAdjust: percent(sizeAdjust),
  };
}

/** Fallback metrics for `family`: explicit `Arial` / `Times New Roman`, `false` for none, else a serif-name heuristic. */
export function pickFallbackFont(
  family: string,
  option?: "Arial" | "Times New Roman" | boolean,
): FontMetrics | undefined {
  if (option === false) {
    return undefined;
  }
  if (option === "Arial") {
    return ARIAL_METRICS;
  }
  if (option === "Times New Roman") {
    return TIMES_NEW_ROMAN_METRICS;
  }
  const name = family.toLowerCase();
  const sansSerif = /sans[\s-]?serif/.test(name);
  const serif = SERIF_HINTS.some((hint) => (hint === "serif" ? !sansSerif && name.includes("serif") : name.includes(hint)));
  return serif ? TIMES_NEW_ROMAN_METRICS : ARIAL_METRICS;
}

/** `@font-face` rule of a size-adjusted local fallback (`'Inter Fallback'`). */
export function fallbackFontFaceCss(family: string, overrides: FallbackOverrides | undefined, fallbackFamily = "Arial"): string {
  if (!overrides) {
    return `@font-face {\n  font-family: ${quoteFamily(`${family} Fallback`)};\n  src: local('${fallbackFamily}');\n}\n`;
  }
  return (
    `@font-face {\n  font-family: ${quoteFamily(`${family} Fallback`)};\n  src: local('${overrides.fallbackFamily}');\n` +
    `  ascent-override: ${overrides.ascentOverride};\n  descent-override: ${overrides.descentOverride};\n` +
    `  line-gap-override: ${overrides.lineGapOverride};\n  size-adjust: ${overrides.sizeAdjust};\n}\n`
  );
}

// ---------------------------------------------------------------------------
// Google css2 parsing and self-hosting
// ---------------------------------------------------------------------------

/** One `@font-face` block parsed from a stylesheet. */
export interface ParsedFontFace {
  readonly subset?: string;
  readonly family: string;
  readonly style: string;
  readonly weight: string;
  readonly display?: string;
  readonly sources: ReadonlyArray<{ readonly url: string; readonly format?: string }>;
  readonly unicodeRange?: string;
}

/** Parses `@font-face` blocks (and their leading `/* subset *\/` comments) from a stylesheet. */
export function parseFontFaces(css: string): ParsedFontFace[] {
  const faces: ParsedFontFace[] = [];
  const blockRe = /(?:\/\*\s*([^*]+?)\s*\*\/\s*)?@font-face\s*\{([^}]*)\}/g;
  for (let match = blockRe.exec(css); match; match = blockRe.exec(css)) {
    const declarations = new Map<string, string>();
    for (const declaration of (match[2] ?? "").split(";")) {
      const colon = declaration.indexOf(":");
      if (colon === -1) {
        continue;
      }
      declarations.set(declaration.slice(0, colon).trim().toLowerCase(), declaration.slice(colon + 1).trim());
    }
    const sources: Array<{ url: string; format?: string }> = [];
    const srcRe = /url\((['"]?)([^'")]+)\1\)(?:\s*format\((['"]?)([^'")]+)\3\))?/g;
    const src = declarations.get("src") ?? "";
    for (let source = srcRe.exec(src); source; source = srcRe.exec(src)) {
      const url = source[2];
      if (url) {
        sources.push(source[4] ? { url, format: source[4] } : { url });
      }
    }
    const face: ParsedFontFace = {
      family: (declarations.get("font-family") ?? "").replace(/^['"]|['"]$/g, ""),
      style: declarations.get("font-style") ?? "normal",
      weight: declarations.get("font-weight") ?? "400",
      sources,
      ...(match[1] ? { subset: match[1] } : {}),
      ...(declarations.has("font-display") ? { display: declarations.get("font-display") } : {}),
      ...(declarations.has("unicode-range") ? { unicodeRange: declarations.get("unicode-range") } : {}),
    };
    faces.push(face);
  }
  return faces;
}

/** Options of `prepareFonts`. */
export interface PrepareFontsOptions {
  readonly root: string;
  readonly fontsRoute?: string;
  readonly cacheDir?: string;
  readonly fetchImpl?: typeof fetch;
  readonly offline?: boolean;
  readonly dev?: boolean;
  readonly requests?: ReadonlyArray<FontPlanLite>;
  readonly retries?: number;
  readonly timeoutMs?: number;
}

/** Result of `prepareFonts`. */
export interface PreparedFonts {
  readonly css: string;
  readonly assets: ReadonlyArray<FontFaceAsset>;
  readonly plans: ReadonlyArray<FontPlanLite>;
  readonly diagnostics: ReadonlyArray<string>;
}

async function fetchWithRetry(
  fetchImpl: typeof fetch,
  url: string,
  userAgent: string,
  retries: number,
  timeoutMs: number,
): Promise<Response | undefined> {
  for (let attempt = 0; attempt < retries; attempt += 1) {
    try {
      const response = await fetchImpl(url, {
        headers: { "user-agent": userAgent, accept: "*/*" },
        signal: AbortSignal.timeout(timeoutMs),
      });
      if (response.ok) {
        return response;
      }
    } catch {
      // Retry below.
    }
  }
  return undefined;
}

function extensionOf(url: string, format?: string): string {
  const pathname = new URL(url, "http://yolo.local").pathname;
  const ext = extname(pathname).slice(1).toLowerCase();
  if (ext in FORMAT_BY_EXTENSION) {
    return ext;
  }
  switch (format) {
    case "woff2":
      return "woff2";
    case "woff":
      return "woff";
    case "opentype":
      return "otf";
    default:
      return "ttf";
  }
}

function classRulesCss(plan: FontPlanLite): string {
  let css = `.${plan.className} {\n  font-family: ${plan.familyCss};\n`;
  if (plan.fontWeight !== undefined) {
    css += `  font-weight: ${plan.fontWeight};\n`;
  }
  if (plan.fontStyle !== undefined) {
    css += `  font-style: ${plan.fontStyle};\n`;
  }
  css += "}\n";
  if (plan.variableClass && plan.variable) {
    css += `.${plan.variableClass} {\n  ${plan.variable}: ${plan.familyCss};\n}\n`;
  }
  return css;
}

/** Downloads (Google) or copies (local) every registered font, writes `fonts.css` and returns the plan. */
export async function prepareFonts(options: PrepareFontsOptions): Promise<PreparedFonts> {
  const fontsRoute = options.fontsRoute ?? DEFAULT_FONTS_ROUTE;
  const cacheRoot = resolve(options.root, options.cacheDir ?? DEFAULT_FONTS_CACHE_DIR);
  const fetchImpl = options.fetchImpl ?? fetch;
  const offline = options.offline === true;
  const dev = options.dev ?? Bun.env.NODE_ENV !== "production";
  const retries = options.retries ?? 3;
  const timeoutMs = options.timeoutMs ?? 3000;
  const plans = options.requests ?? listFontRequests();
  const assets: FontFaceAsset[] = [];
  const diagnostics: string[] = [];
  const downloaded = new Map<string, string>();
  let css = "/* generated by @yolo-rs/next prepareFonts */\n";
  await mkdir(cacheRoot, { recursive: true });

  const store = async (fileName: string, bytes: Uint8Array): Promise<string> => {
    const target = resolve(cacheRoot, fileName);
    if (!existsSync(target)) {
      await Bun.write(target, bytes);
    }
    return `${fontsRoute}/${fileName}`;
  };

  const download = async (plan: FontPlanLite, url: string, format?: string): Promise<string | undefined> => {
    const known = downloaded.get(url);
    if (known) {
      return known;
    }
    const fileName = `${fontSlug(plan.family)}-${sha256(url).slice(0, 12)}.${extensionOf(url, format)}`;
    const target = resolve(cacheRoot, fileName);
    if (!existsSync(target)) {
      if (offline) {
        return undefined;
      }
      const response = await fetchWithRetry(fetchImpl, url, GOOGLE_FONTS_USER_AGENT, retries, timeoutMs);
      if (!response) {
        return undefined;
      }
      await Bun.write(target, new Uint8Array(await response.arrayBuffer()));
    }
    const localPath = `${fontsRoute}/${fileName}`;
    downloaded.set(url, localPath);
    return localPath;
  };

  for (const plan of plans) {
    css += `\n/* ${plan.kind}: ${plan.family} (${plan.className}) */\n`;
    let metrics: FontMetrics | undefined;
    if (plan.kind === "google" && plan.css2Url) {
      const response = offline ? undefined : await fetchWithRetry(fetchImpl, plan.css2Url, GOOGLE_FONTS_USER_AGENT, retries, timeoutMs);
      const faces = response ? parseFontFaces(await response.text()) : [];
      if (!response || faces.length === 0) {
        if (!offline && !dev) {
          throw new Error(`Failed to download Google font "${plan.family}" from ${plan.css2Url}`);
        }
        diagnostics.push(`google font "${plan.family}" unavailable; emitted local fallback only`);
      }
      for (const face of faces) {
        const rewritten: string[] = [];
        for (const source of face.sources) {
          const localPath = await download(plan, source.url, source.format);
          if (!localPath) {
            diagnostics.push(`could not download ${source.url}`);
            continue;
          }
          const ext = extensionOf(source.url, source.format);
          const format = source.format ?? FORMAT_BY_EXTENSION[ext] ?? ext;
          rewritten.push(`url(${localPath}) format('${format}')`);
          assets.push({
            source: source.url,
            localPath,
            family: plan.family,
            weight: face.weight,
            style: face.style,
            format,
            preload: plan.preload && (face.subset === undefined || plan.subsets.length === 0 || plan.subsets.includes(face.subset)),
            ...(face.unicodeRange ? { unicodeRange: face.unicodeRange } : {}),
            ...(face.subset ? { subset: face.subset } : {}),
          });
        }
        if (rewritten.length === 0) {
          continue;
        }
        if (face.subset) {
          css += `/* ${face.subset} */\n`;
        }
        css += `@font-face {\n  font-family: ${quoteFamily(plan.family)};\n  font-style: ${face.style};\n  font-weight: ${face.weight};\n  font-display: ${plan.display};\n  src: ${rewritten.join(", ")};\n`;
        if (face.unicodeRange) {
          css += `  unicode-range: ${face.unicodeRange};\n`;
        }
        css += "}\n";
      }
      if (plan.adjustFontFallback !== false && !offline) {
        const legacy = await fetchWithRetry(fetchImpl, plan.css2Url, GOOGLE_FONTS_METRICS_USER_AGENT, retries, timeoutMs);
        const legacyFaces = legacy ? parseFontFaces(await legacy.text()) : [];
        const ttf = legacyFaces.flatMap((face) => face.sources).find((source) => /\.ttf(\?|$)/i.test(source.url) || source.format === "truetype");
        if (ttf) {
          const file = await fetchWithRetry(fetchImpl, ttf.url, GOOGLE_FONTS_METRICS_USER_AGENT, retries, timeoutMs);
          if (file) {
            metrics = parseSfntMetrics(new Uint8Array(await file.arrayBuffer()), plan.family);
          }
        }
      }
    } else if (plan.kind === "local") {
      for (const source of plan.sources) {
        const file = Bun.file(source.path);
        if (!(await file.exists())) {
          diagnostics.push(`local font file not found: ${source.path}`);
          continue;
        }
        const bytes = new Uint8Array(await file.arrayBuffer());
        metrics ??= parseSfntMetrics(bytes, plan.family);
        const ext = extname(source.path).slice(1).toLowerCase();
        const format = FORMAT_BY_EXTENSION[ext] ?? ext;
        const localPath = await store(`${fontSlug(plan.family)}-${sha256(bytes).slice(0, 12)}.${ext}`, bytes);
        const weight = source.weight ?? "400";
        const style = source.style ?? "normal";
        assets.push({ source: source.path, localPath, family: plan.family, weight, style, format, preload: plan.preload });
        css += `@font-face {\n  font-family: ${quoteFamily(plan.family)};\n  font-style: ${style};\n  font-weight: ${weight};\n  font-display: ${plan.display};\n  src: url(${localPath}) format('${format}');\n`;
        for (const declaration of plan.declarations) {
          css += `  ${declaration.prop}: ${declaration.value};\n`;
        }
        css += "}\n";
      }
    }
    if (plan.adjustFontFallback !== false) {
      const fallback = pickFallbackFont(plan.family, plan.adjustFontFallback);
      const overrides = metrics && fallback ? computeFallbackOverrides(metrics, fallback) : undefined;
      css += fallbackFontFaceCss(plan.family, overrides, fallback?.family ?? "Arial");
    }
    css += classRulesCss(plan);
  }
  await Bun.write(resolve(cacheRoot, "fonts.css"), css);
  return { css, assets, plans, diagnostics };
}

/** Options of `createFontsHandler`. */
export interface FontsHandlerOptions {
  readonly root: string;
  readonly fontsRoute?: string;
  readonly cacheDir?: string;
}

/** Serves self-hosted font files and `fonts.css` from the cache directory (immutable caching, CORS `*`). */
export function createFontsHandler(options: FontsHandlerOptions): (request: Request) => Response {
  const fontsRoute = options.fontsRoute ?? DEFAULT_FONTS_ROUTE;
  const cacheRoot = resolve(options.root, options.cacheDir ?? DEFAULT_FONTS_CACHE_DIR);
  return (request: Request): Response => {
    const pathname = new URL(request.url).pathname;
    if (!pathname.startsWith(`${fontsRoute}/`)) {
      return new Response("Not Found", { status: 404 });
    }
    let name: string;
    try {
      name = decodeURIComponent(pathname.slice(fontsRoute.length + 1));
    } catch {
      return new Response("Bad Request", { status: 400 });
    }
    if (name.length === 0 || name.includes("/") || name.includes("\\") || name.startsWith(".")) {
      return new Response("Not Found", { status: 404 });
    }
    const path = resolve(cacheRoot, name);
    if (!existsSync(path) || !statSync(path).isFile()) {
      return new Response("Not Found", { status: 404 });
    }
    const ext = extname(name).slice(1).toLowerCase();
    return new Response(Bun.file(path), {
      headers: {
        "content-type": FONT_MIME[ext] ?? "application/octet-stream",
        "cache-control": "public, max-age=31536000, immutable",
        "access-control-allow-origin": "*",
      },
    });
  };
}

/** `<link rel="preload" as="font">` tags for the preloadable assets. */
export function fontPreloadLinks(assets: ReadonlyArray<FontFaceAsset>): string {
  const seen = new Set<string>();
  const links: string[] = [];
  for (const asset of assets) {
    if (!asset.preload || seen.has(asset.localPath)) {
      continue;
    }
    seen.add(asset.localPath);
    const ext = extname(asset.localPath).slice(1).toLowerCase();
    const type = FONT_MIME[ext] ?? "font/woff2";
    links.push(`<link rel="preload" as="font" type="${type}" crossorigin href="${asset.localPath}">`);
  }
  return links.join("\n");
}

/** Font stacks of the 8 Yolo presets (sans / mono), mirroring `@yolo-rs/react` PRESET_CATALOG. */
export const PRESET_FONT_STACKS: Readonly<Record<YoloPresetId, { readonly sans: string; readonly mono: string }>> = {
  "yolo-neutral": {
    sans: "Inter, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
    mono: "'JetBrains Mono', ui-monospace, SFMono-Regular, Consolas, monospace",
  },
  "yolo-m3": {
    sans: "'Google Sans Flex', 'Google Sans', Roboto, system-ui, sans-serif",
    mono: "'Google Sans Code', 'JetBrains Mono', monospace",
  },
  "yolo-fluent": {
    sans: "'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif",
    mono: "'Cascadia Code', Consolas, monospace",
  },
  "yolo-apple": {
    sans: "'SF Pro', -apple-system, BlinkMacSystemFont, 'Helvetica Neue', sans-serif",
    mono: "'SF Mono', ui-monospace, Menlo, monospace",
  },
  "yolo-omarchy": {
    sans: "'Geist Variable', 'JetBrains Mono Variable', 'CaskaydiaMono Nerd Font', monospace",
    mono: "'JetBrains Mono Variable', 'JetBrains Mono', 'CaskaydiaMono Nerd Font', monospace",
  },
  "yolo-hyprland": {
    sans: "'Inter', 'Noto Sans', 'JetBrains Mono', system-ui, sans-serif",
    mono: "'JetBrains Mono', 'CaskaydiaCove Nerd Font', monospace",
  },
  "yolo-aphrody": {
    sans: "'Google Sans Flex', 'Zed Plex Sans', Inter, system-ui, sans-serif",
    mono: "'Google Sans Code', 'Zed Plex Mono', 'JetBrains Mono', monospace",
  },
  "yolo-chromium": {
    sans: "'Google Sans Flex', system-ui",
    mono: "'Google Sans Code', monospace",
  },
};

/** Sans and mono font stacks of a Yolo preset. */
export function presetFontStack(preset: YoloPresetId): { readonly sans: string; readonly mono: string } {
  return PRESET_FONT_STACKS[preset];
}
