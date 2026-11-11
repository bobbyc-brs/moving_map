/// Moving map display — zero-dependency CLI entry point.

mod dis;
mod cache;
mod map;
mod render;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::time::Instant;

use dis::entity::{Activity, EntityId, EntityRecord, Status};
use dis::packet::{EntityDeathPdu, EntityStatePdu, EntityRecreationPdu, PduHeader};
use render::nato::entity_char;

// ---------------------------------------------------------------------------
// Demo data
// ---------------------------------------------------------------------------

fn seed_entities() -> Vec<EntityRecord> {
    let now = Instant::now();
    let lat0 = 45.0_f64;
    let lon0 = -75.0_f64;
    vec![
        EntityRecord::new(
            EntityId::new(1, 1, 1), "OWN-01".into(),
            dis::entity::Station::Surface, Activity::Sedan, Status::Friendly,
            lat0, lon0, 150.0, 45.0, 2.5, now,
        ),
        EntityRecord::new(
            EntityId::new(1, 1, 2), "FRI-02".into(),
            dis::entity::Station::Surface, Activity::Tank, Status::Friendly,
            lat0 + 0.01, lon0 + 0.005, 200.0, 120.0, 15.0, now,
        ),
        EntityRecord::new(
            EntityId::new(1, 2, 1), "RED-11".into(),
            dis::entity::Station::Air, Activity::Fighter, Status::Hostile,
            lat0 - 0.02, lon0 + 0.03, 3000.0, 270.0, 250.0, now,
        ),
        EntityRecord::new(
            EntityId::new(1, 2, 2), "RED-12".into(),
            dis::entity::Station::Air, Activity::Helicopter, Status::Hostile,
            lat0 + 0.03, lon0 - 0.02, 500.0, 180.0, 80.0, now,
        ),
        EntityRecord::new(
            EntityId::new(1, 3, 1), "UNKN-1".into(),
            dis::entity::Station::Surface, Activity::APC, Status::Unknown,
            lat0 + 0.05, lon0 + 0.05, 300.0, 90.0, 20.0, now,
        ),
        EntityRecord::new(
            EntityId::new(1, 4, 1), "BLUE-01".into(),
            dis::entity::Station::SurfaceVessel, Activity::PatrolBoat, Status::Neutral,
            lat0 - 0.04, lon0 - 0.04, 10.0, 0.0, 12.0, now,
        ),
        EntityRecord::new(
            EntityId::new(1, 5, 1), "ELEV-STAT".into(),
            dis::entity::Station::Installation, Activity::Unknown, Status::Friendly,
            lat0 + 0.005, lon0 - 0.01, 50.0, 0.0, 0.0, now,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Text map renderer
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Cell {
    ch: char,
    fg: &'static str,
}

impl Cell {
    fn blank() -> Self { Self { ch: ' ', fg: "" } }
    fn ownship() -> Self { Self { ch: 'o', fg: "\x1b[36m" } }
    fn symbol(ch: char, colour: &'static str) -> Self { Self { ch, fg: colour } }
}

fn render_text_map(
    entities: &[EntityRecord],
    ownship_id: EntityId,
    origin_lat: f64,
    origin_lon: f64,
    range_nm: f64,
    width: usize,
    height: usize,
    show_grid: bool,
    show_rings: bool,
) -> String {
    let mut grid: Vec<Vec<Cell>> = vec![vec![Cell::blank(); width]; height];
    let cx = width / 2;
    let cy = height * 7 / 10;
    let origin_utm = map::projection::LatLon { lat: origin_lat, lon: origin_lon }.to_utm();
    let px_per_nm = width as f64 / (2.0 * range_nm);
    let m_per_nm = 1852.0;

    // Grid lines (1 NM spacing)
    if show_grid {
        let grid_m = m_per_nm;
        let grid_px = grid_m * px_per_nm;
        if grid_px > 1.0 {
            for x in (0..width as i32).step_by(grid_px as usize) {
                for y in 0..height {
                    if grid[y][x as usize].ch == ' ' {
                        grid[y][x as usize] = Cell::symbol('·', "\x1b[90m");
                    }
                }
            }
            for y in (0..height as i32).step_by(grid_px as usize) {
                for x in 0..width {
                    if grid[y as usize][x].ch == ' ' {
                        grid[y as usize][x] = Cell::symbol('·', "\x1b[90m");
                    }
                }
            }
        }
    }

    // Range rings
    if show_rings {
        for &mult in &[0.5, 1.0, 2.0] {
            let ring_m = m_per_nm * range_nm * mult;
            let ring_px = ring_m * px_per_nm;
            if ring_px > 0.0 && ring_px < 5000.0 {
                for a in (0..360).step_by(2) {
                    let rad = (a as f64).to_radians();
                    let px = cx as f64 + ring_px * rad.cos();
                    let py = cy as f64 + ring_px * rad.sin();
                    let ix = px as i32;
                    let iy = py as i32;
                    if ix >= 0 && ix < width as i32 && iy >= 0 && iy < height as i32 {
                        if grid[iy as usize][ix as usize].ch == ' ' {
                            grid[iy as usize][ix as usize] = Cell::symbol('·', "\x1b[90m");
                        }
                    }
                }
            }
        }
    }

    // Draw entities
    for entity in entities {
        let entity_utm = map::projection::LatLon {
            lat: entity.latitude,
            lon: entity.longitude,
        }.to_utm();

        let dx = entity_utm.easting - origin_utm.easting;
        let dy = entity_utm.northing - origin_utm.northing;

        let sx = cx as f64 + dx * px_per_nm / m_per_nm;
        let sy = cy as f64 - dy * px_per_nm / m_per_nm;

        let px = sx as i32;
        let py = sy as i32;
        if px < 0 || px >= width as i32 || py < 0 || py >= height as i32 {
            continue;
        }

        let y = py as usize;
        let x = px as usize;

        let cell = if entity.entity_id == ownship_id {
            Cell::ownship()
        } else {
            Cell::symbol(entity_char(entity), entity.status.ansi_colour())
        };
        grid[y][x] = cell;
    }

    // Build output
    let mut out = String::new();
    out.push_str("\x1b[2J\x1b[H");
    out.push_str(&format!(
        "╔══════════════════════════════════════════════════════════════════╗\n"));
    out.push_str(&format!(
        "║  Moving Map Display  |  Range: {:>5.0} NM  |  Entities: {:>3}  ║\n",
        range_nm, entities.len()));
    out.push_str("╠══════════════════════════════════════════════════════════════════╣\n");

    for row in &grid {
        out.push_str("║");
        for cell in row {
            out.push_str(cell.fg);
            out.push(cell.ch);
        }
        out.push_str("\x1b[0m║\n");
    }

    let zone = origin_utm.zone;
    out.push_str("╠══════════════════════════════════════════════════════════════════╣\n");
    out.push_str("║  [F]=Friendly  [A]=Hostile  [U]=Unknown  [N]=Neutral  [o]=Ownship  ║\n");
    out.push_str(&format!(
        "║  Centre: {:.4}°N {:.4}°W  |  UTM Zone: {}                           ║\n",
        origin_lat, origin_lon.abs(), zone));
    out.push_str("╚══════════════════════════════════════════════════════════════════╝\n");
    out
}

fn render_entity_table(entities: &[EntityRecord]) -> String {
    let mut out = String::new();
    out.push_str("\nEntity Table:\n");
    out.push_str(&format!(
        "  {:>12}  {:<8}  {:<4}  {:<8}  {:>10}  {:>10}  {:>8}\n",
        "ID", "Sts", "Stn", "Act", "Lat", "Lon", "Alt(m)"));
    out.push_str("  ───────────────────────────────────────────────────────────\n");

    for e in entities {
        out.push_str(&format!(
            "  {:>12}  {:<8}  {:<4}  {:<8}  {:>10.4}  {:>10.4}  {:>8.0}\n",
            e.entity_id.display(), e.status.label(), e.station.short(),
            e.activity.short(), e.latitude, e.longitude, e.altitude));
    }
    out
}

// ---------------------------------------------------------------------------
// PDU processing
// ---------------------------------------------------------------------------

fn process_pdu(buf: &[u8]) -> Result<String, String> {
    let header = PduHeader::parse(buf).map_err(|e| format!("Header: {e:?}"))?;

    match header.pdu_type {
        1 => {
            let pdu = EntityStatePdu::parse(buf).map_err(|e| format!("EntityState: {e:?}"))?;
            Ok(format!(
                "Entity State [{}.{}.{}]  lat={:.4}  lon={:.4}  alt={:.0}m  spd={:.1}m/s  hdg={:.1}°\n  station={:?}  activity={:?}  marking=0x{:04X}",
                pdu.entity_id.site, pdu.entity_id.application, pdu.entity_id.entity,
                pdu.latitude, pdu.longitude, pdu.altitude,
                pdu.velocity_magnitude(), pdu.heading_degrees(),
                pdu.station(), pdu.activity(), pdu.marking))
        }
        2 => {
            let pdu = EntityDeathPdu::parse(buf).map_err(|e| format!("EntityDeath: {e:?}"))?;
            Ok(format!(
                "Entity Death [{}.{}.{}]  cause={}  area_id={}",
                pdu.entity_id.site, pdu.entity_id.application, pdu.entity_id.entity,
                pdu.cause, pdu.area_or_object_id))
        }
        4 => {
            let pdu = EntityRecreationPdu::parse(buf).map_err(|e| format!("EntityRecreation: {e:?}"))?;
            Ok(format!(
                "Entity Recreation [{}.{}.{}]  type={}  lat={:.4}  lon={:.4}  alt={:.0}m",
                pdu.site, pdu.application, pdu.entity,
                pdu.entity_type, pdu.latitude, pdu.longitude, pdu.altitude))
        }
        5 => Ok("Entity Detection PDU (type 5)".into()),
        _ => Ok(format!("PDU type {} (unhandled)", header.pdu_type)),
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn print_usage() {
    eprintln!("Usage: moving-map-display <command> [args]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  demo           Render text map with seeded entities");
    eprintln!("  parse <file>   Parse DIS PDUs from a binary file");
    eprintln!("  parse-stdin    Parse DIS PDUs from stdin");
    eprintln!("  entities       Display seeded entity table");
    eprintln!("  cache-gen      Download map tiles for a mission area");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str());

    match command {
        Some("demo") => run_demo(),
        Some("parse") => {
            let filename = args.get(1).map(|s| s.as_str());
            match filename {
                Some(path) => {
                    let data = fs::read(path).unwrap_or_else(|e| {
                        eprintln!("Error reading {}: {}", path, e);
                        std::process::exit(1);
                    });
                    parse_file(&data);
                }
                None => {
                    eprintln!("Error: parse requires a filename argument");
                    print_usage();
                    std::process::exit(1);
                }
            }
        }
        Some("parse-stdin") => {
            let mut data = Vec::new();
            io::stdin().read_to_end(&mut data).unwrap_or_else(|e| {
                eprintln!("Error reading stdin: {}", e);
                std::process::exit(1);
            });
            parse_file(&data);
        }
        Some("entities") => run_entity_table(),
        Some("cache-gen") => run_cache_gen(&args[2..]),
        Some(arg) => {
            eprintln!("Unknown command: {}", arg);
            print_usage();
            std::process::exit(1);
        }
        None => { print_usage(); std::process::exit(0); }
    }
}

fn run_demo() {
    let entities = seed_entities();
    let ownship_id = EntityId::new(1, 1, 1);
    let map_output = render_text_map(&entities, ownship_id, 45.0, -75.0, 25.0, 80, 24, true, true);
    println!("{}", map_output);
    println!("{}", render_entity_table(&entities));
}

fn run_entity_table() {
    let entities = seed_entities();
    println!("{}", render_entity_table(&entities));
}

fn parse_file(data: &[u8]) {
    println!("DIS PDU Parser — {} bytes", data.len());
    println!("────────────────────────────────────────────");

    let mut offset = 0;
    let mut pdu_count = 0;
    while offset + 12 <= data.len() {
        let header = match PduHeader::parse(&data[offset..]) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("  Offset {}: Header parse error: {:?}", offset, e);
                break;
            }
        };

        let end = offset + header.length as usize;
        if end > data.len() {
            eprintln!("  Offset {}: PDU extends beyond data ({} > {})", offset, end, data.len());
            break;
        }

        let pdu_data = &data[offset..end];
        match process_pdu(pdu_data) {
            Ok(info) => println!("  Offset {}: {}", offset, info),
            Err(e) => println!("  Offset {}: {}", offset, e),
        }

        offset = end;
        pdu_count += 1;
    }

    println!("────────────────────────────────────────────");
    println!("Processed {} bytes → {} PDUs", data.len(), pdu_count);
}

// ---------------------------------------------------------------------------
// Cache-gen: download map tiles for a mission area
// ---------------------------------------------------------------------------



// ---------------------------------------------------------------------------
// Cache-gen: download map tiles for a mission area
// ---------------------------------------------------------------------------

fn parse_flag(args: &[String], flag: &str) -> Option<String> {
    let pos = args.iter().position(|a| a == flag)?;
    args.get(pos + 1).map(String::from)
}


fn run_cache_gen(args: &[String]) {
    // Walk args: skip flag-value pairs (--foo bar), first leftover is cache_dir
    let mut cache_dir = "tile-cache";
    let mut i = 0;
    while i < args.len() {
        if args[i].starts_with("--") {
            i += 2; // skip flag + value
        } else {
            cache_dir = &args[i];
            i += 1;
        }
    }

    let lat: f64 = parse_flag(args, "--lat").unwrap_or("45.0".into()).parse().unwrap_or(45.0);
    let lon: f64 = parse_flag(args, "--lon").unwrap_or("-75.0".into()).parse().unwrap_or(-75.0);
    let range_nm: f64 = parse_flag(args, "--range-nm").unwrap_or("100.0".into()).parse().unwrap_or(100.0);
    let zoom: Option<u32> = parse_flag(args, "--zoom").map(|z| z.parse().unwrap_or(12));

    let mission = cache::tile_bounds::MissionArea { lat, lon, range_nm };
    let z = zoom.unwrap_or_else(|| mission.recommended_zoom());

    eprintln!("Cache gen: lat={:.4} lon={:.4} range={} NM zoom={}",
        lat, lon, range_nm, z);

    let cache = cache::manager::TileCache::new(cache_dir);
    let bounds = mission.to_tile_bounds(z);
    let total = bounds.tile_count();

    eprintln!("  Tile bounds: z={} [{},{}]x[{},{}] ({} tiles)",
        z, bounds.min_x, bounds.max_x, bounds.min_y, bounds.max_y, total);

    if total == 0 {
        eprintln!("  No tiles to cache (area too small at this zoom)");
        return;
    }

    let server = cache::manager::TileServerData;

    // Check how many are already cached
    let existing = cache.count_cached(z).unwrap_or(0);
    eprintln!("  Already cached: {} / {}", existing, total);

    // Download missing tiles
    let mut downloaded = 0usize;
    let mut skipped = 0usize;
    let start = Instant::now();

    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            if cache.is_cached(z, x, y) {
                skipped += 1;
                continue;
            }
            if let Some(data) = server.get_tile(z, x, y) {
                if let Err(e) = cache.write_tile(z, x, y, &data) {
                    eprintln!("  Error writing tile ({}, {}, {}): {}", z, x, y, e);
                } else {
                    downloaded += 1;
                }
            }
        }
    }

    let elapsed = start.elapsed();
    eprintln!("  Cached {} tiles ({} new, {} skipped) in {:.2}s",
        downloaded + skipped, downloaded, skipped, elapsed.as_secs_f64());
}
