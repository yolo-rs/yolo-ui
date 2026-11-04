// SPDX-License-Identifier: Apache-2.0
/**
 * `@yolo-rs/shadcn-bun`: 100% Native Bun v1.4.4+ Shadcn UI, Base UI, Radix UI & Tailwind CSS v4
 * Inspector & Rust Multi-Engine Migrator Bridge (`yolo-migrator` / `yolo-layout`).
 *
 * Uses strictly native Bun APIs (`Bun.file`, `Bun.write`, `Bun.Glob`, `Bun.$`, `Bun.spawn`)
 * with zero Node.js wrapper imports.
 */

import { $ } from "bun";

/** All 6 Rust UI migration targets supported by `yolo-migrator`. */
export type YoloMigrationTarget =
  "dioxus" | "wgpu" | "egui" | "cosmic" | "gtk" | "qml";

/** All 7 Yolo UI theme presets (`yolo-shared::YoloThemePreset`). */
export type YoloThemePresetId =
  "neutral" | "m3" | "fluent" | "apple" | "omarchy" | "hyprland" | "aphrody";

export const YOLO_MIGRATION_TARGETS: readonly YoloMigrationTarget[] = [
  "dioxus",
  "wgpu",
  "egui",
  "cosmic",
  "gtk",
  "qml",
] as const;

export const YOLO_THEME_PRESETS: readonly YoloThemePresetId[] = [
  "neutral",
  "m3",
  "fluent",
  "apple",
  "omarchy",
  "hyprland",
  "aphrody",
] as const;

/** Parsed Shadcn UI `components.json` configuration. */
export interface ShadcnComponentsConfig {
  $schema?: string;
  style: string;
  rsc: boolean;
  tsx: boolean;
  tailwind: {
    config?: string;
    css: string;
    baseColor: string;
    cssVariables: boolean;
    prefix?: string;
  };
  aliases: {
    components: string;
    utils: string;
    ui?: string;
    lib?: string;
    hooks?: string;
  };
  iconLibrary?: string;
}

/** Parsed Tailwind CSS v4 `@theme` and `:root` / `.dark` CSS custom property tokens. */
export interface TailwindThemeTokens {
  /** `--color-*` tokens from `@theme` (e.g. `primary` -> `oklch(...)` or `var(--primary)`). */
  colors: Record<string, string>;
  /** `--radius-*` tokens in px or raw CSS units. */
  radii: Record<string, string>;
  /** `--spacing-*` tokens. */
  spacing: Record<string, string>;
  /** `--font-*` font family tokens. */
  fonts: Record<string, string>;
  /** Light-mode `:root` Shadcn semantic variables (`--background`, `--primary`, `--radius`, etc.). */
  rootVars: Record<string, string>;
  /** Dark-mode `.dark` Shadcn semantic variables. */
  darkVars: Record<string, string>;
}

/** Extracted `cva()` (`class-variance-authority`) variant specification from a Shadcn/BaseUI component. */
export interface CvaVariantSpec {
  /** Variable identifier (e.g. `"buttonVariants"`). */
  ident: string;
  /** Base Tailwind classes applied unconditionally. */
  baseClasses: string;
  /** Variant axes (`variant` -> `{ default: "bg-primary ...", outline: "border ..." }`). */
  axes: Record<string, Record<string, string>>;
  /** Default variant selection (`{ variant: "default", size: "default" }`). */
  defaultVariants: Record<string, string>;
}

/** Discovered Shadcn / Base UI / Radix UI component file metadata. */
export interface DiscoveredComponent {
  /** File path relative to project root (normalized with `/`). */
  relativePath: string;
  /** Absolute file path on disk. */
  absolutePath: string;
  /** Inferred component stem (e.g. `"button"`, `"card"`, `"dialog"`). */
  stem: string;
  /** Detected primitive family (`"shadcn"`, `"radix"`, `"base-ui"`, `"react"`). */
  family: "shadcn" | "radix" | "base-ui" | "react";
  /** Imported `@radix-ui/react-*` or `@base-ui-components/react/*` modules. */
  primitiveImports: string[];
  /** Extracted `cva()` variant specs in this file. */
  cvaSpecs: CvaVariantSpec[];
}

/** Complete inspection summary of a Shadcn UI + Base UI + Radix + Tailwind project. */
export interface ShadcnProjectInspection {
  rootDir: string;
  config: ShadcnComponentsConfig | null;
  tailwindConfigPath: string | null;
  themeTokens: TailwindThemeTokens;
  components: DiscoveredComponent[];
}

/** Options for invoking `yolo-migrate` from Bun. */
export interface YoloMigrateOptions {
  /** Input `.tsx` / `.jsx` file path. */
  inputPath: string;
  /** Output file path (`.rs`, `.ui`, or `.qml`). */
  outputPath: string;
  /** Target Rust UI engine (`"dioxus"`, `"wgpu"`, `"egui"`, `"cosmic"`, `"gtk"`, `"qml"`). */
  target: YoloMigrationTarget;
  /** Theme preset (`"neutral"`, `"m3"`, `"fluent"`, `"apple"`, `"omarchy"`, `"hyprland"`, `"aphrody"`). */
  theme?: YoloThemePresetId;
  /** Optional explicit path to compiled `yolo-migrate` binary. */
  binaryPath?: string;
  /** Optional Cargo manifest path if falling back to `cargo run --bin yolo-migrate`. */
  cargoManifestPath?: string;
}

/** Joins path segments using forward slashes without importing `node:path`. */
export function joinPath(...parts: string[]): string {
  return parts
    .filter((p) => p.length > 0)
    .map((p, idx) => {
      const normalized = p.replaceAll("\\", "/");
      if (idx === 0) {
        return normalized.replace(/\/+$/, "");
      }
      return normalized.replace(/^\/+|\/+$/g, "");
    })
    .join("/");
}

/**
 * Parses a Shadcn `components.json` file using `Bun.file(path).json()`.
 */
export async function readShadcnConfig(
  projectRoot: string,
): Promise<ShadcnComponentsConfig | null> {
  const configFile = Bun.file(joinPath(projectRoot, "components.json"));
  if (!(await configFile.exists())) {
    return null;
  }
  const raw = (await configFile.json()) as Partial<ShadcnComponentsConfig>;
  return {
    $schema: raw.$schema,
    style: raw.style ?? "new-york",
    rsc: raw.rsc ?? true,
    tsx: raw.tsx ?? true,
    tailwind: {
      config: raw.tailwind?.config ?? "",
      css: raw.tailwind?.css ?? "src/app/globals.css",
      baseColor: raw.tailwind?.baseColor ?? "neutral",
      cssVariables: raw.tailwind?.cssVariables ?? true,
      prefix: raw.tailwind?.prefix ?? "",
    },
    aliases: {
      components: raw.aliases?.components ?? "@/components",
      utils: raw.aliases?.utils ?? "@/lib/utils",
      ui: raw.aliases?.ui ?? "@/components/ui",
      lib: raw.aliases?.lib ?? "@/lib",
      hooks: raw.aliases?.hooks ?? "@/hooks",
    },
    iconLibrary: raw.iconLibrary ?? "lucide",
  };
}

/**
 * Parses Tailwind CSS v4 `@theme` blocks and Shadcn `:root` / `.dark` CSS variables
 * from a stylesheet string.
 */
export /* @__PURE__ */ function parseTailwindV4Theme(
  cssSource: string,
): TailwindThemeTokens {
  const tokens: TailwindThemeTokens = {
    colors: {},
    radii: {},
    spacing: {},
    fonts: {},
    rootVars: {},
    darkVars: {},
  };

  for (const block of extractCssBlocks(cssSource, "@theme")) {
    for (const [prop, val] of parseCssCustomProps(block)) {
      if (prop.startsWith("--color-")) {
        tokens.colors[prop.slice("--color-".length)] = val;
      } else if (prop.startsWith("--radius-")) {
        tokens.radii[prop.slice("--radius-".length)] = val;
      } else if (prop.startsWith("--spacing-")) {
        tokens.spacing[prop.slice("--spacing-".length)] = val;
      } else if (prop.startsWith("--font-")) {
        tokens.fonts[prop.slice("--font-".length)] = val;
      }
    }
  }

  for (const block of extractCssBlocks(cssSource, ":root")) {
    for (const [prop, val] of parseCssCustomProps(block)) {
      tokens.rootVars[prop.slice(2)] = val;
    }
  }

  for (const block of extractCssBlocks(cssSource, ".dark")) {
    for (const [prop, val] of parseCssCustomProps(block)) {
      tokens.darkVars[prop.slice(2)] = val;
    }
  }

  return tokens;
}

/**
 * Extracts `cva()` (`class-variance-authority`) variant specifications from TSX/TS source code.
 */
export /* @__PURE__ */ function extractCvaVariants(
  tsxSource: string,
): CvaVariantSpec[] {
  const specs: CvaVariantSpec[] = [];
  const cvaRegex =
    /(?:export\s+)?const\s+([A-Za-z0-9_$]+)\s*=\s*cva\s*\(\s*(["'`])([\s\S]*?)\2\s*(?:,\s*(\{[\s\S]*?\}))?\s*\)/g;

  for (const match of tsxSource.matchAll(cvaRegex)) {
    const ident = match[1]!;
    const baseClasses = match[3]!.trim();
    const configBlock = match[4] ?? "";
    const axes: Record<string, Record<string, string>> = {};
    const defaultVariants: Record<string, string> = {};

    const variantsBody = extractNamedBraceBlock(configBlock, "variants");
    if (variantsBody) {
      for (const [axisName, axisBlock] of extractChildBraceBlocks(
        variantsBody,
      )) {
        const options: Record<string, string> = {};
        const kvRegex = /(["']?)([A-Za-z0-9_-]+)\1\s*:\s*(["'`])([\s\S]*?)\3/g;
        for (const kv of axisBlock.matchAll(kvRegex)) {
          options[kv[2]!] = kv[4]!.trim();
        }
        if (Object.keys(options).length > 0) {
          axes[axisName] = options;
        }
      }
    }

    const defaultsBody = extractNamedBraceBlock(configBlock, "defaultVariants");
    if (defaultsBody) {
      const kvRegex = /(["']?)([A-Za-z0-9_-]+)\1\s*:\s*(["'`])([\s\S]*?)\3/g;
      for (const kv of defaultsBody.matchAll(kvRegex)) {
        defaultVariants[kv[2]!] = kv[4]!.trim();
      }
    }

    specs.push({ ident, baseClasses, axes, defaultVariants });
  }

  return specs;
}

/**
 * Scans a directory using `Bun.Glob` to discover all Shadcn UI, Base UI, and Radix UI `.tsx` components.
 */
export async function discoverShadcnComponents(
  projectRoot: string,
): Promise<DiscoveredComponent[]> {
  const glob = new Bun.Glob("**/*.tsx");
  const discovered: DiscoveredComponent[] = [];

  for await (const rel of glob.scan({ cwd: projectRoot, onlyFiles: true })) {
    const normalizedRel = rel.replaceAll("\\", "/");
    if (
      normalizedRel.includes("node_modules/") ||
      normalizedRel.includes("dist/") ||
      normalizedRel.endsWith(".test.tsx") ||
      normalizedRel.endsWith(".spec.tsx")
    ) {
      continue;
    }

    const abs = joinPath(projectRoot, normalizedRel);
    const source = await Bun.file(abs).text();
    const primitiveImports: string[] = [];
    const importRegex = /from\s+["']([^"']+)["']/g;
    for (const m of source.matchAll(importRegex)) {
      const spec = m[1]!;
      if (
        spec.startsWith("@radix-ui/") ||
        spec.startsWith("@base-ui-components/") ||
        spec.startsWith("@base-ui/") ||
        spec.startsWith("@/components/ui/")
      ) {
        primitiveImports.push(spec);
      }
    }

    const cvaSpecs = extractCvaVariants(source);
    const family: DiscoveredComponent["family"] = primitiveImports.some(
      (i) => i.startsWith("@base-ui-components/") || i.startsWith("@base-ui/"),
    )
      ? "base-ui"
      : primitiveImports.some((i) => i.startsWith("@radix-ui/"))
        ? "radix"
        : normalizedRel.includes("components/ui/") ||
            primitiveImports.some((i) => i.startsWith("@/components/ui/")) ||
            cvaSpecs.length > 0
          ? "shadcn"
          : "react";

    const fileName = normalizedRel.split("/").pop() ?? normalizedRel;
    const stem = fileName.replace(/\.tsx$/i, "");

    discovered.push({
      relativePath: normalizedRel,
      absolutePath: abs,
      stem,
      family,
      primitiveImports,
      cvaSpecs,
    });
  }

  return discovered.sort((a, b) =>
    a.relativePath.localeCompare(b.relativePath),
  );
}

/**
 * Inspects a Shadcn UI / Base UI / Radix UI / Tailwind CSS v3/v4 project root using 100% native Bun APIs.
 */
export async function inspectShadcnProject(
  projectRoot: string,
): Promise<ShadcnProjectInspection> {
  const config = await readShadcnConfig(projectRoot);

  let tailwindConfigPath: string | null = null;
  for (const candidate of [
    config?.tailwind.config,
    "tailwind.config.ts",
    "tailwind.config.js",
    "tailwind.config.mjs",
  ]) {
    if (!candidate) continue;
    const full = joinPath(projectRoot, candidate);
    if (await Bun.file(full).exists()) {
      tailwindConfigPath = full;
      break;
    }
  }

  let themeTokens: TailwindThemeTokens = {
    colors: {},
    radii: {},
    spacing: {},
    fonts: {},
    rootVars: {},
    darkVars: {},
  };

  const cssCandidates = [
    config?.tailwind.css,
    "src/app/globals.css",
    "app/globals.css",
    "src/index.css",
    "styles/globals.css",
  ].filter((c): c is string => Boolean(c));

  for (const cssRel of cssCandidates) {
    const cssFile = Bun.file(joinPath(projectRoot, cssRel));
    if (await cssFile.exists()) {
      themeTokens = parseTailwindV4Theme(await cssFile.text());
      break;
    }
  }

  const components = await discoverShadcnComponents(projectRoot);
  return {
    rootDir: projectRoot.replaceAll("\\", "/"),
    config,
    tailwindConfigPath,
    themeTokens,
    components,
  };
}

/**
 * Generates a Rust `yolo-shared` / `yolo-layout` theme & variant helper module from
 * a `ShadcnProjectInspection` without requiring an external subprocess.
 */
export /* @__PURE__ */ function generateRustProjectBridge(
  inspection: ShadcnProjectInspection,
  preset: YoloThemePresetId = "neutral",
): string {
  const lines: string[] = [
    "// @generated by @yolo-rs/shadcn-bun — DO NOT EDIT",
    "use yolo_shared::{YoloThemePreset, YoloThemeTokens};",
    "",
    `pub const DEFAULT_THEME_PRESET: YoloThemePreset = ${presetToRustVariant(preset)};`,
    "",
    "pub fn project_theme_tokens() -> YoloThemeTokens {",
    "    YoloThemeTokens::from_preset(DEFAULT_THEME_PRESET)",
    "}",
  ];

  for (const comp of inspection.components) {
    for (const cva of comp.cvaSpecs) {
      for (const [axisName, options] of Object.entries(cva.axes)) {
        const enumName = `${toPascalCase(cva.ident)}${toPascalCase(axisName)}`;
        const defaultOpt =
          cva.defaultVariants[axisName] ?? Object.keys(options)[0];
        lines.push("");
        lines.push(
          "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]",
        );
        lines.push(`pub enum ${enumName} {`);
        for (const optKey of Object.keys(options)) {
          if (optKey === defaultOpt) {
            lines.push("    #[default]");
          }
          lines.push(`    ${toPascalCase(optKey)},`);
        }
        lines.push("}");
      }
    }
  }

  lines.push("");
  return lines.join("\n");
}

/**
 * Invokes `yolo-migrate` via `Bun.$` / `Bun.spawn` to compile a Shadcn/BaseUI/Radix/Tailwind `.tsx`
 * component into Rust (`dioxus`, `wgpu`, `egui`, `cosmic`, `gtk`, or `qml`).
 */
export async function migrateToRust(options: YoloMigrateOptions): Promise<{
  exitCode: number;
  outputPath: string;
  stdout: string;
  stderr: string;
}> {
  const theme = options.theme ?? "neutral";
  const args = [
    "--target",
    options.target,
    "--theme",
    theme,
    "--out",
    options.outputPath,
    options.inputPath,
  ];

  if (options.binaryPath && (await Bun.file(options.binaryPath).exists())) {
    const proc = Bun.spawn([options.binaryPath, ...args], {
      stdout: "pipe",
      stderr: "pipe",
    });
    const [stdout, stderr, exitCode] = await Promise.all([
      proc.stdout.text(),
      proc.stderr.text(),
      proc.exited,
    ]);
    return { exitCode, outputPath: options.outputPath, stdout, stderr };
  }

  const manifestArgs = options.cargoManifestPath
    ? ["--manifest-path", options.cargoManifestPath]
    : ["-p", "yolo-migrator"];
  const res =
    await $`cargo run --quiet ${manifestArgs} --bin yolo-migrate -- ${args}`
      .nothrow()
      .quiet();

  return {
    exitCode: res.exitCode,
    outputPath: options.outputPath,
    stdout: res.stdout.toString(),
    stderr: res.stderr.toString(),
  };
}

function presetToRustVariant(preset: YoloThemePresetId): string {
  switch (preset) {
    case "neutral":
      return "YoloThemePreset::Neutral";
    case "m3":
      return "YoloThemePreset::M3";
    case "fluent":
      return "YoloThemePreset::Fluent";
    case "apple":
      return "YoloThemePreset::Apple";
    case "omarchy":
      return "YoloThemePreset::Omarchy";
    case "hyprland":
      return "YoloThemePreset::Hyprland";
    case "aphrody":
      return "YoloThemePreset::Aphrody";
  }
}

function toPascalCase(raw: string): string {
  const out = raw
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean)
    .map((part) => part[0]!.toUpperCase() + part.slice(1))
    .join("");
  if (!out) return "Default";
  return /^[0-9]/.test(out) ? `V${out}` : out;
}

function extractCssBlocks(css: string, selector: string): string[] {
  const blocks: string[] = [];
  let searchFrom = 0;
  while (searchFrom < css.length) {
    const idx = css.indexOf(selector, searchFrom);
    if (idx === -1) break;
    const braceStart = css.indexOf("{", idx + selector.length);
    if (braceStart === -1) break;
    let depth = 0;
    let endIdx = -1;
    for (let i = braceStart; i < css.length; i++) {
      if (css[i] === "{") depth++;
      else if (css[i] === "}") {
        depth--;
        if (depth === 0) {
          endIdx = i;
          break;
        }
      }
    }
    if (endIdx !== -1) {
      blocks.push(css.slice(braceStart + 1, endIdx));
      searchFrom = endIdx + 1;
    } else {
      break;
    }
  }
  return blocks;
}

function parseCssCustomProps(block: string): Array<[string, string]> {
  const entries: Array<[string, string]> = [];
  for (const rawDecl of block.split(";")) {
    const decl = rawDecl.trim();
    if (!decl.startsWith("--")) continue;
    const colonIdx = decl.indexOf(":");
    if (colonIdx === -1) continue;
    const prop = decl.slice(0, colonIdx).trim();
    const val = decl.slice(colonIdx + 1).trim();
    if (prop && val) {
      entries.push([prop, val]);
    }
  }
  return entries;
}

function extractNamedBraceBlock(source: string, key: string): string | null {
  const idx = source.indexOf(key);
  if (idx === -1) return null;
  const braceStart = source.indexOf("{", idx + key.length);
  if (braceStart === -1) return null;
  let depth = 0;
  for (let i = braceStart; i < source.length; i++) {
    if (source[i] === "{") depth++;
    else if (source[i] === "}") {
      depth--;
      if (depth === 0) {
        return source.slice(braceStart + 1, i);
      }
    }
  }
  return null;
}

function extractChildBraceBlocks(body: string): Array<[string, string]> {
  const children: Array<[string, string]> = [];
  let i = 0;
  while (i < body.length) {
    const braceStart = body.indexOf("{", i);
    if (braceStart === -1) break;
    const prefix = body.slice(i, braceStart);
    const keyMatch = /([A-Za-z0-9_-]+)\s*:\s*$/.exec(prefix);
    let depth = 0;
    let endIdx = -1;
    for (let j = braceStart; j < body.length; j++) {
      if (body[j] === "{") depth++;
      else if (body[j] === "}") {
        depth--;
        if (depth === 0) {
          endIdx = j;
          break;
        }
      }
    }
    if (endIdx === -1) break;
    if (keyMatch?.[1]) {
      children.push([keyMatch[1], body.slice(braceStart + 1, endIdx)]);
    }
    i = endIdx + 1;
  }
  return children;
}
