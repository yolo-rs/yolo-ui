// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import {
  YOLO_MIGRATION_TARGETS,
  YOLO_THEME_PRESETS,
  extractCvaVariants,
  generateRustProjectBridge,
  inspectShadcnProject,
  parseTailwindV4Theme,
} from "../src/index.ts";

describe("@yolo-rs/shadcn-bun", () => {
  test("supports all 6 Rust targets and all 8 YoloThemePresets", () => {
    expect(YOLO_MIGRATION_TARGETS).toEqual([
      "dioxus",
      "wgpu",
      "egui",
      "cosmic",
      "gtk",
      "qml",
    ]);
    expect(YOLO_THEME_PRESETS).toEqual([
      "neutral",
      "m3",
      "fluent",
      "apple",
      "omarchy",
      "hyprland",
      "aphrody",
      "chromium",
    ]);
  });

  test("parses Tailwind CSS v4 @theme and Shadcn :root / .dark variables", () => {
    const css = `
      @import "tailwindcss";
      @theme {
        --color-primary: oklch(0.62 0.21 259);
        --color-card: oklch(0.99 0 0);
        --radius-lg: 12px;
        --font-sans: "Google Sans Flex", sans-serif;
      }
      :root {
        --background: 0 0% 100%;
        --primary: 222.2 47.4% 11.2%;
        --radius: 0.75rem;
      }
      .dark {
        --background: 222.2 84% 4.9%;
      }
    `;
    const parsed = parseTailwindV4Theme(css);
    expect(parsed.colors.primary).toBe("oklch(0.62 0.21 259)");
    expect(parsed.radii.lg).toBe("12px");
    expect(parsed.fonts.sans).toContain("Google Sans Flex");
    expect(parsed.rootVars.radius).toBe("0.75rem");
    expect(parsed.darkVars.background).toBe("222.2 84% 4.9%");
  });

  test("extracts cva() class-variance-authority variants and emits Rust enums", async () => {
    const buttonTsx = `
      import * as React from "react";
      import { Slot } from "@radix-ui/react-slot";
      import { cva } from "class-variance-authority";

      export const buttonVariants = cva(
        "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium",
        {
          variants: {
            variant: {
              default: "bg-primary text-primary-foreground shadow-sm",
              destructive: "bg-destructive text-white",
              outline: "border bg-background shadow-xs"
            },
            size: {
              default: "h-9 px-4 py-2",
              sm: "h-8 rounded-md px-3",
              lg: "h-10 rounded-md px-6"
            }
          },
          defaultVariants: {
            variant: "default",
            size: "default"
          }
        }
      );
    `;
    const specs = extractCvaVariants(buttonTsx);
    expect(specs).toHaveLength(1);
    expect(specs[0]!.ident).toBe("buttonVariants");
    expect(Object.keys(specs[0]!.axes.variant!)).toEqual([
      "default",
      "destructive",
      "outline",
    ]);
    expect(specs[0]!.defaultVariants.variant).toBe("default");

    const rustCode = generateRustProjectBridge(
      {
        rootDir: "/workspace/app",
        config: null,
        tailwindConfigPath: null,
        themeTokens: {
          colors: {},
          radii: {},
          spacing: {},
          fonts: {},
          rootVars: {},
          darkVars: {},
        },
        components: [
          {
            relativePath: "components/ui/button.tsx",
            absolutePath: "/workspace/app/components/ui/button.tsx",
            stem: "button",
            family: "shadcn",
            primitiveImports: ["@radix-ui/react-slot"],
            cvaSpecs: specs,
          },
        ],
      },
      "hyprland",
    );
    expect(rustCode).toContain("YoloThemePreset::Hyprland");
    expect(rustCode).toContain("pub enum ButtonVariantsVariant");
    expect(rustCode).toContain("pub enum ButtonVariantsSize");
  });

  test("inspects a temporary Shadcn + Base UI project using Bun.file and Bun.Glob", async () => {
    const tmpDir = `${import.meta.dir}/.tmp-shadcn-fixture`;
    await Bun.write(
      `${tmpDir}/components.json`,
      JSON.stringify({
        style: "new-york",
        rsc: true,
        tsx: true,
        tailwind: {
          css: "src/app/globals.css",
          baseColor: "neutral",
          cssVariables: true,
        },
        aliases: {
          components: "@/components",
          utils: "@/lib/utils",
          ui: "@/components/ui",
        },
      }),
    );
    await Bun.write(
      `${tmpDir}/src/app/globals.css`,
      `@theme { --color-primary: #38bdf8; --radius-md: 8px; }`,
    );
    await Bun.write(
      `${tmpDir}/src/components/ui/switch.tsx`,
      `import { Switch } from "@base-ui-components/react/switch"; export const AppSwitch = Switch.Root;`,
    );

    const inspection = await inspectShadcnProject(tmpDir);
    expect(inspection.config?.style).toBe("new-york");
    expect(inspection.themeTokens.colors.primary).toBe("#38bdf8");
    expect(inspection.components).toHaveLength(1);
    expect(inspection.components[0]!.family).toBe("base-ui");

    await Bun.$`rm -rf ${tmpDir}`.quiet().nothrow();
  });
});
