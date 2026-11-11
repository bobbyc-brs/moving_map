---
date: 2026-11-08
updated: 2026-11-10
---

# moving-map-display

A std-only Rust program for a **moving map** of entities reported over
**DIS** (Distributed Interactive Simulation) and drawn with **NATO APP-6A**
symbols. Today it renders that map as text in a terminal from seeded or
file-loaded data; the GUI and the live DIS receiver are not built yet (see
`PLAN.md`).

---

## The pieces

| Path | System | What it is | Stack |
|---|---|---|---|
| `src/dis/` | Library | DIS PDU parser, entity records, entity table | Rust std |
| `src/map/` | Library | WGS84 ↔ UTM projection (per DD-002) | Rust std |
| `src/render/` | Library | APP-6A character symbology for the text map (per DD-003) | Rust std |
| `src/cache/` | Library | Map tile cache: mission-area tile bounds, atomic tile writes | Rust std |
| `src/app/` | Library | Application state shared by receiver and renderer | Rust std |
| `src/main.rs` | Binary | CLI: `demo`, `entities`, `parse`, `parse-stdin`, `cache-gen` | Rust std |

No external crates — all of `[dependencies]` in `Cargo.toml` is empty, per DD-001.

## Data flow

PDU bytes (file or stdin) → `dis::packet` parse → entity table → `map::projection` (WGS84 → UTM) → `render::nato` symbol → 80×24 text map.

## How the pieces talk

- **DIS input**: configured for UDP multicast `239.255.1.1:30000`
  (`DisConfig::default`, `src/dis/mod.rs`). No socket listens yet; PDUs arrive
  from a file or stdin.
- **PDU dispatch**: a 12-byte header, then dispatch on the type byte
  (`process_pdu`, `src/main.rs:222`) — 1 Entity State, 2 Entity Death,
  4 Entity Recreation; 5 (Entity Detection) is recognised but not decoded.
  These numbers may not match IEEE 1278.1 — see `PLAN.md`, *Discrepancies*.
- **Coordinates**: entities carry WGS84 lat/lon; the map works in UTM metres
  relative to ownship, in ownship's zone.
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
| `DECISIONS.md` | Why it is built this way (DD-001 to DD-003) |
| `TEST-PLAN.md` | How each requirement will be verified |
| `SESSION-STATUS.md` | Hand-off notes from the most recent working session |

Shared conventions live in `~/Projects/standards/` — referenced, never copied.
