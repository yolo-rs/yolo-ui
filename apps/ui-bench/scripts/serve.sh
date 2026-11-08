#!/usr/bin/env bash
# Starts or stops both lane servers in the background with pid files under results/tmp/.
#   bash scripts/serve.sh start [path/to/ui-bench-leptos]   |   bash scripts/serve.sh stop
set -euo pipefail
APP="$(cd "$(dirname "$0")/.." && pwd)"
cd "$APP"
RUN="$APP/results/tmp"
mkdir -p "$RUN"

stop() {
  for name in bun leptos; do
    if [ -f "$RUN/$name.pid" ]; then
      kill "$(cat "$RUN/$name.pid")" 2>/dev/null || true
      rm -f "$RUN/$name.pid"
    fi
  done
}

wait_up() { # url
  for _ in $(seq 1 100); do
    curl -fs -o /dev/null "$1" && return 0
    sleep 0.2
  done
  echo "server at $1 did not start" >&2
  return 1
}

case "${1:-start}" in
  stop) stop ;;
  start)
    stop
    NODE_ENV=production PORT=4100 setsid nohup bun lane-bun/server.ts >"$RUN/bun.log" 2>&1 </dev/null &
    echo $! >"$RUN/bun.pid"
    wait_up http://127.0.0.1:4100/health
    BIN="${2:-$APP/dist/ui-bench-leptos}"
    if [ -x "$BIN" ]; then
      CSS_DIR="$APP/dist/leptos-css" PORT=4200 setsid nohup "$BIN" >"$RUN/leptos.log" 2>&1 </dev/null &
      echo $! >"$RUN/leptos.pid"
      wait_up http://127.0.0.1:4200/health
    else
      echo "no Leptos binary at $BIN; Bun lane only" >&2
    fi
    echo "servers up"
    ;;
esac
