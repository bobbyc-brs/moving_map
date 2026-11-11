//! Screen ↔ ground transform for the moving map (no egui types, so it is
//! unit-tested on its own).
//!
//! Ground positions are projected into **one held UTM zone** (DD-002, DD-011)
//! so the whole picture shares one plane. The plane is rotated so that true
//! north is up (north-up) or ownship's true heading is up (track-up), allowing
//! for grid convergence — grid north and true north differ by up to ~2° at a
//! zone edge at 45°N.

use crate::map::mgrs::convergence_deg;
use crate::map::projection::{EastingNorthing, Hemisphere, LatLon};

pub const M_PER_NM: f64 = 1852.0;
const FALSE_NORTHING_SOUTH: f64 = 10_000_000.0;
const EARTH_RADIUS_M: f64 = 6_371_008.8;

/// Ownship must be this far (°) past its held zone's edge before the map
/// switches zone — stops the grid flipping back and forth on the boundary.
pub const ZONE_HYSTERESIS_DEG: f64 = 0.5;

/// Display ranges offered by the − / + buttons (NM, REQUIREMENTS UN-003).
pub const RANGE_STEPS_NM: [f64; 9] = [1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 200.0, 500.0];

pub fn natural_zone(lon: f64) -> u32 {
    (((lon + 180.0) / 6.0).floor() as i64).clamp(0, 59) as u32 + 1
}

/// Keep `held` unless `lon` is more than [`ZONE_HYSTERESIS_DEG`] outside it.
pub fn hold_zone(held: Option<u32>, lon: f64) -> u32 {
    let natural = natural_zone(lon);
    match held {
        None => natural,
        Some(z) if z == natural => z,
        Some(z) => {
            let west = -180.0 + 6.0 * (z - 1) as f64;
            let east = west + 6.0;
            if lon < west - ZONE_HYSTERESIS_DEG || lon > east + ZONE_HYSTERESIS_DEG {
                natural
            } else {
                z
            }
        }
    }
}

/// Grid easting/northing in `zone`, with northing continuous across the
/// equator (southern points negative rather than offset by 10 000 km).
pub fn grid(lat: f64, lon: f64, zone: u32) -> (f64, f64) {
    let en = LatLon { lat, lon }.to_utm_zone(zone);
    let n = if en.hemisphere == Hemisphere::South { en.northing - FALSE_NORTHING_SOUTH } else { en.northing };
    (en.easting, n)
}

fn grid_to_latlon(e: f64, n: f64, zone: u32) -> (f64, f64) {
    let (northing, hemisphere) = if n < 0.0 {
        (n + FALSE_NORTHING_SOUTH, Hemisphere::South)
    } else {
        (n, Hemisphere::North)
    };
    let ll = EastingNorthing { easting: e, northing, zone, hemisphere }.to_latlon();
    (ll.lat, ll.lon)
}

#[derive(Debug, Clone, Copy)]
pub struct View {
    pub zone: u32,
    /// Grid coordinates of the point drawn at `centre_px`.
    pub centre_e: f64,
    pub centre_n: f64,
    pub centre_px: (f32, f32),
    pub m_per_px: f64,
    /// True bearing (°) that points up the screen: 0 north-up, heading track-up.
    pub up_true_deg: f64,
    /// Clockwise rotation (rad) from grid azimuth to screen azimuth.
    rot: f64,
}

impl View {
    pub fn new(zone: u32, centre: (f64, f64), centre_px: (f32, f32), m_per_px: f64, up_true_deg: f64) -> Self {
        let (centre_e, centre_n) = grid(centre.0, centre.1, zone);
        let gamma = convergence_deg(centre.0, centre.1, zone);
        Self {
            zone, centre_e, centre_n, centre_px, m_per_px, up_true_deg,
            rot: (gamma - up_true_deg).to_radians(),
        }
    }

    /// Grid coordinates → screen pixels.
    pub fn grid_to_screen(&self, e: f64, n: f64) -> (f32, f32) {
        let (de, dn) = (e - self.centre_e, n - self.centre_n);
        let (s, c) = self.rot.sin_cos();
        let er = de * c + dn * s;
        let nr = dn * c - de * s;
        (self.centre_px.0 + (er / self.m_per_px) as f32, self.centre_px.1 - (nr / self.m_per_px) as f32)
    }

    pub fn to_screen(&self, lat: f64, lon: f64) -> (f32, f32) {
        let (e, n) = grid(lat, lon, self.zone);
        self.grid_to_screen(e, n)
    }

    /// Screen pixels → grid coordinates.
    pub fn screen_to_grid(&self, x: f32, y: f32) -> (f64, f64) {
        let er = (x - self.centre_px.0) as f64 * self.m_per_px;
        let nr = (self.centre_px.1 - y) as f64 * self.m_per_px;
        let (s, c) = self.rot.sin_cos();
        (self.centre_e + er * c - nr * s, self.centre_n + er * s + nr * c)
    }

    pub fn to_latlon(&self, x: f32, y: f32) -> (f64, f64) {
        let (e, n) = self.screen_to_grid(x, y);
        grid_to_latlon(e, n, self.zone)
    }

    /// Screen offset (px) of a ground vector given as true bearing and length.
    pub fn true_vector(&self, bearing_deg: f64, length_m: f64) -> (f32, f32) {
        let a = (bearing_deg - self.up_true_deg).to_radians();
        ((length_m * a.sin() / self.m_per_px) as f32, (-length_m * a.cos() / self.m_per_px) as f32)
    }

    /// Screen angle (rad, clockwise from screen-up) of a true bearing.
    pub fn screen_angle(&self, bearing_deg: f64) -> f32 {
        (bearing_deg - self.up_true_deg).to_radians() as f32
    }
}

/// Metres per pixel so that `range_nm` spans from the centre to the nearer
/// screen edge, less a margin.
pub fn m_per_px_for_range(range_nm: f64, width: f32, height: f32) -> f64 {
    let half = (width.min(height) / 2.0 - 24.0).max(40.0) as f64;
    range_nm * M_PER_NM / half
}

/// Range-ring spacing (NM): the largest step that still gives ≥ 3 rings.
pub fn ring_step_nm(range_nm: f64) -> f64 {
    const STEPS: [f64; 13] = [0.1, 0.25, 0.5, 1.0, 2.0, 2.5, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0];
    STEPS.iter().rev().copied().find(|s| range_nm / s >= 3.0).unwrap_or(0.1)
}

/// Grid spacing (m): the finest of 100 m, 1 km, 10 km, 100 km that keeps
/// lines at least `min_px` apart.
pub fn grid_step_m(m_per_px: f64, min_px: f64) -> f64 {
    [100.0, 1_000.0, 10_000.0, 100_000.0].into_iter().find(|s| s / m_per_px >= min_px).unwrap_or(100_000.0)
}

/// Next smaller / larger preset range.
pub fn step_range(range_nm: f64, larger: bool) -> f64 {
    if larger {
        RANGE_STEPS_NM.iter().copied().find(|r| *r > range_nm * 1.001).unwrap_or(500.0)
    } else {
        RANGE_STEPS_NM.iter().rev().copied().find(|r| *r < range_nm * 0.999).unwrap_or(1.0)
    }
}

/// Great-circle range (m) and initial true bearing (°) from a to b, on a
/// sphere of mean radius — within 0.5 % of the ellipsoid, enough for a
/// readout (positions themselves use the ellipsoid).
pub fn range_bearing(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let (p1, p2) = (a.0.to_radians(), b.0.to_radians());
    let dl = (b.1 - a.1).to_radians();
    let dp = p2 - p1;
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    let range = 2.0 * EARTH_RADIUS_M * h.sqrt().min(1.0).asin();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    let bearing = (y.atan2(x).to_degrees() + 360.0) % 360.0;
    (range, bearing)
}

/// Move a point by a true bearing and distance (flat-earth; used for dead
/// reckoning over a few seconds, where the error is millimetres).
pub fn offset(lat: f64, lon: f64, bearing_deg: f64, dist_m: f64) -> (f64, f64) {
    let b = bearing_deg.to_radians();
    let dlat = dist_m * b.cos() / 111_320.0;
    let dlon = dist_m * b.sin() / (111_320.0 * lat.to_radians().cos().max(1e-6));
    (lat + dlat, lon + dlon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_is_held_within_hysteresis() {
        // Flying west out of zone 18 (78°W–72°W) into 17.
        assert_eq!(hold_zone(None, -77.9), 18);
        assert_eq!(hold_zone(Some(18), -78.3), 18); // 0.3° past the edge: hold
        assert_eq!(hold_zone(Some(18), -78.6), 17); // 0.6° past: switch
        assert_eq!(hold_zone(Some(17), -77.8), 17); // and back again: hold 17
        assert_eq!(hold_zone(Some(17), -77.4), 18);
    }

    #[test]
    fn screen_round_trip_north_up_and_track_up() {
        for up in [0.0, 54.5, 270.0] {
            let v = View::new(18, (45.4215, -75.6972), (640.0, 400.0), 25.0, up);
            for (lat, lon) in [(45.4215, -75.6972), (45.5, -75.5), (45.3, -76.0)] {
                let (x, y) = v.to_screen(lat, lon);
                let (lat2, lon2) = v.to_latlon(x, y);
                // f32 pixels limit this to ~1 mm at 25 m/px.
                assert!((lat2 - lat).abs() < 1e-6 && (lon2 - lon).abs() < 1e-6, "up {up}: {lat},{lon} → {lat2},{lon2}");
            }
        }
    }

    #[test]
    fn true_north_is_up_despite_convergence() {
        // Near the zone edge convergence is ~2°: a point due true-north of
        // the centre must still be straight up the screen in north-up.
        let v = View::new(18, (45.0, -72.2), (500.0, 500.0), 10.0, 0.0);
        let (x, y) = v.to_screen(45.05, -72.2);
        assert!((x - 500.0).abs() < 0.5, "x {x}");
        assert!(y < 500.0);
    }

    #[test]
    fn track_up_puts_heading_up() {
        // Heading 090: a point due east of the centre is straight up.
        let v = View::new(18, (45.0, -75.0), (500.0, 500.0), 10.0, 90.0);
        let (x, y) = v.to_screen(45.0, -74.95);
        assert!((x - 500.0).abs() < 1.0 && y < 500.0, "({x},{y})");
        let (dx, dy) = v.true_vector(90.0, 1000.0);
        assert!(dx.abs() < 1e-3 && (dy + 100.0).abs() < 1e-3);
    }

    #[test]
    fn ring_and_grid_steps() {
        assert_eq!(ring_step_nm(25.0), 5.0);
        assert_eq!(ring_step_nm(1.0), 0.25);
        assert_eq!(ring_step_nm(500.0), 100.0);
        assert_eq!(grid_step_m(10.0, 60.0), 1_000.0);
        assert_eq!(grid_step_m(500.0, 60.0), 100_000.0);
        assert_eq!(step_range(25.0, true), 50.0);
        assert_eq!(step_range(25.0, false), 10.0);
        assert_eq!(step_range(30.0, false), 25.0);
    }

    #[test]
    fn range_bearing_known() {
        // 1′ of latitude ≈ 1 NM (1 853 m on the mean sphere), due north.
        let (r, b) = range_bearing((45.0, -75.0), (45.0 + 1.0 / 60.0, -75.0));
        assert!((r - 1853.2).abs() < 1.0 && b.abs() < 1e-9, "{r} {b}");
        let (_, b) = range_bearing((0.0, 0.0), (0.0, 1.0));
        assert!((b - 90.0).abs() < 1e-9);
    }
}
