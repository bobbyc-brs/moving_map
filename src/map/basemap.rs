//! Vector basemap (DD-009, DD-012): roads, railways, rivers and lakes as
//! lat/lon polylines, read from a pre-staged text file — never fetched at
//! run time. Files come from `devel-scripts/fetch-osm-basemap.sh`.
//!
//! Format: `#` comment lines, then one feature per line:
//! `<class> <lat>,<lon> <lat>,<lon> ...`

use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Motorway,
    Trunk,
    Primary,
    Secondary,
    Tertiary,
    Rail,
    River,
    Water,
}

impl Class {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "motorway" => Class::Motorway,
            "trunk" => Class::Trunk,
            "primary" => Class::Primary,
            "secondary" => Class::Secondary,
            "tertiary" => Class::Tertiary,
            "rail" => Class::Rail,
            "river" => Class::River,
            "water" => Class::Water,
            _ => return None,
        })
    }

    /// Coarsest scale (metres per pixel) at which this class is still drawn,
    /// so a 200 NM picture is not buried in minor roads.
    pub fn max_m_per_px(self) -> f64 {
        match self {
            Class::Motorway | Class::Trunk | Class::River | Class::Water => f64::INFINITY,
            Class::Primary | Class::Rail => 250.0,
            Class::Secondary => 120.0,
            Class::Tertiary => 50.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Feature {
    pub class: Class,
    pub points: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Default)]
pub struct Basemap {
    pub features: Vec<Feature>,
    /// Attribution / licence lines from the file header (`# licence: …`).
    pub attribution: Option<String>,
    /// Lines that could not be read — counted, not fatal.
    pub skipped: usize,
}

impl Basemap {
    pub fn parse(text: &str) -> Self {
        let mut map = Basemap::default();
        for line in text.lines() {
            if let Some(comment) = line.strip_prefix('#') {
                if let Some(lic) = comment.trim().strip_prefix("licence:") {
                    map.attribution = Some(lic.trim().to_string());
                }
                continue;
            }
            if line.trim().is_empty() {
                continue;
            }
            match parse_feature(line) {
                Some(f) => map.features.push(f),
                None => map.skipped += 1,
            }
        }
        map
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        Ok(Self::parse(&fs::read_to_string(path)?))
    }

    /// Merge every `*.txt` in a directory (e.g. `data/basemap/`).
    pub fn load_dir(dir: &Path) -> io::Result<Self> {
        let mut all = Basemap::default();
        let mut paths: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "txt"))
            .collect();
        paths.sort();
        for p in paths {
            let m = Self::load(&p)?;
            all.features.extend(m.features);
            all.skipped += m.skipped;
            all.attribution = all.attribution.or(m.attribution);
        }
        Ok(all)
    }
}

fn parse_feature(line: &str) -> Option<Feature> {
    let mut parts = line.split_ascii_whitespace();
    let class = Class::parse(parts.next()?)?;
    let points = parts
        .map(|p| {
            let (lat, lon) = p.split_once(',')?;
            Some((lat.parse().ok()?, lon.parse().ok()?))
        })
        .collect::<Option<Vec<(f64, f64)>>>()?;
    (points.len() >= 2).then_some(Feature { class, points })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_features_and_attribution() {
        let text = "# moving-map basemap v1\n# licence: ODbL — © OpenStreetMap contributors\n\
                    motorway 45.1,-75.1 45.2,-75.2\n\
                    water 45.0,-75.0 45.0,-75.1 45.1,-75.1 45.0,-75.0\n\
                    \n\
                    footpath 45.0,-75.0 45.1,-75.0\n\
                    primary 45.0,-75.0\n\
                    river 45.0,abc 45.1,-75.0\n";
        let m = Basemap::parse(text);
        assert_eq!(m.features.len(), 2);
        assert_eq!(m.features[0].class, Class::Motorway);
        assert_eq!(m.features[0].points, vec![(45.1, -75.1), (45.2, -75.2)]);
        assert_eq!(m.features[1].points.len(), 4);
        // Unknown class, single point, bad number: skipped, counted.
        assert_eq!(m.skipped, 3);
        assert_eq!(m.attribution.as_deref(), Some("ODbL — © OpenStreetMap contributors"));
    }

    #[test]
    fn minor_roads_drop_out_at_long_range() {
        assert!(Class::Tertiary.max_m_per_px() < Class::Secondary.max_m_per_px());
        assert!(Class::Motorway.max_m_per_px().is_infinite());
    }
}
