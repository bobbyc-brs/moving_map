//! Touch-screen moving map (DD-010, DD-011): egui/eframe, OpenGL (glow).
//!
//! Layout: status bar (top), map (centre), control bar (bottom), and an
//! optional side panel with the entity list or the selected entity's details.
//! Touch: tap selects an entity or drops a mark; drag pans; pinch zooms.

mod draw;
pub mod feed;
pub mod view;

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, Color32, FontId, Pos2, RichText, Sense, Stroke, TextStyle, Vec2};

use crate::dis::entity::{EntityId, EntityRecord, Status};
use crate::dis::DisConfig;
use crate::map::mgrs::to_mgrs;
use crate::render::symbol::{classify, Dimension};
use draw::Palette;
pub use draw::Scheme;
use view::{View, M_PER_NM};

/// Dim a symbol once it has been silent this long. DIS sends a heartbeat
/// every 5 s for an unchanging entity; it is dropped at 12 s (ENTITY_TIMEOUT_SECS).
const STALE_SECS: f64 = 7.0;
/// Dead-reckon displayed positions for at most this long after the last PDU.
const DR_MAX_SECS: f64 = 5.0;
const TRAIL_SAMPLE: Duration = Duration::from_secs(2);
/// 150 samples × 2 s = 5 minutes of trail.
const TRAIL_LEN: usize = 150;
const SYMBOL_SIZE: f32 = 34.0;
const TAP_RADIUS_PX: f32 = 30.0;
const KT_PER_MS: f64 = 1.943_844;
const FT_PER_M: f64 = 3.280_84;

pub struct GuiOptions {
    pub dis: DisConfig,
    pub ownship: Option<EntityId>,
    pub centre: Option<(f64, f64)>,
    pub range_nm: f64,
    pub fullscreen: bool,
    /// Save a PPM screenshot to this path after `screenshot_after`, then exit.
    pub screenshot: Option<String>,
    pub screenshot_after: f64,
    pub track_up: bool,
    pub scheme: Scheme,
    /// Open the entity list at start.
    pub list: bool,
}

pub fn run(opts: GuiOptions) -> eframe::Result {
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Moving Map")
            .with_inner_size([1280.0, 800.0])
            .with_fullscreen(opts.fullscreen),
        ..Default::default()
    };
    eframe::run_native("moving-map-display", native, Box::new(|cc| {
        let ctx = cc.egui_ctx.clone();
        let shared = feed::spawn(opts.dis.clone(), move || ctx.request_repaint());
        Ok(Box::new(MapApp::new(&cc.egui_ctx, opts, shared)))
    }))
}

#[derive(PartialEq)]
enum SidePanel {
    None,
    List,
}

struct MapApp {
    opts: GuiOptions,
    shared: Arc<Mutex<feed::Shared>>,
    started: Instant,
    range_nm: f64,
    track_up: bool,
    scheme: Scheme,
    show_rings: bool,
    show_grid: bool,
    show_trails: bool,
    show_labels: bool,
    side: SidePanel,
    held_zone: Option<u32>,
    /// `Some` when the operator has panned away from ownship.
    pan_centre: Option<(f64, f64)>,
    selected: Option<EntityId>,
    mark: Option<(f64, f64)>,
    trails: HashMap<EntityId, VecDeque<(f64, f64)>>,
    last_trail: Instant,
    screenshot_requested: bool,
    screenshot_done: Arc<Mutex<Option<Result<(), String>>>>,
}

impl MapApp {
    fn new(ctx: &egui::Context, opts: GuiOptions, shared: Arc<Mutex<feed::Shared>>) -> Self {
        touch_style(ctx);
        let list = opts.list;
        Self {
            range_nm: opts.range_nm,
            track_up: opts.track_up,
            scheme: opts.scheme,
            opts,
            shared,
            started: Instant::now(),
            show_rings: true,
            show_grid: true,
            show_trails: true,
            show_labels: true,
            side: if list { SidePanel::List } else { SidePanel::None },
            held_zone: None,
            pan_centre: None,
            selected: None,
            mark: None,
            trails: HashMap::new(),
            last_trail: Instant::now(),
            screenshot_requested: false,
            screenshot_done: Arc::new(Mutex::new(None)),
        }
    }
}

/// Large controls and text for a touch screen (≥ 48 px targets).
fn touch_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.spacing.interact_size = Vec2::new(64.0, 48.0);
        style.spacing.button_padding = Vec2::new(14.0, 10.0);
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        for (text_style, font) in style.text_styles.iter_mut() {
            font.size = match text_style {
                TextStyle::Heading => 24.0,
                TextStyle::Button => 20.0,
                TextStyle::Body => 18.0,
                TextStyle::Monospace => 17.0,
                TextStyle::Small => 14.0,
                _ => font.size,
            };
        }
    });
}

/// An entity as displayed: dead-reckoned position and age.
struct Shown<'a> {
    rec: &'a EntityRecord,
    lat: f64,
    lon: f64,
    age: f64,
}

fn dead_reckon(rec: &EntityRecord) -> Shown<'_> {
    let age = rec.age_seconds();
    let (lat, lon) = if rec.speed > 0.2 {
        view::offset(rec.latitude, rec.longitude, rec.heading as f64, rec.speed as f64 * age.min(DR_MAX_SECS))
    } else {
        (rec.latitude, rec.longitude)
    };
    Shown { rec, lat, lon, age }
}

impl eframe::App for MapApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let palette = Palette::for_scheme(self.scheme);
        ctx.set_visuals(palette.visuals());
        touch_style(&ctx);

        // Snapshot the shared table so the lock is not held while drawing.
        let (mut entities, stats, error) = {
            let s = self.shared.lock().unwrap();
            (s.table.iter().cloned().collect::<Vec<_>>(), s.stats, s.error.clone())
        };
        entities.sort_by_key(|e| (e.entity_id.site, e.entity_id.application, e.entity_id.entity));
        let shown: Vec<Shown> = entities.iter().map(dead_reckon).collect();
        let own = self.opts.ownship.and_then(|id| shown.iter().find(|s| s.rec.entity_id == id));
        let own_pos = own.map(|o| (o.lat, o.lon));
        self.update_trails(&shown);

        if let Some(sel) = self.selected {
            if !entities.iter().any(|e| e.entity_id == sel) {
                self.selected = None;
            }
        }

        self.status_bar(ui, &palette, own, stats, error.as_deref());
        self.control_bar(ui);
        if self.side == SidePanel::List || self.selected.is_some() {
            self.side_panel(ui, &palette, &shown, own_pos);
        }

        egui::CentralPanel::no_frame().show(ui, |ui| {
            self.map(ui, &palette, &shown, own);
        });

        self.screenshot(&ctx);
        // Keep dead reckoning and stale dimming moving between PDUs.
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

impl MapApp {
    fn update_trails(&mut self, shown: &[Shown]) {
        if self.last_trail.elapsed() < TRAIL_SAMPLE {
            return;
        }
        self.last_trail = Instant::now();
        let live: Vec<EntityId> = shown.iter().map(|s| s.rec.entity_id).collect();
        self.trails.retain(|id, _| live.contains(id));
        for s in shown {
            let t = self.trails.entry(s.rec.entity_id).or_default();
            t.push_back((s.lat, s.lon));
            while t.len() > TRAIL_LEN {
                t.pop_front();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Bars and panels
    // -----------------------------------------------------------------------

    fn status_bar(&self, ui: &mut egui::Ui, p: &Palette, own: Option<&Shown>, stats: crate::dis::receiver::Stats, error: Option<&str>) {
        egui::Panel::top("status").exact_size(52.0).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                match (own, self.opts.ownship) {
                    (Some(o), _) => {
                        let r = o.rec;
                        ui.label(RichText::new(&r.name).strong().size(20.0));
                        ui.label(format!("HDG {:03.0}°T", r.heading));
                        ui.label(format!("GS {:.0} kt", r.speed as f64 * KT_PER_MS));
                        ui.label(format!("ALT {:.0} ft", r.altitude * FT_PER_M));
                        if let Some(m) = to_mgrs(o.lat, o.lon, 5) {
                            ui.label(RichText::new(m).monospace());
                        }
                    }
                    (None, Some(id)) => {
                        ui.label(RichText::new(format!("Waiting for ownship {}", id.display())).color(p.dim(Color32::from_rgb(230, 150, 0))));
                    }
                    (None, None) => {
                        ui.label("No ownship set (--ownship s.a.e)");
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let ex = match self.opts.dis.exercise_id {
                        Some(e) => format!("ex {e}"),
                        None => "all ex".into(),
                    };
                    match error {
                        Some(e) => {
                            ui.label(RichText::new(e).color(p.dim(Color32::from_rgb(220, 40, 40))));
                        }
                        None => {
                            let text = format!("DIS :{} {}  ·  {} PDUs  ·  {} bad", self.opts.dis.port, ex, stats.pdus, stats.malformed);
                            let colour = if stats.malformed > 0 { p.dim(Color32::from_rgb(220, 140, 0)) } else { p.label };
                            ui.label(RichText::new(text).color(colour).size(15.0));
                        }
                    }
                });
            });
        });
    }

    fn control_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("controls").exact_size(68.0).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if ui.button(RichText::new("  −  ").size(26.0)).clicked() {
                    self.range_nm = view::step_range(self.range_nm, false);
                }
                ui.label(RichText::new(format!("{} NM", fmt_nm(self.range_nm))).size(22.0).strong());
                if ui.button(RichText::new("  +  ").size(26.0)).clicked() {
                    self.range_nm = view::step_range(self.range_nm, true);
                }
                ui.separator();
                if ui.button(if self.track_up { "Track up" } else { "North up" }).clicked() {
                    self.track_up = !self.track_up;
                }
                if ui.add_enabled(self.pan_centre.is_some(), egui::Button::new("Centre")).clicked() {
                    self.pan_centre = None;
                }
                ui.separator();
                toggle(ui, &mut self.show_rings, "Rings");
                toggle(ui, &mut self.show_grid, "Grid");
                toggle(ui, &mut self.show_trails, "Trails");
                toggle(ui, &mut self.show_labels, "Labels");
                ui.separator();
                if ui.button(self.scheme.label()).clicked() {
                    self.scheme = self.scheme.next();
                }
                let mut list = self.side == SidePanel::List;
                if toggle(ui, &mut list, "List") {
                    self.side = if list { SidePanel::List } else { SidePanel::None };
                }
                if self.mark.is_some() && ui.button("Clear mark").clicked() {
                    self.mark = None;
                }
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui, p: &Palette, shown: &[Shown], own_pos: Option<(f64, f64)>) {
        egui::Panel::right("side").exact_size(360.0).show(ui, |ui| {
            if let Some(sel) = self.selected {
                if let Some(s) = shown.iter().find(|s| s.rec.entity_id == sel) {
                    if ui.button("✕ Close").clicked() {
                        self.selected = None;
                    }
                    details(ui, p, s, own_pos);
                    ui.separator();
                }
            }
            if self.side == SidePanel::List {
                ui.heading(format!("{} entities", shown.len()));
                let mut rows: Vec<(f64, &Shown)> = shown.iter().map(|s| {
                    let r = own_pos.map(|o| view::range_bearing(o, (s.lat, s.lon)).0).unwrap_or(0.0);
                    (r, s)
                }).collect();
                rows.sort_by(|a, b| a.0.total_cmp(&b.0));
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (range, s) in rows {
                        let (_, icon) = classify(&s.rec.entity_type);
                        let label = format!("{:<10} {:<4} {:>6.1} NM", s.rec.name, icon.code(), range / M_PER_NM);
                        let text = RichText::new(label).monospace().color(p.dim(status_colour(s.rec.status)));
                        if ui.selectable_label(self.selected == Some(s.rec.entity_id), text).clicked() {
                            self.selected = Some(s.rec.entity_id);
                        }
                    }
                });
            }
        });
    }

    // -----------------------------------------------------------------------
    // Map
    // -----------------------------------------------------------------------

    fn map(&mut self, ui: &mut egui::Ui, p: &Palette, shown: &[Shown], own: Option<&Shown>) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, p.background);

        let centre = self.pan_centre
            .or(own.map(|o| (o.lat, o.lon)))
            .or(self.opts.centre)
            .or(shown.first().map(|s| (s.lat, s.lon)))
            .unwrap_or((45.0, -75.0));
        let zone_lon = own.map(|o| o.lon).unwrap_or(centre.1);
        let zone = view::hold_zone(self.held_zone, zone_lon);
        self.held_zone = Some(zone);

        let up = match (self.track_up, own) {
            (true, Some(o)) => o.rec.heading as f64,
            _ => 0.0,
        };
        // Track-up looks ahead: ownship sits low on the screen.
        let centre_px = if self.track_up && self.pan_centre.is_none() {
            (rect.center().x, rect.top() + rect.height() * 0.68)
        } else {
            (rect.center().x, rect.center().y)
        };
        let mpp = view::m_per_px_for_range(self.range_nm, rect.width(), rect.height());
        let v = View::new(zone, centre, centre_px, mpp, up);
        let pos = |lat: f64, lon: f64| {
            let (x, y) = v.to_screen(lat, lon);
            Pos2::new(x, y)
        };
        let painter = painter.with_clip_rect(rect);

        // --- input: pinch / scroll zoom, drag pan, tap select / mark ---
        if response.hovered() || response.dragged() {
            let (zoom, scroll) = ui.ctx().input(|i| (i.zoom_delta(), i.smooth_scroll_delta().y));
            let mut factor = zoom as f64;
            if scroll != 0.0 {
                factor *= (scroll as f64 / 200.0).exp();
            }
            if (factor - 1.0).abs() > 1e-4 {
                self.range_nm = (self.range_nm / factor).clamp(0.5, 600.0);
            }
        }
        if response.dragged() && ui.ctx().input(|i| i.multi_touch().is_none()) {
            let d = response.drag_delta();
            if d.length() > 0.0 {
                let (lat, lon) = v.to_latlon(centre_px.0 - d.x, centre_px.1 - d.y);
                self.pan_centre = Some((lat, lon));
            }
        }
        if response.clicked() {
            if let Some(tap) = response.interact_pointer_pos() {
                let hit = shown.iter()
                    .map(|s| (pos(s.lat, s.lon).distance(tap), s.rec.entity_id))
                    .filter(|(d, _)| *d <= TAP_RADIUS_PX)
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                match hit {
                    Some((_, id)) => self.selected = Some(id),
                    None => {
                        self.selected = None;
                        self.mark = Some(v.to_latlon(tap.x, tap.y));
                    }
                }
            }
        }

        if self.show_grid {
            draw_grid(&painter, p, &v, rect);
        }

        let own_px = own.map(|o| pos(o.lat, o.lon));
        if self.show_rings {
            if let Some(c) = own_px {
                let step = view::ring_step_nm(self.range_nm);
                let mut r = step;
                while r <= self.range_nm * 1.5 + 1e-9 {
                    let px = (r * M_PER_NM / mpp) as f32;
                    draw::ring(&painter, c, px, Stroke::new(1.2, p.ring));
                    let label_at = c + Vec2::new(px * 0.707, -px * 0.707);
                    painter.text(label_at, Align2::LEFT_BOTTOM, fmt_nm(r), FontId::proportional(14.0), p.ring_text);
                    r += step;
                }
            }
        }

        if self.show_trails {
            for (id, t) in &self.trails {
                if t.len() < 2 {
                    continue;
                }
                let is_own = Some(*id) == self.opts.ownship;
                let colour = if is_own { p.ownship } else { p.trail };
                let pts: Vec<Pos2> = t.iter().map(|(la, lo)| pos(*la, *lo)).collect();
                painter.add(egui::Shape::line(pts, Stroke::new(if is_own { 3.0 } else { 1.5 }, colour)));
            }
        }

        // Entities: leaders first, then symbols, then labels on top.
        for s in shown.iter().filter(|s| Some(s.rec.entity_id) != self.opts.ownship) {
            let c = pos(s.lat, s.lon);
            if s.rec.speed > 0.5 {
                let (dx, dy) = v.true_vector(s.rec.heading as f64, s.rec.speed as f64 * 60.0);
                let mut d = Vec2::new(dx, dy);
                if d.length() > 160.0 {
                    d = d.normalized() * 160.0;
                }
                if d.length() > SYMBOL_SIZE * 0.5 {
                    painter.line_segment([c, c + d], Stroke::new(2.0, p.label));
                }
            }
        }
        for s in shown.iter().filter(|s| Some(s.rec.entity_id) != self.opts.ownship) {
            let c = pos(s.lat, s.lon);
            let (dim, icon) = classify(&s.rec.entity_type);
            let alpha = if s.age > STALE_SECS { 0.35 } else { 1.0 };
            let half_w = draw::symbol(&painter, p, c, dim, icon, s.rec.status, SYMBOL_SIZE, alpha);
            if self.selected == Some(s.rec.entity_id) {
                painter.circle_stroke(c, SYMBOL_SIZE * 0.85, Stroke::new(3.0, p.mark));
            }
            if self.show_labels {
                let mut text = s.rec.name.clone();
                if dim == Dimension::Air {
                    text.push_str(&format!("\n{:03.0} {:.0}kt", s.rec.altitude * FT_PER_M / 100.0, s.rec.speed as f64 * KT_PER_MS));
                }
                if s.age > STALE_SECS {
                    text.push_str(&format!("\n{:.0}s", s.age));
                }
                painter.text(c + Vec2::new(half_w + 6.0, 0.0), Align2::LEFT_CENTER, text,
                             FontId::proportional(14.0), p.label.gamma_multiply(alpha.max(0.6)));
            }
        }

        if let (Some(o), Some(c)) = (own, own_px) {
            let a = v.screen_angle(o.rec.heading as f64);
            let far = c + Vec2::new(a.sin(), -a.cos()) * rect.size().length();
            draw::dashed(&painter, c, far, Stroke::new(1.5, p.ownship));
            draw::ownship(&painter, p, c, a, SYMBOL_SIZE * 0.9);
        }

        // North indicator (top-right): where true north points on screen.
        draw::north_arrow(&painter, p, rect.right_top() + Vec2::new(-40.0, 48.0), v.screen_angle(0.0), 22.0);
        draw_scale_bar(&painter, p, &v, rect);
        if let Some(z) = self.held_zone {
            painter.text(rect.right_bottom() + Vec2::new(-10.0, -10.0), Align2::RIGHT_BOTTOM,
                         format!("UTM {z}"), FontId::proportional(14.0), p.grid_text);
        }

        // Readout: the mark if set, else the hovered point (mouse only).
        let target = self.mark.or_else(|| response.hover_pos().map(|h| v.to_latlon(h.x, h.y)));
        if let Some(m) = self.mark {
            let c = pos(m.0, m.1);
            painter.line_segment([c - Vec2::X * 12.0, c + Vec2::X * 12.0], Stroke::new(2.5, p.mark));
            painter.line_segment([c - Vec2::Y * 12.0, c + Vec2::Y * 12.0], Stroke::new(2.5, p.mark));
            if let Some(oc) = own_px {
                draw::dashed(&painter, oc, c, Stroke::new(1.5, p.mark));
            }
        }
        if let Some((lat, lon)) = target {
            let mut lines = vec![
                to_mgrs(lat, lon, 5).unwrap_or_else(|| "outside MGRS".into()),
                format!("{}  {}", fmt_dms(lat, 'N', 'S'), fmt_dms(lon, 'E', 'W')),
            ];
            if let Some(o) = own {
                let (r, b) = view::range_bearing((o.lat, o.lon), (lat, lon));
                lines.push(format!("{:.2} NM  {:03.0}°T from ownship", r / M_PER_NM, b));
            }
            draw::boxed_text(&painter, p, rect.left_top() + Vec2::new(10.0, 10.0), &lines, 16.0);
        }
    }

    fn screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.opts.screenshot.clone() else { return };
        if let Some(result) = self.screenshot_done.lock().unwrap().take() {
            match result {
                Ok(()) => eprintln!("screenshot saved: {path}"),
                Err(e) => eprintln!("screenshot failed: {e}"),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if !self.screenshot_requested && self.started.elapsed().as_secs_f64() >= self.opts.screenshot_after {
            self.screenshot_requested = true;
            let done = Arc::clone(&self.screenshot_done);
            let wake = ctx.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |img| {
                *done.lock().unwrap() = Some(write_ppm(&path, &img));
                wake.request_repaint();
            })));
        }
    }
}

/// A framed on/off button — unlike `toggle_value`, it still looks like a
/// button when off, which matters on a touch screen. Returns true if tapped.
fn toggle(ui: &mut egui::Ui, on: &mut bool, label: &str) -> bool {
    let tapped = ui.add(egui::Button::new(label).selected(*on)).clicked();
    if tapped {
        *on = !*on;
    }
    tapped
}

fn details(ui: &mut egui::Ui, p: &Palette, s: &Shown, own_pos: Option<(f64, f64)>) {
    let r = s.rec;
    let (dim, icon) = classify(&r.entity_type);
    ui.heading(&r.name);
    ui.label(RichText::new(format!("{}  ·  {}", status_name(r.status), icon.description())).color(p.dim(status_colour(r.status))).strong());
    egui::Grid::new("details").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        let mut row = |k: &str, v: String| {
            ui.label(k);
            ui.label(RichText::new(v).monospace());
            ui.end_row();
        };
        row("Entity ID", r.entity_id.display());
        row("DIS type", r.entity_type.display());
        row("Force ID", format!("{} ({})", r.force_id, status_name(r.status)));
        row("Dimension", format!("{dim:?}"));
        row("MGRS", to_mgrs(s.lat, s.lon, 5).unwrap_or_default());
        row("Lat", fmt_dms(s.lat, 'N', 'S'));
        row("Lon", fmt_dms(s.lon, 'E', 'W'));
        row("Altitude", format!("{:.0} ft ({:.0} m)", r.altitude * FT_PER_M, r.altitude));
        row("Heading", format!("{:03.0}°T", r.heading));
        row("Speed", format!("{:.0} kt", r.speed as f64 * KT_PER_MS));
        row("Pitch / roll", format!("{:+.1}° / {:+.1}°", r.pitch, r.roll));
        if let Some(o) = own_pos {
            let (rng, brg) = view::range_bearing(o, (s.lat, s.lon));
            row("From ownship", format!("{:.2} NM {:03.0}°T", rng / M_PER_NM, brg));
        }
        row("Last heard", format!("{:.1} s ago", s.age));
    });
}

fn draw_grid(painter: &egui::Painter, p: &Palette, v: &View, rect: egui::Rect) {
    let step = view::grid_step_m(v.m_per_px, 70.0);
    // Grid extent: the screen corners in grid coordinates.
    let corners = [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()]
        .map(|c| v.screen_to_grid(c.x, c.y));
    let (mut e0, mut e1, mut n0, mut n1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for (e, n) in corners {
        e0 = e0.min(e);
        e1 = e1.max(e);
        n0 = n0.min(n);
        n1 = n1.max(n);
    }
    let stroke = Stroke::new(1.0, p.grid);
    let font = FontId::proportional(12.0);
    // Label in MGRS style: kilometres within the 100 km square.
    let label = |m: f64| -> String {
        let km = (m.rem_euclid(100_000.0) / 1000.0).floor();
        if step >= 100_000.0 { format!("{:.0}", m / 100_000.0) } else { format!("{km:02.0}") }
    };
    let mut e = (e0 / step).floor() * step;
    while e <= e1 {
        let (ax, ay) = v.grid_to_screen(e, n0);
        let (bx, by) = v.grid_to_screen(e, n1);
        painter.line_segment([Pos2::new(ax, ay), Pos2::new(bx, by)], stroke);
        if let Some(at) = edge_point(Pos2::new(ax, ay), Pos2::new(bx, by), rect, true) {
            painter.text(at, Align2::CENTER_BOTTOM, label(e), font.clone(), p.grid_text);
        }
        e += step;
    }
    let mut n = (n0 / step).floor() * step;
    while n <= n1 {
        let (ax, ay) = v.grid_to_screen(e0, n);
        let (bx, by) = v.grid_to_screen(e1, n);
        painter.line_segment([Pos2::new(ax, ay), Pos2::new(bx, by)], stroke);
        if let Some(at) = edge_point(Pos2::new(ax, ay), Pos2::new(bx, by), rect, false) {
            painter.text(at, Align2::LEFT_CENTER, label(n), font.clone(), p.grid_text);
        }
        n += step;
    }
}

/// Where a grid line meets the bottom edge (eastings) or left edge
/// (northings), nudged inside the map, if it does.
fn edge_point(a: Pos2, b: Pos2, rect: egui::Rect, bottom: bool) -> Option<Pos2> {
    let inside = rect.shrink(4.0);
    if bottom {
        let y = inside.bottom() - 2.0;
        if (a.y - b.y).abs() < 1e-3 { return None; }
        let t = (y - a.y) / (b.y - a.y);
        let x = a.x + t * (b.x - a.x);
        (inside.left() + 20.0..=inside.right() - 60.0).contains(&x).then(|| Pos2::new(x, y))
    } else {
        let x = inside.left() + 2.0;
        if (a.x - b.x).abs() < 1e-3 { return None; }
        let t = (x - a.x) / (b.x - a.x);
        let y = a.y + t * (b.y - a.y);
        (inside.top() + 110.0..=inside.bottom() - 60.0).contains(&y).then(|| Pos2::new(x, y))
    }
}

fn draw_scale_bar(painter: &egui::Painter, p: &Palette, v: &View, rect: egui::Rect) {
    // A round number of NM about a fifth of the map width.
    let target_nm = rect.width() as f64 * 0.2 * v.m_per_px / M_PER_NM;
    let nice = [0.1, 0.2, 0.25, 0.5, 1.0, 2.0, 2.5, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0]
        .into_iter().rev().find(|n| *n <= target_nm).unwrap_or(0.1);
    let px = (nice * M_PER_NM / v.m_per_px) as f32;
    let y = rect.bottom() - 22.0;
    let x0 = rect.left() + 16.0;
    let st = Stroke::new(3.0, p.ring_text);
    painter.line_segment([Pos2::new(x0, y), Pos2::new(x0 + px, y)], st);
    painter.line_segment([Pos2::new(x0, y - 7.0), Pos2::new(x0, y + 1.0)], st);
    painter.line_segment([Pos2::new(x0 + px, y - 7.0), Pos2::new(x0 + px, y + 1.0)], st);
    painter.text(Pos2::new(x0 + px + 8.0, y), Align2::LEFT_CENTER, format!("{} NM", fmt_nm(nice)),
                 FontId::proportional(14.0), p.ring_text);
}

fn status_name(s: Status) -> &'static str {
    match s {
        Status::Friendly => "Friendly",
        Status::Hostile => "Hostile",
        Status::Neutral => "Neutral",
        Status::Unknown => "Unknown",
    }
}

fn status_colour(s: Status) -> Color32 {
    match s {
        Status::Friendly => Color32::from_rgb(40, 140, 220),
        Status::Hostile => Color32::from_rgb(220, 50, 50),
        Status::Neutral => Color32::from_rgb(40, 170, 60),
        Status::Unknown => Color32::from_rgb(200, 170, 0),
    }
}

fn fmt_nm(nm: f64) -> String {
    if nm >= 10.0 || (nm - nm.round()).abs() < 1e-9 { format!("{nm:.0}") } else { format!("{nm}") }
}

/// Degrees, minutes, tenths of seconds: `45°25′17.4″N`.
fn fmt_dms(v: f64, pos: char, neg: char) -> String {
    let h = if v >= 0.0 { pos } else { neg };
    let a = v.abs();
    let d = a.floor();
    let m = ((a - d) * 60.0).floor();
    let s = ((a - d) * 60.0 - m) * 60.0;
    format!("{d:.0}°{m:02.0}′{s:04.1}″{h}")
}

/// Binary PPM (P6): needs no image crate; convert with ImageMagick if needed.
fn write_ppm(path: &str, img: &egui::ColorImage) -> Result<(), String> {
    let [w, h] = img.size;
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in &img.pixels {
        out.extend_from_slice(&[px.r(), px.g(), px.b()]);
    }
    std::fs::write(path, out).map_err(|e| e.to_string())
}
