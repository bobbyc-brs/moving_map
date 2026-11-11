---
date: 2026-11-10
---

# Session Status — moving-map-display

Hand-off for the next session (written late 2026-11-10). Replaces the
evening version (in git history).

## Current state

- **Touch-screen GUI works** — `cargo run -- gui --ownship 1.42.1` with any
  Sensors scenario: APP-6A symbols for every DIS type Sensors sends, OSM
  roads and water underneath (Ottawa area), north-up / track-up, rings,
  MGRS grid and readout, trails, stale dimming, Day / Dusk / Night.
  Screenshots in TESTLOG; DD-010, DD-011, DD-012.
- **`cargo test`: 67/67.** Needs **Rust 1.99.1 from rustup**
  (`devel-scripts/01-install-rust.sh`); the distro `rustc` 1.80 cannot build it.
- **UTM zone is held** across a boundary with 0.5° hysteresis (`gui::view`).
- **Pushed** to `github.com/bobbyc-brs/moving_map` (remote is SSH).
- DIS defaults: port **3800**, exercise **1**, broadcast (DD-007 – DD-009).

## Shell note

This Claude session's PATH predates rustup, so plain `cargo` here is the old
`/usr/bin/cargo`. Scripts call `~/.cargo/bin/cargo` explicitly. A fresh login
shell (or `source ~/.cargo/env`) fixes PATH.

## Next here

See `PLAN.md`, *Update 2026-11-10 (late)*: declutter; frame rate with
load-200; basemap polish (filled lakes, names, more areas); latency; try the
real touch screen; offline build from the vendor tarball.

## Waiting on Sensors

**CR-005** (`57496cd`): symbol gallery, ownship manoeuvres, dead-reckoning
send policy, pcap replay, demo tour. Session `sensors-71` was told.

## ⚖ Waiting on Bobby

Production road source (simulator terrain DB vs OSM); satellite-imagery source
with an offline-caching licence (if wanted); chart and elevation formats
(future); a real simulator capture.

## Standards

`~/Projects/standards/` — reference only, never fork: `BOBBYC_STYLE.md`,
`SCRIPTING.md`, `CHANGE-REQUESTS.md`. Multi-step shell work goes in
`devel-scripts/` as named scripts.
