#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
// yolo-next CLI: dev, start, build, analyze, migrate, routes, config, fonts, version.

import { existsSync } from "node:fs";
import { join, resolve } from "node:path";

import { buildYoloNext } from "./build.ts";
import { loadNextConfig, resolveYoloConfig, serializeYoloConfig } from "./config.ts";
import { analyzeProject, migrateProject } from "./migrate.ts";
import { generateRouteTree, scanAppDir } from "./router.ts";
import type { ResolvedYoloConfig } from "./types.ts";

const USAGE = `usage: yolo-next <command> [options]

Commands:
  dev [dir] [--port <n>]                         Bun.serve with HMR (src/server.ts createYoloNextApp)
  start [dir] [--port <n>]                       production server
  build [dir] [--export] [--standalone]          three-pass Bun.build + manifests
  analyze <dir> [--json]                         route manifest and feature counts
  migrate <dir> [--out <dir>] [--dry-run] [--rewrite-imports]
  routes <dir> [--format json|bun|tanstack|dioxus|leptos|tree]
  config <dir> [--json|--ts]                     resolved Yolo config
  fonts <file>                                   next/font requests of a module
  version

Exit status: 0 success, 1 errors, 2 usage.`;

/** Parsed command line. */
export interface ParsedArgs {
  readonly command: string | undefined;
  readonly positional: string[];
  readonly flags: Record<string, string | boolean>;
}

const VALUE_FLAGS = new Set(["--port", "--out", "--format", "--hostname", "--target"]);

/** Parses argv into command, positionals and flags (`--flag value` for the known value flags). */
export function parseArgs(argv: ReadonlyArray<string>): ParsedArgs {
  const positional: string[] = [];
  const flags: Record<string, string | boolean> = {};
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i] ?? "";
    if (!arg.startsWith("--")) {
      positional.push(arg);
      continue;
    }
    const eq = arg.indexOf("=");
    if (eq !== -1) {
      flags[arg.slice(2, eq)] = arg.slice(eq + 1);
      continue;
    }
    const next = argv[i + 1];
    if (VALUE_FLAGS.has(arg) && next !== undefined && !next.startsWith("--")) {
      flags[arg.slice(2)] = next;
      i += 1;
    } else {
      flags[arg.slice(2)] = true;
    }
  }
  const [command, ...rest] = positional;
  return { command, positional: rest, flags };
}

interface ServerModule {
  createYoloNextApp?: (options: { root: string; config: ResolvedYoloConfig; dev?: boolean; port?: number }) => { listen?: (options?: { port?: number }) => unknown; fetch?: (request: Request) => Promise<Response> };
  default?: unknown;
}

async function loadConfig(root: string): Promise<ResolvedYoloConfig> {
  const loaded = await loadNextConfig(root);
  return resolveYoloConfig(loaded.config, { root });
}

async function importOptional<T>(file: string): Promise<T | undefined> {
  if (!existsSync(file)) return undefined;
  try {
    return (await import(Bun.pathToFileURL(file).href)) as T;
  } catch (error) {
    console.error(`cannot import ${file}: ${error instanceof Error ? error.message : String(error)}`);
    return undefined;
  }
}

async function serve(root: string, flags: Record<string, string | boolean>, dev: boolean): Promise<number> {
  const config = await loadConfig(root);
  const port = typeof flags.port === "string" ? Number(flags.port) : config.server.port;
  const server = await importOptional<ServerModule>(join(import.meta.dir, "server.ts"));
  const create = server?.createYoloNextApp ?? (typeof server?.default === "function" ? (server.default as ServerModule["createYoloNextApp"]) : undefined);
  if (!create) {
    console.error("server module not available (src/server.ts); run `bun run src/server.ts` of the project instead");
    return 1;
  }
  const app = create({ root, config, dev, port });
  if (typeof app.listen === "function") {
    app.listen({ port });
    console.log(`yolo-next ${dev ? "dev" : "start"} http://localhost:${port} (${root})`);
    return 0;
  }
  if (typeof app.fetch === "function") {
    Bun.serve({ port, hostname: config.server.hostname, fetch: (request) => app.fetch!(request) });
    console.log(`yolo-next ${dev ? "dev" : "start"} http://localhost:${port} (${root})`);
    return 0;
  }
  console.error("createYoloNextApp returned neither listen() nor fetch()");
  return 1;
}

/** Runs the CLI with the given argv; returns the exit code. */
export async function main(argv: ReadonlyArray<string>): Promise<number> {
  const { command, positional, flags } = parseArgs(argv);
  const dir = resolve(positional[0] ?? ".");
  switch (command) {
    case "version":
    case "--version":
    case "-V":
      console.log("yolo-next 0.1.0 (next 16.4.5 / bun 1.4.4)");
      return 0;
    case "dev":
      return serve(dir, flags, true);
    case "start":
      return serve(dir, flags, false);
    case "build": {
      const config = await loadConfig(dir);
      const result = await buildYoloNext({ root: dir, config, export: flags.export === true, standalone: flags.standalone === true, log: (message) => console.log(message) });
      for (const warning of result.warnings) console.log(`[${warning.severity}] ${warning.code}: ${warning.message}`);
      console.log(`build ${result.success ? "ok" : "failed"} in ${result.durationMs}ms: BUILD_ID ${result.buildId}, ${result.outputs.length} outputs, ${result.manifests.length} manifests, ${result.clientModules.length} client modules`);
      return result.success ? 0 : 1;
    }
    case "analyze": {
      if (!positional[0]) {
        console.error("analyze needs <dir>");
        return 2;
      }
      const analysis = analyzeProject(dir);
      if (flags.json === true) {
        console.log(JSON.stringify({ ...analysis, manifest: { ...analysis.manifest, tree: undefined } }, null, 2));
      } else {
        console.log(`project      ${analysis.name}`);
        console.log(`next         ${analysis.nextVersion ?? "?"}`);
        console.log(`config       ${analysis.configFile ?? "-"}`);
        for (const [key, value] of Object.entries(analysis.counts)) console.log(`${key.padEnd(22)} ${value}`);
        console.log(`diagnostics  ${analysis.diagnostics.length}`);
        for (const d of analysis.diagnostics) console.log(`  [${d.severity}] ${d.code}: ${d.message}`);
      }
      return analysis.diagnostics.some((d) => d.severity === "error") ? 1 : 0;
    }
    case "migrate": {
      if (!positional[0]) {
        console.error("migrate needs <dir>");
        return 2;
      }
      const out = typeof flags.out === "string" ? resolve(flags.out) : undefined;
      const result = await migrateProject({ root: dir, out, mode: out ? "copy" : "in-place", dryRun: flags["dry-run"] === true, rewriteImports: flags["rewrite-imports"] === true, log: (message) => console.log(message) });
      for (const change of result.changes) console.log(`${change.kind.padEnd(7)} ${change.path}${change.note ? `  (${change.note})` : ""}`);
      console.log(`${result.dryRun ? "planned" : "wrote"} ${result.changes.length} changes -> ${result.out}; pages ${result.counts.pages}, handlers ${result.counts.routeHandlers}, client components ${result.counts.useClientFiles}, diagnostics ${result.diagnostics.length}`);
      return result.diagnostics.some((d) => d.severity === "error") ? 1 : 0;
    }
    case "routes": {
      if (!positional[0]) {
        console.error("routes needs <dir>");
        return 2;
      }
      const manifest = scanAppDir(dir);
      const format = typeof flags.format === "string" ? flags.format : "json";
      switch (format) {
        case "json":
          console.log(JSON.stringify(manifest, null, 2));
          break;
        case "bun":
          for (const e of manifest.entries) console.log(`${e.kind.padEnd(13)} ${e.bunPattern}${e.slot ? `  @${e.slot}` : ""}`);
          break;
        case "tanstack":
          for (const e of manifest.entries) console.log(e.tanstackPath);
          break;
        case "dioxus":
          for (const e of manifest.entries) console.log(`#[route(${JSON.stringify(e.dioxusRoute)})] ${e.dioxusVariant}`);
          break;
        case "leptos":
          for (const e of manifest.entries) console.log(e.leptosPath);
          break;
        case "tree":
          console.log(generateRouteTree(manifest));
          break;
        default:
          console.error(`unknown format ${format}`);
          return 2;
      }
      return 0;
    }
    case "config": {
      if (!positional[0]) {
        console.error("config needs <dir>");
        return 2;
      }
      const config = await loadConfig(dir);
      if (flags.ts === true) console.log(serializeYoloConfig(config, { root: dir }));
      else console.log(JSON.stringify(config, (key, value: unknown) => (typeof value === "function" ? undefined : value), 2));
      return 0;
    }
    case "fonts": {
      const file = positional[0];
      if (!file) {
        console.error("fonts needs <file>");
        return 2;
      }
      const fonts = await importOptional<{ planFonts?: (source: string, file: string) => unknown }>(join(import.meta.dir, "fonts.ts"));
      if (!fonts || typeof fonts.planFonts !== "function") {
        console.log("fonts module not available");
        return 1;
      }
      const source = await Bun.file(resolve(file)).text();
      console.log(JSON.stringify(await fonts.planFonts(source, resolve(file)), null, 2));
      return 0;
    }
    case undefined:
    case "help":
    case "--help":
    case "-h":
      console.log(USAGE);
      return command === undefined ? 2 : 0;
    default:
      console.error(`unknown command ${command}\n\n${USAGE}`);
      return 2;
  }
}

if (import.meta.main) {
  main(Bun.argv.slice(2)).then(
    (code) => {
      if (code !== 0 || !["dev", "start"].includes(Bun.argv[2] ?? "")) process.exit(code);
    },
    (error: unknown) => {
      console.error(error instanceof Error ? (error.stack ?? error.message) : String(error));
      process.exit(1);
    },
  );
}
