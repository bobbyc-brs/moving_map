/// Tile cache file I/O and tile download coordination.
///
/// Handles:
/// - Checking if a tile exists in the cache directory
/// - Atomic tile writes (write to .tmp, then rename)
/// - Sentinel file tracking (.downloaded)
/// - Listing cached tile count per zoom level

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use super::tile_bounds::MissionArea;

/// Tile cache manager — handles file I/O for cached tiles.
pub struct TileCache {
    pub root: PathBuf,
}

impl TileCache {
    /// Create a TileCache pointing to the given directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        fs::create_dir_all(&root).ok();
        Self { root }
    }

    /// Get the path to a tile file: cache.{z}/x/y.png
    pub fn tile_path(&self, z: u32, x: u32, y: u32) -> PathBuf {
        self.root.join(format!("cache.{}", z)).join(format!("{}", x))
            .join(format!("{}.png", y))
    }

    /// Get the sentinel path: cache.{z}/x/y.png.downloaded
    pub fn sentinel_path(&self, z: u32, x: u32, y: u32) -> PathBuf {
        self.tile_path(z, x, y).with_extension("png.downloaded")
    }

    /// Check if a tile is cached and marked as downloaded.
    pub fn is_cached(&self, z: u32, x: u32, y: u32) -> bool {
        self.sentinel_path(z, x, y).exists()
    }

    /// Write a tile file atomically.
    pub fn write_tile(&self, z: u32, x: u32, y: u32, data: &[u8]) -> std::io::Result<()> {
        let tile_path = self.tile_path(z, x, y);
        let tmp_path = tile_path.with_extension("png.tmp");

        if let Some(parent) = tile_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut tmp_file = fs::File::create(&tmp_path)?;
        tmp_file.write_all(data)?;
        tmp_file.flush()?;
        fs::rename(&tmp_path, &tile_path)?;

        let sentinel = self.sentinel_path(z, x, y);
        fs::File::create(&sentinel)?;

        Ok(())
    }

    /// Read a cached tile, if it exists.
    pub fn read_tile(&self, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        let path = self.tile_path(z, x, y);
        if path.exists() {
            fs::read(&path).ok()
        } else {
            None
        }
    }

    /// Cache tiles for a mission area. Returns number downloaded.
    pub fn cache_mission(&self, mission: &MissionArea, tile_server: &dyn TileServer) -> Result<usize, std::io::Error> {
        let z = mission.recommended_zoom();
        let bounds = mission.to_tile_bounds(z);

        if bounds.is_empty() {
            eprintln!("  Tile bounds are empty (no tiles to download)");
            return Ok(0);
        }

        eprintln!("  Mission: lat={:.4} lon={:.4} range={} NM", mission.lat, mission.lon, mission.range_nm);
        eprintln!("  Cache zoom: {}", z);
        eprintln!("  Tile bounds: z={} [{},{}]x[{},{}] ({} tiles)",
            z, bounds.min_x, bounds.max_x, bounds.min_y, bounds.max_y,
            bounds.tile_count());

        let mut downloaded = 0;
        let total = bounds.tile_count();

        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                if self.is_cached(z, x, y) {
                    continue;
                }

                if let Some(data) = tile_server.get_tile(z, x, y) {
                    self.write_tile(z, x, y, &data)?;
                    downloaded += 1;
                }
            }
        }

        eprintln!("  Downloaded {}/{} tiles", downloaded, total);
        Ok(downloaded)
    }

    /// Count cached tiles for a given zoom level.
    pub fn count_cached(&self, z: u32) -> Result<usize, std::io::Error> {
        let zoom_dir = self.root.join(format!("cache.{}", z));
        let entries = match fs::read_dir(&zoom_dir) {
            Ok(e) => e,
            Err(_) => return Ok(0),
        };

        let mut count = 0;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                for sub_entry in fs::read_dir(&path)? {
                    let sub_entry = sub_entry?;
                    let sub_path = sub_entry.path();
                    if sub_path.extension().map_or(false, |ext| ext == "png") {
                        let sentinel = sub_path.with_extension("png.downloaded");
                        if sentinel.exists() {
                            count += 1;
                        }
                    }
                }
            }
        }
        Ok(count)
    }

    /// Clear all cached tiles.
    pub fn clear(&self) -> std::io::Result<()> {
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("cache.") {
                fs::remove_dir_all(entry.path())?;
            }
        }
        Ok(())
    }

    /// List all cached zoom levels.
    pub fn list_cached_zooms(&self) -> std::io::Result<Vec<u32>> {
        let mut zooms = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(z_str) = name_str.strip_prefix("cache.") {
                if let Ok(z) = z_str.parse::<u32>() {
                    zooms.push(z);
                }
            }
        }
        zooms.sort();
        Ok(zooms)
    }
}

/// Tile server data source — returns raw PNG bytes for a given tile.
pub trait TileServer {
    /// Get raw PNG data for a tile, or None if not available.
    fn get_tile(&self, z: u32, x: u32, y: u32) -> Option<Vec<u8>>;
}

/// Stub tile server that returns no data (no network access).
pub struct OfflineTileServer;

impl TileServer for OfflineTileServer {
    fn get_tile(&self, _z: u32, _x: u32, _y: u32) -> Option<Vec<u8>> {
        None
    }
}

/// Tile server data preloaded from a map (for testing).
pub struct FileTileServer {
    pub tiles: std::collections::HashMap<(u32, u32, u32), Vec<u8>>,
}

impl FileTileServer {
    pub fn new(tiles: std::collections::HashMap<(u32, u32, u32), Vec<u8>>) -> Self {
        Self { tiles }
    }
}

impl TileServer for FileTileServer {
    fn get_tile(&self, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        self.tiles.get(&(z, x, y)).cloned()
    }
}

/// Tile server data with built-in PNG placeholder generation.
pub struct TileServerData;

impl TileServerData {
    pub fn get_tile(&self, _z: u32, _x: u32, _y: u32) -> Option<Vec<u8>> {
        Some(self.minimal_png())
    }

    fn minimal_png(&self) -> Vec<u8> {
        // Minimal valid PNG: 1×1 white pixel
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1×1 RGB
            0x08, 0x02, 0x00, 0x00, 0x00,
            0x90, 0x77, 0x53, 0xDE, // IHDR CRC
            0x00, 0x00, 0x00, 0x0B, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
            0x08, 0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00,
            0x05, 0xFE, 0x02, 0xFE,
            0xA3, 0xB5, 0xFC, 0x7A, // IDAT CRC
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
            0xAE, 0x42, 0x60, 0x82, // IEND CRC
        ]
    }
}

impl TileServer for TileServerData {
    fn get_tile(&self, _z: u32, _x: u32, _y: u32) -> Option<Vec<u8>> {
        Some(self.minimal_png())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::panic;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Unique temp directory prefix per test run.
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Guards automatic cleanup of the temp directory on drop,
    /// including during unwinds (test panics).
    struct TmpGuard(PathBuf);

    impl Drop for TmpGuard {
        fn drop(&mut self) {
            let _ = panic::catch_unwind(|| {
                let _ = fs::remove_dir_all(&self.0);
            });
        }
    }

    /// Create an isolated cache with a unique temp directory.
    fn isolated_cache() -> (TileCache, TmpGuard) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = format!("/tmp/moving-map-test-{}", n);
        let _ = fs::remove_dir_all(&tmp);
        let cache = TileCache::new(&tmp);
        (cache, TmpGuard(PathBuf::from(tmp)))
    }

    fn write_fake_tile(cache: &TileCache, z: u32, x: u32, y: u32) {
        cache.write_tile(z, x, y, b"fake tile data").unwrap();
    }

    #[test]
    fn tile_path_format() {
        let (cache, _guard) = isolated_cache();
        let path = cache.tile_path(13, 1024, 2048);
        assert!(path.to_string_lossy().contains("moving-map-test-"));
        assert!(path.to_string_lossy().ends_with("cache.13/1024/2048.png"));
    }

    #[test]
    fn sentinel_path_format() {
        let (cache, _guard) = isolated_cache();
        let sentinel = cache.sentinel_path(13, 1024, 2048);
        assert!(sentinel.to_string_lossy().ends_with("cache.13/1024/2048.png.downloaded"));
    }

    #[test]
    fn write_and_read_tile() {
        let (cache, _guard) = isolated_cache();
        let data = b"fake tile data";
        cache.write_tile(13, 1024, 2048, data).unwrap();
        assert!(cache.is_cached(13, 1024, 2048));
        let read = cache.read_tile(13, 1024, 2048).unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn not_cached_returns_none() {
        let (cache, _guard) = isolated_cache();
        assert!(!cache.is_cached(13, 1024, 2048));
        assert!(cache.read_tile(13, 1024, 2048).is_none());
    }

    #[test]
    fn count_cached_zero() {
        let (cache, _guard) = isolated_cache();
        assert_eq!(cache.count_cached(13).unwrap(), 0);
    }

    #[test]
    fn count_cached_after_write() {
        let (cache, _guard) = isolated_cache();
        write_fake_tile(&cache, 13, 1024, 2048);
        assert_eq!(cache.count_cached(13).unwrap(), 1);
    }

    #[test]
    fn count_cached_multiple_zooms() {
        let (cache, _guard) = isolated_cache();
        write_fake_tile(&cache, 12, 0, 0);
        write_fake_tile(&cache, 13, 1024, 2048);
        assert_eq!(cache.count_cached(12).unwrap(), 1);
        assert_eq!(cache.count_cached(13).unwrap(), 1);
    }

    #[test]
    fn list_cached_zooms() {
        let (cache, _guard) = isolated_cache();
        write_fake_tile(&cache, 13, 1024, 2048);
        write_fake_tile(&cache, 12, 0, 0);
        let zooms = cache.list_cached_zooms().unwrap();
        assert_eq!(zooms, vec![12, 13]);
    }

    #[test]
    fn clear_cache() {
        let (cache, _guard) = isolated_cache();
        write_fake_tile(&cache, 12, 0, 0);
        write_fake_tile(&cache, 13, 1024, 20448);
        cache.clear().unwrap();
        assert_eq!(cache.count_cached(12).unwrap(), 0);
        assert_eq!(cache.count_cached(13).unwrap(), 0);
    }

    #[test]
    fn file_tile_server_returns_data() {
        let mut tiles = HashMap::new();
        tiles.insert((13, 1024, 2048), b"test tile".to_vec());
        let server = FileTileServer::new(tiles);
        assert_eq!(server.get_tile(13, 1024, 2048).unwrap(), b"test tile");
        assert!(server.get_tile(13, 999, 999).is_none());
    }

    #[test]
    fn cache_tile_from_file_server() {
        let (cache, _guard) = isolated_cache();
        let mut tiles = HashMap::new();
        let expected = b"from server".to_vec();
        tiles.insert((13, 1024, 2048), expected.clone());
        let _server = FileTileServer::new(tiles);

        // Write tile directly (simulating server download)
        cache.write_tile(13, 1024, 2048, &expected).unwrap();

        assert!(cache.is_cached(13, 1024, 2048));
        let read = cache.read_tile(13, 1024, 2048).unwrap();
        assert_eq!(read, expected);
    }
}
