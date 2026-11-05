import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import {
  executeUiTool,
  handleMcpJsonRpcRequest,
  MCP_TOOL_DEFINITIONS,
} from "../packages/yolo-ui-mcp/src/index";
import {
  computeContrast,
  DEVTOOLS_REGISTRY,
  type EmitTarget,
  emitThemeCode,
  type MigrateTarget,
  migrateTsxToRust,
  OMARCHY_SUBTHEMES,
  resolveThemeTokens,
  YOLO_THEME_PRESETS,
  yoloUiBunPlugin,
} from "../packages/yolo-ui-plugin/src/index";
import { runUnifiedUiCli } from "../scripts/ui";
import {
  AVAILABLE_STACKS,
  search,
  searchStack,
} from "../skills/yolo-ui-pro-max/scripts/core";

const ROOT = resolve(import.meta.dir, "..");

describe("Unified Yolo UI Skill, Plugin, MCP & CLI Suite", () => {
  test("1. Skill yolo-ui-pro-max has all 34 stacks, 8 Yolo brand presets, and mirrored SKILL.md", async () => {
    const skillPath = join(ROOT, "skills/yolo-ui-pro-max/SKILL.md");
    const mirrorPath = join(ROOT, ".agents/skills/yolo-ui-pro-max/SKILL.md");
    const bunOsRefPath = join(
      ROOT,
      "skills/yolo-ui-pro-max/references/bun-os.md",
    );
    const brandsCsvPath = join(ROOT, "skills/yolo-ui-pro-max/data/brands.csv");

    expect(existsSync(skillPath)).toBe(true);
    expect(existsSync(mirrorPath)).toBe(true);
    expect(existsSync(bunOsRefPath)).toBe(true);

    const skillText = readFileSync(skillPath, "utf8");
    const mirrorText = readFileSync(mirrorPath, "utf8");
    const bunOsText = readFileSync(bunOsRefPath, "utf8");
    expect(skillText).toBe(mirrorText);
    expect(skillText.includes("@aphrody/")).toBe(false);
    expect(bunOsText.includes("@aphrody/")).toBe(false);

    const requiredNewStacks = [
      "dioxus",
      "gpui",
      "bevy",
      "tauri-v3",
      "gtk4-rs",
      "cxx-qt",
      "base-ui",
      "omarchy",
      "hyprland",
    ];
    for (const s of requiredNewStacks) {
      expect(AVAILABLE_STACKS).toContain(s);
      const res = await searchStack("layout state theme", s, 3);
      expect(res.count).toBeGreaterThan(0);
    }
    expect(AVAILABLE_STACKS.length).toBe(34);

    const brandsText = readFileSync(brandsCsvPath, "utf8");
    for (const preset of [
      "yolo-neutral",
      "yolo-m3",
      "yolo-fluent",
      "yolo-apple",
      "yolo-omarchy",
      "yolo-hyprland",
      "yolo-aphrody",
      "yolo-chromium",
    ]) {
      expect(brandsText).toContain(preset);
    }

    const brandSearch = await search("yolo-hyprland", "brand", 3);
    expect(brandSearch.count).toBeGreaterThan(0);
  });

  test("2. @yolo-rs/ui-plugin emits all 8 presets, 23 Omarchy subthemes, 11 targets, and 6 Rust migration targets", () => {
    expect(Object.keys(YOLO_THEME_PRESETS)).toHaveLength(8);
    expect(Object.keys(OMARCHY_SUBTHEMES)).toHaveLength(23);
    expect(Object.keys(DEVTOOLS_REGISTRY)).toHaveLength(10);

    const omarchyTokyo = resolveThemeTokens("omarchy", "tokyo-night");
    expect(omarchyTokyo.backgroundPrimary).toBe("#1A1B26");
    expect(omarchyTokyo.subtheme).toBe("tokyo-night");

    const targets: EmitTarget[] = [
      "css",
      "gtk",
      "kde",
      "cosmic",
      "qml",
      "winui",
      "fluent",
      "apple",
      "omarchy",
      "hyprland",
      "dtcg",
    ];
    for (const t of targets) {
      const code = emitThemeCode({ preset: "aphrody", target: t });
      expect(code.length).toBeGreaterThan(30);
    }

    const tsxSnippet = `
      export function StudioToolbar() {
        return (
          <Dialog className="flex items-center gap-4 p-4 rounded-xl bg-slate-900">
            <Button className="px-3 py-2">Render</Button>
          </Dialog>
        );
      }
    `;
    const migrateTargets: MigrateTarget[] = [
      "dioxus",
      "wgpu",
      "egui",
      "cosmic",
      "gtk",
      "qml",
    ];
    for (const mt of migrateTargets) {
      const migrated = migrateTsxToRust(tsxSnippet, mt, "hyprland");
      expect(migrated.metadata.componentName).toBe("StudioToolbar");
      expect(migrated.metadata.detectedPrimitives).toContain("Dialog");
      expect(migrated.metadata.detectedPrimitives).toContain("Button");
      expect(migrated.code.length).toBeGreaterThan(40);
    }

    const contrast = computeContrast("#F8FAFC", "#0A0E17");
    expect(contrast.ratio).toBeGreaterThan(15);
    expect(contrast.aaNormal).toBe(true);
    expect(contrast.aaaNormal).toBe(true);
    expect(contrast.verdict).toBe("AAA");
  });

  test("3. @yolo-rs/ui-plugin Bun.plugin resolves virtual:yolo-theme and virtual:yolo-devtools", async () => {
    const plugin = yoloUiBunPlugin({ preset: "yolo-hyprland" });
    expect(plugin.name).toBe("yolo-ui-bun-plugin");

    const virtualEntry = join(ROOT, "tests/.virtual-entry-test.ts");
    await Bun.write(
      virtualEntry,
      `import tokens, { themeCss } from "virtual:yolo-theme";\nimport devtools from "virtual:yolo-devtools";\nexport { tokens, themeCss, devtools };\n`,
    );
    try {
      const buildResult = await Bun.build({
        entrypoints: [virtualEntry],
        plugins: [plugin],
      });
      expect(buildResult.success).toBe(true);
      const outputText = await buildResult.outputs[0]!.text();
      expect(outputText).toContain("yolo-hyprland");
      expect(outputText).toContain("dx serve --hot-reload --subsecond");
    } finally {
      await Bun.file(virtualEntry).delete();
    }
  });

  test("4. @yolo-rs/ui-mcp exposes and executes all 12 unified tools + JSON-RPC 2.0", async () => {
    expect(MCP_TOOL_DEFINITIONS).toHaveLength(12);
    const toolNames = MCP_TOOL_DEFINITIONS.map((t) => t.name);
    expect(toolNames).toEqual([
      "ui_search",
      "ui_design_system",
      "ui_theme_emit",
      "ui_migrate",
      "ui_contrast",
      "ui_devtools",
      "ui_awesome",
      "ui_catalog_lookup",
      "ui_workbenches",
      "ui_fleet_status",
      "ui_icons",
      "ui_local_model",
    ]);

    const searchOut = (await executeUiTool("ui_search", {
      query: "dock virtual list",
      stack: "gpui",
    })) as {
      result: { count: number };
    };
    expect(searchOut.result.count).toBeGreaterThan(0);

    const dsOut = (await executeUiTool("ui_design_system", {
      query: "creative video studio",
      projectName: "CutCraft Pro",
      preset: "aphrody",
      format: "markdown",
    })) as { preset: string; formatted: string };
    expect(dsOut.preset).toBe("yolo-aphrody");
    expect(dsOut.formatted).toContain("CutCraft Pro");

    const themeOut = (await executeUiTool("ui_theme_emit", {
      preset: "omarchy",
      omarchySubtheme: "cyberpunk",
      target: "omarchy",
    })) as { code: string };
    expect(themeOut.code).toContain(
      "Omarchy Unified Desktop Bundle (cyberpunk)",
    );

    const migrateOut = (await executeUiTool("ui_migrate", {
      source: `export function Header() { return <Tabs className="p-2"><Button>Go</Button></Tabs>; }`,
      target: "all",
      preset: "m3",
    })) as { mode: string; targets: Record<string, unknown> };
    expect(migrateOut.mode).toBe("multi-target");
    expect(Object.keys(migrateOut.targets)).toHaveLength(6);

    const contrastOut = (await executeUiTool("ui_contrast", {
      foreground: "#FAF7F0",
      background: "#1C1A17",
    })) as { aaNormal: boolean };
    expect(contrastOut.aaNormal).toBe(true);

    const devtoolsOut = (await executeUiTool("ui_devtools", {
      engine: "tauri",
    })) as {
      descriptor: { id: string };
    };
    expect(devtoolsOut.descriptor.id).toBe("tauri");

    const awesomeOut = (await executeUiTool("ui_awesome", {
      query: "gpui",
    })) as {
      curatedEcosystems: unknown[];
    };
    expect(awesomeOut.curatedEcosystems.length).toBeGreaterThan(0);

    const catalogOut = (await executeUiTool("ui_catalog_lookup", {
      query: "Dialog",
    })) as {
      matches: Record<string, unknown>;
    };
    expect(Object.keys(catalogOut.matches).length).toBeGreaterThan(0);

    const workbenchesOut = (await executeUiTool("ui_workbenches", {})) as {
      craftWorkbenchCount: number;
      storytellerModuleCount: number;
    };
    expect(workbenchesOut.craftWorkbenchCount).toBe(13);
    expect(workbenchesOut.storytellerModuleCount).toBe(10);

    const fleetOut = (await executeUiTool("ui_fleet_status", {
      stack: "dioxuslabs",
    })) as {
      totalSubmodules: number;
      matchedCount: number;
    };
    expect(fleetOut.totalSubmodules).toBeGreaterThanOrEqual(70);
    expect(fleetOut.matchedCount).toBeGreaterThan(0);

    // JSON-RPC 2.0 test
    const initResp = await handleMcpJsonRpcRequest({
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
    });
    expect(initResp?.result).toBeDefined();

    const listResp = (await handleMcpJsonRpcRequest({
      jsonrpc: "2.0",
      id: 2,
      method: "tools/list",
    })) as {
      result: { tools: unknown[] };
    };
    expect(listResp.result.tools).toHaveLength(12);

    const callResp = (await handleMcpJsonRpcRequest({
      jsonrpc: "2.0",
      id: 3,
      method: "tools/call",
      params: {
        name: "ui_contrast",
        arguments: { foreground: "#FFFFFF", background: "#000000" },
      },
    })) as { result: { isError: boolean } };
    expect(callResp.result.isError).toBe(false);
  });

  test("5. Unified CLI (scripts/ui.ts -> bun ui) doctor and subcommands succeed", async () => {
    const codeDoctor = await runUnifiedUiCli(["doctor"]);
    expect(codeDoctor).toBe(0);

    const codeContrast = await runUnifiedUiCli([
      "contrast",
      "#E8C547",
      "#1C1A17",
    ]);
    expect(codeContrast).toBe(0);

    const codeDevtools = await runUnifiedUiCli(["devtools", "dioxus"]);
    expect(codeDevtools).toBe(0);
  });
});
