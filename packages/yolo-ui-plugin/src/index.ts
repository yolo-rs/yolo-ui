import type { BunPlugin } from "bun";
import { readFileSync } from "node:fs";

export * as shadcnBun from "./shadcn-bun";

export type YoloPresetId =
  | "neutral"
  | "m3"
  | "fluent"
  | "apple"
  | "omarchy"
  | "hyprland"
  | "aphrody"
  | "chromium"
  | "yolo-neutral"
  | "yolo-m3"
  | "yolo-fluent"
  | "yolo-apple"
  | "yolo-omarchy"
  | "yolo-hyprland"
  | "yolo-aphrody"
  | "yolo-chromium";

export type NormalizedPresetId =
  "neutral" | "m3" | "fluent" | "apple" | "omarchy" | "hyprland" | "aphrody" | "chromium";

export type EmitTarget =
  | "css"
  | "gtk"
  | "kde"
  | "cosmic"
  | "qml"
  | "winui"
  | "fluent"
  | "apple"
  | "omarchy"
  | "hyprland"
  | "dtcg";

export type MigrateTarget =
  "dioxus" | "wgpu" | "egui" | "cosmic" | "gtk" | "qml";

export type DevtoolsEngineId =
  | "dioxus"
  | "gpui"
  | "egui"
  | "iced"
  | "cosmic"
  | "bevy"
  | "tauri"
  | "gtk4"
  | "qt6"
  | "shadcn-tailwind";

export interface YoloThemeTokens {
  id: NormalizedPresetId;
  brandId: `yolo-${NormalizedPresetId}`;
  name: string;
  backgroundPrimary: string;
  backgroundSecondary: string;
  surfaceElevated: string;
  textPrimary: string;
  textMuted: string;
  accentPrimary: string;
  accentSecondary: string;
  accentSuccess: string;
  accentWarning: string;
  accentDanger: string;
  borderSubtle: string;
  radiusPx: number;
  borderWidthPx: number;
  blurRadiusPx: number;
  fontSans: string;
  fontMono: string;
  gradientAngleDeg?: number;
  subtheme?: string;
}

export interface OmarchySubThemeSpec {
  id: string;
  name: string;
  bg: string;
  surface: string;
  fg: string;
  accent: string;
  secondary: string;
  border: string;
}

export const OMARCHY_SUBTHEMES: Record<string, OmarchySubThemeSpec> = {
  "tokyo-night": {
    id: "tokyo-night",
    name: "Tokyo Night",
    bg: "#1A1B26",
    surface: "#24283B",
    fg: "#C0CAF5",
    accent: "#7AA2F7",
    secondary: "#BB9AF7",
    border: "#414868",
  },
  catppuccin: {
    id: "catppuccin",
    name: "Catppuccin Mocha",
    bg: "#1E1E2E",
    surface: "#313244",
    fg: "#CDD6F4",
    accent: "#89B4FA",
    secondary: "#CBA6F7",
    border: "#45475A",
  },
  "catppuccin-latte": {
    id: "catppuccin-latte",
    name: "Catppuccin Latte",
    bg: "#EFF1F5",
    surface: "#E6E9EF",
    fg: "#4C4F69",
    accent: "#1E66F5",
    secondary: "#8839EF",
    border: "#BCC0CC",
  },
  gruvbox: {
    id: "gruvbox",
    name: "Gruvbox Dark",
    bg: "#282828",
    surface: "#3C3836",
    fg: "#EBDBB2",
    accent: "#FABD2F",
    secondary: "#83A598",
    border: "#504945",
  },
  kanagawa: {
    id: "kanagawa",
    name: "Kanagawa Wave",
    bg: "#1F1F28",
    surface: "#2A2A37",
    fg: "#DCD7BA",
    accent: "#7E9CD8",
    secondary: "#957FB8",
    border: "#54546D",
  },
  nord: {
    id: "nord",
    name: "Nord",
    bg: "#2E3440",
    surface: "#3B4252",
    fg: "#ECEFF4",
    accent: "#88C0D0",
    secondary: "#81A1C1",
    border: "#4C566A",
  },
  "rose-pine": {
    id: "rose-pine",
    name: "Rose Pine",
    bg: "#191724",
    surface: "#1F1D2E",
    fg: "#E0DEF4",
    accent: "#EBBCBA",
    secondary: "#C4A7E7",
    border: "#403D52",
  },
  everforest: {
    id: "everforest",
    name: "Everforest Dark",
    bg: "#2D353B",
    surface: "#343F44",
    fg: "#D3C6AA",
    accent: "#A7C080",
    secondary: "#7FBBB3",
    border: "#475258",
  },
  onedark: {
    id: "onedark",
    name: "OneDark Pro",
    bg: "#282C34",
    surface: "#21252B",
    fg: "#ABB2BF",
    accent: "#61AFEF",
    secondary: "#C678DD",
    border: "#3E4451",
  },
  dracula: {
    id: "dracula",
    name: "Dracula",
    bg: "#282A36",
    surface: "#44475A",
    fg: "#F8F8F2",
    accent: "#BD93F9",
    secondary: "#FF79C6",
    border: "#6272A4",
  },
  solarized: {
    id: "solarized",
    name: "Solarized Dark",
    bg: "#002B36",
    surface: "#073642",
    fg: "#839496",
    accent: "#268BD2",
    secondary: "#2AA198",
    border: "#586E75",
  },
  "matte-black": {
    id: "matte-black",
    name: "Matte Black",
    bg: "#0A0A0A",
    surface: "#141414",
    fg: "#E5E5E5",
    accent: "#FFFFFF",
    secondary: "#A3A3A3",
    border: "#262626",
  },
  ristretto: {
    id: "ristretto",
    name: "Monokai Ristretto",
    bg: "#2C2525",
    surface: "#403838",
    fg: "#FFF1F3",
    accent: "#FD6883",
    secondary: "#A8A9EB",
    border: "#5B5353",
  },
  "osaka-jade": {
    id: "osaka-jade",
    name: "Osaka Jade",
    bg: "#111C18",
    surface: "#1A2B24",
    fg: "#D1E5DD",
    accent: "#50C878",
    secondary: "#2E8B57",
    border: "#2D473E",
  },
  synthwave84: {
    id: "synthwave84",
    name: "SynthWave '84",
    bg: "#262335",
    surface: "#34294F",
    fg: "#FFFFFF",
    accent: "#FF7EDB",
    secondary: "#36F9F6",
    border: "#495495",
  },
  cyberpunk: {
    id: "cyberpunk",
    name: "Cyberpunk Neon",
    bg: "#0D0221",
    surface: "#19053B",
    fg: "#00FF9F",
    accent: "#FF0055",
    secondary: "#00F0FF",
    border: "#3A0CA3",
  },
  monokai: {
    id: "monokai",
    name: "Monokai Classic",
    bg: "#272822",
    surface: "#3E3D32",
    fg: "#F8F8F2",
    accent: "#A6E22E",
    secondary: "#F92672",
    border: "#75715E",
  },
  zenbones: {
    id: "zenbones",
    name: "Zenbones",
    bg: "#191919",
    surface: "#262626",
    fg: "#B4BDC3",
    accent: "#8BAE68",
    secondary: "#CF86C1",
    border: "#404040",
  },
  "ayu-dark": {
    id: "ayu-dark",
    name: "Ayu Dark",
    bg: "#0B0E14",
    surface: "#11151C",
    fg: "#BFBDB6",
    accent: "#E6B450",
    secondary: "#39BAE6",
    border: "#1F2430",
  },
  "ayu-mirage": {
    id: "ayu-mirage",
    name: "Ayu Mirage",
    bg: "#1F2430",
    surface: "#242B38",
    fg: "#CCCAC2",
    accent: "#FFCC66",
    secondary: "#5CCFE6",
    border: "#33415E",
  },
  nightfox: {
    id: "nightfox",
    name: "Nightfox",
    bg: "#192330",
    surface: "#212E3F",
    fg: "#CDCECF",
    accent: "#719CD6",
    secondary: "#9D79D6",
    border: "#39506D",
  },
  vesper: {
    id: "vesper",
    name: "Vesper",
    bg: "#101010",
    surface: "#1C1C1C",
    fg: "#FFFFFF",
    accent: "#FFC799",
    secondary: "#99FFE4",
    border: "#2E2E2E",
  },
  oxocarbon: {
    id: "oxocarbon",
    name: "IBM Oxocarbon",
    bg: "#161616",
    surface: "#262626",
    fg: "#F2F4F8",
    accent: "#78A9FF",
    secondary: "#BE95FF",
    border: "#393939",
  },
};

export const BYRON_LOVE_PALETTE = {
  sacredGold: "#E8C547",
  olympusWhite: "#FAF7F0",
  aphroditeRose: "#F28B9C",
  celestialWing: "#EDE6D6",
  aegeanAzure: "#2B5B84",
  zeusAmber: "#D4AF37",
  sanctuaryMarble: "#1C1A17",
  etherealPlum: "#8B5CF6",
  ambrosiaCoral: "#F43F5E",
  divineHalo: "#FEF08A",
} as const;

export const HYPRLAND_DECORATION_SPEC = {
  activeBorderStart: "#33CCFF",
  activeBorderEnd: "#00FF99",
  activeBorderAngleDeg: 45,
  inactiveBorder: "#595959AA",
  roundingPx: 10,
  borderSizePx: 2,
  gapsInPx: 5,
  gapsOutPx: 20,
  blurSize: 8,
  blurPasses: 2,
} as const;

export const YOLO_THEME_PRESETS: Record<NormalizedPresetId, YoloThemeTokens> = {
  neutral: {
    id: "neutral",
    brandId: "yolo-neutral",
    name: "Yolo Neutral Dark",
    backgroundPrimary: "#0A0E17",
    backgroundSecondary: "#121826",
    surfaceElevated: "#1A2234",
    textPrimary: "#F8FAFC",
    textMuted: "#94A3B8",
    accentPrimary: "#6366F1",
    accentSecondary: "#06B6D4",
    accentSuccess: "#10B981",
    accentWarning: "#F59E0B",
    accentDanger: "#EF4444",
    borderSubtle: "#1E293B",
    radiusPx: 8,
    borderWidthPx: 1,
    blurRadiusPx: 12,
    fontSans: "'Plus Jakarta Sans', 'Inter', system-ui, sans-serif",
    fontMono: "'JetBrains Mono', monospace",
  },
  m3: {
    id: "m3",
    brandId: "yolo-m3",
    name: "Material Design 3 Expressive",
    backgroundPrimary: "#141218",
    backgroundSecondary: "#1D1B20",
    surfaceElevated: "#2B2930",
    textPrimary: "#E6E0E9",
    textMuted: "#CAC4D0",
    accentPrimary: "#D0BCFF",
    accentSecondary: "#CCC2DC",
    accentSuccess: "#A8DAB5",
    accentWarning: "#FFD8E4",
    accentDanger: "#F2B8B5",
    borderSubtle: "#49454F",
    radiusPx: 16,
    borderWidthPx: 1,
    blurRadiusPx: 0,
    fontSans: "'Google Sans Flex', 'Roboto Flex', system-ui, sans-serif",
    fontMono: "'Google Sans Code', 'JetBrains Mono', monospace",
  },
  fluent: {
    id: "fluent",
    brandId: "yolo-fluent",
    name: "Microsoft Fluent 2 Mica Dark",
    backgroundPrimary: "#202020",
    backgroundSecondary: "#272727",
    surfaceElevated: "#323232",
    textPrimary: "#FFFFFF",
    textMuted: "#C5C5C5",
    accentPrimary: "#60CDFF",
    accentSecondary: "#0078D4",
    accentSuccess: "#6CCB5F",
    accentWarning: "#FCE100",
    accentDanger: "#FF99A4",
    borderSubtle: "#3D3D3D",
    radiusPx: 8,
    borderWidthPx: 1,
    blurRadiusPx: 24,
    fontSans: "'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif",
    fontMono: "'Cascadia Code', monospace",
  },
  apple: {
    id: "apple",
    brandId: "yolo-apple",
    name: "Apple HIG Vibrancy / Liquid Glass",
    backgroundPrimary: "#000000",
    backgroundSecondary: "#1C1C1E",
    surfaceElevated: "#2C2C2E",
    textPrimary: "#FFFFFF",
    textMuted: "#8E8E93",
    accentPrimary: "#0A84FF",
    accentSecondary: "#5E5CE6",
    accentSuccess: "#30D158",
    accentWarning: "#FFD60A",
    accentDanger: "#FF453A",
    borderSubtle: "#38383A",
    radiusPx: 12,
    borderWidthPx: 1,
    blurRadiusPx: 30,
    fontSans:
      "-apple-system, BlinkMacSystemFont, 'SF Pro Display', 'Inter', sans-serif",
    fontMono: "'SF Mono', 'JetBrains Mono', monospace",
  },
  omarchy: {
    id: "omarchy",
    brandId: "yolo-omarchy",
    name: "Omarchy Wayland TUI (Catppuccin Mocha Default)",
    backgroundPrimary: "#1E1E2E",
    backgroundSecondary: "#181825",
    surfaceElevated: "#313244",
    textPrimary: "#CDD6F4",
    textMuted: "#A6ADC8",
    accentPrimary: "#89B4FA",
    accentSecondary: "#CBA6F7",
    accentSuccess: "#A6E3A1",
    accentWarning: "#F9E2AF",
    accentDanger: "#F38BA8",
    borderSubtle: "#45475A",
    radiusPx: 0,
    borderWidthPx: 2,
    blurRadiusPx: 8,
    fontSans: "'JetBrains Mono', 'Inter', monospace",
    fontMono: "'JetBrains Mono', monospace",
    subtheme: "catppuccin",
  },
  hyprland: {
    id: "hyprland",
    brandId: "yolo-hyprland",
    name: "Hyprland Wayland Compositor",
    backgroundPrimary: "#11111B",
    backgroundSecondary: "#181825",
    surfaceElevated: "#1E1E2E",
    textPrimary: "#CDD6F4",
    textMuted: "#9399B2",
    accentPrimary: "#33CCFF",
    accentSecondary: "#00FF99",
    accentSuccess: "#00FF99",
    accentWarning: "#F9E2AF",
    accentDanger: "#F38BA8",
    borderSubtle: "#595959",
    radiusPx: 10,
    borderWidthPx: 2,
    blurRadiusPx: 16,
    fontSans: "'Inter', 'JetBrains Mono', sans-serif",
    fontMono: "'JetBrains Mono', monospace",
    gradientAngleDeg: 45,
  },
  aphrody: {
    id: "aphrody",
    brandId: "yolo-aphrody",
    name: "Aphrody Byron Love Sanctuary",
    backgroundPrimary: "#1C1A17",
    backgroundSecondary: "#26231E",
    surfaceElevated: "#332E27",
    textPrimary: "#FAF7F0",
    textMuted: "#EDE6D6",
    accentPrimary: "#E8C547",
    accentSecondary: "#F28B9C",
    accentSuccess: "#10B981",
    accentWarning: "#D4AF37",
    accentDanger: "#F43F5E",
    borderSubtle: "#4A4236",
    radiusPx: 12,
    borderWidthPx: 1,
    blurRadiusPx: 16,
    fontSans: "'Cinzel', 'Plus Jakarta Sans', serif",
    fontMono: "'JetBrains Mono', monospace",
  },
  chromium: {
    id: "chromium",
    brandId: "yolo-chromium",
    name: "Chromium Views + WebUI",
    backgroundPrimary: "#202124",
    backgroundSecondary: "#292a2d",
    surfaceElevated: "#35363a",
    textPrimary: "#e8eaed",
    textMuted: "#9aa0a6",
    accentPrimary: "#8ab4f8",
    accentSecondary: "#a8c7fa",
    accentSuccess: "#81c995",
    accentWarning: "#fdd663",
    accentDanger: "#f28b82",
    borderSubtle: "rgba(255, 255, 255, 0.10)",
    radiusPx: 8,
    borderWidthPx: 1,
    blurRadiusPx: 0,
    fontSans: "Arial, system-ui, sans-serif",
    fontMono: "'Roboto Mono', ui-monospace, monospace",
  },
};

/* @__PURE__ */
export function normalizePresetId(preset?: string): NormalizedPresetId {
  if (!preset) return "neutral";
  const cleaned = preset
    .toLowerCase()
    .trim()
    .replace(/^yolo-/, "");
  if (
    cleaned === "neutral" ||
    cleaned === "m3" ||
    cleaned === "fluent" ||
    cleaned === "apple" ||
    cleaned === "omarchy" ||
    cleaned === "hyprland" ||
    cleaned === "aphrody"
    || cleaned === "chromium"
  ) {
    return cleaned;
  }
  return "neutral";
}

/* @__PURE__ */
export function resolveThemeTokens(
  preset?: string,
  omarchySubtheme?: string,
): YoloThemeTokens {
  const id = normalizePresetId(preset);
  const base = { ...YOLO_THEME_PRESETS[id] };
  if (id === "omarchy" && omarchySubtheme) {
    const key = omarchySubtheme.toLowerCase().trim();
    const sub = OMARCHY_SUBTHEMES[key];
    if (sub) {
      base.name = `Omarchy (${sub.name})`;
      base.backgroundPrimary = sub.bg;
      base.backgroundSecondary = sub.bg;
      base.surfaceElevated = sub.surface;
      base.textPrimary = sub.fg;
      base.accentPrimary = sub.accent;
      base.accentSecondary = sub.secondary;
      base.borderSubtle = sub.border;
      base.subtheme = sub.id;
    }
  }
  return base;
}

export interface EmitThemeOptions {
  preset?: string;
  omarchySubtheme?: string;
  target?: EmitTarget;
}

/* @__PURE__ */
export function emitThemeCode(options: EmitThemeOptions = {}): string {
  const tokens = resolveThemeTokens(options.preset, options.omarchySubtheme);
  const target: EmitTarget = options.target ?? "css";

  switch (target) {
    case "css":
    case "fluent":
    case "apple":
      return [
        `/* Generated by @yolo-rs/ui-plugin — Preset: ${tokens.name} (${tokens.brandId}) */`,
        `:root, [data-theme="${tokens.id}"], [data-theme="${tokens.brandId}"] {`,
        `  color-scheme: dark;`,
        `  --yolo-bg-primary: ${tokens.backgroundPrimary};`,
        `  --yolo-bg-secondary: ${tokens.backgroundSecondary};`,
        `  --yolo-surface-elevated: ${tokens.surfaceElevated};`,
        `  --yolo-text-primary: ${tokens.textPrimary};`,
        `  --yolo-text-muted: ${tokens.textMuted};`,
        `  --yolo-accent-primary: ${tokens.accentPrimary};`,
        `  --yolo-accent-secondary: ${tokens.accentSecondary};`,
        `  --yolo-accent-success: ${tokens.accentSuccess};`,
        `  --yolo-accent-warning: ${tokens.accentWarning};`,
        `  --yolo-accent-danger: ${tokens.accentDanger};`,
        `  --yolo-border-subtle: ${tokens.borderSubtle};`,
        `  --yolo-radius: ${tokens.radiusPx}px;`,
        `  --yolo-border-width: ${tokens.borderWidthPx}px;`,
        `  --yolo-blur-radius: ${tokens.blurRadiusPx}px;`,
        `  --yolo-font-sans: ${tokens.fontSans};`,
        `  --yolo-font-mono: ${tokens.fontMono};`,
        tokens.id === "aphrody"
          ? [
              `  --byron-sacred-gold: ${BYRON_LOVE_PALETTE.sacredGold};`,
              `  --byron-olympus-white: ${BYRON_LOVE_PALETTE.olympusWhite};`,
              `  --byron-aphrodite-rose: ${BYRON_LOVE_PALETTE.aphroditeRose};`,
              `  --byron-aegean-azure: ${BYRON_LOVE_PALETTE.aegeanAzure};`,
            ].join("\n")
          : `  --yolo-gradient-angle: ${tokens.gradientAngleDeg ?? 135}deg;`,
        `}`,
      ].join("\n");

    case "gtk":
      return [
        `/* GTK4 / libadwaita CSS — Preset: ${tokens.name} */`,
        `@define-color window_bg_color ${tokens.backgroundPrimary};`,
        `@define-color window_fg_color ${tokens.textPrimary};`,
        `@define-color view_bg_color ${tokens.backgroundSecondary};`,
        `@define-color view_fg_color ${tokens.textPrimary};`,
        `@define-color headerbar_bg_color ${tokens.surfaceElevated};`,
        `@define-color headerbar_fg_color ${tokens.textPrimary};`,
        `@define-color card_bg_color ${tokens.surfaceElevated};`,
        `@define-color accent_bg_color ${tokens.accentPrimary};`,
        `@define-color accent_fg_color ${tokens.backgroundPrimary};`,
        `@define-color accent_color ${tokens.accentPrimary};`,
        `@define-color destructive_bg_color ${tokens.accentDanger};`,
        `@define-color success_color ${tokens.accentSuccess};`,
        `@define-color warning_color ${tokens.accentWarning};`,
      ].join("\n");

    case "kde":
      return [
        `# KDE Plasma .colors — Preset: ${tokens.name}`,
        `[General]`,
        `ColorScheme=Yolo${tokens.id.charAt(0).toUpperCase() + tokens.id.slice(1)}`,
        `Name=${tokens.name}`,
        ``,
        `[Colors:Window]`,
        `BackgroundNormal=${tokens.backgroundPrimary}`,
        `BackgroundAlternate=${tokens.backgroundSecondary}`,
        `ForegroundNormal=${tokens.textPrimary}`,
        `ForegroundInactive=${tokens.textMuted}`,
        `DecorationFocus=${tokens.accentPrimary}`,
        `DecorationHover=${tokens.accentSecondary}`,
      ].join("\n");

    case "cosmic":
      return [
        `// System76 COSMIC Theme RON — Preset: ${tokens.name}`,
        `CosmicTheme(`,
        `  name: "${tokens.name}",`,
        `  is_dark: true,`,
        `  background: "${tokens.backgroundPrimary}",`,
        `  primary_container: "${tokens.surfaceElevated}",`,
        `  secondary_container: "${tokens.backgroundSecondary}",`,
        `  text_primary: "${tokens.textPrimary}",`,
        `  accent: "${tokens.accentPrimary}",`,
        `  corner_radii: (${tokens.radiusPx}, ${tokens.radiusPx}, ${tokens.radiusPx}, ${tokens.radiusPx}),`,
        `)`,
      ].join("\n");

    case "qml":
      return [
        `// Qt6 / Kirigami / cxx-qt QML Theme Singleton — Preset: ${tokens.name}`,
        `pragma Singleton`,
        `import QtQuick`,
        ``,
        `QtObject {`,
        `    readonly property string presetId: "${tokens.id}"`,
        `    readonly property color backgroundPrimary: "${tokens.backgroundPrimary}"`,
        `    readonly property color backgroundSecondary: "${tokens.backgroundSecondary}"`,
        `    readonly property color surfaceElevated: "${tokens.surfaceElevated}"`,
        `    readonly property color textPrimary: "${tokens.textPrimary}"`,
        `    readonly property color textMuted: "${tokens.textMuted}"`,
        `    readonly property color accentPrimary: "${tokens.accentPrimary}"`,
        `    readonly property color accentSecondary: "${tokens.accentSecondary}"`,
        `    readonly property int radiusPx: ${tokens.radiusPx}`,
        `    readonly property int borderWidthPx: ${tokens.borderWidthPx}`,
        `}`,
      ].join("\n");

    case "winui":
      return [
        `<!-- WinUI 3 / Fluent 2 ResourceDictionary — Preset: ${tokens.name} -->`,
        `<ResourceDictionary xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation">`,
        `  <Color x:Key="ApplicationPageBackgroundThemeBrushColor">${tokens.backgroundPrimary}</Color>`,
        `  <Color x:Key="CardBackgroundFillColorDefault">${tokens.surfaceElevated}</Color>`,
        `  <Color x:Key="TextFillColorPrimary">${tokens.textPrimary}</Color>`,
        `  <Color x:Key="TextFillColorSecondary">${tokens.textMuted}</Color>`,
        `  <Color x:Key="SystemAccentColor">${tokens.accentPrimary}</Color>`,
        `  <CornerRadius x:Key="ControlCornerRadius">${tokens.radiusPx}</CornerRadius>`,
        `</ResourceDictionary>`,
      ].join("\n");

    case "omarchy":
      return [
        `# Omarchy Unified Desktop Bundle (${tokens.subtheme ?? "catppuccin"})`,
        `# 1. hyprland.conf`,
        `general {`,
        `    col.active_border = rgb(${tokens.accentPrimary.slice(1)})`,
        `    col.inactive_border = rgb(${tokens.borderSubtle.slice(1)})`,
        `    border_size = ${tokens.borderWidthPx}`,
        `}`,
        `decoration {`,
        `    rounding = ${tokens.radiusPx}`,
        `}`,
        `# 2. waybar.css`,
        `@define-color bg ${tokens.backgroundPrimary};`,
        `@define-color fg ${tokens.textPrimary};`,
        `@define-color accent ${tokens.accentPrimary};`,
        `# 3. ghostty / alacritty`,
        `background = ${tokens.backgroundPrimary}`,
        `foreground = ${tokens.textPrimary}`,
      ].join("\n");

    case "hyprland":
      return [
        `# Hyprland Compositor Decoration Config — Preset: ${tokens.name}`,
        `general {`,
        `    gaps_in = ${HYPRLAND_DECORATION_SPEC.gapsInPx}`,
        `    gaps_out = ${HYPRLAND_DECORATION_SPEC.gapsOutPx}`,
        `    border_size = ${HYPRLAND_DECORATION_SPEC.borderSizePx}`,
        `    col.active_border = rgba(${HYPRLAND_DECORATION_SPEC.activeBorderStart.slice(1)}ee) rgba(${HYPRLAND_DECORATION_SPEC.activeBorderEnd.slice(1)}ee) ${HYPRLAND_DECORATION_SPEC.activeBorderAngleDeg}deg`,
        `    col.inactive_border = rgba(595959aa)`,
        `    layout = dwindle`,
        `}`,
        `decoration {`,
        `    rounding = ${HYPRLAND_DECORATION_SPEC.roundingPx}`,
        `    blur {`,
        `        enabled = true`,
        `        size = ${HYPRLAND_DECORATION_SPEC.blurSize}`,
        `        passes = ${HYPRLAND_DECORATION_SPEC.blurPasses}`,
        `    }`,
        `}`,
      ].join("\n");

    case "dtcg":
      return JSON.stringify(
        {
          $schema: "https://tr.designtokens.org/format/",
          preset: tokens.brandId,
          name: tokens.name,
          color: {
            background: {
              primary: { $value: tokens.backgroundPrimary, $type: "color" },
              secondary: { $value: tokens.backgroundSecondary, $type: "color" },
              elevated: { $value: tokens.surfaceElevated, $type: "color" },
            },
            text: {
              primary: { $value: tokens.textPrimary, $type: "color" },
              muted: { $value: tokens.textMuted, $type: "color" },
            },
            accent: {
              primary: { $value: tokens.accentPrimary, $type: "color" },
              secondary: { $value: tokens.accentSecondary, $type: "color" },
              success: { $value: tokens.accentSuccess, $type: "color" },
              warning: { $value: tokens.accentWarning, $type: "color" },
              danger: { $value: tokens.accentDanger, $type: "color" },
            },
            border: {
              subtle: { $value: tokens.borderSubtle, $type: "color" },
            },
          },
          dimension: {
            radius: { $value: `${tokens.radiusPx}px`, $type: "dimension" },
            borderWidth: {
              $value: `${tokens.borderWidthPx}px`,
              $type: "dimension",
            },
            blurRadius: {
              $value: `${tokens.blurRadiusPx}px`,
              $type: "dimension",
            },
          },
        },
        null,
        2,
      );
  }
}

export interface MigratedMetadata {
  componentName: string;
  tailwindClasses: string[];
  detectedPrimitives: string[];
  preset: NormalizedPresetId;
}

export interface MigrateResult {
  target: MigrateTarget;
  preset: NormalizedPresetId;
  code: string;
  metadata: MigratedMetadata;
}

/* @__PURE__ */
export function extractTsxMetadata(
  source: string,
  preset?: string,
): MigratedMetadata {
  const fnMatch =
    source.match(/export\s+(?:default\s+)?function\s+([A-Za-z0-9_]+)/) ??
    source.match(/const\s+([A-Za-z0-9_]+)\s*=/) ??
    source.match(/function\s+([A-Za-z0-9_]+)/);
  const componentName = fnMatch?.[1] ?? "MigratedView";

  const classMatches = [
    ...source.matchAll(/class(?:Name)?=["'`]([^"'`]+)["'`]/g),
  ];
  const tailwindClasses = Array.from(
    new Set(
      classMatches
        .flatMap((m) => m[1]?.split(/\s+/) ?? [])
        .map((c) => c.trim())
        .filter(Boolean),
    ),
  );

  const primitivePatterns = [
    "Dialog",
    "Popover",
    "Tooltip",
    "Tabs",
    "Switch",
    "Slider",
    "Select",
    "Accordion",
    "DropdownMenu",
    "Card",
    "Button",
    "Input",
    "Badge",
  ];
  const detectedPrimitives = primitivePatterns.filter((p) =>
    new RegExp(`\\b${p}\\b`).test(source),
  );
  if (detectedPrimitives.length === 0) {
    detectedPrimitives.push("Container", "Button");
  }

  return {
    componentName,
    tailwindClasses,
    detectedPrimitives,
    preset: normalizePresetId(preset),
  };
}

/* @__PURE__ */
export function migrateTsxToRust(
  source: string,
  target: MigrateTarget = "dioxus",
  preset?: string,
): MigrateResult {
  const meta = extractTsxMetadata(source, preset);
  const tokens = resolveThemeTokens(meta.preset);
  const snakeName = meta.componentName
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .toLowerCase();

  let code = "";
  switch (target) {
    case "dioxus":
      code = [
        `// Migrated by @yolo-rs/ui-plugin (yolo-migrator) -> Dioxus 0.8 RSX`,
        `// Preset: ${tokens.name} | Primitives: ${meta.detectedPrimitives.join(", ")}`,
        `use dioxus::prelude::*;`,
        `use yolo_shared::{YoloStyle, YoloThemePreset};`,
        ``,
        `#[component]`,
        `pub fn ${meta.componentName}() -> Element {`,
        `    let theme = YoloThemePreset::${tokens.id.charAt(0).toUpperCase() + tokens.id.slice(1)}.tokens();`,
        `    let style = YoloStyle::from_tailwind("${meta.tailwindClasses.join(" ")}");`,
        `    rsx! {`,
        `        div {`,
        `            class: "${meta.tailwindClasses.join(" ")}",`,
        `            style: "background: {theme.background_primary.to_css_hex()}; color: {theme.text_primary.to_css_hex()}; border-radius: {style.border_radius}px;",`,
        `            h2 { "${meta.componentName}" }`,
        `            button {`,
        `                style: "background: {theme.accent_primary.to_css_hex()};",`,
        `                "Action"`,
        `            }`,
        `        }`,
        `    }`,
        `}`,
      ].join("\n");
      break;

    case "wgpu":
      code = [
        `// Migrated by @yolo-rs/ui-plugin (yolo-migrator) -> WGPU / Taffy Layout Quad`,
        `// Preset: ${tokens.name} | Classes: ${meta.tailwindClasses.join(" ")}`,
        `use yolo_layout::YoloLayoutTree;`,
        `use yolo_shared::{YoloStyle, YoloThemePreset};`,
        ``,
        `pub fn build_${snakeName}_wgpu_scene(tree: &mut YoloLayoutTree) {`,
        `    let _theme = YoloThemePreset::${tokens.id.charAt(0).toUpperCase() + tokens.id.slice(1)}.tokens();`,
        `    let _style = YoloStyle::from_tailwind("${meta.tailwindClasses.join(" ")}");`,
        `    let _root = tree.new_leaf();`,
        `}`,
      ].join("\n");
      break;

    case "egui":
      code = [
        `// Migrated by @yolo-rs/ui-plugin (yolo-migrator) -> egui Immediate-Mode UI`,
        `// Preset: ${tokens.name} | Primitives: ${meta.detectedPrimitives.join(", ")}`,
        `use yolo_shared::{YoloStyle, YoloThemePreset};`,
        ``,
        `pub fn ${snakeName}_ui(ui: &mut egui::Ui) {`,
        `    let _theme = YoloThemePreset::${tokens.id.charAt(0).toUpperCase() + tokens.id.slice(1)}.tokens();`,
        `    let _style = YoloStyle::from_tailwind("${meta.tailwindClasses.join(" ")}");`,
        `    egui::Frame::group(ui.style()).show(ui, |ui| {`,
        `        ui.heading("${meta.componentName}");`,
        `        let _ = ui.button("Action");`,
        `    });`,
        `}`,
      ].join("\n");
      break;

    case "cosmic":
      code = [
        `// Migrated by @yolo-rs/ui-plugin (yolo-migrator) -> System76 libcosmic MVU`,
        `// Preset: ${tokens.name} | Primitives: ${meta.detectedPrimitives.join(", ")}`,
        `use yolo_shared::YoloStyle;`,
        ``,
        `pub fn ${snakeName}_view() -> &'static str {`,
        `    let _style = YoloStyle::from_tailwind("${meta.tailwindClasses.join(" ")}");`,
        `    "cosmic::widget::container(${meta.componentName})"`,
        `}`,
      ].join("\n");
      break;

    case "gtk":
      code = [
        `// Migrated by @yolo-rs/ui-plugin (yolo-migrator) -> GTK4 / libadwaita`,
        `// Preset: ${tokens.name} | Primitives: ${meta.detectedPrimitives.join(", ")}`,
        `pub fn ${snakeName}_gtk_xml() -> &'static str {`,
        `    r#"<interface><object class="AdwClamp"><child><object class="GtkBox"><property name="orientation">vertical</property></object></child></object></interface>"#`,
        `}`,
      ].join("\n");
      break;

    case "qml":
      code = [
        `// Migrated by @yolo-rs/ui-plugin (yolo-migrator) -> Qt6 QML / cxx-qt`,
        `// Preset: ${tokens.name} | Component: ${meta.componentName}`,
        `import QtQuick`,
        `import QtQuick.Controls`,
        `import QtQuick.Layouts`,
        ``,
        `Rectangle {`,
        `    color: "${tokens.backgroundPrimary}"`,
        `    radius: ${tokens.radiusPx}`,
        `    ColumnLayout {`,
        `        Text { text: "${meta.componentName}"; color: "${tokens.textPrimary}" }`,
        `        Button { text: "Action" }`,
        `    }`,
        `}`,
      ].join("\n");
      break;
  }

  return {
    target,
    preset: meta.preset,
    code,
    metadata: meta,
  };
}

export interface ContrastReport {
  foreground: string;
  background: string;
  ratio: number;
  aaNormal: boolean;
  aaLarge: boolean;
  aaaNormal: boolean;
  aaaLarge: boolean;
  apcaLc: number;
  verdict: "AAA" | "AA" | "AA-Large" | "Fail";
}

function parseHexToRgb(hex: string): [number, number, number] {
  const clean = hex.trim().replace(/^#/, "");
  if (clean.length === 3) {
    const r = parseInt(clean[0]! + clean[0]!, 16);
    const g = parseInt(clean[1]! + clean[1]!, 16);
    const b = parseInt(clean[2]! + clean[2]!, 16);
    return [r, g, b];
  }
  const r = parseInt(clean.slice(0, 2) || "00", 16);
  const g = parseInt(clean.slice(2, 4) || "00", 16);
  const b = parseInt(clean.slice(4, 6) || "00", 16);
  return [
    Number.isNaN(r) ? 0 : r,
    Number.isNaN(g) ? 0 : g,
    Number.isNaN(b) ? 0 : b,
  ];
}

function relativeLuminance([r, g, b]: [number, number, number]): number {
  const toLinear = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b);
}

/* @__PURE__ */
export function computeContrast(fgHex: string, bgHex: string): ContrastReport {
  const fgRgb = parseHexToRgb(fgHex);
  const bgRgb = parseHexToRgb(bgHex);
  const l1 = relativeLuminance(fgRgb);
  const l2 = relativeLuminance(bgRgb);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  const ratio = Number(((lighter + 0.05) / (darker + 0.05)).toFixed(2));

  // Simplified APCA Lc estimate
  const yTxt = l1;
  const yBg = l2;
  const rawLc =
    yBg > yTxt
      ? (Math.pow(yBg, 0.56) - Math.pow(yTxt, 0.57)) * 1.14 * 100
      : (Math.pow(yBg, 0.65) - Math.pow(yTxt, 0.62)) * 1.14 * 100;
  const apcaLc = Number(rawLc.toFixed(1));

  const aaNormal = ratio >= 4.5;
  const aaLarge = ratio >= 3.0;
  const aaaNormal = ratio >= 7.0;
  const aaaLarge = ratio >= 4.5;

  const verdict: ContrastReport["verdict"] = aaaNormal
    ? "AAA"
    : aaNormal
      ? "AA"
      : aaLarge
        ? "AA-Large"
        : "Fail";

  return {
    foreground: fgHex.startsWith("#")
      ? fgHex.toUpperCase()
      : `#${fgHex.toUpperCase()}`,
    background: bgHex.startsWith("#")
      ? bgHex.toUpperCase()
      : `#${bgHex.toUpperCase()}`,
    ratio,
    aaNormal,
    aaLarge,
    aaaNormal,
    aaaLarge,
    apcaLc,
    verdict,
  };
}

export interface DevtoolsDescriptor {
  id: DevtoolsEngineId;
  name: string;
  ecosystem: string;
  crateBridge: string;
  submodulePath: string;
  launchCommand: string;
  envVars: Record<string, string>;
  features: string[];
  awesomeRepo?: string;
  stackCsv: string;
}

export const DEVTOOLS_REGISTRY: Record<DevtoolsEngineId, DevtoolsDescriptor> = {
  dioxus: {
    id: "dioxus",
    name: "Dioxus 0.8 Subsecond Hot-Patch & Devtools",
    ecosystem: "DioxusLabs",
    crateBridge: "packages/dioxus",
    submodulePath: "submodules/subsecond",
    launchCommand: "dx serve --hot-reload --subsecond",
    envVars: { DIOXUS_LOG: "debug", SUBSECOND_HOTPATCH: "1" },
    features: [
      "RSX Subsecond binary hot-patching",
      "Signal dependency graph inspector",
      "Blitz WebGPU/WGPU layout overlay",
      "Fullstack SSR hydration diagnostics",
    ],
    awesomeRepo: "submodules/awesome-dioxus",
    stackCsv: "skills/yolo-ui-pro-max/data/stacks/dioxus.csv",
  },
  gpui: {
    id: "gpui",
    name: "Zed GPUI Inspector & Longbridge GPUI-Component Catalog",
    ecosystem: "Zed Industries + Longbridge",
    crateBridge: "packages/yolo-gpui",
    submodulePath: "submodules/gpui-component",
    launchCommand: "cargo run -p yolo-showcase -- --engine gpui --devtools",
    envVars: { GPUI_DEBUG_LAYOUT: "1", WGPU_BACKEND: "primary" },
    features: [
      "60+ GPU-accelerated Longbridge widgets gallery",
      "DockArea & VirtualList frame budget profiler (120fps)",
      "WGPU / Blade shader & glyph atlas inspector",
    ],
    awesomeRepo: "submodules/awesome-gpui",
    stackCsv: "skills/yolo-ui-pro-max/data/stacks/gpui.csv",
  },
  egui: {
    id: "egui",
    name: "egui Puffin Profiler & Kittest Visual Harness",
    ecosystem: "emilk / egui",
    crateBridge: "packages/yolo-egui",
    submodulePath: "submodules/egui",
    launchCommand: "cargo test -p yolo-egui -- --nocapture",
    envVars: { EGUI_PUFFIN_PROFILER: "1" },
    features: [
      "puffin_egui frame & scope flamegraph profiler",
      "egui_kittest headless AccessKit + WGPU pixel snapshot testing",
      "Immediate-mode style & tessellation bounds overlay",
    ],
    awesomeRepo: "submodules/awesome-egui",
    stackCsv: "skills/yolo-ui-pro-max/data/stacks/egui.csv",
  },
  iced: {
    id: "iced",
    name: "Iced Comet Time-Travel MVU Debugger",
    ecosystem: "Iced-rs",
    crateBridge: "packages/yolo-iced",
    submodulePath: "submodules/comet",
    launchCommand: "cargo run -p yolo-showcase -- --engine iced --comet",
    envVars: { ICED_PRESENT_MODE: "auto_vsync", COMET_PORT: "9421" },
    features: [
      "Comet MVU Message history & time-travel state replay",
      "Iced_AW & Iced_Audio DSP widget inspector",
      "Cryoglyph WGPU text atlas telemetry",
    ],
    awesomeRepo: "submodules/awesome-iced",
    stackCsv: "skills/yolo-ui-pro-max/data/stacks/iced-libcosmic.csv",
  },
  cosmic: {
    id: "cosmic",
    name: "System76 libcosmic Theme & Applet Inspector",
    ecosystem: "System76 COSMIC",
    crateBridge: "packages/yolo-cosmic",
    submodulePath: "submodules/libcosmic",
    launchCommand: "cargo run -p yolo-emit -- --preset omarchy --target cosmic",
    envVars: { COSMIC_THEME_DEBUG: "1" },
    features: [
      "cosmic-theme RON live builder & dark/light token verifier",
      "Adaptive nav_bar & context_drawer layout inspector",
      "Wayland layer-shell applet preview",
    ],
    awesomeRepo: "submodules/awesome-iced",
    stackCsv: "skills/yolo-ui-pro-max/data/stacks/iced-libcosmic.csv",
  },
  bevy: {
    id: "bevy",
    name: "Bevy ECS UI & bevy-inspector-egui",
    ecosystem: "Bevy Engine",
    crateBridge: "packages/yolo-bevy",
    submodulePath: "submodules/bevy",
    launchCommand: "cargo run -p yolo-showcase -- --engine bevy --inspector",
    envVars: { BEVY_ASSET_WATCH: "1" },
    features: [
      "bevy-inspector-egui live ECS World & Query entity inspector",
      "Taffy 0.15 flexbox/grid node bounding box overlay",
      "Reflect resource & state transitions debugger",
    ],
    awesomeRepo: "submodules/awesome-bevy",
    stackCsv: "skills/yolo-ui-pro-max/data/stacks/bevy.csv",
  },
  tauri: {
    id: "tauri",
    name: "Tauri v3 Alpha + CrabNebula DevTools & Window Vibrancy Inspector",
    ecosystem: "Tauri-Apps v3",
    crateBridge: "packages/yolo-tauri",
    submodulePath: "submodules/tauri",
    launchCommand: "cargo run -p yolo-showcase -- --engine tauri --devtools",
    envVars: {
      TAURI_DEVTOOLS: "1",
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--auto-open-devtools-for-tabs",
    },
    features: [
      "CrabNebula DevTools IPC command & event waterfall tracer",
      "Tauri v3 ACL permissions & capability auditor",
      "Wry 0.57 / Tao 0.37 / CEF / Window-Vibrancy (Mica, Acrylic, Liquid Glass) switcher",
    ],
    awesomeRepo: "submodules/awesome-tauri",
    stackCsv: "skills/yolo-ui-pro-max/data/stacks/tauri-v3.csv",
  },
  gtk4: {
    id: "gtk4",
    name: "GTK 4.16 Interactive Inspector & libadwaita 1.6 Debugger",
    ecosystem: "GNOME / gtk-rs",
    crateBridge: "packages/yolo-gtk4",
    submodulePath: "submodules/gtk4-rs",
    launchCommand:
      "GTK_DEBUG=interactive cargo run -p yolo-emit -- --preset hyprland --target gtk",
    envVars: { GTK_DEBUG: "interactive", ADW_DISABLE_PORTAL: "0" },
    features: [
      "GTK_DEBUG=interactive live CSS @define-color & node tree inspector",
      "libadwaita AdwBreakpoint & adaptive sheet previewer",
      "Relm4 & GtkBuilder XML template validator",
    ],
    stackCsv: "skills/yolo-ui-pro-max/data/stacks/gtk4-rs.csv",
  },
  qt6: {
    id: "qt6",
    name: "KDAB cxx-qt QML Profiler & Live Preview",
    ecosystem: "KDAB / Qt6 / KDE Kirigami",
    crateBridge: "packages/yolo-qt",
    submodulePath: "submodules/cxx-qt",
    launchCommand: "cargo run -p yolo-emit -- --preset aphrody --target qml",
    envVars: { QML_IMPORT_TRACE: "1", QT_QUICK_CONTROLS_STYLE: "Basic" },
    features: [
      "QML Profiler & Live Preview bridge for cxx-qt #[cxx_qt::bridge] QObjects",
      "QtQuick Theme Singleton & KDE Plasma .colors live reloader",
      "RHI (Vulkan/D3D12/Metal) scenegraph batching analyzer",
    ],
    stackCsv: "skills/yolo-ui-pro-max/data/stacks/cxx-qt.csv",
  },
  "shadcn-tailwind": {
    id: "shadcn-tailwind",
    name: "Shadcn UI + Base UI + Tailwind v4 Inspector & Rust Migrator",
    ecosystem: "Shadcn + MUI Base UI + Tailwind Labs",
    crateBridge: "crates/yolo-migrator",
    submodulePath: "submodules/shadcn-ui",
    launchCommand: "bun scripts/ui.ts migrate --help",
    envVars: { YOLO_THEME_PRESET: "neutral" },
    features: [
      "Tailwind v4 @theme inline & W3C DTCG token auditor",
      "Shadcn / Radix / Base UI TSX -> Rust (Dioxus, WGPU, egui, COSMIC, GTK4, QML) transpiler",
      "WCAG 2.2 AA/AAA & APCA contrast verifier",
    ],
    awesomeRepo: "submodules/awesome-shadcn-ui",
    stackCsv: "skills/yolo-ui-pro-max/data/stacks/base-ui.csv",
  },
};

export interface YoloUiPluginOptions {
  preset?: YoloPresetId;
  omarchySubtheme?: string;
  defaultRustTarget?: MigrateTarget;
}

/* @__PURE__ */
export function yoloUiBunPlugin(options: YoloUiPluginOptions = {}): BunPlugin {
  const preset = normalizePresetId(options.preset);
  const omarchySubtheme = options.omarchySubtheme;
  const defaultRustTarget: MigrateTarget =
    options.defaultRustTarget ?? "dioxus";

  return {
    name: "yolo-ui-bun-plugin",
    setup(build) {
      // 1. Virtual module resolution: `virtual:yolo-theme`, `yolo-ui/theme.css`, `virtual:yolo-devtools`
      build.onResolve(
        {
          filter:
            /^(virtual:yolo-theme|yolo-ui\/theme\.css|virtual:yolo-devtools)$/,
        },
        (args) => {
          return {
            path: args.path,
            namespace: "yolo-ui-virtual",
          };
        },
      );

      build.onLoad({ filter: /.*/, namespace: "yolo-ui-virtual" }, (args) => {
        if (args.path === "virtual:yolo-devtools") {
          return {
            loader: "js",
            contents: `export const DEVTOOLS_REGISTRY = ${JSON.stringify(DEVTOOLS_REGISTRY, null, 2)};\nexport default DEVTOOLS_REGISTRY;`,
          };
        }

        const tokens = resolveThemeTokens(preset, omarchySubtheme);
        const css = emitThemeCode({ preset, omarchySubtheme, target: "css" });

        if (args.path === "yolo-ui/theme.css") {
          return {
            loader: "css",
            contents: css,
          };
        }

        return {
          loader: "js",
          contents: [
            `export const themeTokens = ${JSON.stringify(tokens, null, 2)};`,
            `export const themeCss = ${JSON.stringify(css)};`,
            `export default themeTokens;`,
          ].join("\n"),
        };
      });

      // 2. `.tsx?rust` or `.jsx?rust` loader -> migrates React/Shadcn/Base UI/Tailwind to all 6 Rust targets
      build.onResolve({ filter: /\.[tj]sx\?rust(?:=[a-z0-9]+)?$/ }, (args) => {
        const [rawPath, query] = args.path.split("?");
        const resolvedPath = Bun.resolveSync(
          rawPath!,
          args.importer
            ? args.importer.replace(/[\\/][^\\/]+$/, "")
            : process.cwd(),
        );
        return {
          path: `${resolvedPath}?${query ?? "rust"}`,
          namespace: "yolo-tsx-rust",
        };
      });

      build.onLoad({ filter: /.*/, namespace: "yolo-tsx-rust" }, (args) => {
        const [filePath, query = "rust"] = args.path.split("?");
        const targetMatch = query.match(/rust=([a-z0-9]+)/);
        const requestedTarget =
          (targetMatch?.[1] as MigrateTarget | undefined) ?? defaultRustTarget;
        const source = readFileSync(filePath!, "utf8");

        const primary = migrateTsxToRust(source, requestedTarget, preset);
        const allTargets: Record<MigrateTarget, string> = {
          dioxus: migrateTsxToRust(source, "dioxus", preset).code,
          wgpu: migrateTsxToRust(source, "wgpu", preset).code,
          egui: migrateTsxToRust(source, "egui", preset).code,
          cosmic: migrateTsxToRust(source, "cosmic", preset).code,
          gtk: migrateTsxToRust(source, "gtk", preset).code,
          qml: migrateTsxToRust(source, "qml", preset).code,
        };

        return {
          loader: "js",
          contents: [
            `export const rustCode = ${JSON.stringify(primary.code)};`,
            `export const metadata = ${JSON.stringify(primary.metadata, null, 2)};`,
            `export const targets = ${JSON.stringify(allTargets, null, 2)};`,
            `export default rustCode;`,
          ].join("\n"),
        };
      });
    },
  };
}

export default yoloUiBunPlugin;
