---
date: 2026-11-08
updated: 2026-11-10
---

# Requirements — moving-map-display

A standalone Rust application that renders a **moving map** centred on the
platform's own position, showing ownship and all discovered entities with
standard **NATO APP-6A** symbology. DIS is the sole source of positional data.

## 1.  Scope

### 1.1  In scope

- **Ownship tracking** — receive ownship Entity State PDU via DIS UDP and
  maintain position, heading, pitch, roll, and speed.
- **Entity discovery** — discover remote entities via Entity State PDUs;
  remove on Remove Entity PDU or heartbeat timeout. *Revised 2026-11-10
  (DD-004): "Entity Death / Recreation / Detection" PDUs do not exist in
  IEEE 1278.1.*
- **Moving map render** — render a top-down cartographic view with a
  centring mechanism that keeps ownship in view while the map scrolls
  beneath.
- **NATO symbology** — render entities to MIL-STD-2525B / APP-6A standard
  (shape, colour, modifier, and annotation text).
- **Scale and range rings** — adjustable display range (1 NM … 500 NM) with
  annular range rings and a scale bar.
- **Grid overlay** — display MGRS grid lines at the current zoom level.
- **Minimal UI controls** — zoom slider, range-ring toggle, grid toggle.
- **Entity info panel** — click/tap an entity to display its DIS attributes
  in a side panel.

### 1.2  Out of scope

- Two-way DIS — the application is **receive-only** for entity data.
- Terrain or satellite imagery basemaps — vector grid lines only.
- Multi-platform / fleet view — single ownship at a time.
- Replay / recording of DIS traffic.
- Networking configuration UI — DIS multicast address and port are
  compile-time constants or CLI flags.
- Mobile / embedded deployment — Linux desktop only (x86_64).

## 2.  User needs

| # | Need | Priority |
|---|---|---|
| UN-001 | Operator must see ownship position and heading at all times. | Must |
| UN-002 | Operator must identify friend, foe, and unknown entities at a glance. | Must |
| UN-003 | Operator must zoom from tactical (1 NM) to operational (500 NM) without losing context. | Should |
| UN-004 | Operator must identify any entity by its DIS attributes. | Should |
| UN-005 | Display must remain above 30 fps at 1080p with up to 200 entities. | Should |

## 3.  Functional requirements

### FR-001 — DIS reception

*Revised 2026-11-10 (DD-004, DD-005). The original text specified PDU types,
family, version and byte layout that are not IEEE 1278.1 — see PLAN.md,
Discrepancies #5.*

- Receive DIS on the **local LAN**: UDP broadcast or unicast on a configurable
  port (default **3800**, DD-007), optionally joining one IPv4 multicast group.
  Filter by exercise ID — default **1** (DD-008); `--exercise any` accepts all.
- Parse IEEE 1278.1 **protocol versions 6 and 7**, big-endian, including
  bundled datagrams (several PDUs per datagram).
- Parse **Entity State PDU** (type 1, family 1) — the authoritative source of
  entity position (geocentric ECEF), orientation, identity, force ID, entity
  type and marking.
- Parse **Remove Entity PDU** (type 12, family 5) to remove an entity at once.
- Count, but do not decode, all other PDU types; never fail on them.
- Maintain an **entity table**; drop an entity after **12 s** without an
  Entity State PDU (IEEE 1278.1 default heartbeat 5 s × multiplier 2.4).

### FR-002 — Map projection

- Use **UTM / MGRS** projection centred on the ownship position.
- Use the **WGS84** ellipsoid. *(Revised 2026-11-10: the original line listed
  "WGS84 and World Geodetic Datum 1984" — the same datum.)*
- **Accuracy:** projection round trip (lat/lon → UTM → lat/lon) within
  **10 cm** anywhere in a zone. *Added 2026-11-10 — Bobby: 10 cm is the right
  bar for aircraft; met with sub-millimetre margin (DD-006, TESTLOG).*
- Handle UTM zone transitions as ownship crosses zone boundaries.

### FR-003 — NATO symbology rendering

- Render entities according to **APP-6A** standard:
  - Shape encodes **station** (infantry, surface, air, surface vessel,
    installation, equipment, activity, command/control).
  - Colour encodes **status** (friendly = blue, hostile = red,
    unknown = yellow, neutral = green).
  - Modifiers encode **type** (squad, platoon, company, battalion,
    brigade; sedan, APC, tank; fighter, helicopter, cargo, tanker).
  - *Added 2026-11-10 (Bobby):* civilian **cars, trucks, buses (incl. school
    buses)** and **horses**, mapped from the DIS entity type (SISO-REF-010:
    land categories 81 car, 82 bus / sub 12 school bus, 83 cargo truck;
    life form, category 200 mammal, sub 80 horse). Road-following is future work.
    *Note 2026-11-10:* Wireshark 4.2.6 names categories 81–83 but shows car
    sub 10, school-bus sub 12 and life-form 200 / 80 / 90 as bare numbers —
    those codes are not yet confirmed against SISO-REF-010 itself.
  - Country is the **SISO-REF-010** code, not ISO 3166: Canada = **39**
    (124 is Lesotho). Corrected 2026-11-10; see TESTLOG.
  - Annotate with **T-Grid reference**, **call sign**, and
    **equipment count**.
- Draw ownship as a distinct aircraft symbol with heading vector.

### FR-004 — Moving map view

- Centring mode: ownship remains at a fixed screen offset (default
  **30% from bottom**, 50% horizontal centre).
- Smooth scroll interpolation with configurable lag (0–2 s).
- Range rings at regular intervals (e.g. 0.5 ×, 1.0 ×, 2.0 × the
  current range).
- MGRS grid lines with labels.
- Scale bar in nautical miles and kilometres.

### FR-005 — UI controls

- Zoom slider (range: 1 NM to 500 NM).
- Range rings on/off toggle.
- MGRS grid on/off toggle.
- Entity info panel (toggle).
- Entity filtering by status (F / N / U / A).

### FR-006 — Entity info panel

- Display on click/tap on an entity symbol.
- Show: entity ID, name/callsign, station, activity, type, status,
  position (lat/long/MGRS), heading, speed, altitude, and last-heard
  timestamp.

## 4.  Performance requirements

| Metric | Target | Condition |
|---|---|---|
| Render framerate | ≥ 30 fps | 1080p, 200 entities |
| Render framerate | ≥ 60 fps | 1080p, 50 entities |
| DIS packet processing latency | < 50 ms | PDU received to entity table updated |
| Memory footprint | < 200 MB | 200 entities, full UI |
| Startup time | < 3 s | Cold start to first frame |

## 5.  Constraints

- **Rust 2021 edition** (or latest stable LTS).
- **Linux x86_64 only** — Wayland or X11 windowing.
- **No network access** beyond DIS UDP multicast — no HTTP, no DNS.
- **Single binary** — no plugin system, no external data files.
- ⚖ *Noted 2026-11-10 — conflicts with work already done: the tile cache
  (`src/cache/`, `cache-gen`) assumes downloaded map tiles held as external
  files, while this section and §1.2 say "vector grid only, no external data
  files, no HTTP". The deployed target allows one-way download from approved
  sites only. Bobby to decide whether basemap tiles are in scope (PLAN.md ⚖).*
- **No graphical assets** — all symbology drawn programmatically.

## 6.  Assumptions

- A DIS simulation or SIOS JCATS / JTTS is running and sending DIS on the
  local LAN (broadcast, unicast or multicast — confirm on site, PLAN.md ⚖).
  For development, Sensors' DIS gateway stands in (Sensors CR-001).
- The host machine has a network interface on that LAN.
- WGS84 is sufficient for positional data (no local datum required).

## Notes

- MIL-STD-2525D (current standard) symbol shapes differ slightly from
  APP-6A (legacy). This specification targets APP-6A for recognisability;
  DD-003 documents the choice. *(Revised 2026-11-10: said DD-001.)*
- MGRS grid is preferred over lat/long grid because it is the NATO
  standard for tactical map referencing.
