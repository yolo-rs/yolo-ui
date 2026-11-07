#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { join } from "node:path";
import { runIconsCli } from "./ui-icons";

// The MCP, plugin and skill modules load lazily so `bun ui icons` runs without the workspace packages installed.
async function loadUiCore() {
  const [mcp, plugin, core] = await Promise.all([
    import("@yolo-rs/ui-mcp"),
    import("@yolo-rs/ui-plugin"),
    import("../skills/yolo-ui-pro-max/scripts/core"),
  ]);
  return {
    executeUiTool: mcp.executeUiTool,
    MCP_TOOL_DEFINITIONS: mcp.MCP_TOOL_DEFINITIONS,
    REPO_ROOT: mcp.REPO_ROOT,
    runToolHostCli: mcp.runToolHostCli,
    startMcpStdioServer: mcp.startMcpStdioServer,
    DEVTOOLS_REGISTRY: plugin.DEVTOOLS_REGISTRY,
    OMARCHY_SUBTHEMES: plugin.OMARCHY_SUBTHEMES,
    YOLO_THEME_PRESETS: plugin.YOLO_THEME_PRESETS,
    AVAILABLE_STACKS: core.AVAILABLE_STACKS,
    CSV_CONFIG: core.CSV_CONFIG,
  };
}

function printHelp(): void {
  console.log(`
Yolo UI Unified CLI (bun ui / just ui) — v0.8.0
One Command • One Plugin (@yolo-rs/ui-plugin) • One MCP (@yolo-rs/ui-mcp) • UI/UX Pro Max

Usage:
  bun ui search <query> [--domain <d>] [--stack <s>] [-n <max>]
  bun ui system <query> [-p "Project Name"] [--preset neutral|m3|fluent|apple|omarchy|hyprland|aphrody|chromium] [--format ascii|markdown|json]
  bun ui theme emit --preset <p> --target <t>|all [--out <dir>] [--audit]   (native yolo-emit; YOLO_EMIT_BIN overrides the binary)
  bun ui theme [--preset <p>] [--omarchy <subtheme>] [--target css|gtk|kde|cosmic|qml|winui|fluent|apple|omarchy|hyprland|dtcg]
  bun ui migrate <file.tsx|snippet> [--target dioxus|wgpu|egui|cosmic|gtk|qml|all] [--preset <p>]
  bun ui contrast <fg-hex> <bg-hex>
  bun ui devtools [dioxus|gpui|egui|iced|cosmic|bevy|tauri|gtk4|qt6|shadcn-tailwind]
  bun ui awesome [query]
  bun ui catalog [query]
  bun ui workbenches [filter]
  bun ui fleet [query] [--stack <owner>]
  bun ui mcp [list | call <tool> '<json>' | serve]
  bun ui icons fetch|build|search|favicon|verify|optimize [...]   (bun ui icons help)
  bun ui doctor
`);
}

/** Locates the yolo-emit binary: `YOLO_EMIT_BIN`, then the Cargo target directory. */
function resolveYoloEmit(): string[] {
  const fromEnv = process.env.YOLO_EMIT_BIN;
  if (fromEnv) return [fromEnv];
  const exe = process.platform === "win32" ? "yolo-emit.exe" : "yolo-emit";
  const targetDir = process.env.CARGO_TARGET_DIR ?? join(REPO_ROOT, "target");
  for (const profile of ["release", "debug"]) {
    const candidate = join(targetDir, profile, exe);
    if (existsSync(candidate)) return [candidate];
  }
  return ["cargo", "run", "--quiet", "--locked", "-p", "yolo-emit", "--"];
}

/** `bun ui theme emit`: forwards to the Rust emitter so every target comes from yolo-tokens. */
async function runYoloEmit(
  positional: string[],
  flags: Record<string, string | boolean>,
): Promise<number> {
  const args: string[] = [];
  const preset =
    typeof flags.preset === "string" ? flags.preset : positional[0];
  if (preset) args.push("--preset", preset);
  for (const key of ["seed", "variant", "contrast", "target", "out"]) {
    const value = flags[key];
    if (typeof value === "string") args.push(`--${key}`, value);
  }
  if (flags.t && typeof flags.t === "string") args.push("--target", flags.t);
  if (flags.audit) args.push("--audit");
  if (flags["list-targets"]) args.push("--list-targets");
  const proc = Bun.spawn([...resolveYoloEmit(), ...args], {
    cwd: REPO_ROOT,
    stdout: "inherit",
    stderr: "inherit",
  });
  return await proc.exited;
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
  if (subcommand === "icons") {
    return await runIconsCli(rest);
  }
  const {
    executeUiTool,
    MCP_TOOL_DEFINITIONS,
    REPO_ROOT,
    runToolHostCli,
    startMcpStdioServer,
    DEVTOOLS_REGISTRY,
    OMARCHY_SUBTHEMES,
    YOLO_THEME_PRESETS,
    AVAILABLE_STACKS,
    CSV_CONFIG,
  } = await loadUiCore();
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
      if (positional[0] === "emit") {
        return await runYoloEmit(positional.slice(1), flags);
      }
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
