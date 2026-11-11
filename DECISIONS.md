---
date: 2026-11-08
---

# Decisions — moving-map-display

## DD-001 — Zero external dependencies

**Date:** 2026-11-08  **Status:** Final

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
