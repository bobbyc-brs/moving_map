# TESTLOG — moving-map-display

What was actually verified, and when. Newest first; entries are never
rewritten — a later entry corrects an earlier one.

---

## 2026-11-10 — Recordings as regression tests; exercise 1 by default (DD-008)

**Tested:** `src/dis/recordings.rs` over the four `.bin` files copied from
Sensors `0e3c036` into `tests/data/dis/`; the new `DisConfig` default.

**Results:**
- `cargo test`: **50/50 passing** (was 46; 4 recording tests added).
- Each recording: every PDU decodes, no framing error, every Entity State is
  country 39, and the counts are pinned — lifecycle 237 / 5, load-200
  3 020 / 200, ottawa-mixed 541 / 13, zone-crossing 541 / 5 (Entity State /
  Remove), matching the 2026-11-10 entries below.
- Default `exercise_id = Some(1)` pinned by `defaults_port_3800_exercise_1`.

**Method:** measured.

---

## 2026-11-10 — Default port 3800 end to end (DD-007)

**Tested:** Sensors `0c9520f`/`4d1bda0` (`DEFAULT_PORT = 3800`) `cgf.runner
scenarios/ottawa-mixed.json --duration 15` into `listen --ownship 1.42.1`
(`c622a43`), **neither given `--port`**.

**Results:**
- `listen` bound UDP **:3800**; runner sent **285 datagrams / 285 PDUs**,
  `listen` received **285 / 285, malformed 0**; 13 tracked, **13 → 0** on the
  runner's Remove Entity at exit. Nothing left running.
- Resolves the 37-PDU split question in the entry below: **4 339 Entity State /
  223 Remove** is right; the Sensors CR-004 addendum had an addition error
  (corrected in Sensors `4d1bda0`).

**Method:** measured. Sensors' own run of the same check: 301 / 301, 0 malformed.

---

## 2026-11-10 — Sensors recordings regenerated with country 39

**Tested:** Sensors `0e3c036` (country fix, recordings regenerated) with `parse`.

**Results:**
- All four recordings decode, **0 errors**: lifecycle 242, load-200 3 220,
  ottawa-mixed 554, zone-crossing 546 PDUs — counts unchanged.
- Entity State with country 39: **4 339**; with 124: **0**. Remove Entity: 223
  (5 / 200 / 13 / 5). Total 4 562, as the Sensors agent reported; its split
  (4 302 / 260) differs from this one by 37 — not yet reconciled.
- ottawa-mixed carries all 12 catalogue types, e.g. `1.1.39.81.10`,
  `1.1.39.82.12`, `3.1.39.200.80`, `3.1.39.200.90`.

**Method:** measured.

---

## 2026-11-10 — Wireshark (tshark 4.2.6) cross-check; country code corrected

**Tested:** all four Sensors recordings (4 562 PDUs: 4 302 Entity State, 260
Remove Entity), decoded three ways — raw bytes, tshark's DIS dissector on the
`.pcap`, and this repo's `parse` on the `.bin`. Run by the Sensors agent
(Sensors `5b58db7`, CR-002/CR-004 addenda); country enum re-checked here in
`packet-dis.c` (`DIS_PDU_COUNTRY_CANADA_CAN = 39`, `…LESOTHO_LSO = 124`).

**Results:**
- PDU type, exercise, version, entity ID, force, all 7 entity-type fields,
  marking and Remove's receiving ID agree on **every PDU**; ECEF agrees to
  tshark's print precision (5 nm), lat/lon/alt to `parse`'s 6 dp / 0.1 m.
- **Country 124 decodes as Lesotho.** DIS uses SISO-REF-010 country codes, where
  Canada is **39**; 124 is Canada's ISO 3166 numeric. The error was this repo's —
  CR-002's type table said "country 124 = Canada", and Sensors followed it.
- Fixed here: test encoders in `packet.rs`, `dis/mod.rs`, `receiver.rs` now
  use 39; `cargo test` **45/45 passing**. Sensors' fix (catalogue, tests,
  recordings) agreed with the Sensors agent, pending at time of writing.
- Not confirmed by Wireshark 4.2.6 (shown as bare numbers): car sub 10,
  school-bus sub 12, life-form category 200 and horse subs 80 / 90.
- Wireshark limits for anyone reading captures: it dissects only the first
  PDU of a bundled datagram, and decodes random garbage as junk PDUs rather
  than flagging them.

**Method:** measured (by the Sensors agent; enum value checked here).

**Invalidates:** the 2026-11-10 CR-002→004 entry's "types decode exactly as
CR-002 asked" for the country field — they matched the request, but the request
was wrong (`1.1.124.…` should be `1.1.39.…`).

## 2026-11-10 — Sensors CGF scenarios, faults and recordings (Sensors CR-002 → CR-004)

**Tested:** Sensors `1f3f99a`, `477c126`, `a1e3ac9` (CGF runner, scenarios,
fault injection, recordings) against `listen` and `parse` on private port 3911.
Independent re-run of the Sensors agent's acceptance with a scratch script
(`verify-cr002-004.sh`, not committed).

**Results:**
- Sensors unit tests: **46/46 passing**.
- **Recordings** (`Sensors/recordings/*.bin`) decode fully with `parse`, 0
  errors: lifecycle 242, load-200 3 220, ottawa-mixed 554, zone-crossing 546 PDUs.
- **Entity types and forces** in ottawa-mixed decode exactly as CR-002 asked:
  car `1.1.124.81.10`, truck `…83`, bus `…82`, school bus `…82.12`, horse
  `3.1.124.200.80`, horse with rider `…200.90` (force 0), tanks force 1 and 2,
  AFV, attack helicopter, patrol craft, airliner `…87` — 13 entities.
- **ottawa-mixed live**, `--ownship 1.42.1`: 13 tracked, **0 malformed**, all
  four statuses present (3 A, 2 F, 7 N, 1 U); map centred on HELO1 near Ottawa.
- **load-200 live:** **200 tracked**; **4 601 PDUs sent, 4 601 received, 0
  malformed** (Sensors reported 200.1 PDU/s, 7 ms lag); 200 → 0 on exit.
- **Faults** (truncate 0.05, garbage 0.02, unknown types, seed 7): runner
  sent 358 datagrams / 335 PDUs with 15 truncated + 8 garbage; `listen`
  counted **358 / 335 / malformed 23** — exact agreement, no crash, entities
  kept tracking.
- **Zone crossing:** map's zone label went **18 → 17** as ownship crossed 78°W.

**Method:** measured. Not re-run here: lifecycle timings, multicast, and the
full-fault bundle case — reported by the Sensors agent (Sensors CR-003/004 notes).

**Unblocks:** moving-map symbol work for the new types; lifecycle, load and
zone-crossing tests have a driver.

**Invalidates:** CR-003's own text — it said "Ottawa towards Pembroke"
crosses 78°W; Pembroke is at ~77.1°W, so it does not. The Sensors agent flew
near Bancroft/Maynooth instead. The error was the requester's (this repo's).

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
