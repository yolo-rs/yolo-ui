import { dlopen, FFIType, ptr, suffix } from "bun:ffi";
import { spawn } from "bun";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// ============================================================================
// 1. Cross-Platform Zero-Copy FFI Binding to `yolo_zig` (`bun:ffi`)
// ============================================================================
const PKG_DIR = dirname(fileURLToPath(import.meta.url));
const YOLO_UI_ROOT = resolve(PKG_DIR, "..", "..", "..");

function resolveZigLibraryPath(): string {
  const envOverride = process.env.YOLO_ZIG_LIB_PATH;
  if (envOverride && existsSync(envOverride)) {
    return envOverride;
  }

  const zigOutDir = join(YOLO_UI_ROOT, "zig", "zig-out");
  const candidates =
    process.platform === "win32"
      ? [
          join(zigOutDir, "bin", `yolo_zig.${suffix}`),
          join(zigOutDir, "lib", `yolo_zig.${suffix}`),
          join(zigOutDir, "bin", `libyolo_zig.${suffix}`),
        ]
      : [
          join(zigOutDir, "lib", `libyolo_zig.${suffix}`),
          join(zigOutDir, "lib", `yolo_zig.${suffix}`),
          join(zigOutDir, "bin", `libyolo_zig.${suffix}`),
        ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }
  throw new Error(
    `[yolo-design] Native Zig library not found. Expected one of: ${candidates.join(", ")}`
  );
}

export const zigLibPath = resolveZigLibraryPath();

export const yoloZig = dlopen(zigLibPath, {
  yolo_apca_contrast: {
    args: [FFIType.f32, FFIType.f32, FFIType.f32, FFIType.f32, FFIType.f32, FFIType.f32],
    returns: FFIType.f32,
  },
  yolo_wcag_contrast_ratio: {
    args: [FFIType.f32, FFIType.f32, FFIType.f32, FFIType.f32, FFIType.f32, FFIType.f32],
    returns: FFIType.f32,
  },
  yolo_concentric_inner_radius: {
    args: [FFIType.f32, FFIType.f32],
    returns: FFIType.f32,
  },
  yolo_rounded_rect_sdf_coverage: {
    args: [
      FFIType.f32,
      FFIType.f32,
      FFIType.f32,
      FFIType.f32,
      FFIType.f32,
      FFIType.f32,
      FFIType.f32,
    ],
    returns: FFIType.f32,
  },
  yolo_is_ai_purple_slop: {
    args: [FFIType.f32, FFIType.f32, FFIType.f32],
    returns: FFIType.bool,
  },
  yolo_spring_position: {
    args: [FFIType.f32, FFIType.f32, FFIType.f32],
    returns: FFIType.f32,
  },
  yolo_golden_signature: {
    args: [FFIType.ptr, FFIType.usize],
    returns: FFIType.u64,
  },
});

export function computeGoldenSignature(input: string | Uint8Array): bigint {
  const bytes = typeof input === "string" ? new TextEncoder().encode(input) : input;
  if (bytes.byteLength === 0) {
    return 0xcbf29ce484222325n;
  }
  return BigInt(yoloZig.symbols.yolo_golden_signature(ptr(bytes), bytes.byteLength));
}

export function formatGoldenHex(sig: bigint): string {
  return `0x${BigInt.asUintN(64, sig).toString(16).padStart(16, "0")}`;
}

// ============================================================================
// 2. Hybrid `DESIGN.md` Parser, 11-Rule Linter & 6-Target Codegen Compiler
// ============================================================================
export interface TypographyToken {
  fontFamily: string;
  fontSize: string;
  fontWeight?: number;
  lineHeight?: number;
  letterSpacing?: string;
  fontFeature?: string;
}

export interface DesignMdSpec {
  name: string;
  description?: string;
  colors: Record<string, string>;
  typography: Record<string, TypographyToken>;
  rounded: Record<string, string>;
  spacing: Record<string, string>;
  sections: string[];
  interleavedYamlBlocks: number;
}

export const CANONICAL_DESIGN_MD_SECTIONS = [
  "Overview",
  "Colors",
  "Typography",
  "Layout",
  "Elevation & Depth",
  "Shapes",
  "Components",
  "Do's and Don'ts",
] as const;

export function parseDesignMd(rawContent: string): DesignMdSpec {
  const normalized = rawContent.replace(/\r\n/g, "\n").trimStart();
  if (!normalized.startsWith("---\n")) {
    throw new Error("DESIGN.md must start with a YAML frontmatter block delimited by ---");
  }
  const endIdx = normalized.indexOf("\n---", 4);
  if (endIdx === -1) {
    throw new Error("Unclosed YAML frontmatter block in DESIGN.md");
  }

  const frontmatterStr = normalized.slice(4, endIdx);
  const body = normalized.slice(endIdx + 4).trim();
  const parsedFrontmatter = Bun.YAML.parse(frontmatterStr) as Record<string, unknown>;

  const sections: string[] = [];
  for (const line of body.split("\n")) {
    const match = /^##\s+(.+?)\s*$/.exec(line);
    if (match) {
      sections.push(match[1]);
    }
  }

  const yamlFenceRegex = /```ya?ml\n([\s\S]*?)```/g;
  let interleavedCount = 0;
  let fenceMatch: RegExpExecArray | null = null;
  while ((fenceMatch = yamlFenceRegex.exec(body)) !== null) {
    Bun.YAML.parse(fenceMatch[1]);
    interleavedCount += 1;
  }

  return {
    name: String(parsedFrontmatter.name ?? "YOLO Sovereign Design System"),
    description:
      typeof parsedFrontmatter.description === "string"
        ? parsedFrontmatter.description
        : undefined,
    colors: (parsedFrontmatter.colors as Record<string, string>) ?? {},
    typography: (parsedFrontmatter.typography as Record<string, TypographyToken>) ?? {},
    rounded: (parsedFrontmatter.rounded as Record<string, string>) ?? {},
    spacing: (parsedFrontmatter.spacing as Record<string, string>) ?? {},
    sections,
    interleavedYamlBlocks: interleavedCount,
  };
}

export function verifyAntiSlopInvariants(spec: DesignMdSpec): void {
  if (!spec.colors.primary) {
    throw new Error("[missing-primary] DESIGN.md must define colors.primary");
  }

  for (const [name, val] of Object.entries(spec.colors)) {
    const lower = val.toLowerCase();
    if (lower.includes("indigo") || lower.includes("#6366f1") || lower.includes("#8b5cf6")) {
      throw new Error(
        `[no-ai-purple-slop] Color token '${name}' uses banned AI indigo/purple (${val})`
      );
    }
  }

  const hasTabular = Object.values(spec.typography).some((t) => t.fontFeature === "tnum");
  if (!hasTabular) {
    throw new Error(
      '[missing-tabular-nums] At least one typography token must enable fontFeature: "tnum"'
    );
  }

  for (const requiredSection of CANONICAL_DESIGN_MD_SECTIONS) {
    if (!spec.sections.includes(requiredSection)) {
      throw new Error(
        `[missing-sections] DESIGN.md is missing canonical H2 section: ## ${requiredSection}`
      );
    }
  }
}

export function compileDesignMdToSixTargets(
  designMdPath: string,
  outDir = join(YOLO_UI_ROOT, "packages", "design-tokens", "dist")
): {
  files: string[];
  goldenSignature: string;
} {
  const raw = readFileSync(designMdPath, "utf8");
  const spec = parseDesignMd(raw);
  verifyAntiSlopInvariants(spec);
  mkdirSync(outDir, { recursive: true });

  // Target 1: Tailwind v4 CSS
  const cssLines: string[] = [
    "/* AUTO-GENERATED BY YOLO-DESIGN FROM DESIGN.md */",
    '@import "tailwindcss";',
    "",
    "@theme {",
  ];
  for (const [k, v] of Object.entries(spec.colors)) {
    cssLines.push(`  --color-${k}: ${v};`);
  }
  for (const [k, v] of Object.entries(spec.rounded)) {
    cssLines.push(`  --radius-${k}: ${v};`);
  }
  for (const [k, v] of Object.entries(spec.spacing)) {
    cssLines.push(`  --spacing-${k}: ${v};`);
  }
  for (const [k, t] of Object.entries(spec.typography)) {
    cssLines.push(`  --font-${k}: "${t.fontFamily}", sans-serif;`);
  }
  cssLines.push(
    "  --ease-emil-out: cubic-bezier(0.22, 1, 0.36, 1);",
    "  --duration-micro: 160ms;",
    "}",
    "",
    "h1, h2, h3, h4 { text-wrap: balance; letter-spacing: -0.03em; }",
    "p { text-wrap: pretty; }",
    ".metric, table, [data-numeric] { font-variant-numeric: tabular-nums; }",
    ".concentric-card {",
    "  border-radius: var(--radius-lg);",
    "  padding: var(--spacing-md);",
    "  border: 1px solid var(--color-border-hairline);",
    "}",
    ".concentric-card > .concentric-inner {",
    "  border-radius: calc(var(--radius-lg) - var(--spacing-md));",
    "}",
    ""
  );
  const tailwindCss = cssLines.join("\n");

  // Target 2: W3C DTCG JSON
  const dtcgColor: Record<string, { $value: string; $type: string }> = {};
  for (const [k, v] of Object.entries(spec.colors)) {
    dtcgColor[k] = { $value: v, $type: "color" };
  }
  const dtcgRounded: Record<string, { $value: string; $type: string }> = {};
  for (const [k, v] of Object.entries(spec.rounded)) {
    dtcgRounded[k] = { $value: v, $type: "dimension" };
  }
  const dtcgJson = JSON.stringify({ color: dtcgColor, rounded: dtcgRounded }, null, 2);

  // Target 3: Rust GPUI (`longbridge/gpui-component` v0.8)
  const gpuiFields = Object.keys(spec.colors)
    .map((k) => `    pub color_${k.replace(/-/g, "_")}: &'static str,`)
    .concat(
      Object.keys(spec.rounded).map((k) => `    pub radius_${k.replace(/-/g, "_")}: f32,`)
    );
  const gpuiValues = Object.entries(spec.colors)
    .map(([k, v]) => `    color_${k.replace(/-/g, "_")}: "${v}",`)
    .concat(
      Object.entries(spec.rounded).map(([k, v]) => {
        const px = Number.parseFloat(v.replace("px", "")) || 8.0;
        return `    radius_${k.replace(/-/g, "_")}: ${px.toFixed(1)},`;
      })
    );
  const gpuiRust = [
    "//! AUTO-GENERATED GPUI THEME FROM DESIGN.md",
    "",
    "#[derive(Debug, Clone, Copy)]",
    "pub struct YoloGpuiTheme {",
    ...gpuiFields,
    "}",
    "",
    "pub const YOLO_GPUI_THEME: YoloGpuiTheme = YoloGpuiTheme {",
    ...gpuiValues,
    "};",
    "",
    "impl YoloGpuiTheme {",
    "    #[inline]",
    "    pub const fn concentric_radius(outer_px: f32, padding_px: f32) -> f32 {",
    "        let diff = outer_px - padding_px;",
    "        if diff > 0.0 { diff } else { 0.0 }",
    "    }",
    "}",
    "",
  ].join("\n");

  // Target 4: Rust Dioxus 0.8 / Blitz
  const dioxusConsts = Object.entries(spec.colors)
    .map(
      ([k, v]) => `    pub const COLOR_${k.replace(/-/g, "_").toUpperCase()}: &str = "${v}";`
    )
    .concat(
      Object.entries(spec.rounded).map(
        ([k, v]) =>
          `    pub const RADIUS_${k.replace(/-/g, "_").toUpperCase()}: &str = "${v}";`
      )
    );
  const dioxusRust = [
    "//! AUTO-GENERATED DIOXUS 0.8 THEME FROM DESIGN.md",
    "",
    "pub mod tokens {",
    ...dioxusConsts,
    "}",
    "",
  ].join("\n");

  // Target 5: COSMIC Epoch (`pop-os/cosmic-epoch` RON)
  const cosmicRon = [
    "// AUTO-GENERATED COSMIC EPOCH THEME",
    "(",
    `    name: "${spec.name}",`,
    `    primary: "${spec.colors.primary ?? ""}",`,
    `    accent: "${spec.colors.tertiary ?? ""}",`,
    `    surface_dark: "${spec.colors["surface-dark"] ?? ""}",`,
    "    corner_radii: (",
    "        radius_xs: 4.0,",
    "        radius_s: 8.0,",
    "        radius_m: 12.0,",
    "        radius_l: 20.0,",
    "    ),",
    ")",
    "",
  ].join("\n");

  // Target 6: `vercel-labs/native` (`ui_schema.zig` schema_version = 2 lowercase elements)
  const outerPx = Number.parseFloat((spec.rounded.lg ?? "20px").replace("px", "")) || 20;
  const padPx = Number.parseFloat((spec.spacing.md ?? "12px").replace("px", "")) || 12;
  const innerPx = yoloZig.symbols.yolo_concentric_inner_radius(outerPx, padPx);

  const nativeMarkup = [
    `<!-- AUTO-GENERATED VERCEL-LABS/NATIVE VIEW (ui_schema.zig v2) FROM DESIGN.md (${spec.name}) -->`,
    `<column id="root" padding="${padPx * 2}" gap="16" bg="${spec.colors.neutral ?? "#0f1014"}">`,
    `  <row id="header-card" justify="between" align="center" padding="${padPx}" radius="${outerPx}" bg="${spec.colors["surface-1"] ?? "#12141a"}">`,
    `    <text id="title" size="28" weight="semibold" color="${spec.colors.primary ?? "#ff73a8"}">${spec.name} — Native Engine (Zig + MVU)</text>`,
    `    <button id="audit-btn" label="Verify DESIGN.md" press="RunDesignAudit" bg="${spec.colors.tertiary ?? "#ff9e3b"}" radius="${innerPx}" padding="${padPx}" />`,
    "  </row>",
    "</column>",
    "",
  ].join("\n");

  const nativeSchemaJson = JSON.stringify(
    {
      $schema: "https://schema.native-sdk.dev/app/v1.json",
      id: "com.aphrody.yolo.studio",
      name: "yolo-studio",
      display_name: spec.name,
      version: "0.8.0",
      theme: "geist",
      theme_accent: spec.colors.primary ?? "#ff73a8",
      compiledTypography: spec.typography,
      themeOverrides: {
        colors: spec.colors,
        radii: spec.rounded,
        spacing: spec.spacing,
        concentricInnerRadiusPx: innerPx,
      },
    },
    null,
    2
  );

  const written: Array<[string, string]> = [
    ["theme.css", tailwindCss],
    ["tokens.json", dtcgJson],
    ["gpui_theme.rs", gpuiRust],
    ["dioxus_theme.rs", dioxusRust],
    ["cosmic_theme.ron", cosmicRon],
    ["app.native", nativeMarkup],
    ["native.theme.schema.json", nativeSchemaJson],
  ];

  const files: string[] = [];
  let combinedPayload = "";
  for (const [name, content] of written) {
    const targetPath = join(outDir, name);
    writeFileSync(targetPath, content, "utf8");
    files.push(targetPath);
    combinedPayload += content;
  }

  return {
    files,
    goldenSignature: formatGoldenHex(computeGoldenSignature(combinedPayload)),
  };
}

// ============================================================================
// 3. Bridge to `vercel-labs/fx` (Zig Agent Kernel)
// ============================================================================
export interface FxAgentResponse {
  final_output: string;
  goldenSignature: string;
}

export async function invokeFxAgent(
  prompt: string,
  fxBin = "./forks/vercel-fx/zig-out/bin/fx"
): Promise<FxAgentResponse> {
  const proc = spawn([fxBin, "ask", "--json", "--full-access", prompt], {
    stdout: "pipe",
    stderr: "pipe",
    env: { ...process.env, FX_PERMISSION_MODE: "yolo" },
  });

  const rawOut = await new Response(proc.stdout).text();
  const code = await proc.exited;
  if (code !== 0) {
    const err = await new Response(proc.stderr).text();
    throw new Error(`[fx-bridge] Zig fx agent exited with code ${code}: ${err}`);
  }

  const parsed = JSON.parse(rawOut) as { final_output: string };
  const sig = computeGoldenSignature(parsed.final_output);
  return {
    final_output: parsed.final_output,
    goldenSignature: formatGoldenHex(sig),
  };
}

// ============================================================================
// 4. Generative UI Pipeline: Google Stitch (`@google/stitch-sdk`) + Zig Gate
// ============================================================================
export interface GenerativeScreenResult {
  screenId: string;
  htmlDownloadUrl: string;
  htmlContent?: string;
  screenshotDownloadUrl: string;
  concentricInnerRadiusPx: number;
  apcaBodyContrast: number;
  wcagBodyContrast: number;
  goldenHash: string;
}

export async function verifyScreenMathGate(options?: {
  outerRadiusPx?: number;
  paddingPx?: number;
  fgRgb?: [number, number, number];
  bgRgb?: [number, number, number];
}): Promise<{
  concentricInnerRadiusPx: number;
  apcaBodyContrast: number;
  wcagBodyContrast: number;
}> {
  const outer = options?.outerRadiusPx ?? 20;
  const padding = options?.paddingPx ?? 12;
  const [fgR, fgG, fgB] = options?.fgRgb ?? [0.953, 0.957, 0.965];
  const [bgR, bgG, bgB] = options?.bgRgb ?? [0.043, 0.047, 0.059];

  const innerRadius = yoloZig.symbols.yolo_concentric_inner_radius(outer, padding);
  const apcaScore = yoloZig.symbols.yolo_apca_contrast(fgR, fgG, fgB, bgR, bgG, bgB);
  const wcagScore = yoloZig.symbols.yolo_wcag_contrast_ratio(fgR, fgG, fgB, bgR, bgG, bgB);

  if (Math.abs(apcaScore) < 60) {
    throw new Error(
      `[Anti-Slop Gate] Insufficient APCA contrast (|${apcaScore.toFixed(1)}| < 60)`
    );
  }
  if (wcagScore < 4.5) {
    throw new Error(
      `[Anti-Slop Gate] Insufficient WCAG 2.1 contrast (${wcagScore.toFixed(2)} < 4.5)`
    );
  }

  return {
    concentricInnerRadiusPx: innerRadius,
    apcaBodyContrast: apcaScore,
    wcagBodyContrast: wcagScore,
  };
}

export async function fetchStitchSignedHtml(downloadUrl: string): Promise<string> {
  const response = await fetch(downloadUrl);
  if (!response.ok) {
    throw new Error(
      `[stitch-sdk] Failed to download HTML from signed URL (${response.status})`
    );
  }
  return await response.text();
}

// ============================================================================
// 5. WebGPU Shader Manifest (`@figma/expressive-components` + M3 Expressive)
// ============================================================================
export interface YoloExpressiveShaderConfig {
  name: string;
  isAnimated: boolean;
  usesMouse: boolean;
  stiffness: number;
  dampingRatio: number;
  settledPositionAt180ms: number;
}

export function createM3ExpressiveShaderManifest(
  name: string,
  stiffness = 1600,
  dampingRatio = 1.0
): YoloExpressiveShaderConfig {
  const settledPositionAt180ms = yoloZig.symbols.yolo_spring_position(
    0.18,
    stiffness,
    dampingRatio
  );
  return {
    name,
    isAnimated: true,
    usesMouse: true,
    stiffness,
    dampingRatio,
    settledPositionAt180ms,
  };
}
