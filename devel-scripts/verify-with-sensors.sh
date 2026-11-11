#!/usr/bin/env bash
# Purpose: check moving-map's `listen` and `parse` against Sensors' CGF
# scenarios on a private port (default 3801, DD-007). Changes nothing in
# either repo; logs go to target/verify/. Needs ~/Projects/Sensors (or $SENSORS).
# Usage: devel-scripts/verify-with-sensors.sh [port]
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
PORT="${1:-3801}"
SENSORS="${SENSORS:-$HOME/Projects/Sensors}"
MAP="$REPO/target/debug/moving-map-display"
OUT="$REPO/target/verify"
mkdir -p "$OUT"

# Strip ANSI colour codes from a listen log.
strip() { sed 's/\x1b\[[0-9;]*[A-Za-z]//g' "$1"; }

# run_live <log-name> <scenario> <seconds> [listen flags...]
# Starts listen for seconds+3, plays the scenario for <seconds>, waits.
run_live() {
  local name="$1"
  local scenario="$2"
  local seconds="$3"
  shift 3
  "$MAP" listen --port "$PORT" --seconds "$((seconds + 3))" "$@" > "$OUT/$name.listen" 2>&1 &
  local listen_pid=$!
  sleep 1
  (cd "$SENSORS" && python3 -m cgf.runner "scenarios/$scenario.json" --port "$PORT" --duration "$seconds") \
    > "$OUT/$name.runner" 2>&1 || true
  wait "$listen_pid" || true
  strip "$OUT/$name.listen" > "$OUT/$name.txt"
}

echo "===== build ====="
"$HOME/.cargo/bin/cargo" build --quiet --manifest-path "$REPO/Cargo.toml"

echo "===== 1. regression vectors decode (tests/data/dis) ====="
for f in "$REPO"/tests/data/dis/*.bin; do
  "$MAP" parse "$f" > "$OUT/$(basename "$f").parse" 2>&1
  printf '%-20s %s\n' "$(basename "$f")" "$(grep -a 'Processed' "$OUT/$(basename "$f").parse")"
done

echo "===== 2. ottawa-mixed live, ownship 1.42.1 ====="
run_live mixed ottawa-mixed 15 --ownship 1.42.1 --range-nm 25
grep -a "datagrams=" "$OUT/mixed.txt" | tail -1
tail -3 "$OUT/mixed.runner"

echo "===== 3. load-200 live ====="
run_live load load-200 15 --range-nm 120
grep -a "datagrams=" "$OUT/load.txt" | tail -1
tail -1 "$OUT/load.runner"

echo "===== 4. pcap replay (Sensors CR-005), x4 speed ====="
# ottawa-mixed.pcap: 30 s, 554 PDUs → ~7.5 s at ×4.
"$MAP" listen --port "$PORT" --seconds 12 > "$OUT/replay.listen" 2>&1 &
LISTEN_PID=$!
sleep 1
(cd "$SENSORS" && python3 -m cgf.replay recordings/ottawa-mixed.pcap --port "$PORT" --speed 4) \
  > "$OUT/replay.runner" 2>&1 || true
wait "$LISTEN_PID" || true
strip "$OUT/replay.listen" > "$OUT/replay.txt"
grep -a "datagrams=" "$OUT/replay.txt" | tail -1
tail -2 "$OUT/replay.runner"

echo "===== 5. leftovers ====="
if pgrep -af "cgf.runner|moving-map-display listen"; then
  echo "WARNING: processes still running"
else
  echo "clean: nothing left running"
fi
