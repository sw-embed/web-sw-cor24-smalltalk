#!/usr/bin/env bash
#
# vendor-artifacts.sh -- Refresh vendored sources and artifacts
#
# - Re-builds basic.p24 via the sibling sw-cor24-basic CLI repo
#   (`./scripts/build-basic.sh`) when --build is passed, then copies
#   the resulting build/basic.p24 into assets/.
# - Without --build: just copies the existing build/basic.p24 from
#   the sibling. Falls back to ../web-sw-cor24-basic/assets/basic.p24
#   if the BASIC repo has not been built locally.
#
# Note: build.rs reads the smalltalk source repo's vm.bas, image_*.bas,
# and dN_*.bas directly from $SMALLTALK_DIR every build, so those are
# always fresh -- no vendoring step needed for them.
#
# Usage: ./scripts/vendor-artifacts.sh [--build]
#   BASIC_DIR=/custom/path WEB_BASIC_DIR=/custom/path ./scripts/vendor-artifacts.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BASIC_DIR="${BASIC_DIR:-$REPO_DIR/../sw-cor24-basic}"
WEB_BASIC_DIR="${WEB_BASIC_DIR:-$REPO_DIR/../web-sw-cor24-basic}"

DO_BUILD=0
for arg in "$@"; do
    case "$arg" in
        --build) DO_BUILD=1 ;;
        -h|--help)
            sed -n '2,/^set /p' "$0" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $arg" >&2 ; exit 2 ;;
    esac
done

# Pick the source: prefer freshly built, then BASIC web's vendored copy.
SRC=""
if [ "$DO_BUILD" = "1" ]; then
    if [ ! -x "$BASIC_DIR/scripts/build-basic.sh" ]; then
        echo "error: $BASIC_DIR/scripts/build-basic.sh not found" >&2
        exit 1
    fi
    echo "Building BASIC interpreter in $BASIC_DIR..."
    ( cd "$BASIC_DIR" && ./scripts/build-basic.sh )
    SRC="$BASIC_DIR/build/basic.p24"
elif [ -f "$BASIC_DIR/build/basic.p24" ]; then
    SRC="$BASIC_DIR/build/basic.p24"
elif [ -f "$WEB_BASIC_DIR/assets/basic.p24" ]; then
    SRC="$WEB_BASIC_DIR/assets/basic.p24"
else
    echo "error: no basic.p24 found at:" >&2
    echo "  $BASIC_DIR/build/basic.p24" >&2
    echo "  $WEB_BASIC_DIR/assets/basic.p24" >&2
    echo "rerun with --build to build it from $BASIC_DIR" >&2
    exit 1
fi

DST="$REPO_DIR/assets/basic.p24"
mkdir -p "$REPO_DIR/assets"
cp "$SRC" "$DST"

src_size="$(wc -c < "$SRC" | tr -d ' ')"
dst_size="$(wc -c < "$DST" | tr -d ' ')"
echo ""
echo "Vendored:"
printf "  from  %s\n" "$SRC"
printf "  to    %s\n" "$DST"
printf "  size  %s bytes\n" "$dst_size"
[ "$src_size" = "$dst_size" ] || echo "  warning: size mismatch ($src_size vs $dst_size)" >&2
