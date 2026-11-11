#!/usr/bin/env bash
# Purpose: play a Sensors scenario into the GUI on a private port, save a
# screenshot after N seconds, and convert it to PNG in target/shots/.
# Opens a window on the current display for ~N seconds. Changes nothing else.
# Usage: devel-scripts/screenshot-gui.sh [scenario] [seconds] [extra gui flags...]
#   e.g. devel-scripts/screenshot-gui.sh ottawa-mixed 10 --track-up --scheme night
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
SCENARIO="${1:-ottawa-mixed}"
AFTER="${2:-10}"
shift 2 || shift $#
PORT=3802
SENSORS="${SENSORS:-$HOME/Projects/Sensors}"
MAP="$REPO/target/debug/moving-map-display"
OUT="$REPO/target/shots"
NAME="$SCENARIO-$(date +%H%M%S)"
mkdir -p "$OUT"
cd "$REPO"    # the GUI loads data/basemap/ relative to here

echo "===== build ====="
"$HOME/.cargo/bin/cargo" build --quiet --manifest-path "$REPO/Cargo.toml"

echo "===== scenario $SCENARIO on :$PORT ====="
(cd "$SENSORS" && python3 -m cgf.runner "scenarios/$SCENARIO.json" --port "$PORT" --duration "$((AFTER + 5))") \
  > "$OUT/$NAME.runner" 2>&1 &
RUNNER=$!

echo "===== gui, screenshot after ${AFTER}s ====="
# X11 (XWayland), not Wayland: a locked Wayland session sends no frame
# callbacks, so the window never draws and the screenshot never fires.
WAYLAND_DISPLAY= "$MAP" gui --port "$PORT" --ownship 1.42.1 --screenshot "$OUT/$NAME.ppm" --after "$AFTER" "$@"
wait "$RUNNER" || true

echo "===== convert ====="
convert "$OUT/$NAME.ppm" "$OUT/$NAME.png"
rm -f "$OUT/$NAME.ppm"
echo "$OUT/$NAME.png"
