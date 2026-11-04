#!/usr/bin/env bun

import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

if (process.platform !== "darwin") {
	console.log("Skipping daemon entitlement fix (macOS only)");
	process.exit(0);
}

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDir, "..");
const workspaceRoot = resolve(appRoot, "../../../../");
const manifestPath = join(workspaceRoot, "Cargo.toml");
const metadata = spawnSync(
	"cargo",
	["metadata", "--no-deps", "--format-version", "1", "--manifest-path", manifestPath],
	{ cwd: workspaceRoot, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] },
);
if (metadata.status !== 0) process.exit(metadata.status ?? 1);

const { target_directory: targetDirectory } = JSON.parse(metadata.stdout) as {
	target_directory: string;
};
const bundle = join(targetDirectory, "release", "bundle", "macos", "Spacedrive.app");
const daemon = join(bundle, "Contents", "MacOS", "sd-daemon");
const entitlements = join(appRoot, "src-tauri", "DaemonEntitlements.plist");
if (!existsSync(daemon)) throw new Error(`Bundled daemon not found at ${daemon}`);
if (!existsSync(entitlements)) throw new Error(`Daemon entitlements not found at ${entitlements}`);

const result = spawnSync(
	"codesign",
	["--force", "--sign", "-", "--entitlements", entitlements, "--options", "runtime", daemon],
	{ stdio: "inherit" },
);
if (result.status !== 0) process.exit(result.status ?? 1);
console.log("Daemon re-signed with the Spacedrive entitlements");
