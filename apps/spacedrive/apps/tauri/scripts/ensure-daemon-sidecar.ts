#!/usr/bin/env bun

import { copyFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDir, "..");
const workspaceRoot = resolve(appRoot, "../../../../");
const manifestPath = join(workspaceRoot, "Cargo.toml");

function run(command: string, args: string[]) {
	const result = spawnSync(command, args, { cwd: workspaceRoot, stdio: "inherit" });
	if (result.status !== 0) process.exit(result.status ?? 1);
}

const target = process.env.CARGO_BUILD_TARGET;
run("cargo", [
	"build",
	"--release",
	"--package",
	"sd-core",
	"--bin",
	"sd-daemon",
	"--manifest-path",
	manifestPath,
	...(target ? ["--target", target] : []),
]);

const metadata = spawnSync(
	"cargo",
	["metadata", "--no-deps", "--format-version", "1", "--manifest-path", manifestPath],
	{ cwd: workspaceRoot, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] },
);
if (metadata.status !== 0) process.exit(metadata.status ?? 1);

const { target_directory: targetDirectory } = JSON.parse(metadata.stdout) as {
	target_directory: string;
};
const host = target
	? target
	: spawnSync("rustc", ["-vV"], { encoding: "utf8" }).stdout
			.split(/\r?\n/)
			.find((line: string) => line.startsWith("host: "))
			?.slice("host: ".length);
if (!host) throw new Error("Could not determine the Rust target triple");

const releaseDir = join(targetDirectory, ...(target ? [target] : []), "release");
const extension = host.includes("windows") ? ".exe" : "";
const source = join(releaseDir, `sd-daemon${extension}`);
const sidecar = join(releaseDir, `sd-daemon-${host}${extension}`);
if (!existsSync(source)) throw new Error(`Built daemon not found at ${source}`);
copyFileSync(source, sidecar);
console.log(`daemon sidecar ready: ${sidecar}`);
