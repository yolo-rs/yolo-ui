// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/react/core — Universal 7-Preset Theme Engine, Context Contract & React Web + React Native Token Bridge
// Unifies @aphrody/m3-theme, @aphrody/m3-tokens, and crates/yolo-shared + crates/yolo-tokens.

export type YoloPresetId =
  "neutral" | "m3" | "fluent" | "apple" | "omarchy" | "hyprland" | "aphrody";

export type CanonicalYoloPresetId =
  | "yolo-neutral"
  | "yolo-m3"
  | "yolo-fluent"
  | "yolo-apple"
  | "yolo-omarchy"
  | "yolo-hyprland"
  | "yolo-aphrody";

export type OmarchySubThemeId =
  | "tokyo-night"
  | "catppuccin"
  | "catppuccin-latte"
  | "ethereal"
  | "everforest"
  | "flexoki-light"
  | "gruvbox"
  | "hackerman"
  | "osaka-jade"
  | "kanagawa"
  | "last-horizon"
  | "lumon"
  | "lupine"
  | "matte-black"
  | "miasma"
  | "nord"
  | "retro-82"
  | "ristretto"
  | "rose-pine"
  | "solitude"
  | "starship"
  | "vantablack"
  | "white";

export const OMARCHY_SUBTHEME_IDS: readonly OmarchySubThemeId[] = [
  "tokyo-night",
  "catppuccin",
  "catppuccin-latte",
  "ethereal",
  "everforest",
  "flexoki-light",
  "gruvbox",
  "hackerman",
  "osaka-jade",
  "kanagawa",
  "last-horizon",
  "lumon",
  "lupine",
  "matte-black",
  "miasma",
  "nord",
  "retro-82",
  "ristretto",
  "rose-pine",
  "solitude",
  "starship",
  "vantablack",
  "white",
] as const;

export interface YoloThemeTokens {
  readonly name: string;
  readonly bgPrimary: string;
  readonly bgSurface: string;
  readonly bgElevated: string;
  readonly textPrimary: string;
  readonly textSecondary: string;
  readonly accentPrimary: string;
  readonly accentSecondary: string;
  readonly borderSubtle: string;
}

export interface YoloStructuralStyle {
  readonly radiusSmPx: number;
  readonly radiusMdPx: number;
  readonly radiusLgPx: number;
  readonly radiusPillPx: number;
  readonly spacingGridPx: number;
  readonly fontFamilyUi: string;
  readonly fontFamilyMono: string;
  readonly backdropBlurPx: number;
  readonly elevationLevel: number;
  readonly borderWidthPx: number;
  readonly windowVibrancy: string;
}

export interface YoloReactNativeStyleSheet {
  readonly container: {
    readonly flex: number;
    readonly backgroundColor: string;
  };
  readonly surface: {
    readonly backgroundColor: string;
    readonly borderRadius: number;
    readonly borderWidth: number;
    readonly borderColor: string;
    readonly padding: number;
  };
  readonly elevatedCard: {
    readonly backgroundColor: string;
    readonly borderRadius: number;
    readonly borderWidth: number;
    readonly borderColor: string;
    readonly padding: number;
  };
  readonly textPrimary: {
    readonly color: string;
    readonly fontFamily: string;
    readonly fontSize: number;
  };
  readonly textSecondary: {
    readonly color: string;
    readonly fontFamily: string;
    readonly fontSize: number;
  };
  readonly accentButton: {
    readonly backgroundColor: string;
    readonly borderRadius: number;
    readonly paddingHorizontal: number;
    readonly paddingVertical: number;
    readonly alignItems: "center";
    readonly justifyContent: "center";
  };
}

export interface YoloTheme {
  readonly preset: YoloPresetId;
  readonly canonicalId: CanonicalYoloPresetId;
  readonly displayName: string;
  readonly dark: boolean;
  readonly seedHex: string;
  readonly seedArgb: number;
  readonly omarchySubtheme?: OmarchySubThemeId;
  readonly tokens: YoloThemeTokens;
  readonly style: YoloStructuralStyle;
  readonly cssVariables: Record<string, string>;
  readonly cssText: string;
  readonly reactNativeStyleSheet: YoloReactNativeStyleSheet;
}

export interface YoloReactConfig {
  readonly appId: string;
  readonly title: string;
  readonly preset: YoloPresetId;
  readonly dark: boolean;
  readonly omarchySubtheme?: OmarchySubThemeId;
  readonly customSeedHex?: string;
  readonly rscEndpoint: string;
  readonly themeCssEndpoint: string;
  readonly enableReactCompiler: boolean;
  readonly enableFastRefresh: boolean;
  readonly enableByronLovePet: boolean;
}

interface PresetRecord {
  readonly preset: YoloPresetId;
  readonly canonicalId: CanonicalYoloPresetId;
  readonly displayName: string;
  readonly seedHex: string;
  readonly seedArgb: number;
  readonly dark: YoloThemeTokens;
  readonly light: YoloThemeTokens;
  readonly style: YoloStructuralStyle;
}

const PRESET_CATALOG: Record<YoloPresetId, PresetRecord> = {
  neutral: {
    preset: "neutral",
    canonicalId: "yolo-neutral",
    displayName: "Yolo Neutral (Customizable Default)",
    seedHex: "#6366F1",
    seedArgb: 0xff6366f1,
    dark: {
      name: "yolo-neutral-dark",
      bgPrimary: "#09090b",
      bgSurface: "#18181b",
      bgElevated: "#27272a",
      textPrimary: "#fafafa",
      textSecondary: "#a1a1aa",
      accentPrimary: "#6366f1",
      accentSecondary: "#818cf8",
      borderSubtle: "rgba(255, 255, 255, 0.08)",
    },
    light: {
      name: "yolo-neutral-light",
      bgPrimary: "#ffffff",
      bgSurface: "#f4f4f5",
      bgElevated: "#e4e4e7",
      textPrimary: "#09090b",
      textSecondary: "#52525b",
      accentPrimary: "#4f46e5",
      accentSecondary: "#6366f1",
      borderSubtle: "rgba(9, 9, 11, 0.10)",
    },
    style: {
      radiusSmPx: 6,
      radiusMdPx: 8,
      radiusLgPx: 12,
      radiusPillPx: 9999,
      spacingGridPx: 4,
      fontFamilyUi:
        "Inter, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
      fontFamilyMono:
        "'JetBrains Mono', ui-monospace, SFMono-Regular, Consolas, monospace",
      backdropBlurPx: 12,
      elevationLevel: 1,
      borderWidthPx: 1,
      windowVibrancy: "none",
    },
  },
  m3: {
    preset: "m3",
    canonicalId: "yolo-m3",
    displayName: "Google Material Design 3 (Jetpack Compose)",
    seedHex: "#6750A4",
    seedArgb: 0xff6750a4,
    dark: {
      name: "yolo-m3-dark",
      bgPrimary: "#141218",
      bgSurface: "#1d1b20",
      bgElevated: "#2b2930",
      textPrimary: "#e6e0e9",
      textSecondary: "#cac4d0",
      accentPrimary: "#d0bcff",
      accentSecondary: "#ccc2dc",
      borderSubtle: "#49454f",
    },
    light: {
      name: "yolo-m3-light",
      bgPrimary: "#fef7ff",
      bgSurface: "#f3edf7",
      bgElevated: "#ece6f0",
      textPrimary: "#1d1b20",
      textSecondary: "#49454f",
      accentPrimary: "#6750a4",
      accentSecondary: "#625b71",
      borderSubtle: "#cac4d0",
    },
    style: {
      radiusSmPx: 8,
      radiusMdPx: 12,
      radiusLgPx: 16,
      radiusPillPx: 9999,
      spacingGridPx: 4,
      fontFamilyUi:
        "'Google Sans Flex', 'Google Sans', Roboto, system-ui, sans-serif",
      fontFamilyMono: "'Google Sans Code', 'JetBrains Mono', monospace",
      backdropBlurPx: 0,
      elevationLevel: 2,
      borderWidthPx: 1,
      windowVibrancy: "surface-tint",
    },
  },
  fluent: {
    preset: "fluent",
    canonicalId: "yolo-fluent",
    displayName: "Microsoft Fluent UI 2 (WinUI 3 Mica/Acrylic)",
    seedHex: "#0078D4",
    seedArgb: 0xff0078d4,
    dark: {
      name: "yolo-fluent-dark",
      bgPrimary: "#202020",
      bgSurface: "#2c2c2c",
      bgElevated: "#383838",
      textPrimary: "#ffffff",
      textSecondary: "#c5c5c5",
      accentPrimary: "#60cdff",
      accentSecondary: "#0078d4",
      borderSubtle: "rgba(255, 255, 255, 0.08)",
    },
    light: {
      name: "yolo-fluent-light",
      bgPrimary: "#f3f3f3",
      bgSurface: "#fbfbfb",
      bgElevated: "#ffffff",
      textPrimary: "#1b1b1b",
      textSecondary: "#5d5d5d",
      accentPrimary: "#0078d4",
      accentSecondary: "#005fb8",
      borderSubtle: "rgba(0, 0, 0, 0.06)",
    },
    style: {
      radiusSmPx: 4,
      radiusMdPx: 4,
      radiusLgPx: 8,
      radiusPillPx: 9999,
      spacingGridPx: 4,
      fontFamilyUi: "'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif",
      fontFamilyMono: "'Cascadia Code', Consolas, monospace",
      backdropBlurPx: 20,
      elevationLevel: 1,
      borderWidthPx: 1,
      windowVibrancy: "mica",
    },
  },
  apple: {
    preset: "apple",
    canonicalId: "yolo-apple",
    displayName: "Apple HIG Liquid Glass (macOS Tahoe & iOS Cupertino)",
    seedHex: "#007AFF",
    seedArgb: 0xff007aff,
    dark: {
      name: "yolo-apple-dark",
      bgPrimary: "#1e1e1e",
      bgSurface: "#28282a",
      bgElevated: "#323236",
      textPrimary: "#ffffff",
      textSecondary: "#98989d",
      accentPrimary: "#0a84ff",
      accentSecondary: "#30d158",
      borderSubtle: "rgba(255, 255, 255, 0.14)",
    },
    light: {
      name: "yolo-apple-light",
      bgPrimary: "#f5f5f7",
      bgSurface: "#ffffff",
      bgElevated: "#ebebf0",
      textPrimary: "#1d1d1f",
      textSecondary: "#86868b",
      accentPrimary: "#007aff",
      accentSecondary: "#34c759",
      borderSubtle: "rgba(0, 0, 0, 0.10)",
    },
    style: {
      radiusSmPx: 6,
      radiusMdPx: 10,
      radiusLgPx: 14,
      radiusPillPx: 9999,
      spacingGridPx: 4,
      fontFamilyUi:
        "'SF Pro', -apple-system, BlinkMacSystemFont, 'Helvetica Neue', sans-serif",
      fontFamilyMono: "'SF Mono', ui-monospace, Menlo, monospace",
      backdropBlurPx: 24,
      elevationLevel: 2,
      borderWidthPx: 0.5,
      windowVibrancy: "macos-liquid-glass",
    },
  },
  omarchy: {
    preset: "omarchy",
    canonicalId: "yolo-omarchy",
    displayName: "Omacom Omarchy (Tokyo Night & 23 TUI-Modern Subthemes)",
    seedHex: "#7AA2F7",
    seedArgb: 0xff7aa2f7,
    dark: {
      name: "yolo-omarchy-tokyo-night",
      bgPrimary: "#1a1b26",
      bgSurface: "#24283b",
      bgElevated: "#292e42",
      textPrimary: "#c0caf5",
      textSecondary: "#a9b1d6",
      accentPrimary: "#7aa2f7",
      accentSecondary: "#bb9af7",
      borderSubtle: "#414868",
    },
    light: {
      name: "yolo-omarchy-catppuccin-latte",
      bgPrimary: "#eff1f5",
      bgSurface: "#dce0e8",
      bgElevated: "#ccd0da",
      textPrimary: "#4c4f69",
      textSecondary: "#5c5f77",
      accentPrimary: "#1e66f5",
      accentSecondary: "#ea76cb",
      borderSubtle: "#acb0be",
    },
    style: {
      radiusSmPx: 0,
      radiusMdPx: 0,
      radiusLgPx: 2,
      radiusPillPx: 2,
      spacingGridPx: 8,
      fontFamilyUi:
        "'Geist Variable', 'JetBrains Mono Variable', 'CaskaydiaMono Nerd Font', monospace",
      fontFamilyMono:
        "'JetBrains Mono Variable', 'JetBrains Mono', 'CaskaydiaMono Nerd Font', monospace",
      backdropBlurPx: 16,
      elevationLevel: 0,
      borderWidthPx: 2,
      windowVibrancy: "hyprland-blur",
    },
  },
  hyprland: {
    preset: "hyprland",
    canonicalId: "yolo-hyprland",
    displayName:
      "Hyprland Wayland Compositor (Cyan-Mint 45deg Border & Kawase Blur)",
    seedHex: "#33CCFF",
    seedArgb: 0xff33ccff,
    dark: {
      name: "yolo-hyprland-dark",
      bgPrimary: "#0b0e14",
      bgSurface: "#111622",
      bgElevated: "#192030",
      textPrimary: "#eef4fc",
      textSecondary: "#94a3b8",
      accentPrimary: "#33ccff",
      accentSecondary: "#00ff99",
      borderSubtle: "rgba(89, 89, 89, 0.67)",
    },
    light: {
      name: "yolo-hyprland-light",
      bgPrimary: "#f0f6fc",
      bgSurface: "#ffffff",
      bgElevated: "#e2ecf7",
      textPrimary: "#0b0e14",
      textSecondary: "#475569",
      accentPrimary: "#0099cc",
      accentSecondary: "#00b36b",
      borderSubtle: "rgba(15, 23, 42, 0.14)",
    },
    style: {
      radiusSmPx: 8,
      radiusMdPx: 10,
      radiusLgPx: 12,
      radiusPillPx: 9999,
      spacingGridPx: 5,
      fontFamilyUi:
        "'Inter', 'Noto Sans', 'JetBrains Mono', system-ui, sans-serif",
      fontFamilyMono: "'JetBrains Mono', 'CaskaydiaCove Nerd Font', monospace",
      backdropBlurPx: 24,
      elevationLevel: 2,
      borderWidthPx: 2,
      windowVibrancy: "kawase-blur",
    },
  },
  aphrody: {
    preset: "aphrody",
    canonicalId: "yolo-aphrody",
    displayName: "Aphrody Byron Love (10-Teinte Sprite Atlas & Celestial Gold)",
    seedHex: "#17335C",
    seedArgb: 0xff17335c,
    dark: {
      name: "aphrody-m3-dark",
      bgPrimary: "#090b10",
      bgSurface: "#111520",
      bgElevated: "#181e2e",
      textPrimary: "#f3f6fc",
      textSecondary: "#9ba6bf",
      accentPrimary: "#7c5cff",
      accentSecondary: "#00d4ff",
      borderSubtle: "rgba(255, 255, 255, 0.08)",
    },
    light: {
      name: "yolo-aphrody-light",
      bgPrimary: "#F7F3F0",
      bgSurface: "#EADBA6",
      bgElevated: "#C2B19E",
      textPrimary: "#131420",
      textSecondary: "#4E3738",
      accentPrimary: "#17335C",
      accentSecondary: "#91704C",
      borderSubtle: "rgba(23, 51, 92, 0.14)",
    },
    style: {
      radiusSmPx: 8,
      radiusMdPx: 12,
      radiusLgPx: 16,
      radiusPillPx: 9999,
      spacingGridPx: 4,
      fontFamilyUi:
        "'Google Sans Flex', 'Zed Plex Sans', Inter, system-ui, sans-serif",
      fontFamilyMono:
        "'Google Sans Code', 'Zed Plex Mono', 'JetBrains Mono', monospace",
      backdropBlurPx: 20,
      elevationLevel: 3,
      borderWidthPx: 1,
      windowVibrancy: "acrylic-dark-glass",
    },
  },
};

export const YOLO_PRESET_IDS: readonly YoloPresetId[] = [
  "neutral",
  "m3",
  "fluent",
  "apple",
  "omarchy",
  "hyprland",
  "aphrody",
] as const;

export function parseYoloPreset(input?: string): YoloPresetId {
  const cleaned = (input ?? "neutral").trim().toLowerCase();
  switch (cleaned) {
    case "neutral":
    case "yolo-neutral":
    case "default":
      return "neutral";
    case "m3":
    case "yolo-m3":
    case "material":
    case "material3":
      return "m3";
    case "fluent":
    case "yolo-fluent":
    case "fluent2":
    case "winui":
    case "winui3":
      return "fluent";
    case "apple":
    case "yolo-apple":
    case "macos":
    case "ios":
    case "cupertino":
    case "liquid-glass":
      return "apple";
    case "omarchy":
    case "yolo-omarchy":
    case "omacom":
    case "tokyo-night":
      return "omarchy";
    case "hyprland":
    case "yolo-hyprland":
    case "hyprwm":
    case "hypr":
      return "hyprland";
    case "aphrody":
    case "yolo-aphrody":
    case "byron-love":
    case "byron":
      return "aphrody";
    default:
      throw new RangeError(
        `Unknown YoloThemePreset "${input}". Expected one of: ${YOLO_PRESET_IDS.join(", ")}`,
      );
  }
}

export function normalizeSeedHex(seed: string): string {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(seed.trim());
  if (!m || !m[1]) {
    throw new TypeError(`Invalid hex seed color: ${JSON.stringify(seed)}`);
  }
  const raw = m[1];
  const full =
    raw.length === 3
      ? raw
          .split("")
          .map((c) => c + c)
          .join("")
      : raw;
  return `#${full.toUpperCase()}`;
}

/* @__PURE__ */
export function createYoloTheme(
  presetInput: YoloPresetId | CanonicalYoloPresetId | string = "neutral",
  dark = true,
  options: { omarchySubtheme?: OmarchySubThemeId; customSeedHex?: string } = {},
): YoloTheme {
  const preset = parseYoloPreset(presetInput);
  const entry = PRESET_CATALOG[preset];
  const tokens = dark ? entry.dark : entry.light;
  const style = entry.style;
  const seedHex = options.customSeedHex
    ? normalizeSeedHex(options.customSeedHex)
    : entry.seedHex;

  const cssVariables: Record<string, string> = {
    "--yolo-theme-preset": `"${entry.canonicalId}"`,
    "--yolo-theme-seed": seedHex,
    "--yolo-bg-primary": tokens.bgPrimary,
    "--yolo-bg-surface": tokens.bgSurface,
    "--yolo-bg-elevated": tokens.bgElevated,
    "--yolo-text-primary": tokens.textPrimary,
    "--yolo-text-secondary": tokens.textSecondary,
    "--yolo-accent-primary": tokens.accentPrimary,
    "--yolo-accent-secondary": tokens.accentSecondary,
    "--yolo-border-subtle": tokens.borderSubtle,
    "--yolo-radius-sm": `${style.radiusSmPx}px`,
    "--yolo-radius-md": `${style.radiusMdPx}px`,
    "--yolo-radius-lg": `${style.radiusLgPx}px`,
    "--yolo-radius-pill": `${style.radiusPillPx}px`,
    "--yolo-spacing-grid": `${style.spacingGridPx}px`,
    "--yolo-font-sans": style.fontFamilyUi,
    "--yolo-font-mono": style.fontFamilyMono,
    "--yolo-backdrop-blur": `${style.backdropBlurPx}px`,
    "--yolo-border-width": `${style.borderWidthPx}px`,
    "--yolo-window-vibrancy": style.windowVibrancy,
    // Material 3 / Shadcn compatibility aliases so migrated m3-react & shadcn components work out of the box
    "--md-sys-color-primary": tokens.accentPrimary,
    "--md-sys-color-secondary": tokens.accentSecondary,
    "--md-sys-color-surface": tokens.bgSurface,
    "--md-sys-color-on-surface": tokens.textPrimary,
    "--md-sys-color-outline-variant": tokens.borderSubtle,
  };

  if (preset === "hyprland") {
    cssVariables["--yolo-hyprland-active-border"] =
      "linear-gradient(45deg, #33ccff, #00ff99)";
    cssVariables["--yolo-hyprland-kawase-passes"] = "3";
  }

  const declarations = Object.entries(cssVariables)
    .map(([k, v]) => `  ${k}: ${v};`)
    .join("\n");
  const cssText = `:root {\n${declarations}\n}\n`;

  const reactNativeStyleSheet: YoloReactNativeStyleSheet = {
    container: {
      flex: 1,
      backgroundColor: tokens.bgPrimary,
    },
    surface: {
      backgroundColor: tokens.bgSurface,
      borderRadius: style.radiusMdPx,
      borderWidth: style.borderWidthPx,
      borderColor: tokens.borderSubtle,
      padding: style.spacingGridPx * 3,
    },
    elevatedCard: {
      backgroundColor: tokens.bgElevated,
      borderRadius: style.radiusLgPx,
      borderWidth: style.borderWidthPx,
      borderColor: tokens.borderSubtle,
      padding: style.spacingGridPx * 4,
    },
    textPrimary: {
      color: tokens.textPrimary,
      fontFamily: style.fontFamilyUi,
      fontSize: 16,
    },
    textSecondary: {
      color: tokens.textSecondary,
      fontFamily: style.fontFamilyUi,
      fontSize: 14,
    },
    accentButton: {
      backgroundColor: tokens.accentPrimary,
      borderRadius:
        preset === "omarchy" ? style.radiusMdPx : style.radiusPillPx,
      paddingHorizontal: style.spacingGridPx * 4,
      paddingVertical: style.spacingGridPx * 2.5,
      alignItems: "center",
      justifyContent: "center",
    },
  };

  return {
    preset,
    canonicalId: entry.canonicalId,
    displayName: entry.displayName,
    dark,
    seedHex,
    seedArgb: entry.seedArgb,
    omarchySubtheme:
      preset === "omarchy"
        ? (options.omarchySubtheme ?? "tokyo-night")
        : options.omarchySubtheme,
    tokens,
    style,
    cssVariables,
    cssText,
    reactNativeStyleSheet,
  };
}

/* @__PURE__ */
export function createYoloReactConfig(
  overrides: Partial<YoloReactConfig> = {},
): YoloReactConfig {
  const preset = overrides.preset
    ? parseYoloPreset(overrides.preset)
    : "neutral";
  return {
    appId: overrides.appId ?? "org.yolo.react-app",
    title: overrides.title ?? "Yolo Universal React App",
    preset,
    dark: overrides.dark ?? true,
    omarchySubtheme: overrides.omarchySubtheme,
    customSeedHex: overrides.customSeedHex,
    rscEndpoint: overrides.rscEndpoint ?? "/_yolo/rsc",
    themeCssEndpoint: overrides.themeCssEndpoint ?? "/_yolo/theme.css",
    enableReactCompiler: overrides.enableReactCompiler ?? true,
    enableFastRefresh: overrides.enableFastRefresh ?? true,
    enableByronLovePet: overrides.enableByronLovePet ?? preset === "aphrody",
  };
}
