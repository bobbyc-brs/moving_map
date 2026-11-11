---
date: 2026-11-08
updated: 2026-11-10
---

# DEVELOPING — working in moving-map-display

## Golden rules

- **One external crate, vendored** — `eframe` (egui) for the GUI, and its
  dependencies; everything else is `std` only. New crates need a reason in the
  DD log, and every build must work offline from `vendor/` (DD-010, superseding
  DD-001's zero-dependency rule).
- **Rust comes from rustup, pinned** by `rust-toolchain.toml` (1.99.1). The
  distro `rustc` (1.80 on this machine; apt tops out at 1.82) cannot build the
  GUI crates.
- **The target has one-way download from approved sites only** and DIS on the
  local LAN. Never add code that fetches from the internet at run time without an
  explicit, configured allowlist.
- **Reference `~/Projects/standards/`** for conventions, never fork.
- **Read from disk before writing**: `~/Projects/standards/BOBBYC_STYLE.md`,
  `~/Projects/standards/SCRIPTING.md`, `~/Projects/standards/CHANGE-REQUESTS.md`.

## Setup on a new machine

*Added 2026-11-10 (DD-010).* Ubuntu packages are too old, so install Rust with
**rustup** — no sudo, installs under `~/.rustup` and `~/.cargo`, leaves any apt
`rustc` alone. The script installs the version pinned in `rust-toolchain.toml`:

```bash
cd ~/Projects/moving-map-display
bash devel-scripts/01-install-rust.sh
source "$HOME/.cargo/env"        # or open a new login shell
cargo --version                  # must be ~/.cargo/bin/cargo, 1.99.1
```

GUI build prerequisites (Ubuntu): an OpenGL driver and the Wayland/X11 client
libraries, present on any desktop install.

**Offline machines** (the target's one-way download): on a connected machine
run `bash devel-scripts/02-vendor-crates.sh`, carry `vendor-<date>.tar.gz` and
the rustup offline installer across, then:

```bash
tar xzf vendor-*.tar.gz
cargo build --offline --config .cargo/vendor.toml
```

**Not in git, and why:** `vendor/` (~355 MB, mostly Windows and web stubs that
never build here) is regenerated exactly from `Cargo.lock`; `target/` is build
output.

## Basemap data

*Added 2026-11-10 (DD-012).* The GUI reads every `*.txt` in `data/basemap/`
at start (`--basemap <file|dir>` to choose, `--no-basemap` to skip); it never
fetches. To add an area, on a connected machine:

```bash
bash devel-scripts/fetch-osm-basemap.sh <name> <south> <west> <north> <east>
bash devel-scripts/fetch-osm-basemap.sh ottawa 44.95 -76.35 45.75 -75.00   # the committed one
```

One Overpass query per run — keep areas to about a degree square (Ottawa:
45 MB of JSON → 11 MB of text). Data © OpenStreetMap contributors, ODbL.

## Ports

| Port | Who |
|---|---|
| 3800/udp | DIS default — `gui`, `listen`, and Sensors (DD-007) |
| 3801/udp | `devel-scripts/verify-with-sensors.sh` |
| 3802/udp | `devel-scripts/screenshot-gui.sh` |
| 3803–3809/udp | Free for private test runs |
| 3901–3904/udp | Taken: the ports Sensors' recordings were captured on |

## Building and running

```bash
cd ~/Projects/moving-map-display
cargo build          # build
cargo test           # run tests
cargo run -- gui --ownship 1.42.1                 # touch-screen map, DIS on :3800
cargo run -- gui --ownship 1.42.1 --track-up --scheme night --fullscreen
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

*Revised 2026-11-10 (GUI added; `src/app/` removed — it never compiled).*

```
src/
  dis/
    mod.rs          — DisConfig (port 3800, exercise 1), EntityTable, 12 s timeout
    entity.rs       — EntityRecord (incl. DIS entity type, force ID), Status, EntityId
    packet.rs       — IEEE 1278.1 header, Entity State, Remove Entity, attitude (DD-004)
    receiver.rs     — std-only UDP receiver: broadcast/unicast/multicast (DD-005)
    recordings.rs   — regression tests over tests/data/dis/*.bin (DD-008)
  map/
    projection.rs   — UTM (Krüger series, DD-006), ECEF ↔ WGS84
    mgrs.rs         — MGRS strings, grid convergence
  render/
    symbol.rs       — DIS entity type → APP-6A dimension + icon (DD-011)
    nato.rs         — characters for the text map
  gui/
    mod.rs          — app: status bar, map, controls, list/details, touch input
    view.rs         — screen ↔ ground transform, held zone, rings, range/bearing
    feed.rs         — DIS receiver thread → shared entity table
    draw.rs         — palettes (Day/Dusk/Night), APP-6A frames and icons, ownship
  cache/
    tile_bounds.rs  — lat/lon ↔ XYZ tiles, MissionArea, auto-zoom
    manager.rs      — TileCache I/O, TileServer trait, placeholder server
main.rs             — CLI: gui, listen, demo, entities, parse, parse-stdin, cache-gen
devel-scripts/      — 01-install-rust, 02-vendor-crates, verify-with-sensors, screenshot-gui
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

*Revised 2026-11-10 (late): the GUI and symbol mapping are built (DD-011).*
Remaining is in `PLAN.md` — declutter, basemap (vector roads, DD-009),
latency measurement, and the Sensors test features in Sensors CR-005.

## Testing

Unit tests live in each module's `#[cfg(test)] mod tests`. Run all:

```bash
cargo test
```

Against live Sensors scenarios (needs `~/Projects/Sensors`):

```bash
bash devel-scripts/verify-with-sensors.sh          # listen + parse, port 3801
bash devel-scripts/screenshot-gui.sh ottawa-mixed 10 --track-up --scheme night
# → target/shots/<scenario>-<time>.png; opens a window for ~10 s
```

The screenshot script runs the GUI under X11 (`WAYLAND_DISPLAY=` empty): on a
**locked** Wayland session the compositor sends no frames, so a Wayland window
never draws and the screenshot never fires (found 2026-11-10).

To add a DIS PDU test, create a byte array with known values, parse it,
and assert each field matches the expected value.
