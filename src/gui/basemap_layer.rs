//! Basemap layer for the GUI (DD-012). Projecting ~500 000 points into UTM
//! is too slow to do every frame, so it is done once per held zone on a
//! background thread; each frame then only rotates, scales, culls and
//! thins the cached grid coordinates.

use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui::{Painter, Pos2, Rect, Shape};

use super::draw::Palette;
use super::view::{grid, View};
use crate::map::basemap::{Basemap, Class};

/// Water bodies smaller than this on screen are not drawn.
const MIN_WATER_PX: f64 = 12.0;

pub struct Projected {
    pub zone: u32,
    /// Sorted so water is drawn first and motorways last (on top).
    features: Vec<GridFeature>,
}

struct GridFeature {
    class: Class,
    points: Vec<(f64, f64)>,
    /// Grid bounding box: min e, min n, max e, max n.
    bbox: [f64; 4],
}

pub struct Layer {
    source: Arc<Basemap>,
    ready: Arc<Mutex<Option<Arc<Projected>>>>,
    pending_zone: Option<u32>,
    pub attribution: Option<String>,
}

fn draw_order(c: Class) -> u8 {
    match c {
        Class::Water => 0,
        Class::River => 1,
        Class::Rail => 2,
        Class::Tertiary => 3,
        Class::Secondary => 4,
        Class::Primary => 5,
        Class::Trunk => 6,
        Class::Motorway => 7,
    }
}

impl Layer {
    pub fn new(source: Basemap) -> Self {
        let attribution = source.attribution.clone();
        Self { source: Arc::new(source), ready: Arc::new(Mutex::new(None)), pending_zone: None, attribution }
    }

    pub fn feature_count(&self) -> usize {
        self.source.features.len()
    }

    /// The projection for `zone` if ready; otherwise start computing it
    /// (once) and return `None`. `wake` repaints when it is done.
    pub fn get(&mut self, zone: u32, wake: impl Fn() + Send + 'static) -> Option<Arc<Projected>> {
        if let Some(p) = self.ready.lock().unwrap().as_ref() {
            if p.zone == zone {
                return Some(Arc::clone(p));
            }
        }
        if self.pending_zone != Some(zone) {
            self.pending_zone = Some(zone);
            let source = Arc::clone(&self.source);
            let ready = Arc::clone(&self.ready);
            thread::spawn(move || {
                let projected = project(&source, zone);
                *ready.lock().unwrap() = Some(Arc::new(projected));
                wake();
            });
        }
        None
    }
}

fn project(source: &Basemap, zone: u32) -> Projected {
    let mut features: Vec<GridFeature> = source.features.iter().map(|f| {
        let points: Vec<(f64, f64)> = f.points.iter().map(|&(lat, lon)| grid(lat, lon, zone)).collect();
        let mut bbox = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for &(e, n) in &points {
            bbox = [bbox[0].min(e), bbox[1].min(n), bbox[2].max(e), bbox[3].max(n)];
        }
        GridFeature { class: f.class, points, bbox }
    }).collect();
    features.sort_by_key(|f| draw_order(f.class));
    Projected { zone, features }
}

/// Draw the cached basemap. Returns the number of features drawn.
pub fn draw(painter: &Painter, p: &Palette, v: &View, rect: Rect, proj: &Projected) -> usize {
    // Visible grid extent, from the screen corners (the view may be rotated).
    let mut view_box = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for c in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()] {
        let (e, n) = v.screen_to_grid(c.x, c.y);
        view_box = [view_box[0].min(e), view_box[1].min(n), view_box[2].max(e), view_box[3].max(n)];
    }
    let mut drawn = 0;
    for f in &proj.features {
        if v.m_per_px > f.class.max_m_per_px() {
            continue;
        }
        let b = f.bbox;
        if b[2] < view_box[0] || b[0] > view_box[2] || b[3] < view_box[1] || b[1] > view_box[3] {
            continue;
        }
        // Ponds smaller than ~12 px across are clutter at this scale.
        if f.class == Class::Water && ((b[2] - b[0]).max(b[3] - b[1]) / v.m_per_px) < MIN_WATER_PX {
            continue;
        }
        // Thin to points at least 1.5 px apart; keep the last point.
        let mut pts: Vec<Pos2> = Vec::with_capacity(f.points.len());
        for (i, &(e, n)) in f.points.iter().enumerate() {
            let (x, y) = v.grid_to_screen(e, n);
            let pt = Pos2::new(x, y);
            let last = i + 1 == f.points.len();
            if last || pts.last().is_none_or(|q| q.distance(pt) >= 1.5) {
                pts.push(pt);
            }
        }
        if pts.len() >= 2 {
            painter.add(Shape::line(pts, p.basemap_stroke(f.class)));
            drawn += 1;
        }
    }
    drawn
}
