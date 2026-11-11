---
date: 2026-11-08
---

# moving-map-display

A zero-dependency Rust application that displays a **moving map** with
entities tracked via **DIS** (Distributed Interactive Simulation),
rendered with **NATO APP-6A** symbology.

## The pieces

| Path            | System       | What it is                       | Stack      |
|-----------------|--------------|----------------------------------|------------|
| `src/dis/`      | Library      | DIS PDU parser + entity table    | Rust std   |
| `src/map/`      | Library      | WGS84 → UTM projection           | Rust std   |
| `src/render/`   | Library      | NATO APP-6A symbol lookup        | Rust std   |
| `src/main.rs`   | Binary       | CLI: text-map demo + PDU parser  | Rust std   |

## Data flow

```
DIS UDP PDU → dis::packet::parse() → EntityRecord → map::projection::to_utm()
                                                              → render::nato::entity_char()
                                                              → text map output
```

## Start here

1. **Read the docs**: `REQUIREMENTS.md` (what), `PLAN.md` (how), `TEST-PLAN.md` (proof).
2. **Build**: `cargo build`
3. **Run the demo**: `cargo run -- demo`
4. **Parse DIS PDUs**: `cargo run -- parse <file>`

See `DEVELOPING.md` for project conventions.

## Current status

- ✅ DIS PDU parsing (Entity State, Death, Recreation, Detection)
- ✅ WGS84 → UTM forward projection
- ✅ UTM → lat/lon inverse projection (±50 m accuracy)
- ✅ NATO APP-6A character-based symbology
- ✅ Text-based moving map renderer (grid, range rings, scale bar)
- ⏳ egui GUI (blocked: no crates.io network access)
- ⏳ Real DIS UDP receiver (blocked: needs network stack)
