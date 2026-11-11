#!/usr/bin/env bash
# Purpose: fetch roads, railways, rivers and lakes for one area from
# OpenStreetMap (Overpass API) and write data/basemap/<name>.txt (DD-012).
# Needs network — run on a connected machine; the output file is what gets
# carried to the target. One query per run (Overpass fair use). Re-runnable.
# Usage: devel-scripts/fetch-osm-basemap.sh <name> <south> <west> <north> <east>
#   e.g. devel-scripts/fetch-osm-basemap.sh ottawa 44.95 -76.35 45.75 -75.00
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
NAME="$1"
BBOX="$2,$3,$4,$5"
OUT="$REPO/data/basemap/$NAME.txt"
RAW="$REPO/target/osm-$NAME.json"
QUERY="$(mktemp)"
trap 'rm -f "$QUERY"' EXIT
mkdir -p "$REPO/data/basemap" "$REPO/target"

cat > "$QUERY" <<EOF
[out:json][timeout:300];
(
  way["highway"~"^(motorway|trunk|primary|secondary|tertiary)(_link)?\$"]($BBOX);
  way["railway"="rail"]($BBOX);
  way["waterway"="river"]($BBOX);
  way["natural"="water"]($BBOX);
  relation["natural"="water"]($BBOX);
);
out geom;
EOF

echo "===== query Overpass for $BBOX ====="
curl --fail --silent --show-error --retry 4 --retry-delay 10 --retry-all-errors \
  --user-agent "moving-map-display basemap fetch (one-off)" \
  --data-urlencode "data@$QUERY" \
  https://overpass-api.de/api/interpreter -o "$RAW"
ls -lh "$RAW"

echo "===== convert ====="
python3 -I "$REPO/devel-scripts/osm_to_basemap.py" "$RAW" "$OUT" "$BBOX"
ls -lh "$OUT"
