// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/react/build — Native Bun.build Wrapper for React 19 & Expo Web/Universal
// Purges Webpack/Babel/Node in favor of Bun.build + Rust YoloThemePreset CSS & RN JSON emission.

import { join } from "node:path";
import { createYoloTheme, type YoloPresetId } from "./core.ts";

export interface YoloReactBuildOptions {
  readonly entrypoints: string[];
  readonly outdir: string;
  readonly preset?: YoloPresetId;
  readonly dark?: boolean;
  readonly minify?: boolean;
  readonly sourcemap?: "none" | "linked" | "inline" | "external";
  readonly target?: "browser" | "bun";
}

export interface YoloReactBuildResult {
  readonly success: boolean;
  readonly outputs: string[];
  readonly themeCssPath: string;
  readonly themeRnJsonPath: string;
  readonly preset: YoloPresetId;
  readonly logs: string[];
}

export async function buildYoloReactApp(
  options: YoloReactBuildOptions,
): Promise<YoloReactBuildResult> {
  const preset = options.preset ?? "neutral";
  const dark = options.dark ?? true;
  const theme = createYoloTheme(preset, dark);

  const buildResult = await Bun.build({
    entrypoints: options.entrypoints,
    outdir: options.outdir,
    target: options.target ?? "browser",
    minify: options.minify ?? true,
    sourcemap: options.sourcemap ?? "linked",
    define: {
      "process.env.NODE_ENV": JSON.stringify("production"),
      __YOLO_THEME_PRESET__: JSON.stringify(theme.canonicalId),
      __YOLO_THEME_SEED__: JSON.stringify(theme.seedHex),
    },
  });

  const themeCssPath = join(options.outdir, "yolo-theme.css");
  const themeRnJsonPath = join(options.outdir, "yolo-theme-rn.json");

  await Bun.write(themeCssPath, theme.cssText);
  await Bun.write(
    themeRnJsonPath,
    JSON.stringify(
      {
        preset: theme.preset,
        canonicalId: theme.canonicalId,
        seedHex: theme.seedHex,
        dark: theme.dark,
        tokens: theme.tokens,
        style: theme.style,
        reactNativeStyleSheet: theme.reactNativeStyleSheet,
      },
      null,
      2,
    ),
  );

  const outputs = [
    ...buildResult.outputs.map((o) => o.path),
    themeCssPath,
    themeRnJsonPath,
  ];

  return {
    success: buildResult.success,
    outputs,
    themeCssPath,
    themeRnJsonPath,
    preset,
    logs: buildResult.logs.map((l) => String(l)),
  };
}
