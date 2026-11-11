# TESTLOG — moving-map-display

What was actually verified, and when. Newest first; entries are never
rewritten — a later entry corrects an earlier one.

---

## 2026-11-10 — UTM projection held to 10 cm (DD-006)

**Tested:** `src/map/projection.rs` after replacing the UTM series with Krüger
(commit `231b97c`), against Bobby's 10 cm round-trip requirement.

**Results:**
- `cargo test`: **45/45 passing** (was 42; 2 wrong tests replaced, 5 added).
- **Old code, measured first** with the new metre-based test: worst round trip
  **10 063 412 m** at 70°S 77.99°W — test FAILED.
- **New code:** worst round trip **< 0.1 mm** over 45 points (9 latitudes
  70°S–70°N × 5 longitudes from zone centre to ±2.99°).
- Central meridian (75°W): easting 500 000 m; northing equals k0 × the
  meridian arc from an independent Simpson integration within 1 mm at 10°,
  45°, 45.42°, 70°N; 45°N = 4 982 950.40 m.
- (0°, 0°): zone 31, easting 166 021.44 m. Zone boundary: 72°W is zone 19.
- `demo` still plots all 7 seeded entities in their correct relative positions.

**Method:** measured.

**Invalidates:**
- PLAN Phase 2 "✅ Complete, ±50 m" (2026-11-08) — the claim was never
  tested, and the code was off by kilometres to thousands of kilometres. The
  *reasoning* was wrong: round-trip tests passed because forward and inverse
  shared the same errors, and the 0.05° tolerance could not see them.
- The old `zone_18_values` test expected 700–800 km easting at 75°W, the
  zone's central meridian (correct: 500 000 m) — it encoded the bug.
- TEST-PLAN rows that named zone 17 for 75.6°W and zone 18 for 72°W.

---

## 2026-11-10 — Sensors DIS gateway drives `listen` (Sensors CR-001)

**Tested:** Sensors `gateways/dis_gateway.py` (Sensors commit `644203f`) feeding
`moving-map-display listen` over loopback, port 3911, 3 aircraft, 25 s run then
SIGINT. Independent re-run of the Sensors agent's acceptance, using
a scratch script (`verify-cr001.sh`, not committed).

**Results:**
- Sensors gateway unit tests: **11/11 passing** (`python3 -m unittest tests.test_dis_gateway`).
- **Case 1, truth only:** 39 datagrams / 39 PDUs / **0 malformed**; 3 tracked
  (`1.42.1–3`, all Neutral — force 3 per Bobby) at YYT, YHZ, YQB, the departure
  airports of Sensors' first three default routes. Tracked fell **3 → 0** at the
  first redraw after SIGINT (Remove Entity), not after the 12 s timeout.
- **Case 2, `--dis-fused`:** 84 datagrams / 84 PDUs / **0 malformed**; 6 tracked —
  the 3 truth plus `2.42.1–3` as Unknown (force 0), each within ~50 m of its truth
  position (noisy sensor default 50 m). 6 → 0 at the first redraw after SIGINT.
- No simulator processes left running after either case.

**Method:** measured.

**Unblocks:** Sensors as the scenario driver for moving-map development.

**Invalidates:** nothing. Known limit carried forward: Sensors sends each
aircraft's *initial* great-circle bearing for the whole flight, so DIS heading
drifts from the true track on long routes (Sensors CR-001 notes).

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
