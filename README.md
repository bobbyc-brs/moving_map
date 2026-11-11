---
date: 2026-11-08
updated: 2026-11-10
---

# moving-map-display

A std-only Rust program for a **moving map** of entities reported over
**DIS** (Distributed Interactive Simulation) and drawn with **NATO APP-6A**
symbols. Today it receives DIS from the LAN and renders the map as text in a
terminal; the GUI is not built yet (see `PLAN.md`).

---

## The pieces

| Path | System | What it is | Stack |
|---|---|---|---|
| `src/dis/` | Library | DIS PDU parser (IEEE 1278.1), UDP receiver, entity table | Rust std |
| `src/map/` | Library | WGS84 ↔ UTM projection (per DD-002) | Rust std |
| `src/render/` | Library | APP-6A character symbology for the text map (per DD-003) | Rust std |
| `src/cache/` | Library | Map tile cache: mission-area tile bounds, atomic tile writes | Rust std |
| `src/app/` | Library | Application state shared by receiver and renderer | Rust std |
| `src/main.rs` | Binary | CLI: `listen`, `demo`, `entities`, `parse`, `parse-stdin`, `cache-gen` | Rust std |

No external crates — all of `[dependencies]` in `Cargo.toml` is empty, per DD-001.

## Data flow

LAN UDP datagram → `dis::receiver` → `dis::packet` parse (ECEF → WGS84) → entity table → `map::projection` (WGS84 → UTM) → `render::nato` symbol → 80×24 text map.

## How the pieces talk

- **DIS input**: UDP on the local LAN, default port 3000, broadcast or unicast;
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
| `REQUIREMENTS.md` | What the system must do |
| `PLAN.md` | Phases, status, open ⚖ decisions, known discrepancies |
| `DECISIONS.md` | Why it is built this way (DD-001 to DD-005) |
| `TEST-PLAN.md` | How each requirement will be verified |
| `TESTLOG.md` | What was actually verified, and when |
| `SESSION-STATUS.md` | Hand-off notes from the most recent working session |

Shared conventions live in `~/Projects/standards/` — referenced, never copied.
