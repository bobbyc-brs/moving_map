---
date: 2026-11-08
---

# DEVELOPING — working in moving-map-display

## Golden rules

- **Zero external dependencies** — only `std` is used. No `egui`, no `reqwest`, no `serde`.
- **Reference `~/Projects/standards/`** for conventions, never fork.
- **Read from disk before writing**: `~/Projects/standards/BOBBYC_STYLE.md`,
  `~/Projects/standards/SCRIPTING.md`, `~/Projects/standards/CHANGE-REQUESTS.md`.

## Building and running

```bash
cd ~/Projects/moving-map-display
cargo build          # build
cargo test           # run tests
cargo run -- demo    # text map demo
cargo run -- parse-stdin < file.bin  # parse DIS PDUs from stdin
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

Tiles are stored in a flat directory indexed by `z/x/y.png`:

```
cache/tiles/
  12/
    1024/
      2048.png
      2049.png
    1025/
      2048.png
  13/
    2048/
      4096.png
```

### Mission area calculation

A 25 NM mission range at the target zoom level gives:
- Zoom 13: ~48 tiles per side (144 tiles total)
- Zoom 15: ~153 tiles per side (23,409 tiles total)

For a 500 NM maximum-range helicopter, the cache can be generated per-leg
rather than storing the entire globe — only tiles within the mission
bounding box are downloaded.

### Caching commands (planned, Phase 7)

```bash
# Generate cache from mission parameters
cargo run -- cache-gen \
  --lat 45.0 --lon -75.0 \
  --range-nm 50 --zoom 13 \
  --output cache/

# List cached tiles
cargo run -- cache-list

# Clear cache
cargo run -- cache-clear
```

### Tile server options

| Server | URL | Notes |
|---|---|---|
| OSM (Mapnik) | `https://tile.openstreetmap.org/{z}/{x}/{y}.png` | Standard, always-on |
| CartoDB Positron | `https://tiles.stadiamaps.com/tiles/stamen_toner/{z}/{x}/{y}.png` | Clean, low-detail |
| OSM Topo | `https://tile.opentopomap.org/{z}/{x}/{y}.png` | Topographic relief |

### Offline strategy

Tiles are downloaded sequentially (no concurrency needed) and written
atomically (write to `.tmp`, then rename). A `.downloaded` sentinel
file marks successful tile completion to support resume on interrupted
downloads.

## Project structure

```
src/
  dis/
    mod.rs          — EntityTable, DisConfig
    entity.rs       — EntityRecord, Status, Station, Activity, EntityId
    packet.rs       — PduHeader, EntityStatePdu, EntityDeathPdu, etc.
  map/
    mod.rs          — (empty, exports projection module)
    projection.rs   — LatLon, EastingNorthing, UTM forward+inverse
  render/
    mod.rs          — (empty, exports nato module)
    nato.rs         — entity_char() for text map NATO symbology
  app/
    mod.rs          — (empty, reserved for future GUI state)
    state.rs        — (empty, reserved)
  cache/
    mod.rs          — (empty, reserved for tile cache logic)
main.rs             — CLI entry: demo, parse, entities, cache-* subcommands
```

## Adding a new PDU type

1. Add struct to `src/dis/packet.rs` with fields matching the DIS spec.
2. Add `parse()` method.
3. Add `PduType` enum variant if needed (currently matched by `pdu_type` value).
4. Add unit test with a known-good PDU byte blob.

## UTM projection notes

The forward projection uses standard EPSG formulas. The inverse uses an
iterative Newton method on the meridional arc — accurate to ~50 m for
mid-latitude points. For display purposes this is sufficient; higher
accuracy would require additional series terms.

## Planned next phase (requires network access)

1. Add `egui` + `eframe` dependencies for GUI.
2. Implement `dis::receiver` with UDP multicast socket.
3. Wire DIS receiver → entity table → egui painter for live map.
4. Add click-to-select entity info panel.
5. Add zoom/pan controls.

## Testing

Unit tests live in each module's `#[cfg(test)] mod tests`. Run all:

```bash
cargo test
```

To add a DIS PDU test, create a byte array with known values, parse it,
and assert each field matches the expected value.
