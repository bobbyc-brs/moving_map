---
date: 2026-11-08
---

# Decisions — moving-map-display

Newest first from DD-004 on, per `~/Projects/standards/BOBBYC_STYLE.md`;
DD-001 to DD-003 predate that and stay in their original order below.

## DD-012 — First basemap: OpenStreetMap roads and water, pre-staged as a text file
**Date:** 2026-11-10
**Status:** Decided for development and demo; the production road source is
still ⚖ Bobby's (PLAN — simulator terrain database vs OpenStreetMap)

### Decision
- Bobby: "I don't see a graphic of the area we're flying over." Implements
  DD-009's main layer: **vector roads** (motorway → tertiary), **railways**,
  **rivers** and **lakes / wide rivers** as polylines.
- **Source:** OpenStreetMap via one **Overpass API** query
  (`devel-scripts/fetch-osm-basemap.sh`), run on a connected machine and
  converted by `devel-scripts/osm_to_basemap.py` to a plain text file —
  `data/basemap/ottawa.txt` (44.95–45.75°N, 76.35–75.00°W; 30 282
  features, 11 MB). The GUI never fetches; it reads `data/basemap/` at start.
- **Format:** one feature per line, `<class> lat,lon lat,lon …` — readable,
  diff-able, parsed with `std` (`map::basemap`), no GIS crate.
- **Drawing:** projected into the held UTM zone once per zone on a
  background thread, then rotated / scaled per frame; classes drop out by
  scale (tertiary below 50 m/px, secondary below 120 m/px); ponds under 12 px
  hidden; roads thinned to ≥ 1.5 px between points. Dimmed for Dusk / Night.
  Attribution "© OpenStreetMap contributors" drawn on the map (ODbL).
- The file **is committed** (11 MB text) so a fresh clone shows a map.

### Rationale
- Same projection path as the entities, so roads and symbols agree to the
  projection's accuracy (DD-006) plus OSM's own (typically metres).
- One Overpass query per area is within its fair-use policy; bulk tile
  downloads would not be (OSM tile policy), nor would most imagery services.
- Text over a binary format: 11 MB is acceptable, and anyone can inspect it.

### Alternatives Considered
- **Raster OSM tiles:** rejected as the main layer (DD-009); bulk
  pre-fetching for offline use breaches the OSM tile usage policy.
- **Satellite imagery:** deferred — needs a source whose licence allows
  offline caching, via the approved one-way download (PLAN ⚖).
- **Geofabrik `.pbf` extract:** deferred — needs a protobuf / zlib reader;
  Overpass gives a small, targeted area instead.

### Known Gaps
- **Lakes and rivers are outlines, not filled**; multipolygon pieces are not
  assembled into rings.
- **Covers the Ottawa area only** — the zone-crossing scenario (near
  Bancroft) and most of load-200's 100 NM radius are off the map. Add areas
  with the fetch script; every `*.txt` in `data/basemap/` is loaded.
- No road names, no towns, no labels. No terrain or elevation (future, Bobby).
- OSM roads may not match the simulator's terrain database (DD-009).
- A dev build needs optimised dependencies (`Cargo.toml` `[profile.dev]`)
  to draw the map at interactive rates; frame rate not yet measured.

---

## DD-011 — Touch-screen GUI design: APP-6A from the DIS entity type, held UTM zone, true-north display
**Date:** 2026-11-10
**Status:** Decided (implementation choices within Bobby's brief: in-flight /
simulator moving map, touch screen)

### Decision
- **Symbols come from the DIS entity type** (`render::symbol::classify`):
  kind + domain pick the APP-6A **frame** (air, ground, sea surface,
  subsurface); category / subcategory pick the **icon**; force ID picks the
  **affiliation** colour. Country is ignored.
- **One held UTM zone** per picture, switching only when ownship is **0.5°**
  past the zone edge; the display is rotated for **grid convergence**, so
  north-up means true north and track-up means true heading.
- **Touch first:** ≥ 48 px controls; tap selects an entity or drops a mark
  (MGRS, DMS, range / bearing from ownship); drag pans (Centre returns);
  pinch or scroll zooms; presets 1–500 NM.
- **Time behaviour:** dead-reckon displayed positions for up to **5 s**;
  dim a symbol after **7 s** silent (DIS heartbeat is 5 s); drop at 12 s
  (unchanged). Trails 5 min, sampled every 2 s. Velocity leader = 1 min.
- **Schemes:** Day, Dusk, Night; Night dims symbols to 40 % and all panel
  text and widgets to 45 %.
- **Screenshot mode** (`gui --screenshot f.ppm --after N`) for unattended
  checks; PPM so no image crate is needed.

### Rationale
- The old `Station`/`Activity` codes were invented and never sent by any
  simulator (DD-004); the entity type is what DIS actually carries.
- Mixing zones would put entities either side of 78°W on different planes;
  hysteresis stops the grid flipping while flying along a boundary (the
  zone-crossing scenario does exactly this).
- Grid north is up to ~2.1° off true north at a zone edge at 45°N — visible
  against a heading line, so the display corrects for it.
- 7 s stale threshold: one missed heartbeat plus jitter; dimming before the
  12 s drop tells the crew a track is ageing rather than having it vanish.

### Alternatives Considered
- **Hard-coded symbol table per entity:** rejected — breaks on any new type;
  classification by category ranges degrades to a framed "?" instead.
- **Web Mercator display:** rejected — scale varies with latitude; UTM is
  conformal and metric within a zone (DD-002).
- **Civilian pictograms from MIL-STD-2525C:** deferred — APP-6A has no car,
  bus or horse icons, so these are local pictograms / text for now.

### Known Gaps
- **Icons are simplified**, and civilian / animal icons are local extensions,
  not APP-6A. Echelon, task-force and other modifiers are not drawn.
- **No declutter** — symbols overlap at long range (seen at 25 NM with 13 entities).
- MGRS ignores the Norway / Svalbard zone exceptions and the polar UPS areas.
- Range / bearing readouts use a sphere (≤ 0.5 % error); positions use the ellipsoid.
- Dead reckoning uses heading and speed only (flat, constant velocity), not
  the PDU's dead-reckoning algorithm field.
- Touch gestures were checked with a mouse only; no touch screen here yet.

---

## DD-010 — Dependencies: vendored crates allowed; eframe for the GUI; Rust from rustup
**Date:** 2026-11-10
**Status:** Decided (Bobby) — supersedes DD-001's zero-dependency rule

### Decision
- **Vendored crates are allowed** (Bobby: "Vendored crates are fine, go
  ahead with the GUI"). The GUI uses **`eframe` 0.37** (egui) with the
  **glow** (OpenGL) backend, Wayland and X11; ~290 crates in `Cargo.lock`.
- **`vendor/` is not committed.** `devel-scripts/02-vendor-crates.sh`
  regenerates it exactly from `Cargo.lock` and packs a tarball for the one-way
  download; `.cargo/vendor.toml` switches cargo to it
  (`cargo build --offline --config .cargo/vendor.toml`).
- **Rust 1.99.1 from rustup**, pinned in `rust-toolchain.toml`;
  `devel-scripts/01-install-rust.sh` installs it (Bobby agreed to update Rust).

### Rationale
- A GPU GUI written on `std` alone is not practical; egui is immediate-mode,
  pure Rust, and draws everything with a painter — symbology stays
  programmatic (REQUIREMENTS §5).
- **Size:** `vendor/` is **355 MB**, mostly `windows-sys`, `web-sys` and
  similar stubs that never compile on Linux; in git it would bloat every
  clone for no information `Cargo.lock` (with checksums) does not already pin.
- **Toolchain:** current crates need edition 2024 (Rust ≥ 1.85). The machine
  had 1.80 (Ubuntu 24.10, end of life); apt's newest is **1.82**. Staying on
  1.80 meant pinning dozens of crates to old releases, missing fixes.
- glow over wgpu: fewer crates, and OpenGL is available on any target with a display.

### Alternatives Considered
- **Stay std-only** (DD-001): rejected by Bobby for the GUI.
- **Commit `vendor/`:** rejected — 355 MB; deferred, not ruled out, if the
  site requires the source tree alone to build offline.
- **Pin crates for Rust 1.80:** tried first; rejected — dozens of
  downgrades, repeated for every future dependency.

### Known Gaps
- The offline build path (`02-vendor-crates.sh` → tarball → offline build) is
  scripted but **not yet run end to end** on a machine without network.
- The target's Rust toolchain must arrive by the one-way download too (rustup
  offline installer); not yet tried.
- Ubuntu 24.10 itself is end of life (packages come from `old-releases`).

---

## DD-009 — Basemap: vector roads first, raster tiles an optional backdrop; DIS broadcast by default
**Date:** 2026-11-10
**Status:** Decided (Bobby)

### Decision
- The basemap's main layer is **vector roads**, projected through the same
  lat/lon → UTM → screen path as entities. **Raster tiles** are an optional,
  faded backdrop. Both are pre-staged files from an approved site (one-way
  download), never fetched at run time.
- DIS input is **broadcast** by default (Bobby: "I typically see DIS
  broadcasts"); `--group` joins a multicast group when a site uses one.
  Unchanged from DD-005's default — now a decision, not an assumption.

### Rationale
- Vector roads share the entities' projection, so they align to well under a
  metre, stay sharp at any range, keep labels upright in heading-up, and are
  the data future road-following needs. Raster tiles are Web Mercator pictures:
  they must be warped to sit on a UTM display, labels rotate with heading-up,
  and roads are only pixels.
- The larger alignment risk is the *source*, not the format: sim vehicles drive
  on the simulator's terrain-database roads. If the map's roads come from
  elsewhere, vehicles can sit metres to tens of metres off the drawn road.
  Hence the open ⚖ on road data source (PLAN).
- Supersedes REQUIREMENTS §1.2 "vector grid lines only" and §5 "no external
  data files" for basemap data.

### Alternatives Considered
- **Raster tiles as the main layer:** rejected — misalignment, rotating labels,
  no road geometry.
- **Grid only (original REQUIREMENTS):** rejected — operator needs roads for context.
- **Multicast by default:** rejected — not what Bobby sees on DIS sites.

## DD-008 — Default exercise ID 1; recordings copied into this repo
**Date:** 2026-11-10
**Status:** Decided (Bobby)

### Decision
- `listen` accepts **exercise 1** by default and counts others as
  `other-exercise`; `--exercise N` picks another, `--exercise any` accepts all.
- Sensors' recordings are **copied** into `tests/data/dis/` (630 KB) and
  decoded by `src/dis/recordings.rs`, not read from `../Sensors`.

### Rationale
- Exercise 1: Bobby's site value; Sensors already sends 1. Filtering by default
  means a second exercise on the same LAN cannot leak onto the map.
- Copy: "the tests are related to moving map, not Sensors" (Bobby). A path into
  another repo breaks when it moves, is absent on another machine, or
  re-records; the copy is pinned by commit (README gives provenance).

### Alternatives Considered
- **Accept all exercises by default** (the DD-005 behaviour): rejected — silent
  mixing of exercises is worse than a visible `other-exercise` count.
- **Reference `../Sensors/recordings` by path:** rejected for the reasons above.

## DD-007 — Default DIS port 3800
**Date:** 2026-11-10
**Status:** Decided (Bobby)

### Decision
Default UDP port is **3800**, in moving-map (`DisConfig::port`, `listen`) and
Sensors (gateway and CGF runner) together. 3801–3809 are kept for private test
runs on one machine. `--port` overrides on both sides. Supersedes the port
part of DD-005; the rest of DD-005 stands.

### Rationale
- 3000 is too common (Bobby) — many dev servers and DIS tools default to it,
  so a stray sender or listener on the LAN is likely.
- Bobby avoids x911 because of his other work; 380x is clear of it and below
  Linux's ephemeral range (32768–60999).
- Recordings are unaffected: raw `.bin` files carry no port, and Sensors'
  `.pcap` files were captured on 3901–3904.

### Alternatives Considered
- **3000:** rejected, too common. **3911:** used for today's tests; rejected (x911).
- On site the port must still match what JCATS/JTTS send — PLAN ⚖ 1.

## DD-006 — UTM by the Krüger series; 10 cm accuracy requirement
**Date:** 2026-11-10
**Status:** Decided

### Decision
Implement UTM forward and inverse with the **Krüger n-series** (Karney 2011,
truncated at n⁴), and require a lat/lon → UTM → lat/lon round trip within
**10 cm** anywhere in a zone (Bobby: 1 mm is too stringent for aircraft; 10 cm
is the right bar).

### Rationale
- Measuring the old code in metres exposed three bugs a 0.05° (~5.5 km)
  tolerance had hidden: no southern false northing (10 063 km error), no inverse
  latitude correction, and the zone's west edge used as its central meridian
  (every point ~3° off-centre). Patching a hand-rolled series was riskier than
  replacing it with the standard formulation.
- Krüger to n⁴ is sub-millimetre within a zone — measured worst < 0.1 mm over
  45 points, 70°S–70°N; independent checks against a numerically integrated
  meridian arc (± 1 mm) and the 166 021.44 m equator zone-edge easting
  (TESTLOG 2026-11-10, commit `231b97c`).
- Pure arithmetic: no dependencies (DD-001 holds).

### Alternatives Considered
- **Fix the old Snyder-style series in place:** rejected — three independent
  bugs, and its tests encoded the wrong values.
- **`proj` crate:** rejected for now — C library dependency, at odds with DD-001
  and the one-way-download target.

### Known Gaps
- Zone transitions as ownship crosses a boundary (FR-002) are not exercised yet;
  `to_utm_zone()` exists so the renderer can hold ownship's zone. Sensors CR
  for a zone-crossing scenario requested.
- Accuracy degrades far outside a zone (> ~3 000 km from the central meridian);
  irrelevant at the 500 NM display limit.

## DD-005 — Live DIS receiver: std-only UDP on the LAN
**Date:** 2026-11-10
**Status:** Decided

### Decision
Receive DIS with **`std::net::UdpSocket` only**, bound to `0.0.0.0:<port>`,
accepting broadcast and unicast, and joining one IPv4 multicast group when
`--group` is given (`src/dis/receiver.rs`). Each datagram is split into PDUs
by header length, so v7 bundles work. Entities not heard for **12 s** are
dropped (IEEE 1278.1 default heartbeat 5 s × multiplier 2.4).

### Rationale
- DIS traffic is on the local LAN (per Bobby, 2026-11-10); nothing leaves it.
- Needs no crates, so DD-001 still holds and the one-way-download target is unaffected.
- Loopback tests and a live run prove it end to end — TESTLOG 2026-11-10.

### Alternatives Considered
- **`tokio` async socket:** rejected for now — one socket, one thread, a 200 ms poll is enough.
- **Fixed multicast `239.255.1.1:30000`** (the old `DisConfig` default): rejected —
  invented values; DIS sites commonly use broadcast on port 3000. Default is now
  port 3000, no group.

### Known Gaps
- ⚖ Port, broadcast vs multicast, and exercise ID must be confirmed against the simulators on site.
- No `SO_REUSEADDR` in `std`, so a second listener on the same host and port fails to bind.
- Dead reckoning is not applied between updates; entities jump at each Entity State PDU.

## DD-004 — DIS wire format follows IEEE 1278.1
**Date:** 2026-11-10
**Status:** Decided — supersedes the parser in place before commit `cb76995`

### Decision
Rewrite `src/dis/packet.rs` to the **IEEE 1278.1 wire format** (versions 6
and 7): big-endian; header `version, exercise, type, family, timestamp, length,
status, padding`; Entity State (type 1) at 144 bytes plus 16 per variable
parameter, location as geocentric ECEF `f64` metres, orientation as ECEF-relative
Euler angles in radians. Decode **Entity State** and **Remove Entity** (type 12);
report every other type by number without decoding.

### Rationale
- The old parser was not DIS: little-endian, wrong header order, fixed-point
  lat/lon, and invented type numbers (2 "Death", 4 "Recreation", 5 "Detection"
  are Fire, Collision, and Service Request in the standard). It could not read
  any real simulator's traffic.
- Offsets cross-checked against two independent implementations, downloaded
  2026-11-10: Wireshark `epan/dissectors/packet-dis.c` (type, family, kind and
  domain enums; big-endian throughout) and Open-DIS `opendis/dis7.py`
  (field order of header, Entity State, Remove Entity; `'>'` struct packing).
- A third, separately written encoder (Python `struct`) round-trips exactly
  through the binary — TESTLOG 2026-11-10.

### Alternatives Considered
- **Keep the old format as a "simplified DIS":** rejected — no simulator emits it.
- **Use an Open-DIS crate:** deferred, not rejected — the hand parser is ~200
  lines for two PDUs; revisit if more PDU types are needed (DD-001 policy).

### Known Gaps
- Force ID values (1 Friendly, 2 Opposing, 3 Neutral) are from SISO-REF-010
  UID 6 as recalled; neither downloaded source enumerates them.
- Entity type → `Activity` needs SISO-REF-010 category tables; every live
  entity currently shows Activity `?`.
- Not yet tested against a capture from a real simulator.

## DD-001 — Zero external dependencies

**Date:** 2026-11-08  **Status:** Final — *superseded by DD-010 (2026-11-10): vendored crates allowed for the GUI*

**Problem:** The sandbox has no network access, so crates.io downloads are
impossible. The initial plan targeted `egui`/`eframe` for the GUI.

**Decision:** Defer all external dependencies. Implement the core logic
(DIS parsing, UTM projection, NATO symbology) as pure `std`-only Rust.
The `demo` subcommand renders a text-based moving map as proof of concept.
The egui GUI will be added once network access is available (Phase 5 in
PLAN.md).

**Consequences:**
- The demo output is text-based (terminal escape codes for color).
- No real-time DIS reception until a UDP receiver is added.
- The binary is ~2 MB without debug symbols, runs instantly.

## DD-002 — UTM vs lat/long for the map

**Date:** 2026-11-08  **Status:** Final

**Problem:** Should the map use lat/long with Mercator projection, or UTM?

**Decision:** Use UTM (Universal Transverse Mercator) because:
- UTM meters are uniform within a zone — simpler to map to pixels.
- NATO standard for tactical operations.
- Grid labels in MGRS-style format are natural.

**Consequences:** Each entity must be in the same UTM zone as the ownship
for simple delta math. Zone transitions are handled by the projection code
(UTM zone is computed from longitude). For cross-zone entities, the inverse
projection computes the UTM coordinates which are always in the same zone
as the reference (ownship).

## DD-003 — NATO APP-6A vs MIL-STD-2525D symbology

**Date:** 2026-11-08  **Status:** Final

**Problem:** APP-6A (legacy) vs MIL-STD-2525D (current NATO standard).

**Decision:** Target APP-6A for the text-based renderer (character-based
symbols) and keep 2525D as a future enhancement. APP-6A is widely
recognised and the character encoding (single glyphs) is simpler.

**Consequences:** Future GUI work may need to support both standards via
a configuration flag.
