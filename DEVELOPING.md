---
date: 2026-11-08
updated: 2026-11-10
---

# DEVELOPING — working in moving-map-display

## Golden rules

- **Zero external dependencies** — only `std` is used. No `egui`, no `reqwest`, no `serde`
  (DD-001; whether to allow *vendored* crates is a ⚖ in `PLAN.md`).
- **The target has one-way download from approved sites only** and DIS on the
  local LAN. Never add code that fetches from the internet at run time without an
  explicit, configured allowlist.
- **Reference `~/Projects/standards/`** for conventions, never fork.
- **Read from disk before writing**: `~/Projects/standards/BOBBYC_STYLE.md`,
  `~/Projects/standards/SCRIPTING.md`, `~/Projects/standards/CHANGE-REQUESTS.md`.

## Building and running

```bash
cd ~/Projects/moving-map-display
cargo build          # build
cargo test           # run tests
cargo run -- demo    # text map demo
cargo run -- entities                # seeded entity table only
cargo run -- parse-stdin < file.bin  # parse DIS PDUs from stdin
cargo run -- parse file.bin          # parse DIS PDUs from a file
cargo run -- listen --port 3800 --ownship 1.1.1     # live DIS from the LAN (Ctrl-C to stop)
cargo run -- listen --port 3800 --group 239.1.2.3 --exercise 1 --seconds 30
cargo run -- cache-gen --lat 45.0 --lon -75.0 --range-nm 100 tile-cache   # placeholder tiles only (#2)
```

## Mission map data

The app caches map tiles for the mission area at startup. The workflow:

```
Mission area (lat/lon bounding box)
    ↓  Calculate tile bounds for target zoom level
Tile server (e.g. OpenStreetMap)
    ↓  Download PNG tiles (256×256 each)
Local cache directory (flat file tree)
    ↓  App reads tiles from disk for display
```

### Tile scheme (standard OSM / XYZ)

| Parameter | Value |
|---|---|
| Format | PNG, 256×256 px |
| Zoom 0 | whole world (1×1 tiles) |
| Zoom 10 | ~5 km per tile |
| Zoom 13 | ~600 m per tile |
| Zoom 15 | ~150 m per tile |
| URL template | `https://tile.openstreetmap.org/{z}/{x}/{y}.png` |

### Cache layout

*Revised 2026-11-10: the earlier `cache/tiles/z/x/y.png` tree did not match the
code.* `TileCache` (`src/cache/manager.rs:29`) writes one directory per zoom
level under the cache root, written to `.tmp` then renamed, with a sentinel
marking each complete tile:

```
tile-cache/                 # root: cache-gen's positional argument, default tile-cache
  cache.12/
    1024/
      2048.png
      2048.png.downloaded   # sentinel: tile complete
  cache.13/
    2048/
      4096.png
      4096.png.downloaded
```

### Mission area calculation

A 25 NM mission range at the target zoom level gives:
- Zoom 13: ~48 tiles per side (144 tiles total)
- Zoom 15: ~153 tiles per side (23,409 tiles total)

For a 200 NM design-range helicopter (500 NM is the display zoom limit, UN-003 —
revised 2026-11-10 per SESSION-STATUS.md), the cache can be generated per-leg
rather than storing the entire globe — only tiles within the mission
bounding box are downloaded.

### Caching commands

*Revised 2026-11-10:* `cache-gen` exists but writes **1×1 placeholder PNGs** —
there is no tile fetcher yet (PLAN.md Discrepancies #2). `cache-list` and
`cache-clear` are not implemented.

```bash
cargo run -- cache-gen --lat 45.0 --lon -75.0 --range-nm 50 --zoom 13 tile-cache
```

### Tile server options

| Server | URL | Notes |
|---|---|---|
| OSM (Mapnik) | `https://tile.openstreetmap.org/{z}/{x}/{y}.png` | Standard, always-on |
| Stamen Toner (via Stadia) | `https://tiles.stadiamaps.com/tiles/stamen_toner/{z}/{x}/{y}.png` | High-contrast. *Revised 2026-11-10: was labelled "CartoDB Positron", which this URL is not* |
| OSM Topo | `https://tile.opentopomap.org/{z}/{x}/{y}.png` | Topographic relief |

None of these is reachable from the target unless it is on the approved-site
list (⚖ PLAN.md); any fetcher must take an explicit server allowlist.

### Offline strategy

Tiles are downloaded sequentially (no concurrency needed) and written
atomically (write to `.tmp`, then rename). A `.downloaded` sentinel
file marks successful tile completion to support resume on interrupted
downloads.

## Project structure

*Revised 2026-11-10.*

```
src/
  dis/
    mod.rs          — DisConfig, EntityTable, 12 s timeout
    entity.rs       — EntityRecord, Status (from Force ID), Station, Activity, EntityId
    packet.rs       — IEEE 1278.1 header, Entity State, Remove Entity, split_pdus (DD-004)
    receiver.rs     — std-only UDP receiver: broadcast/unicast/multicast (DD-005)
  map/
    projection.rs   — UTM (Krüger series, DD-006), ECEF ↔ WGS84
  render/
    nato.rs         — entity_char() for text map NATO symbology
  cache/
    tile_bounds.rs  — lat/lon ↔ XYZ tiles, MissionArea, auto-zoom
    manager.rs      — TileCache I/O, TileServer trait, placeholder server
  app/              — NOT COMPILED: no `mod app;` in main.rs (PLAN.md #8)
main.rs             — CLI: listen, demo, entities, parse, parse-stdin, cache-gen
```

## Adding a new PDU type

1. Add struct to `src/dis/packet.rs` with fields matching the DIS spec.
2. Add `parse()` method.
3. Add `PduType` enum variant if needed (currently matched by `pdu_type` value).
4. Add unit test with a known-good PDU byte blob.

> **Clarification 2026-11-10 (DD-004):** take offsets from IEEE 1278.1 or a
> reference implementation (Wireshark `packet-dis.c`, Open-DIS), never by
> inventing them — all fields are big-endian. Add the type to `pdu_type`, a
> variant to `Pdu`, a case in `parse_pdu`, and an `encode::` helper so tests
> can build it field by field.

## UTM projection notes

*Revised 2026-11-10 (DD-006).* Forward and inverse use the **Krüger series**
(Karney 2011, to n⁴) — sub-millimetre within a zone; the requirement is 10 cm
round trip, proven by `utm_round_trip_within_10_cm`. DIS positions arrive as
geocentric ECEF and are converted by `ecef_to_geodetic` (iterative, sub-mm).

## Next work

*Revised 2026-11-10: the receiver is built (DD-005), std-only.* Remaining, in
`PLAN.md` order: entity-type → symbol mapping for the new civilian and animal
types; then a GUI, which needs a dependency decision first (⚖ vendored crates).

## Testing

Unit tests live in each module's `#[cfg(test)] mod tests`. Run all:

```bash
cargo test
```

To add a DIS PDU test, create a byte array with known values, parse it,
and assert each field matches the expected value.
