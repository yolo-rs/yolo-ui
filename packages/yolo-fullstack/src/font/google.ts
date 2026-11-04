// SPDX-License-Identifier: MIT OR Apache-2.0
// `next/font/google` at build time: the Google Fonts CSS API is queried once per font and option set, the
// woff2 files are downloaded next to a generated stylesheet (self-hosted, no request to Google from the
// browser) and the result is cached under `.yolo/fonts/<id>/` so later builds are offline.
import { existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { type FontArtifact, fontClassNames, fontFamilyList, fontId, writeArtifact } from "./shared";

export interface GoogleFontOptions {
  weight?: string | string[];
  style?: string | string[];
  subsets?: string[];
  display?: "auto" | "block" | "swap" | "fallback" | "optional";
  variable?: string;
  preload?: boolean;
  fallback?: string[];
  adjustFontFallback?: boolean;
  axes?: string[];
}

/** The user agent Google serves woff2 with `unicode-range` subsets to. */
export const WOFF2_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";
export const GOOGLE_CSS_ENDPOINT = "https://fonts.googleapis.com/css2";

/** `Roboto_Mono` (the export name of `next/font/google`) is the family `Roboto Mono`. */
export function familyFromExport(name: string): string {
  return name.replaceAll("_", " ");
}

const list = (value: string | string[] | undefined): string[] => (value === undefined ? [] : Array.isArray(value) ? value : [value]);

/** The `family=` value of the CSS API: `Inter:wght@400;700`, `Roboto:ital,wght@0,400;1,400`, `Inter:wght@100..900`. */
export function familyParam(family: string, options: GoogleFontOptions): string {
  const weights = list(options.weight);
  const styles = list(options.style);
  const variable = weights.length === 0 || weights.includes("variable");
  const italic = styles.includes("italic");
  const normal = styles.length === 0 || styles.includes("normal");
  const axes = [...(options.axes ?? [])].sort();
  const name = family.replaceAll(" ", "+");
  if (variable) {
    const tags = [...(italic ? ["ital"] : []), "wght", ...axes.filter(a => a !== "wght")];
    const ranges = (ital: 0 | 1) => [...(italic ? [String(ital)] : []), "100..900", ...axes.filter(a => a !== "wght").map(() => "")].filter(v => v !== "").join(",");
    const tuples = italic ? [...(normal ? [ranges(0)] : []), ranges(1)] : [ranges(0)];
    return `${name}:${tags.join(",")}@${tuples.join(";")}`;
  }
  const sorted = [...weights].map(Number).sort((a, b) => a - b);
  if (!italic) return `${name}:wght@${sorted.join(";")}`;
  const tuples: string[] = [];
  if (normal) for (const w of sorted) tuples.push(`0,${w}`);
  for (const w of sorted) tuples.push(`1,${w}`);
  return `${name}:ital,wght@${tuples.join(";")}`;
}

/** The CSS API URL of a font. */
export function googleCssUrl(family: string, options: GoogleFontOptions, variableRange = true): string {
  const params = new URLSearchParams();
  params.set("family", variableRange ? familyParam(family, options) : family.replaceAll(" ", "+"));
  params.set("display", options.display ?? "swap");
  // `family` must keep `:`/`@`/`;`/`,`/`+` as Google expects them.
  return `${GOOGLE_CSS_ENDPOINT}?${params.toString().replaceAll("%3A", ":").replaceAll("%40", "@").replaceAll("%3B", ";").replaceAll("%2C", ",").replaceAll("%2B", "+").replaceAll("%2E", ".")}`;
}

export interface FontFaceBlock {
  subset?: string;
  css: string;
  urls: string[];
}

/** Split a Google CSS response into its `@font-face` blocks with the `/* subset *\/` comment that precedes each. */
export function parseFontFaces(css: string): FontFaceBlock[] {
  const blocks: FontFaceBlock[] = [];
  const re = /(?:\/\*\s*([\w-]+)\s*\*\/\s*)?@font-face\s*\{[^}]*\}/g;
  for (const m of css.matchAll(re)) {
    const urls = [...m[0].matchAll(/url\(([^)]+)\)/g)].map(u => u[1]!.replace(/^["']|["']$/g, ""));
    blocks.push({ subset: m[1], css: m[0].replace(/^\/\*[^*]*\*\/\s*/, ""), urls });
  }
  return blocks;
}

/** Keep the blocks of the requested subsets (every block when `subsets` is empty or the CSS has no subset comments). */
export function filterSubsets(blocks: FontFaceBlock[], subsets: string[] | undefined): FontFaceBlock[] {
  if (!subsets?.length || blocks.every(b => !b.subset)) return blocks;
  const wanted = new Set(subsets);
  return blocks.filter(b => !b.subset || wanted.has(b.subset));
}

export interface GoogleFontContext {
  /** Cache root (`<root>/.yolo/fonts`). */
  cacheDir: string;
  fetch?: typeof fetch;
  /** Never touch the network: fail when the cache has no entry. */
  offline?: boolean;
}

const fileName = (url: string, index: number): string => {
  const ext = /\.(woff2|woff|ttf|otf)(?:$|\?)/.exec(url)?.[1] ?? "woff2";
  return `${index}-${Bun.hash(url).toString(36)}.${ext}`;
};

/** Resolve (download or read from cache) one Google font. */
export async function googleFont(exportName: string, options: GoogleFontOptions, ctx: GoogleFontContext): Promise<FontArtifact> {
  const family = familyFromExport(exportName);
  const id = fontId("google", family, options);
  const dir = join(ctx.cacheDir, id);
  const cssFile = join(dir, "font.css");
  const metaFile = join(dir, "artifact.json");
  if (existsSync(cssFile) && existsSync(metaFile)) return (await Bun.file(metaFile).json()) as FontArtifact;
  if (ctx.offline) throw new Error(`@yolo-rs/fullstack: font ${family} is not cached and the build is offline`);
  const doFetch = ctx.fetch ?? fetch;
  const headers = { "user-agent": WOFF2_USER_AGENT, accept: "text/css,*/*;q=0.1" };
  let response = await doFetch(googleCssUrl(family, options, true), { headers });
  if (!response.ok && list(options.weight).length === 0) response = await doFetch(googleCssUrl(family, options, false), { headers });
  if (!response.ok) throw new Error(`@yolo-rs/fullstack: Google Fonts returned ${response.status} for ${family} (${JSON.stringify(options)})`);
  const blocks = filterSubsets(parseFontFaces(await response.text()), options.subsets);
  if (!blocks.length) throw new Error(`@yolo-rs/fullstack: Google Fonts returned no @font-face for ${family} with subsets ${JSON.stringify(options.subsets)}`);
  mkdirSync(join(dir, "files"), { recursive: true });
  const faces: string[] = [];
  const preload: string[] = [];
  let index = 0;
  for (const block of blocks) {
    let css = block.css;
    for (const url of block.urls) {
      const name = fileName(url, index++);
      const target = join(dir, "files", name);
      if (!existsSync(target)) {
        const file = await doFetch(url, { headers });
        if (!file.ok) throw new Error(`@yolo-rs/fullstack: download of ${url} failed (${file.status})`);
        await Bun.write(target, await file.arrayBuffer());
      }
      css = css.replace(url, `./files/${name}`);
      if (options.preload !== false && (!block.subset || (options.subsets ?? ["latin"]).includes(block.subset))) preload.push(`files/${name}`);
    }
    faces.push(css);
  }
  const weights = list(options.weight).filter(w => w !== "variable").map(Number);
  const names = fontClassNames(id, options.variable);
  const fallback = options.fallback ?? [];
  const artifact: FontArtifact = {
    id,
    kind: "google",
    family,
    cssFile,
    className: names.className,
    variableClassName: names.variableClassName,
    variable: options.variable,
    style: {
      fontFamily: fontFamilyList(family, fallback),
      ...(weights.length === 1 ? { fontWeight: weights[0] } : {}),
      ...(list(options.style).length === 1 ? { fontStyle: list(options.style)[0] } : {}),
    },
    preload,
    fallback: false,
  };
  await writeArtifact(artifact, faces.join("\n"), options.display ?? "swap");
  return artifact;
}
