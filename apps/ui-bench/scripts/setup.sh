#!/usr/bin/env bash
# Prepares apps/ui-bench on a Linux host: kit sources at pinned commits (read-only consumers),
# their own Bun installs, the lane node_modules links, and this app's single dev dependency.
# Usage: bash scripts/setup.sh            (clones kits into .kits/)
#        KITS_SRC=/path bash scripts/setup.sh   (links an existing kits checkout instead)
set -euo pipefail
APP="$(cd "$(dirname "$0")/.." && pwd)"
cd "$APP"

ROSEGRIFFON_SHA=8b296e74fa07147aef9bba10749902be49dac9e5
SHENRON_SHA=2e46577228fcefd71502d247a4e57f9d22325cd3

clone_at() { # name sha
  local dir=".kits/$1"
  if [ ! -d "$dir/.git" ]; then
    git init -q "$dir"
    git -C "$dir" remote add origin "https://github.com/aphrody-labs/$1.git"
  fi
  git -C "$dir" fetch -q --depth 1 origin "$2"
  git -C "$dir" checkout -q --detach "$2"
}

if [ -n "${KITS_SRC:-}" ]; then
  [ -e .kits ] || ln -s "$KITS_SRC" .kits
else
  mkdir -p .kits
  clone_at rosegriffon "$ROSEGRIFFON_SHA"
  clone_at shenron "$SHENRON_SHA"
fi

# @aphrody/shenron-ui depends on the shenron-config workspace, a git submodule.
git -C .kits/shenron submodule update --init --depth 1 packages/config

for k in rosegriffon:$ROSEGRIFFON_SHA shenron:$SHENRON_SHA; do
  name=${k%%:*}; want=${k#*:}
  got=$(git -C ".kits/$name" rev-parse HEAD)
  [ "$got" = "$want" ] || echo "warning: .kits/$name at $got, pinned $want" >&2
done

(cd .kits/rosegriffon && bun install --filter @rosegriffon/ui --frozen-lockfile)
(cd .kits/shenron && bun install --filter @aphrody/shenron-ui --frozen-lockfile)

# Lane entries resolve react, react-dom and Base UI through the kit's own install, so the page
# code and the kit components share one React instance.
ln -sfn ../../.kits/rosegriffon/node_modules lane-bun/rosegriffon/node_modules
ln -sfn ../../.kits/shenron/packages/ui/node_modules lane-bun/shenron/node_modules

bun install
echo "setup done: bun $(bun --version)"
