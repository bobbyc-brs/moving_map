# TESTLOG — moving-map-display

What was actually verified, and when. Newest first; entries are never
rewritten — a later entry corrects an earlier one.

---

## 2026-11-10 — DIS parser rewrite (DD-004) and LAN receiver (DD-005)

**Tested:** `src/dis/packet.rs`, `src/dis/receiver.rs`, `ecef_to_geodetic` in
`src/map/projection.rs`, and the `parse` / `listen` subcommands.

**Results:**
- `cargo test`: **42/42 passing** (was 27). New: 9 packet, 3 receiver, 1
  entity-table, 2 ECEF tests.
- **ECEF ↔ geodetic**: exact at (0°, 0°) and the pole (b = 6 356 752.314245 m);
  round trip within 1e-8° (~1 mm) and 1 mm height at 4 points including 89.9°N.
- **Independent encoder**: 5 PDUs packed with Python `struct` (4 Entity State,
  1 Remove Entity, 604 bytes) decoded by `parse` with every field exact —
  ID, force, entity type, marking, lat/lon to 6 dp, altitude, and heading
  (90°, 225°, 0°, 180° at 45.4°N).
- **Live UDP**: `listen --port 39123 --seconds 4` received 4 datagrams / 13
  PDUs / 0 malformed from the Python sender over loopback; Remove Entity
  dropped `BANDIT` (4 → 3 tracked); map centred on ownship with the hostile
  tank plotted north-east, as placed.

**Method:** measured — all of the above was run, not inferred. The layout the
tests check was read from Wireshark and Open-DIS source (see DD-004); no real
simulator capture has been tested yet.

**Unblocks:** Phase 5 receiver; on-site testing against real simulators.

**Invalidates:**
- `PLAN.md` Phase 1 "✅ Complete" for the DIS layer — the parser it refers to
  was not DIS. The *reasoning* was wrong, not just the claim: the old tests
  encoded and decoded the same invented format, so they could never catch it.
- `PLAN.md` "1 dis/entity test" in the 27 — there was no such test; the 27
  were 11 cache/manager, 13 cache/tile_bounds, 3 projection.
- "±50 m" inverse UTM accuracy remains untested (Discrepancies #4).
