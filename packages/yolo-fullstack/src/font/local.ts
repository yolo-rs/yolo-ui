// SPDX-License-Identifier: MIT OR Apache-2.0
// `next/font/local`: font files of the project become `@font-face` rules in a generated stylesheet whose
// `url()`s point at the files; the CSS bundler copies them next to the client assets.
import { existsSync } from "node:fs";
import { isAbsolute, join, relative, resolve } from "node:path";
import { type FontArtifact, fontClassNames, fontFamilyList, fontId, writeArtifact } from "./shared";

export interface LocalFontSrc {
  path: string;
  weight?: string;
  style?: string;
}

export interface LocalFontOptions {
  src: string | LocalFontSrc[];
  display?: "auto" | "block" | "swap" | "fallback" | "optional";
  weight?: string;
  style?: string;
  variable?: string;
  preload?: boolean;
  fallback?: string[];
  adjustFontFallback?: string | false;
  declarations?: { prop: string; value: string }[];
}

export interface LocalFontContext {
  cacheDir: string;
  /** Directory of the module calling `localFont` (relative `src` paths resolve from it). */
  importerDir: string;
}

const FORMATS: Record<string, string> = {
  woff2: "woff2",
  woff: "woff",
  ttf: "truetype",
  otf: "opentype",
  eot: "embedded-opentype",
};
const toPosix = (path: string) => path.replaceAll("\\", "/");

/** The family name derived from the first file (`my-font.woff2` -> `my-font`), as Next does. */
export function localFamilyName(src: string | LocalFontSrc[]): string {
  const first = typeof src === "string" ? src : src[0]?.path;
  if (!first) throw new Error("@yolo-rs/fullstack: localFont needs at least one src");
  return first
    .split(/[\\/]/)
    .pop()!
    .replace(/\.[a-z0-9]+$/i, "");
}

/** Resolve a local font into its stylesheet. */
export async function localFont(options: LocalFontOptions, ctx: LocalFontContext): Promise<FontArtifact> {
  const family = localFamilyName(options.src);
  const srcs: LocalFontSrc[] =
    typeof options.src === "string"
      ? [{ path: options.src, weight: options.weight, style: options.style }]
      : options.src;
  const files = srcs.map(s => ({ ...s, abs: isAbsolute(s.path) ? s.path : resolve(ctx.importerDir, s.path) }));
  for (const file of files)
    if (!existsSync(file.abs))
      throw new Error(`@yolo-rs/fullstack: localFont src ${file.path} not found (from ${ctx.importerDir})`);
  const id = fontId("local", family, { ...options, files: files.map(f => toPosix(f.abs)) });
  const dir = join(ctx.cacheDir, id);
  const cssFile = join(dir, "font.css");
  const extra = (options.declarations ?? []).map(d => `${d.prop}:${d.value};`).join("");
  const faces = files
    .map(f => {
      const ext = f.abs.split(".").pop()!.toLowerCase();
      const format = FORMATS[ext] ? `format("${FORMATS[ext]}")` : "";
      const url = toPosix(relative(dir, f.abs));
      return `@font-face {font-family:'${family}';src:url("${url.startsWith(".") ? url : `./${url}`}") ${format};${(f.weight ?? options.weight) ? `font-weight:${f.weight ?? options.weight};` : ""}${(f.style ?? options.style) ? `font-style:${f.style ?? options.style};` : ""}${extra}}`;
    })
    .join("\n");
  const names = fontClassNames(id, options.variable);
  const single = (value: string | undefined) => (value && !/\s/.test(value) ? value : undefined);
  const artifact: FontArtifact = {
    id,
    kind: "local",
    family,
    cssFile,
    className: names.className,
    variableClassName: names.variableClassName,
    variable: options.variable,
    style: {
      fontFamily: fontFamilyList(family, options.fallback ?? []),
      ...(files.length === 1 && single(options.weight) && Number(options.weight)
        ? { fontWeight: Number(options.weight) }
        : {}),
      ...(files.length === 1 && single(options.style) ? { fontStyle: options.style } : {}),
    },
    preload: options.preload === false ? [] : files.map(f => toPosix(relative(dir, f.abs))),
    fallback: false,
  };
  await writeArtifact(artifact, faces, options.display ?? "swap");
  return artifact;
}
