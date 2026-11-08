// SPDX-License-Identifier: Apache-2.0
// Shared test helpers: fixture path, a PluginBuilder double, temp directories inside the package
// (so that `react` and friends resolve from the package's node_modules).

import type { OnLoadArgs, OnLoadResult, OnResolveArgs, OnResolveResult, PluginBuilder } from "bun";
import { mkdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";

/** The static Next.js 16 fixture project. */
export const FIXTURE: string = join(import.meta.dir, "..", "..", "..", "apps", "next-fixture");

/** Scratch directory inside the package (node_modules of the package are reachable from it). */
export const SCRATCH: string = join(import.meta.dir, "..", ".tmp");

/** Creates an empty scratch directory `<pkg>/.tmp/<name>-<random>` and returns its path. */
export function scratchDir(name: string): string {
  const dir = join(SCRATCH, `${name}-${Math.random().toString(36).slice(2, 8)}`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  return dir;
}

/** Removes a scratch directory. */
export function removeDir(dir: string): void {
  rmSync(dir, { recursive: true, force: true });
}

type ResolveCb = (args: OnResolveArgs) => OnResolveResult | undefined | Promise<OnResolveResult | undefined>;
type LoadCb = (args: OnLoadArgs) => OnLoadResult | Promise<OnLoadResult>;

/** A recorded PluginBuilder: runs the registered onResolve/onLoad callbacks like Bun would. */
export interface FakeBuild {
  readonly build: PluginBuilder;
  resolve(path: string, importer?: string): Promise<OnResolveResult | undefined>;
  load(path: string, namespace?: string): Promise<OnLoadResult>;
  counts(): { resolvers: number; loaders: number };
}

/** Minimal PluginBuilder double recording onResolve/onLoad callbacks. */
export function fakeBuild(target: "browser" | "bun" = "bun"): FakeBuild {
  const resolvers: Array<{ filter: RegExp; namespace?: string; cb: ResolveCb }> = [];
  const loaders: Array<{ filter: RegExp; namespace?: string; cb: LoadCb }> = [];
  const build = {
    config: { target, plugins: [], entrypoints: [] },
    onResolve(constraints: { filter: RegExp; namespace?: string }, cb: ResolveCb) {
      resolvers.push({ ...constraints, cb });
      return build;
    },
    onLoad(constraints: { filter: RegExp; namespace?: string }, cb: LoadCb) {
      loaders.push({ ...constraints, cb });
      return build;
    },
    onStart() {
      return build;
    },
    onEnd() {
      return build;
    },
    onBeforeParse() {
      return build;
    },
    module() {
      return build;
    },
  };
  return {
    build: build as unknown as PluginBuilder,
    async resolve(path: string, importer = join(FIXTURE, "app", "page.tsx")): Promise<OnResolveResult | undefined> {
      for (const r of resolvers) {
        if (!r.filter.test(path) || (r.namespace && r.namespace !== "file")) continue;
        const result = await r.cb({ path, importer, namespace: "file", resolveDir: dirname(importer), kind: "import-statement" });
        if (result) return result;
      }
      return undefined;
    },
    async load(path: string, namespace = "file"): Promise<OnLoadResult> {
      for (const l of loaders) {
        if (!l.filter.test(path) || (l.namespace ?? "file") !== namespace) continue;
        const result = await l.cb({ path, namespace, loader: "js", defer: () => Promise.resolve() } as unknown as OnLoadArgs);
        if (result) return result;
      }
      return undefined;
    },
    counts: () => ({ resolvers: resolvers.length, loaders: loaders.length }),
  };
}
