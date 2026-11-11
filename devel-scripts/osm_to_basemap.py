#!/usr/bin/env python3
"""Purpose: convert an Overpass API JSON answer (`out geom`) into the
moving-map basemap text format (DD-012). Reads the JSON file, writes the
basemap file; changes nothing else. Stdlib only.

Usage: osm_to_basemap.py <overpass.json> <out.txt> <bbox "S,W,N,E">

Format: '#' comment lines, then one feature per line:
    <class> <lat>,<lon> <lat>,<lon> ...
Classes: motorway trunk primary secondary tertiary rail river water.
"""
import json
import sys
from datetime import date

ROAD_CLASSES = ("motorway", "trunk", "primary", "secondary", "tertiary")


def classify(tags):
    """Basemap class for an OSM way's tags, or None to skip it."""
    hw = tags.get("highway", "")
    base = hw[:-5] if hw.endswith("_link") else hw
    if base in ROAD_CLASSES:
        return base
    if tags.get("railway") == "rail":
        return "rail"
    if tags.get("waterway") == "river":
        return "river"
    if tags.get("natural") == "water":
        return "water"
    return None


def line(cls, geometry):
    # 6 decimal places = 0.11 m, well inside the 10 cm-class projection.
    pts = " ".join(f"{p['lat']:.6f},{p['lon']:.6f}" for p in geometry)
    return f"{cls} {pts}"


def main():
    src, out, bbox = sys.argv[1], sys.argv[2], sys.argv[3]
    with open(src, encoding="utf-8") as f:
        data = json.load(f)

    rows = []
    counts = {}
    for el in data.get("elements", []):
        if el["type"] == "way" and "geometry" in el:
            cls = classify(el.get("tags", {}))
            if cls:
                rows.append(line(cls, el["geometry"]))
                counts[cls] = counts.get(cls, 0) + 1
        elif el["type"] == "relation" and classify(el.get("tags", {})) == "water":
            # Lakes and wide rivers as multipolygons: draw each outer ring piece.
            for m in el.get("members", []):
                if m.get("role") == "outer" and "geometry" in m:
                    rows.append(line("water", m["geometry"]))
                    counts["water"] = counts.get("water", 0) + 1

    with open(out, "w", encoding="utf-8") as f:
        f.write("# moving-map basemap v1 (DD-012)\n")
        f.write(f"# source: OpenStreetMap via Overpass API, fetched {date.today().isoformat()}\n")
        f.write("# licence: ODbL — © OpenStreetMap contributors\n")
        f.write(f"# bbox S,W,N,E: {bbox}\n")
        for r in rows:
            f.write(r + "\n")

    summary = ", ".join(f"{k} {v}" for k, v in sorted(counts.items()))
    print(f"{len(rows)} features: {summary}")


if __name__ == "__main__":
    main()
