// SPDX-License-Identifier: Apache-2.0
// `dev | build | start` of an App Router project (an `app/` directory). Frameworks built on it pass
// their own Bun plugins for the browser bundle (`clientPlugins`) and their own labels.
import type { BunPlugin } from "bun";
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { buildApp, resolveAppDir } from "./build";
import { devApp } from "./dev";
import { serveApp } from "./host";
import { prerenderApp } from "./prerender";

export interface AppProject {
  root: string;
  appDir: string;
  outDir: string;
  port?: number;
  define?: Record<string, string>;
  /** Bun plugins of the browser bundle (CSS, assets, virtual modules). */
  clientPlugins?: BunPlugin[];
  /** Prefix of the console report (default `app`). */
  label?: string;
}

/** The App Router project at `cwd` (`app/` or `src/app/`), or `undefined` when there is none. */
export function findAppProject(cwd: string): AppProject | undefined {
  const appDir = resolveAppDir(cwd);
  return existsSync(join(cwd, appDir)) ? { root: cwd, appDir, outDir: join(cwd, "dist") } : undefined;
}

export interface AppCommandFlags {
  port?: number;
  outdir?: string;
  dev?: boolean;
  /** Skip static generation in `build`. */
  prerender?: boolean;
}

/** Build an App Router project into `options.outDir`, with its console report. */
export async function buildAppProject(
  project: AppProject,
  options: { outDir: string; dev?: boolean; prerender?: boolean },
): Promise<void> {
  const { outDir } = options;
  const label = project.label ?? "app";
  const started = performance.now();
  const { manifest } = await buildApp({
    root: project.root,
    appDir: project.appDir,
    outDir,
    dev: options.dev === true,
    define: project.define,
    clientPlugins: project.clientPlugins,
  });
  const built = Math.round(performance.now() - started);
  console.log(
    `${label} build: ${manifest.routes.length} routes, ${Object.keys(manifest.clientModules).length} client modules in ${built} ms`,
  );
  if (options.prerender !== false) {
    const { pages, dynamic } = await prerenderApp(outDir);
    for (const page of pages) console.log(`  ○ ${page}`);
    for (const page of dynamic) console.log(`  λ ${page.path}  (${page.reason})`);
  }
  console.log(`${label} build: ${outDir} in ${Math.round(performance.now() - started)} ms`);
}

/** Run `dev`, `build` or `start` for an App Router project; resolves to the exit code. */
export async function runAppCommand(
  command: "dev" | "build" | "start",
  project: AppProject,
  flags: AppCommandFlags,
): Promise<number> {
  const label = project.label ?? "app";
  const outDir = flags.outdir ? resolve(project.root, flags.outdir) : project.outDir;
  const port = flags.port ?? (Bun.env.PORT ? undefined : project.port);
  if (command === "dev") {
    const started = performance.now();
    const dev = await devApp({
      root: project.root,
      appDir: project.appDir,
      port,
      clientPlugins: project.clientPlugins,
    });
    console.log(`${label} dev: ${dev.url} (ready in ${Math.round(performance.now() - started)} ms)`);
    return 0;
  }
  if (command === "build") {
    await buildAppProject(project, { outDir, dev: flags.dev, prerender: flags.prerender });
    return 0;
  }
  if (!existsSync(join(outDir, "manifest.json")))
    throw new Error(`${label} start: no build in ${outDir} (run ${label} build)`);
  const server = await serveApp({ outDir, port });
  console.log(`${label} start: ${server.url}`);
  return 0;
}
