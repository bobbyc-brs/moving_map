---
date: 2026-11-10
---

# Session Status — moving-map-display

> Last updated: 2026-11-10. User went to bed with this file ready.

## Current State

**27/27 tests passing. Clean build. All core modules implemented.**

The other language model completed the project through Phase 4 (text UI + tile cache).
This session fixed the remaining issues and extended the CLI.

## What This Session Did (2026-11-10)

### 1. Fixed 2 failing tests
- `count_cached_zero` and `count_cached_multiple_zooms`
- **Root cause:** All tests shared `/tmp/moving-map-test-<pid>` — stale files from earlier tests polluted later ones
- **Fix:** Replaced shared temp dir with atomic counter-based unique dirs (`/tmp/moving-map-test-0`, `/tmp/moving-map-test-1`, ...) + `Drop` guard for automatic cleanup (even on panic)
- Result: **27/27 pass** (was 25/27)

### 2. Cleaned up compiler warnings
- Removed unused `Path` and `TileBounds` imports in `src/cache/manager.rs`
- Prefixed unused vars `r1`, `d4`, `d6` in `src/map/projection.rs` with underscores
- (24 remaining warnings are all dead code from deferred modules — accepted)

### 3. Added `cache-gen` CLI subcommand
- New: `cargo run -- cache-gen [--lat F] [--lon F] [--range-nm F] [--zoom U]`
- Flags parsed from args after the `cache-gen` command
- Cache directory: first non-flag positional arg, defaults to `tile-cache/`
- Layout: `cache.{z}/x/y.png` + `cache.{z}/x/y.png.downloaded` sentinel
- Shows existing count, downloads missing tiles, reports stats + timing

### 4. Added `.gitignore`
- `target/`, `tile-cache/`, `cache.*` directories
- IDE files, OS junk

## How to Run

```bash
cd ~/Projects/moving-map-display

cargo test          # 27/27 pass
cargo build         # clean build
cargo run -- demo           # text map with 7 seeded entities
cargo run -- entities       # entity table
cargo run -- cache-gen --lat 45.0 --lon -75.0 --range-nm 100 --zoom 10
```

## Deferred Work

### Phase 5: Real DIS UDP Receiver
- `dis::receiver::recv_loop` — bind `239.255.1.1:30000`, non-blocking recv
- Dispatch by PDU type → entity table updates
- Needs `tokio` or std `udp::UdpSocket` with non-blocking I/O

### Phase 5: GUI (egui/eframe)
- Central viewport with painter API for map rendering
- Sidebar entity info panel
- Top bar: zoom slider, grid/range-ring toggles
- **Blocked until crates.io network access available**

### Phase 4: Real Tile Server Fetcher
- `TileServerData` currently returns 1×1 white PNG placeholders
- Need HTTP tile fetcher for OSM/CartoDB
- **Blocked until network access available**

### Polish
- LICENSE file
- `.git` init with initial commit
- Test data files (binary PDU test vectors)

## Architecture Summary

```
src/
  main.rs          — CLI: demo, parse, parse-stdin, entities, cache-gen
  dis/
    entity.rs      — EntityRecord, Status, Station, Activity, EntityId
    packet.rs      — PduHeader + 4 PDU parsers
    mod.rs         — EntityTable with upsert/delete/recreation/stale-purge
  map/
    projection.rs  — WGS84→UTM forward + inverse (±50m accuracy)
    mod.rs
  render/
    nato.rs        — NATO APP-6A character lookup for text map
    mod.rs
  cache/
    tile_bounds.rs — lat/lon ↔ xyz conversion, MissionArea, GeoBounds, TileBounds
    manager.rs     — TileCache I/O, TileServer trait, FileTileServer, OfflineTileServer
    mod.rs
  app/             — placeholder (not yet used)
    mod.rs
    state.rs
```

**Key decisions:**
- Zero external dependencies (std-only)
- UTM projection (NATO standard) over lat/lon Mercator
- NATO APP-6A (not MIL-STD-2525D) for character symbols
- Tile cache: `cache.{z}/x/y.png` with `.downloaded` sentinels
- Auto-zoom: `14 - floor(range_nm / 100)`, clamped [10, 15]
- Helicopter range: 200 NM design (not 500 NM)

## Standards Reference

All standards at `~/Projects/standards/` — reference only, never fork:
- `~/Projects/standards/BOBBYC_STYLE.md` — Canadian English, concise, verb-led bullets
- `~/Projects/standards/DEVELOPING.md` — repo conventions
- `~/Projects/standards/CHANGE-REQUESTS.md` — multi-agent change protocol
- `~/Projects/standards/SCRIPTING.md` — shell conventions

---
*Next agent: pick up from "Deferred Work" above. Start with Phase 5 (UDP receiver) if network available, or continue polish work.*
