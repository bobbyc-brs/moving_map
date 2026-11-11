/// Mission map tile cache: tile coordinate conversion and cache file management.
///
/// Provides:
/// - `tile_bounds` — lat/lon ↔ xyz coordinate conversion, mission area bounds
/// - `manager`     — tile file I/O, atomic writes, sentinel tracking

pub mod tile_bounds;
pub mod manager;
