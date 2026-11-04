#!/usr/bin/env bun

import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, "../../../../");
const manifestPath = join(workspaceRoot, "Cargo.toml");
const target = process.env.CARGO_BUILD_TARGET;
const result = spawnSync(
	"cargo",
	[
		"build",
		"--package",
		"sd-core",
		"--bin",
		"sd-daemon",
		"--manifest-path",
		manifestPath,
		...(target ? ["--target", target] : []),
	],
	{ cwd: workspaceRoot, stdio: "inherit" },
);

if (result.status !== 0) process.exit(result.status ?? 1);
