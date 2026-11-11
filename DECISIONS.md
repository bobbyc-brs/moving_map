---
date: 2026-11-08
---

# Decisions — moving-map-display

Newest first from DD-004 on, per `~/Projects/standards/BOBBYC_STYLE.md`;
DD-001 to DD-003 predate that and stay in their original order below.

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
