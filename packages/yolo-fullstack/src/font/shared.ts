// SPDX-License-Identifier: MIT OR Apache-2.0
// What a resolved font is: a generated stylesheet (self-hosted files next to it), the class names applied
// by `className` / `variable`, and the `style` object, exactly what `next/font` hands a component.
import { mkdirSync } from "node:fs";
import { dirname } from "node:path";

export interface FontStyle {
  fontFamily: string;
  fontWeight?: number;
  fontStyle?: string;
}

export interface FontArtifact {
  /** Stable id of the font and its options (directory name under the cache). */
  id: string;
  kind: "google" | "local";
  family: string;
  /** Absolute path of the generated stylesheet. */
  cssFile: string;
  className: string;
  /** Class setting the CSS variable (`variable` option), when requested. */
  variableClassName?: string;
  variable?: string;
  style: FontStyle;
  /** Files to `<link rel="preload" as="font">`, relative to the stylesheet's directory. */
  preload: string[];
  /** The network was unavailable: the stylesheet only declares the fallback stack. */
  fallback: boolean;
  /** Stylesheet rewritten to public URLs (set by `materializeFont`). */
  builtCssFile?: string;
  /** Preloads relative to the client directory (set by `materializeFont`). */
  publicPreloads?: string[];
}

/** What the generated module exports to the app (`inter.className`, `inter.style`, `inter.variable`). */
export interface FontResult {
  className: string;
  style: FontStyle;
  variable?: string;
}

export function fontId(kind: string, family: string, options: unknown): string {
  return `${kind}-${family.toLowerCase().replace(/[^a-z0-9]+/g, "-")}-${Bun.hash(JSON.stringify([kind, family, options])).toString(36)}`;
}

export function fontClassNames(id: string, variable?: string): { className: string; variableClassName?: string } {
  const hash = Bun.hash(id).toString(36).slice(0, 8);
  return { className: `yf_${hash}`, variableClassName: variable ? `yfv_${hash}` : undefined };
}

const quote = (name: string) =>
  /^[a-zA-Z-]+$/.test(name) &&
  [
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-serif",
    "ui-sans-serif",
    "ui-monospace",
    "ui-rounded",
    "emoji",
    "math",
    "fangsong",
    "inherit",
    "initial",
    "unset",
  ].includes(name)
    ? name
    : `'${name.replace(/'/g, "\\'")}'`;

/** `'Inter', 'Segoe UI', sans-serif` from a family and its fallback list. */
export function fontFamilyList(family: string, fallback: string[]): string {
  return [family, ...fallback].map(quote).join(", ");
}

/** Write the stylesheet and the artifact record of a font. */
export async function writeArtifact(artifact: FontArtifact, faces: string, display: string): Promise<void> {
  mkdirSync(dirname(artifact.cssFile), { recursive: true });
  const rules = [
    faces.replace(/font-display:\s*[^;]+;/g, "").replace(/@font-face\s*\{/g, `@font-face {font-display:${display};`),
    `.${artifact.className}{font-family:${artifact.style.fontFamily};${artifact.style.fontWeight ? `font-weight:${artifact.style.fontWeight};` : ""}${artifact.style.fontStyle ? `font-style:${artifact.style.fontStyle};` : ""}}`,
    artifact.variableClassName && artifact.variable
      ? `.${artifact.variableClassName}{${artifact.variable}:${artifact.style.fontFamily};}`
      : "",
  ].filter(Boolean);
  await Bun.write(artifact.cssFile, `${rules.join("\n")}\n`);
  await Bun.write(`${dirname(artifact.cssFile)}/artifact.json`, JSON.stringify(artifact, null, 2));
}

/** A stylesheet that only names the fallback stack (offline builds). */
export function fallbackArtifact(base: Omit<FontArtifact, "fallback" | "preload">): FontArtifact {
  return { ...base, preload: [], fallback: true };
}
