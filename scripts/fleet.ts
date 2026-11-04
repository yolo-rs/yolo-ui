import { join, resolve } from "node:path";

export type UpstreamStack =
  | "dioxuslabs"
  | "longbridge"
  | "zed-industries"
  | "iced-rs"
  | "tauri-apps"
  | "emilk"
  | "pop-os"
  | "gtk-rs"
  | "Relm4"
  | "KDAB"
  | "microsoft"
  | "material-components"
  | "material-foundation"
  | "JetBrains"
  | "macosui"
  | "vinceliuice"
  | "omacom"
  | "hyprwm"
  | "shadcn-ui"
  | "mui"
  | "radix-ui"
  | "tailwindlabs"
  | "bevyengine"
  | "crabnebula-dev"
  | "EmbarkStudios"
  | "facebook"
  | "reactjs"
  | "expo"
  | "ecrindigital"
  | "jhugman"
  | "oxc-project";

export interface SubmoduleEntry {
  name: string;
  path: string;
  url: string;
  sha: string;
  stack: UpstreamStack;
  exists: boolean;
}

export interface FleetInventory {
  generatedAt: string;
  repository: string;
  version: string;
  totalSubmodules: number;
  byStack: Record<UpstreamStack, number>;
  submodules: SubmoduleEntry[];
}

const ROOT_DIR = resolve(import.meta.dir, "..");
const SUBMODULES_DIR = join(ROOT_DIR, "submodules");
const GITMODULES_PATH = join(ROOT_DIR, ".gitmodules");
const FLEET_JSON_PATH = join(ROOT_DIR, "fleet.json");

export const ECOSYSTEM_REPOS: Record<UpstreamStack, string[]> = {
  "zed-industries": ["gpui-fast", "zed", "zed-fonts", "font-kit", "scap"],
  longbridge: [
    "gpui-kit",
    "gpui-component",
    "rust-i18n",
    "nsh",
    "longbridge",
    "autocorrect",
    "async-dispatcher",
    "wef",
    "gpui-mobile",
    "create-gpui-app",
    "awesome-gpui",
  ],
  "iced-rs": [
    "iced",
    "iced_aw",
    "iced-aw",
    "iced_audio",
    "iced-audio",
    "iced-book",
    "iced-opengl",
    "iced-rfcs",
    "cryoglyph",
    "comet",
    "awesome-iced",
    "mdbook-iced",
  ],
  "tauri-apps": [
    "tauri",
    "wry",
    "tao",
    "muda",
    "tray-icon",
    "Cargo-mobile2",
    "cargo-mobile2",
    "plugins-workspace",
    "tauri-plugin",
    "webkit2gtk",
    "javascriptcore",
    "soup",
    "dpi",
    "global-hotkey",
    "accelerator",
    "window-vibrancy",
    "window-shadows",
    "raw-window-handle",
    "tauri-docs",
    "tauri-bindgen",
    "tauri-vscode",
    "create-tauri-app",
    "tauri-action",
    "awesome-tauri",
    "velato",
  ],
  emilk: ["egui", "awesome-egui"],
  "pop-os": ["libcosmic"],
  "gtk-rs": ["gtk4-rs"],
  Relm4: ["relm4", "Relm4"],
  KDAB: ["cxx-qt"],
  microsoft: [
    "fluentui",
    "fluentui-system-icons",
    "winui-gallery",
    "WinUI-Gallery",
  ],
  "material-components": ["material-web"],
  "material-foundation": ["material-tokens", "material-color-utilities"],
  JetBrains: ["compose-multiplatform"],
  macosui: ["macos-ui", "macos_ui"],
  vinceliuice: [
    "whitesur-gtk-theme",
    "WhiteSur-gtk-theme",
    "mactahoe-gtk-theme",
    "MacTahoe-gtk-theme",
    "whitesur-icon-theme",
    "WhiteSur-icon-theme",
  ],
  omacom: ["omarchy", "omarchy-site", "awesome-omarchy"],
  hyprwm: [
    "hyprland",
    "Hyprland",
    "hyprland-website",
    "hyprtoolkit",
    "hyprqt6engine",
    "hyprlock",
    "hyprlauncher",
    "hyprperks-dotfiles",
    "awesome-hyprland",
    "wayle",
    "caelestia-shell",
    "noctalia",
    "noctalia-shell",
  ],
  "shadcn-ui": ["shadcn-ui", "ui", "awesome-shadcn-ui"],
  mui: ["base-ui"],
  "radix-ui": ["radix-primitives", "primitives"],
  tailwindlabs: ["tailwindcss", "awesome-tailwindcss"],
  bevyengine: ["bevy", "awesome-bevy", "bevy-inspector-egui"],
  "crabnebula-dev": ["tauri-devtools", "crabnebula-devtools", "devtools"],
  EmbarkStudios: ["puffin"],
  facebook: ["react", "react-native"],
  reactjs: ["react-dev", "react.dev", "react-docgen"],
  expo: [
    "expo",
    "expo-skills",
    "eas-cli",
    "expo-device-hub",
    "expo-ibex",
    "ibex",
  ],
  ecrindigital: ["facetpack"],
  jhugman: ["uniffi-react-native", "uniffi-bindgen-react-native"],
  "oxc-project": ["oxc"],
  dioxuslabs: [
    "blitz",
    "dioxus",
    "taffy",
    "anyrender",
    "sdk",
    "markdown",
    "rsrpc",
    "warnings",
    "prettier-please",
    "accessibility-cli",
    "wasm-bindgen-wry",
    "bedina",
    "web-adapters",
    "include_mdbook",
    "playground",
    "rink",
    "example-projects",
    "awesome-dioxus",
    "deploy-action",
    "parley",
    "vello_svg",
    "winkin",
    "rustd",
    "docsite",
  ],
};

export const STACK_PATTERNS: Record<UpstreamStack, string[]> = ECOSYSTEM_REPOS;

const DIRECT_ORGS: UpstreamStack[] = [
  "emilk",
  "pop-os",
  "gtk-rs",
  "Relm4",
  "KDAB",
  "microsoft",
  "material-components",
  "material-foundation",
  "JetBrains",
  "macosui",
  "vinceliuice",
  "omacom",
  "hyprwm",
  "shadcn-ui",
  "mui",
  "radix-ui",
  "tailwindlabs",
  "bevyengine",
  "crabnebula-dev",
  "EmbarkStudios",
  "facebook",
  "reactjs",
  "expo",
  "ecrindigital",
  "jhugman",
  "oxc-project",
];

/* @__PURE__ */
export function categorizeSubmodule(name: string, url = ""): UpstreamStack {
  const lower = `${name} ${url}`.toLowerCase();
  const nameLower = name.toLowerCase();

  for (const org of DIRECT_ORGS) {
    for (const token of ECOSYSTEM_REPOS[org]) {
      if (
        nameLower === token.toLowerCase() ||
        lower.includes(`/${token.toLowerCase()}.git`)
      ) {
        return org;
      }
    }
  }

  for (const token of STACK_PATTERNS["zed-industries"]) {
    if (nameLower === token.toLowerCase()) {
      return "zed-industries";
    }
  }
  if (lower.includes("zed-industries")) {
    return "zed-industries";
  }

  for (const token of STACK_PATTERNS.longbridge) {
    if (lower.includes(token.toLowerCase())) {
      return "longbridge";
    }
  }

  for (const token of STACK_PATTERNS["iced-rs"]) {
    if (nameLower === token.toLowerCase() || nameLower.startsWith("iced")) {
      return "iced-rs";
    }
  }

  if (name === "wasm-bindgen-wry") {
    return "dioxuslabs";
  }

  for (const token of STACK_PATTERNS["tauri-apps"]) {
    if (nameLower === token.toLowerCase() || nameLower.startsWith("tauri")) {
      return "tauri-apps";
    }
  }

  return "dioxuslabs";
}

export async function parseGitmodules(): Promise<Map<string, string>> {
  const map = new Map<string, string>();
  const file = Bun.file(GITMODULES_PATH);
  if (!(await file.exists())) {
    return map;
  }
  const text = await file.text();
  let currentPath = "";
  let currentUrl = "";
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line.startsWith("[submodule")) {
      if (currentPath && currentUrl) {
        map.set(currentPath.replace(/\\/g, "/"), currentUrl);
      }
      currentPath = "";
      currentUrl = "";
    } else if (line.startsWith("path")) {
      const parts = line.split("=");
      currentPath = (parts[1] ?? "").trim();
    } else if (line.startsWith("url")) {
      const parts = line.split("=");
      currentUrl = (parts[1] ?? "").trim();
    }
  }
  if (currentPath && currentUrl) {
    map.set(currentPath.replace(/\\/g, "/"), currentUrl);
  }
  return map;
}

async function resolveSubmoduleSha(subPath: string): Promise<string> {
  const fullPath = join(ROOT_DIR, subPath);
  const proc = Bun.spawn(["git", "-C", fullPath, "rev-parse", "HEAD"], {
    stdout: "pipe",
    stderr: "ignore",
  });
  const out = await new Response(proc.stdout).text();
  const code = await proc.exited;
  if (code === 0 && out.trim().length >= 7) {
    return out.trim();
  }
  return "uninitialized";
}

export async function discoverSubmodules(): Promise<SubmoduleEntry[]> {
  const gitmodulesMap = await parseGitmodules();
  const discoveredNames = new Set<string>();

  for (const relPath of gitmodulesMap.keys()) {
    if (relPath.startsWith("submodules/")) {
      discoveredNames.add(relPath.slice("submodules/".length));
    }
  }

  const glob = new Bun.Glob("*");
  try {
    for await (const entry of glob.scan({
      cwd: SUBMODULES_DIR,
      onlyFiles: false,
    })) {
      if (!entry.startsWith(".")) {
        discoveredNames.add(entry);
      }
    }
  } catch {
    // Submodules directory may not exist yet
  }

  const sortedNames = Array.from(discoveredNames).sort((a, b) =>
    a.localeCompare(b),
  );

  const entries = await Promise.all(
    sortedNames.map(async (name): Promise<SubmoduleEntry> => {
      const relPath = `submodules/${name}`;
      const url =
        gitmodulesMap.get(relPath) ?? `https://github.com/yolo-rs/${name}.git`;
      const gitHeadFile = Bun.file(join(ROOT_DIR, relPath, "Cargo.toml"));
      const readmeFile = Bun.file(join(ROOT_DIR, relPath, "README.md"));
      const [hasCargo, hasReadme, sha] = await Promise.all([
        gitHeadFile.exists(),
        readmeFile.exists(),
        resolveSubmoduleSha(relPath),
      ]);
      const exists = hasCargo || hasReadme || sha !== "uninitialized";
      const stack = categorizeSubmodule(name, url);

      return {
        name,
        path: relPath,
        url,
        sha,
        stack,
        exists,
      };
    }),
  );

  return entries;
}

export async function buildInventory(): Promise<FleetInventory> {
  const submodules = await discoverSubmodules();
  const byStack: Record<UpstreamStack, number> = {
    dioxuslabs: 0,
    longbridge: 0,
    "zed-industries": 0,
    "iced-rs": 0,
    "tauri-apps": 0,
    emilk: 0,
    "pop-os": 0,
    "gtk-rs": 0,
    Relm4: 0,
    KDAB: 0,
    microsoft: 0,
    "material-components": 0,
    "material-foundation": 0,
    JetBrains: 0,
    macosui: 0,
    vinceliuice: 0,
    omacom: 0,
    hyprwm: 0,
    "shadcn-ui": 0,
    mui: 0,
    "radix-ui": 0,
    tailwindlabs: 0,
    bevyengine: 0,
    "crabnebula-dev": 0,
    EmbarkStudios: 0,
    facebook: 0,
    reactjs: 0,
    expo: 0,
    ecrindigital: 0,
    jhugman: 0,
    "oxc-project": 0,
  };

  for (const sub of submodules) {
    byStack[sub.stack] += 1;
  }

  return {
    generatedAt: new Date().toISOString(),
    repository: "https://github.com/yolo-rs/yolo-ui",
    version: "0.8.0-alpha.3",
    totalSubmodules: submodules.length,
    byStack,
    submodules,
  };
}

async function runStatus(): Promise<void> {
  const inv = await buildInventory();
  console.log(
    `=== Aphrody Labs Yolo UI Fleet Status (${inv.totalSubmodules} submodules) ===`,
  );
  console.log("Stack Breakdown:");
  for (const [stack, count] of Object.entries(inv.byStack)) {
    console.log(`  - ${stack.padEnd(20)}: ${count}`);
  }
  console.log("\nSubmodules:");
  for (const sub of inv.submodules) {
    const shortSha =
      sub.sha === "uninitialized" ? "uninit  " : sub.sha.slice(0, 8);
    console.log(
      `  [${sub.stack.padEnd(19)}] ${sub.path.padEnd(36)} ${shortSha} -> ${sub.url}`,
    );
  }
}

async function runSync(): Promise<void> {
  console.log("Syncing submodules (git submodule update --init --depth 1)...");
  const proc = Bun.spawn(
    ["git", "submodule", "update", "--init", "--depth", "1"],
    {
      cwd: ROOT_DIR,
      stdout: "inherit",
      stderr: "inherit",
    },
  );
  const exitCode = await proc.exited;
  if (exitCode !== 0) {
    process.exit(exitCode);
  }
}

async function runInventory(): Promise<void> {
  const inv = await buildInventory();
  await Bun.write(FLEET_JSON_PATH, `${JSON.stringify(inv, null, 2)}\n`);
  console.log(
    `Wrote fleet inventory (${inv.totalSubmodules} submodules) to ${FLEET_JSON_PATH}`,
  );
}

if (import.meta.main) {
  const rawCmd = Bun.argv[2] ?? "status";
  const cmd = rawCmd.replace(/^--+/, "");
  if (cmd === "status") {
    await runStatus();
  } else if (cmd === "sync") {
    await runSync();
  } else if (cmd === "inventory") {
    await runInventory();
  } else {
    console.error(
      `Unknown command: ${rawCmd}. Expected: status | sync | inventory (or --status | --sync | --inventory)`,
    );
    process.exit(1);
  }
}
