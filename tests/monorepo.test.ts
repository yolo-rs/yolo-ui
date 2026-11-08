import { describe, expect, test } from "bun:test";
import { join, resolve } from "node:path";
import {
  buildInventory,
  categorizeSubmodule,
  discoverSubmodules,
} from "../scripts/fleet.ts";
import { auditBunMigration } from "../scripts/migrate-bun.ts";
import { analyzeWorkspaceOptimization } from "../scripts/optimize.ts";

const ROOT_DIR = resolve(import.meta.dir, "..");

describe("Aphrody Labs Yolo UI Monorepo & Fleet Integrity", () => {
  test("Root Cargo.toml exists and defines flattened workspace, package, dependencies, and optimized profiles", async () => {
    const cargoFile = Bun.file(join(ROOT_DIR, "Cargo.toml"));
    expect(await cargoFile.exists()).toBe(true);

    const content = await cargoFile.text();
    expect(content).toContain("[workspace]");
    expect(content).toContain("[workspace.package]");
    expect(content).toContain("[workspace.dependencies]");
    expect(content).toContain("[profile.dev]");
    expect(content).toContain("[profile.release]");
    expect(content).toContain("[profile.wasm-split-release]");
    expect(content).toContain("[profile.cli-release-dev]");

    const parsed = Bun.TOML.parse(content) as {
      workspace?: {
        resolver?: string;
        members?: string[];
        exclude?: string[];
        package?: { version?: string; edition?: string };
        dependencies?: Record<string, unknown>;
      };
      profile?: Record<string, Record<string, unknown>>;
    };

    expect(parsed.workspace?.resolver).toBe("2");
    expect(parsed.workspace?.package?.version).toBe("0.8.0-alpha.3");
    expect(parsed.workspace?.package?.edition).toBe("2024");
    expect(parsed.workspace?.members).toContain("crates/yolo-shared");
    expect(parsed.workspace?.members).toContain("apps/yolo-showcase");
    expect(parsed.workspace?.members).toContain("packages/yolo-ui");
    expect(parsed.workspace?.members).toContain("packages/yolo-gpui");
    expect(parsed.workspace?.members).toContain("packages/yolo-iced");
    expect(parsed.workspace?.members).toContain("packages/yolo-tauri");
    expect(parsed.workspace?.members).toContain("packages/dioxus");
    expect(parsed.profile?.["dev"]?.["debug"]).toBe("line-tables-only");
    expect(parsed.profile?.["release"]?.["incremental"]).toBe(true);
    expect(parsed.profile?.["wasm-split-release"]?.["lto"]).toBe(true);
    expect(parsed.profile?.["cli-release-dev"]?.["opt-level"]).toBe(3);
  });

  test("Shared foundation crate crates/yolo-shared and umbrella packages/yolo-ui export unified tokens & AppShell", async () => {
    const sharedLib = Bun.file(
      join(ROOT_DIR, "crates", "yolo-shared", "src", "lib.rs"),
    );
    const uiLib = Bun.file(
      join(ROOT_DIR, "packages", "yolo-ui", "src", "lib.rs"),
    );
    expect(await sharedLib.exists()).toBe(true);
    expect(await uiLib.exists()).toBe(true);

    const sharedSource = await sharedLib.text();
    expect(sharedSource).toContain("pub struct RgbaColor");
    expect(sharedSource).toContain("pub struct YoloThemeTokens");
    expect(sharedSource).toContain("AphrodyThemeTokens");
    expect(sharedSource).toContain("APHRODY_DARK_THEME");

    const libSource = await uiLib.text();
    expect(libSource).toContain("pub mod theme");
    expect(libSource).toContain("AphrodyThemeTokens");
    expect(libSource).toContain("APHRODY_DARK_THEME");
    expect(libSource).toContain("pub mod seo");
    expect(libSource).toContain("pub fn SiteMeta");
    expect(libSource).toContain("pub mod shell");
    expect(libSource).toContain("pub fn AppShell");
    expect(libSource).toContain("pub mod prelude");
  });

  test("End-to-end Justfile, .cargo/config.toml (sccache), and Bun n2b migration are configured cleanly", async () => {
    const justfile = Bun.file(join(ROOT_DIR, "Justfile"));
    const cargoConfig = Bun.file(join(ROOT_DIR, ".cargo", "config.toml"));
    expect(await justfile.exists()).toBe(true);
    expect(await cargoConfig.exists()).toBe(true);

    const cargoConfigText = await cargoConfig.text();
    expect(cargoConfigText).toContain('rustc-wrapper = "sccache"');
    expect(cargoConfigText).toContain('protocol = "sparse"');

    const audit = await auditBunMigration();
    expect(audit.legacyLockfilesFound).toEqual([]);
    expect(audit.legacyConfigFilesFound).toEqual([]);
    expect(audit.bunWorkspacesConfigured).toBe(true);
    expect(audit.migratedScriptsCount).toBeGreaterThanOrEqual(20);
  });

  test("Management and optimization scripts exist and are valid TypeScript", async () => {
    const scriptNames = [
      "fleet.ts",
      "build.ts",
      "test.ts",
      "optimize.ts",
      "migrate-bun.ts",
      "ui.ts",
      "react-bun.ts",
    ];
    for (const scriptName of scriptNames) {
      const scriptPath = join(ROOT_DIR, "scripts", scriptName);
      const file = Bun.file(scriptPath);
      expect(await file.exists()).toBe(true);
      const source = await file.text();
      expect(source.length).toBeGreaterThan(100);
    }

    const transpiler = new Bun.Transpiler({ loader: "ts" });
    for (const scriptName of scriptNames) {
      const scriptPath = join(ROOT_DIR, "scripts", scriptName);
      const source = await Bun.file(scriptPath).text();
      const transpiled = transpiler.transformSync(source);
      expect(typeof transpiled).toBe("string");
      expect(transpiled.length).toBeGreaterThan(0);
    }
  });

  test("Fleet discovery and stack categorization classify all 5 upstream ecosystems accurately", async () => {
    expect(categorizeSubmodule("blitz")).toBe("dioxuslabs");
    expect(categorizeSubmodule("gpui-kit")).toBe("longbridge");
    expect(categorizeSubmodule("rust-i18n")).toBe("longbridge");
    expect(categorizeSubmodule("nsh")).toBe("longbridge");
    expect(categorizeSubmodule("gpui-fast")).toBe("zed-industries");
    expect(categorizeSubmodule("iced")).toBe("iced-rs");
    expect(categorizeSubmodule("iced_aw")).toBe("iced-rs");
    expect(categorizeSubmodule("cryoglyph")).toBe("iced-rs");
    expect(categorizeSubmodule("comet")).toBe("iced-rs");
    expect(categorizeSubmodule("tauri")).toBe("tauri-apps");
    expect(categorizeSubmodule("wry")).toBe("tauri-apps");
    expect(categorizeSubmodule("tao")).toBe("tauri-apps");
    expect(categorizeSubmodule("muda")).toBe("tauri-apps");
    expect(categorizeSubmodule("tray-icon")).toBe("tauri-apps");
    expect(categorizeSubmodule("egui")).toBe("emilk");
    expect(categorizeSubmodule("libcosmic")).toBe("pop-os");
    expect(categorizeSubmodule("gtk4-rs")).toBe("gtk-rs");
    expect(categorizeSubmodule("cxx-qt")).toBe("KDAB");
    expect(categorizeSubmodule("fluentui")).toBe("microsoft");
    expect(categorizeSubmodule("material-web")).toBe("material-components");
    expect(categorizeSubmodule("omarchy")).toBe("omacom");
    expect(categorizeSubmodule("hyprland")).toBe("hyprwm");
    expect(categorizeSubmodule("shadcn-ui")).toBe("shadcn-ui");
    expect(categorizeSubmodule("base-ui")).toBe("mui");
    expect(categorizeSubmodule("tailwindcss")).toBe("tailwindlabs");
    expect(categorizeSubmodule("react")).toBe("facebook");
    expect(categorizeSubmodule("expo")).toBe("expo");

    const submodules = await discoverSubmodules();
    expect(submodules.length).toBeGreaterThanOrEqual(120);

    const inventory = await buildInventory();
    expect(inventory.version).toBe("0.8.0-alpha.3");
    expect(inventory.totalSubmodules).toBe(inventory.submodules.length);
    expect(inventory.totalSubmodules).toBeGreaterThanOrEqual(120);
  });

  test("Workspace optimization analyzer reports valid metrics and zero duplicate dependencies", async () => {
    const report = await analyzeWorkspaceOptimization();
    expect(report.workspaceMembersCount).toBeGreaterThanOrEqual(80);
    expect(report.workspaceDependenciesCount).toBeGreaterThanOrEqual(300);
    expect(report.duplicateWorkspaceDeps).toEqual([]);
    expect(report.compilerOptimizations.devLineTablesOnly).toBe(true);
    expect(report.compilerOptimizations.releaseIncremental).toBe(true);
    expect(report.compilerOptimizations.wasmSplitLto).toBe(true);
    expect(report.compilerOptimizations.cliReleaseOptLevel3).toBe(true);
  }, 15000);
});
