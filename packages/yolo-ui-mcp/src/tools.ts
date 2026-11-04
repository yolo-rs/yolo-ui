import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import {
  AVAILABLE_STACKS,
  CSV_CONFIG,
  search,
  searchStack,
} from "../../../skills/yolo-ui-pro-max/scripts/core";
import {
  formatAsciiBox,
  generateDesignSystem,
} from "../../../skills/yolo-ui-pro-max/scripts/design-system";
import {
  computeContrast,
  DEVTOOLS_REGISTRY,
  type DevtoolsEngineId,
  type EmitTarget,
  emitThemeCode,
  type MigrateTarget,
  migrateTsxToRust,
  normalizePresetId,
  OMARCHY_SUBTHEMES,
  resolveThemeTokens,
  YOLO_THEME_PRESETS,
} from "@yolo-rs/ui-plugin";

function findWorkspaceRoot(start: string): string | undefined {
  let current = resolve(start);
  for (let depth = 0; depth < 8; depth++) {
    if (
      existsSync(join(current, "Cargo.toml")) &&
      existsSync(join(current, "packages/yolo-ui-mcp/src/index.ts"))
    ) {
      return current;
    }
    const parent = resolve(current, "..");
    if (parent === current) break;
    current = parent;
  }
  return undefined;
}

export const REPO_ROOT =
  findWorkspaceRoot(process.cwd()) ??
  findWorkspaceRoot(dirname(process.execPath)) ??
  resolve(import.meta.dir, "../../..");

export interface McpToolDefinition {
  name: string;
  description: string;
  inputSchema: {
    type: "object";
    properties: Record<string, unknown>;
    required?: string[];
  };
}

export interface CraftWorkbenchInfo {
  id: string;
  title: string;
  domain: string;
  primaryEngine: string;
  secondaryEngines: string[];
  description: string;
}

export interface StorytellerModuleInfo {
  id: string;
  title: string;
  category: string;
  description: string;
}

export const CRAFT_WORKBENCHES: CraftWorkbenchInfo[] = [
  {
    id: "CadCraft",
    title: "CADCraft",
    domain: "Parametric 3D CAD & BREP Solid Modeling",
    primaryEngine: "bevy",
    secondaryEngines: ["egui", "gpui"],
    description:
      "BREP kernel viewport, sketch constraint solver, and high-density property tree.",
  },
  {
    id: "MeshCraft",
    title: "MeshCraft",
    domain: "Digital Sculpting, Retopology & UV Unwrapping",
    primaryEngine: "bevy",
    secondaryEngines: ["egui", "dioxus"],
    description:
      "WGPU compute brush pipeline, radial pie menus, and PBR shader ball preview.",
  },
  {
    id: "PixelCraft",
    title: "PixelCraft",
    domain: "Raster Painting, Photo Compositing & Pixel Art",
    primaryEngine: "gpui",
    secondaryEngines: ["iced", "egui"],
    description:
      "120fps tiled GPU canvas, non-destructive adjustment layers, and color wheel dock.",
  },
  {
    id: "VectorCraft",
    title: "VectorCraft",
    domain: "Bezier Vector Illustration & Icon Design",
    primaryEngine: "dioxus",
    secondaryEngines: ["gpui", "iced"],
    description:
      "AnyRender Vello/Skia GPU vector paths, boolean pathfinder, and SVG/DTCG exporter.",
  },
  {
    id: "CutCraft",
    title: "CutCraft",
    domain: "Non-Linear Video Editing (NLE) & Color Grading",
    primaryEngine: "gpui",
    secondaryEngines: ["egui", "tauri"],
    description:
      "Multi-track magnetic timeline, hardware-decoded scopes, and LUT node graph.",
  },
  {
    id: "MotionCraft",
    title: "MotionCraft",
    domain: "2D/3D Motion Graphics & Keyframe Animation",
    primaryEngine: "egui",
    secondaryEngines: ["bevy", "dioxus"],
    description:
      "Cubic bezier dope sheet, curve editor, and spring physicschoreographer.",
  },
  {
    id: "WaveCraft",
    title: "WaveCraft",
    domain: "Digital Audio Workstation (DAW) & Mastering",
    primaryEngine: "iced",
    secondaryEngines: ["egui", "gpui"],
    description:
      "Iced_Audio DSP knobs, faders, FFT spectrum analyzer, and low-latency mixer.",
  },
  {
    id: "TuneCraft",
    title: "TuneCraft",
    domain: "Modular Synthesizer, Tracker & MIDI Sequencer",
    primaryEngine: "iced",
    secondaryEngines: ["egui", "bevy"],
    description:
      "Patch cable modular rack, piano roll, and microtonal scale matrix.",
  },
  {
    id: "LevelCraft",
    title: "LevelCraft",
    domain: "Game World Editor, Tilemap & ECS Scene Builder",
    primaryEngine: "bevy",
    secondaryEngines: ["egui", "gpui"],
    description:
      "Bevy ECS live entity hierarchy, gizmo transform tools, and navmesh baker.",
  },
  {
    id: "NodeCraft",
    title: "NodeCraft",
    domain: "Visual Shader, Material & Procedural Graph Studio",
    primaryEngine: "egui",
    secondaryEngines: ["bevy", "gpui"],
    description:
      "Directed acyclic graph (DAG) canvas with live WGSL/GLSL node previews.",
  },
  {
    id: "SheetCraft",
    title: "SheetCraft",
    domain: "Reactive Data Grid, BI Formulas & Pivot Analytics",
    primaryEngine: "gpui",
    secondaryEngines: ["dioxus", "tauri"],
    description:
      "VirtualList 1M-row GPU spreadsheet, DuckDB/SQLite query bar, and chart tiles.",
  },
  {
    id: "DeckCraft",
    title: "DeckCraft",
    domain: "Interactive Slide Decks & Live Code Presentations",
    primaryEngine: "dioxus",
    secondaryEngines: ["tauri", "gpui"],
    description:
      "Subsecond live RSX slide canvas, speaker notes teleprompter, and transitions.",
  },
  {
    id: "WordCraft",
    title: "WordCraft",
    domain: "Structured Technical Publishing & Markdown/Typst IDE",
    primaryEngine: "dioxus",
    secondaryEngines: ["cosmic", "gtk4"],
    description:
      "Distraction-free Zen editor, live bibliography, and multi-target PDF/HTML export.",
  },
];

export const STORYTELLER_MODULES: StorytellerModuleInfo[] = [
  {
    id: "PromptBox",
    title: "PromptBox",
    category: "Input & Conditioning",
    description:
      "Multi-modal prompt composer with negative prompt weights, token counter, and LoRA/embedding pills.",
  },
  {
    id: "VoiceSelector",
    title: "VoiceSelector",
    category: "Audio & Speech",
    description:
      "Prosody, emotion, pitch, and reference voice cloning selector with real-time waveform preview.",
  },
  {
    id: "StyleWheel",
    title: "StyleWheel",
    category: "Visual Direction",
    description:
      "Interactive 2D aesthetic compass blending lighting, era, palette, and rendering medium.",
  },
  {
    id: "CameraRig",
    title: "CameraRig",
    category: "Cinematography",
    description:
      "6-DoF virtual camera controller for focal length (14mm-200mm), aperture, dolly, and orbital paths.",
  },
  {
    id: "TimelineEditor",
    title: "TimelineEditor",
    category: "Sequencing",
    description:
      "Frame-accurate multi-track storyboard timeline with dialogue lip-sync, SFX, and camera cuts.",
  },
  {
    id: "NodeGraph",
    title: "NodeGraph",
    category: "Pipeline Orchestration",
    description:
      "Generative conditioning pipeline connecting diffusion, upscaler, ControlNet, and audio synthesis nodes.",
  },
  {
    id: "MediaViewer",
    title: "MediaViewer",
    category: "Inspection",
    description:
      "A/B wipe comparison viewport with 4K zoom/pan, histogram scopes, and alpha-channel checker.",
  },
  {
    id: "AssetBrowser",
    title: "AssetBrowser",
    category: "Library Management",
    description:
      "Tag-filtered virtual grid for takes, seeds, checkpoints, 3D meshes, and reference boards.",
  },
  {
    id: "ExportQueue",
    title: "ExportQueue",
    category: "Render & Delivery",
    description:
      "Batch render farm monitor with codec presets (ProRes, AV1, WebP), ETA, and VRAM telemetry.",
  },
  {
    id: "CharacterSheet",
    title: "CharacterSheet",
    category: "Continuity & Lore",
    description:
      "Consistent character bible storing turnaround views, voice print, outfit tokens, and narrative traits.",
  },
];

export interface AwesomeEntry {
  repo: string;
  stack: string;
  title: string;
  url: string;
  description: string;
}

export const CURATED_AWESOME_ENTRIES: AwesomeEntry[] = [
  {
    repo: "submodules/awesome-dioxus",
    stack: "dioxus",
    title: "Dioxus 0.8 + Blitz + Subsecond",
    url: "https://github.com/dioxuslabs/awesome-dioxus",
    description:
      "Official curated list of Dioxus components, hooks, fullstack templates, and Blitz WGPU renderers.",
  },
  {
    repo: "submodules/awesome-gpui",
    stack: "gpui",
    title: "Zed GPUI & Longbridge GPUI-Component",
    url: "https://github.com/zed-industries/awesome-gpui",
    description:
      "GPU-accelerated UI crates, 60+ Longbridge widgets, DockArea, VirtualList, and native desktop apps.",
  },
  {
    repo: "submodules/awesome-egui",
    stack: "egui",
    title: "egui + eframe + puffin_egui + egui_kittest",
    url: "https://github.com/emilk/egui",
    description:
      "Immediate-mode Rust UI ecosystem: egui_plot, egui_dock, egui_tiles, bevy_egui, puffin profiler, and kittest.",
  },
  {
    repo: "submodules/awesome-bevy",
    stack: "bevy",
    title: "Bevy Engine UI + bevy-inspector-egui + bevy_lunex",
    url: "https://github.com/bevyengine/awesome-bevy",
    description:
      "ECS-driven game & tool UI plugins, Taffy layout bridges, inspectors, and shader materials.",
  },
  {
    repo: "submodules/awesome-iced",
    stack: "iced-libcosmic",
    title: "Iced MVU + Iced_AW + Iced_Audio + System76 libcosmic",
    url: "https://github.com/iced-rs/awesome-iced",
    description:
      "Elm-inspired Rust GUI libraries, audio DSP widgets, Comet time-travel debugger, and COSMIC desktop applets.",
  },
  {
    repo: "submodules/awesome-tauri",
    stack: "tauri-v3",
    title: "Tauri v3 Alpha + Wry + Tao + CrabNebula DevTools",
    url: "https://github.com/tauri-apps/awesome-tauri",
    description:
      "Cross-platform desktop/mobile plugins, window-vibrancy Mica/Acrylic/Liquid Glass, CEF, and ACL security.",
  },
  {
    repo: "submodules/awesome-omarchy",
    stack: "omarchy",
    title: "Omarchy Opinionated Arch/Hyprland/TUI Ecosystem",
    url: "https://github.com/basecamp/omarchy",
    description:
      "23 harmonized desktop & terminal themes across Hyprland, Waybar, Walker, Ghostty, and Neovim.",
  },
  {
    repo: "submodules/awesome-hyprland",
    stack: "hyprland",
    title: "Hyprland Wayland Compositor & Hyprland Ecosystem",
    url: "https://github.com/hyprwm/Hyprland",
    description:
      "Wayland plugins, hyprlock, hypridle, hyprpaper, hyprtoolkit, AGS/Waybar bars, and shader overlays.",
  },
  {
    repo: "submodules/awesome-shadcn-ui",
    stack: "shadcn",
    title: "shadcn/ui + Radix Primitives + Tailwind v4 Registries",
    url: "https://github.com/birobirobiro/awesome-shadcn-ui",
    description:
      "Accessible copy-paste component registries, data tables, command palettes, charts, and theme generators.",
  },
  {
    repo: "submodules/awesome-tailwindcss",
    stack: "html-tailwind",
    title: "Tailwind CSS v4 + OKLCH Tokens + Container Queries",
    url: "https://github.com/aniftyco/awesome-tailwindcss",
    description:
      "CSS-first @theme design systems, typography plugins, fluid spacing utilities, and animation presets.",
  },
  {
    repo: "submodules/base-ui",
    stack: "base-ui",
    title: "MUI Base UI (@base-ui-components/react)",
    url: "https://github.com/mui/base-ui",
    description:
      "Unstyled accessible React primitives with data-* state attributes and render prop composition.",
  },
  {
    repo: "submodules/gtk4-rs",
    stack: "gtk4-rs",
    title: "GTK4-rs + libadwaita + Relm4",
    url: "https://github.com/gtk-rs/gtk4-rs",
    description:
      "Safe Rust bindings for GNOME GTK 4.16, libadwaita 1.6 adaptive widgets, and Relm4 MVU.",
  },
  {
    repo: "submodules/cxx-qt",
    stack: "cxx-qt",
    title: "KDAB cxx-qt + Qt6 QML + KDE Kirigami",
    url: "https://github.com/KDAB/cxx-qt",
    description:
      "Safe Rust/C++ bridge exposing Rust structs as Qt6 QObjects and QML singletons.",
  },
];

export const MCP_TOOL_DEFINITIONS: McpToolDefinition[] = [
  {
    name: "ui_search",
    description:
      "Search the UI/UX Pro Max intelligence catalog across all 13 domains (ux, style, color, typography, product, landing, chart, gsap, react, web, icons, google-fonts, brand) or all 34 stacks (dioxus, gpui, egui, iced-libcosmic, bevy, tauri-v3, gtk4-rs, cxx-qt, shadcn, base-ui, html-tailwind, omarchy, hyprland, m3, etc.).",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description:
            "Keywords to search for (e.g. 'dock virtual list gpu', 'glassmorphism dark', 'contrast focus').",
        },
        domain: {
          type: "string",
          description:
            "Optional domain name (ux, style, color, typography, product, landing, chart, gsap, react, web, icons, google-fonts, brand).",
        },
        stack: {
          type: "string",
          description:
            "Optional stack name (dioxus, gpui, egui, iced-libcosmic, bevy, tauri-v3, gtk4-rs, cxx-qt, base-ui, shadcn, html-tailwind, omarchy, hyprland, m3, etc.).",
        },
        maxResults: {
          type: "number",
          description: "Maximum number of results to return (default: 5).",
        },
      },
      required: ["query"],
    },
  },
  {
    name: "ui_design_system",
    description:
      "Generate a complete UI/UX Pro Max Design System (pattern, style, colors, typography, key effects, anti-patterns, checklist, and dials) bound to any of the 8 YoloThemePresets (neutral, m3, fluent, apple, omarchy, hyprland, aphrody, chromium).",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "Product/application description or domain query.",
        },
        projectName: {
          type: "string",
          description: "Project or application display name.",
        },
        preset: {
          type: "string",
          description:
            "YoloThemePreset: neutral | m3 | fluent | apple | omarchy | hyprland | aphrody.",
        },
        format: {
          type: "string",
          enum: ["ascii", "markdown", "json"],
          description: "Output format (default: markdown).",
        },
        variance: { type: "number", description: "Design variance dial 1-10." },
        motion: { type: "number", description: "Motion intensity dial 1-10." },
        density: { type: "number", description: "Layout density dial 1-10." },
      },
      required: ["query"],
    },
  },
  {
    name: "ui_theme_emit",
    description:
      "Emit native theme code for any of the 8 YoloThemePresets (and any of the 23 Omarchy sub-themes) targeting css, gtk, kde, cosmic, qml, winui, fluent, apple, omarchy, hyprland, or dtcg.",
    inputSchema: {
      type: "object",
      properties: {
        preset: {
          type: "string",
          description:
            "Theme preset: neutral | m3 | fluent | apple | omarchy | hyprland | aphrody (default: neutral).",
        },
        omarchySubtheme: {
          type: "string",
          description:
            "Optional Omarchy sub-theme (e.g. tokyo-night, catppuccin, gruvbox, kanagawa, nord, rose-pine, cyberpunk, oxocarbon).",
        },
        target: {
          type: "string",
          description:
            "Target format: css | gtk | kde | cosmic | qml | winui | fluent | apple | omarchy | hyprland | dtcg (default: css).",
        },
      },
    },
  },
  {
    name: "ui_migrate",
    description:
      "Migrate a React / Shadcn / Radix / Base UI / Tailwind TSX snippet or file path to native Rust UI code (dioxus, wgpu, egui, cosmic, gtk, qml).",
    inputSchema: {
      type: "object",
      properties: {
        source: {
          type: "string",
          description:
            "Inline TSX source code or a relative/absolute path to a .tsx file.",
        },
        target: {
          type: "string",
          description:
            "Target Rust engine: dioxus | wgpu | egui | cosmic | gtk | qml | all (default: dioxus).",
        },
        preset: {
          type: "string",
          description:
            "Theme preset to bind: neutral | m3 | fluent | apple | omarchy | hyprland | aphrody.",
        },
      },
      required: ["source"],
    },
  },
  {
    name: "ui_contrast",
    description:
      "Compute WCAG 2.2 AA / AAA contrast ratio and APCA Lc readability score between any foreground and background hex color pair.",
    inputSchema: {
      type: "object",
      properties: {
        foreground: {
          type: "string",
          description: "Foreground hex color (e.g. '#F8FAFC').",
        },
        background: {
          type: "string",
          description: "Background hex color (e.g. '#0A0E17').",
        },
      },
      required: ["foreground", "background"],
    },
  },
  {
    name: "ui_devtools",
    description:
      "List, inspect, or get launch commands for the 10 unified stack DevTools (dioxus, gpui, egui, iced, cosmic, bevy, tauri, gtk4, qt6, shadcn-tailwind).",
    inputSchema: {
      type: "object",
      properties: {
        engine: {
          type: "string",
          description:
            "Optional engine ID to filter (dioxus, gpui, egui, iced, cosmic, bevy, tauri, gtk4, qt6, shadcn-tailwind).",
        },
      },
    },
  },
  {
    name: "ui_awesome",
    description:
      "Search across all bundled awesome-* repositories and curated ecosystems (dioxus, gpui, egui, bevy, iced, libcosmic, tauri, omarchy, hyprland, shadcn, base-ui, tailwindcss, gtk4, cxx-qt).",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description:
            "Optional search query or ecosystem keyword (e.g. 'gpui', 'egui', 'bevy', 'shadcn', 'hyprland').",
        },
        maxResults: {
          type: "number",
          description: "Max markdown lines/entries to return (default: 15).",
        },
      },
    },
  },
  {
    name: "ui_catalog_lookup",
    description:
      "Query catalog/conversion.toml for cross-toolkit component, token, layout, or lifecycle equivalents across web, dioxus, egui, cosmic, gtk4, qt6, gpui, iced, winui, kde, and hyprland.",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description:
            "Optional component, token, or toolkit name to filter (e.g. 'Dialog', 'Tabs', 'm3', 'hyprland', 'spacing').",
        },
      },
    },
  },
  {
    name: "ui_workbenches",
    description:
      "List and inspect the 13 yolo-craft creative workbenches (CadCraft..WordCraft) and 10 yolo-storyteller creative AI studio modules (PromptBox..CharacterSheet).",
    inputSchema: {
      type: "object",
      properties: {
        filter: {
          type: "string",
          description:
            "Optional search filter by workbench/module name, engine, or domain.",
        },
      },
    },
  },
  {
    name: "ui_fleet_status",
    description:
      "Inspect the 100+ submodule yolo-rs fleet inventory from fleet.json, optionally filtering by stack or submodule name.",
    inputSchema: {
      type: "object",
      properties: {
        stack: {
          type: "string",
          description:
            "Optional stack owner filter (e.g. dioxuslabs, longbridge, zed-industries, iced-rs, tauri-apps, hyprwm, bevyengine).",
        },
        query: {
          type: "string",
          description: "Optional submodule name substring filter.",
        },
      },
    },
  },
];

export async function executeUiTool(
  name: string,
  args: Record<string, unknown> = {},
): Promise<unknown> {
  switch (name) {
    case "ui_search": {
      const query = String(args.query ?? "").trim();
      const stack = args.stack ? String(args.stack).trim() : undefined;
      const domain = args.domain ? String(args.domain).trim() : undefined;
      const maxResults =
        typeof args.maxResults === "number" ? args.maxResults : 5;

      if (stack) {
        return {
          mode: "stack",
          availableStacks: AVAILABLE_STACKS,
          result: await searchStack(query, stack, maxResults),
        };
      }
      return {
        mode: "domain",
        availableDomains: Object.keys(CSV_CONFIG),
        result: await search(query, domain, maxResults),
      };
    }

    case "ui_design_system": {
      const query = String(args.query ?? "modern desktop studio").trim();
      const projectName = args.projectName
        ? String(args.projectName)
        : "Yolo UI App";
      const preset = normalizePresetId(
        args.preset ? String(args.preset) : "neutral",
      );
      const format = args.format ? String(args.format) : "markdown";
      const tokens = resolveThemeTokens(preset);
      const { design_system: ds, text } = await generateDesignSystem(
        `${query} ${tokens.brandId} ${tokens.name}`,
        {
          projectName,
          format: format === "ascii" ? "ascii" : "markdown",
          variance: typeof args.variance === "number" ? args.variance : null,
          motion: typeof args.motion === "number" ? args.motion : null,
          density: typeof args.density === "number" ? args.density : null,
        },
      );
      const formatted =
        format === "ascii"
          ? formatAsciiBox(ds)
          : format === "json"
            ? JSON.stringify({ designSystem: ds, yoloPreset: tokens }, null, 2)
            : `${text}\n### Bound YoloThemePreset (\`${tokens.brandId}\`)\n- **Name**: ${tokens.name}\n- **Background**: \`${tokens.backgroundPrimary}\` / \`${tokens.surfaceElevated}\`\n- **Text**: \`${tokens.textPrimary}\` (Muted: \`${tokens.textMuted}\`)\n- **Accent**: \`${tokens.accentPrimary}\` / \`${tokens.accentSecondary}\`\n- **Radius / Border / Blur**: \`${tokens.radiusPx}px\` / \`${tokens.borderWidthPx}px\` / \`${tokens.blurRadiusPx}px\`\n`;

      return {
        preset: tokens.brandId,
        tokens,
        designSystem: ds,
        formatted,
      };
    }

    case "ui_theme_emit": {
      const preset = args.preset ? String(args.preset) : "neutral";
      const omarchySubtheme = args.omarchySubtheme
        ? String(args.omarchySubtheme)
        : undefined;
      const target = (args.target ? String(args.target) : "css") as EmitTarget;
      const tokens = resolveThemeTokens(preset, omarchySubtheme);
      const code = emitThemeCode({ preset, omarchySubtheme, target });
      return {
        preset: tokens.brandId,
        subtheme: tokens.subtheme,
        target,
        tokens,
        availablePresets: Object.keys(YOLO_THEME_PRESETS),
        availableOmarchySubthemes: Object.keys(OMARCHY_SUBTHEMES),
        code,
      };
    }

    case "ui_migrate": {
      const rawSource = String(args.source ?? "");
      const targetArg = args.target ? String(args.target) : "dioxus";
      const preset = args.preset ? String(args.preset) : "neutral";

      let sourceCode = rawSource;
      const candidatePath = resolve(REPO_ROOT, rawSource);
      if (
        !rawSource.includes("\n") &&
        (rawSource.endsWith(".tsx") || rawSource.endsWith(".jsx")) &&
        existsSync(candidatePath)
      ) {
        sourceCode = readFileSync(candidatePath, "utf8");
      } else if (
        !rawSource.includes("\n") &&
        (rawSource.endsWith(".tsx") || rawSource.endsWith(".jsx")) &&
        existsSync(rawSource)
      ) {
        sourceCode = readFileSync(rawSource, "utf8");
      }

      const allTargets: MigrateTarget[] = [
        "dioxus",
        "wgpu",
        "egui",
        "cosmic",
        "gtk",
        "qml",
      ];
      if (targetArg === "all" || targetArg.includes(",")) {
        const requested =
          targetArg === "all"
            ? allTargets
            : (targetArg.split(",").map((t) => t.trim()) as MigrateTarget[]);
        const outputs: Record<string, MigrateResult> = {};
        for (const t of requested) {
          outputs[t] = migrateTsxToRust(sourceCode, t, preset);
        }
        return {
          mode: "multi-target",
          preset: normalizePresetId(preset),
          targets: outputs,
        };
      }

      return migrateTsxToRust(sourceCode, targetArg as MigrateTarget, preset);
    }

    case "ui_contrast": {
      const fg = String(args.foreground ?? "#F8FAFC");
      const bg = String(args.background ?? "#0A0E17");
      return computeContrast(fg, bg);
    }

    case "ui_devtools": {
      const engine = args.engine
        ? (String(args.engine).toLowerCase().trim() as DevtoolsEngineId)
        : undefined;
      if (engine && DEVTOOLS_REGISTRY[engine]) {
        return {
          engine,
          descriptor: DEVTOOLS_REGISTRY[engine],
        };
      }
      return {
        totalEngines: Object.keys(DEVTOOLS_REGISTRY).length,
        engines: DEVTOOLS_REGISTRY,
      };
    }

    case "ui_awesome": {
      const query = String(args.query ?? "")
        .toLowerCase()
        .trim();
      const maxResults =
        typeof args.maxResults === "number" ? args.maxResults : 15;

      const matchedCurated = CURATED_AWESOME_ENTRIES.filter((entry) => {
        if (!query) return true;
        return (
          entry.stack.toLowerCase().includes(query) ||
          entry.title.toLowerCase().includes(query) ||
          entry.description.toLowerCase().includes(query) ||
          entry.repo.toLowerCase().includes(query)
        );
      });

      const readmeHits: Array<{ repo: string; line: string }> = [];
      const awesomeDirs = [
        "submodules/awesome-dioxus",
        "submodules/awesome-gpui",
        "submodules/awesome-hyprland",
        "submodules/awesome-iced",
        "submodules/awesome-omarchy",
        "submodules/awesome-tauri",
      ];

      for (const relDir of awesomeDirs) {
        const readmePath = join(REPO_ROOT, relDir, "README.md");
        if (existsSync(readmePath)) {
          const text = readFileSync(readmePath, "utf8");
          const lines = text.split(/\r?\n/);
          for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed.startsWith("- [") && !trimmed.startsWith("* ["))
              continue;
            if (
              !query ||
              trimmed.toLowerCase().includes(query) ||
              relDir.includes(query)
            ) {
              readmeHits.push({ repo: relDir, line: trimmed });
              if (readmeHits.length >= maxResults) break;
            }
          }
        }
        if (readmeHits.length >= maxResults) break;
      }

      return {
        query: query || "*",
        curatedEcosystems: matchedCurated,
        submoduleReadmeLinks: readmeHits.slice(0, maxResults),
      };
    }

    case "ui_catalog_lookup": {
      const query = String(args.query ?? "")
        .toLowerCase()
        .trim();
      const catalogPath = join(REPO_ROOT, "catalog/conversion.toml");
      const rawToml = readFileSync(catalogPath, "utf8");
      const parsed = Bun.TOML.parse(rawToml) as Record<string, unknown>;

      if (!query) {
        return {
          catalogPath: "catalog/conversion.toml",
          sections: Object.keys(parsed),
          catalog: parsed,
        };
      }

      const filtered: Record<string, unknown> = {};
      for (const [sectionKey, sectionVal] of Object.entries(parsed)) {
        const serialized = JSON.stringify(sectionVal).toLowerCase();
        if (
          sectionKey.toLowerCase().includes(query) ||
          serialized.includes(query)
        ) {
          if (typeof sectionVal === "object" && sectionVal !== null) {
            const subMatches: Record<string, unknown> = {};
            for (const [subK, subV] of Object.entries(
              sectionVal as Record<string, unknown>,
            )) {
              if (
                subK.toLowerCase().includes(query) ||
                JSON.stringify(subV).toLowerCase().includes(query) ||
                sectionKey.toLowerCase().includes(query)
              ) {
                subMatches[subK] = subV;
              }
            }
            filtered[sectionKey] =
              Object.keys(subMatches).length > 0 ? subMatches : sectionVal;
          } else {
            filtered[sectionKey] = sectionVal;
          }
        }
      }

      return {
        query,
        matches: filtered,
      };
    }

    case "ui_workbenches": {
      const filter = String(args.filter ?? "")
        .toLowerCase()
        .trim();
      const workbenches = CRAFT_WORKBENCHES.filter(
        (w) =>
          !filter ||
          w.id.toLowerCase().includes(filter) ||
          w.domain.toLowerCase().includes(filter) ||
          w.primaryEngine.toLowerCase().includes(filter) ||
          w.description.toLowerCase().includes(filter),
      );
      const storytellerModules = STORYTELLER_MODULES.filter(
        (m) =>
          !filter ||
          m.id.toLowerCase().includes(filter) ||
          m.category.toLowerCase().includes(filter) ||
          m.description.toLowerCase().includes(filter),
      );
      return {
        craftWorkbenchCount: workbenches.length,
        storytellerModuleCount: storytellerModules.length,
        workbenches,
        storytellerModules,
      };
    }

    case "ui_fleet_status": {
      const fleetPath = join(REPO_ROOT, "fleet.json");
      const fleet = JSON.parse(readFileSync(fleetPath, "utf8")) as {
        generatedAt: string;
        repository: string;
        version: string;
        totalSubmodules: number;
        byStack: Record<string, number>;
        submodules: Array<{
          name: string;
          path: string;
          url: string;
          sha: string;
          stack: string;
          exists: boolean;
        }>;
      };

      const stackFilter = args.stack
        ? String(args.stack).toLowerCase().trim()
        : undefined;
      const queryFilter = args.query
        ? String(args.query).toLowerCase().trim()
        : undefined;

      const filteredSubmodules = fleet.submodules.filter((sub) => {
        if (stackFilter && !sub.stack.toLowerCase().includes(stackFilter))
          return false;
        if (
          queryFilter &&
          !sub.name.toLowerCase().includes(queryFilter) &&
          !sub.path.toLowerCase().includes(queryFilter)
        ) {
          return false;
        }
        return true;
      });

      return {
        repository: fleet.repository,
        version: fleet.version,
        totalSubmodules: fleet.totalSubmodules,
        byStack: fleet.byStack,
        matchedCount: filteredSubmodules.length,
        submodules: filteredSubmodules,
      };
    }

    default:
      throw new Error(`Unknown MCP tool: ${name}`);
  }
}

type MigrateResult = ReturnType<typeof migrateTsxToRust>;
