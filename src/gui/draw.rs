//! Painting: colour palettes, APP-6A frames and icons, ownship.
//!
//! Frames per APP-6A / MIL-STD-2525B: friendly rectangle / dome / circle,
//! hostile diamond / tent, neutral square, unknown quatrefoil; air frames are
//! open at the bottom, subsurface frames open at the top. Icons are simplified
//! (DD-011). All drawn with the painter — no image assets (REQUIREMENTS §5).

use std::f32::consts::{PI, TAU};

use eframe::egui::{self, Align2, Color32, FontId, Mesh, Painter, Pos2, Shape, Stroke, Vec2};

use crate::dis::entity::Status;
use crate::render::symbol::{Dimension, Icon};

// ---------------------------------------------------------------------------
// Palettes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Day,
    Dusk,
    Night,
}

impl Scheme {
    pub fn next(self) -> Self {
        match self {
            Scheme::Day => Scheme::Dusk,
            Scheme::Dusk => Scheme::Night,
            Scheme::Night => Scheme::Day,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Scheme::Day => "Day",
            Scheme::Dusk => "Dusk",
            Scheme::Night => "Night",
        }
    }
}

pub struct Palette {
    pub dark_ui: bool,
    pub background: Color32,
    pub grid: Color32,
    pub grid_text: Color32,
    pub ring: Color32,
    pub ring_text: Color32,
    pub label: Color32,
    pub frame_line: Color32,
    pub ownship: Color32,
    pub trail: Color32,
    pub mark: Color32,
    pub readout_bg: Color32,
    pub readout_text: Color32,
    /// Multiplier on symbol fills (night dims them).
    fill_scale: f32,
    /// Multiplier on panel text colours; < 1 also dims all widgets.
    ui_scale: f32,
    /// Multiplier on basemap line colours.
    basemap_scale: f32,
}

impl Palette {
    pub fn for_scheme(s: Scheme) -> Self {
        match s {
            Scheme::Day => Self {
                dark_ui: false,
                background: Color32::from_rgb(238, 236, 228),
                grid: Color32::from_rgb(170, 170, 160),
                grid_text: Color32::from_rgb(110, 110, 100),
                ring: Color32::from_rgb(90, 90, 90),
                ring_text: Color32::from_rgb(60, 60, 60),
                label: Color32::BLACK,
                frame_line: Color32::BLACK,
                ownship: Color32::from_rgb(255, 170, 0),
                trail: Color32::from_rgb(120, 120, 140),
                mark: Color32::from_rgb(200, 0, 160),
                readout_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                readout_text: Color32::BLACK,
                fill_scale: 1.0,
                ui_scale: 1.0,
                basemap_scale: 1.0,
            },
            Scheme::Dusk => Self {
                dark_ui: true,
                background: Color32::from_rgb(34, 40, 50),
                grid: Color32::from_rgb(70, 80, 95),
                grid_text: Color32::from_rgb(120, 130, 145),
                ring: Color32::from_rgb(150, 160, 175),
                ring_text: Color32::from_rgb(170, 180, 195),
                label: Color32::from_rgb(225, 225, 225),
                frame_line: Color32::from_rgb(15, 15, 15),
                ownship: Color32::from_rgb(255, 170, 0),
                trail: Color32::from_rgb(130, 140, 160),
                mark: Color32::from_rgb(255, 90, 220),
                readout_bg: Color32::from_rgba_unmultiplied(20, 24, 30, 220),
                readout_text: Color32::from_rgb(225, 225, 225),
                fill_scale: 0.85,
                ui_scale: 1.0,
                basemap_scale: 0.55,
            },
            Scheme::Night => Self {
                dark_ui: true,
                background: Color32::BLACK,
                grid: Color32::from_rgb(28, 40, 28),
                grid_text: Color32::from_rgb(60, 85, 60),
                ring: Color32::from_rgb(60, 90, 60),
                ring_text: Color32::from_rgb(80, 115, 80),
                label: Color32::from_rgb(110, 150, 110),
                frame_line: Color32::from_rgb(0, 0, 0),
                ownship: Color32::from_rgb(150, 105, 0),
                trail: Color32::from_rgb(55, 70, 55),
                mark: Color32::from_rgb(130, 60, 115),
                readout_bg: Color32::from_rgba_unmultiplied(0, 0, 0, 220),
                readout_text: Color32::from_rgb(110, 150, 110),
                fill_scale: 0.4,
                ui_scale: 0.45,
                basemap_scale: 0.3,
            },
        }
    }

    /// Basemap line style per class (DD-012): roads warm, water blue, rail
    /// grey; scaled down for Dusk and Night so symbols stay on top visually.
    pub fn basemap_stroke(&self, c: crate::map::basemap::Class) -> Stroke {
        use crate::map::basemap::Class;
        let (rgb, width) = match c {
            Class::Motorway => ((225, 120, 60), 3.2),
            Class::Trunk => ((230, 145, 75), 2.8),
            Class::Primary => ((225, 180, 80), 2.3),
            Class::Secondary => ((185, 175, 140), 1.7),
            Class::Tertiary => ((190, 185, 170), 1.2),
            Class::Rail => ((120, 120, 125), 1.3),
            Class::River => ((100, 155, 215), 2.0),
            Class::Water => ((100, 155, 215), 1.5),
        };
        let colour = scale(Color32::from_rgb(rgb.0, rgb.1, rgb.2), self.basemap_scale);
        Stroke::new(width, colour)
    }

    /// Dim a UI colour (text, list rows) to suit the scheme.
    pub fn dim(&self, c: Color32) -> Color32 {
        scale(c, self.ui_scale)
    }

    /// egui visuals for the panels; Night dims every widget and all text.
    pub fn visuals(&self) -> egui::Visuals {
        let mut v = if self.dark_ui { egui::Visuals::dark() } else { egui::Visuals::light() };
        if self.ui_scale < 1.0 {
            v.override_text_color = Some(self.label);
            v.panel_fill = Color32::from_rgb(4, 6, 4);
            v.window_fill = v.panel_fill;
            v.extreme_bg_color = Color32::BLACK;
            v.selection.bg_fill = Color32::from_rgb(18, 45, 18);
            for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.noninteractive] {
                w.bg_fill = Color32::from_rgb(12, 18, 12);
                w.weak_bg_fill = Color32::from_rgb(12, 18, 12);
                w.fg_stroke.color = self.label;
                w.bg_stroke.color = self.grid;
            }
        }
        v
    }

    /// APP-6A affiliation fill colours.
    pub fn fill(&self, status: Status) -> Color32 {
        let c = match status {
            Status::Friendly => Color32::from_rgb(128, 224, 255),
            Status::Hostile => Color32::from_rgb(255, 128, 128),
            Status::Neutral => Color32::from_rgb(170, 255, 170),
            Status::Unknown => Color32::from_rgb(255, 255, 128),
        };
        scale(c, self.fill_scale)
    }
}

fn scale(c: Color32, k: f32) -> Color32 {
    let f = |v: u8| (v as f32 * k).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(f(c.r()), f(c.g()), f(c.b()), c.a())
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

/// Frame outline relative to the symbol centre (y down), and whether it is
/// open (air / subsurface): the closing segment is filled but not stroked.
fn frame_outline(dim: Dimension, status: Status, s: f32) -> (Vec<Vec2>, bool) {
    let flip = |pts: Vec<Vec2>| pts.into_iter().map(|p| Vec2::new(p.x, -p.y)).collect::<Vec<_>>();
    match (status, dim) {
        (Status::Friendly, Dimension::Ground) => (rect(0.75 * s, 0.5 * s), false),
        (Status::Friendly, Dimension::SeaSurface) => (arc(Vec2::ZERO, 0.6 * s, 0.0, TAU, 40), false),
        (Status::Friendly, Dimension::Air) => (arc(Vec2::new(0.0, 0.2 * s), 0.6 * s, PI, TAU, 24), true),
        (Status::Friendly, Dimension::Subsurface) => (flip(arc(Vec2::new(0.0, 0.2 * s), 0.6 * s, PI, TAU, 24)), true),

        (Status::Hostile, Dimension::Ground | Dimension::SeaSurface) => {
            let r = 0.72 * s;
            (vec![Vec2::new(0.0, -r), Vec2::new(r, 0.0), Vec2::new(0.0, r), Vec2::new(-r, 0.0)], false)
        }
        (Status::Hostile, Dimension::Air) => (tent(s), true),
        (Status::Hostile, Dimension::Subsurface) => (flip(tent(s)), true),

        (Status::Neutral, Dimension::Ground | Dimension::SeaSurface) => (rect(0.58 * s, 0.58 * s), false),
        (Status::Neutral, Dimension::Air) => (open_box(s), true),
        (Status::Neutral, Dimension::Subsurface) => (flip(open_box(s)), true),

        (Status::Unknown, Dimension::Ground | Dimension::SeaSurface) => (quatrefoil(s, 0.0, TAU), false),
        (Status::Unknown, Dimension::Air) => (quatrefoil(s, PI, TAU), true),
        (Status::Unknown, Dimension::Subsurface) => (flip(quatrefoil(s, PI, TAU)), true),
    }
}

fn rect(hw: f32, hh: f32) -> Vec<Vec2> {
    vec![Vec2::new(-hw, -hh), Vec2::new(hw, -hh), Vec2::new(hw, hh), Vec2::new(-hw, hh)]
}

/// Arc from angle a0 to a1 (rad; 0 = +x, π/2 = +y which is screen-down).
fn arc(c: Vec2, r: f32, a0: f32, a1: f32, n: usize) -> Vec<Vec2> {
    (0..=n).map(|i| {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        c + Vec2::new(r * a.cos(), r * a.sin())
    }).collect()
}

fn tent(s: f32) -> Vec<Vec2> {
    vec![Vec2::new(-0.55 * s, 0.35 * s), Vec2::new(-0.55 * s, -0.1 * s), Vec2::new(0.0, -0.65 * s),
         Vec2::new(0.55 * s, -0.1 * s), Vec2::new(0.55 * s, 0.35 * s)]
}

fn open_box(s: f32) -> Vec<Vec2> {
    vec![Vec2::new(-0.55 * s, 0.4 * s), Vec2::new(-0.55 * s, -0.55 * s),
         Vec2::new(0.55 * s, -0.55 * s), Vec2::new(0.55 * s, 0.4 * s)]
}

/// Quatrefoil: outer boundary of four lobes, sampled between angles a0, a1.
fn quatrefoil(s: f32, a0: f32, a1: f32) -> Vec<Vec2> {
    let (d, rl) = (0.3 * s, 0.36 * s);
    let lobes = [Vec2::new(0.0, -d), Vec2::new(d, 0.0), Vec2::new(0.0, d), Vec2::new(-d, 0.0)];
    let n = 64;
    (0..=n).map(|i| {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        let u = Vec2::new(a.cos(), a.sin());
        let t = lobes.iter().filter_map(|c| {
            let b = c.dot(u);
            let disc = b * b - c.length_sq() + rl * rl;
            (disc >= 0.0).then(|| b + disc.sqrt())
        }).fold(0.0f32, f32::max);
        u * t
    }).collect()
}

/// Fill a star-shaped polygon with a triangle fan from its mean point.
fn fill_star(painter: &Painter, centre: Pos2, pts: &[Vec2], colour: Color32) {
    let mean = pts.iter().fold(Vec2::ZERO, |a, p| a + *p) / pts.len() as f32;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre + mean, colour);
    for p in pts {
        mesh.colored_vertex(centre + *p, colour);
    }
    let n = pts.len() as u32;
    for i in 1..n {
        mesh.add_triangle(0, i, i + 1);
    }
    mesh.add_triangle(0, n, 1);
    painter.add(Shape::mesh(mesh));
}

/// Draw a complete symbol. `alpha` < 1 dims stale entities. Returns the
/// frame's half-width, for placing labels.
pub fn symbol(painter: &Painter, p: &Palette, centre: Pos2, dim: Dimension, icon: Icon,
              status: Status, s: f32, alpha: f32) -> f32 {
    let (outline, open) = frame_outline(dim, status, s);
    let fill = p.fill(status).gamma_multiply(alpha);
    let line = Stroke::new(2.0, p.frame_line.gamma_multiply(alpha.max(0.5)));
    fill_star(painter, centre, &outline, fill);
    let pts: Vec<Pos2> = outline.iter().map(|v| centre + *v).collect();
    if open {
        painter.add(Shape::line(pts, line));
    } else {
        painter.add(Shape::closed_line(pts, line));
    }
    let icon_centre = match dim {
        Dimension::Air => centre + Vec2::new(0.0, -0.05 * s),
        Dimension::Subsurface => centre + Vec2::new(0.0, 0.05 * s),
        _ => centre,
    };
    draw_icon(painter, icon_centre, icon, s, Stroke::new(1.6, line.color), fill);
    outline.iter().map(|v| v.x.abs()).fold(0.0, f32::max)
}

// ---------------------------------------------------------------------------
// Icons
// ---------------------------------------------------------------------------

fn draw_icon(painter: &Painter, c: Pos2, icon: Icon, s: f32, st: Stroke, fill: Color32) {
    let at = |x: f32, y: f32| c + Vec2::new(x * s, y * s);
    let text = |t: &str, size: f32| {
        painter.text(c, Align2::CENTER_CENTER, t, FontId::proportional(size * s), st.color);
    };
    let ellipse = |hw: f32, hh: f32| {
        let pts: Vec<Pos2> = (0..32).map(|i| {
            let a = TAU * i as f32 / 32.0;
            at(hw * a.cos(), hh * a.sin())
        }).collect();
        painter.add(Shape::closed_line(pts, st));
    };
    let wheels = |xs: &[f32], y: f32| {
        for x in xs {
            painter.circle_filled(at(*x, y), 0.08 * s, st.color);
        }
    };
    match icon {
        Icon::Armour => ellipse(0.45, 0.22),
        Icon::ArmouredFightingVehicle => {
            ellipse(0.45, 0.22);
            painter.line_segment([at(-0.7, -0.45), at(0.7, 0.45)], st);
            painter.line_segment([at(-0.7, 0.45), at(0.7, -0.45)], st);
        }
        Icon::ArmouredUtility => {
            ellipse(0.45, 0.22);
            painter.text(at(0.0, 0.0), Align2::CENTER_CENTER, "U", FontId::proportional(0.3 * s), st.color);
        }
        Icon::Artillery => {
            painter.circle_filled(c, 0.13 * s, st.color);
        }
        Icon::Infantry => {
            painter.line_segment([at(-0.7, -0.45), at(0.7, 0.45)], st);
            painter.line_segment([at(-0.7, 0.45), at(0.7, -0.45)], st);
        }
        Icon::Car => {
            painter.add(Shape::closed_line(vec![at(-0.45, 0.12), at(-0.45, -0.05), at(-0.25, -0.08),
                at(-0.15, -0.25), at(0.18, -0.25), at(0.3, -0.08), at(0.45, -0.05), at(0.45, 0.12)], st));
            wheels(&[-0.27, 0.27], 0.15);
        }
        Icon::Bus | Icon::SchoolBus => {
            let body = vec![at(-0.55, -0.28), at(0.55, -0.28), at(0.55, 0.12), at(-0.55, 0.12)];
            if icon == Icon::SchoolBus {
                fill_star(painter, c, &body.iter().map(|p| *p - c).collect::<Vec<_>>(),
                          Color32::from_rgb(255, 200, 0).gamma_multiply(fill.a() as f32 / 255.0));
            }
            painter.add(Shape::closed_line(body, st));
            painter.line_segment([at(-0.45, -0.12), at(0.45, -0.12)], st);
            wheels(&[-0.33, 0.33], 0.15);
        }
        Icon::Truck => {
            painter.add(Shape::closed_line(vec![at(-0.55, -0.3), at(0.15, -0.3), at(0.15, 0.12), at(-0.55, 0.12)], st));
            painter.add(Shape::closed_line(vec![at(0.2, -0.15), at(0.42, -0.15), at(0.55, 0.0), at(0.55, 0.12), at(0.2, 0.12)], st));
            wheels(&[-0.35, 0.05, 0.38], 0.15);
        }
        Icon::AttackHelicopter | Icon::UtilityHelicopter | Icon::Helicopter => {
            painter.add(Shape::closed_line(vec![at(-0.42, -0.18), at(0.42, 0.18), at(0.42, -0.18), at(-0.42, 0.18)], st));
            let letter = match icon {
                Icon::AttackHelicopter => "A",
                Icon::UtilityHelicopter => "U",
                _ => "",
            };
            if !letter.is_empty() {
                painter.text(at(0.0, 0.3), Align2::CENTER_CENTER, letter, FontId::proportional(0.28 * s), st.color);
            }
        }
        Icon::FixedWing | Icon::CivilAircraft => {
            painter.line_segment([at(0.0, -0.4), at(0.0, 0.3)], st);
            painter.add(Shape::line(vec![at(-0.42, 0.05), at(0.0, -0.12), at(0.42, 0.05)], st));
            painter.line_segment([at(-0.16, 0.28), at(0.16, 0.28)], st);
            if icon == Icon::CivilAircraft {
                painter.text(at(0.3, 0.25), Align2::CENTER_CENTER, "C", FontId::proportional(0.26 * s), st.color);
            }
        }
        Icon::Ship => {
            painter.add(Shape::closed_line(vec![at(-0.42, -0.05), at(0.42, -0.05), at(0.3, 0.15), at(-0.3, 0.15)], st));
            painter.line_segment([at(0.0, -0.05), at(0.0, -0.3)], st);
        }
        Icon::Horse | Icon::HorseWithRider | Icon::Animal | Icon::MilitaryVehicle
        | Icon::CivilianVehicle | Icon::Uav | Icon::PatrolCraft | Icon::Submarine => {
            let t = match icon {
                Icon::HorseWithRider => "HRS R",
                other => other.code(),
            };
            text(t, if t.len() > 3 { 0.26 } else { 0.34 });
        }
        Icon::Unknown => text("?", 0.5),
    }
}

// ---------------------------------------------------------------------------
// Ownship
// ---------------------------------------------------------------------------

/// Ownship: a solid aircraft shape pointing along `angle` (screen rad,
/// clockwise from up), distinct from any APP-6A frame.
pub fn ownship(painter: &Painter, p: &Palette, c: Pos2, angle: f32, s: f32) {
    let rot = |x: f32, y: f32| {
        let (sn, cs) = angle.sin_cos();
        c + Vec2::new(x * cs - y * sn, x * sn + y * cs) * s
    };
    let body = [rot(0.0, -0.8), rot(0.55, 0.55), rot(0.0, 0.3), rot(-0.55, 0.55)];
    painter.add(Shape::convex_polygon(vec![body[0], body[1], body[2]], p.ownship, Stroke::NONE));
    painter.add(Shape::convex_polygon(vec![body[0], body[2], body[3]], p.ownship, Stroke::NONE));
    painter.add(Shape::closed_line(body.to_vec(), Stroke::new(2.0, p.frame_line)));
}

/// Direction arrow for the north indicator; `angle` is where true north
/// points on screen (rad, clockwise from up).
pub fn north_arrow(painter: &Painter, p: &Palette, c: Pos2, angle: f32, r: f32) {
    let (sn, cs) = angle.sin_cos();
    let dir = Vec2::new(sn, -cs);
    let side = Vec2::new(cs, sn);
    let tip = c + dir * r;
    let tail = c - dir * r * 0.6;
    painter.add(Shape::convex_polygon(vec![tip, c + side * r * 0.35, tail, c - side * r * 0.35],
                                      p.ring, Stroke::new(1.5, p.ring_text)));
    painter.text(c + dir * (r + 12.0), Align2::CENTER_CENTER, "N", FontId::proportional(16.0), p.ring_text);
}

pub fn dashed(painter: &Painter, a: Pos2, b: Pos2, stroke: Stroke) {
    painter.extend(Shape::dashed_line(&[a, b], stroke, 8.0, 6.0));
}

pub fn ring(painter: &Painter, c: Pos2, r: f32, stroke: Stroke) {
    painter.circle_stroke(c, r, stroke);
}

/// Text with a translucent backing box, top-left anchored.
pub fn boxed_text(painter: &Painter, p: &Palette, pos: Pos2, lines: &[String], size: f32) {
    let font = FontId::monospace(size);
    let galleys: Vec<_> = lines.iter()
        .map(|l| painter.layout_no_wrap(l.clone(), font.clone(), p.readout_text))
        .collect();
    let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max);
    let h: f32 = galleys.iter().map(|g| g.size().y).sum();
    let rect = egui::Rect::from_min_size(pos, Vec2::new(w + 16.0, h + 12.0));
    painter.rect_filled(rect, 6.0, p.readout_bg);
    let mut y = pos.y + 6.0;
    for g in galleys {
        let gh = g.size().y;
        painter.galley(Pos2::new(pos.x + 8.0, y), g, p.readout_text);
        y += gh;
    }
}
