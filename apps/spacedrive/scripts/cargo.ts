#!/usr/bin/env bun

import { spawnSync } from 'node:child_process';
import { dirname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const spacedriveRoot = resolve(scriptDir, '..');
const workspaceManifest = resolve(spacedriveRoot, '..', '..', 'Cargo.toml');
const [command, ...forwardedArgs] = process.argv.slice(2);

if (command !== 'test') {
	console.error('Usage: bun apps/spacedrive/scripts/cargo.ts test [cargo test options]');
	process.exit(2);
}

const metadataResult = spawnSync(
	'cargo',
	['metadata', '--no-deps', '--format-version', '1', '--manifest-path', workspaceManifest],
	{ cwd: spacedriveRoot, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'] }
);

if (metadataResult.status !== 0) {
	process.exit(metadataResult.status ?? 1);
}

const metadata = JSON.parse(metadataResult.stdout) as {
	packages: Array<{ name: string; manifest_path: string }>;
};
const memberPrefix = `${spacedriveRoot}${sep}`.toLowerCase();
const packages = metadata.packages
	.filter((pkg) => resolve(pkg.manifest_path).toLowerCase().startsWith(memberPrefix))
	.map((pkg) => pkg.name);

if (packages.length === 0) {
	console.error(`No Spacedrive Cargo packages found under ${spacedriveRoot}`);
	process.exit(1);
}

const cargoArgs = [
	'test',
	'--manifest-path',
	workspaceManifest,
	...packages.flatMap((name) => ['-p', name]),
	...forwardedArgs
];
const result = spawnSync('cargo', cargoArgs, { cwd: spacedriveRoot, stdio: 'inherit' });
process.exit(result.status ?? 1);
