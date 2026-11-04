#!/usr/bin/env bash
# Tauri bundles the daemon as an external binary and its build script requires
# the triple-suffixed sidecar (target/release/sd-daemon-<host-triple>) to exist
# before `tauri dev` or `tauri build` can start. Cargo makes the rebuild a
# fast no-op when the daemon is unchanged, so running this every time is cheap.
set -euo pipefail

cd "$(dirname "$0")/.."

cargo build --release --bin sd-daemon --manifest-path ../../../../Cargo.toml \
	${CARGO_BUILD_TARGET:+--target "$CARGO_BUILD_TARGET"}

metadata="$(cargo metadata --no-deps --format-version 1 --manifest-path ../../../../Cargo.toml)"
target_dir="$(printf '%s' "$metadata" | bun -e 'console.log((await Bun.stdin.json()).target_directory)')"
if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
	triple="$CARGO_BUILD_TARGET"
	release_dir="$target_dir/$CARGO_BUILD_TARGET/release"
else
	triple="$(rustc -vV | sed -n 's/^host: //p')"
	release_dir="$target_dir/release"
fi

ext=""
case "$triple" in *windows*) ext=".exe" ;; esac

cp "$release_dir/sd-daemon$ext" "$release_dir/sd-daemon-$triple$ext"
echo "daemon sidecar ready: $release_dir/sd-daemon-$triple$ext"
