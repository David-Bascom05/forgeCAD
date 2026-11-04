//! A drawing made from its document: the models built, every view drawn (exact hidden lines,
//! sections cut, details enlarged, flat patterns), every annotation measured from the model as
//! it is now, and each sheet laid out as a page. A reference that no longer resolves makes its
//! annotation sick: drawn in magenta, reported, never measured against something else.

use super::Model;
use super::doc::{
    self, AnnotDef, AnnotKind, At, DrawError, Item, ItemKind, Problem, Ref, SetStyle, ViewDef, ViewKind,
};
use super::ops::DrawOp;
use crate::syntax::Span;
use forge_draw::annot::{self, Style, number};
use forge_draw::geom2::Place;
use forge_draw::page::{Align, Page, Pen, Stroke, Text};
use forge_draw::section::{Cut, cut_body, cut_faces, face_loops};
use forge_draw::{Class, Geom2, Instance, Piece, Source, View};
use forge_geom::{Curve, CurveGeom, Surface};
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_topo::{Body, EdgeId, Name};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A built model and what its views and lists read of it.
pub struct ModelOut {
    pub id: String,
    pub file: String,
    pub model: Model,
    pub run: crate::Run,
    /// Bill of materials: part file, count, part number, description; in item order.
    pub bom: Vec<(String, usize, String, String)>,
}

/// A detail's boundary: centre, radius, and a rectangle's half sizes where it is one.
pub type Fence = (Point2, f64, Option<(f64, f64)>);

#[derive(Clone)]
pub struct ViewOut {
    pub id: String,
    pub sheet: usize,
    pub kind: &'static str,
    pub parent: Option<String>,
    pub model: usize,
    /// The orientation; a flat pattern's is the sheet's own plane (front).
    pub view: View,
    pub scale: f64,
    /// View (model mm on the view plane) to sheet.
    pub place: Place,
    /// The view's box on the sheet.
    pub lo: Point2,
    pub hi: Point2,
    pub pieces: Vec<Piece>,
    /// Hatching, bend lines and the like, in view coordinates.
    pub extra: Vec<Stroke>,
    pub extra_text: Vec<Text>,
    /// The solids drawn: body, pose and occurrence index in the model.
    pub insts: Vec<(Arc<Body>, Frame, usize)>,
    pub label: String,
    /// Where the view clips its parent's lines (a detail's circle), in view coordinates.
    pub clip: Option<Fence>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemOut {
    pub id: String,
    pub sheet: usize,
    pub kind: &'static str,
    pub lo: Point2,
    pub hi: Point2,
    pub sick: bool,
    pub value: Option<f64>,
    pub view: Option<String>,
}

pub struct Drawing {
    pub doc: doc::Doc,
    pub models: Vec<ModelOut>,
    pub sheets: Vec<String>,
    pub pages: Vec<Page>,
    pub views: Vec<ViewOut>,
    pub items: Vec<ItemOut>,
    pub errors: Vec<DrawError>,
}

impl Drawing {
    pub fn ok(&self) -> bool {
        self.errors.iter().all(|e| e.warning)
    }
    pub fn view(&self, id: &str) -> Option<&ViewOut> {
        self.views.iter().find(|v| v.id == id)
    }
}

fn warn(problem: Problem, span: Span, fixes: Vec<DrawOp>) -> DrawError {
    DrawError {
        problem,
        span,
        warning: true,
        fixes,
    }
}

fn fail(problem: Problem, span: Span) -> DrawError {
    DrawError {
        problem,
        span,
        warning: false,
        fixes: vec![],
    }
}

/// A part file's first comment line, read as part number and description: `# RACK-101 post: ..`.
fn describe(path: &Path, file: &str) -> (String, String) {
    let stem = Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().into_owned());
    let Ok(src) = std::fs::read_to_string(path) else {
        return (stem, String::new());
    };
    let first = src
        .lines()
        .find(|l| l.trim_start().starts_with('#'))
        .unwrap_or("");
    let text = first.trim_start().trim_start_matches('#').trim();
    let mut words = text.splitn(2, ' ');
    let head = words.next().unwrap_or("");
    let looks_like_number = head.contains('-') && head.chars().any(|c| c.is_ascii_digit());
    let (number, rest) = if looks_like_number {
        (head.trim_end_matches(':').to_string(), words.next().unwrap_or(""))
    } else {
        (stem, text)
    };
    let desc = rest
        .split(". ")
        .next()
        .unwrap_or("")
        .trim_end_matches('.')
        .trim()
        .to_string();
    (number, desc)
}

fn load(id: &str, file: &str, dir: &Path) -> Result<ModelOut, String> {
    let path = dir.join(file);
    let src = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let opts = crate::Options {
        build: true,
        dir: path.parent().map(Path::to_path_buf),
        ..crate::Options::default()
    };
    let (run, _) = crate::run(&src, opts, crate::build::Cache::default());
    if !run.ok() {
        let e = &run.errors[0];
        return Err(format!("does not build: {}: {}", e.kind(), e.text()));
    }
    let model = Model::of(&run).ok_or("has no solid")?;
    let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let bom = match &run.asm {
        Some(a) => a
            .bom
            .iter()
            .map(|(f, n)| {
                let (num, desc) = describe(&resolve(&base, &model, f), f);
                (f.clone(), *n, num, desc)
            })
            .collect(),
        None => {
            let (num, desc) = describe(&path, file);
            vec![(file.to_string(), 1, num, desc)]
        }
    };
    Ok(ModelOut {
        id: id.to_string(),
        file: file.to_string(),
        model,
        run,
        bom,
    })
}

/// Where a part file of an assembly is: as written, from the assembly's directory or a
/// subassembly's (the first that exists).
fn resolve(base: &Path, model: &Model, f: &str) -> PathBuf {
    let direct = base.join(f);
    if direct.exists() {
        return direct;
    }
    let name = Path::new(f)
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    for other in &model.files {
        let p = base.join(other);
        if p.file_name() == Some(&name) && p.exists() {
            return p;
        }
    }
    let _ = name;
    direct
}

/// Every piece's box, low and high corners.
fn bounds(pieces: &[Piece], extra: &[Stroke]) -> (Point2, Point2) {
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for g in pieces
        .iter()
        .map(|p| &p.geom)
        .chain(extra.iter().map(|s| &s.geom))
    {
        let (a, b) = g.bounds();
        lo = Point2::new(lo.x.min(a.x), lo.y.min(a.y));
        hi = Point2::new(hi.x.max(b.x), hi.y.max(b.y));
    }
    if lo.x > hi.x {
        return (Point2::ORIGIN, Point2::ORIGIN);
    }
    (lo, hi)
}

/// Whether an occurrence path is `prefix` or inside it.
fn under(path: &str, prefix: &str) -> bool {
    path == prefix || path.strip_prefix(prefix).is_some_and(|r| r.starts_with('.'))
}

fn is_hardware(file: &str) -> bool {
    file.contains("hw/")
}

/// How far each occurrence moves when the assembly is drawn `t` of the way apart: each top-level
/// unit away from the assembly's middle along the axis it sits farthest out on, by its own size,
/// and hardware out along its own axis by its length (Inventor's tweaks, made automatically).
fn explode_offsets(model: &Model, t: f64) -> Vec<Vec3> {
    let n = model.bodies.len();
    let mut out = vec![Vec3::ZERO; n];
    if t == 0.0 {
        return out;
    }
    let boxes: Vec<forge_math::Box3> = model
        .bodies
        .iter()
        .map(|(b, pose)| {
            let lb = b.faces_box();
            let mut w = forge_math::Box3::EMPTY;
            for k in 0..8 {
                let p = Point3::new(
                    if k & 1 == 0 { lb.x.lo } else { lb.x.hi },
                    if k & 2 == 0 { lb.y.lo } else { lb.y.hi },
                    if k & 4 == 0 { lb.z.lo } else { lb.z.hi },
                );
                w.include(pose.point_to_world(p));
            }
            w
        })
        .collect();
    let all = boxes.iter().fold(forge_math::Box3::EMPTY, |a, b| a.hull(b));
    let mid = all.center();
    let unit = |k: usize| model.paths[k].split('.').next().unwrap_or("").to_string();
    let mut units: Vec<String> = (0..n).map(unit).collect();
    units.sort();
    units.dedup();
    for u in &units {
        let ks: Vec<usize> = (0..n).filter(|&k| &unit(k) == u).collect();
        let ub = ks.iter().fold(forge_math::Box3::EMPTY, |a, &k| a.hull(&boxes[k]));
        let d = ub.center() - mid;
        let size = [all.x.width(), all.y.width(), all.z.width()];
        let rel = [
            d.x / size[0].max(1e-9),
            d.y / size[1].max(1e-9),
            d.z / size[2].max(1e-9),
        ];
        let axis = (0..3)
            .max_by(|&a, &b| rel[a].abs().total_cmp(&rel[b].abs()))
            .unwrap_or(2);
        let mut dir = Vec3::ZERO;
        let s = if rel[axis] >= 0.0 { 1.0 } else { -1.0 };
        let reach = [ub.x.width(), ub.y.width(), ub.z.width()][axis];
        match axis {
            0 => dir.x = s,
            1 => dir.y = s,
            _ => dir.z = s,
        }
        // a unit at the middle stays
        if rel[axis].abs() < 0.05 {
            continue;
        }
        for &k in &ks {
            out[k] = dir * (t * (0.6 * reach + 0.25 * size[axis]));
        }
    }
    for (k, o) in out.iter_mut().enumerate() {
        if is_hardware(&model.files[k]) {
            let (b, pose) = &model.bodies[k];
            let lb = b.faces_box();
            let len = lb.z.width().max(lb.x.width()).max(lb.y.width());
            *o += pose.z().get() * (-1.5 * t * len);
        }
    }
    out
}

struct Ctx {
    dir: PathBuf,
    models: Vec<ModelOut>,
    sheets: Vec<(String, doc::SheetDef)>,
    views: Vec<ViewOut>,
    errors: Vec<DrawError>,
    letters: usize,
}

fn sheet_of(ctx: &Ctx, s: &Option<String>, parent: Option<&ViewOut>) -> usize {
    match s {
        Some(id) => ctx.sheets.iter().position(|x| &x.0 == id).unwrap_or(0),
        None => parent.map_or(0, |p| p.sheet),
    }
}

impl Ctx {
    fn next_letter(&mut self, given: &Option<String>) -> String {
        if let Some(l) = given {
            return l.clone();
        }
        // I, O, Q are not used as view letters
        let letters: Vec<char> = ('A'..='Z').filter(|c| !"IOQ".contains(*c)).collect();
        let l = letters[self.letters % letters.len()].to_string();
        self.letters += 1;
        l
    }

    fn model_of(&self, id: &str) -> Option<usize> {
        self.models.iter().position(|m| m.id == id)
    }

    fn view(&mut self, item: &Item, def: &ViewDef) -> Option<ViewOut> {
        let tangent = def.tangent.unwrap_or(false);
        match &def.kind {
            ViewKind::Base {
                model,
                toward,
                up,
                explode,
            } => {
                let m = self.model_of(model)?;
                let view = View::new(*toward, *up)?;
                let mo = &self.models[m].model;
                let offs = explode_offsets(mo, *explode);
                let insts: Vec<(Arc<Body>, Frame, usize)> = (0..mo.bodies.len())
                    .filter(|&k| !def.hide.iter().any(|h| under(&mo.paths[k], h)))
                    .map(|k| {
                        let (b, pose) = &mo.bodies[k];
                        (b.clone(), pose.with_origin(pose.origin + offs[k]), k)
                    })
                    .collect();
                let iso = view.toward.get().x.abs() > 1e-9
                    && view.toward.get().y.abs() > 1e-9
                    && view.toward.get().z.abs() > 1e-9;
                let hidden = def.hidden.unwrap_or(!iso);
                let pieces = draw(&insts, &view, tangent, hidden);
                let s = def.scale.unwrap_or(1.0);
                let (lo, hi) = bounds(&pieces, &[]);
                let c = lo.lerp(hi, 0.5);
                Some(self.out(
                    item,
                    def,
                    "base",
                    None,
                    m,
                    view,
                    s,
                    Place::new(def.at - c.to_vec() * s, s),
                    pieces,
                    insts,
                ))
            }
            ViewKind::Projected { parent } => {
                let p = self.views.iter().find(|v| &v.id == parent)?.clone();
                let third = self.sheets.get(p.sheet).is_none_or(|s| s.1.third_angle);
                let pc = p.lo.lerp(p.hi, 0.5);
                let (dx, dy) = (def.at.x - pc.x, def.at.y - pc.y);
                let diagonal = dx.abs().min(dy.abs()) > 0.3 * dx.abs().max(dy.abs());
                let view = if diagonal {
                    p.view.projected(dx, dy, third)?
                } else if dx.abs() >= dy.abs() {
                    p.view.projected(dx, 0.0, third)?
                } else {
                    p.view.projected(0.0, dy, third)?
                };
                let hidden = def.hidden.unwrap_or(!diagonal);
                let pieces = draw(&p.insts, &view, tangent, hidden);
                let s = def.scale.unwrap_or(p.scale);
                let (lo, hi) = bounds(&pieces, &[]);
                let c = lo.lerp(hi, 0.5);
                let mut o = def.at - c.to_vec() * s;
                // an orthographic child stays aligned with its parent
                if !diagonal && s == p.scale {
                    if dx.abs() >= dy.abs() {
                        o.y = p.place.o.y;
                    } else {
                        o.x = p.place.o.x;
                    }
                }
                Some(self.out(
                    item,
                    def,
                    "projected",
                    Some(parent.clone()),
                    p.model,
                    view,
                    s,
                    Place::new(o, s),
                    pieces,
                    p.insts.clone(),
                ))
            }
            ViewKind::Section { parent, line } => {
                let p = self.views.iter().find(|v| &v.id == parent)?.clone();
                let at = p.place.inverse_point(def.at);
                let n = (line[1] - line[0]).perp().normalize()?;
                let away = if (at - line[0]).dot(n) >= 0.0 { n } else { n * -1.0 };
                let view = p.view.projected(away.x, away.y, true)?;
                let cut = Cut {
                    view: p.view,
                    line: line.clone(),
                    away,
                };
                let reach = 4.0
                    * p.insts
                        .iter()
                        .map(|(b, pose, _)| b.faces_box().diagonal() + pose.origin.to_vec().length())
                        .fold(1.0, f64::max);
                let mut insts = Vec::new();
                let mut faces = Vec::new();
                let mo = &self.models[p.model].model;
                for (k, (b, pose, occ)) in p.insts.iter().enumerate() {
                    if is_hardware(&mo.files[*occ]) {
                        insts.push((b.clone(), *pose, *occ));
                        continue;
                    }
                    match cut_body(b, pose, &cut, reach, 0x5ec7_0000 + k as u64) {
                        Ok(c) => {
                            if c.face_ids().next().is_none() {
                                continue;
                            }
                            let c = Arc::new(c);
                            faces.push((insts.len(), cut_faces(&c, pose, &cut)));
                            insts.push((c, *pose, *occ));
                        }
                        Err(why) => {
                            self.errors.push(warn(
                                Problem::BadValue {
                                    func: "section",
                                    arg: "line".into(),
                                    why: format!(
                                        "{} could not be cut ({why}); it is drawn whole",
                                        mo.paths[*occ]
                                    ),
                                },
                                item.span,
                                vec![],
                            ));
                            insts.push((b.clone(), *pose, *occ));
                        }
                    }
                }
                let hidden = def.hidden.unwrap_or(false);
                let pieces = draw(&insts, &view, tangent, hidden);
                let mut extra = Vec::new();
                let s = def.scale.unwrap_or(p.scale);
                for (n, (i, fs)) in faces.iter().enumerate() {
                    let (b, pose, _) = &insts[*i];
                    for &f in fs {
                        let loops = face_loops(b, pose, f, &view, 0.01);
                        let mut page = Page::default();
                        // hatch 3 mm apart on the sheet, each part its own way
                        let angle = if n % 2 == 0 { 45.0 } else { 135.0 };
                        annot::hatch(&mut page, &loops, angle, 3.0 / s);
                        extra.extend(page.strokes);
                    }
                }
                let (lo, hi) = bounds(&pieces, &extra);
                let c = lo.lerp(hi, 0.5);
                let mut o = def.at - c.to_vec() * s;
                if s == p.scale {
                    if away.x.abs() >= away.y.abs() {
                        o.y = p.place.o.y;
                    } else {
                        o.x = p.place.o.x;
                    }
                }
                let mut v = self.out(
                    item,
                    def,
                    "section",
                    Some(parent.clone()),
                    p.model,
                    view,
                    s,
                    Place::new(o, s),
                    pieces,
                    insts,
                );
                v.extra = extra;
                v.label = self.next_letter(&def.label);
                v.clip = None;
                // the cutting line on the parent, kept with the section for drawing
                let pts: Vec<Point2> = line.clone();
                let look = away * -1.0;
                v.extra_text.push(Text {
                    at: Point2::ORIGIN,
                    h: 0.0,
                    angle: 0.0,
                    align: Align::Left,
                    text: encode_line(&pts, look),
                    pen: Pen::Cut,
                });
                Some(v)
            }
            ViewKind::Detail {
                parent,
                centre,
                r,
                rect,
            } => {
                let p = self.views.iter().find(|v| &v.id == parent)?.clone();
                let s = def.scale.unwrap_or(2.0 * p.scale);
                let inside = |q: Point2| match rect {
                    Some((w, h)) => (q.x - centre.x).abs() <= *w && (q.y - centre.y).abs() <= *h,
                    None => q.distance(*centre) <= *r,
                };
                let mut pieces = Vec::new();
                for pc in &p.pieces {
                    for g in pc.geom.runs_where(0.01, |t| inside(pc.geom.at(t))) {
                        pieces.push(Piece {
                            geom: g,
                            ..pc.clone()
                        });
                    }
                }
                let mut extra = Vec::new();
                for st in &p.extra {
                    for g in st.geom.runs_where(0.01, |t| inside(st.geom.at(t))) {
                        extra.push(Stroke { pen: st.pen, geom: g });
                    }
                }
                let o = def.at - centre.to_vec() * s;
                let mut v = self.out(
                    item,
                    def,
                    "detail",
                    Some(parent.clone()),
                    p.model,
                    p.view,
                    s,
                    Place::new(o, s),
                    pieces,
                    p.insts.clone(),
                );
                v.extra = extra;
                v.label = self.next_letter(&def.label);
                v.clip = Some((*centre, *r, *rect));
                Some(v)
            }
            ViewKind::Flat { model } => {
                let m = self.model_of(model)?;
                let run = &self.models[m].run;
                let Some(pattern) = run.sheets.iter().find_map(|sh| sh.pattern().ok()) else {
                    self.errors.push(fail(
                        Problem::NotHere {
                            item: item.id.clone(),
                            why: "the model has no sheet metal to flatten".into(),
                        },
                        item.span,
                    ));
                    return None;
                };
                let mut extra = Vec::new();
                let mut texts = Vec::new();
                for c in pattern.contours.iter().flatten() {
                    extra.push(Stroke {
                        pen: Pen::Visible,
                        geom: curve2(c),
                    });
                }
                for b in &pattern.bends {
                    extra.push(Stroke {
                        pen: Pen::Center,
                        geom: Geom2::Line { a: b.a, b: b.b },
                    });
                    let d = b.b - b.a;
                    let mut ang = dmath::atan2(d.y, d.x);
                    if ang > dmath::FRAC_PI_2 + 1e-9 {
                        ang -= dmath::PI;
                    } else if ang <= -dmath::FRAC_PI_2 + 1e-9 {
                        ang += dmath::PI;
                    }
                    texts.push(Text {
                        at: b.a.lerp(b.b, 0.5),
                        h: 2.5,
                        angle: ang.to_degrees(),
                        align: Align::Centre,
                        text: forge_sheet::bend_note(b),
                        pen: Pen::Thin,
                    });
                }
                let s = def.scale.unwrap_or(1.0);
                let (lo, hi) = bounds(&[], &extra);
                let c = lo.lerp(hi, 0.5);
                let mut v = self.out(
                    item,
                    def,
                    "flat",
                    None,
                    m,
                    View::named("front")?,
                    s,
                    Place::new(def.at - c.to_vec() * s, s),
                    vec![],
                    vec![],
                );
                v.extra = extra;
                v.extra_text = texts;
                Some(v)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn out(
        &self,
        item: &Item,
        def: &ViewDef,
        kind: &'static str,
        parent: Option<String>,
        model: usize,
        view: View,
        scale: f64,
        place: Place,
        pieces: Vec<Piece>,
        insts: Vec<(Arc<Body>, Frame, usize)>,
    ) -> ViewOut {
        let parent_view = parent
            .as_ref()
            .and_then(|p| self.views.iter().find(|v| &v.id == p));
        let sheet = sheet_of(self, &def.sheet, parent_view);
        let (lo, hi) = bounds(&pieces, &[]);
        ViewOut {
            id: item.id.clone(),
            sheet,
            kind,
            parent,
            model,
            view,
            scale,
            place,
            lo: place.point(lo),
            hi: place.point(hi),
            pieces,
            extra: vec![],
            extra_text: vec![],
            insts,
            label: def.label.clone().unwrap_or_default(),
            clip: None,
        }
    }
}

/// A section's cutting line, carried as text on the view until the parent is drawn.
fn encode_line(pts: &[Point2], look: Vec2) -> String {
    let mut s = format!("{} {}", look.x, look.y);
    for p in pts {
        s.push_str(&format!(" {} {}", p.x, p.y));
    }
    s
}

fn decode_line(s: &str) -> (Vec2, Vec<Point2>) {
    let v: Vec<f64> = s.split(' ').filter_map(|x| x.parse().ok()).collect();
    let look = Vec2::new(v[0], v[1]);
    (look, v[2..].chunks(2).map(|c| Point2::new(c[0], c[1])).collect())
}

fn curve2(c: &forge_sheet::Curve2) -> Geom2 {
    match c {
        forge_sheet::Curve2::Line { a, b } => Geom2::Line { a: *a, b: *b },
        forge_sheet::Curve2::Arc { c, r, start, sweep } => {
            let (t0, t1) = if *sweep >= 0.0 {
                (*start, start + sweep)
            } else {
                (start + sweep, *start)
            };
            Geom2::Conic {
                c: *c,
                u: Vec2::new(*r, 0.0),
                v: Vec2::new(0.0, *r),
                t0,
                t1,
            }
        }
        forge_sheet::Curve2::Spline {
            degree,
            knots,
            points,
            weights,
        } => {
            let n = 64 * points.len();
            let (lo, hi) = (knots[*degree], knots[knots.len() - 1 - degree]);
            Geom2::Poly(
                (0..=n)
                    .map(|k| {
                        de_boor(
                            *degree,
                            knots,
                            points,
                            weights.as_deref(),
                            lo + (hi - lo) * k as f64 / n as f64,
                        )
                    })
                    .collect(),
            )
        }
    }
}

/// A point of a clamped (rational) B-spline.
fn de_boor(p: usize, knots: &[f64], pts: &[Point2], w: Option<&[f64]>, t: f64) -> Point2 {
    let n = pts.len();
    let mut k = p;
    while k + 1 < n && knots[k + 1] <= t {
        k += 1;
    }
    let wt = |i: usize| w.map_or(1.0, |w| w[i]);
    let mut d: Vec<(f64, f64, f64)> = (0..=p)
        .map(|j| {
            let i = j + k - p;
            (pts[i].x * wt(i), pts[i].y * wt(i), wt(i))
        })
        .collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = j + k - p;
            let den = knots[i + p + 1 - r] - knots[i];
            let a = if den == 0.0 { 0.0 } else { (t - knots[i]) / den };
            d[j] = (
                (1.0 - a) * d[j - 1].0 + a * d[j].0,
                (1.0 - a) * d[j - 1].1 + a * d[j].1,
                (1.0 - a) * d[j - 1].2 + a * d[j].2,
            );
        }
    }
    Point2::new(d[p].0 / d[p].2, d[p].1 / d[p].2)
}

fn draw(insts: &[(Arc<Body>, Frame, usize)], view: &View, tangent: bool, hidden: bool) -> Vec<Piece> {
    let is: Vec<Instance> = insts
        .iter()
        .map(|(b, pose, _)| Instance { body: b, pose: *pose })
        .collect();
    let opts = forge_draw::Options { tangent, chord: 0.01 };
    let mut p = forge_draw::hlr(&is, view, &opts);
    if !hidden {
        p.retain(|x| x.visible);
    }
    p
}

/// What a reference is on a view: a point, a line, or a circle (centre and radius in model mm).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Feat {
    Point(Point2),
    Line(Point2, Point2),
    Circle { c: Point2, r: f64, a: Point2, b: Point2 },
}

impl Feat {
    fn anchor(&self) -> Point2 {
        match *self {
            Feat::Point(p) => p,
            Feat::Line(a, b) => a.lerp(b, 0.5),
            Feat::Circle { c, .. } => c,
        }
    }
}

/// The edges a name finds in a body: itself, its pieces, or what it was merged into.
fn edges_named(body: &Body, n: Name) -> Vec<EdgeId> {
    let alias = body.joins.aliases.iter().find(|a| a.0 == n).map(|a| a.1);
    body.edge_ids()
        .filter(|&e| {
            let m = body.edge(e).name;
            m == n || m.is_piece_of(n) || Some(m) == alias
        })
        .collect()
}

fn faces_named(body: &Body, n: Name) -> Vec<forge_topo::FaceId> {
    body.face_ids()
        .filter(|&f| {
            let m = body.face(f).name;
            m == n || m.is_piece_of(n)
        })
        .collect()
}

/// A reference resolved on a view: what it is there, and whether the view draws it.
struct Resolved {
    feat: Feat,
    shown: bool,
    inst: usize,
}

fn resolve_ref(model: &Model, v: &ViewOut, r: &Ref) -> Option<Resolved> {
    let occ = if r.occ.is_empty() {
        Some(0)
    } else {
        model.paths.iter().position(|p| p == &r.occ)
    }?;
    let inst = v.insts.iter().position(|x| x.2 == occ)?;
    let (body, pose, _) = &v.insts[inst];
    let pr = |p: Point3| v.view.project(pose.point_to_world(p));
    if r.face {
        let fs = faces_named(body, r.name);
        let f = *fs.first()?;
        let b = body.face_box(f);
        let shown = v
            .pieces
            .iter()
            .any(|p| p.inst == inst && p.source == Source::Outline(body.face(f).name))
            || body.face(f).loops.iter().any(|&l| {
                body.lp(l).coedges().any(|c| {
                    v.pieces.iter().any(|p| {
                        p.inst == inst && p.source == Source::Edge(body.edge(body.coedge(c).edge).name)
                    })
                })
            });
        return Some(Resolved {
            feat: Feat::Point(pr(b.center())),
            shown,
            inst,
        });
    }
    let es = edges_named(body, r.name);
    let e0 = *es.first()?;
    let shown = es.iter().any(|&e| {
        v.pieces
            .iter()
            .any(|p| p.inst == inst && p.source == Source::Edge(body.edge(e).name))
    });
    let edge = body.edge(e0);
    let (lo, hi) = (edge.range.lo, edge.range.hi);
    // the whole of a split edge: its pieces' extreme ends
    let ends = |e: EdgeId| {
        let x = body.edge(e);
        (x.curve.eval(x.range.lo), x.curve.eval(x.range.hi))
    };
    let feat = match &edge.curve {
        Curve::Circle(c) => {
            let (a, b) = ends(e0);
            Feat::Circle {
                c: pr(c.frame.origin),
                r: c.radius,
                a: pr(a),
                b: pr(b),
            }
        }
        Curve::Line(l) => {
            let mut ts: Vec<f64> = Vec::new();
            for &e in &es {
                let (a, b) = ends(e);
                ts.push(l.param_of(a));
                ts.push(l.param_of(b));
            }
            let (t0, t1) = ts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |m, &t| {
                (m.0.min(t), m.1.max(t))
            });
            let (a, b) = (pr(l.eval(t0)), pr(l.eval(t1)));
            if a.distance(b) < 1e-9 {
                Feat::Point(a)
            } else {
                Feat::Line(a, b)
            }
        }
        c => Feat::Line(pr(c.eval(lo)), pr(c.eval(hi))),
    };
    let feat = match r.at {
        None => feat,
        Some(At::Start) => Feat::Point(pr(edge.curve.eval(lo))),
        Some(At::End) => Feat::Point(pr(edge.curve.eval(hi))),
        Some(At::Mid) => Feat::Point(pr(edge.curve.eval(0.5 * (lo + hi)))),
        Some(At::Center) => match feat {
            Feat::Circle { c, .. } => Feat::Point(c),
            f => Feat::Point(f.anchor()),
        },
    };
    Some(Resolved { feat, shown, inst })
}

/// The edges of a view near a sheet point, as reattach suggestions: same kind first.
fn nearest_refs(model: &Model, v: &ViewOut, at: Point2, want_circle: bool) -> Vec<Ref> {
    let mut c: Vec<(f64, Ref)> = Vec::new();
    for p in &v.pieces {
        let Source::Edge(n) = p.source else { continue };
        let (body, _, occ) = &v.insts[p.inst];
        let Some(&e) = edges_named(body, n).first() else {
            continue;
        };
        let circle = matches!(body.edge(e).curve, Curve::Circle(_));
        let q = v
            .place
            .point(p.geom.at(0.5 * (p.geom.range().0 + p.geom.range().1)));
        let d = q.distance(at) + if circle == want_circle { 0.0 } else { 1e6 };
        let r = Ref {
            occ: if model.paths.len() > 1 {
                model.paths[*occ].clone()
            } else {
                String::new()
            },
            name: n,
            face: false,
            at: None,
            span: Span::new(0, 0),
        };
        if !c.iter().any(|x| x.1 == r) {
            c.push((d, r));
        }
    }
    c.sort_by(|a, b| a.0.total_cmp(&b.0));
    c.into_iter().take(3).map(|x| x.1).collect()
}

struct Annotator<'a> {
    st: Style,
    page: &'a mut Page,
}

fn fmt_len(v: f64) -> String {
    number(v, 2)
}

/// Lays out one annotation on its sheet; the measured value, or why it could not be.
fn annotate(
    ctx: &Ctx,
    item: &Item,
    a: &AnnotDef,
    page: &mut Page,
    errors: &mut Vec<DrawError>,
    items: &mut Vec<ItemOut>,
) -> Option<()> {
    let v = ctx.views.iter().find(|x| x.id == a.view)?;
    let mo = &ctx.models[v.model];
    let model = &mo.model;
    let mut an = Annotator {
        st: Style::default(),
        page,
    };
    let s = v.scale;
    let sp = |p: Point2| v.place.point(p);
    let default_at = v.hi + Vec2::new(5.0, 5.0);
    let at_sheet = a.at.map_or(default_at, sp);
    let before = (an.page.strokes.len(), an.page.texts.len(), an.page.fills.len());
    let mut sick = false;
    // resolves a reference; a gone one makes the item sick, with reattach fixes
    let mut get = |r: &Ref, arg: &str, circle: bool, errors: &mut Vec<DrawError>| -> Option<Resolved> {
        match resolve_ref(model, v, r) {
            Some(x) => {
                if !x.shown {
                    errors.push(warn(
                        Problem::NotInView {
                            item: item.id.clone(),
                            reference: r.text(),
                        },
                        r.span,
                        vec![],
                    ));
                }
                Some(x)
            }
            None => {
                sick = true;
                let mut fixes = vec![DrawOp::DeleteItem {
                    item: item.id.clone(),
                }];
                for n in nearest_refs(model, v, at_sheet, circle) {
                    fixes.push(DrawOp::SetArg {
                        item: item.id.clone(),
                        arg: arg.to_string(),
                        value: n.text(),
                    });
                }
                errors.push(warn(
                    Problem::Sick {
                        item: item.id.clone(),
                        reference: r.text(),
                    },
                    r.span,
                    fixes,
                ));
                None
            }
        }
    };
    let text = |measured: String| format!("{}{}{}", a.prefix, a.text.clone().unwrap_or(measured), a.suffix);
    let pen = Pen::Thin;
    let mut value = None;
    let kind: &'static str = match &a.kind {
        AnnotKind::Linear { a: ra, b: rb, dir } => {
            let fa = get(ra, "1", false, errors);
            let fb = match rb {
                Some(rb) => get(rb, "2", false, errors).map(Some),
                None => Some(None),
            };
            if let (Some(fa), Some(fb)) = (fa, fb) {
                let (p, q, d) = match (fa.feat, fb.map(|x| x.feat)) {
                    (Feat::Line(a0, a1), None) => (a0, a1, (a1 - a0).normalize()),
                    (Feat::Circle { c, r, .. }, None) => (c - Vec2::X * (r), c + Vec2::X * r, Some(Vec2::X)),
                    (Feat::Point(p), None) => (p, p, None),
                    (Feat::Line(a0, a1), Some(Feat::Line(b0, b1)))
                        if (a1 - a0)
                            .normalize()
                            .zip((b1 - b0).normalize())
                            .is_some_and(|(x, y)| x.cross(y).abs() < 1e-6) =>
                    {
                        let u = (a1 - a0).normalize().expect("a line");
                        let m = a0.lerp(a1, 0.5);
                        let foot = b0 + u * (m - b0).dot(u);
                        (m, foot, Some(u.perp()))
                    }
                    (fa, Some(fb)) => {
                        // from a point to a line: to the foot of the perpendicular
                        let foot = |p: Point2, l0: Point2, l1: Point2| {
                            let u = (l1 - l0).normalize().unwrap_or(Vec2::X);
                            (l0 + u * (p - l0).dot(u), Some(u.perp()))
                        };
                        match (fa, fb) {
                            (Feat::Point(_) | Feat::Circle { .. }, Feat::Line(b0, b1)) => {
                                let p = fa.anchor();
                                let (q, d) = foot(p, b0, b1);
                                (p, q, d)
                            }
                            (Feat::Line(a0, a1), Feat::Point(_) | Feat::Circle { .. }) => {
                                let q = fb.anchor();
                                let (p, d) = foot(q, a0, a1);
                                (p, q, d)
                            }
                            _ => (fa.anchor(), fb.anchor(), None),
                        }
                    }
                };
                let (ps, qs) = (sp(p), sp(q));
                let dir = match dir.as_deref() {
                    Some("horizontal") => Vec2::X,
                    Some("vertical") => Vec2::Y,
                    Some(_) => (qs - ps).normalize().unwrap_or(Vec2::X),
                    None => match d {
                        Some(d) => d,
                        None => {
                            // placed above or below: horizontal; beside: vertical
                            let m = ps.lerp(qs, 0.5);
                            let off = at_sheet - m;
                            let span = qs - ps;
                            if span.y.abs() < 1e-9 || (off.y.abs() > off.x.abs() && span.x.abs() > 1e-9) {
                                Vec2::X
                            } else {
                                Vec2::Y
                            }
                        }
                    },
                };
                let val = ((qs - ps).dot(dir) / s).abs();
                value = Some(val);
                let at = a.at.map_or(ps.lerp(qs, 0.5) + dir.perp() * 10.0, sp);
                annot::linear(an.page, pen, &an.st, ps, qs, dir, at, &text(fmt_len(val)));
            }
            "dim"
        }
        AnnotKind::Angle { a: ra, b: rb } => {
            if let (Some(fa), Some(fb)) = (get(ra, "1", false, errors), get(rb, "2", false, errors))
                && let (Feat::Line(a0, a1), Feat::Line(b0, b1)) = (fa.feat, fb.feat)
            {
                let (a0, a1, b0, b1) = (sp(a0), sp(a1), sp(b0), sp(b1));
                let (u, w) = (a1 - a0, b1 - b0);
                let den = u.cross(w);
                if den.abs() > 1e-12 {
                    let t = (b0 - a0).cross(w) / den;
                    let vtx = a0 + u * t;
                    let at = a.at.map_or(
                        vtx + (u.normalize().unwrap_or(Vec2::X) + w.normalize().unwrap_or(Vec2::Y)) * 10.0,
                        sp,
                    );
                    // the arms toward where the dimension is placed
                    let pick = |d: Vec2| if d.dot(at - vtx) >= 0.0 { d } else { d * -1.0 };
                    let (da, db) = (pick(u), pick(w));
                    let ang = dmath::acos_clamped(da.normalize()?.dot(db.normalize()?)).to_degrees();
                    value = Some(ang);
                    annot::angular(
                        an.page,
                        pen,
                        &an.st,
                        vtx,
                        da,
                        db,
                        at,
                        &text(format!("{}\u{b0}", number(ang, 1))),
                    );
                }
            }
            "angle"
        }
        AnnotKind::Radius { e } | AnnotKind::Diameter { e } => {
            let diameter = matches!(a.kind, AnnotKind::Diameter { .. });
            if let Some(f) = get(e, "1", true, errors) {
                match f.feat {
                    Feat::Circle { c, r, .. } => {
                        let val = if diameter { 2.0 * r } else { r };
                        value = Some(val);
                        let at = a.at.map_or(sp(c) + Vec2::new(1.0, 1.0) * (r * s + 8.0), sp);
                        let t = if diameter {
                            format!("\u{2300}{}", fmt_len(val))
                        } else {
                            format!("R{}", fmt_len(val))
                        };
                        annot::radial(an.page, pen, &an.st, sp(c), r * s, at, &text(t));
                    }
                    _ => errors.push(fail(
                        Problem::BadValue {
                            func: if diameter { "diameter" } else { "radius" },
                            arg: "edge".into(),
                            why: "the edge is not a circle or arc".into(),
                        },
                        e.span,
                    )),
                }
            }
            if diameter { "diameter" } else { "radius" }
        }
        AnnotKind::HoleNote { e } => {
            if let Some(f) = get(e, "1", true, errors)
                && let Feat::Circle { c, r, .. } = f.feat
            {
                let (body, _, _) = &v.insts[f.inst];
                let depth = hole_depth(body, e.name);
                // like holes on this view: same radius, drawn as circles
                let mut centres: Vec<Point2> = Vec::new();
                for p in &v.pieces {
                    if let (Some((pc, pr)), Source::Edge(_)) = (p.geom.circle(), p.source)
                        && (pr - r).abs() < 1e-6
                        && p.inst == f.inst
                        && !centres.iter().any(|q| q.distance(pc) < 1e-6)
                    {
                        centres.push(pc);
                    }
                }
                let count = centres.len().max(1);
                let head = if count > 1 {
                    format!("{count}X ")
                } else {
                    String::new()
                };
                let d = match depth {
                    Some(h) => format!(" \u{2193}{}", fmt_len(h)),
                    None => " THRU".into(),
                };
                value = Some(2.0 * r);
                let mut lines = vec![format!(
                    "{}{head}\u{2300}{}{d}{}",
                    a.prefix,
                    fmt_len(2.0 * r),
                    a.suffix
                )];
                if let Some(t) = &a.text {
                    lines.extend(t.split('\n').map(str::to_string));
                }
                let at = a.at.map_or(sp(c) + Vec2::new(12.0, 12.0), sp);
                let dir = (at - sp(c)).normalize().unwrap_or(Vec2::X);
                annot::leader(an.page, pen, &an.st, sp(c) + dir * (r * s), at, &lines);
            }
            "hole_note"
        }
        AnnotKind::Set {
            base,
            refs,
            dir,
            style,
        } => {
            let fb = get(base, "1", false, errors);
            let fs: Vec<Option<Resolved>> = refs
                .iter()
                .enumerate()
                .map(|(k, r)| get(r, &format!("refs[{k}]"), false, errors))
                .collect();
            if let Some(fb) = fb {
                let u = if dir == "vertical" { Vec2::Y } else { Vec2::X };
                let o = sp(fb.feat.anchor());
                let at = a.at.map_or(o + u.perp() * -12.0, sp);
                let mut pts: Vec<Point2> = fs.iter().flatten().map(|f| sp(f.feat.anchor())).collect();
                pts.sort_by(|p, q| (*p - o).dot(u).total_cmp(&(*q - o).dot(u)));
                match style {
                    SetStyle::Ordinate => {
                        let line_at = (at - o).dot(u.perp());
                        for p in std::iter::once(o).chain(pts.iter().copied()) {
                            let foot = p + u.perp() * (line_at - (p - o).dot(u.perp()));
                            annot::stroke(
                                an.page,
                                pen,
                                p + (foot - p).normalize().unwrap_or(u.perp()) * an.st.gap,
                                foot,
                            );
                            let val = (p - o).dot(u) / s;
                            an.page.texts.push(Text {
                                at: foot + (foot - p).normalize().unwrap_or(u.perp()) * 1.5,
                                h: an.st.text_h,
                                angle: if dir == "vertical" { 0.0 } else { 90.0 },
                                align: Align::Left,
                                text: fmt_len(val),
                                pen,
                            });
                        }
                    }
                    SetStyle::Baseline => {
                        for (k, p) in pts.iter().enumerate() {
                            let line = at + u.perp() * (-(k as f64) * 7.0 * (at - o).dot(u.perp()).signum());
                            let val = ((*p - o).dot(u) / s).abs();
                            annot::linear(an.page, pen, &an.st, o, *p, u, line, &fmt_len(val));
                        }
                    }
                    SetStyle::Chain => {
                        let mut prev = o;
                        for p in &pts {
                            let val = ((*p - prev).dot(u) / s).abs();
                            annot::linear(an.page, pen, &an.st, prev, *p, u, at, &fmt_len(val));
                            prev = *p;
                        }
                    }
                }
            }
            match style {
                SetStyle::Ordinate => "ordinate",
                SetStyle::Baseline => "baseline",
                SetStyle::Chain => "chain",
            }
        }
        AnnotKind::CenterMark { e } => {
            if let Some(f) = get(e, "1", true, errors)
                && let Feat::Circle { c, r, .. } = f.feat
            {
                annot::center_mark(an.page, sp(c), r * s, 3.0);
            }
            "center_mark"
        }
        AnnotKind::CenterMarks => {
            let mut done: Vec<Point2> = Vec::new();
            for p in &v.pieces {
                if !p.visible && p.class != Class::Sharp {
                    continue;
                }
                let Some((c, r)) = p.geom.circle() else { continue };
                let (t0, t1) = p.geom.range();
                if t1 - t0 < dmath::PI - 1e-6 || done.iter().any(|q| q.distance(c) < 1e-6) {
                    continue;
                }
                // concentric circles share one mark, the largest's
                let rmax = v
                    .pieces
                    .iter()
                    .filter_map(|q| q.geom.circle())
                    .filter(|(qc, _)| qc.distance(c) < 1e-6)
                    .map(|x| x.1)
                    .fold(r, f64::max);
                done.push(c);
                annot::center_mark(an.page, sp(c), rmax * s, 3.0);
            }
            "center_marks"
        }
        AnnotKind::Centerlines => {
            // cylinders seen from the side: their axes, as far as the faces reach
            let mut drawn: Vec<(Point2, Point2)> = Vec::new();
            for (k, (body, pose, _)) in v.insts.iter().enumerate() {
                for f in body.face_ids() {
                    let Surface::Cylinder(cy) = &body.face(f).surface else {
                        continue;
                    };
                    let ax = pose.vec_to_world(cy.frame.z().get());
                    if ax.dot(v.view.toward.get()).abs() > 1e-6 {
                        continue;
                    }
                    let name = body.face(f).name;
                    if !v
                        .pieces
                        .iter()
                        .any(|p| p.inst == k && p.source == Source::Outline(name))
                    {
                        continue;
                    }
                    let b = body.face_box(f);
                    let o = cy.frame.origin;
                    let z = cy.frame.z().get();
                    let corners = (0..8).map(|i| {
                        Point3::new(
                            if i & 1 == 0 { b.x.lo } else { b.x.hi },
                            if i & 2 == 0 { b.y.lo } else { b.y.hi },
                            if i & 4 == 0 { b.z.lo } else { b.z.hi },
                        )
                    });
                    let (lo, hi) = corners.fold((f64::INFINITY, f64::NEG_INFINITY), |m, p| {
                        let t = (p - o).dot(z);
                        (m.0.min(t), m.1.max(t))
                    });
                    let (pa, pb) = (
                        sp(v.view.project(pose.point_to_world(o + z * lo))),
                        sp(v.view.project(pose.point_to_world(o + z * hi))),
                    );
                    if pa.distance(pb) < 1.0
                        || drawn
                            .iter()
                            .any(|(x, y)| x.distance(pa) < 1e-3 && y.distance(pb) < 1e-3)
                    {
                        continue;
                    }
                    drawn.push((pa, pb));
                    annot::centerline(an.page, pa, pb, 2.0);
                }
            }
            "centerlines"
        }
        AnnotKind::Centerline { a: ra, b: rb } => {
            if let (Some(fa), Some(fb)) = (get(ra, "1", true, errors), get(rb, "2", true, errors)) {
                annot::centerline(an.page, sp(fa.feat.anchor()), sp(fb.feat.anchor()), 3.0);
            }
            "centerline"
        }
        AnnotKind::Leader { r } => {
            if let Some(f) = get(r, "1", false, errors) {
                let lines: Vec<String> = a
                    .text
                    .clone()
                    .unwrap_or_default()
                    .split('\n')
                    .map(str::to_string)
                    .collect();
                annot::leader(an.page, pen, &an.st, sp(f.feat.anchor()), at_sheet, &lines);
            }
            "leader"
        }
        AnnotKind::Balloon { occ } => {
            match balloon_tip(model, v, occ) {
                Some(tip) => {
                    let n = item_number(mo, occ);
                    annot::balloon(an.page, pen, &an.st, tip, at_sheet, &n);
                }
                None => {
                    sick = true;
                    errors.push(warn(
                        Problem::Sick {
                            item: item.id.clone(),
                            reference: format!("\"{occ}\""),
                        },
                        item.span,
                        vec![DrawOp::DeleteItem {
                            item: item.id.clone(),
                        }],
                    ));
                }
            }
            "balloon"
        }
        AnnotKind::AutoBalloons => {
            if model.paths.len() <= 1 {
                errors.push(fail(
                    Problem::NotHere {
                        item: item.id.clone(),
                        why: "balloons number an assembly's parts".into(),
                    },
                    item.span,
                ));
            }
            auto_balloons(mo, v, &mut an);
            "auto_balloons"
        }
    };
    if sick {
        // drawn where it was placed, in the sick colour, saying what it was
        an.page.strokes.truncate(before.0);
        an.page.texts.truncate(before.1);
        an.page.fills.truncate(before.2);
        an.page.texts.push(Text {
            at: at_sheet,
            h: 3.5,
            angle: 0.0,
            align: Align::Centre,
            text: format!("? {}", item.id),
            pen: Pen::Sick,
        });
    }
    let (mut lo, mut hi) = (at_sheet, at_sheet);
    for st in &an.page.strokes[before.0..] {
        let (a, b) = st.geom.bounds();
        lo = Point2::new(lo.x.min(a.x), lo.y.min(a.y));
        hi = Point2::new(hi.x.max(b.x), hi.y.max(b.y));
    }
    items.push(ItemOut {
        id: item.id.clone(),
        sheet: v.sheet,
        kind,
        lo,
        hi,
        sick,
        value,
        view: Some(v.id.clone()),
    });
    Some(())
}

/// A blind hole's depth: the bore's length when its far end is a floor (a face it alone bounds);
/// None for a hole that goes through.
fn hole_depth(body: &Body, rim: Name) -> Option<f64> {
    let e = *edges_named(body, rim).first()?;
    let faces: Vec<_> = body
        .edge(e)
        .coedges
        .iter()
        .map(|&c| body.coedge_face(c))
        .collect();
    let bore = *faces
        .iter()
        .find(|&&f| matches!(body.face(f).surface, Surface::Cylinder(_)))?;
    let Surface::Cylinder(cy) = &body.face(bore).surface else {
        return None;
    };
    let z = cy.frame.z().get();
    // the bore's other rims and what is past them
    for &l in body.face(bore).loops.iter() {
        for c in body.lp(l).coedges() {
            let ce = body.coedge(c).edge;
            if ce == e || !matches!(body.edge(ce).curve, Curve::Circle(_)) {
                continue;
            }
            let other = body.partner(c).map(|o| body.coedge_face(o))?;
            if body.face(other).loops.len() == 1 && other != bore {
                let b = body.face_box(bore);
                let (o, _) = (cy.frame.origin, 0);
                let corners = (0..8).map(|i| {
                    Point3::new(
                        if i & 1 == 0 { b.x.lo } else { b.x.hi },
                        if i & 2 == 0 { b.y.lo } else { b.y.hi },
                        if i & 4 == 0 { b.z.lo } else { b.z.hi },
                    )
                });
                let (lo, hi) = corners.fold((f64::INFINITY, f64::NEG_INFINITY), |m, p| {
                    let t = (p - o).dot(z);
                    (m.0.min(t), m.1.max(t))
                });
                return Some(hi - lo);
            }
        }
    }
    None
}

/// Where a balloon on an occurrence points: the middle of its longest drawn piece.
fn balloon_tip(model: &Model, v: &ViewOut, occ: &str) -> Option<Point2> {
    let k = model.paths.iter().position(|p| p == occ)?;
    let inst = v.insts.iter().position(|x| x.2 == k)?;
    let p = v
        .pieces
        .iter()
        .filter(|p| p.inst == inst && p.visible)
        .max_by(|a, b| a.geom.length().total_cmp(&b.geom.length()))?;
    let (t0, t1) = p.geom.range();
    Some(v.place.point(p.geom.at(0.5 * (t0 + t1))))
}

/// An occurrence's item number: its part file's place in the BOM.
fn item_number(mo: &ModelOut, occ: &str) -> String {
    let Some(k) = mo.model.paths.iter().position(|p| p == occ) else {
        return "?".into();
    };
    let file = &mo.model.files[k];
    let name = Path::new(file).file_name();
    mo.bom
        .iter()
        .position(|b| &b.0 == file || Path::new(&b.0).file_name() == name)
        .map_or("?".into(), |i| (i + 1).to_string())
}

/// A balloon on each part of the BOM seen in the view, round the view, none on another.
fn auto_balloons(mo: &ModelOut, v: &ViewOut, an: &mut Annotator) {
    let centre = v.lo.lerp(v.hi, 0.5);
    let reach = 0.5 * (v.hi - v.lo).length() + 12.0;
    let mut tips: Vec<(f64, Point2, String)> = Vec::new();
    for (i, (file, ..)) in mo.bom.iter().enumerate() {
        let name = Path::new(file).file_name();
        let occ = (0..mo.model.paths.len()).find(|&k| {
            (&mo.model.files[k] == file || Path::new(&mo.model.files[k]).file_name() == name)
                && v.insts.iter().any(|x| x.2 == k)
                && balloon_tip(&mo.model, v, &mo.model.paths[k]).is_some()
        });
        let Some(k) = occ else { continue };
        let tip = balloon_tip(&mo.model, v, &mo.model.paths[k]).expect("checked");
        let d = tip - centre;
        tips.push((dmath::atan2(d.y, d.x), tip, (i + 1).to_string()));
    }
    tips.sort_by(|a, b| a.0.total_cmp(&b.0));
    // spread round the ring at least a balloon apart
    let gap = 12.0 / reach;
    let mut last = f64::NEG_INFINITY;
    for (ang, tip, n) in tips {
        let a = ang.max(last + gap);
        last = a;
        let (s, c) = dmath::sin_cos(a);
        let at = centre + Vec2::new(c, s) * reach;
        annot::balloon(an.page, Pen::Thin, &an.st, tip, at, &n);
    }
}

/// The model the title block reads: the first model's first base view's.
fn defaults(ctx: &Ctx, f: &mut forge_draw::sheet::Fields, sheet: usize, n_sheets: usize) {
    let first_view = ctx
        .views
        .iter()
        .find(|v| v.sheet == sheet && v.kind == "base")
        .or(ctx.views.iter().find(|v| v.sheet == sheet));
    if let Some(v) = first_view {
        let mo = &ctx.models[v.model];
        if mo.model.paths.len() <= 1 {
            let (num, desc) = mo
                .bom
                .first()
                .map(|b| (b.2.clone(), b.3.clone()))
                .unwrap_or_default();
            if f.number.is_empty() {
                f.number = num;
            }
            if f.title.is_empty() {
                f.title = desc.to_uppercase();
            }
            if f.material.is_empty()
                && let Some(sh) = mo.run.sheets.first()
            {
                f.material = format!(
                    "{} t{}",
                    sh.material.clone().unwrap_or_default().to_uppercase(),
                    number(sh.sheet.gauge.thickness, 2)
                );
            }
        } else if f.title.is_empty() {
            f.title = Path::new(&mo.file)
                .file_stem()
                .map_or(String::new(), |s| s.to_string_lossy().to_uppercase());
        }
        if f.scale.is_empty() {
            f.scale = forge_draw::sheet::scale_text(v.scale);
        }
    }
    if f.sheet.is_empty() {
        f.sheet = format!("{} OF {}", sheet + 1, n_sheets);
    }
}

/// Makes the drawing: `src` read, its models built (paths from `dir`), its sheets laid out.
pub fn make(src: &str, dir: &Path) -> Drawing {
    let d = doc::read(src);
    let mut ctx = Ctx {
        dir: dir.to_path_buf(),
        models: vec![],
        sheets: vec![],
        views: vec![],
        errors: d.errors.clone(),
        letters: 0,
    };
    for item in &d.items {
        match &item.kind {
            ItemKind::Model { path, .. } => match load(&item.id, path, &ctx.dir) {
                Ok(m) => ctx.models.push(m),
                Err(why) => ctx.errors.push(fail(
                    Problem::Model {
                        file: path.clone(),
                        why,
                    },
                    item.span,
                )),
            },
            ItemKind::Sheet(s) => ctx.sheets.push((item.id.clone(), s.clone())),
            _ => {}
        }
    }
    if ctx.sheets.is_empty() {
        let (w, h) = forge_draw::sheet::size("B").expect("B is a size");
        ctx.sheets.push((
            "sheet".into(),
            doc::SheetDef {
                size: "B".into(),
                w,
                h,
                third_angle: true,
                fields: Default::default(),
            },
        ));
    }
    for item in &d.items {
        if let ItemKind::View(def) = &item.kind
            && let Some(v) = ctx.view(item, def)
        {
            ctx.views.push(v);
        }
    }
    let n = ctx.sheets.len();
    let mut pages: Vec<Page> = ctx
        .sheets
        .iter()
        .map(|(_, s)| Page {
            w: s.w,
            h: s.h,
            ..Page::default()
        })
        .collect();
    for (k, page) in pages.iter_mut().enumerate() {
        let mut f = ctx.sheets[k].1.fields.clone();
        defaults(&ctx, &mut f, k, n);
        forge_draw::sheet::format(page, &f, ctx.sheets[k].1.third_angle);
    }
    let mut items: Vec<ItemOut> = Vec::new();
    for v in &ctx.views {
        let page = &mut pages[v.sheet];
        for p in &v.pieces {
            page.strokes.push(Stroke {
                pen: match (p.visible, p.class) {
                    (false, _) => Pen::Hidden,
                    (true, Class::Tangent) => Pen::Tangent,
                    _ => Pen::Visible,
                },
                geom: p.geom.placed(&v.place),
            });
        }
        for st in &v.extra {
            page.strokes.push(Stroke {
                pen: st.pen,
                geom: st.geom.placed(&v.place),
            });
        }
        for t in v.extra_text.iter().filter(|t| t.pen != Pen::Cut) {
            page.texts.push(Text {
                at: v.place.point(t.at),
                ..t.clone()
            });
        }
        let st = Style::default();
        let below = Point2::new(0.5 * (v.lo.x + v.hi.x), v.lo.y - 9.0);
        match v.kind {
            "section" => {
                annot::view_label(
                    page,
                    below,
                    &format!("SECTION {0}-{0}", v.label),
                    &scale_note(&ctx, v),
                );
                if let (Some(p), Some(t)) = (
                    ctx.views.iter().find(|x| Some(&x.id) == v.parent.as_ref()),
                    v.extra_text.iter().find(|t| t.pen == Pen::Cut),
                ) {
                    let (look, pts) = decode_line(&t.text);
                    let sheet_pts: Vec<Point2> = pts.iter().map(|q| p.place.point(*q)).collect();
                    annot::cutting_line(&mut pages[p.sheet], &st, &sheet_pts, look, &v.label);
                }
            }
            "detail" => {
                let page = &mut pages[v.sheet];
                annot::view_label(
                    page,
                    below,
                    &format!("DETAIL {}", v.label),
                    &forge_draw::sheet::scale_text(v.scale),
                );
                if let (Some((c, r, rect)), Some(p)) = (
                    v.clip,
                    ctx.views.iter().find(|x| Some(&x.id) == v.parent.as_ref()),
                ) {
                    fence(&mut pages[v.sheet], &v.place, c, r, rect);
                    fence(&mut pages[p.sheet], &p.place, c, r, rect);
                    let lp = p
                        .place
                        .point(c + Vec2::new(r, r) * std::f64::consts::FRAC_1_SQRT_2)
                        + Vec2::new(2.0, 2.0);
                    pages[p.sheet].texts.push(Text {
                        at: lp,
                        h: 5.0,
                        angle: 0.0,
                        align: Align::Left,
                        text: v.label.clone(),
                        pen: Pen::Thin,
                    });
                }
            }
            _ => {
                if !v.label.is_empty() {
                    annot::view_label(page, below, &v.label, &scale_note(&ctx, v));
                }
            }
        }
        items.push(ItemOut {
            id: v.id.clone(),
            sheet: v.sheet,
            kind: v.kind,
            lo: v.lo,
            hi: v.hi,
            sick: false,
            value: None,
            view: Some(v.id.clone()),
        });
    }
    let mut errors = std::mem::take(&mut ctx.errors);
    for item in &d.items {
        match &item.kind {
            ItemKind::Annot(a) => {
                let sheet = ctx.views.iter().find(|v| v.id == a.view).map_or(0, |v| v.sheet);
                annotate(&ctx, item, a, &mut pages[sheet], &mut errors, &mut items);
            }
            ItemKind::Note { sheet, at, text } => {
                let k = sheet_of(&ctx, sheet, None);
                for (i, l) in text.split('\n').enumerate() {
                    pages[k].texts.push(Text {
                        at: *at - Vec2::new(0.0, 5.5 * i as f64),
                        h: 3.5,
                        angle: 0.0,
                        align: Align::Left,
                        text: l.to_string(),
                        pen: Pen::Thin,
                    });
                }
                items.push(ItemOut {
                    id: item.id.clone(),
                    sheet: k,
                    kind: "note",
                    lo: *at,
                    hi: *at,
                    sick: false,
                    value: None,
                    view: None,
                });
            }
            ItemKind::PartsList { sheet, view, at } => {
                let v = view
                    .as_ref()
                    .and_then(|id| ctx.views.iter().find(|x| &x.id == id));
                let k = sheet_of(&ctx, sheet, v);
                let Some(mo) = v.map(|v| &ctx.models[v.model]).or(ctx.models.first()) else {
                    continue;
                };
                let (_, i1) = forge_draw::sheet::inside(pages[k].w, pages[k].h);
                let corner = at.unwrap_or(Point2::new(
                    i1.x,
                    forge_draw::sheet::MARGIN + forge_draw::sheet::BAND + forge_draw::sheet::TITLE_H,
                ));
                let rows: Vec<Vec<String>> = mo
                    .bom
                    .iter()
                    .enumerate()
                    .map(|(i, (_, n, num, desc))| {
                        vec![(i + 1).to_string(), n.to_string(), num.clone(), desc.clone()]
                    })
                    .collect();
                let h = annot::table(
                    &mut pages[k],
                    corner,
                    &["ITEM", "QTY", "PART NUMBER", "DESCRIPTION"],
                    &[12.0, 12.0, 46.0, 110.0],
                    &rows,
                );
                items.push(ItemOut {
                    id: item.id.clone(),
                    sheet: k,
                    kind: "parts_list",
                    lo: Point2::new(corner.x - 180.0, corner.y),
                    hi: Point2::new(corner.x, corner.y + h),
                    sick: false,
                    value: None,
                    view: view.clone(),
                });
            }
            _ => {}
        }
    }
    let sheets = ctx.sheets.iter().map(|s| s.0.clone()).collect();
    Drawing {
        sheets,
        pages,
        views: ctx.views,
        items,
        errors,
        models: ctx.models,
        doc: d,
    }
}

/// A view's scale under its label, where it differs from the sheet's.
fn scale_note(ctx: &Ctx, v: &ViewOut) -> String {
    let sheet_scale = ctx
        .views
        .iter()
        .find(|x| x.sheet == v.sheet && x.kind == "base")
        .map(|x| x.scale);
    if sheet_scale.is_some_and(|s| (s - v.scale).abs() < 1e-12) {
        String::new()
    } else {
        forge_draw::sheet::scale_text(v.scale)
    }
}

/// A detail's fence: its circle (or rectangle) where it is on a view.
fn fence(page: &mut Page, place: &Place, c: Point2, r: f64, rect: Option<(f64, f64)>) {
    match rect {
        None => page.strokes.push(Stroke {
            pen: Pen::Thin,
            geom: Geom2::Conic {
                c,
                u: Vec2::new(r, 0.0),
                v: Vec2::new(0.0, r),
                t0: 0.0,
                t1: dmath::TAU,
            }
            .placed(place),
        }),
        Some((w, h)) => {
            let pts = [
                c + Vec2::new(-w, -h),
                c + Vec2::new(w, -h),
                c + Vec2::new(w, h),
                c + Vec2::new(-w, h),
            ];
            for k in 0..4 {
                page.strokes.push(Stroke {
                    pen: Pen::Thin,
                    geom: Geom2::Line {
                        a: pts[k],
                        b: pts[(k + 1) % 4],
                    }
                    .placed(place),
                });
            }
        }
    }
}

/// The JSON report: items, views, problems.
pub fn json(d: &Drawing, src: &str) -> forge_agent::json::Json {
    use forge_agent::json::Json;
    let pt = |p: Point2| Json::Arr(vec![p.x.into(), p.y.into()]);
    Json::obj(vec![
        ("ok", d.ok().into()),
        (
            "sheets",
            Json::Arr(d.sheets.iter().map(|s| Json::from(s.as_str())).collect()),
        ),
        (
            "views",
            Json::Arr(
                d.views
                    .iter()
                    .map(|v| {
                        Json::obj(vec![
                            ("id", v.id.as_str().into()),
                            ("kind", v.kind.into()),
                            ("sheet", v.sheet.into()),
                            ("parent", v.parent.as_deref().map_or(Json::Null, Json::from)),
                            ("scale", v.scale.into()),
                            ("lo", pt(v.lo)),
                            ("hi", pt(v.hi)),
                            ("pieces", v.pieces.len().into()),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "items",
            Json::Arr(
                d.items
                    .iter()
                    .map(|i| {
                        Json::obj(vec![
                            ("id", i.id.as_str().into()),
                            ("kind", i.kind.into()),
                            ("sheet", i.sheet.into()),
                            ("sick", i.sick.into()),
                            ("value", i.value.map_or(Json::Null, Json::from)),
                            ("lo", pt(i.lo)),
                            ("hi", pt(i.hi)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "problems",
            Json::Arr(d.errors.iter().map(|e| e.json(src)).collect()),
        ),
    ])
}

/// Every drawable edge of a view: its reference text, kind, whether seen, and where on the sheet
/// (`fg query D refs VIEW`), so an agent can pick what to dimension.
pub fn refs(d: &Drawing, view: &str) -> Option<forge_agent::json::Json> {
    use forge_agent::json::Json;
    let v = d.view(view)?;
    let model = &d.models[v.model].model;
    let mut seen: Vec<(Name, usize)> = Vec::new();
    let mut out = Vec::new();
    for p in &v.pieces {
        let (n, face) = match p.source {
            Source::Edge(n) => (n, false),
            Source::Outline(n) => (n, true),
        };
        if seen.contains(&(n, p.inst)) {
            continue;
        }
        seen.push((n, p.inst));
        let (body, _, occ) = &v.insts[p.inst];
        let kind = if face {
            "outline"
        } else {
            edges_named(body, n)
                .first()
                .map_or("edge", |&e| match body.edge(e).curve {
                    Curve::Line(_) => "line",
                    Curve::Circle(_) => "circle",
                    Curve::Ellipse(_) => "ellipse",
                    _ => "curve",
                })
        };
        let r = Ref {
            occ: if model.paths.len() > 1 {
                model.paths[*occ].clone()
            } else {
                String::new()
            },
            name: n,
            face,
            at: None,
            span: Span::new(0, 0),
        };
        let (a, b) = p.geom.bounds();
        out.push(Json::obj(vec![
            ("ref", r.text().into()),
            ("kind", kind.into()),
            ("visible", p.visible.into()),
            ("view_lo", Json::Arr(vec![a.x.into(), a.y.into()])),
            ("view_hi", Json::Arr(vec![b.x.into(), b.y.into()])),
        ]));
    }
    Some(Json::Arr(out))
}

/// The edge geometry of a ref on a view, for tests and the app's picking.
pub fn locate(d: &Drawing, view: &str, r: &Ref) -> Option<(Point2, Point2)> {
    let v = d.view(view)?;
    let x = resolve_ref(&d.models[v.model].model, v, r)?;
    Some(match x.feat {
        Feat::Point(p) => (v.place.point(p), v.place.point(p)),
        Feat::Line(a, b) => (v.place.point(a), v.place.point(b)),
        Feat::Circle { c, .. } => (v.place.point(c), v.place.point(c)),
    })
}
