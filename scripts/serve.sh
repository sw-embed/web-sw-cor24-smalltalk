#!/usr/bin/env bash
# Dev server. Holds an exclusive lock on dist/ so a stray `trunk build`
# (or a second `serve`) cannot race the wasm-bindgen pipeline and leave
# dist/ with empty/missing artifacts — which manifests in the browser as
# SRI failures and the Yew app never booting (demos appear to "hang").
set -euo pipefail

PORT=9072
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

mkdir -p "$REPO_DIR/target"
# Lock lives outside dist/ because trunk wipes dist/ on every rebuild.
LOCK="$REPO_DIR/target/.trunk-dist.lock"
if ! mkdir "$LOCK" 2>/dev/null; then
  HOLDER="$(cat "$LOCK/pid" 2>/dev/null || echo unknown)"
  if [ "$HOLDER" != "unknown" ] && ! kill -0 "$HOLDER" 2>/dev/null; then
    echo "serve.sh: removing stale lock from pid $HOLDER" >&2
    rm -rf "$LOCK"
    mkdir "$LOCK"
  else
    echo "serve.sh: another trunk process (pid $HOLDER) holds $LOCK — refusing to share dist/" >&2
    exit 1
  fi
fi
echo $$ > "$LOCK/pid"
trap 'rm -rf "$LOCK"' EXIT INT TERM

cd "$REPO_DIR"
trunk serve --port "$PORT" "$@"
