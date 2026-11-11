/// UTM projection (WGS84) - forward and inverse.

const WGS84_A: f64 = 6378137.0;
const WGS84_F: f64 = 1.0 / 298.257223563;
const WGS84_E2: f64 = 2.0 * WGS84_F - WGS84_F * WGS84_F;
const WGS84_EP2: f64 = WGS84_E2 / (1.0 - WGS84_E2);
const WGS84_N: f64 = WGS84_F / (2.0 - WGS84_F);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLon { pub lat: f64, pub lon: f64 }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EastingNorthing { pub easting: f64, pub northing: f64, pub zone: u32, pub hemisphere: Hemisphere }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hemisphere { North, South }

fn meridional_coeffs() -> (f64, f64, f64, f64) {
    let e2 = WGS84_E2;
    let a0 = 1.0 - e2 / 4.0 - 3.0 * e2 * e2 / 64.0 - 5.0 * e2 * e2 * e2 / 256.0;
    let a2 = 3.0 * e2 / 8.0 + 3.0 * e2 * e2 / 32.0 + 45.0 * e2 * e2 * e2 / 1024.0;
    let a4 = 15.0 * e2 * e2 / 256.0 + 45.0 * e2 * e2 * e2 / 1024.0;
    let a6 = 35.0 * e2 * e2 * e2 / 3072.0;
    (a0, a2, a4, a6)
}

fn meridional_arc(lat: f64) -> f64 {
    let (a0, a2, a4, a6) = meridional_coeffs();
    WGS84_A * (a0 * lat - a2 * (2.0 * lat).sin() + a4 * (4.0 * lat).sin() - a6 * (6.0 * lat).sin())
}

impl LatLon {
    pub fn to_utm(self) -> EastingNorthing {
        let lat = self.lat.to_radians();
        let lon = self.lon.to_radians();
        let lon_deg = self.lon;
        let zone = ((lon_deg + 180.0) / 6.0).floor() as u32 + 1;
        let lon0 = (-180.0 + (zone as f64 - 1.0) * 6.0).to_radians();
        let k0: f64 = 0.9996;
        let sin_lat = lat.sin();
        let cos_lat = lat.cos();
        let tan_lat = lat.tan();
        let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
        let t = tan_lat * tan_lat;
        let c = WGS84_EP2 * sin_lat * sin_lat;
        let a = cos_lat * (lon - lon0);

        let m = meridional_arc(lat);

        let mut easting = n * a
            + n * a * a * a / 6.0 * (1.0 - t + c)
            + n * a * a * a * a * a / 120.0
                * (5.0 - 18.0 * t + t * t + 72.0 * c - 58.0 * WGS84_EP2 * WGS84_EP2 * sin_lat * sin_lat)
            + 500_000.0;

        let n2 = n * tan_lat / 2.0;
        let n4 = n / 24.0 * (5.0 - t + 9.0 * c + 4.0 * c * c);
        let n6 = n / 720.0 * (61.0 - 58.0 * t + t * t + 600.0 * c - 330.0 * WGS84_EP2 * WGS84_EP2 * sin_lat * sin_lat);
        let mut northing = m + n2 * a * a + n4 * a * a * a * a + n6 * a * a * a * a * a * a;

        easting *= k0;
        northing *= k0;
        if self.lat < 0.0 {
            northing += 10_000_000.0;
        }
        EastingNorthing { easting, northing, zone, hemisphere: if self.lat >= 0.0 { Hemisphere::North } else { Hemisphere::South } }
    }
}

impl EastingNorthing {
    pub fn to_latlon(self) -> LatLon {
        let k0: f64 = 0.9996;
        let m = self.northing / k0;
        let (a0, _a2, _a4, _a6) = meridional_coeffs();
        let mu = m / (WGS84_A * a0);
        let n = WGS84_N;
        let phi1 = mu
            + (3.0 * n - 3.0 * n * n + 12.0 * n * n * n) / 8.0 * (2.0 * mu).sin()
            + (15.0 * n * n - 15.0 * n * n * n + 105.0 * n * n * n * n) / 16.0 * (4.0 * mu).sin()
            + (355.0 * n * n * n / 384.0) * (6.0 * mu).sin();

        // Iterative refinement using Newton's method
        let mut lat = phi1;
        for _ in 0..10 {
            let m2 = meridional_arc(lat);
            let rho = WGS84_A * (1.0 - WGS84_E2) / (1.0 - WGS84_E2 * lat.sin() * lat.sin()).powf(1.5);
            let dlat = (m - m2) / (rho * k0);
            lat += dlat;
        }

        let sp = lat.sin();
        let tp = lat.tan();
        let n1 = WGS84_A / (1.0 - WGS84_E2 * sp * sp).sqrt();
        let t1 = tp * tp;
        let _r1 = WGS84_A * (1.0 - WGS84_E2) / (1.0 - WGS84_E2 * sp * sp).powf(1.5);
        let d = (self.easting / k0 - 500_000.0) / (n1 * k0);
        let d2 = d * d;
        let d3 = d2 * d;
        let _d4 = d2 * d2;
        let d5 = d3 * d2;
        let _d6 = d3 * d3;
        let lon = (d - (1.0 + 2.0 * t1 + sp * sp) * d3 / 6.0 + (5.0 + 28.0 * t1 + 24.0 * t1 * t1 + 6.0 * sp * sp + 8.0 * WGS84_E2 * sp * sp) * d5 / 120.0) / lat.cos()
            + (-180.0 + (self.zone as f64 - 1.0) * 6.0).to_radians();
        LatLon { lat: lat.to_degrees(), lon: lon.to_degrees() }
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

    #[test]
    fn roundtrip_equator() {
        let ll = LatLon { lat: 0.0, lon: 0.0 };
        let utm = ll.to_utm();
        assert_eq!(utm.zone, 31);
        assert!(utm.easting > 499_500.0 && utm.easting < 500_500.0);
        assert!(utm.northing.abs() < 1.0);
        let ll_back = utm.to_latlon();
        assert!((ll.lat - ll_back.lat).abs() < 0.05);
        assert!((ll.lon - ll_back.lon).abs() < 0.05);
    }

    #[test]
    fn zone_18_values() {
        let ll = LatLon { lat: 45.0, lon: -75.0 };
        let utm = ll.to_utm();
        assert_eq!(utm.zone, 18);
        assert!(utm.easting > 700_000.0 && utm.easting < 800_000.0);
        assert!(utm.northing > 4_900_000.0 && utm.northing < 5_000_000.0);
    }
}
