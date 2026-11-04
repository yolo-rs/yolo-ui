#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { join } from "node:path";
import {
  executeUiTool,
  MCP_TOOL_DEFINITIONS,
  REPO_ROOT,
  runToolHostCli,
  startMcpStdioServer,
} from "../packages/yolo-ui-mcp/src/index";
import {
  DEVTOOLS_REGISTRY,
  OMARCHY_SUBTHEMES,
  YOLO_THEME_PRESETS,
} from "../packages/yolo-ui-plugin/src/index";
import {
  AVAILABLE_STACKS,
  CSV_CONFIG,
} from "../skills/yolo-ui-pro-max/scripts/core";

function printHelp(): void {
  console.log(`
Yolo UI Unified CLI (bun ui / just ui) — v0.8.0
One Command • One Plugin (@yolo-rs/ui-plugin) • One MCP (@yolo-rs/ui-mcp) • UI/UX Pro Max

Usage:
  bun ui search <query> [--domain <d>] [--stack <s>] [-n <max>]
  bun ui system <query> [-p "Project Name"] [--preset neutral|m3|fluent|apple|omarchy|hyprland|aphrody] [--format ascii|markdown|json]
  bun ui theme [--preset <p>] [--omarchy <subtheme>] [--target css|gtk|kde|cosmic|qml|winui|fluent|apple|omarchy|hyprland|dtcg]
  bun ui migrate <file.tsx|snippet> [--target dioxus|wgpu|egui|cosmic|gtk|qml|all] [--preset <p>]
  bun ui contrast <fg-hex> <bg-hex>
  bun ui devtools [dioxus|gpui|egui|iced|cosmic|bevy|tauri|gtk4|qt6|shadcn-tailwind]
  bun ui awesome [query]
  bun ui catalog [query]
  bun ui workbenches [filter]
  bun ui fleet [query] [--stack <owner>]
  bun ui plan [--pillar sites|iced|egui|taffy|wgpui|gpui|rust-cli]
  bun ui site [--emit <dir>]
  bun ui mcp [list | call <tool> '<json>' | serve]
  bun ui doctor
`);
}

function parseFlags(args: string[]): {
  positional: string[];
  flags: Record<string, string | boolean>;
} {
  const positional: string[] = [];
  const flags: Record<string, string | boolean> = {};
  for (let i = 0; i < args.length; i++) {
    const arg = args[i]!;
    if (arg.startsWith("--")) {
      const eqIdx = arg.indexOf("=");
      if (eqIdx !== -1) {
        flags[arg.slice(2, eqIdx)] = arg.slice(eqIdx + 1);
      } else {
        const key = arg.slice(2);
        const next = args[i + 1];
        if (next && !next.startsWith("-")) {
          flags[key] = next;
          i++;
        } else {
          flags[key] = true;
        }
      }
    } else if (arg.startsWith("-") && arg.length === 2) {
      const key = arg.slice(1);
      const next = args[i + 1];
      if (next && !next.startsWith("-")) {
        flags[key] = next;
        i++;
      } else {
        flags[key] = true;
      }
    } else {
      positional.push(arg);
    }
  }
  return { positional, flags };
}

export async function runUnifiedUiCli(
  argv: string[] = process.argv.slice(2),
): Promise<number> {
  if (
    argv.length === 0 ||
    argv[0] === "--help" ||
    argv[0] === "-h" ||
    argv[0] === "help"
  ) {
    printHelp();
    return 0;
  }

  const subcommand = argv[0]!;
  const rest = argv.slice(1);
  const { positional, flags } = parseFlags(rest);

  switch (subcommand) {
    case "search": {
      const query = positional.join(" ").trim() || "ui";
      const domain =
        typeof flags.domain === "string"
          ? flags.domain
          : typeof flags.d === "string"
            ? flags.d
            : undefined;
      const stack =
        typeof flags.stack === "string"
          ? flags.stack
          : typeof flags.s === "string"
            ? flags.s
            : undefined;
      const maxResults = Number(flags.max ?? flags.n ?? 5);
      const out = await executeUiTool("ui_search", {
        query,
        domain,
        stack,
        maxResults,
      });
      console.log(JSON.stringify(out, null, 2));
      return 0;
    }

    case "system": {
      const query =
        positional.join(" ").trim() || "modern multi-engine desktop studio";
      const projectName =
        typeof flags.project === "string"
          ? flags.project
          : typeof flags.p === "string"
            ? flags.p
            : "Yolo UI Application";
      const preset =
        typeof flags.preset === "string" ? flags.preset : "neutral";
      const format =
        typeof flags.format === "string"
          ? flags.format
          : typeof flags.f === "string"
            ? flags.f
            : "markdown";
      const variance = flags.variance ? Number(flags.variance) : undefined;
      const motion = flags.motion ? Number(flags.motion) : undefined;
      const density = flags.density ? Number(flags.density) : undefined;

      const res = (await executeUiTool("ui_design_system", {
        query,
        projectName,
        preset,
        format,
        variance,
        motion,
        density,
      })) as { formatted: string };
      console.log(res.formatted);
      return 0;
    }

    case "theme": {
      const preset =
        typeof flags.preset === "string"
          ? flags.preset
          : (positional[0] ?? "neutral");
      const omarchySubtheme =
        typeof flags.omarchy === "string"
          ? flags.omarchy
          : typeof flags.subtheme === "string"
            ? flags.subtheme
            : undefined;
      const target =
        typeof flags.target === "string"
          ? flags.target
          : typeof flags.t === "string"
            ? flags.t
            : "css";
      const res = (await executeUiTool("ui_theme_emit", {
        preset,
        omarchySubtheme,
        target,
      })) as { code: string };
      console.log(res.code);
      return 0;
    }

    case "migrate": {
      const source =
        positional[0] ??
        `<Card className="p-4 rounded-lg bg-slate-900"><Button>Launch</Button></Card>`;
      const target =
        typeof flags.target === "string"
          ? flags.target
          : typeof flags.t === "string"
            ? flags.t
            : "dioxus";
      const preset =
        typeof flags.preset === "string" ? flags.preset : "neutral";
      const res = (await executeUiTool("ui_migrate", {
        source,
        target,
        preset,
      })) as {
        code?: string;
        targets?: Record<string, { code: string }>;
      };
      if (res.code) {
        console.log(res.code);
      } else {
        console.log(JSON.stringify(res, null, 2));
      }
      return 0;
    }

    case "contrast": {
      const fg = positional[0] ?? "#F8FAFC";
      const bg = positional[1] ?? "#0A0E17";
      const res = await executeUiTool("ui_contrast", {
        foreground: fg,
        background: bg,
      });
      console.log(JSON.stringify(res, null, 2));
      return 0;
    }

    case "devtools": {
      const engine =
        positional[0] ??
        (typeof flags.engine === "string" ? flags.engine : undefined);
      const res = await executeUiTool("ui_devtools", { engine });
      console.log(JSON.stringify(res, null, 2));
      return 0;
    }

    case "awesome": {
      const query = positional.join(" ").trim();
      const res = await executeUiTool("ui_awesome", { query });
      console.log(JSON.stringify(res, null, 2));
      return 0;
    }

    case "catalog": {
      const query = positional.join(" ").trim();
      const res = await executeUiTool("ui_catalog_lookup", { query });
      console.log(JSON.stringify(res, null, 2));
      return 0;
    }

    case "workbenches": {
      const filter = positional.join(" ").trim();
      const res = await executeUiTool("ui_workbenches", { filter });
      console.log(JSON.stringify(res, null, 2));
      return 0;
    }

    case "fleet": {
      const query = positional.join(" ").trim();
      const stack = typeof flags.stack === "string" ? flags.stack : undefined;
      const res = await executeUiTool("ui_fleet_status", { query, stack });
      console.log(JSON.stringify(res, null, 2));
      return 0;
    }

    case "mcp": {
      const mcpCmd = rest[0] ?? "list";
      if (mcpCmd === "serve") {
        await startMcpStdioServer();
        return 0;
      }
      return await runToolHostCli(rest);
    }

    case "plan": {
      const pillarFilter =
        typeof flags.pillar === "string"
          ? flags.pillar.toLowerCase()
          : positional[0]?.toLowerCase();
      const pillars = [
        {
          id: "sites-to-dioxus",
          short: "sites",
          title: "1. Sites Web & Fullstack -> Dioxus 0.8 (SSR + WASM + Blitz)",
          targetCrate: "packages/yolo-ui (dioxus 0.8 + blitz + subsecond)",
          targets: [
            "yolo.aphrody.com (Unified Hub: Awesome + Showcase + Templates + Migration)",
            "aphrody-web & m3-showcase (bun-os/apps/aphrody-web & bun-ui/apps/m3-web)",
            "13 *craft-web apps (word, grid, deck, pdf, photo, vector, light, film, effect, sound, design, fig, cad)",
            "iecode-web / iecode-web-wasm / iecode-web-wii-wasm",
            "rg-website & azalee-web (rosegriffon)",
            "shenron-site & shenron-activity (shenron)",
            "auto-web & auto-admin (auto)",
            "ft-home & ft-design-system (ft)",
          ],
        },
        {
          id: "desktop-to-iced",
          short: "iced",
          title: "2A. Apps Desktop Retained MVU -> Iced 0.14 & libcosmic",
          targetCrate: "packages/yolo-iced + packages/yolo-cosmic",
          targets: [
            "bunos-win11-chrome & bun-os desktop shell",
            "craft-hub & bunos-flutter-store",
            "spacedrive & spotcraft (Iced 0.14 + iced_audio DSP)",
            "wii-os menu-native & auto-desktop",
          ],
        },
        {
          id: "desktop-to-egui",
          short: "egui",
          title:
            "2B. Apps Desktop Workbenches & Emulateurs -> Egui 0.36 (Craft & Storyteller)",
          targetCrate:
            "packages/yolo-egui + crates/yolo-craft + crates/yolo-storyteller",
          targets: [
            "13 Craft Creative Workbenches (Word, Grid, Deck, Pdf, Photo, Vector, Light, Film, Effect, Sound, Design, Fig, Cad)",
            "10 Storyteller Narrative Studio Modules + Byron Love Character Sheet",
            "gecko-debugger (wii-os), aphrody-ctr-desktop & ruzu (bun-os)",
          ],
        },
        {
          id: "css-to-taffy",
          short: "taffy",
          title:
            "3. CSS & Tailwind v4 -> Taffy 0.15 Layout Engine (yolo-layout)",
          targetCrate:
            "crates/yolo-layout (taffy 0.15 + TailwindStyleResolver)",
          targets: [
            "All Tailwind v4 utility classes -> taffy::Style Flexbox/Grid",
            "4px/8px subpixel layout snapping (snap_to_grid)",
            "48x48px WCAG AAA touch target enforcement (enforce_min_touch_target)",
          ],
        },
        {
          id: "render3d-to-wgpui",
          short: "wgpui",
          title:
            "4. Rendu 3D & Shaders -> GPUI + WGPU 24 + WGPUI (yolo-gpu & Bevy 0.16)",
          targetCrate:
            "crates/yolo-gpu + crates/yolo-render + packages/yolo-gpui",
          targets: [
            "aphrody-model-view, aphrody-nie-render3d & aphrody-tauri-3d",
            "gecko-backend-wgpu, aphrody-dust-wgpu & vectorcraft-three-d",
            "Byron Love 8x11 Sprite & 3D Stage Viewport",
          ],
        },
        {
          id: "dev-ui-to-zed-gpui",
          short: "gpui",
          title:
            "5. UI de Dev, IDE & Cockpits Agents -> Zed & GPUI (yolo-gpui)",
          targetCrate:
            "packages/yolo-gpui (zed gpui + longbridge gpui-component)",
          targets: [
            "aphrody-agent-home, ai-agent-runner & shenron-cockpit",
            "aphrody-code-graph, aphrody-term & MCP Inspector Studio (ft/mcp, FCMCP)",
            "yolo-devtools-studio (120 FPS DockArea + VirtualList + CommandPalette + TreeDiff)",
          ],
        },
        {
          id: "tools-to-native-rust",
          short: "rust-cli",
          title: "6. Outils, CLIs & Pipelines -> Rust Natif (Cargo Binaries)",
          targetCrate: "crates/yolo-migrator + apps/yolo-showcase",
          targets: [
            "All 14 *craft-cli binaries",
            "iecode-dotnet -> aphrody-nie-cli",
            "obscura-cli, gecko-multitool, yolo-migrate & yolo-showcase",
          ],
        },
      ];
      const filtered = pillarFilter
        ? pillars.filter(
            (p) =>
              p.id.includes(pillarFilter) ||
              p.short.includes(pillarFilter) ||
              p.title.toLowerCase().includes(pillarFilter),
          )
        : pillars;
      console.log(
        JSON.stringify(
          {
            domain: "yolo.aphrody.com",
            planDoc: "docs/YOLO-MASTER-MIGRATION-AND-SITE-PLAN.md",
            totalPillars: pillars.length,
            pillars: filtered,
          },
          null,
          2,
        ),
      );
      return 0;
    }

    case "site": {
      const emitDir =
        typeof flags.emit === "string"
          ? flags.emit
          : positional[0] || "dist/yolo-aphrody-com";
      const proc = Bun.spawnSync(
        [
          "cargo",
          "run",
          "-q",
          "-p",
          "yolo-showcase",
          "--",
          "--emit-site",
          emitDir,
        ],
        {
          cwd: REPO_ROOT,
          stdout: "pipe",
          stderr: "pipe",
        },
      );
      if (proc.exitCode !== 0) {
        console.error(proc.stderr.toString());
        return proc.exitCode;
      }
      console.log(
        JSON.stringify(
          {
            site: "https://yolo.aphrody.com",
            outputDir: emitDir,
            indexHtml: join(REPO_ROOT, emitDir, "index.html"),
            sections: ["/showcase", "/migration", "/templates", "/awesome"],
            ok: existsSync(join(REPO_ROOT, emitDir, "index.html")),
          },
          null,
          2,
        ),
      );
      return 0;
    }

    case "doctor": {
      const skillExists = existsSync(
        join(REPO_ROOT, "skills/yolo-ui-pro-max/SKILL.md"),
      );
      const agentSkillExists = existsSync(
        join(REPO_ROOT, ".agents/skills/yolo-ui-pro-max/SKILL.md"),
      );
      const pluginExists = existsSync(
        join(REPO_ROOT, "packages/yolo-ui-plugin/src/index.ts"),
      );
      const mcpExists = existsSync(
        join(REPO_ROOT, "packages/yolo-ui-mcp/src/index.ts"),
      );
      const catalogExists = existsSync(
        join(REPO_ROOT, "catalog/conversion.toml"),
      );
      const fleetExists = existsSync(join(REPO_ROOT, "fleet.json"));

      const status = {
        ok:
          skillExists &&
          agentSkillExists &&
          pluginExists &&
          mcpExists &&
          catalogExists &&
          fleetExists,
        bunVersion: Bun.version,
        components: {
          cli: "scripts/ui.ts (bun ui / just ui)",
          plugin: "@yolo-rs/ui-plugin (packages/yolo-ui-plugin)",
          mcpServer: "@yolo-rs/ui-mcp (packages/yolo-ui-mcp)",
          skill:
            "yolo-ui-pro-max (skills/yolo-ui-pro-max & .agents/skills/yolo-ui-pro-max)",
        },
        counts: {
          domains: Object.keys(CSV_CONFIG).length,
          stacks: AVAILABLE_STACKS.length,
          themePresets: Object.keys(YOLO_THEME_PRESETS).length,
          omarchySubthemes: Object.keys(OMARCHY_SUBTHEMES).length,
          devtoolsEngines: Object.keys(DEVTOOLS_REGISTRY).length,
          mcpTools: MCP_TOOL_DEFINITIONS.length,
        },
        checks: {
          skillExists,
          agentSkillExists,
          pluginExists,
          mcpExists,
          catalogExists,
          fleetExists,
        },
      };
      console.log(JSON.stringify(status, null, 2));
      return status.ok ? 0 : 1;
    }

    default:
      console.error(`Unknown subcommand: ${subcommand}`);
      printHelp();
      return 1;
  }
}

if (import.meta.main) {
  process.exit(await runUnifiedUiCli());
}
