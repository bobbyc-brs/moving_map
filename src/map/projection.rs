/// UTM projection (WGS84) — forward and inverse, by the Krüger series
/// (Karney 2011, "Transverse Mercator with an accuracy of a few nanometers",
/// truncated at n⁴): sub-millimetre within a UTM zone. Replaces the earlier
/// hand-rolled series, which dropped the southern false northing, omitted the
/// inverse latitude correction, and scaled the false easting by k0 (PLAN.md,
/// Discrepancies #4; TESTLOG 2026-11-10).

const WGS84_A: f64 = 6378137.0;
const WGS84_F: f64 = 1.0 / 298.257223563;
const WGS84_E2: f64 = 2.0 * WGS84_F - WGS84_F * WGS84_F;
/// Third flattening.
const WGS84_N: f64 = WGS84_F / (2.0 - WGS84_F);
const K0: f64 = 0.9996;
const FALSE_EASTING: f64 = 500_000.0;
const FALSE_NORTHING_SOUTH: f64 = 10_000_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLon { pub lat: f64, pub lon: f64 }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EastingNorthing { pub easting: f64, pub northing: f64, pub zone: u32, pub hemisphere: Hemisphere }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hemisphere { North, South }

/// Rectifying radius A and the α, β, δ series coefficients.
struct Kruger { a: f64, alpha: [f64; 4], beta: [f64; 4], delta: [f64; 4] }

fn kruger() -> Kruger {
    let n = WGS84_N;
    let (n2, n3, n4) = (n * n, n * n * n, n * n * n * n);
    Kruger {
        a: WGS84_A / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0),
        alpha: [
            n / 2.0 - 2.0 * n2 / 3.0 + 5.0 * n3 / 16.0 + 41.0 * n4 / 180.0,
            13.0 * n2 / 48.0 - 3.0 * n3 / 5.0 + 557.0 * n4 / 1440.0,
            61.0 * n3 / 240.0 - 103.0 * n4 / 140.0,
            49561.0 * n4 / 161280.0,
        ],
        beta: [
            n / 2.0 - 2.0 * n2 / 3.0 + 37.0 * n3 / 96.0 - n4 / 360.0,
            n2 / 48.0 + n3 / 15.0 - 437.0 * n4 / 1440.0,
            17.0 * n3 / 480.0 - 37.0 * n4 / 840.0,
            4397.0 * n4 / 161280.0,
        ],
        delta: [
            2.0 * n - 2.0 * n2 / 3.0 - 2.0 * n3 + 116.0 * n4 / 45.0,
            7.0 * n2 / 3.0 - 8.0 * n3 / 5.0 - 227.0 * n4 / 45.0,
            56.0 * n3 / 15.0 - 136.0 * n4 / 35.0,
            4279.0 * n4 / 630.0,
        ],
    }
}

fn central_meridian_deg(zone: u32) -> f64 {
    -183.0 + 6.0 * zone as f64
}

impl LatLon {
    /// Forward projection into the UTM zone containing this longitude.
    pub fn to_utm(self) -> EastingNorthing {
        let zone = (((self.lon + 180.0) / 6.0).floor() as i64).clamp(0, 59) as u32 + 1;
        self.to_utm_zone(zone)
    }

    /// Forward projection into a given zone (lets callers keep one zone, per DD-002).
    pub fn to_utm_zone(self, zone: u32) -> EastingNorthing {
        let k = kruger();
        let e = WGS84_E2.sqrt();
        let phi = self.lat.to_radians();
        let dlam = (self.lon - central_meridian_deg(zone)).to_radians();

        // Conformal latitude via t = tan χ.
        let t = (phi.sin().atanh() - e * (e * phi.sin()).atanh()).sinh();
        let xi_p = t.atan2(dlam.cos());
        let eta_p = (dlam.sin() / (1.0 + t * t).sqrt()).atanh();

        let (mut xi, mut eta) = (xi_p, eta_p);
        for (j, a) in k.alpha.iter().enumerate() {
            let m = 2.0 * (j + 1) as f64;
            xi += a * (m * xi_p).sin() * (m * eta_p).cosh();
            eta += a * (m * xi_p).cos() * (m * eta_p).sinh();
        }

        let hemisphere = if self.lat >= 0.0 { Hemisphere::North } else { Hemisphere::South };
        let n0 = if hemisphere == Hemisphere::South { FALSE_NORTHING_SOUTH } else { 0.0 };
        EastingNorthing {
            easting: FALSE_EASTING + K0 * k.a * eta,
            northing: n0 + K0 * k.a * xi,
            zone,
            hemisphere,
        }
    }
}

impl EastingNorthing {
    pub fn to_latlon(self) -> LatLon {
        let k = kruger();
        let n0 = if self.hemisphere == Hemisphere::South { FALSE_NORTHING_SOUTH } else { 0.0 };
        let xi = (self.northing - n0) / (K0 * k.a);
        let eta = (self.easting - FALSE_EASTING) / (K0 * k.a);

        let (mut xi_p, mut eta_p) = (xi, eta);
        for (j, b) in k.beta.iter().enumerate() {
            let m = 2.0 * (j + 1) as f64;
            xi_p -= b * (m * xi).sin() * (m * eta).cosh();
            eta_p -= b * (m * xi).cos() * (m * eta).sinh();
        }

        let chi = (xi_p.sin() / eta_p.cosh()).asin();
        let mut phi = chi;
        for (j, d) in k.delta.iter().enumerate() {
            phi += d * (2.0 * (j + 1) as f64 * chi).sin();
        }
        let dlam = eta_p.sinh().atan2(xi_p.cos());

        LatLon {
            lat: phi.to_degrees(),
            lon: central_meridian_deg(self.zone) + dlam.to_degrees(),
        }
    }
}

/// WGS84 geodetic (degrees, metres above the ellipsoid) → geocentric ECEF metres.
pub fn geodetic_to_ecef(lat_deg: f64, lon_deg: f64, h: f64) -> [f64; 3] {
    let (lat, lon) = (lat_deg.to_radians(), lon_deg.to_radians());
    let n = WGS84_A / (1.0 - WGS84_E2 * lat.sin() * lat.sin()).sqrt();
    [
        (n + h) * lat.cos() * lon.cos(),
        (n + h) * lat.cos() * lon.sin(),
        (n * (1.0 - WGS84_E2) + h) * lat.sin(),
    ]
}

/// Geocentric ECEF metres (the DIS world frame) → WGS84 latitude (°),
/// longitude (°), and height above the ellipsoid (m). Fixed-point iteration
/// on latitude; converges to sub-millimetre in well under 10 rounds.
pub fn ecef_to_geodetic(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let lon = y.atan2(x);
    let p = x.hypot(y);
    let mut lat = z.atan2(p * (1.0 - WGS84_E2));
    let mut h = 0.0;
    for _ in 0..10 {
        let sin_lat = lat.sin();
        let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
        // Use whichever form is well-conditioned: cos→0 near the poles.
        h = if lat.cos().abs() > 1e-3 {
            p / lat.cos() - n
        } else {
            z / sin_lat - n * (1.0 - WGS84_E2)
        };
        lat = z.atan2(p * (1.0 - WGS84_E2 * n / (n + h)));
    }
    (lat.to_degrees(), lon.to_degrees(), h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecef_known_points() {
        // Equator / prime meridian: x = semi-major axis.
        let [x, y, z] = geodetic_to_ecef(0.0, 0.0, 0.0);
        assert!((x - 6_378_137.0).abs() < 1e-6 && y.abs() < 1e-6 && z.abs() < 1e-6);
        // North pole: z = semi-minor axis b = a(1 - f) = 6 356 752.314245 m.
        let [_, _, z] = geodetic_to_ecef(90.0, 0.0, 0.0);
        assert!((z - 6_356_752.314245).abs() < 1e-5, "b {z}");
        let (lat, _, h) = ecef_to_geodetic(0.0, 0.0, 6_356_752.314245 + 100.0);
        assert!((lat - 90.0).abs() < 1e-9 && (h - 100.0).abs() < 1e-6);
    }

    #[test]
    fn ecef_round_trip_sub_millimetre() {
        for &(lat, lon, h) in &[(45.4215, -75.6972, 300.0), (-33.9, 151.2, 0.0),
                                (89.9, 10.0, 5000.0), (0.0, 179.9, -50.0)] {
            let [x, y, z] = geodetic_to_ecef(lat, lon, h);
            let (lat2, lon2, h2) = ecef_to_geodetic(x, y, z);
            // 1e-8° ≈ 1.1 mm of latitude.
            assert!((lat - lat2).abs() < 1e-8, "lat {lat} vs {lat2}");
            assert!((lon - lon2).abs() < 1e-8, "lon {lon} vs {lon2}");
            assert!((h - h2).abs() < 1e-3, "h {h} vs {h2}");
        }
    }

    #[test]
    fn roundtrip_45n_75w() {
        let ll = LatLon { lat: 45.0, lon: -75.0 };
        let utm = ll.to_utm();
        let ll_back = utm.to_latlon();
        assert!((ll.lat - ll_back.lat).abs() < 0.05);
        assert!((ll.lon - ll_back.lon).abs() < 0.05);
    }

    /// Ground distance in metres between two nearby points (local flat-earth
    /// approximation using WGS84 meridian and prime-vertical radii — exact
    /// enough at the sub-metre scale being measured).
    fn ground_error_m(a: LatLon, b: LatLon) -> f64 {
        let lat = a.lat.to_radians();
        let s2 = lat.sin() * lat.sin();
        let m = WGS84_A * (1.0 - WGS84_E2) / (1.0 - WGS84_E2 * s2).powf(1.5);
        let n = WGS84_A / (1.0 - WGS84_E2 * s2).sqrt();
        let dn = (b.lat - a.lat).to_radians() * m;
        let de = (b.lon - a.lon).to_radians() * n * lat.cos();
        dn.hypot(de)
    }

    /// LatLon → UTM → LatLon must land within 10 cm (Bobby, 2026-11-10: the
    /// right bar for aircraft display). Covers the zone's central meridian
    /// and edges, and latitudes from the equator to 70°N/S.
    #[test]
    fn utm_round_trip_within_10_cm() {
        let mut worst = (0.0_f64, LatLon { lat: 0.0, lon: 0.0 });
        for &lat in &[0.0, 15.0, 30.0, 45.0, 45.4215, 60.0, 70.0, -33.9, -70.0] {
            // Zone 18 spans 78°W..72°W, central meridian 75°W.
            for &lon in &[-75.0, -75.6972, -77.99, -72.01, -76.5] {
                let ll = LatLon { lat, lon };
                let back = ll.to_utm().to_latlon();
                let err = ground_error_m(ll, back);
                if err > worst.0 { worst = (err, ll); }
            }
        }
        eprintln!("worst UTM round-trip error: {:.4} m at {:?}", worst.0, worst.1);
        assert!(worst.0 < 0.10, "worst error {:.4} m at {:?}", worst.0, worst.1);
    }

    /// Meridian arc length from the equator, by Simpson integration of the
    /// meridian radius — independent of the Krüger series under test.
    fn meridian_arc_numeric(lat_deg: f64) -> f64 {
        let steps = 20_000;
        let h = lat_deg.to_radians() / steps as f64;
        let m = |p: f64| WGS84_A * (1.0 - WGS84_E2) / (1.0 - WGS84_E2 * p.sin() * p.sin()).powf(1.5);
        let mut sum = m(0.0) + m(lat_deg.to_radians());
        for i in 1..steps {
            sum += m(i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
        }
        sum * h / 3.0
    }

    /// On the central meridian, easting is exactly the false easting and
    /// northing is k0 × the meridian arc. 75°W is zone 18's central meridian.
    #[test]
    fn central_meridian_matches_meridian_arc() {
        for &lat in &[10.0, 45.0, 45.4215, 70.0] {
            let utm = LatLon { lat, lon: -75.0 }.to_utm();
            assert_eq!(utm.zone, 18);
            assert!((utm.easting - 500_000.0).abs() < 1e-6, "easting {}", utm.easting);
            let expected = K0 * meridian_arc_numeric(lat);
            assert!((utm.northing - expected).abs() < 0.001, "lat {lat}: {} vs {expected}", utm.northing);
        }
        // Published figure for 45°N on a central meridian: 4 982 950.40 m.
        let n45 = LatLon { lat: 45.0, lon: -75.0 }.to_utm().northing;
        assert!((n45 - 4_982_950.40).abs() < 0.01, "45N northing {n45}");
    }

    /// (0°, 0°) is 3° west of zone 31's central meridian (3°E); the zone-edge
    /// easting at the equator is the well-known 166 021.44 m.
    #[test]
    fn equator_zone_edge_easting() {
        let utm = LatLon { lat: 0.0, lon: 0.0 }.to_utm();
        assert_eq!(utm.zone, 31);
        assert!((utm.easting - 166_021.44).abs() < 0.01, "easting {}", utm.easting);
        assert!(utm.northing.abs() < 1e-6);
    }

    /// Southern hemisphere uses a 10 000 km false northing.
    #[test]
    fn southern_hemisphere_false_northing() {
        let utm = LatLon { lat: -0.000_001, lon: 3.0 }.to_utm();
        assert_eq!(utm.hemisphere, Hemisphere::South);
        assert!((utm.northing - 10_000_000.0).abs() < 1.0, "northing {}", utm.northing);
    }

    /// Zone boundaries: 72°W is the first longitude of zone 19.
    #[test]
    fn zone_boundaries() {
        assert_eq!(LatLon { lat: 44.9, lon: -72.000_001 }.to_utm().zone, 18);
        assert_eq!(LatLon { lat: 44.9, lon: -72.0 }.to_utm().zone, 19);
        assert_eq!(LatLon { lat: 45.0, lon: 180.0 }.to_utm().zone, 60);
        assert_eq!(LatLon { lat: 45.0, lon: -180.0 }.to_utm().zone, 1);
    }
}
