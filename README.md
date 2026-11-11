---
date: 2026-11-08
updated: 2026-11-10
---

# moving-map-display

A Rust **moving map** for a touch screen, showing entities reported over
**DIS** (Distributed Interactive Simulation) with **NATO APP-6A** symbols,
centred on ownship — for an in-flight / simulator crew display. It receives DIS
from the LAN and draws the map in a GUI (`gui`) or as text in a terminal
(`listen`).

---

## The pieces

| Path | System | What it is | Stack |
|---|---|---|---|
| `src/dis/` | Library | DIS PDU parser (IEEE 1278.1), UDP receiver, entity table, attitude | Rust std |
| `src/map/` | Library | WGS84 ↔ UTM (Krüger series, 10 cm; DD-006), ECEF ↔ WGS84, MGRS, grid convergence | Rust std |
| `src/render/` | Library | DIS entity type → APP-6A dimension and icon (`symbol`, DD-011); text-map characters (`nato`) | Rust std |
| `src/gui/` | Library | Touch-screen map: view transform, DIS feed thread, APP-6A painting, panels (DD-011) | egui/eframe 0.37 |
| `src/cache/` | Library | Map tile cache: mission-area tile bounds, atomic tile writes (placeholder tiles only) | Rust std |
| `src/main.rs` | Binary | CLI: `gui`, `listen`, `demo`, `entities`, `parse`, `parse-stdin`, `cache-gen` | Rust std |
| `data/basemap/` | Data | Vector basemap — OpenStreetMap roads, rail, rivers, lakes for the Ottawa area (DD-012) | text |
| `devel-scripts/` | Scripts | Install Rust, vendor crates, fetch basemap, verify against Sensors, screenshot the GUI | bash, python |

*Revised 2026-11-10 (DD-010):* the GUI uses **one external crate, `eframe`**
(with its dependencies), vendored for offline builds. Everything else is std-only.
Building needs **Rust 1.99.1 from rustup** — the distro `rustc` is too old;
`devel-scripts/01-install-rust.sh` installs it (see `DEVELOPING.md`).

## Data flow

LAN UDP datagram → `dis::receiver` (feed thread) → `dis::packet` parse (ECEF → WGS84, attitude) → entity table → `gui::view` (WGS84 → held UTM zone → screen) → `gui::draw` APP-6A symbol over the `data/basemap` roads and water, on the touch screen.

## How the pieces talk

- **DIS input**: UDP on the local LAN, default port 3800 (DD-007), broadcast or unicast;
  `--group` joins one IPv4 multicast group (`src/dis/receiver.rs`, DD-005).
  Nothing is fetched from outside the LAN.
- **PDU wire format**: IEEE 1278.1 v6/v7, big-endian, 12-byte header
  (DD-004). Decodes Entity State (type 1) and Remove Entity (type 12); other
  types are counted and named by number. Entities time out after 12 s.
- **Coordinates**: DIS location is geocentric ECEF; the parser converts to WGS84
  lat/lon, and the map works in UTM metres relative to ownship, in ownship's zone.
- **Tile cache contract**: `<root>/cache.{z}/{x}/{y}.png`, written to `.tmp`
  then renamed, with a `{y}.png.downloaded` sentinel marking a complete tile
  (`src/cache/manager.rs:29-60`). `cache-gen` currently writes 1×1 placeholder
  tiles, not downloaded ones.

---

## Start here

| Document | Answers |
|---|---|
| `DEVELOPING.md` | How to build, run, and test; project conventions |
| `DEMO.md` | How to run the demo with Sensors, and what to look for |
| `REQUIREMENTS.md` | What the system must do |
| `PLAN.md` | Phases, status, open ⚖ decisions, known discrepancies |
| `DECISIONS.md` | Why it is built this way (DD-001 to DD-012) |
| `TEST-PLAN.md` | How each requirement will be verified |
| `TESTLOG.md` | What was actually verified, and when |
| `SESSION-STATUS.md` | Hand-off notes from the most recent working session |

Shared conventions live in `~/Projects/standards/` — referenced, never copied.
