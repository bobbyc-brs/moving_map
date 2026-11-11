---
date: 2026-11-08
updated: 2026-11-10
---

# Plan — moving-map-display

## Phase 0 — Project scaffold and build

- [x] `cargo init --name moving-map-display`
- [x] Zero dependencies (no crates.io — the *original sandbox* had no network; this dev machine does, measured 2026-11-10 — see ⚖ dependency policy)
- [x] `cargo check` succeeds
- [x] `cargo build` produces working binary (~2 MB)

**Status:** ✅ Complete (2026-11-08)

## Phase 1 — DIS protocol layer

*Revised 2026-11-10.* The first parser (2026-11-08) was not IEEE 1278.1 and
was rewritten (DD-004); its "✅ Complete" is withdrawn (TESTLOG 2026-11-10).

- [x] `dis::packet` — IEEE 1278.1 v6/v7 header, Entity State (144 B + 16 per
      variable parameter), Remove Entity (28 B), bundle splitting
- [x] `dis::entity` — `EntityRecord`; `Status` from DIS Force ID; `Station` from kind/domain
- [x] `dis::receiver` — std-only UDP, broadcast/unicast/multicast, exercise filter (DD-005)
- [ ] Entity type → `Activity` / symbol from SISO-REF-010 categories, incl.
      cars, trucks, buses, school buses, horses (Bobby, 2026-11-10)

**Status:** ⏳ Rewritten and verified 2026-11-10; type mapping not started

## Phase 2 — Map geometry

- [x] `map::projection` — UTM forward/inverse by Krüger series (DD-006),
      ECEF ↔ WGS84 for DIS positions
- [x] Accuracy: **10 cm** round trip required (Bobby, 2026-11-10); measured
      < 0.1 mm over 45 points (TESTLOG 2026-11-10)

**Status:** ✅ Complete (2026-11-10). *Revised: the 2026-11-08 version claimed
±50 m but was off by up to 10 063 km (southern hemisphere) and placed every
point ~3° off-centre in its zone.*

## Phase 3 — NATO symbology renderer

- [x] `render::nato` module:
      - `entity_char()` — character lookup for text map
      - Station→shape mapping (rectangle, circle, diamond)
      - Status→colour mapping (ANSI escape codes)
      - Activity modifiers (platoon bracket, tank X, etc.)
      - Ownship symbol (cyan 'o')

**Status:** ✅ Complete (2026-11-08)

## Phase 4 — Application UI

- [x] Text-based CLI renderer:
      - Moving map viewport (80×24 characters)
      - Grid lines (1 NM spacing)
      - Range rings (0.5×, 1.0×, 2.0×)
      - Scale bar (NM)
      - Entity table output
- [x] Seed data for demo (7 entities)
- [x] Demo subcommand: `cargo run -- demo`
- [x] PDU parser subcommand: `cargo run -- parse <file>`
- [x] **Tile cache CLI: `cache-gen` subcommand** — 2026-11-10
      - Writes **placeholder** tiles for a mission area (no fetcher yet — Discrepancies #2)
      - Flags: `--lat`, `--lon`, `--range-nm`, `--zoom`
      - Defaults: lat=45, lon=-75, range=100 NM, auto-zoom from range
      - Cache layout: `cache.{z}/x/y.png` with `.downloaded` sentinels

- [ ] **egui/eframe GUI** — needs the ⚖ dependency-policy decision (not network access)
  - Central viewport with painter API
  - Sidebar entity info panel
  - Top bar: zoom slider, grid/ring toggles
  - Click-to-select entity highlighting

**Status:** ⏳ Partial (text UI + tile cache complete, GUI blocked)

## Phase 5 — Real-time DIS reception

- [x] UDP receiver (`dis::receiver`, `listen`) — port 3800 default (DD-007; was 3000), optional
      multicast group (DD-005; the planned `239.255.1.1:30000` was dropped)
- [x] Driven end to end by Sensors' DIS gateway (Sensors CR-001, TESTLOG)
- [ ] GUI integration (after the dependency decision)
- [ ] Performance: ≥ 30 fps with 200 entities (Sensors CR for a 200-entity load scenario)

**Status:** ⏳ Receiver done 2026-11-10; GUI and performance not started

## Phase 6 — Polish

- [x] CLI flags for cache-gen
- [x] README.md, DEVELOPING.md, DECISIONS.md, REQUIREMENTS.md, TEST-PLAN.md
- [x] `.gitignore` (target/, tile-cache/, cache.*, IDE files)
- [ ] LICENSE file
- [x] `.git` init with initial commit (`96c1a76`, 2026-11-10); remote
      `github.com/bobbyc-brs/moving_map` added, not yet pushed

**Status:** ⏳ Partial (docs + cache-gen done, license needed)

## 🧪 Test status (updated 2026-11-10)

**`cargo test`: 45 passing, 0 failing** — 11 cache/manager, 13
cache/tile_bounds, 8 projection, 9 dis/packet, 3 dis/receiver, 1 dis table.
Evidence and history in `TESTLOG.md`. *(The earlier "27 tests incl. 1
dis/entity test" was a miscount: there was no dis/entity test.)*

**Test isolation fix (2026-11-10):** Tests now use unique temp dirs (`/tmp/moving-map-test-{0..N}`) with automatic cleanup via `Drop` guard.

## 📁 Current file layout

See `DEVELOPING.md`, *Project structure* (kept in one place to stop drift).

## ⚖ Decisions

1. **GUI framework** — egui chosen for speed; `wgpu`+`winit` can replace later if GPU rendering is needed. *(deferred until network)*
2. **DIS version** — v2 (HLA DIS) assumed. v1 support would add a parser variant. *(deferred until testing with real sim)*
3. **Map basemap** — vector grid + cached tiles. Tile servers planned: OSM (Mapnik), CartoDB Positron (preferred), CartoDB Dark Matter, OSM Topo.
4. **Tile cache layout** — `cache.{z}/x/y.png` with `.downloaded` sentinel files. Directory per zoom level, x subdirectory, y.png file.
5. **Auto-zoom formula** — `14 - floor(range_nm / 100)`, clamped to [10, 15]. 200 NM → zoom 12.
6. **Zero dependencies** — std-only for now. *Revised 2026-11-10:* the reason
   is no longer "no network" — it is the target's one-way, approved-site-only
   download. Decision pending: stay std-only, or allow crates vendored into the repo.
7. **Basemap scope (new 2026-11-10)** — `REQUIREMENTS.md` §1.2/§5 say vector
   grid only and no external data files; the tile cache assumes downloaded
   tiles. Keep tiles in scope (and from which approved source), or drop them?

## 🔜 Next session work

See `SESSION-STATUS.md` for detailed context.

## Discrepancies found 2026-11-10

Found while bringing `README.md` into line with `~/Projects/standards/`.
Recorded here rather than silently fixed; each needs an owner decision or a
code change.

| # | Discrepancy | Evidence | Method |
|---|---|---|---|
| 1 | `parse <file>` reads the word `parse` as the filename, so it always fails | `src/main.rs:276` uses `args.get(1)`; should be `args.get(2)` | Measured: ran it, got "Error reading parse" |
| 2 | `cache-gen` writes 1×1 white placeholder PNGs; it downloads nothing. Phase 4 above says "Downloads/renders" | `TileServerData`, `src/cache/manager.rs:202` | Read the code |
| 3 | Entity Detection (type 5) is labelled, not decoded — the old README marked it ✅ | `src/main.rs:249` | Read the code |
| 4 | "±50 m" inverse-projection accuracy is not tested: round-trip tests allow 0.05° (~5.5 km latitude) | `src/map/projection.rs:115-128` | Read the tests |
| 5 | PDU type numbers (2 Death, 4 Recreation, 5 Detection) and lat/lon in Entity State may not match IEEE 1278.1, where 2 is Fire, 4 Collision, 5 Service Request, and location is geocentric XYZ | `process_pdu`, `src/main.rs:222` | Hypothesis from knowledge of the standard — not yet checked against the spec text |
| 6 | `DEVELOPING.md` gives the cache layout as `cache/tiles/z/x/y.png`; the code writes `cache.{z}/x/y.png` | `src/cache/manager.rs:29` | Read the code |
| 7 | Both blockers ("no crates.io network", "needs network stack") date from the original sandbox. `std::net::UdpSocket` needs no crates, so Phase 5's receiver is not blocked on network access | DD-001 | Hypothesis — re-check network access on this machine |

⚖ #5 decides whether this parser can read real DIS traffic at all; worth settling before Phase 5.

## Update 2026-11-10 (later) — DIS rewrite and LAN receiver

**Discrepancies status:**

| # | Status | Ref |
|---|---|---|
| 1 | Done — `parse <file>` fixed | commit `cb76995` |
| 2 | Open — placeholder tiles remain; see ⚖ 3 below | |
| 3 | Done — Entity Detection was not a DIS PDU; removed | DD-004 |
| 4 | Done — replaced by Krüger series; 10 cm proven (< 0.1 mm measured) | DD-006, commit `231b97c` |
| 5 | Done — confirmed non-standard (measured against Wireshark + Open-DIS source); parser rewritten | DD-004, TESTLOG 2026-11-10 |
| 6 | Done — `DEVELOPING.md` cache layout rewritten to match the code | docs commit 2026-11-10 |
| 7 | Done — network measured available on this dev machine: `index.crates.io` and `tile.openstreetmap.org` both HTTP 200 | |
| 8 | **New, open** — `src/app/` is not compiled (`main.rs` has no `mod app;`) and would not compile: it calls the absent `log` crate and an old `to_record` signature. Its 2 tests have never run | `src/app/state.rs` |

**Phase 5 receiver: done** (DD-005) — `cargo run -- listen`, std-only UDP,
broadcast/unicast plus optional multicast join, v7 bundles, exercise filter,
12 s entity timeout. GUI remains Not started.

**Deployment constraint (Bobby, 2026-11-10):** the target system has
**one-way download from specific approved sites only**, and DIS arrives on the
local LAN. The dev machine having internet does not make crates or tile
servers available on the target.

**⚖ Decisions for Bobby, in order:**
1. **DIS site settings** — port (**3800** default, Bobby 2026-11-10 — still must match the simulators), exercise ID **1** (Bobby, DD-008). Open: broadcast or multicast group (239.238.237.236 proposed) — needs the simulators' configuration.
2. **Dependency policy (revisits DD-001)** — DD-001's premise ("sandbox has no network") no longer holds. Options: stay std-only; or allow crates **vendored into the repo** (`cargo vendor`) so builds never reach crates.io. `egui` would arrive this way.
3. **Tile source on the target** — which approved site serves tiles, or whether tiles are pre-staged by `cache-gen` on a connected machine and carried in. Either way the fetcher takes an explicit server allowlist with no fallback hosts.
4. **Real DIS capture** — a few seconds of `.pcap` from the site simulators would replace synthetic test vectors with real ones.

**Also pending:** an Open-DIS (PyPI) cross-check was blocked by the session's
permission rules; the Python `struct` encoder in TESTLOG 2026-11-10 stood in.

## Update 2026-11-10 (evening) — Sensors drives the map

**Sensors CR-001: Processed** (Sensors `644203f`, `033eb02`) and re-verified
here (TESTLOG 2026-11-10, "Sensors DIS gateway"). Run it with:

```bash
# ~/Projects/Sensors
python3 simulation_manager.py -v 3 --headless --dis            # truth only
python3 simulation_manager.py -v 3 --headless --dis --dis-fused # + sensed tracks
# ~/Projects/moving-map-display
cargo run -- listen --port 3800 --range-nm 200
```

**Decided by Bobby:** civil aircraft are Neutral (force 3); sensed traffic is
optional (`--dis-fused`) and shows as Unknown (force 0), site 2.

**Open (Bobby's call):** Sensors' heading drift (initial bearing held for the
whole flight) — fix lives in Sensors `vehicles/vehicle_sim.py`, not here.

## Update 2026-11-10 (night) — Sensors is a CGF

**Sensors CR-002 → CR-004: Processed** and re-verified here (TESTLOG
2026-11-10, "Sensors CGF scenarios"). Scenarios: `ottawa-mixed`, `lifecycle`,
`load-200`, `zone-crossing`; ownship `1.42.1` (none in `lifecycle`).

```bash
# ~/Projects/Sensors
python3 -m cgf.runner scenarios/ottawa-mixed.json --duration 60
python3 -m cgf.runner scenarios/load-200.json --fault-truncate 0.05 --fault-seed 7
# ~/Projects/moving-map-display
cargo run -- listen --ownship 1.42.1 --range-nm 25
cargo run -- parse tests/data/dis/ottawa-mixed.bin
```

**Next here (moving-map):**
1. Entity type → `Activity` / symbol for every type in ottawa-mixed (Phase 1).
2. Show a per-entity "stale" marker before the 12 s drop; decode pitch/roll.
3. ~~Recordings as regression tests~~ — done 2026-11-10: copied into
   `tests/data/dis/` (Bobby: the tests belong to moving-map), 4 tests.
4. Measure PDU → table latency from the DIS timestamp (REQUIREMENTS §4, < 50 ms).
