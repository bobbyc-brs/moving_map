---
date: 2026-11-11
---

# DEMO — running the moving map with Sensors

How to show the touch-screen moving map with live DIS from Sensors' CGF
scenarios, and what to look for. Setup and build details are in
`DEVELOPING.md`; what was verified is in `TESTLOG.md`.

---

## Before you start

- **Rust 1.99.1 from rustup** on PATH — open a fresh login shell after
  `bash devel-scripts/01-install-rust.sh`, or run `source ~/.cargo/env`.
  The distro `cargo` (1.80) cannot build the GUI (DD-010).
- `~/Projects/Sensors` checked out at `72897b4` or later (Sensors CR-005).
- Both sides default to **UDP port 3800, exercise 1** (DD-007, DD-008).
- On a **locked** Wayland screen the window never draws; unlock it, or run the
  GUI under X11 with `WAYLAND_DISPLAY= cargo run -- gui …`.

## The demo tour

Two terminals:

```bash
cd ~/Projects/Sensors && python3 -m cgf.runner scenarios/demo-tour.json
cd ~/Projects/moving-map-display && cargo run -- gui --ownship 1.42.1 --range-nm 10
```

The tour is a **600 s cycle that repeats**; entities keep their IDs across
cycles and the ownship never drops out. Scripted events (Sensors CR-005 note):

| Time in cycle | Event | What the map shows |
|---|---|---|
| 0 s → | Ownship helicopter transits from YOW; convoy (truck, AFV, tank), school bus, cars, horse with rider | Symbols over the OSM roads and rivers; trails grow |
| 100 s | Hostile attack helicopter VIPER1 appears ahead, closes, turns away | Red air "tent" with altitude / speed label |
| 300 s | Unknown UAV-X goes silent (no Remove) | Dims after 7 s silent, shows its age, drops at 12 s |
| 360 s | CAR2 sent Remove Entity | Disappears at once |
| 450 s | VIPER1 leaves | — |
| 600 s | Cycle restarts | Only the two late convoy vehicles are removed |

Everything except the ownship sends by **dead reckoning** with a 5 s
heartbeat, as a real simulator does — so dead reckoning and stale dimming are
exercised.

## Things to try on the screen

- **− / +** or pinch: range 1–500 NM. **North up / Track up**.
- **Tap a symbol**: details panel (DIS type, force, MGRS, altitude, speed,
  pitch / roll, range / bearing from ownship, last heard).
- **Tap the map**: drops a mark with MGRS, lat/lon and range / bearing.
- **Drag**: pans; **Centre** returns to ownship.
- **Map, Rings, Grid, Trails, Labels** toggles; **Day → Dusk → Night**;
  **List** shows all entities by range.

Useful flags: `--track-up`, `--scheme night`, `--fullscreen`, `--list`,
`--no-basemap`.

## Other scenarios

| Scenario | Shows | GUI flags |
|---|---|---|
| `symbol-gallery.json` | All 72 symbols — 4 forces × 18 types, incl. submarine, UAV, ship, person, unknown | `--range-nm 2` |
| `ownship-manoeuvres.json` | Orbits both ways, heading through north, hover yaw, pitch and roll | `--track-up --range-nm 2` |
| `ottawa-mixed.json` | Every entity type on all four sides within 25 NM of YOW | `--range-nm 10` |
| `load-200.json` | 200 entities, 200 PDU/s | `--range-nm 100` |
| `zone-crossing.json` | Ownship crosses 78°W (UTM 18 → 17); off the current basemap | `--range-nm 10` |
| `lifecycle.json` | Late appearance, silent timeout, remove and reappear, duplicate IDs | no ownship |

Run any of them with `python3 -m cgf.runner scenarios/<name>.json` in Sensors.

**Replay a capture** instead of a scenario — Sensors' recordings now, a real
simulator capture later:

```bash
cd ~/Projects/Sensors && python3 -m cgf.replay recordings/ottawa-mixed.pcap --speed 1
```

## Unattended checks

```bash
bash devel-scripts/screenshot-gui.sh demo-tour 120 --range-nm 10   # PNG in target/shots/
bash devel-scripts/verify-with-sensors.sh                          # counters, port 3801
```

## Known limits

Symbols overlap at long range (no declutter yet); lakes are outlines; the
basemap covers the Ottawa area only; touch gestures have been tested with a
mouse, not a touch screen. See `PLAN.md`.
