//! MGRS (Military Grid Reference System) from WGS84 lat/lon, and UTM grid
//! convergence. MGRS here is the WGS84 "AA" lettering scheme, 80°S–84°N; the
//! polar UPS areas and the Norway/Svalbard zone exceptions are not handled
//! (DD-011 Known Gaps).

use super::projection::LatLon;

const BANDS: &[u8] = b"CDEFGHJKLMNPQRSTUVWX";
const COLUMN_SETS: [&[u8]; 3] = [b"ABCDEFGH", b"JKLMNPQR", b"STUVWXYZ"];
const ROWS: &[u8] = b"ABCDEFGHJKLMNPQRSTUV";

/// Latitude band letter, or `None` outside 80°S–84°N.
pub fn band_letter(lat: f64) -> Option<char> {
    if !(-80.0..=84.0).contains(&lat) {
        return None;
    }
    let i = (((lat + 80.0) / 8.0).floor() as usize).min(BANDS.len() - 1);
    Some(BANDS[i] as char)
}

/// The 100 km square letters for an easting/northing in a zone.
pub fn square_letters(zone: u32, easting: f64, northing: f64) -> (char, char) {
    let set = COLUMN_SETS[((zone - 1) % 3) as usize];
    let col = ((easting / 100_000.0).floor() as i64 - 1).clamp(0, 7) as usize;
    let row_offset = if zone % 2 == 0 { 5 } else { 0 };
    let row = (((northing / 100_000.0).floor() as i64 + row_offset).rem_euclid(20)) as usize;
    (set[col] as char, ROWS[row] as char)
}

/// MGRS string, e.g. `18T VR 44590 30209`, with `digits` per axis (1–5;
/// 5 = 1 m). `None` outside the UTM latitudes.
pub fn to_mgrs(lat: f64, lon: f64, digits: u32) -> Option<String> {
    let band = band_letter(lat)?;
    let digits = digits.clamp(1, 5);
    let utm = LatLon { lat, lon }.to_utm();
    let (c, r) = square_letters(utm.zone, utm.easting, utm.northing);
    let div = 10f64.powi(5 - digits as i32);
    let e = ((utm.easting.rem_euclid(100_000.0)) / div).floor() as u64;
    let n = ((utm.northing.rem_euclid(100_000.0)) / div).floor() as u64;
    let w = digits as usize;
    Some(format!("{}{} {}{} {:0w$} {:0w$}", utm.zone, band, c, r, e, n))
}

/// Grid convergence (°): the angle from true north to grid north, clockwise,
/// at a point projected into `zone`. Positive east of the central meridian in
/// the northern hemisphere. Spherical formula — within 0.01° inside a zone.
pub fn convergence_deg(lat: f64, lon: f64, zone: u32) -> f64 {
    let dlam = (lon - (-183.0 + 6.0 * zone as f64)).to_radians();
    (dlam.tan() * lat.to_radians().sin()).atan().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_is_31n_aa() {
        // (0°, 0°): zone 31, easting 166 021.44 m (projection test), northing 0.
        assert_eq!(to_mgrs(0.0, 0.0, 5).unwrap(), "31N AA 66021 00000");
    }

    #[test]
    fn letters_follow_the_aa_scheme() {
        // Zone 18 (even, set 3: S–Z), 45.4215°N 75.6972°W — Ottawa.
        let utm = LatLon { lat: 45.4215, lon: -75.6972 }.to_utm();
        assert_eq!(utm.zone, 18);
        let (c, r) = square_letters(18, utm.easting, utm.northing);
        // Easting 445 km → column 4 of S–Z = V; northing 5 030 km → 50 + 5 = 55 → 15 = R.
        assert_eq!((c, r), ('V', 'R'), "E {} N {}", utm.easting, utm.northing);
        assert!(to_mgrs(45.4215, -75.6972, 5).unwrap().starts_with("18T VR "));
    }

    #[test]
    fn bands() {
        assert_eq!(band_letter(-80.0), Some('C'));
        assert_eq!(band_letter(0.0), Some('N'));
        assert_eq!(band_letter(-0.1), Some('M'));
        assert_eq!(band_letter(45.4), Some('T'));
        assert_eq!(band_letter(83.9), Some('X'));
        assert_eq!(band_letter(84.1), None);
    }

    #[test]
    fn precision_truncates_not_rounds() {
        // MGRS truncates: 66021.44 m at 1 km precision is 66, not 66.0214.
        assert_eq!(to_mgrs(0.0, 0.0, 2).unwrap(), "31N AA 66 00");
    }

    #[test]
    fn convergence_signs() {
        assert!(convergence_deg(45.0, -75.0, 18).abs() < 1e-12); // on the CM
        assert!(convergence_deg(45.0, -72.5, 18) > 0.0); // east of CM, north
        assert!(convergence_deg(45.0, -77.5, 18) < 0.0);
        // At the zone edge, 45°N: atan(tan 3° · sin 45°) ≈ 2.12°.
        assert!((convergence_deg(45.0, -72.0, 18) - 2.1219).abs() < 1e-3);
    }
}
