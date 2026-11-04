#!/usr/bin/env bun
// SPDX-License-Identifier: MIT OR Apache-2.0
// `yolo-fullstack dev | build | start | compile | docker | migrate | fonts` for a Next.js-style `app/` project.
import { resolve } from "node:path";
import { compileAppRouter, dockerAppRouter } from "../src/app/compile";
import { findAppProject, runAppCommand } from "../src/app/cli";
import { loadNextConfig } from "../src/config/next-config";
import { stripBundlerFlags } from "../src/config/turbopack";
import { formatMigrationReport, migrateNextProject } from "../src/migrate";

const HELP = `yolo-fullstack <command> [options]

Commands
  dev        Development server with rebuilds and RSC refresh
  build      Production build (three Bun.build passes) and static generation
  start      Serve a build with Bun.serve
  compile    Build and link one executable with the Bun runtime
  docker     Build and write a Docker context
  migrate    Report (and with --apply rewrite) a Next.js project for this framework
  config     Print the resolved next.config and the keys the build ignores

Options
  --port <n>        Port (dev, start)
  --outdir <dir>    Build directory (default dist)
  --no-prerender    Skip static generation
  --offline         next/font: fail on cache misses instead of fetching
  --target <t>      compile: Bun compile target (bun-linux-x64-musl, ...)
  --apply           migrate: write package.json, tsconfig.json and .gitignore
  --turbopack       Accepted for compatibility; Bun.build is the bundler
`;

function flag(argv: string[], name: string): string | undefined {
  const i = argv.indexOf(name);
  if (i === -1) return undefined;
  const value = argv[i + 1];
  argv.splice(i, value === undefined || value.startsWith("--") ? 1 : 2);
  return value;
}

function bool(argv: string[], name: string): boolean {
  const i = argv.indexOf(name);
  if (i === -1) return false;
  argv.splice(i, 1);
  return true;
}

export async function main(args: string[]): Promise<number> {
  const stripped = stripBundlerFlags(args);
  const argv = [...stripped.argv];
  const command = argv.shift();
  if (!command || command === "--help" || command === "-h") {
    console.log(HELP);
    return command ? 0 : 1;
  }
  const root = resolve(flag(argv, "--root") ?? process.cwd());
  const port = flag(argv, "--port") ?? flag(argv, "-p");
  const outdir = flag(argv, "--outdir");
  const prerender = !bool(argv, "--no-prerender");
  const offline = bool(argv, "--offline");
  const apply = bool(argv, "--apply");
  const target = flag(argv, "--target");

  if (command === "migrate") {
    const report = await migrateNextProject(root, { apply });
    console.log(formatMigrationReport(report));
    return report.errors.length ? 1 : 0;
  }
  if (command === "config") {
    const config = await loadNextConfig(root);
    console.log(
      JSON.stringify(
        {
          file: config.file,
          basePath: config.basePath,
          assetPrefix: config.assetPrefix,
          images: config.images,
          turbopack: config.turbopack,
          rewrites: config.rewrites,
          redirects: config.redirects,
          headers: config.headers,
          unsupported: config.unsupported,
        },
        null,
        2,
      ),
    );
    return 0;
  }
  const project = findAppProject(root);
  if (!project) {
    console.error(`yolo-fullstack: no app/ or src/app/ directory in ${root}`);
    return 1;
  }
  project.label = "yolo-fullstack";
  if (command === "dev" || command === "build" || command === "start")
    return runAppCommand(command, project, {
      port: port ? Number(port) : undefined,
      outdir,
      prerender,
      offline,
      turbopack: stripped.turbopack,
    });
  if (command === "compile") {
    const report = await compileAppRouter({
      project,
      outDir: outdir ? resolve(root, outdir) : undefined,
      target,
      prerender,
    });
    console.log(
      `yolo-fullstack compile: ${report.outfile} (${report.bytes} bytes, ${report.files} files, ${report.target}) in ${report.ms} ms`,
    );
    return 0;
  }
  if (command === "docker") {
    const report = await dockerAppRouter({ project, prerender });
    console.log(`yolo-fullstack docker: ${report.contextDir} (${report.files.join(", ")}) in ${report.ms} ms`);
    return 0;
  }
  console.error(`yolo-fullstack: unknown command ${command}\n${HELP}`);
  return 1;
}

if (import.meta.main) {
  main(process.argv.slice(2)).then(
    code => {
      if (code !== 0) process.exit(code);
    },
    (error: unknown) => {
      console.error(error instanceof Error ? (error.stack ?? error.message) : String(error));
      process.exit(1);
    },
  );
}
