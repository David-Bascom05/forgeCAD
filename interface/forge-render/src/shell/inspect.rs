//! Inventor's Inspect tools for a Forge document: Measure (a panel that reads what's picked,
//! drawn on the model as dimensions), the section views (half, quarter, three quarter, end: the
//! GPU cuts the picture, the file is untouched), the centre of gravity, and the iProperties
//! dialog, whose fields leave as SetProperty ops. The host fills in what it works out
//! (readings, physical properties, planes picked) and does the cutting.

use super::Shell;
use super::fg::FgAct;
use super::fgasm::{dialog_frame, dialog_head};
use super::materials::{button_at, clipped, section, tab_strip};
use super::theme::{Theme, medium, mono, sans, semi};
use crate::render::Entity;
use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use forge_agent::measure::{Quantity, Reading, Shown};
use forge_lang::error::Op;
use forge_lang::props::{FIELDS, Physical, Props, STATES, Tab};
use forge_math::Point3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectCmd {
    Measure,
    Wall,
    Section,
    Properties,
    /// The iProperties dialog on its Physical tab.
    Mass,
    /// The centre of gravity's marker, on or off.
    Cg,
    /// The Material Browser.
    Materials,
}

/// The Measure panel: what's picked (placement, entity, and how it reads), what it measures.
#[derive(Clone, Debug, Default)]
pub struct MeasurePanel {
    pub picks: Vec<(u32, Entity)>,
    pub names: Vec<String>,
    pub reading: Option<Result<Reading, String>>,
    pub busy: bool,
    /// Decimal places shown.
    pub precision: usize,
    pub unit: &'static str,
    /// The thinnest wall, not picks: its thickness and where.
    pub wall: bool,
    pub wall_at: Option<Result<(f64, Point3), String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionKind {
    Half,
    Quarter,
    ThreeQuarter,
    End,
}

const KINDS: [(SectionKind, &str); 4] = [
    (SectionKind::Half, "Half"),
    (SectionKind::Quarter, "Quarter"),
    (SectionKind::ThreeQuarter, "Three Quarter"),
    (SectionKind::End, "End"),
];

/// A cutting plane: through a point, its normal pointing at what it cuts away.
#[derive(Clone, Debug, PartialEq)]
pub struct CutPlane {
    pub name: String,
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    /// Along the normal from the origin.
    pub offset: f64,
    pub flip: bool,
}

impl CutPlane {
    /// An origin plane: its normal along the positive axis, flipped where the part of the model
    /// nearer the opening view's eye is what goes.
    pub fn origin_plane(k: usize) -> CutPlane {
        let (name, normal, flip) = [
            ("XY Plane", [0.0, 0.0, 1.0], false),
            ("XZ Plane", [0.0, 1.0, 0.0], true),
            ("YZ Plane", [1.0, 0.0, 0.0], false),
        ][k];
        CutPlane {
            name: name.into(),
            origin: [0.0; 3],
            normal,
            offset: 0.0,
            flip,
        }
    }

    /// Its normal as it cuts (flipped or not), and a point on it.
    pub fn at(&self) -> ([f64; 3], [f64; 3]) {
        let s = if self.flip { -1.0 } else { 1.0 };
        let n = self.normal.map(|x| x * s);
        let p = [0, 1, 2].map(|k| self.origin[k] + self.normal[k] * self.offset);
        (n, p)
    }

    /// The renderer's form: cut away where a x + b y + c z + d > 0.
    pub fn equation(&self) -> [f32; 4] {
        let (n, p) = self.at();
        let d = -(n[0] * p[0] + n[1] * p[1] + n[2] * p[2]);
        [n[0] as f32, n[1] as f32, n[2] as f32, d as f32]
    }
}

/// A section view as it's shown.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub kind: SectionKind,
    pub planes: [CutPlane; 2],
}

/// The renderer's planes: the first, the second and whether either cuts, and the end view.
pub type Cut = ([f32; 4], Option<([f32; 4], bool)>, bool);

impl Section {
    pub fn renderer(&self) -> Cut {
        let a = self.planes[0].equation();
        match self.kind {
            SectionKind::Half => (a, None, false),
            SectionKind::End => (a, None, true),
            SectionKind::Quarter => (a, Some((self.planes[1].equation(), true)), false),
            SectionKind::ThreeQuarter => (a, Some((self.planes[1].equation(), false)), false),
        }
    }

    fn two(&self) -> bool {
        matches!(self.kind, SectionKind::Quarter | SectionKind::ThreeQuarter)
    }
}

/// The iProperties dialog: its tab, fields being typed, a custom row being added.
#[derive(Clone, Debug, Default)]
pub struct PropsPanel {
    pub tab: usize,
    pub typing: std::collections::HashMap<String, String>,
    /// Values sent as ops, shown till a build has them.
    pub pending: std::collections::HashMap<String, String>,
    pub new_custom: (String, String),
}

const PROP_TABS: [&str; 6] = ["General", "Summary", "Project", "Status", "Custom", "Physical"];
const IPROPS_W: f32 = 600.0;
const IPROPS_BODY: f32 = 560.0;
/// The iProperties rows: their words' column, a value column's width, a date field's.
const LABEL_W: f32 = 150.0;
const COL_W: f32 = 140.0;
const DATE_W: f32 = 130.0;
const TENSOR_W: f32 = 116.0;

#[derive(Default)]
pub struct Inspect {
    pub measure: Option<MeasurePanel>,
    /// The Section dialog is up; `cut` is what the view shows.
    pub section_open: bool,
    pub cut: Option<Section>,
    /// A plane (0 or 1) waits for a planar face picked in the view.
    pub pick_plane: Option<usize>,
    pub props: Option<PropsPanel>,
    /// The centre of gravity's marker is on.
    pub cg: bool,
    /// The open document's iProperties, its kind, and its physical properties as last worked out.
    pub doc: Props,
    pub assembly: bool,
    pub physical: Option<Result<Physical, String>>,
    pub physical_busy: bool,
    /// Other states of the part beside the modelled one (a formed part's simulated state, say):
    /// each a column of the Physical tab after Modeled, by its name.
    pub physical_more: Vec<(String, Physical)>,
    /// The host is asked to work the physical properties out (again).
    pub want_physical: bool,
    /// The model's middle and reach in the world, for drawing planes.
    pub bounds: ([f32; 3], f32),
    pub browser: Option<super::materials::Browser>,
}

/// The folder a document is in: where its project's materials are.
pub fn doc_dir(file: &str) -> Option<std::path::PathBuf> {
    std::fs::canonicalize(file)
        .ok()?
        .parent()
        .map(|d| d.to_path_buf())
}

/// A unit's factor from base units (mm, kg) and its words.
fn per_unit(unit: &str) -> f64 {
    match forge_doc::units::compound(unit) {
        Ok((_, n, m)) => m / n,
        Err(_) => 1.0,
    }
}

/// A value read in the panel's unit, at its precision, with the unit.
pub fn shown(q: Quantity, v: f64, unit: &str, places: usize) -> String {
    match q.power() {
        0 => format!("{v:.places$}°"),
        2 => format!("{:.places$} {unit}²", v * per_unit(unit).powi(2)),
        _ => format!("{:.places$} {unit}", v * per_unit(unit)),
    }
}

/// Mass, kg to the unit system: lbm for inch documents.
fn mass_unit(length: &str) -> &'static str {
    if matches!(length, "in" | "ft") { "lb" } else { "kg" }
}

impl Shell {
    pub fn inspect_command(&mut self, c: InspectCmd) {
        let i = &mut self.inspect;
        match c {
            InspectCmd::Measure | InspectCmd::Wall => {
                let wall = c == InspectCmd::Wall;
                let open = i.measure.as_ref().map(|m| m.wall);
                i.measure = if open == Some(wall) {
                    None
                } else {
                    Some(MeasurePanel {
                        precision: 3,
                        unit: self.units,
                        wall,
                        ..MeasurePanel::default()
                    })
                };
            }
            InspectCmd::Section => {
                i.section_open = !i.section_open;
                if i.section_open && i.cut.is_none() {
                    let c = i.bounds.0.map(f64::from);
                    let mut a = CutPlane::origin_plane(1);
                    a.offset = c[1];
                    let mut b = CutPlane::origin_plane(2);
                    b.offset = c[0];
                    i.cut = Some(Section {
                        kind: SectionKind::Half,
                        planes: [a, b],
                    });
                }
            }
            InspectCmd::Properties | InspectCmd::Mass => {
                let tab = if c == InspectCmd::Mass { 5 } else { 1 };
                i.props = match &i.props {
                    Some(p) if p.tab == tab => None,
                    _ => Some(PropsPanel {
                        tab,
                        ..PropsPanel::default()
                    }),
                };
                if tab == 5 {
                    i.want_physical = true;
                }
            }
            InspectCmd::Materials => {
                i.browser = match i.browser {
                    Some(_) => None,
                    None => Some(super::materials::Browser::open(
                        self.fg.as_ref().and_then(|f| doc_dir(&f.path)),
                        i.doc.material.clone(),
                    )),
                };
            }
            InspectCmd::Cg => {
                i.cg = !i.cg;
                if i.cg && i.physical.is_none() {
                    i.want_physical = true;
                }
            }
        }
    }

    /// The panels and dialogs, and what's drawn on the model for them.
    pub(super) fn inspect_ui(&mut self, ctx: &egui::Context) {
        if self.fg.is_none() {
            self.inspect.measure = None;
            self.inspect.props = None;
            self.inspect.section_open = false;
            self.inspect.cut = None;
            return;
        }
        self.inspect_marks(ctx);
        let mut places: Vec<(String, Rect)> = vec![];
        let mut blocks: Vec<Rect> = vec![];
        let mut ops: Vec<Op> = vec![];
        let t = self.theme;
        let v = self.view_rect;
        if self.inspect.measure.is_some() {
            blocks.extend(self.measure_window(ctx, &t, v, &mut places));
        }
        if self.inspect.section_open {
            blocks.extend(self.section_window(ctx, &t, v, &mut places));
        }
        if self.inspect.props.is_some() {
            blocks.extend(self.iprops_window(ctx, &t, v, &mut places, &mut ops));
        }
        self.materials_ui(ctx);
        self.view_blocks.extend(blocks);
        self.places.extend(places);
        if let Some(fg) = &mut self.fg {
            fg.out.extend(ops.into_iter().map(FgAct::Apply));
        }
    }

    fn inspect_window<'a>(title: &'a str, id: &str, t: &Theme, at: Pos2, w: f32) -> egui::Window<'a> {
        egui::Window::new(title)
            .id(egui::Id::unique(("inspect-window", id)))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fade_in(false)
            .default_pos(at)
            .min_width(w)
            .max_width(w)
            .frame(dialog_frame(t))
    }

    fn measure_window(
        &mut self,
        ctx: &egui::Context,
        t: &Theme,
        v: Rect,
        places: &mut Vec<(String, Rect)>,
    ) -> Option<Rect> {
        let mut close = false;
        let at = pos2(v.left() + 16.0, v.top() + 56.0);
        let m = self.inspect.measure.as_mut()?;
        let title = if m.wall { "Wall Thickness" } else { "Measure" };
        let r = Self::inspect_window(title, "measure", t, at, 300.0).show(ctx, |ui| {
            close = dialog_head(ui, t, "measure", title, "measure:close", places);
            if m.wall {
                ui.label(
                    egui::RichText::new("The thinnest wall of the part picked, or of the part")
                        .font(sans(11.0))
                        .color(t.ink2),
                );
            }
            for k in 0..if m.wall { 1 } else { 2 } {
                let name = m.names.get(k).cloned().unwrap_or_else(|| "Pick an entity".into());
                ui.horizontal(|ui| {
                    ui.add_sized(
                        vec2(80.0, 18.0),
                        egui::Label::new(
                            egui::RichText::new(format!("Selection {}", k + 1))
                                .font(medium(12.0))
                                .color(t.ink),
                        ),
                    );
                    let r = ui.label(egui::RichText::new(name).font(sans(11.0)).color(t.ink2));
                    places.push((format!("measure:pick{}", k + 1), r.rect));
                });
            }
            ui.add_space(4.0);
            let unit = m.unit;
            let places_n = m.precision;
            if m.wall {
                match &m.wall_at {
                    Some(Ok((w, p))) => {
                        Self::result_row(
                            ui,
                            t,
                            "Thinnest wall",
                            shown(Quantity::Length, *w, unit, places_n),
                            places,
                        );
                        Self::result_row(ui, t, "At", format!("{:.1}, {:.1}, {:.1}", p.x, p.y, p.z), places);
                    }
                    Some(Err(e)) => {
                        ui.label(egui::RichText::new(e).font(sans(11.0)).color(t.error));
                    }
                    None => {
                        ui.label(
                            egui::RichText::new("Working it out...")
                                .font(sans(11.0))
                                .color(t.ink2),
                        );
                    }
                }
            } else {
                match &m.reading {
                    Some(Ok(r)) => {
                        for (q, x) in &r.values {
                            Self::result_row(ui, t, q.label(), shown(*q, *x, unit, places_n), places);
                        }
                    }
                    Some(Err(e)) => {
                        ui.label(egui::RichText::new(e).font(sans(11.0)).color(t.error));
                    }
                    None if m.busy => {
                        ui.label(egui::RichText::new("Measuring...").font(sans(11.0)).color(t.ink2));
                    }
                    None => {}
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Precision").font(sans(11.0)).color(t.ink2));
                let r = egui::ComboBox::from_id_salt("measure-precision")
                    .selected_text(format!("{}", m.precision))
                    .width(44.0)
                    .show_ui(ui, |ui| {
                        for p in 0..=6 {
                            ui.selectable_value(&mut m.precision, p, format!("{p}"));
                        }
                    });
                places.push(("measure:precision".into(), r.response.rect));
                ui.label(egui::RichText::new("Units").font(sans(11.0)).color(t.ink2));
                let r = egui::ComboBox::from_id_salt("measure-unit")
                    .selected_text(m.unit)
                    .width(44.0)
                    .show_ui(ui, |ui| {
                        for u in forge_doc::units::DOC_LENGTHS {
                            ui.selectable_value(&mut m.unit, u, u);
                        }
                    });
                places.push(("measure:unit".into(), r.response.rect));
                if !m.wall {
                    let r = ui.button(egui::RichText::new("Restart").font(sans(12.0)));
                    places.push(("measure:restart".into(), r.rect));
                    if r.clicked() {
                        m.picks.clear();
                        m.names.clear();
                        m.reading = None;
                    }
                }
            });
        });
        if close {
            self.inspect.measure = None;
        }
        r.map(|r| r.response.rect)
    }

    /// A result: its words, its value, and a button that copies the value.
    fn result_row(ui: &mut Ui, t: &Theme, what: &str, value: String, places: &mut Vec<(String, Rect)>) {
        ui.horizontal(|ui| {
            ui.add_sized(
                vec2(118.0, 18.0),
                egui::Label::new(egui::RichText::new(what).font(sans(12.0)).color(t.ink2)),
            );
            let r = ui.label(egui::RichText::new(&value).font(mono(12.0)).color(t.ink));
            places.push((format!("measure:value:{what}"), r.rect));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let c = ui
                    .add(egui::Button::new(egui::RichText::new("Copy").font(sans(10.0))).small())
                    .on_hover_text("Copy the value");
                places.push((format!("measure:copy:{what}"), c.rect));
                if c.clicked() {
                    ui.ctx().copy_text(value.clone());
                }
            });
        });
    }

    fn section_window(
        &mut self,
        ctx: &egui::Context,
        t: &Theme,
        v: Rect,
        places: &mut Vec<(String, Rect)>,
    ) -> Option<Rect> {
        let (mut close, mut end) = (false, false);
        let i = &mut self.inspect;
        let reach = i.bounds.1.max(1.0) as f64;
        let mid = i.bounds.0.map(f64::from);
        let mut pick = i.pick_plane;
        let s = i.cut.as_mut()?;
        let r = Self::inspect_window(
            "Section View",
            "section",
            t,
            pos2(v.left() + 16.0, v.top() + 56.0),
            320.0,
        )
        .show(ctx, |ui| {
            close = dialog_head(ui, t, "section", "Section View", "section:close", places);
            ui.horizontal_wrapped(|ui| {
                for (k, name) in KINDS {
                    let r = ui.selectable_label(s.kind == k, egui::RichText::new(name).font(sans(12.0)));
                    places.push((format!("section:{name}"), r.rect));
                    if r.clicked() {
                        s.kind = k;
                    }
                }
            });
            let n = if s.two() { 2 } else { 1 };
            for k in 0..n {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("Plane {}", k + 1))
                        .font(medium(12.0))
                        .color(t.ink),
                );
                let p = &mut s.planes[k];
                ui.horizontal(|ui| {
                    let r = egui::ComboBox::from_id_salt(("section-plane", k))
                        .selected_text(&p.name)
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for o in 0..3 {
                                let c = CutPlane::origin_plane(o);
                                if ui.selectable_label(p.name == c.name, &c.name).clicked() {
                                    // the same place along the new normal: the model's middle
                                    let off = (0..3).map(|j| mid[j] * c.normal[j]).sum::<f64>();
                                    *p = CutPlane { offset: off, ..c };
                                }
                            }
                        });
                    places.push((format!("section:plane{}", k + 1), r.response.rect));
                    let r = ui
                        .selectable_label(pick == Some(k), egui::RichText::new("Pick face").font(sans(12.0)));
                    places.push((format!("section:pick{}", k + 1), r.rect));
                    if r.clicked() {
                        pick = if pick == Some(k) { None } else { Some(k) };
                    }
                    let r = ui.checkbox(&mut p.flip, egui::RichText::new("Flip").font(sans(12.0)));
                    places.push((format!("section:flip{}", k + 1), r.rect));
                });
                let base = (0..3).map(|j| mid[j] * p.normal[j]).sum::<f64>()
                    - (0..3).map(|j| p.origin[j] * p.normal[j]).sum::<f64>();
                // the Drive dialog's slider: a visible track, its value, its range under it
                let (lo, hi) = (base - reach, base + reach);
                ui.label(egui::RichText::new("Offset").font(sans(12.0)).color(t.ink2));
                ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(120.0);
                ui.visuals_mut().widgets.inactive.bg_fill = t.ink2.gamma_multiply(0.35);
                let r = ui.add(
                    egui::Slider::new(&mut p.offset, lo..=hi)
                        .trailing_fill(true)
                        .suffix(" mm")
                        .max_decimals(2),
                );
                ui.horizontal(|ui| {
                    ui.set_width(ui.spacing().slider_width);
                    ui.label(
                        egui::RichText::new(format!("{lo:.0} mm"))
                            .font(sans(10.0))
                            .color(t.ink2),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{hi:.0} mm"))
                                .font(sans(10.0))
                                .color(t.ink2),
                        );
                    });
                });
                places.push((format!("section:offset{}", k + 1), r.rect));
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let r = ui.button(egui::RichText::new("OK").font(sans(12.0)));
                places.push(("section:ok".into(), r.rect));
                close |= r.clicked();
                let r = ui.button(egui::RichText::new("End Section View").font(sans(12.0)));
                places.push(("section:end".into(), r.rect));
                end = r.clicked();
            });
        });
        i.pick_plane = pick;
        if end {
            i.cut = None;
            close = true;
        }
        if close {
            i.section_open = false;
            i.pick_plane = None;
        }
        r.map(|r| r.response.rect)
    }

    /// What the tools draw on the model: the measured geometry as dimensions, the cutting planes
    /// and their handles while the Section dialog is up, the centre of gravity.
    fn inspect_marks(&mut self, ctx: &egui::Context) {
        let Some((camera, size)) = self.camera else { return };
        let v = self.view_rect;
        let ppp = ctx.pixels_per_point();
        let to_screen = |p: [f64; 3]| {
            camera
                .pixel_of([p[0] as f32, p[1] as f32, p[2] as f32], size.0, size.1)
                .map(|q| v.min + vec2(q.0, q.1) / ppp)
        };
        let t = self.theme;
        let painter = ctx
            .layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                egui::Id::unique("inspect-marks"),
            ))
            .with_clip_rect(v);
        let ink = Color32::from_rgb(0xFF, 0xD0, 0x40);
        let font = mono(12.0);
        let label = |at: Pos2, text: String| {
            let g = painter.layout_no_wrap(text, font.clone(), ink);
            let r = Rect::from_center_size(at, g.size() + vec2(8.0, 4.0));
            painter.rect_filled(r, 3.0, Color32::from_black_alpha(190));
            painter.galley(r.min + vec2(4.0, 2.0), g, ink);
        };
        let w = |p: Point3| [p.x, p.y, p.z];
        if let Some(m) = &self.inspect.measure {
            let (unit, n) = (m.unit, m.precision);
            if let Some(Ok(r)) = &m.reading {
                for s in &r.shown {
                    match *s {
                        Shown::Line(a, b, q) => {
                            let (Some(pa), Some(pb)) = (to_screen(w(a)), to_screen(w(b))) else {
                                continue;
                            };
                            let value = r.get(q).unwrap_or(a.distance(b));
                            painter.line_segment([pa, pb], Stroke::new(1.5, ink));
                            for e in [pa, pb] {
                                painter.circle_filled(e, 3.0, ink);
                            }
                            // the minimum below its line, the rest above: lines in line keep apart
                            let off = if q == Quantity::Distance { 12.0 } else { -12.0 };
                            label(pa + (pb - pa) * 0.5 + vec2(0.0, off), shown(q, value, unit, n));
                        }
                        Shown::Point(p) => {
                            if let Some(q) = to_screen(w(p)) {
                                painter.circle_stroke(q, 5.0, Stroke::new(1.5, ink));
                                painter.line_segment(
                                    [q - vec2(8.0, 0.0), q + vec2(8.0, 0.0)],
                                    Stroke::new(1.0, ink),
                                );
                                painter.line_segment(
                                    [q - vec2(0.0, 8.0), q + vec2(0.0, 8.0)],
                                    Stroke::new(1.0, ink),
                                );
                            }
                        }
                        Shown::Circle(c, z, rad) => {
                            let z = z.normalize().map_or(forge_math::Vec3::Z, |u| u.get());
                            let x = if z.x.abs() < 0.9 {
                                forge_math::Vec3::X
                            } else {
                                forge_math::Vec3::Y
                            };
                            let u = z.cross(x).normalize().map_or(x, |u| u.get());
                            let vv = z.cross(u);
                            let pts: Vec<Pos2> = (0..=48)
                                .filter_map(|k| {
                                    let a = k as f64 / 48.0 * std::f64::consts::TAU;
                                    to_screen(w(c
                                        + (u * forge_math::dmath::cos(a) + vv * forge_math::dmath::sin(a))
                                            * rad))
                                })
                                .collect();
                            painter.add(egui::Shape::line(pts, Stroke::new(1.5, ink)));
                            if let Some(q) = to_screen(w(c)) {
                                painter.circle_filled(q, 2.5, ink);
                                label(
                                    q + vec2(0.0, -14.0),
                                    format!("Ø {}", shown(Quantity::Diameter, 2.0 * rad, unit, n)),
                                );
                            }
                        }
                    }
                }
            }
            if let Some(Ok((thick, p))) = &m.wall_at
                && m.wall
                && let Some(q) = to_screen(w(*p))
            {
                painter.circle_stroke(q, 6.0, Stroke::new(2.0, ink));
                label(q + vec2(0.0, -16.0), shown(Quantity::Length, *thick, unit, n));
            }
        }
        if self.inspect.section_open
            && let Some(s) = &self.inspect.cut
        {
            let (mid, reach) = (
                self.inspect.bounds.0.map(f64::from),
                self.inspect.bounds.1 as f64 * 0.8,
            );
            let planes = if s.two() { 2 } else { 1 };
            let mut moved: Vec<(usize, f64)> = vec![];
            for (k, p) in s.planes.iter().take(planes).enumerate() {
                let (n, o) = p.at();
                // the model's middle, put on the plane
                let d = (0..3).map(|j| (mid[j] - o[j]) * n[j]).sum::<f64>();
                let c = [0, 1, 2].map(|j| mid[j] - n[j] * d);
                let x = if n[0].abs() < 0.9 {
                    [1.0, 0.0, 0.0]
                } else {
                    [0.0, 1.0, 0.0]
                };
                let cross = |a: [f64; 3], b: [f64; 3]| {
                    [
                        a[1] * b[2] - a[2] * b[1],
                        a[2] * b[0] - a[0] * b[2],
                        a[0] * b[1] - a[1] * b[0],
                    ]
                };
                let unit = |a: [f64; 3]| {
                    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
                    a.map(|v| v / l)
                };
                let u = unit(cross(n, x));
                let vv = cross(n, u);
                let corner =
                    |a: f64, b: f64| to_screen([0, 1, 2].map(|j| c[j] + (u[j] * a + vv[j] * b) * reach));
                let quad: Option<Vec<Pos2>> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .iter()
                    .map(|&(a, b)| corner(a, b))
                    .collect();
                let colour = if k == 0 {
                    t.focus
                } else {
                    Color32::from_rgb(0x40, 0xA0, 0xE0)
                };
                if let Some(q) = quad {
                    painter.add(egui::Shape::convex_polygon(
                        q.clone(),
                        colour.gamma_multiply(0.07),
                        Stroke::new(1.0, colour.gamma_multiply(0.8)),
                    ));
                }
                // the handle: an arrow along the normal from the plane's middle, dragged to move it
                let (Some(from), Some(one)) = (to_screen(c), to_screen([0, 1, 2].map(|j| c[j] + n[j])))
                else {
                    continue;
                };
                let per_mm = (one - from).length().max(1e-6);
                let dir = (one - from) / per_mm;
                let tip = from + dir * 60.0;
                painter.arrow(from, tip - from, Stroke::new(2.5, colour));
                let hit = Rect::from_center_size(tip, vec2(22.0, 22.0));
                let id = egui::Id::unique(("section-handle", k));
                let r = egui::Area::new(id)
                    .fixed_pos(hit.min)
                    .order(egui::Order::Middle)
                    .show(ctx, |ui| ui.allocate_response(hit.size(), Sense::drag()));
                let r = r.inner;
                painter.circle_filled(tip, if r.hovered() || r.dragged() { 8.0 } else { 6.0 }, colour);
                self.places.push((format!("section:handle{}", k + 1), hit));
                self.view_blocks.push(hit);
                if r.dragged() {
                    let along = r.drag_delta().dot(dir) / per_mm;
                    // the handle moves the cut as drawn: flipped, the normal's the other way
                    let s = if p.flip { -1.0 } else { 1.0 };
                    moved.push((k, along as f64 * s));
                }
            }
            if let Some(s) = &mut self.inspect.cut {
                for (k, d) in moved {
                    s.planes[k].offset += d;
                }
            }
        }
        if self.inspect.cg
            && let Some(Ok(p)) = &self.inspect.physical
            && let Some(q) = to_screen(p.centroid)
        {
            // the centre of gravity's symbol: a circle in quarters, two of them filled
            let r = 9.0;
            painter.circle_filled(q, r, Color32::WHITE);
            for a in [0.0f32, std::f32::consts::PI] {
                let pts: Vec<Pos2> = std::iter::once(q)
                    .chain((0..=8).map(|k| {
                        let ang = a + k as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
                        q + vec2(ang.cos(), -ang.sin()) * r
                    }))
                    .collect();
                painter.add(egui::Shape::convex_polygon(pts, Color32::BLACK, Stroke::NONE));
            }
            painter.circle_stroke(q, r, Stroke::new(1.5, Color32::BLACK));
            painter.text(
                q + vec2(r + 4.0, -r),
                Align2::LEFT_BOTTOM,
                "CG",
                sans(11.0),
                Color32::WHITE,
            );
        }
    }

    fn iprops_window(
        &mut self,
        ctx: &egui::Context,
        t: &Theme,
        v: Rect,
        places: &mut Vec<(String, Rect)>,
        ops: &mut Vec<Op>,
    ) -> Option<Rect> {
        let mut close = false;
        let file = self
            .fg
            .as_ref()
            .map(|f| {
                if f.path.is_empty() {
                    f.file.clone()
                } else {
                    f.path.clone()
                }
            })
            .unwrap_or_default();
        let units = self.units;
        let (contents, dirty) = self
            .fg
            .as_ref()
            .map_or((String::new(), false), |f| (contents(f), f.dirty));
        let i = &mut self.inspect;
        let assembly = i.assembly;
        let doc = i.doc.clone();
        let physical = i.physical.clone();
        let more = i.physical_more.clone();
        let busy = i.physical_busy;
        let mut want = false;
        let mut browse = false;
        let dir = doc_dir(&file);
        let p = i.props.as_mut()?;
        let name = std::path::Path::new(&file)
            .file_name()
            .map_or("Untitled".to_string(), |n| n.to_string_lossy().into_owned());
        let title = format!("{name} iProperties");
        let at = pos2(v.left() + 40.0, v.top() + 40.0);
        let r = Self::inspect_window(&title, "iprops", t, at, IPROPS_W).show(ctx, |ui| {
            let icon = if assembly { "subasm" } else { "part" };
            close = dialog_head(ui, t, icon, &title, "iprops:close", places);
            ui.add_space(6.0);
            if tab_strip(ui, t, &PROP_TABS, &mut p.tab, "iprops:tab", places) && p.tab == 5 {
                want = true;
            }
            egui::ScrollArea::vertical()
                .id_salt(("iprops-body", p.tab))
                .auto_shrink([false, false])
                .min_scrolled_height(IPROPS_BODY)
                .max_height(IPROPS_BODY)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    match p.tab {
                        0 => Self::general_tab(ui, t, &file, assembly, units, &contents, dirty),
                        1..=3 => Self::fields_tab(ui, t, p, &doc, &file, ops, places),
                        4 => Self::custom_tab(ui, t, p, &doc, ops, places),
                        _ => {
                            let mut states: Vec<(&str, &Physical)> = vec![];
                            let got = physical.as_ref().map(|r| r.as_ref());
                            if let Some(Ok(m)) = got {
                                states.push(("Modeled", m));
                                states.extend(more.iter().map(|(n, x)| (n.as_str(), x)));
                            }
                            let (u, b) = Self::physical_tab(
                                ui,
                                t,
                                (assembly, dir.as_deref()),
                                &doc,
                                (got, &states),
                                busy,
                                units,
                                places,
                                ops,
                            );
                            want |= u;
                            browse |= b;
                        }
                    }
                    ui.add_space(8.0);
                });
        });
        let i = &mut self.inspect;
        i.want_physical |= want;
        if browse && i.browser.is_none() {
            i.browser = Some(super::materials::Browser::open(dir, i.doc.material.clone()));
        }
        if close {
            i.props = None;
        }
        r.map(|r| r.response.rect)
    }

    /// A row of words and a value beside them; its rect.
    fn info_row(ui: &mut Ui, t: &Theme, words: &str, value: &str, font: egui::FontId, ink: Color32) -> Rect {
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
        ui.painter().text(
            pos2(r.left(), r.center().y),
            Align2::LEFT_CENTER,
            words,
            sans(12.5),
            t.ink2,
        );
        let v = Rect::from_min_max(
            pos2(r.left() + LABEL_W, r.top()),
            pos2(r.right() - 4.0, r.bottom()),
        );
        clipped(ui, v, value, font, ink, words);
        v
    }

    #[allow(clippy::too_many_arguments)]
    fn general_tab(
        ui: &mut Ui,
        t: &Theme,
        file: &str,
        assembly: bool,
        units: &str,
        contents: &str,
        dirty: bool,
    ) {
        let path = std::path::Path::new(file);
        let meta = std::fs::metadata(file).ok();
        ui.add_space(12.0);
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, 48.0), Sense::hover());
        let p = ui.painter();
        let tile = Rect::from_min_size(r.min, vec2(44.0, 44.0));
        p.rect_filled(tile, 6.0, t.raised);
        p.rect_stroke(tile, 6.0, Stroke::new(1.0, t.line), egui::StrokeKind::Inside);
        super::icons::draw(
            p,
            tile.min + vec2(10.0, 10.0),
            24.0,
            if assembly { "subasm" } else { "part" },
            1.5,
            t.ink,
        );
        let name = path
            .file_name()
            .map_or("Untitled".to_string(), |n| n.to_string_lossy().into_owned());
        p.text(
            pos2(tile.right() + 14.0, r.top() + 13.0),
            Align2::LEFT_CENTER,
            name,
            semi(16.0),
            t.ink,
        );
        p.text(
            pos2(tile.right() + 14.0, r.top() + 34.0),
            Align2::LEFT_CENTER,
            if assembly { "Forge assembly" } else { "Forge part" },
            sans(12.0),
            t.ink2,
        );
        section(ui, t, "File");
        let dir = doc_dir(file).map_or(String::new(), |d| {
            let d = d.display().to_string();
            match std::env::var("HOME") {
                Ok(h) if !h.is_empty() && d.starts_with(&h) => format!("~{}", &d[h.len()..]),
                _ => d,
            }
        });
        Self::info_row(ui, t, "Location", &dir, sans(12.5), t.ink);
        if let Some(m) = &meta {
            Self::info_row(ui, t, "Size", &size_words(m.len()), sans(12.5), t.ink);
            for (words, at) in [
                ("Created", m.created().ok()),
                ("Modified", m.modified().ok()),
                ("Accessed", m.accessed().ok()),
            ] {
                if let Some(at) = at {
                    Self::info_row(ui, t, words, &when(at), mono(12.0), t.ink);
                }
            }
        }
        section(ui, t, "Document");
        let mass = mass_unit(units);
        let unit_words = |u: &str| match u {
            "mm" => "millimetres",
            "cm" => "centimetres",
            "m" => "metres",
            "in" => "inches",
            "ft" => "feet",
            "kg" => "kilograms",
            "lb" => "pounds",
            _ => "",
        };
        Self::info_row(
            ui,
            t,
            "Units",
            &format!(
                "{} ({units}) · {} ({mass})",
                capitalised(unit_words(units)),
                unit_words(mass)
            ),
            sans(12.5),
            t.ink,
        );
        if !contents.is_empty() {
            Self::info_row(ui, t, "Contents", contents, sans(12.5), t.ink);
        }
        let (state, ink) = if dirty {
            ("Unsaved changes", super::materials::amber(t))
        } else {
            ("Saved", t.ink)
        };
        Self::info_row(ui, t, "State", state, sans(12.5), ink);
    }

    /// A property's text box: what's typed leaves as a SetProperty op when it loses focus.
    #[allow(clippy::too_many_arguments)]
    fn prop_field(
        ui: &mut Ui,
        p: &mut PropsPanel,
        doc: &Props,
        key: &str,
        r: Rect,
        hint: &str,
        ops: &mut Vec<Op>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let now = doc.get(key).unwrap_or("").to_string();
        // a value sent waits for the build that has it
        if p.pending.get(key) == Some(&now) {
            p.pending.remove(key);
        }
        let was = p.pending.get(key).cloned().unwrap_or_else(|| now.clone());
        let text = p.typing.entry(key.to_string()).or_insert_with(|| was.clone());
        let edit = if key == "comments" {
            egui::TextEdit::multiline(text).desired_rows(3)
        } else {
            egui::TextEdit::singleline(text)
        };
        let resp = ui.put(r, edit.font(sans(12.5)).margin(vec2(8.0, 5.0)).hint_text(hint));
        places.push((format!("iprops:{key}"), resp.rect));
        if resp.lost_focus() && *text != was {
            let v = text.trim();
            ops.push(Op::SetProperty {
                key: key.to_string(),
                text: if v.is_empty() {
                    None
                } else if key == "estimated_cost" && v.parse::<f64>().is_ok() {
                    Some(v.to_string())
                } else {
                    Some(forge_lang::props::quote(v))
                },
            });
            p.pending.insert(key.to_string(), text.clone());
        } else if !resp.has_focus() {
            *text = was;
        }
    }

    /// Summary, Project and Status: Inventor's fields, each written back as typed.
    fn fields_tab(
        ui: &mut Ui,
        t: &Theme,
        p: &mut PropsPanel,
        doc: &Props,
        file: &str,
        ops: &mut Vec<Op>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let stem = std::path::Path::new(file)
            .file_stem()
            .map_or(String::new(), |s| s.to_string_lossy().into_owned());
        let tab = [Tab::Summary, Tab::Project, Tab::Status][p.tab - 1];
        let label = |key: &str| FIELDS.iter().find(|f| f.0 == key).map_or("", |f| f.1);
        let hint = |key: &str| -> String {
            match key {
                "part_number" => stem.clone(),
                k if k.ends_with("_date") => "YYYY-MM-DD".into(),
                _ => String::new(),
            }
        };
        ui.add_space(10.0);
        if tab == Tab::Status {
            // what the part is, then its state and who signed it off when
            let number = doc.get("part_number").map_or(stem.clone(), str::to_string);
            Self::info_row(ui, t, "Part Number", &number, sans(12.5), t.ink);
            if let Some(d) = doc.get("description") {
                Self::info_row(ui, t, "Description", d, sans(12.5), t.ink);
            }
            section(ui, t, "Status");
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
            ui.painter().text(
                pos2(r.left(), r.center().y),
                Align2::LEFT_CENTER,
                "Design State",
                sans(12.5),
                t.ink2,
            );
            let now = doc.get("design_state").unwrap_or("").to_string();
            let mut pick = now.clone();
            let at = Rect::from_min_size(pos2(r.left() + LABEL_W, r.top() + 5.0), vec2(200.0, 26.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(at), |ui| {
                let c = egui::ComboBox::from_id_salt("iprops-state")
                    .selected_text(if now.is_empty() { "—" } else { state_words(&now) })
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for s in STATES {
                            ui.selectable_value(&mut pick, s.to_string(), state_words(s));
                        }
                    });
                places.push(("iprops:design_state".into(), c.response.rect));
            });
            if pick != now {
                ops.push(Op::SetProperty {
                    key: "design_state".into(),
                    text: Some(forge_lang::props::quote(&pick)),
                });
            }
            section(ui, t, "Sign-off");
            let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
            let by_x = r.left() + LABEL_W;
            let date_x = r.right() - 4.0 - DATE_W;
            for (x, s) in [(by_x, "BY"), (date_x, "DATE")] {
                ui.painter()
                    .text(pos2(x, r.center().y), Align2::LEFT_CENTER, s, semi(10.0), t.ink2);
            }
            for (words, by, date) in [
                ("Checked", "checked_by", "checked_date"),
                ("Eng. Approved", "eng_approved_by", "eng_approved_date"),
                ("Mfg. Approved", "mfg_approved_by", "mfg_approved_date"),
            ] {
                let (r, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
                ui.painter().text(
                    pos2(r.left(), r.center().y),
                    Align2::LEFT_CENTER,
                    words,
                    sans(12.5),
                    t.ink2,
                );
                let f = Rect::from_min_max(pos2(by_x, r.top() + 5.0), pos2(date_x - 10.0, r.bottom() - 5.0));
                Self::prop_field(ui, p, doc, by, f, "", ops, places);
                let f = Rect::from_min_max(
                    pos2(date_x, r.top() + 5.0),
                    pos2(r.right() - 4.0, r.bottom() - 5.0),
                );
                Self::prop_field(ui, p, doc, date, f, "YYYY-MM-DD", ops, places);
            }
            return;
        }
        for (key, _, _) in FIELDS.iter().filter(|f| f.2 == tab) {
            let w = ui.available_width();
            let h = if *key == "comments" { 76.0 } else { 36.0 };
            let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
            ui.painter().text(
                pos2(r.left(), r.top() + 18.0),
                Align2::LEFT_CENTER,
                label(key),
                sans(12.5),
                t.ink2,
            );
            let f = Rect::from_min_max(
                pos2(r.left() + LABEL_W, r.top() + 5.0),
                pos2(r.right() - 4.0, r.bottom() - 5.0),
            );
            Self::prop_field(ui, p, doc, key, f, &hint(key), ops, places);
        }
    }

    fn custom_tab(
        ui: &mut Ui,
        t: &Theme,
        p: &mut PropsPanel,
        doc: &Props,
        ops: &mut Vec<Op>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let mut rows = doc.custom.clone();
        let mut changed = false;
        ui.add_space(10.0);
        let w = ui.available_width();
        let name_w = 180.0;
        let (h, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
        for (x, s) in [(h.left() + 8.0, "NAME"), (h.left() + name_w + 12.0, "VALUE")] {
            ui.painter()
                .text(pos2(x, h.center().y), Align2::LEFT_CENTER, s, semi(10.0), t.ink2);
        }
        ui.painter()
            .hline(h.x_range(), h.bottom() - 0.5, Stroke::new(1.0, t.line));
        let mut gone = None;
        for (k, (n, val)) in rows.clone().iter().enumerate() {
            let (r, _) = ui.allocate_exact_size(vec2(w, 38.0), Sense::hover());
            if k % 2 == 1 {
                ui.painter().rect_filled(r, 0.0, t.raised);
            }
            clipped(
                ui,
                Rect::from_min_max(pos2(r.left() + 8.0, r.top()), pos2(r.left() + name_w, r.bottom())),
                n,
                medium(12.5),
                t.ink,
                n,
            );
            let key = format!("custom:{n}");
            let text = p.typing.entry(key.clone()).or_insert_with(|| val.clone());
            let f = Rect::from_min_max(
                pos2(r.left() + name_w + 4.0, r.top() + 6.0),
                pos2(r.right() - 40.0, r.bottom() - 6.0),
            );
            let resp = ui.put(
                f,
                egui::TextEdit::singleline(text)
                    .font(sans(12.5))
                    .margin(vec2(8.0, 5.0)),
            );
            places.push((format!("iprops:custom:value:{n}"), resp.rect));
            if resp.lost_focus() && *text != *val {
                rows[k].1 = text.clone();
                changed = true;
            } else if !resp.has_focus() {
                *text = val.clone();
            }
            let x = Rect::from_center_size(pos2(r.right() - 18.0, r.center().y), vec2(22.0, 22.0));
            let resp = ui.interact(x, ui.scope_id().with(("custom-delete", k)), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(x, 4.0, t.hover);
            }
            super::icons::draw(
                ui.painter(),
                x.min + vec2(3.0, 3.0),
                16.0,
                "delete",
                1.5,
                if resp.hovered() { t.error } else { t.ink2 },
            );
            let resp = resp.on_hover_text(format!("Delete {n}"));
            places.push((format!("iprops:custom:delete:{n}"), x));
            if resp.clicked() {
                gone = Some(k);
            }
        }
        if let Some(k) = gone {
            p.typing.remove(&format!("custom:{}", rows[k].0));
            rows.remove(k);
            changed = true;
        }
        ui.add_space(10.0);
        let (r, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
        let f = Rect::from_min_max(
            pos2(r.left(), r.top() + 4.0),
            pos2(r.left() + name_w, r.bottom() - 4.0),
        );
        let resp = ui.put(
            f,
            egui::TextEdit::singleline(&mut p.new_custom.0)
                .font(sans(12.5))
                .margin(vec2(8.0, 5.0))
                .hint_text("Name"),
        );
        places.push(("iprops:custom:name".into(), resp.rect));
        let f = Rect::from_min_max(
            pos2(r.left() + name_w + 4.0, r.top() + 4.0),
            pos2(r.right() - 84.0, r.bottom() - 4.0),
        );
        let resp = ui.put(
            f,
            egui::TextEdit::singleline(&mut p.new_custom.1)
                .font(sans(12.5))
                .margin(vec2(8.0, 5.0))
                .hint_text("Value"),
        );
        places.push(("iprops:custom:value".into(), resp.rect));
        let b = Rect::from_min_max(
            pos2(r.right() - 76.0, r.top() + 2.0),
            pos2(r.right() - 4.0, r.bottom() - 2.0),
        );
        let resp = button_at(ui, t, b, "Add", false, !p.new_custom.0.trim().is_empty());
        places.push(("iprops:custom:add".into(), resp.rect));
        if resp.clicked() {
            let (n, val) = std::mem::take(&mut p.new_custom);
            rows.retain(|x| x.0 != n.trim());
            rows.push((n.trim().to_string(), val));
            changed = true;
        }
        if changed {
            let text = rows
                .iter()
                .map(|(n, val)| {
                    format!(
                        "({}, {})",
                        forge_lang::props::quote(n),
                        forge_lang::props::quote(val)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            ops.push(Op::SetProperty {
                key: "custom".into(),
                text: (!rows.is_empty()).then(|| format!("[{text}]")),
            });
        }
    }

    /// The Physical tab: the material, then each state's properties in a column of its own
    /// (Modeled, and whatever else the part gives); whether Update was pressed, and Browse.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn physical_tab(
        ui: &mut Ui,
        t: &Theme,
        (assembly, dir): (bool, Option<&std::path::Path>),
        doc: &Props,
        (got, states): States,
        busy: bool,
        units: &str,
        places: &mut Vec<(String, Rect)>,
        ops: &mut Vec<Op>,
    ) -> (bool, bool) {
        let mut browse = false;
        let libs = forge_material::Libraries::cached(dir);
        let w = ui.available_width();
        ui.add_space(10.0);
        let material = doc
            .material
            .as_deref()
            .and_then(|n| libs.find(n))
            .map(|x| x.1.clone());
        if !assembly {
            let (r, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
            ui.painter().text(
                pos2(r.left(), r.center().y),
                Align2::LEFT_CENTER,
                "Material",
                sans(12.5),
                t.ink2,
            );
            let now = doc.material.clone().unwrap_or_default();
            let mut pick = now.clone();
            let at = Rect::from_min_size(pos2(r.left() + LABEL_W, r.top() + 5.0), vec2(260.0, 26.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(at), |ui| {
                let c = egui::ComboBox::from_id_salt("iprops-material")
                    .selected_text(if now.is_empty() { "None" } else { now.as_str() })
                    .width(at.width())
                    .height(420.0)
                    .show_ui(ui, |ui| {
                        let mut family = String::new();
                        for (set, m) in libs.all() {
                            let head = if set.lib == forge_material::Lib::ForgeCad {
                                super::materials::capital(&m.family)
                            } else {
                                set.lib.label().to_string()
                            };
                            if head != family {
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new(&head).font(semi(11.0)).color(t.ink2));
                                family = head;
                            }
                            ui.selectable_value(&mut pick, m.name.clone(), &m.name);
                        }
                    });
                places.push(("iprops:material".into(), c.response.rect));
            });
            let b = Rect::from_min_size(pos2(at.right() + 8.0, at.top() - 1.0), vec2(96.0, 28.0));
            let r = button_at(ui, t, b, "Browse…", false, true);
            places.push(("iprops:browse".into(), r.rect));
            browse = r.clicked();
            if pick != now {
                ops.push(Op::SetProperty {
                    key: "material".into(),
                    text: Some(forge_lang::props::quote(&pick)),
                });
            }
        }
        let (r, _) = ui.allocate_exact_size(vec2(w, 38.0), Sense::hover());
        let b = Rect::from_min_size(pos2(r.left(), r.top() + 5.0), vec2(84.0, 28.0));
        let resp = button_at(ui, t, b, "Update", false, !busy);
        places.push(("iprops:update".into(), resp.rect));
        let update = resp.clicked();
        let note = match (busy, got) {
            (true, _) => "Working it out…".to_string(),
            (_, Some(Ok(p))) => format!(
                "Exact B-rep integration · relative error under {:.0e}",
                p.rel_err.max(1e-15)
            ),
            _ => String::new(),
        };
        ui.painter().text(
            pos2(b.right() + 12.0, b.center().y),
            Align2::LEFT_CENTER,
            note,
            sans(12.0),
            t.ink2,
        );
        if let Some(Err(e)) = got {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(e).font(sans(12.0)).color(t.error));
        }
        if states.is_empty() {
            return (update, browse);
        }
        let k = per_unit(units);
        let mu = mass_unit(units);
        let km = per_unit(mu);
        let ik = km * k * k;
        let iu = format!("{mu}·{units}²");
        let n = states.len();
        let col = |j: usize| LABEL_W + COL_W * (j + 1) as f32;
        let p0 = ui.painter().clone();
        if n > 1 {
            let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
            for (j, (name, _)) in states.iter().enumerate() {
                p0.text(
                    pos2(r.left() + col(j), r.bottom() - 6.0),
                    Align2::RIGHT_BOTTOM,
                    *name,
                    medium(12.0),
                    t.ink,
                );
            }
        }
        // a quantity's row: words, a value for each state, its unit
        let mut row = |ui: &mut Ui, words: &str, vals: Vec<Option<f64>>, unit: &str| {
            let (r, _) = ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
            let p = ui.painter();
            p.text(
                pos2(r.left(), r.center().y),
                Align2::LEFT_CENTER,
                words,
                sans(12.5),
                t.ink2,
            );
            for (j, v) in vals.iter().enumerate() {
                let at = pos2(r.left() + col(j), r.center().y);
                let (s, ink) = v.map_or(("—".to_string(), t.ink2), |x| (fig(x), t.ink));
                let g = p.layout_no_wrap(s, mono(12.5), ink);
                let cell = Rect::from_min_max(
                    at - vec2(g.size().x, g.size().y / 2.0),
                    at + vec2(0.0, g.size().y / 2.0),
                );
                p.galley(cell.min, g, ink);
                if j == 0 {
                    places.push((format!("iprops:{words}"), cell));
                }
            }
            p.text(
                pos2(r.left() + col(n - 1) + 10.0, r.center().y),
                Align2::LEFT_CENTER,
                unit,
                sans(12.0),
                t.ink2,
            );
        };
        let all = |f: &dyn Fn(&Physical) -> Option<f64>| states.iter().map(|s| f(s.1)).collect::<Vec<_>>();
        section(ui, t, "General");
        row(ui, "Mass", all(&|p| p.mass.map(|m| m * km)), mu);
        row(ui, "Area", all(&|p| Some(p.area * k * k)), &format!("{units}²"));
        row(
            ui,
            "Volume",
            all(&|p| Some(p.volume * k * k * k)),
            &format!("{units}³"),
        );
        if let Some(m) = &material {
            let d = all(&|p| p.mass.map(|_| m.density));
            row(ui, "Density", d, "kg/m³");
        }
        section(ui, t, "Center of gravity");
        for (j, a) in ["X", "Y", "Z"].iter().enumerate() {
            row(ui, a, all(&|p| Some(p.centroid[j] * k)), units);
        }
        if states.iter().any(|s| s.1.principal.is_some()) {
            section(ui, t, "Principal moments");
            for (j, a) in ["I1", "I2", "I3"].iter().enumerate() {
                row(ui, a, all(&|p| p.principal.map(|x| x.0[j] * ik)), &iu);
            }
        }
        for (what, of) in [
            ("Inertia about the center of gravity", 0),
            ("Inertia about the origin", 1),
        ] {
            let ms: Vec<(&str, [[f64; 3]; 3])> = states
                .iter()
                .filter_map(|(name, p)| {
                    if of == 0 { p.inertia } else { p.origin_inertia }.map(|m| (*name, m))
                })
                .collect();
            if ms.is_empty() {
                continue;
            }
            section(ui, t, what);
            for (name, m) in ms {
                let corner = if n > 1 {
                    format!("{name} · {iu}")
                } else {
                    String::new()
                };
                Self::tensor(ui, t, &corner, &m, ik, &iu, places, what);
            }
        }
        let p = states[0].1;
        if p.solids.len() > 1 && !assembly {
            section(ui, t, "Bodies");
            for s in &p.solids {
                let (r, _) = ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
                let pp = ui.painter();
                pp.text(
                    pos2(r.left(), r.center().y),
                    Align2::LEFT_CENTER,
                    &s.id,
                    sans(12.5),
                    t.ink,
                );
                pp.text(
                    pos2(r.left() + LABEL_W, r.center().y),
                    Align2::LEFT_CENTER,
                    s.material.as_deref().unwrap_or("None"),
                    sans(12.5),
                    if s.material.is_some() { t.ink } else { t.ink2 },
                );
                pp.text(
                    pos2(r.left() + col(0) + COL_W, r.center().y),
                    Align2::RIGHT_CENTER,
                    s.mass.map_or("—".into(), |m| format!("{} {mu}", fig(m * km))),
                    mono(12.5),
                    t.ink,
                );
            }
        }
        if !p.missing.is_empty() {
            ui.add_space(10.0);
            let list = p.missing.iter().take(6).cloned().collect::<Vec<_>>().join(", ");
            let more = if p.missing.len() > 6 {
                format!(" and {} more", p.missing.len() - 6)
            } else {
                String::new()
            };
            ui.label(
                egui::RichText::new(format!("No material, so no mass: {list}{more}"))
                    .font(sans(12.0))
                    .color(super::fg::WARN),
            );
        }
        (update, browse)
    }

    /// A symmetric 3 × 3 tensor as a table, the axes along its edges.
    #[allow(clippy::too_many_arguments)]
    fn tensor(
        ui: &mut Ui,
        t: &Theme,
        corner: &str,
        m: &[[f64; 3]; 3],
        k: f64,
        unit: &str,
        places: &mut Vec<(String, Rect)>,
        what: &str,
    ) {
        let w = ui.available_width();
        let cell = |j: usize| LABEL_W + TENSOR_W * (j + 1) as f32;
        let (r, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
        let p = ui.painter().clone();
        let (corner, ink) = if corner.is_empty() {
            (unit, t.ink2)
        } else {
            (corner, t.ink)
        };
        p.text(
            pos2(r.left(), r.center().y),
            Align2::LEFT_CENTER,
            corner,
            medium(12.0),
            ink,
        );
        for (j, a) in ["X", "Y", "Z"].iter().enumerate() {
            p.text(
                pos2(r.left() + cell(j), r.center().y),
                Align2::RIGHT_CENTER,
                *a,
                semi(10.5),
                t.ink2,
            );
        }
        for (a, axis) in ["X", "Y", "Z"].iter().enumerate() {
            let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
            p.text(
                pos2(r.left() + LABEL_W - 16.0, r.center().y),
                Align2::RIGHT_CENTER,
                *axis,
                semi(10.5),
                t.ink2,
            );
            for b in 0..3 {
                let ink = if a == b { t.ink } else { t.ink2 };
                let at = pos2(r.left() + cell(b), r.center().y);
                let g = p.layout_no_wrap(fig(m[a][b] * k), mono(12.5), ink);
                let c = Rect::from_min_max(
                    at - vec2(g.size().x, g.size().y / 2.0),
                    at + vec2(0.0, g.size().y / 2.0),
                );
                p.galley(c.min, g, ink);
                places.push((format!("iprops:{what}:{axis}{}", ["X", "Y", "Z"][b]), c));
            }
        }
        ui.add_space(4.0);
    }
}

/// The physical properties as worked out (or why not), and each state's, by name.
type States<'a> = (
    Option<Result<&'a Physical, &'a String>>,
    &'a [(&'a str, &'a Physical)],
);

/// What a document holds, in a line.
fn contents(f: &super::fg::FgPanel) -> String {
    use super::fg::plural;
    match &f.asm {
        Some(a) => format!(
            "{} · {}",
            plural(a.bom.len(), "part"),
            plural(a.occ.len(), "occurrence")
        ),
        None => format!(
            "{} · {}",
            plural(f.feats.len(), "feature"),
            plural(f.params.len(), "parameter")
        ),
    }
}

fn capitalised(s: &str) -> String {
    super::materials::capital(s)
}

/// Inventor's words for a design state.
fn state_words(s: &str) -> &str {
    match s {
        "WorkInProgress" => "Work in progress",
        "Pending" => "Pending",
        "Released" => "Released",
        x => x,
    }
}

/// A file's size: in KB or MB, and its bytes.
fn size_words(n: u64) -> String {
    let grouped = {
        let s = n.to_string();
        let mut out = String::new();
        for (k, c) in s.chars().enumerate() {
            if k > 0 && (s.len() - k).is_multiple_of(3) {
                out.push(',');
            }
            out.push(c);
        }
        out
    };
    match n {
        0..1024 => format!("{grouped} bytes"),
        1024..1_048_576 => format!("{:.1} KB ({grouped} bytes)", n as f64 / 1024.0),
        _ => format!("{:.1} MB ({grouped} bytes)", n as f64 / 1_048_576.0),
    }
}

/// A time, to the minute, in UTC.
fn when(at: std::time::SystemTime) -> String {
    let s = at
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let (days, rem) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    // days to a date (Hinnant's civil_from_days)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60
    )
}

/// A value to five figures or so, its thousands apart; very small or large ones in powers of ten.
fn fig(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let a = x.abs();
    if !(1e-4..1e12).contains(&a) {
        return format!("{x:.4e}");
    }
    let int = if a < 1.0 { 1 } else { a.log10().floor() as i32 + 1 };
    let places = (5 - int).clamp(0, 4) as usize;
    let s = format!("{a:.places$}");
    let (i, f) = s.split_once('.').map_or((s.as_str(), ""), |(i, f)| (i, f));
    let mut out = String::new();
    for (k, c) in i.chars().enumerate() {
        if k > 0 && (i.len() - k).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    if !f.is_empty() && f.bytes().any(|b| b != b'0') {
        out.push('.');
        out.push_str(f);
    }
    if x < 0.0 { format!("−{out}") } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_read_as_a_person_writes_them() {
        assert_eq!(fig(14_605_312.4), "14 605 312");
        assert_eq!(fig(71.764), "71.764");
        assert_eq!(fig(0.0096), "0.0096");
        assert_eq!(fig(-42_117.11), "−42 117");
        assert_eq!(fig(395.9859), "395.99");
        assert_eq!(fig(2700.0), "2 700");
        assert_eq!(size_words(2431), "2.4 KB (2,431 bytes)");
        let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_793_836_800 + 3600 * 9 + 60 * 12);
        assert_eq!(when(at), "2026-11-05 09:12 UTC");
    }
}
