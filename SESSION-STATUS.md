---
date: 2026-11-10
---

# Session Status — moving-map-display

Hand-off for the next session. Replaces the 2026-11-10 morning note (in git
history at `96c1a76`), whose next steps and architecture were overtaken the
same day.

## Current state

- **`cargo test`: 45/45 passing.** Evidence in `TESTLOG.md`.
- **DIS:** IEEE 1278.1 parser (Entity State, Remove Entity; DD-004) and a
  std-only LAN receiver, `cargo run -- listen` (DD-005). The original parser
  was not DIS and was rewritten.
- **Projection:** UTM by Krüger series, 10 cm round trip required and met
  (< 0.1 mm measured; DD-006). The original UTM code was off by kilometres.
- **Driven by Sensors:** `~/Projects/Sensors` emits DIS via its gateway
  (Sensors CR-001, verified both truth-only and with sensed tracks).
- **Remote:** `origin` = `https://github.com/bobbyc-brs/moving_map.git`,
  added 2026-11-10, **not yet pushed**.

## In flight

- **Sensors CR-002 → CR-004** (filed 2026-11-10, handed to the Sensors
  session): scenario files, ownship helicopter, mixed entity types and sides
  (incl. cars, trucks, school buses, horses), lifecycle / 200-entity load /
  zone-crossing scenarios, fault injection, pcap recording, multicast. When
  each is Processed, re-run its acceptance here and log it in `TESTLOG.md`.

## Next here

1. Map DIS entity types → `Activity` / symbols, incl. the new civilian and
   animal types (PLAN Phase 1, REQUIREMENTS FR-003).
2. Hold ownship's UTM zone across a boundary (`to_utm_zone`, DD-006 gaps) —
   test with Sensors CR-003's zone-crossing scenario.
3. Use CR-004 recordings as `parse` regression vectors; count faults in `listen`.

## ⚖ Waiting on Bobby

See `PLAN.md`, *⚖ Decisions*: DIS site settings (port, multicast, exercise),
dependency policy (std-only vs vendored crates — gates the GUI), basemap scope
(tiles vs vector-only), a real simulator capture.

## Standards

`~/Projects/standards/` — reference only, never fork: `BOBBYC_STYLE.md`,
`SCRIPTING.md`, `CHANGE-REQUESTS.md`.
