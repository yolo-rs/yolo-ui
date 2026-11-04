import { join, resolve } from "node:path";

export type UpstreamStack =
  "dioxuslabs" | "longbridge" | "zed-industries" | "iced-rs" | "tauri-apps";

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

const STACK_PATTERNS: Record<UpstreamStack, string[]> = {
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

/* @__PURE__ */
export function categorizeSubmodule(name: string, url = ""): UpstreamStack {
  const lower = `${name} ${url}`.toLowerCase();

  for (const token of STACK_PATTERNS["zed-industries"]) {
    if (name.toLowerCase() === token.toLowerCase()) {
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
    if (
      name.toLowerCase() === token.toLowerCase() ||
      name.toLowerCase().startsWith("iced")
    ) {
      return "iced-rs";
    }
  }

  if (name === "wasm-bindgen-wry") {
    return "dioxuslabs";
  }

  for (const token of STACK_PATTERNS["tauri-apps"]) {
    if (
      name.toLowerCase() === token.toLowerCase() ||
      name.toLowerCase().startsWith("tauri")
    ) {
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
        gitmodulesMap.get(relPath) ??
        `https://github.com/aphrody-labs/${name}.git`;
      const gitHeadFile = Bun.file(join(ROOT_DIR, relPath, "Cargo.toml"));
      const readmeFile = Bun.file(join(ROOT_DIR, relPath, "README.md"));
      const [hasCargo, hasReadme, sha] = await Promise.all([
        gitHeadFile.exists(),
        readmeFile.exists(),
        resolveSubmoduleSha(relPath),
      ]);
      const exists = hasCargo || hasReadme;
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
  };

  for (const sub of submodules) {
    byStack[sub.stack] += 1;
  }

  return {
    generatedAt: new Date().toISOString(),
    repository: "https://github.com/aphrody-labs/yolo-ui",
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
    console.log(`  - ${stack.padEnd(16)}: ${count}`);
  }
  console.log("\nSubmodules:");
  for (const sub of inv.submodules) {
    const shortSha =
      sub.sha === "uninitialized" ? "uninit  " : sub.sha.slice(0, 8);
    console.log(
      `  [${sub.stack.padEnd(14)}] ${sub.path.padEnd(36)} ${shortSha} -> ${sub.url}`,
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
  const cmd = Bun.argv[2] ?? "status";
  if (cmd === "status") {
    await runStatus();
  } else if (cmd === "sync") {
    await runSync();
  } else if (cmd === "inventory") {
    await runInventory();
  } else {
    console.error(
      `Unknown command: ${cmd}. Expected: status | sync | inventory`,
    );
    process.exit(1);
  }
}
