---
date: 2026-11-11
---

# Session Status — moving-map-display

Hand-off for the next session (written 2026-11-11, Bobby paused the project).
Replaces the 2026-11-10 version (in git history).

## Current state

- **Touch-screen GUI works** — `cargo run -- gui --ownship 1.42.1` with any
  Sensors scenario: APP-6A symbols for every DIS type Sensors sends, OSM
  roads and water underneath (Ottawa area), north-up / track-up, rings,
  MGRS grid and readout, trails, stale dimming, Day / Dusk / Night.
  Screenshots in TESTLOG; DD-010, DD-011, DD-012.
- **Demo**: `DEMO.md` — run Sensors' `demo-tour.json` (600 s, repeats) beside
  the GUI; timeline of what to look for, controls, other scenarios, replay.
- **`cargo test`: 67/67.** Needs **Rust 1.99.1 from rustup**
  (`devel-scripts/01-install-rust.sh`); the distro `rustc` 1.80 cannot build it.
- **UTM zone is held** across a boundary with 0.5° hysteresis (`gui::view`).
- **Pushed**: `main` == `origin/main` at `github.com/bobbyc-brs/moving_map`
  (remote is SSH). Sensors also pushed.
- DIS defaults: port **3800**, exercise **1**, broadcast (DD-007 – DD-009).

## Sensors CR-005 — done

Processed by `sensors-71` at Sensors `72897b4`: symbol gallery, ownship
manoeuvres, dead-reckoning send policy, `python -m cgf.replay`, demo tour.
Checked here (TESTLOG 2026-11-11): gallery 72 cells, replay 554/554, live
296/296 and 3400/3400. **Not yet checked here**: manoeuvres, DR send policy,
the full demo tour run end to end.

## Shell note

An old Claude session's PATH may predate rustup, so plain `cargo` there is
`/usr/bin/cargo`. Scripts call `~/.cargo/bin/cargo` explicitly. A fresh login
shell (or `source ~/.cargo/env`) fixes PATH. On a locked Wayland screen the
GUI does not draw; use `WAYLAND_DISPLAY=` (X11).

## Next here

1. Run `ownship-manoeuvres.json` and `demo-tour.json` with the GUI and log
   the result in TESTLOG (track-up through north, hover yaw, DR, stale).
2. `PLAN.md`, *Update 2026-11-10 (late)*: declutter; frame rate with
   load-200; basemap polish (filled lakes, names, more areas); latency
   < 50 ms; try the real touch screen; offline build from the vendor tarball.
3. Confirm five entity codes against SISO-REF-010 itself (car sub 10,
   school bus 82.12, life form 200/80/90).

## ⚖ Waiting on Bobby

Production road source (simulator terrain DB vs OSM); satellite-imagery source
with an offline-caching licence (if wanted); chart and elevation formats
(future); a real simulator capture (replay with `cgf.replay`); the multicast
group, if a site uses multicast.

## Standards

`~/Projects/standards/` — reference only, never fork: `BOBBYC_STYLE.md`,
`SCRIPTING.md`, `CHANGE-REQUESTS.md`. Multi-step shell work goes in
`devel-scripts/` as named scripts; one command per Bash call.
