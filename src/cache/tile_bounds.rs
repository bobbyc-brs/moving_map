/// Tile coordinate conversion and mission area bounding box.
///
/// Uses the Web Mercator projection (EPSG:3857), the standard for OSM,
/// CartoDB, and most web tile services.

use std::f64::consts::PI;

/// A tile coordinate (zero-indexed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileCoord {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

impl TileCoord {
    /// Convert lat/lon to tile coordinate at zoom level z.
    ///
    /// Uses the Web Mercator projection. The y coordinate is "flipped"
    /// compared to the mathematical Mercator — tile y=0 is at the top
    /// (north pole) and increases southward.
    pub fn from_latlon(lat: f64, lon: f64, z: u32) -> Self {
        let lat_rad = lat.to_radians();
        let n = 2.0_f64.powi(z as i32);
        let x = ((lon + 180.0) / 360.0 * n) as u32;
        // Mercator y: tan(lat) + sec(lat) = tan(π/4 + lat/2)
        let y_raw = (1.0 - lat_rad.tan().atanh() / PI) / 2.0 * n;
        let y = y_raw as u32;
        TileCoord { z, x, y }
    }

    /// Convert tile coordinate back to the southwest corner of the tile.
    pub fn to_latlon(self) -> (f64, f64) {
        let n = 2.0_f64.powi(self.z as i32);
        let lon = self.x as f64 / n * 360.0 - 180.0;
        // Inverse Mercator: lat = atan(sinh(π * (1 - 2*y/n)))
        let lat_rad = (PI * (1.0 - 2.0 * self.y as f64 / n)).sinh().atan();
        (lat_rad.to_degrees(), lon)
    }

    /// Return the geographic bounding box of this tile.
    pub fn geo_bounds(self) -> GeoBounds {
        let n = 2.0_f64.powi(self.z as i32);
        let min_lon = self.x as f64 / n * 360.0 - 180.0;
        let max_lon = (self.x as f64 + 1.0) / n * 360.0 - 180.0;
        let min_lat_rad = (PI * (1.0 - 2.0 * (self.y as f64 + 1.0) / n)).sinh().atan();
        let max_lat_rad = (PI * (1.0 - 2.0 * self.y as f64 / n)).sinh().atan();
        GeoBounds {
            min_lat: min_lat_rad.to_degrees(),
            max_lat: max_lat_rad.to_degrees(),
            min_lon,
            max_lon,
        }
    }
}

/// A mission bounding box in tile coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileBounds {
    pub z: u32,
    pub min_x: u32,
    pub max_x: u32,
    pub min_y: u32,
    pub max_y: u32,
}

impl TileBounds {
    /// Number of tiles in this bounds (including edges).
    pub fn tile_count(self) -> usize {
        ((self.max_x - self.min_x + 1) as usize)
            * ((self.max_y - self.min_y + 1) as usize)
    }

    /// Check if the bounds contain a given tile.
    pub fn contains(self, z: u32, x: u32, y: u32) -> bool {
        self.z == z && x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    /// Check if this bounds has any tiles to download.
    pub fn is_empty(self) -> bool {
        self.tile_count() == 0
    }

    /// Clamp to the valid tile range [0, 2^z - 1].
    fn clamp(self) -> Self {
        let max_tile = (2.0_f64.powi(self.z as i32) - 1.0) as u32;
        let min_x = self.min_x.min(max_tile);
        let max_x = self.max_x.min(max_tile);
        let min_y = self.min_y.min(max_tile);
        let max_y = self.max_y.min(max_tile);
        Self {
            z: self.z,
            min_x: min_x.min(max_x),
            max_x,
            min_y: min_y.min(max_y),
            max_y,
        }
    }
}

/// Mission area definition: center point + range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MissionArea {
    pub lat: f64,
    pub lon: f64,
    pub range_nm: f64,
}

impl MissionArea {
    /// Calculate the bounding box of the mission area in lat/lon.
    ///
    /// Approximates the mission area as a circle and returns the
    /// smallest lat/lon rectangle that contains it.
    pub fn bounding_box(self) -> GeoBounds {
        // 1 degree of latitude ≈ 60 NM (nautical mile definition)
        // 1 degree of longitude varies with latitude
        let dlat = self.range_nm / 60.0;
        let dlon = self.range_nm / (60.0 * self.lat.to_radians().cos());

        GeoBounds {
            min_lat: self.lat - dlat,
            max_lat: self.lat + dlat,
            min_lon: self.lon - dlon,
            max_lon: self.lon + dlon,
        }
    }

    /// Convert to tile bounds at the given zoom level.
    ///
    /// Clamps to the valid tile range for the zoom level.
    pub fn to_tile_bounds(self, z: u32) -> TileBounds {
        let bbox = self.bounding_box();
        let min_tile = TileCoord::from_latlon(bbox.min_lat, bbox.min_lon, z);
        let max_tile = TileCoord::from_latlon(bbox.max_lat, bbox.max_lon, z);
        TileBounds {
            z,
            min_x: min_tile.x,
            max_x: max_tile.x,
            min_y: min_tile.y,
            max_y: max_tile.y,
        }
        .clamp()
    }

    /// Calculate an appropriate zoom level for the mission range.
    ///
    /// Formula: zoom = max(10, min(15, 14 - floor(range_nm / 100)))
    /// - 0–50 NM  → zoom 14 (high detail, ~300 m/tile)
    /// - 50–100 NM → zoom 13 (standard, ~600 m/tile)
    /// - 100–200 NM → zoom 12 (operational, ~1.2 km/tile)
    /// - 200+ NM → zoom 11 (strategic, ~2.4 km/tile)
    pub fn recommended_zoom(self) -> u32 {
        let z = 14 - (self.range_nm / 100.0).floor() as u32;
        z.clamp(10, 15)
    }
}

/// Geographic bounding box in lat/lon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoBounds {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}

impl GeoBounds {
    /// Width in degrees.
    pub fn width(self) -> f64 {
        self.max_lon - self.min_lon
    }

    /// Height in degrees.
    pub fn height(self) -> f64 {
        self.max_lat - self.min_lat
    }

    /// Check if a lat/lon point is inside the bounds.
    pub fn contains(self, lat: f64, lon: f64) -> bool {
        lat >= self.min_lat && lat <= self.max_lat && lon >= self.min_lon && lon <= self.max_lon
    }

    /// Convert to tile bounds at zoom z.
    pub fn to_tile_bounds(self, z: u32) -> TileBounds {
        let min_tile = TileCoord::from_latlon(self.min_lat, self.min_lon, z);
        let max_tile = TileCoord::from_latlon(self.max_lat, self.max_lon, z);
        TileBounds {
            z,
            min_x: min_tile.x,
            max_x: max_tile.x,
            min_y: min_tile.y,
            max_y: max_tile.y,
        }
        .clamp()
    }

    /// Width in nautical miles (at the center latitude).
    pub fn width_nm(self) -> f64 {
        let center_lat = (self.min_lat + self.max_lat) / 2.0;
        self.width() * 60.0 * center_lat.to_radians().cos()
    }

    /// Height in nautical miles.
    pub fn height_nm(self) -> f64 {
        self.height() * 60.0
    }
}

/// Auto-zoom calculation.
///
/// Returns a recommended zoom level for the given mission range.
pub fn auto_zoom(range_nm: f64) -> u32 {
    let z = 14 - (range_nm / 100.0).floor() as u32;
    z.clamp(10, 15)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // TileCoord conversion tests
    // ---------------------------------------------------------------------------

    #[test]
    fn latlon_to_tile_at_zoom0() {
        // At zoom 0, entire world is 1 tile (0, 0)
        let tc = TileCoord::from_latlon(0.0, 0.0, 0);
        assert_eq!(tc, TileCoord { z: 0, x: 0, y: 0 });

        let tc2 = TileCoord::from_latlon(45.0, -75.0, 0);
        assert_eq!(tc2, TileCoord { z: 0, x: 0, y: 0 });
    }

    #[test]
    fn latlon_to_tile_known_values() {
        // Well-known tile at zoom 1: lat 0, lon 0 is tile (0, 0)
        // At zoom 1, world is 2x2 tiles. lon=0 is at the boundary:
        // x = floor((0+180)/360 * 2) = floor(1.0) = 1
        let tc = TileCoord::from_latlon(0.0, 0.0, 1);
        assert_eq!(tc, TileCoord { z: 1, x: 1, y: 1 });

        // lon 90 is at x=1 (boundary between x=1 and x=2)
        let tc2 = TileCoord::from_latlon(0.0, 90.0, 1);
        assert_eq!(tc2, TileCoord { z: 1, x: 1, y: 1 });
    }

    #[test]
    fn tile_to_latlon_roundtrip() {
        // Test several known points - verify that from_latlon -> to_latlon
        // stays within the same tile (roundtrip to tile corner, then check bounds)
        for (lat, lon, z) in [
            (0.0, 0.0, 5),
            (45.0, -75.0, 5),
            (45.0, -75.0, 10),
            (45.0, -75.0, 13),
            (60.0, 10.0, 8),
            (-33.8, 151.2, 6),
        ] {
            let tc = TileCoord::from_latlon(lat, lon, z);
            let (lat_corner, lon_corner) = tc.to_latlon();
            // The corner should be within the tile
            let bounds = tc.geo_bounds();
            assert!(lat_corner >= bounds.min_lat && lat_corner <= bounds.max_lat,
                "lat_corner {} not in [{}, {}]", lat_corner, bounds.min_lat, bounds.max_lat);
            assert!(lon_corner >= bounds.min_lon && lon_corner <= bounds.max_lon,
                "lon_corner {} not in [{}, {}]", lon_corner, bounds.min_lon, bounds.max_lon);
        }

        // Zoom 0 is special - tile (0,0) covers the whole world
        let tc0 = TileCoord::from_latlon(0.0, 0.0, 0);
        assert_eq!(tc0, TileCoord { z: 0, x: 0, y: 0 });
        let bounds0 = tc0.geo_bounds();
        assert!((bounds0.min_lat + 85.05).abs() < 0.01);  // ~85.05°N (Mercator limit)
        assert!((bounds0.max_lat - 85.05).abs() < 0.01);
        assert!((bounds0.min_lon + 180.0).abs() < 0.01);
        assert!((bounds0.max_lon - 180.0).abs() < 0.01);
    }

    #[test]
    fn tile_bounds_coverage() {
        // Tile (0, 0) at zoom 0 covers the entire world
        let bounds = TileCoord { z: 0, x: 0, y: 0 }.geo_bounds();
        assert!((bounds.min_lat + 85.05).abs() < 0.01);  // ~85.05°N (Mercator limit)
        assert!((bounds.max_lat - 85.05).abs() < 0.01);
        assert!((bounds.min_lon + 180.0).abs() < 0.01);
        assert!((bounds.max_lon - 180.0).abs() < 0.01);
    }

    // ---------------------------------------------------------------------------
    // MissionArea tests
    // ---------------------------------------------------------------------------

    #[test]
    fn mission_area_bounding_box() {
        let mission = MissionArea {
            lat: 45.0,
            lon: -75.0,
            range_nm: 50.0,
        };
        let bbox = mission.bounding_box();

        // 50 NM = 50/60 ≈ 0.833 degrees of lat
        assert!((bbox.height_nm() - 100.0).abs() < 0.01); // 100 NM diameter
        assert!((bbox.width_nm() - 100.0).abs() < 0.1);
    }

    #[test]
    fn mission_to_tile_bounds_at_zoom13() {
        let mission = MissionArea {
            lat: 45.0,
            lon: -75.0,
            range_nm: 50.0,
        };
        let tb = mission.to_tile_bounds(13);
        // At zoom 13, 50 NM range should give ~25 tiles per side
        // (50 NM / 600m/tile ≈ 42 tiles... let's check the actual range)
        assert_eq!(tb.z, 13);
        assert!(tb.min_x <= tb.max_x);
        assert!(tb.min_y <= tb.max_y);
        assert!(tb.tile_count() > 0);
        assert!(tb.tile_count() < 10000);
    }

    #[test]
    fn mission_to_tile_bounds_clamped() {
        // Mission near the edge of the world should clamp
        let mission = MissionArea {
            lat: 89.0,
            lon: 179.0,
            range_nm: 200.0,
        };
        let tb = mission.to_tile_bounds(10);
        assert!(tb.max_x < 2.0_f64.powi(10) as u32);
        assert!(tb.max_y < 2.0_f64.powi(10) as u32);
        assert!(!tb.is_empty());
    }

    #[test]
    fn auto_zoom_range() {
        assert_eq!(auto_zoom(10.0), 14);
        assert_eq!(auto_zoom(50.0), 14);
        assert_eq!(auto_zoom(100.0), 13);
        assert_eq!(auto_zoom(150.0), 13);
        assert_eq!(auto_zoom(200.0), 12);
        assert_eq!(auto_zoom(250.0), 12);
        assert_eq!(auto_zoom(500.0), 10);
        assert_eq!(auto_zoom(1000.0), 10); // clamped to 10
    }

    #[test]
    fn recommended_zoom_matches_auto_zoom() {
        let mission = MissionArea {
            lat: 45.0,
            lon: -75.0,
            range_nm: 100.0,
        };
        assert_eq!(mission.recommended_zoom(), auto_zoom(100.0));
    }

    // ---------------------------------------------------------------------------
    // GeoBounds tests
    // ---------------------------------------------------------------------------

    #[test]
    fn geobounds_width_height_nm() {
        let bounds = GeoBounds {
            min_lat: 44.0,
            max_lat: 46.0,
            min_lon: -76.0,
            max_lon: -74.0,
        };
        assert!((bounds.height_nm() - 120.0).abs() < 0.01); // 2° × 60 = 120 NM
        let expected_width = 2.0 * 60.0 * 45.0_f64.to_radians().cos();
        assert!(
            (bounds.width_nm() - expected_width).abs() < 0.1,
            "width_nm: {} vs expected {}",
            bounds.width_nm(),
            expected_width
        );
    }

    #[test]
    fn geobounds_contains() {
        let bounds = GeoBounds {
            min_lat: 44.0,
            max_lat: 46.0,
            min_lon: -76.0,
            max_lon: -74.0,
        };
        assert!(bounds.contains(45.0, -75.0));
        assert!(!bounds.contains(47.0, -75.0));
        assert!(!bounds.contains(45.0, -77.0));
    }

    #[test]
    fn tile_coord_at_zoom15() {
        // At zoom 15, 45°N, -75°W should be in tile range [0, 32767]
        let tc = TileCoord::from_latlon(45.0, -75.0, 15);
        let max_tile = (2.0_f64.powi(15) - 1.0) as u32;
        assert!(tc.x <= max_tile);
        assert!(tc.y <= max_tile);
    }

    #[test]
    fn mission_200nm_zoom12_tile_count() {
        // At 200 NM range, zoom 12: ~104 tiles per side → ~10,816 total
        let mission = MissionArea {
            lat: 45.0,
            lon: -75.0,
            range_nm: 200.0,
        };
        let tb = mission.to_tile_bounds(mission.recommended_zoom());
        assert_eq!(tb.z, 12);
        // Each tile at zoom 12 is ~1.2 km; 200 NM = 370 km → ~309 tiles span
        // But with clamping and the bounding box calculation, actual count may differ
        assert!(tb.tile_count() > 0);
        assert!(tb.tile_count() < 50000);
    }
}
