//! Inventor's Inspect tools for a Forge document: Measure (a panel that reads what's picked,
//! drawn on the model as dimensions), the section views (half, quarter, three quarter, end: the
//! GPU cuts the picture, the file is untouched), the centre of gravity, and the iProperties
//! dialog, whose fields leave as SetProperty ops. The host fills in what it works out
//! (readings, physical properties, planes picked) and does the cutting.

use super::Shell;
use super::fg::FgAct;
use super::fgasm::{dialog_frame, dialog_head};
use super::theme::{Theme, medium, mono, sans};
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
                        self.fg.as_ref().and_then(|f| doc_dir(&f.file)),
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
                            egui::RichText::new("Working it out…")
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
                        ui.label(egui::RichText::new("Measuring…").font(sans(11.0)).color(t.ink2));
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

    #[allow(clippy::too_many_lines)]
    fn iprops_window(
        &mut self,
        ctx: &egui::Context,
        t: &Theme,
        v: Rect,
        places: &mut Vec<(String, Rect)>,
        ops: &mut Vec<Op>,
    ) -> Option<Rect> {
        let mut close = false;
        let file = self.fg.as_ref().map(|f| f.file.clone()).unwrap_or_default();
        let units = self.units;
        let i = &mut self.inspect;
        let assembly = i.assembly;
        let doc = i.doc.clone();
        let physical = i.physical.clone();
        let busy = i.physical_busy;
        let mut want = false;
        let mut browse = false;
        let dir = doc_dir(&file);
        let p = i.props.as_mut()?;
        let name = std::path::Path::new(&file)
            .file_name()
            .map_or("Untitled".to_string(), |n| n.to_string_lossy().into_owned());
        let title = format!("{name} iProperties");
        let r = Self::inspect_window(&title, "iprops", t, pos2(v.left() + 40.0, v.top() + 40.0), 460.0).show(
            ctx,
            |ui| {
                close = dialog_head(ui, t, "part", &title, "iprops:close", places);
                ui.horizontal_wrapped(|ui| {
                    for (k, name) in PROP_TABS.iter().enumerate() {
                        let r = ui.selectable_label(p.tab == k, egui::RichText::new(*name).font(sans(12.0)));
                        places.push((format!("iprops:tab:{name}"), r.rect));
                        if r.clicked() {
                            p.tab = k;
                            if k == 5 {
                                want = true;
                            }
                        }
                    }
                });
                ui.separator();
                let set = |key: &str, text: &str| -> Op {
                    Op::SetProperty {
                        key: key.into(),
                        text: (!text.is_empty()).then(|| forge_lang::props::quote(text)),
                    }
                };
                match p.tab {
                    0 => {
                        let meta = std::fs::metadata(&file).ok();
                        egui::Grid::new("iprops-general")
                            .num_columns(2)
                            .spacing(vec2(12.0, 6.0))
                            .show(ui, |ui| {
                                let row = |ui: &mut Ui, k: &str, val: String| {
                                    ui.label(egui::RichText::new(k).font(sans(12.0)).color(t.ink2));
                                    ui.label(egui::RichText::new(val).font(sans(12.0)).color(t.ink));
                                    ui.end_row();
                                };
                                row(ui, "Name", name.clone());
                                row(
                                    ui,
                                    "Type",
                                    if assembly { "Forge Assembly" } else { "Forge Part" }.into(),
                                );
                                row(
                                    ui,
                                    "Location",
                                    std::path::Path::new(&file)
                                        .parent()
                                        .map_or(String::new(), |d| d.display().to_string()),
                                );
                                row(
                                    ui,
                                    "Size",
                                    meta.as_ref().map_or("—".into(), |m| format!("{} bytes", m.len())),
                                );
                            });
                    }
                    1..=3 => {
                        let tab = [Tab::Summary, Tab::Project, Tab::Status][p.tab - 1];
                        egui::Grid::new(("iprops-fields", p.tab))
                            .num_columns(2)
                            .spacing(vec2(12.0, 5.0))
                            .show(ui, |ui| {
                                for (key, label, _) in FIELDS.iter().filter(|f| f.2 == tab) {
                                    ui.label(egui::RichText::new(*label).font(sans(12.0)).color(t.ink2));
                                    let now = doc.get(key).unwrap_or("").to_string();
                                    if *key == "design_state" {
                                        let mut pick = now.clone();
                                        let r = egui::ComboBox::from_id_salt("iprops-state")
                                            .selected_text(if now.is_empty() { "—" } else { now.as_str() })
                                            .show_ui(ui, |ui| {
                                                for s in STATES {
                                                    ui.selectable_value(&mut pick, s.to_string(), s);
                                                }
                                            });
                                        places.push(("iprops:design_state".into(), r.response.rect));
                                        if pick != now {
                                            ops.push(set(key, &pick));
                                        }
                                    } else {
                                        // a value sent waits for the build that has it
                                        if p.pending.get(*key) == Some(&now) {
                                            p.pending.remove(*key);
                                        }
                                        let was = p.pending.get(*key).cloned().unwrap_or_else(|| now.clone());
                                        let text =
                                            p.typing.entry(key.to_string()).or_insert_with(|| was.clone());
                                        let r = ui.add(
                                            egui::TextEdit::singleline(text)
                                                .desired_width(280.0)
                                                .font(sans(12.0))
                                                .hint_text(if key.ends_with("_date") {
                                                    "YYYY-MM-DD"
                                                } else {
                                                    ""
                                                }),
                                        );
                                        places.push((format!("iprops:{key}"), r.rect));
                                        if r.lost_focus() && *text != was {
                                            ops.push(
                                                if *key == "estimated_cost"
                                                    && text.trim().parse::<f64>().is_ok()
                                                {
                                                    Op::SetProperty {
                                                        key: key.to_string(),
                                                        text: Some(text.trim().to_string()),
                                                    }
                                                } else {
                                                    set(key, text)
                                                },
                                            );
                                            p.pending.insert(key.to_string(), text.clone());
                                        } else if !r.has_focus() {
                                            *text = was;
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                    }
                    4 => {
                        let mut rows = doc.custom.clone();
                        let mut changed = false;
                        egui::Grid::new("iprops-custom")
                            .num_columns(3)
                            .spacing(vec2(12.0, 5.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Name").font(medium(12.0)).color(t.ink));
                                ui.label(egui::RichText::new("Value").font(medium(12.0)).color(t.ink));
                                ui.end_row();
                                let mut gone = None;
                                for (k, (n, val)) in rows.iter().enumerate() {
                                    ui.label(egui::RichText::new(n).font(sans(12.0)).color(t.ink2));
                                    ui.label(egui::RichText::new(val).font(sans(12.0)).color(t.ink));
                                    let r = ui.small_button("Delete");
                                    places.push((format!("iprops:custom:delete:{n}"), r.rect));
                                    if r.clicked() {
                                        gone = Some(k);
                                    }
                                    ui.end_row();
                                }
                                if let Some(k) = gone {
                                    rows.remove(k);
                                    changed = true;
                                }
                                let r = ui.add(
                                    egui::TextEdit::singleline(&mut p.new_custom.0)
                                        .desired_width(130.0)
                                        .hint_text("Name"),
                                );
                                places.push(("iprops:custom:name".into(), r.rect));
                                let r = ui.add(
                                    egui::TextEdit::singleline(&mut p.new_custom.1)
                                        .desired_width(170.0)
                                        .hint_text("Value"),
                                );
                                places.push(("iprops:custom:value".into(), r.rect));
                                let r = ui.button("Add");
                                places.push(("iprops:custom:add".into(), r.rect));
                                if r.clicked() && !p.new_custom.0.trim().is_empty() {
                                    let (n, val) = std::mem::take(&mut p.new_custom);
                                    rows.retain(|x| x.0 != n.trim());
                                    rows.push((n.trim().to_string(), val));
                                    changed = true;
                                }
                                ui.end_row();
                            });
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
                    _ => {
                        let (u, b) = Self::physical_tab(
                            ui,
                            t,
                            (assembly, dir.as_deref()),
                            &doc,
                            physical.as_ref(),
                            busy,
                            units,
                            places,
                            ops,
                        );
                        want |= u;
                        browse |= b;
                    }
                }
            },
        );
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

    /// The Physical tab; whether Update was pressed, and Browse.
    #[allow(clippy::too_many_arguments)]
    fn physical_tab(
        ui: &mut Ui,
        t: &Theme,
        (assembly, dir): (bool, Option<&std::path::Path>),
        doc: &Props,
        physical: Option<&Result<Physical, String>>,
        busy: bool,
        units: &str,
        places: &mut Vec<(String, Rect)>,
        ops: &mut Vec<Op>,
    ) -> (bool, bool) {
        let (mut update, mut browse) = (false, false);
        let libs = forge_material::Libraries::cached(dir);
        if !assembly {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Material").font(sans(12.0)).color(t.ink2));
                let now = doc.material.clone().unwrap_or_default();
                let mut pick = now.clone();
                let r = egui::ComboBox::from_id_salt("iprops-material")
                    .selected_text(if now.is_empty() { "(none)" } else { now.as_str() })
                    .width(220.0)
                    .height(420.0)
                    .show_ui(ui, |ui| {
                        let mut family = String::new();
                        for (set, m) in libs.all() {
                            let head = if set.lib == forge_material::Lib::ForgeCad {
                                m.family.clone()
                            } else {
                                set.lib.label().to_string()
                            };
                            if head != family {
                                ui.label(egui::RichText::new(&head).font(medium(11.0)).color(t.ink2));
                                family = head;
                            }
                            ui.selectable_value(&mut pick, m.name.clone(), &m.name);
                        }
                    });
                places.push(("iprops:material".into(), r.response.rect));
                let r = ui.button(egui::RichText::new("Browse…").font(sans(12.0)));
                places.push(("iprops:browse".into(), r.rect));
                browse = r.clicked();
                if pick != now {
                    ops.push(Op::SetProperty {
                        key: "material".into(),
                        text: Some(forge_lang::props::quote(&pick)),
                    });
                }
            });
            if let Some((_, m)) = doc.material.as_deref().and_then(|n| libs.find(n)) {
                ui.label(
                    egui::RichText::new(format!(
                        "Density {:.3} g/cm³{}",
                        m.density / 1000.0,
                        m.note.as_ref().map_or(String::new(), |n| format!(" · {n}"))
                    ))
                    .font(sans(11.0))
                    .color(t.ink2),
                );
            }
        }
        ui.horizontal(|ui| {
            let r = ui.button(egui::RichText::new("Update").font(sans(12.0)));
            places.push(("iprops:update".into(), r.rect));
            update = r.clicked();
            if busy {
                ui.label(
                    egui::RichText::new("Working it out…")
                        .font(sans(11.0))
                        .color(t.ink2),
                );
            }
        });
        let Some(p) = physical else {
            return (update, browse);
        };
        let p = match p {
            Ok(p) => p,
            Err(e) => {
                ui.label(egui::RichText::new(e).font(sans(11.0)).color(t.error));
                return (update, browse);
            }
        };
        let k = per_unit(units);
        let mu = mass_unit(units);
        let km = per_unit(mu);
        let fmt = |x: f64| {
            if x.abs() >= 1e5 || (x != 0.0 && x.abs() < 1e-3) {
                format!("{x:.4e}")
            } else {
                format!("{x:.4}")
            }
        };
        let iu = format!("{mu} {units}²");
        let ik = km * k * k;
        let head = |ui: &mut Ui, text: &str| {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(text).font(medium(12.0)).color(t.ink));
        };
        // a section's rows: words, then values in columns
        let grid =
            |ui: &mut Ui, id: &str, rows: Vec<(String, Vec<String>)>, places: &mut Vec<(String, Rect)>| {
                let cols = rows.iter().map(|r| r.1.len()).max().unwrap_or(1) + 1;
                egui::Grid::new(id)
                    .num_columns(cols)
                    .spacing(vec2(14.0, 3.0))
                    .show(ui, |ui| {
                        for (what, vals) in rows {
                            ui.label(egui::RichText::new(&what).font(sans(12.0)).color(t.ink2));
                            for v in vals {
                                let r = ui.label(egui::RichText::new(v).font(mono(12.0)).color(t.ink));
                                places.push((format!("iprops:{what}"), r.rect));
                            }
                            ui.end_row();
                        }
                    });
            };
        egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
            head(ui, "General Properties");
            grid(
                ui,
                "phys-general",
                vec![
                    (
                        "Mass".into(),
                        vec![
                            p.mass
                                .map_or("— (no material)".into(), |m| format!("{} {mu}", fmt(m * km))),
                        ],
                    ),
                    ("Area".into(), vec![format!("{} {units}²", fmt(p.area * k * k))]),
                    (
                        "Volume".into(),
                        vec![format!("{} {units}³", fmt(p.volume * k * k * k))],
                    ),
                ],
                places,
            );
            head(ui, "Center of Gravity");
            grid(
                ui,
                "phys-cg",
                ["X", "Y", "Z"]
                    .iter()
                    .enumerate()
                    .map(|(j, a)| (a.to_string(), vec![format!("{} {units}", fmt(p.centroid[j] * k))]))
                    .collect(),
                places,
            );
            if let Some((pm, _)) = &p.principal {
                head(ui, &format!("Principal Moments ({iu})"));
                grid(
                    ui,
                    "phys-principal",
                    vec![("I1  I2  I3".into(), pm.iter().map(|x| fmt(x * ik)).collect())],
                    places,
                );
            }
            for (what, m) in [
                ("About Center of Gravity", p.inertia),
                ("Global, about the origin", p.origin_inertia),
            ] {
                let Some(m) = m else { continue };
                head(ui, &format!("{what} ({iu})"));
                grid(
                    ui,
                    what,
                    vec![
                        (
                            "Ixx  Iyy  Izz".into(),
                            (0..3).map(|j| fmt(m[j][j] * ik)).collect(),
                        ),
                        (
                            "Ixy  Iyz  Ixz".into(),
                            [(0, 1), (1, 2), (0, 2)]
                                .iter()
                                .map(|&(a, b)| fmt(m[a][b] * ik))
                                .collect(),
                        ),
                    ],
                    places,
                );
            }
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!(
                    "Accuracy: exact B-rep integration, relative error under {:.0e}",
                    p.rel_err.max(1e-15)
                ))
                .font(sans(11.0))
                .color(t.ink2),
            );
            if !p.missing.is_empty() {
                ui.label(
                    egui::RichText::new(format!(
                        "No material (volume only, no mass): {}{}",
                        p.missing.iter().take(6).cloned().collect::<Vec<_>>().join(", "),
                        if p.missing.len() > 6 {
                            format!(" and {} more", p.missing.len() - 6)
                        } else {
                            String::new()
                        }
                    ))
                    .font(sans(11.0))
                    .color(super::fg::WARN),
                );
            }
            if p.solids.len() > 1 && !assembly {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Bodies").font(medium(12.0)).color(t.ink));
                for s in &p.solids {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {} · {}",
                            s.id,
                            s.material.as_deref().unwrap_or("(none)"),
                            s.mass.map_or("—".into(), |m| format!("{} {mu}", fmt(m * km)))
                        ))
                        .font(sans(11.0))
                        .color(t.ink2),
                    );
                }
            }
        });
        (update, browse)
    }
}
