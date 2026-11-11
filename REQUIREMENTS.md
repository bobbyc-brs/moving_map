---
date: 2026-11-08
---

# Requirements — moving-map-display

A standalone Rust application that renders a **moving map** centred on the
platform's own position, showing ownship and all discovered entities with
standard **NATO APP-6A** symbology. DIS is the sole source of positional data.

## 1.  Scope

### 1.1  In scope

- **Ownship tracking** — receive ownship Entity State PDU via DIS UDP and
  maintain position, heading, pitch, roll, and speed.
- **Entity discovery** — discover remote entities via Entity State,
  Entity Detection, and Entity Recreation PDUs; remove on Entity Death PDU.
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

- Listen on multicast address `239.255.1.1` (configurable) port `30000`
  UDP for HLA DIS PDUs.
- Parse **Entity State PDU** (type 1, family 3, version 2) — the
  authoritative source of entity position and identity.
- Parse **Entity Death PDU** (type 2, family 3, version 2) to remove
  entities no longer transmitting.
- Parse **Entity Recreation PDU** (type 4, family 3, version 2) to
  handle entity respawn.
- Parse **Entity Detection PDU** (type 5, family 3, version 2) for
  sensor contact information.
- Maintain an **entity table** with a 15-second stale-marker timeout per
  entity.

### FR-002 — Map projection

- Use **UTM / MGRS** projection centred on the ownship position.
- Support both **WGS84** and **World Geodetic Datum 1984** ellipsoid.
- Handle UTM zone transitions as ownship crosses zone boundaries.

### FR-003 — NATO symbology rendering

- Render entities according to **APP-6A** standard:
  - Shape encodes **station** (infantry, surface, air, surface vessel,
    installation, equipment, activity, command/control).
  - Colour encodes **status** (friendly = blue, hostile = red,
    unknown = yellow, neutral = green).
  - Modifiers encode **type** (squad, platoon, company, battalion,
    brigade; sedan, APC, tank; fighter, helicopter, cargo, tanker).
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
- **No graphical assets** — all symbology drawn programmatically.

## 6.  Assumptions

- A DIS simulation or SIOS JCATS / JTTS is running and broadcasting
  on the configured multicast address.
- The host machine has a multicast-capable network interface.
- WGS84 is sufficient for positional data (no local datum required).

## Notes

- MIL-STD-2525D (current standard) symbol shapes differ slightly from
  APP-6A (legacy). This specification targets APP-6A for recognisability;
  DD-001 will document the choice.
- MGRS grid is preferred over lat/long grid because it is the NATO
  standard for tactical map referencing.
