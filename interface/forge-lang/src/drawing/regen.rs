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
    /// The part's volume (mm3), when it built whole.
    pub volume: Option<f64>,
    /// A sheet metal part's material and thickness, and its flat patterns.
    pub sheet: Option<(String, f64)>,
    pub patterns: Vec<forge_sheet::Pattern>,
    /// Bill of materials: part file, count, part number, description; in item order.
    pub bom: Vec<(String, usize, String, String)>,
    /// g/cm3, as the drawing gives it: an override of the material's.
    pub density: Option<f64>,
    /// The material by name, and as the library (project's, user's, ForgeCAD's) has it.
    pub material: Option<String>,
    pub found: Option<forge_material::Material>,
}

impl ModelOut {
    /// The part's mass in kg: its volume times its material's density (or the drawing's own).
    pub fn mass(&self) -> Option<f64> {
        let rho = match self.density {
            Some(g_cm3) => g_cm3 * 1000.0,
            None => self.found.as_ref()?.density,
        };
        Some(self.volume? * 1e-9 * rho)
    }
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
    /// Where the view was put on the sheet.
    pub at: Point2,
    /// Where the view clips its parent's lines (a detail's circle), in view coordinates.
    pub clip: Option<Fence>,
    /// Bands taken out (along x or y, from, to, in view coordinates) and how far apart the two
    /// sides then stand (view mm).
    pub breaks: Vec<(bool, f64, f64)>,
    pub gap: f64,
}

impl ViewOut {
    /// A point of the view (model mm on its plane) where it is on the sheet, across its breaks.
    pub fn map(&self, p: Point2) -> Point2 {
        self.place.point(self.closed(p))
    }

    /// Where a sheet point is on the view, its breaks opened again.
    pub fn unmap(&self, q: Point2) -> Point2 {
        let mut p = self.place.inverse_point(q);
        for &(x, lo, hi) in self.breaks.iter().rev() {
            let shift = hi - lo - self.gap;
            if x && p.x >= lo + self.gap {
                p.x += shift;
            } else if !x && p.y >= lo + self.gap {
                p.y += shift;
            }
        }
        p
    }

    /// A view point with the bands before it taken out.
    fn closed(&self, p: Point2) -> Point2 {
        let mut q = p;
        for &(x, lo, hi) in &self.breaks {
            let shift = hi - lo - self.gap;
            if x && p.x >= hi {
                q.x -= shift;
            } else if !x && p.y >= hi {
                q.y -= shift;
            }
        }
        q
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemOut {
    pub id: String,
    /// A linear dimension's direction of measure on the sheet.
    pub dir: Option<Vec2>,
    /// It sizes, or is attached to, a feature of size.
    pub fos: bool,
    /// Where its text is (a dimension's value): what a frame or datum attached to it hangs from.
    pub anchor: Point2,
    /// The text's half width and height.
    pub half: Vec2,
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
    /// The part files its assemblies' parts were loaded from, one for each load made: a file
    /// several models use, asked for different references, is loaded once for them all.
    pub part_loads: Vec<String>,
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
fn describe(path: &Path, file: &str, fence: Option<&Path>) -> (String, String) {
    let stem = Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().into_owned());
    // read only inside the fence, links resolved
    let inside = std::fs::canonicalize(path).is_ok_and(|p| fence.is_none_or(|f| p.starts_with(f)));
    let src = if inside {
        std::fs::read_to_string(path)
    } else {
        Err(std::io::ErrorKind::NotFound.into())
    };
    crate::deps::read(path, src.as_ref().ok().map(|s| s.as_bytes()));
    let Ok(src) = src else {
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

/// One component of a run on its own, in its own space: a body (`lid`), a component's first
/// call (`peg`), or a call by its key (`peg(d=6,h=30)`).
fn component_model(run: &crate::Run, file: &str, name: &str) -> Option<Model> {
    let a = run.asm.as_ref()?;
    let (key, sum) = a
        .part_docs
        .iter()
        .find(|(k, _)| k.as_str() == format!("#{name}"))
        .or_else(|| {
            a.part_docs
                .iter()
                .find(|(k, _)| k.starts_with(&format!("#{name}(")))
        })?;
    let bodies: Vec<_> = sum.solids.iter().map(|s| (s.2.clone(), Frame::WORLD)).collect();
    let n = bodies.len();
    Some(Model {
        bodies,
        paths: vec![String::new(); n],
        files: vec![format!("{file}{key}"); n],
        grounded: vec![],
        threads: vec![Default::default(); n],
    })
}

fn load(
    id: &str,
    file: &str,
    dir: &Path,
    pool: &Arc<crate::asm::PartPool>,
    wants: &Arc<crate::asm::Wants>,
    component: Option<&str>,
) -> Result<ModelOut, String> {
    let path = dir.join(file);
    let fence = crate::fence_for(Some(dir), None);
    if crate::outside(Some(dir), fence.as_deref(), file) {
        return Err("is outside this project, not read".into());
    }
    let src = std::fs::read_to_string(&path);
    crate::deps::read(&path, src.as_ref().ok().map(|s| s.as_bytes()));
    let src = src.map_err(|e| e.to_string())?;
    // a drawing shows the model's shape: its checks (overlap, expects) are its own report's
    let opts = crate::Options {
        build: true,
        render: true,
        overlap: false,
        expects: false,
        checks: false,
        // threads as the standards draw them: the envelope, and thin lines (`thread_lines`)
        cosmetic_threads: true,
        // its parts' meshes serve the overlap check and the expects, neither asked for here
        meshes: false,
        dir: path.parent().map(Path::to_path_buf),
        ..crate::Options::default()
    };
    // the drawing's models share their parts' builds (a subassembly also drawn on its own)
    let mut cache = crate::build::Cache::default();
    cache.asm = Some(Box::new(crate::asm::AsmCache::with_wants(
        pool.clone(),
        wants.clone(),
        false,
    )));
    let (run, _) = crate::run(&src, opts, cache);
    if !run.ok() {
        let e = &run.errors[0];
        return Err(format!("does not build: {}: {}", e.kind(), e.text()));
    }
    let model = match component {
        Some(c) => component_model(&run, file, c).ok_or_else(|| format!("has no component {c}"))?,
        None => Model::of(&run).ok_or("has no solid")?,
    };
    // the part as made: a sheet metal part's flat pattern is not part of it
    let volume = if run.asm.is_none() || component.is_some() {
        model
            .bodies
            .first()
            .and_then(|(b, _)| forge_topo::mass::mass(b).ok())
            .map(|m| m.volume)
    } else {
        None
    };
    let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let bom = match run.asm.as_ref().filter(|_| component.is_none()) {
        Some(a) => a
            .bom
            .iter()
            .map(|(f, n)| {
                let (num, desc) = describe(&resolve(&base, &model, f), f, fence.as_deref());
                (f.clone(), *n, num, desc)
            })
            .collect(),
        None => {
            let (num, desc) = describe(&path, file, fence.as_deref());
            vec![(file.to_string(), 1, num, desc)]
        }
    };
    Ok(ModelOut {
        id: id.to_string(),
        file: file.to_string(),
        model,
        volume,
        sheet: run
            .sheets
            .first()
            .map(|sh| (sh.material.clone().unwrap_or_default(), sh.sheet.gauge.thickness)),
        patterns: run.sheets.iter().filter_map(|sh| sh.pattern().ok()).collect(),
        bom,
        density: None,
        material: None,
        found: None,
    })
}

/// What an assembly model asks of each part file, added to `wants`: its document run up to its
/// parts, none of them built. A part model asks nothing.
fn wants_of(file: &str, dir: &Path, pool: &Arc<crate::asm::PartPool>, wants: &Arc<crate::asm::Wants>) {
    let path = dir.join(file);
    let Some(src) = crate::asm::read_capped(&path) else {
        return;
    };
    // any file with components is one (an `assembly` first line is optional)
    if !crate::syntax::parse(&src).is_ok_and(|p| crate::asm::has_components(&p)) {
        return;
    }
    let opts = crate::Options {
        build: true,
        render: true,
        overlap: false,
        expects: false,
        checks: false,
        cosmetic_threads: true,
        meshes: false,
        dir: path.parent().map(Path::to_path_buf),
        ..crate::Options::default()
    };
    let mut cache = crate::build::Cache::default();
    cache.asm = Some(Box::new(crate::asm::AsmCache::with_wants(
        pool.clone(),
        wants.clone(),
        true,
    )));
    let _ = crate::run(&src, opts, cache);
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

/// How far each occurrence moves when the assembly is drawn `t` of the way apart: the app's
/// exploded view's plan (`forge_lang::explode`), units by the part files' folders (frame, panels,
/// casters..) and subassemblies, hardware backing out along its own axis.
fn explode_offsets(model: &Model, t: f64) -> Vec<Vec3> {
    let n = model.bodies.len();
    if t == 0.0 || n < 2 {
        return vec![Vec3::ZERO; n];
    }
    let folder = |f: &str| {
        let p = Path::new(f);
        p.parent()
            .and_then(|d| d.file_name())
            .map_or(String::new(), |d| d.to_string_lossy().into_owned())
    };
    let mut groups: Vec<String> = (0..n).map(|k| folder(&model.files[k])).collect();
    groups.sort();
    groups.dedup();
    let mut subs: Vec<String> = Vec::new();
    let items: Vec<crate::explode::Item> = (0..n)
        .map(|k| {
            let (b, pose) = &model.bodies[k];
            let (x, y, z, o) = (pose.x().get(), pose.y().get(), pose.z().get(), pose.origin);
            let rows = [
                [x.x as f32, y.x as f32, z.x as f32, o.x as f32],
                [x.y as f32, y.y as f32, z.y as f32, o.y as f32],
                [x.z as f32, y.z as f32, z.z as f32, o.z as f32],
            ];
            let lb = b.faces_box();
            let path = &model.paths[k];
            let top = path.split('.').next().unwrap_or("").to_string();
            let sub = path
                .contains('.')
                .then(|| match subs.iter().position(|s| *s == top) {
                    Some(i) => i,
                    None => {
                        subs.push(top.clone());
                        subs.len() - 1
                    }
                });
            let word = folder(&model.files[k]);
            crate::explode::Item {
                rows,
                local: (
                    [lb.x.lo as f32, lb.y.lo as f32, lb.z.lo as f32],
                    [lb.x.hi as f32, lb.y.hi as f32, lb.z.hi as f32],
                ),
                group: groups.iter().position(|g| *g == word).unwrap_or(0),
                sub,
                ground: model.grounded.contains(&top),
                axis: is_hardware(&model.files[k]).then(|| {
                    if crate::explode::screw_like(&model.files[k]) {
                        -1.0
                    } else {
                        1.0
                    }
                }),
            }
        })
        .collect();
    let plan = crate::explode::plan(&items);
    crate::explode::at(&plan, t as f32)
        .into_iter()
        .map(|d| Vec3::new(d[0] as f64, d[1] as f64, d[2] as f64))
        .collect()
}

struct Ctx {
    dir: PathBuf,
    models: Vec<ModelOut>,
    sheets: Vec<(String, doc::SheetDef)>,
    views: Vec<ViewOut>,
    errors: Vec<DrawError>,
    letters: usize,
    /// Views' lines made ahead, all at once (`ahead`): taken by the view that asks the same.
    drawn: std::collections::HashMap<DrawKey, (Vec<Piece>, Vec<Stroke>)>,
    /// What every view's lines take from a part alone (its curves for a direction, its meshes),
    /// made once for the drawing.
    parts: forge_draw::Parts,
}

/// What a view's lines are made of: its direction, its options, and each body (by its `Arc`)
/// where it stands.
type DrawKey = ([u64; 9], bool, bool, Vec<(usize, [u64; 9])>);

fn draw_key(insts: &[(Arc<Body>, Frame, usize)], view: &View, tangent: bool, hidden: bool) -> DrawKey {
    let v = |u: forge_math::UnitVec3| {
        let u = u.get();
        [u.x, u.y, u.z]
    };
    let f = |f: &Frame| {
        let (o, x, z) = (f.origin, f.x().get(), f.z().get());
        [o.x, o.y, o.z, x.x, x.y, x.z, z.x, z.y, z.z].map(f64::to_bits)
    };
    let mut vb = [0u64; 9];
    for (k, x) in [v(view.toward), v(view.right), v(view.up)]
        .concat()
        .into_iter()
        .enumerate()
    {
        vb[k] = x.to_bits();
    }
    (
        vb,
        tangent,
        hidden,
        insts
            .iter()
            .map(|(b, p, _)| (Arc::as_ptr(b) as usize, f(p)))
            .collect(),
    )
}

/// A base or projected view's lines as the views pass will ask for them, worked out from the
/// document alone; a wrong guess only costs the time, the view draws its own then.
struct Plan {
    id: String,
    view: View,
    insts: Vec<(Arc<Body>, Frame, usize)>,
    model: usize,
    tangent: bool,
    hidden: bool,
    sheet: usize,
    at: Point2,
}

fn sheet_of(ctx: &Ctx, s: &Option<String>, parent: Option<&ViewOut>) -> usize {
    match s {
        Some(id) => ctx.sheets.iter().position(|x| &x.0 == id).unwrap_or(0),
        None => parent.map_or(0, |p| p.sheet),
    }
}

impl Ctx {
    /// The base and projected views' hidden lines, all at once on every core.
    fn ahead(&mut self, items: &[Item]) {
        let mut plans: Vec<Plan> = Vec::new();
        for item in items {
            let ItemKind::View(def) = &item.kind else { continue };
            let tangent = def.tangent.unwrap_or(false);
            let plan = match &def.kind {
                ViewKind::Base {
                    model,
                    toward,
                    up,
                    explode,
                } => (|| {
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
                    let t = view.toward.get();
                    let iso = t.x.abs() > 1e-9 && t.y.abs() > 1e-9 && t.z.abs() > 1e-9;
                    let sheet = match &def.sheet {
                        Some(id) => self.sheets.iter().position(|x| &x.0 == id).unwrap_or(0),
                        None => 0,
                    };
                    Some(Plan {
                        id: item.id.clone(),
                        view,
                        insts,
                        model: m,
                        tangent,
                        hidden: def.hidden.unwrap_or(!iso),
                        sheet,
                        at: def.at,
                    })
                })(),
                ViewKind::Projected { parent } => (|| {
                    let p = plans.iter().find(|x| &x.id == parent)?;
                    let third = self.sheets.get(p.sheet).is_none_or(|s| s.1.third_angle);
                    let (dx, dy) = (def.at.x - p.at.x, def.at.y - p.at.y);
                    let diagonal = dx.abs().min(dy.abs()) > 0.3 * dx.abs().max(dy.abs());
                    let view = if diagonal {
                        p.view.projected(dx, dy, third)?
                    } else if dx.abs() >= dy.abs() {
                        p.view.projected(dx, 0.0, third)?
                    } else {
                        p.view.projected(0.0, dy, third)?
                    };
                    let sheet = match &def.sheet {
                        Some(id) => self.sheets.iter().position(|x| &x.0 == id).unwrap_or(0),
                        None => p.sheet,
                    };
                    Some(Plan {
                        id: item.id.clone(),
                        view,
                        insts: p.insts.clone(),
                        model: p.model,
                        tangent,
                        hidden: def.hidden.unwrap_or(!diagonal),
                        sheet,
                        at: def.at,
                    })
                })(),
                _ => None,
            };
            plans.extend(plan);
        }
        let (models, parts) = (&self.models, &self.parts);
        let made = crate::par::map(plans.len(), |k| {
            let p = &plans[k];
            (
                draw(&p.insts, &p.view, p.tangent, p.hidden, parts),
                thread_lines(&models[p.model].model, &p.insts, &p.view, p.hidden),
            )
        });
        for (p, m) in plans.iter().zip(made) {
            self.drawn
                .insert(draw_key(&p.insts, &p.view, p.tangent, p.hidden), m);
        }
    }

    /// A view's hidden lines and thread lines: made ahead, or now.
    fn lines(
        &mut self,
        model: usize,
        insts: &[(Arc<Body>, Frame, usize)],
        view: &View,
        tangent: bool,
        hidden: bool,
    ) -> (Vec<Piece>, Vec<Stroke>) {
        if let Some(m) = self.drawn.remove(&draw_key(insts, view, tangent, hidden)) {
            return m;
        }
        (
            draw(insts, view, tangent, hidden, &self.parts),
            thread_lines(&self.models[model].model, insts, view, hidden),
        )
    }

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
                let (pieces, threads) = self.lines(m, &insts, &view, tangent, hidden);
                let s = def.scale.unwrap_or(1.0);
                let (lo, hi) = bounds(&pieces, &[]);
                let c = lo.lerp(hi, 0.5);
                let mut v = self.out(
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
                );
                v.extra = threads;
                Some(v)
            }
            ViewKind::Projected { parent } => {
                let p = self.views.iter().find(|v| &v.id == parent)?.clone();
                let third = self.sheets.get(p.sheet).is_none_or(|s| s.1.third_angle);
                // where the parent was put, not where its lines happen to end: a break or crop of
                // it does not turn its children
                let pc = p.at;
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
                let (pieces, threads) = self.lines(p.model, &p.insts, &view, tangent, hidden);
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
                let mut v = self.out(
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
                );
                v.extra = threads;
                Some(v)
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
                        // hardware is not cut, but goes with the half taken away when it is all in it
                        let bb = b.faces_box();
                        let gone = [bb.x.lo, bb.x.hi]
                            .iter()
                            .flat_map(|&x| [bb.y.lo, bb.y.hi].map(|y| (x, y)))
                            .flat_map(|(x, y)| [bb.z.lo, bb.z.hi].map(|z| Point3::new(x, y, z)))
                            .all(|q| {
                                let q = p.view.project(pose.point_to_world(q));
                                line.windows(2).all(|w| (q - w[0]).dot(away) > 0.0)
                            });
                        if !gone {
                            insts.push((b.clone(), *pose, *occ));
                        }
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
                let pieces = draw(&insts, &view, tangent, hidden, &self.parts);
                let mut extra = thread_lines(mo, &insts, &view, hidden);
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
                let Some(pattern) = self.models[m].patterns.first().cloned() else {
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
            at: def.at,
            clip: None,
            breaks: vec![],
            gap: 0.0,
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

fn draw(
    insts: &[(Arc<Body>, Frame, usize)],
    view: &View,
    tangent: bool,
    hidden: bool,
    parts: &forge_draw::Parts,
) -> Vec<Piece> {
    for (b, ..) in insts {
        parts.hold(b);
    }
    let is: Vec<Instance> = insts
        .iter()
        .map(|(b, pose, _)| Instance { body: b, pose: *pose })
        .collect();
    let opts = forge_draw::Options { tangent, chord: 0.01 };
    let mut p = forge_draw::hlr_with(&is, view, &opts, parts);
    if !hidden {
        p.retain(|x| x.visible);
    }
    p
}

/// The threads of the bodies in a view drawn as ISO 6410 / ASME Y14.6 draw them, the parts
/// themselves plain to the thread's envelope: across the axis, a thin line at the minor
/// diameter of a screw (the major of a nut) and a thick line where the thread ends; along it,
/// three quarters of a thin circle. Each line is seen or hidden among the parts as an edge is.
fn thread_lines(
    model: &Model,
    insts: &[(Arc<Body>, Frame, usize)],
    view: &View,
    hidden: bool,
) -> Vec<Stroke> {
    const N: usize = 24;
    let toward = view.toward.get();
    let mut lines: Vec<Vec<Point3>> = Vec::new();
    let mut pens: Vec<Pen> = Vec::new();
    for (_, pose, occ) in insts {
        let Some(ts) = model.threads.get(*occ) else {
            continue;
        };
        for t in ts.iter() {
            let at = pose.compose(&t.at);
            let (o, z) = (at.origin, at.z().get());
            let (big, small) = (t.major, t.minor);
            // off the surface toward the eye by a hair, so a line on a face is not hidden by it
            let lift = toward * (1e-6 * big.max(1.0));
            let across = toward - z * z.dot(toward);
            match across.normalize() {
                Some(n) if across.length() > 0.2 => {
                    let (n, u) = (n.get(), z.cross(n.get()));
                    // the thin line: on the screw's near face, or the nut's bore wall at mid depth
                    let (r, front) = if t.internal {
                        (big, 0.0)
                    } else {
                        (small, (big * big - small * small).max(0.0).sqrt())
                    };
                    for side in [-1.0, 1.0] {
                        lines.push(
                            (0..=N)
                                .map(|k| {
                                    o + z * (t.length * k as f64 / N as f64)
                                        + u * (side * r)
                                        + n * front
                                        + lift
                                })
                                .collect(),
                        );
                        pens.push(Pen::Thin);
                    }
                    // the thread's end, across its major diameter
                    let end = o + z * t.length;
                    lines.push(
                        (0..=N)
                            .map(|k| {
                                let s = big * (2.0 * k as f64 / N as f64 - 1.0);
                                let h = if t.internal {
                                    0.0
                                } else {
                                    (big * big - s * s).max(0.0).sqrt()
                                };
                                end + u * s + n * h + lift
                            })
                            .collect(),
                    );
                    pens.push(Pen::Visible);
                }
                _ => {
                    // seen end on: the end nearer the eye, three quarters of the circle
                    let c = if z.dot(toward) < 0.0 { o } else { o + z * t.length };
                    let r = if t.internal { big } else { small };
                    let (u, v) = (view.right.get(), view.up.get());
                    lines.push(
                        (0..=3 * N)
                            .map(|k| {
                                let a = dmath::FRAC_PI_2 * k as f64 / N as f64;
                                let (sn, cs) = dmath::sin_cos(a);
                                c + u * (r * cs) + v * (r * sn) + lift
                            })
                            .collect(),
                    );
                    pens.push(Pen::Thin);
                }
            }
        }
    }
    let is: Vec<Instance> = insts
        .iter()
        .map(|(b, pose, _)| Instance { body: b, pose: *pose })
        .collect();
    let mut out = Vec::new();
    for (runs, pen) in forge_draw::seen_runs(&is, view, &lines).into_iter().zip(pens) {
        for (geom, seen) in runs {
            match (seen, hidden) {
                (true, _) => out.push(Stroke { pen, geom }),
                (false, true) => out.push(Stroke {
                    pen: Pen::Hidden,
                    geom,
                }),
                (false, false) => {}
            }
        }
    }
    out
}

/// What a reference is on a view: a point, a line, or a circle (centre and radius in model mm).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Feat {
    Point(Point2),
    Line(Point2, Point2),
    Circle {
        c: Point2,
        r: f64,
        a: Point2,
        b: Point2,
        /// Seen edge-on: the line it is drawn as.
        edge_on: Option<(Point2, Point2)>,
    },
}

/// A circle seen edge-on: the line it is drawn as on the view.
fn edge_on(c: &forge_geom::Circle, pose: &Frame, view: &View) -> Option<(Point2, Point2)> {
    let n = pose.vec_to_world(c.frame.z().get());
    if n.dot(view.toward.get()).abs() > 1e-9 {
        return None;
    }
    let o = view.project(pose.point_to_world(c.frame.origin));
    let w = view.project_vec(pose.vec_to_world(c.frame.x().get()) * c.radius);
    let u = view.project_vec(pose.vec_to_world(c.frame.y().get()) * c.radius);
    let half = if w.length() >= u.length() { w } else { u };
    Some((o - half, o + half))
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
    /// A feature of size: a hole or shaft (circle edge, cylinder face), what MMC/LMC can apply to.
    fos: bool,
    /// Drawn with a seen (not hidden) line.
    seen: bool,
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
        let fos = matches!(
            body.face(f).surface,
            Surface::Cylinder(_) | Surface::Sphere(_) | Surface::Cone(_)
        );
        return Some(Resolved {
            feat: Feat::Point(pr(b.center())),
            shown,
            inst,
            fos,
            seen: true,
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
                edge_on: edge_on(c, pose, &v.view),
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
    let fos = matches!(edge.curve, Curve::Circle(_));
    let seen = es.iter().any(|&e| {
        let n = body.edge(e).name;
        v.pieces
            .iter()
            .any(|p| p.inst == inst && p.visible && p.source == Source::Edge(n))
    });
    Some(Resolved {
        feat,
        shown,
        inst,
        fos,
        seen,
    })
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
    stack: &mut std::collections::HashMap<String, f64>,
) -> Option<()> {
    let v = ctx.views.iter().find(|x| x.id == a.view)?;
    let mo = &ctx.models[v.model];
    let model = &mo.model;
    let mut an = Annotator {
        st: Style::default(),
        page,
    };
    let s = v.scale;
    let sp = |p: Point2| v.map(p);
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
                } else if !x.seen {
                    errors.push(warn(
                        Problem::HiddenOnly {
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
    let std = ctx
        .sheets
        .get(v.sheet)
        .map_or(forge_draw::gdt::Standard::Asme, |s| s.1.standard);
    let places = a.places.unwrap_or(2);
    // the value as written: symbol, number, unit, then its tolerance
    let text = |sym: &str, val: f64, unit: &str, errors: &mut Vec<DrawError>| -> String {
        match tol_text(sym, val, unit, places, a) {
            Ok(t) => t,
            Err(why) => {
                errors.push(fail(
                    Problem::BadValue {
                        func: "fit",
                        arg: "fit".into(),
                        why,
                    },
                    item.span,
                ));
                format!("{sym}{}{unit}", number(val, places))
            }
        }
    };
    let pen = Pen::Thin;
    let mut value = None;
    let mut fos = false;
    let mut measured_dir: Option<Vec2> = None;
    let kind: &'static str = match &a.kind {
        AnnotKind::Linear { a: ra, b: rb, dir } => {
            let fa = get(ra, "1", false, errors);
            let fb = match rb {
                Some(rb) => get(rb, "2", false, errors).map(Some),
                None => Some(None),
            };
            if let (Some(fa), Some(fb)) = (fa, fb) {
                // a circle seen edge-on is the line it is drawn as
                let flat = |f: Feat| match f {
                    Feat::Circle {
                        edge_on: Some((p, q)),
                        ..
                    } => Feat::Line(p, q),
                    f => f,
                };
                let at_view = a.at.unwrap_or(Point2::ORIGIN);
                let (p, q, d) = match (flat(fa.feat), fb.map(|x| flat(x.feat))) {
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
                        // where the two lines overlap along their length, as near the dimension's
                        // place as can be: the extension lines land on both
                        let t = |p: Point2| (p - a0).dot(u);
                        let (lo, hi) = (
                            t(a0).min(t(a1)).max(t(b0).min(t(b1))),
                            t(a0).max(t(a1)).min(t(b0).max(t(b1))),
                        );
                        let want = if a.at.is_some() {
                            t(at_view)
                        } else {
                            0.5 * (t(a0) + t(a1))
                        };
                        let along = if lo <= hi {
                            want.clamp(lo, hi)
                        } else {
                            0.5 * (lo + hi)
                        };
                        let m = a0 + u * along;
                        let foot = b0 + u * (m - b0).dot(u);
                        fos = true;
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
                // the model distance, whatever breaks the view has
                let val = (q - p).dot(dir).abs();
                value = Some(val);
                measured_dir = Some(dir);
                let at = a.at.map_or(ps.lerp(qs, 0.5) + dir.perp() * 10.0, sp);
                let t = text("", val, "", errors);
                annot::linear(an.page, pen, &an.st, ps, qs, dir, at, &t);
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
                        &text("", ang, "\u{b0}", errors),
                    );
                }
            }
            "angle"
        }
        AnnotKind::Radius { e } | AnnotKind::Diameter { e } => {
            let diameter = matches!(a.kind, AnnotKind::Diameter { .. });
            fos = true;
            if let Some(f) = get(e, "1", true, errors) {
                match f.feat {
                    Feat::Circle { c, r, .. } => {
                        let val = if diameter { 2.0 * r } else { r };
                        value = Some(val);
                        let at = a.at.map_or(sp(c) + Vec2::new(1.0, 1.0) * (r * s + 8.0), sp);
                        let t = if diameter {
                            text("\u{2300}", val, "", errors)
                        } else {
                            text("R", val, "", errors)
                        };
                        annot::radial(an.page, pen, &an.st, sp(c), r * s, at, &t);
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
            fos = true;
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
                let mut pts: Vec<Point2> = fs
                    .iter()
                    .flatten()
                    .filter(|f| f.shown)
                    .map(|f| sp(f.feat.anchor()))
                    .collect();
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
                            let val = (v.unmap(p) - v.unmap(o)).dot(u);
                            let out = (foot - p).normalize().unwrap_or(u.perp());
                            let tat = foot + out * 1.5;
                            let angle = if dir == "vertical" { 0.0 } else { 90.0 };
                            let t = fmt_len(val);
                            if a.tol == Some(doc::TolSpec::Basic) {
                                // a basic ordinate: its value boxed, the box along the text
                                let (h, w) = (an.st.text_h, forge_draw::page::text_width(&t, an.st.text_h));
                                let (along, up) = if angle == 0.0 {
                                    (Vec2::X, Vec2::Y)
                                } else {
                                    (Vec2::Y, Vec2::new(-1.0, 0.0))
                                };
                                let c = [
                                    tat - along * 0.8 - up * 0.8,
                                    tat + along * (w + 0.8) - up * 0.8,
                                    tat + along * (w + 0.8) + up * (h + 0.8),
                                    tat - along * 0.8 + up * (h + 0.8),
                                ];
                                for k in 0..4 {
                                    annot::stroke(an.page, pen, c[k], c[(k + 1) % 4]);
                                }
                            }
                            an.page.texts.push(Text {
                                at: tat
                                    + out
                                        * if a.tol == Some(doc::TolSpec::Basic) {
                                            0.8
                                        } else {
                                            0.0
                                        },
                                h: an.st.text_h,
                                angle,
                                align: Align::Left,
                                text: t,
                                pen,
                            });
                        }
                    }
                    SetStyle::Baseline => {
                        for (k, p) in pts.iter().enumerate() {
                            let line = at + u.perp() * (-(k as f64) * 7.0 * (at - o).dot(u.perp()).signum());
                            let val = (v.unmap(*p) - v.unmap(o)).dot(u).abs();
                            annot::linear(an.page, pen, &an.st, o, *p, u, line, &fmt_len(val));
                        }
                    }
                    SetStyle::Chain => {
                        let mut prev = o;
                        for p in &pts {
                            let val = (v.unmap(*p) - v.unmap(prev)).dot(u).abs();
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
                if t1 - t0 < dmath::TAU - 1e-6 || done.iter().any(|q| q.distance(c) < 1e-6) {
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
                match (fa.feat, fb.feat) {
                    // two edges: the line halfway between them (a bisector), as long as they run
                    (Feat::Line(a0, a1), Feat::Line(b0, b1)) => {
                        let (b0, b1) = if (b0 - a0).length() + (b1 - a1).length()
                            <= (b1 - a0).length() + (b0 - a1).length()
                        {
                            (b0, b1)
                        } else {
                            (b1, b0)
                        };
                        annot::centerline(an.page, sp(a0.lerp(b0, 0.5)), sp(a1.lerp(b1, 0.5)), 3.0);
                    }
                    _ => annot::centerline(an.page, sp(fa.feat.anchor()), sp(fb.feat.anchor()), 3.0),
                }
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
                    stack.insert(format!("balloon:{}:{n}", v.model), 1.0);
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
        AnnotKind::Fcf {
            to,
            rows,
            composite,
            all_around,
            between,
        } => {
            let h = an.st.text_h;
            let (tip, target_fos, hang) = attach_point(to, &mut get, errors, items, v, a.at.map(sp));
            fos = target_fos;
            for (k, row) in rows.iter().enumerate() {
                if !target_fos && row.tol.mods.iter().any(|m| m.needs_size()) {
                    let kept: Vec<String> = cell_words(&row.tol)
                        .into_iter()
                        .filter(|w| w != "M" && w != "L")
                        .collect();
                    errors.push(DrawError {
                        problem: Problem::Gdt {
                            item: item.id.clone(),
                            rule: doc::GdtRule::ModifierNeedsSize,
                            detail: format!("row {}: MMC/LMC applies to a feature of size (a hole, a pin, a width), not this one", k + 1),
                        },
                        span: item.span,
                        warning: false,
                        fixes: if rows.len() == 1 {
                            vec![DrawOp::SetArg { item: item.id.clone(), arg: "3".into(), value: format!("\"{}\"", kept.join(" ")) }]
                        } else {
                            vec![]
                        },
                    });
                }
            }
            // on a dimension: directly under its text (Y14.5 Fig. 3-23), stacked under any frame
            // already there; on the model: where placed, with a leader
            let under = |id: &str, anchor: Point2, half: Vec2| {
                stack.get(id).copied().unwrap_or(anchor.y - half.y - 0.6)
            };
            let at = match (&to, hang, tip) {
                (doc::Attach::Item(id), Some((anchor, half)), _) => Point2::new(
                    anchor.x - half.x,
                    under(id, anchor, half) - 2.0 * h * rows.len() as f64,
                ),
                (_, _, Some(t)) => a.at.map_or(t + Vec2::new(12.0, 10.0), sp),
                _ => a.at.map_or(at_sheet, sp),
            };
            forge_draw::gdt::frame(an.page, at, h, rows, *composite, std);
            if let doc::Attach::Item(id) = &to {
                stack.insert(id.clone(), at.y);
            }
            let top = Point2::new(
                at.x + forge_draw::gdt::frame_width(h, rows, std),
                at.y + 2.0 * h * rows.len() as f64,
            );
            if let Some(tip) = tip {
                let mid_y = 0.5 * (at.y + top.y);
                let from = if tip.x < at.x {
                    Point2::new(at.x, mid_y)
                } else {
                    Point2::new(top.x, mid_y)
                };
                let elbow = Point2::new(from.x + if tip.x < at.x { -4.0 } else { 4.0 }, mid_y);
                annot::stroke(an.page, pen, from, elbow);
                annot::stroke(an.page, pen, elbow, tip);
                annot::arrow(an.page, pen, tip, tip - elbow, an.st.arrow);
                if *all_around {
                    an.page.strokes.push(Stroke {
                        pen,
                        geom: Geom2::Conic {
                            c: elbow,
                            u: Vec2::new(1.2, 0.0),
                            v: Vec2::new(0.0, 1.2),
                            t0: 0.0,
                            t1: dmath::TAU,
                        },
                    });
                }
            }
            if let Some((p, q)) = between {
                let y = at.y - 1.6 * h;
                let w = forge_draw::page::text_width(p, h);
                an.page.texts.push(Text {
                    at: Point2::new(at.x, y),
                    h,
                    angle: 0.0,
                    align: Align::Left,
                    text: p.clone(),
                    pen,
                });
                let (x0, x1) = (at.x + w + 1.0, at.x + w + 1.0 + 3.0 * h);
                annot::stroke(
                    an.page,
                    pen,
                    Point2::new(x0, y + 0.5 * h),
                    Point2::new(x1, y + 0.5 * h),
                );
                annot::arrow(
                    an.page,
                    pen,
                    Point2::new(x0, y + 0.5 * h),
                    Vec2::new(-1.0, 0.0),
                    1.5,
                );
                annot::arrow(an.page, pen, Point2::new(x1, y + 0.5 * h), Vec2::X, 1.5);
                an.page.texts.push(Text {
                    at: Point2::new(x1 + 1.0, y),
                    h,
                    angle: 0.0,
                    align: Align::Left,
                    text: q.clone(),
                    pen,
                });
            }
            "fcf"
        }
        AnnotKind::Datum { to, letter } => {
            let h = an.st.text_h;
            let (tip, target_fos, hang) = attach_point(to, &mut get, errors, items, v, a.at.map(sp));
            fos = target_fos;
            // on a dimension: hung under its text, or under the frames already there
            let foot = match (&to, tip, hang) {
                (_, Some(t), _) => Some(t),
                (doc::Attach::Item(id), None, Some((anchor, half))) => Some(Point2::new(
                    anchor.x - half.x + 1.5 * h,
                    stack.get(id).copied().unwrap_or(anchor.y - half.y - 0.3),
                )),
                _ => None,
            };
            if let Some(foot) = foot {
                let at = match (&to, a.at) {
                    (doc::Attach::Ref(_), Some(p)) => sp(p),
                    _ => foot - Vec2::new(0.0, 2.6 * h),
                };
                forge_draw::gdt::datum_symbol(an.page, letter, at, foot, h);
                if let doc::Attach::Item(id) = &to {
                    stack.insert(id.clone(), at.y - h);
                }
            }
            "datum"
        }
        AnnotKind::DatumTarget {
            r,
            label,
            size,
            kind,
            area,
        } => {
            if let Some(f) = get(r, "1", false, errors) {
                let on = near_on(&f.feat, v, at_sheet);
                let at = a.at.map_or(on + Vec2::new(12.0, 12.0), sp);
                forge_draw::gdt::datum_target(an.page, at, on, label, size, *kind, area * s, an.st.text_h);
            }
            "datum_target"
        }
        AnnotKind::Surface {
            r,
            value: val,
            process,
            lay,
        } => {
            if let Some(f) = get(r, "1", false, errors) {
                let on = near_on(&f.feat, v, at_sheet);
                let up = match f.feat {
                    Feat::Line(p, q) => {
                        let n = (sp(q) - sp(p)).perp();
                        if n.dot(at_sheet - on) >= 0.0 { n } else { n * -1.0 }
                    }
                    _ => at_sheet - on,
                };
                forge_draw::gdt::surface_texture(an.page, on, up, val, *process, lay, an.st.text_h, std);
            }
            "surface"
        }
        AnnotKind::Weld { r, weld } => {
            if let Some(f) = get(r, "1", false, errors) {
                let tip = near_on(&f.feat, v, at_sheet);
                let at = a.at.map_or(tip + Vec2::new(-15.0, 10.0), sp);
                forge_draw::gdt::weld(an.page, tip, at, weld, an.st.text_h, std);
            }
            "weld"
        }
        AnnotKind::Edge { r, upper, lower } => {
            if let Some(f) = get(r, "1", false, errors) {
                let on = near_on(&f.feat, v, at_sheet);
                let at = a.at.map_or(on + Vec2::new(8.0, 8.0), sp);
                forge_draw::gdt::edge(an.page, at, on, upper, lower, an.st.text_h);
            }
            "edge"
        }
        AnnotKind::AutoBalloons { from } => {
            if model.paths.len() <= 1 {
                errors.push(fail(
                    Problem::NotHere {
                        item: item.id.clone(),
                        why: "balloons number an assembly's parts".into(),
                    },
                    item.span,
                ));
            }
            let m = from
                .as_ref()
                .and_then(|f| ctx.views.iter().find(|x| &x.id == f))
                .map_or(v.model, |x| x.model);
            auto_balloons(&ctx.models[m], m, mo, v, &mut an, stack);
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
    // its box: what it drew, or where it was placed if it drew nothing
    let (mut lo, mut hi) = if an.page.strokes.len() > before.0 {
        (
            Point2::new(f64::INFINITY, f64::INFINITY),
            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        )
    } else {
        (at_sheet, at_sheet)
    };
    for st in &an.page.strokes[before.0..] {
        let (a, b) = st.geom.bounds();
        lo = Point2::new(lo.x.min(a.x), lo.y.min(a.y));
        hi = Point2::new(hi.x.max(b.x), hi.y.max(b.y));
    }
    // a dimension's text: where a frame or datum symbol attached to it hangs
    let new_texts = &an.page.texts[before.1.min(an.page.texts.len())..];
    let (anchor, half) = match new_texts.first() {
        Some(t) => {
            let (mut x0, mut x1, mut y0, mut y1) =
                (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
            for t in new_texts {
                let w = forge_draw::page::text_width(&t.text, t.h);
                let x = match t.align {
                    Align::Left => t.at.x,
                    Align::Centre => t.at.x - 0.5 * w,
                    Align::Right => t.at.x - w,
                };
                x0 = x0.min(x);
                x1 = x1.max(x + w);
                y0 = y0.min(t.at.y);
                y1 = y1.max(t.at.y + t.h);
            }
            let _ = t;
            (
                Point2::new(0.5 * (x0 + x1), 0.5 * (y0 + y1)),
                Vec2::new(0.5 * (x1 - x0), 0.5 * (y1 - y0)),
            )
        }
        None => (at_sheet, Vec2::ZERO),
    };
    items.push(ItemOut {
        id: item.id.clone(),
        dir: measured_dir,
        fos,
        anchor,
        half,
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

/// The words of a tolerance cell, as `gdt::parse_tol` reads them.
fn cell_words(t: &forge_draw::gdt::Tol) -> Vec<String> {
    use forge_draw::gdt::Modifier as M;
    let mut w = Vec::new();
    if t.dia {
        w.push(if t.sphere { "Sdia".into() } else { "dia".into() });
    }
    w.push(number(t.value, 4));
    for m in &t.mods {
        w.push(match m {
            M::Mmc => "M".into(),
            M::Lmc => "L".into(),
            M::Rfs => "S".into(),
            M::Projected(None) => "P".into(),
            M::Projected(Some(x)) => format!("P{}", number(*x, 3)),
            M::Free => "F".into(),
            M::Tangent => "T".into(),
            M::Unequal(x) => format!("U{}", number(*x, 3)),
            M::Statistical => "ST".into(),
        });
    }
    w
}

/// The point of a feature on the sheet nearest `q`: on a line, the foot; on a circle, toward q.
fn near_on(f: &Feat, v: &ViewOut, q: Point2) -> Point2 {
    let sp = |p: Point2| v.map(p);
    match *f {
        Feat::Point(p) => sp(p),
        Feat::Line(a, b) => {
            let (a, b) = (sp(a), sp(b));
            let ab = b - a;
            let t = ((q - a).dot(ab) / ab.dot(ab).max(1e-300)).clamp(0.0, 1.0);
            a + ab * t
        }
        Feat::Circle { c, r, .. } => {
            let c = sp(c);
            c + (q - c).normalize().unwrap_or(Vec2::X) * (r * v.scale)
        }
    }
}

/// What resolves an annotation's references (and marks it sick when one is gone).
type Resolver<'a> = dyn FnMut(&Ref, &str, bool, &mut Vec<DrawError>) -> Option<Resolved> + 'a;

/// Where a GD&T symbol hangs: (a leader's end on the model, whether that is a feature of size,
/// the attached dimension's text).
fn attach_point(
    to: &doc::Attach,
    get: &mut Resolver,
    errors: &mut Vec<DrawError>,
    items: &[ItemOut],
    v: &ViewOut,
    at: Option<Point2>,
) -> (Option<Point2>, bool, Option<(Point2, Vec2)>) {
    match to {
        doc::Attach::Ref(r) => match get(r, "1", false, errors) {
            Some(f) => {
                let q = at.unwrap_or(v.hi);
                (Some(near_on(&f.feat, v, q)), f.fos, None)
            }
            None => (None, false, None),
        },
        doc::Attach::Item(id) => match items.iter().find(|i| &i.id == id) {
            Some(i) => (None, i.fos, Some((i.anchor, i.half))),
            None => (None, false, None),
        },
    }
}

/// A signed deviation as written: `+0.021`, `-0.007`, `0`.
fn signed(x: f64, places: usize) -> String {
    let t = number(x, places.max(3));
    if x > 0.0 && t != "0" { format!("+{t}") } else { t }
}

/// A dimension's text: `prefix SYM value UNIT suffix` with its tolerance (in the markup
/// `annot::dim_text` draws: stacked deviations or limits, the basic box).
fn tol_text(sym: &str, v: f64, unit: &str, places: usize, a: &AnnotDef) -> Result<String, String> {
    let base = match &a.text {
        Some(t) => t.clone(),
        None => format!("{sym}{}{unit}", number(v, places)),
    };
    let base = format!("{}{base}{}", a.prefix, a.suffix);
    let (s, u) = (annot::STACK, annot::UNDER);
    Ok(match &a.tol {
        None => base,
        Some(doc::TolSpec::Symmetric(t)) => format!("{base} \u{b1}{}", number(*t, places.max(2))),
        Some(doc::TolSpec::Deviation(hi, lo)) => {
            format!("{base}{s}{}{u}{}", signed(*hi, places), signed(*lo, places))
        }
        Some(doc::TolSpec::Limits(hi, lo)) => {
            let p = places.max(3);
            format!(
                "{}{s}{sym}{}{unit}{u}{sym}{}{unit}",
                a.prefix,
                number(v + hi, p),
                number(v + lo, p)
            )
        }
        Some(doc::TolSpec::Max) => format!("{base} MAX"),
        Some(doc::TolSpec::Min) => format!("{base} MIN"),
        Some(doc::TolSpec::Basic) => format!("{}{base}", annot::BOXED),
        Some(doc::TolSpec::Reference) => format!("({base})"),
        Some(doc::TolSpec::Fit(c)) => {
            let (hi, lo) = forge_draw::fits::deviations(v, c).map_err(|e| e.text())?;
            format!("{base} {c}{s}{}{u}{}", signed(hi, 3), signed(lo, 3))
        }
    })
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

/// Where a balloon on an occurrence points: the middle of its longest drawn piece. Pieces as long
/// to within a nanometre (an occurrence's copies of one edge) are one length, the last of them
/// taken: which is longest does not turn on the last bits of where hidden lines were cut.
fn balloon_tip(model: &Model, v: &ViewOut, occ: &str) -> Option<Point2> {
    let k = model.paths.iter().position(|p| p == occ)?;
    let inst = v.insts.iter().position(|x| x.2 == k)?;
    let seen = || v.pieces.iter().filter(|p| p.inst == inst && p.visible);
    let most = seen().map(|p| p.geom.length()).fold(f64::NEG_INFINITY, f64::max);
    let p = seen().rfind(|p| p.geom.length() >= most - 1e-6)?;
    let (t0, t1) = p.geom.range();
    Some(v.map(p.geom.at(0.5 * (t0 + t1))))
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

/// A balloon on each part of `num`'s BOM seen in the view and not already ballooned (in any view
/// numbered by that BOM), Inventor's Auto Balloon "around": the balloons in tidy columns and rows
/// beside the view's sides, each on the side nearest its part, evenly spaced and kept in the
/// order of their parts along the side, so no two leaders cross. Parts match `num`'s by file name,
/// so a view of a subassembly can carry the top assembly's item numbers.
fn auto_balloons(
    num: &ModelOut,
    m: usize,
    mo: &ModelOut,
    v: &ViewOut,
    an: &mut Annotator,
    stack: &mut std::collections::HashMap<String, f64>,
) {
    let mut tips: Vec<(Point2, String)> = Vec::new();
    for (i, (file, ..)) in num.bom.iter().enumerate() {
        let key = format!("balloon:{m}:{}", i + 1);
        if stack.contains_key(&key) {
            continue;
        }
        let name = Path::new(file).file_name();
        let tip = (0..mo.model.paths.len())
            .filter(|&k| {
                (&mo.model.files[k] == file || Path::new(&mo.model.files[k]).file_name() == name)
                    && v.insts.iter().any(|x| x.2 == k)
            })
            .find_map(|k| balloon_tip(&mo.model, v, &mo.model.paths[k]));
        if let Some(t) = tip {
            tips.push((t, (i + 1).to_string()));
            stack.insert(key, 1.0);
        }
    }
    let r = 0.5 * an.st.text_h.max(forge_draw::page::text_width("88", an.st.text_h)) + 2.0;
    let step = 2.0 * r + 1.5;
    let off = 10.0 + r;
    let (lo, hi) = (v.lo, v.hi);
    // the four sides: where their line is, which way it runs, from where to where
    let sides = [
        (Point2::new(lo.x - off, lo.y), Vec2::Y, hi.y - lo.y),
        (Point2::new(hi.x + off, lo.y), Vec2::Y, hi.y - lo.y),
        (Point2::new(lo.x, hi.y + off), Vec2::X, hi.x - lo.x),
        (Point2::new(lo.x, lo.y - off), Vec2::X, hi.x - lo.x),
    ];
    // a labelled view keeps the side under it for its label
    let open = if v.label.is_empty() { 4 } else { 3 };
    let room: Vec<usize> = sides
        .iter()
        .enumerate()
        .map(|(k, s)| {
            if k < open {
                ((s.2 + 2.0 * off) / step).floor().max(1.0) as usize
            } else {
                0
            }
        })
        .collect();
    let dist = |t: Point2, k: usize| {
        let (o, d, _) = sides[k];
        (t - o).dot(d.perp()).abs()
    };
    // each to its nearest side with room, the most decided first
    let mut order: Vec<usize> = (0..tips.len()).collect();
    order.sort_by(|&a, &b| {
        let best = |i: usize| {
            (0..open)
                .map(|k| dist(tips[i].0, k))
                .fold(f64::INFINITY, f64::min)
        };
        best(a).total_cmp(&best(b))
    });
    let mut on: Vec<Vec<usize>> = vec![vec![]; 4];
    for i in order {
        let mut ks: Vec<usize> = (0..open).collect();
        ks.sort_by(|&a, &b| dist(tips[i].0, a).total_cmp(&dist(tips[i].0, b)));
        let k = ks
            .iter()
            .copied()
            .find(|&k| on[k].len() < room[k])
            .unwrap_or(ks[0]);
        on[k].push(i);
    }
    for (k, list) in on.iter_mut().enumerate() {
        let (o, d, len) = sides[k];
        let along = |t: Point2| (t - o).dot(d);
        list.sort_by(|&a, &b| along(tips[a].0).total_cmp(&along(tips[b].0)));
        // each as near its part as the spacing lets it be, all within the side
        let n = list.len();
        let (start, end) = (-off + r, len + off - r);
        let mut pos: Vec<f64> = list.iter().map(|&i| along(tips[i].0).clamp(start, end)).collect();
        for q in 1..n {
            pos[q] = pos[q].max(pos[q - 1] + step);
        }
        if let Some(&last) = pos.last()
            && last > end
        {
            let back = last - end;
            for p in pos.iter_mut() {
                *p -= back;
            }
            for q in (0..n.saturating_sub(1)).rev() {
                pos[q] = pos[q].min(pos[q + 1] - step);
            }
        }
        // tips at different depths can still cross: swap the places of any two that do (each
        // swap shortens the leaders, so it ends)
        let mut place: Vec<Point2> = pos.iter().map(|&t| o + d * t).collect();
        for _ in 0..n * n {
            let mut swapped = false;
            for a in 0..n {
                for b in a + 1..n {
                    if crosses(tips[list[a]].0, place[a], tips[list[b]].0, place[b]) {
                        place.swap(a, b);
                        swapped = true;
                    }
                }
            }
            if !swapped {
                break;
            }
        }
        for (q, &i) in list.iter().enumerate() {
            annot::balloon(an.page, Pen::Thin, &an.st, tips[i].0, place[q], &tips[i].1);
        }
    }
}

/// Whether segments a0-a1 and b0-b1 cross between their ends.
fn crosses(a0: Point2, a1: Point2, b0: Point2, b1: Point2) -> bool {
    let side = |p: Point2, q: Point2, r: Point2| (q - p).cross(r - p);
    let (d1, d2) = (side(b0, b1, a0), side(b0, b1, a1));
    let (d3, d4) = (side(a0, a1, b0), side(a0, a1, b1));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// The parts no balloon numbers, told on each BOM's last auto_balloons: they are in no view
/// drawn, or inside others in every one (a section shows those).
fn unballooned(
    ctx: &Ctx,
    d: &doc::Doc,
    stack: &std::collections::HashMap<String, f64>,
    errors: &mut Vec<DrawError>,
) {
    let mut last: Vec<(usize, &Item)> = Vec::new();
    for item in &d.items {
        let ItemKind::Annot(a) = &item.kind else { continue };
        let AnnotKind::AutoBalloons { from } = &a.kind else {
            continue;
        };
        let view = |id: &str| ctx.views.iter().find(|x| x.id == id);
        let Some(v) = view(&a.view) else { continue };
        let m = from.as_deref().and_then(view).map_or(v.model, |x| x.model);
        last.retain(|x| x.0 != m);
        last.push((m, item));
    }
    for (m, item) in last {
        let missing: Vec<String> = (1..=ctx.models[m].bom.len())
            .filter(|n| !stack.contains_key(&format!("balloon:{m}:{n}")))
            .map(|n| n.to_string())
            .collect();
        if !missing.is_empty() {
            errors.push(warn(
                Problem::NotHere {
                    item: item.id.clone(),
                    why: format!(
                        "items {} are seen in no ballooned view: balloon them in a section or another view (from: ..)",
                        missing.join(", ")
                    ),
                },
                item.span,
                vec![],
            ));
        }
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
            if f.material.is_empty() {
                let name = mo
                    .found
                    .as_ref()
                    .map(|m| m.name.clone())
                    .or(mo.material.clone())
                    .unwrap_or_default();
                f.material = match &mo.sheet {
                    Some((_, t)) => format!("{} t{}", name.to_uppercase(), number(*t, 2)),
                    None => name.to_uppercase(),
                };
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
    if f.mass.is_empty()
        && let Some(m) = first_view.and_then(|v| ctx.models[v.model].mass())
    {
        f.mass = format!("{} kg", number(m, 3));
    }
    if f.sheet.is_empty() {
        f.sheet = format!("{} OF {}", sheet + 1, n_sheets);
    }
}

/// Makes the drawing: `src` read, its models built (paths from `dir`), its sheets laid out.
pub fn make(src: &str, dir: &Path) -> Drawing {
    make_with(src, dir, None)
}

/// A drawing on its way: the sheets with the views made so far (or, `kept`, as the last make of
/// the same file and everything it read left them), for a screen to show until it is done.
pub struct Shown {
    pub drawing: Drawing,
    pub done: usize,
    pub total: usize,
    pub kept: bool,
}

/// `make`, showing first the sheets as last made (when nothing they were made from changed), then
/// each view as it is made; the sheets made are kept for the next open.
pub fn shown(src: &str, file: &Path, show: &mut dyn FnMut(Shown)) -> Drawing {
    let file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    let dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
    let key = crate::store::key("drawing-pages", |h| {
        h.str(&file.to_string_lossy());
        h.str(src);
    });
    let total = doc::read(src)
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::View(_)))
        .count();
    if let Some(pages) = crate::kept::fetch("drawing-pages", key).and_then(|b| super::pagecode::read(&b)) {
        show(Shown {
            drawing: preview(src, pages),
            done: 0,
            total,
            kept: true,
        });
    }
    let (d, from) = crate::kept::watched(&file, src, || make_with(src, &dir, Some(show)));
    crate::kept::keep("drawing-pages", key, &from, &super::pagecode::write(&d.pages));
    d
}

/// A drawing that is only its sheets: what a screen shows while the real one is made.
fn preview(src: &str, pages: Vec<Page>) -> Drawing {
    Drawing {
        doc: doc::read(src),
        models: vec![],
        sheets: vec![],
        pages,
        views: vec![],
        items: vec![],
        errors: vec![],
        part_loads: vec![],
    }
}

/// The sheets, their frames and the views made so far: no annotations yet.
fn sheets_so_far(ctx: &Ctx) -> Vec<Page> {
    let n = ctx.sheets.len();
    let mut pages: Vec<Page> = ctx
        .sheets
        .iter()
        .enumerate()
        .map(|(k, (_, s))| {
            let mut page = Page {
                w: s.w,
                h: s.h,
                ..Page::default()
            };
            let mut f = s.fields.clone();
            defaults(ctx, &mut f, k, n);
            forge_draw::sheet::format(&mut page, &f, s.third_angle);
            page
        })
        .collect();
    for v in &ctx.views {
        let Some(page) = pages.get_mut(v.sheet) else {
            continue;
        };
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
    }
    pages
}

fn make_with(src: &str, dir: &Path, mut show: Option<&mut dyn FnMut(Shown)>) -> Drawing {
    let d = doc::read(src);
    let mut ctx = Ctx {
        dir: dir.to_path_buf(),
        models: vec![],
        sheets: vec![],
        views: vec![],
        errors: d.errors.clone(),
        letters: 0,
        drawn: Default::default(),
        parts: Default::default(),
    };
    // the models, each built on its own cores
    let wanted: Vec<(&str, &str, Option<&str>)> = d
        .items
        .iter()
        .filter_map(|i| match &i.kind {
            ItemKind::Model { path, component, .. } => {
                Some((i.id.as_str(), path.as_str(), component.as_deref()))
            }
            _ => None,
        })
        .collect();
    let _sp = forge_math::trace::span("drawing models");
    let pool: Arc<crate::asm::PartPool> = Arc::default();
    // first what every assembly asks of each part file, building nothing: then one load of a file
    // serves them all (the rack and the door drawn on its own ask the door's parts for different
    // references)
    let wants: Arc<crate::asm::Wants> = Arc::default();
    crate::par::map(wanted.len(), |k| wants_of(wanted[k].1, &ctx.dir, &pool, &wants));
    let mut loaded = crate::par::map(wanted.len(), |k| {
        load(wanted[k].0, wanted[k].1, &ctx.dir, &pool, &wants, wanted[k].2)
    })
    .into_iter();
    let part_loads = crate::asm::pool_files(&pool);
    drop(_sp);
    for item in &d.items {
        match &item.kind {
            ItemKind::Model {
                path,
                material,
                density,
                ..
            } => match loaded.next().expect("one load per model") {
                Ok(m) => {
                    // the part's material: the drawing's name for it, else its sheet's
                    let name = material
                        .clone()
                        .or_else(|| m.sheet.as_ref().map(|s| s.0.clone()).filter(|n| !n.is_empty()));
                    crate::deps::materials(Some(&ctx.dir));
                    let libs = forge_material::Libraries::cached(Some(&ctx.dir));
                    let found = name
                        .as_deref()
                        .and_then(|n| libs.find(n))
                        .map(|(_, mat)| mat.clone());
                    if let (Some(n), None, None) = (&name, &found, density) {
                        ctx.errors.push(warn(
                            Problem::BadValue {
                                func: "part",
                                arg: "material".into(),
                                why: format!(
                                    "`{n}` is not in the material libraries: near {}",
                                    libs.near(n).join(", ")
                                ),
                            },
                            item.span,
                            vec![],
                        ));
                    }
                    ctx.models.push(ModelOut {
                        density: *density,
                        material: name,
                        found,
                        ..m
                    })
                }
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
                standard: forge_draw::gdt::Standard::Asme,
                size: "B".into(),
                w,
                h,
                third_angle: true,
                fields: Default::default(),
            },
        ));
    }
    let _sp = forge_math::trace::span("drawing views");
    ctx.ahead(&d.items);
    drop(_sp);
    let _sp = forge_math::trace::span("drawing sheets");
    let views_total = d
        .items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::View(_)))
        .count();
    for item in &d.items {
        if let ItemKind::View(def) = &item.kind
            && let Some(mut v) = ctx.view(item, def)
        {
            // a view not lined up with its parent is put where `at` says, cropped or not
            let free = match v.kind {
                "base" | "detail" => true,
                "projected" => false,
                _ => !ctx
                    .views
                    .iter()
                    .find(|p| Some(&p.id) == v.parent.as_ref())
                    .is_some_and(|p| p.scale == v.scale),
            };
            cut_view(&mut v, def, free);
            ctx.views.push(v);
            if let Some(show) = show.as_mut() {
                show(Shown {
                    drawing: preview(src, sheets_so_far(&ctx)),
                    done: ctx.views.len(),
                    total: views_total,
                    kept: false,
                });
            }
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
                at: v.map(t.at),
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
                    let sheet_pts: Vec<Point2> = pts.iter().map(|q| p.map(*q)).collect();
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
                        .point(c + Vec2::new(-r, r) * std::f64::consts::FRAC_1_SQRT_2)
                        + Vec2::new(-2.0, 2.0);
                    // the fence's letter at its upper left, clear of what it encloses
                    pages[p.sheet].texts.push(Text {
                        at: lp,
                        h: 5.0,
                        angle: 0.0,
                        align: Align::Right,
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
            dir: None,
            fos: false,
            anchor: Point2::ORIGIN,
            half: Vec2::ZERO,
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
    let mut stack = std::collections::HashMap::new();
    for item in &d.items {
        match &item.kind {
            ItemKind::Annot(a) => {
                let sheet = ctx.views.iter().find(|v| v.id == a.view).map_or(0, |v| v.sheet);
                // room under it for the frames and datum symbols that hang from it
                let hung: f64 = d
                    .items
                    .iter()
                    .filter_map(|x| match &x.kind {
                        ItemKind::Annot(b) => match &b.kind {
                            AnnotKind::Fcf {
                                to: doc::Attach::Item(id),
                                rows,
                                ..
                            } if id == &item.id => Some(7.0 * rows.len() as f64),
                            AnnotKind::Datum {
                                to: doc::Attach::Item(id),
                                ..
                            } if id == &item.id => Some(10.0),
                            _ => None,
                        },
                        _ => None,
                    })
                    .sum();
                let a = place_clear(&ctx, item, a, &pages[sheet], &errors, &items, &stack, hung);
                annotate(
                    &ctx,
                    item,
                    &a,
                    &mut pages[sheet],
                    &mut errors,
                    &mut items,
                    &mut stack,
                );
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
                    dir: None,
                    fos: false,
                    anchor: Point2::ORIGIN,
                    half: Vec2::ZERO,
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
                // the description column as wide as its longest, within reason; a list taller
                // than the sheet goes on in a second table to its left, its header repeated
                let desc_w = rows
                    .iter()
                    .map(|r| forge_draw::page::text_width(&r[3], 2.5) + 4.0)
                    .fold(40.0, f64::max)
                    .min(150.0);
                let widths = [12.0, 12.0, 46.0, desc_w];
                let table_w: f64 = widths.iter().sum();
                let fit = (((i1.y - corner.y) / 6.0).floor() as usize)
                    .saturating_sub(1)
                    .max(4);
                let mut h: f64 = 0.0;
                for (c, chunk) in rows.chunks(fit).enumerate() {
                    let at = Point2::new(corner.x - c as f64 * (table_w + 6.0), corner.y);
                    h = h.max(annot::table(
                        &mut pages[k],
                        at,
                        &["ITEM", "QTY", "PART NUMBER", "DESCRIPTION"],
                        &widths,
                        chunk,
                    ));
                }
                let n_tables = rows.len().div_ceil(fit).max(1) as f64;
                items.push(ItemOut {
                    dir: None,
                    fos: false,
                    anchor: Point2::ORIGIN,
                    half: Vec2::ZERO,
                    id: item.id.clone(),
                    sheet: k,
                    kind: "parts_list",
                    lo: Point2::new(corner.x - n_tables * (table_w + 6.0) + 6.0, corner.y),
                    hi: Point2::new(corner.x, corner.y + h),
                    sick: false,
                    value: None,
                    view: view.clone(),
                });
            }
            _ => {}
        }
    }
    unballooned(&ctx, &d, &stack, &mut errors);
    gdt_checks(&ctx, &d, &items, &mut errors);
    let sheets = ctx.sheets.iter().map(|s| s.0.clone()).collect();
    Drawing {
        sheets,
        pages,
        views: ctx.views,
        items,
        errors,
        models: ctx.models,
        doc: d,
        part_loads,
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
        Feat::Point(p) => (v.map(p), v.map(p)),
        Feat::Line(a, b) => (v.map(a), v.map(b)),
        Feat::Circle { c, .. } => (v.map(c), v.map(c)),
    })
}

fn gdt_error(
    item: &Item,
    rule: doc::GdtRule,
    detail: String,
    warning: bool,
    fixes: Vec<DrawOp>,
) -> DrawError {
    DrawError {
        problem: Problem::Gdt {
            item: item.id.clone(),
            rule,
            detail,
        },
        span: item.span,
        warning,
        fixes,
    }
}

/// The drawing checked against its standard's GD&T rules: datum letters defined once and legal,
/// every frame's datums defined, datums where the characteristic needs (or forbids) them, the
/// standard's own characteristics, diameter zones only where they mean something, runout about
/// an axis, and position located by basic dimensions.
fn gdt_checks(ctx: &Ctx, d: &doc::Doc, items: &[ItemOut], errors: &mut Vec<DrawError>) {
    use forge_draw::gdt::{Char, Datums, datum_letter_ok};
    let annots = || {
        d.items.iter().filter_map(|i| match &i.kind {
            ItemKind::Annot(a) => Some((i, a)),
            _ => None,
        })
    };
    // datum letters, in order, with whether their feature has a size (an axis)
    let mut letters: Vec<(String, bool)> = Vec::new();
    let next_free = |taken: &[(String, bool)]| {
        ('A'..='Z')
            .filter(|c| !"IOQ".contains(*c))
            .map(|c| c.to_string())
            .find(|c| !taken.iter().any(|t| &t.0 == c))
            .unwrap_or_else(|| "Z".into())
    };
    for (i, a) in annots() {
        match &a.kind {
            AnnotKind::Datum { letter, .. } => {
                let fos = items.iter().find(|x| x.id == i.id).is_some_and(|x| x.fos);
                if !datum_letter_ok(letter) || letter.len() != 1 {
                    let free = next_free(&letters);
                    errors.push(gdt_error(
                        i,
                        doc::GdtRule::DatumLetter,
                        format!("`{letter}`: a datum is one capital letter, not I, O or Q"),
                        false,
                        vec![DrawOp::SetArg {
                            item: i.id.clone(),
                            arg: "2".into(),
                            value: format!("\"{free}\""),
                        }],
                    ));
                } else if letters.iter().any(|l| &l.0 == letter) {
                    let free = next_free(&letters);
                    errors.push(gdt_error(
                        i,
                        doc::GdtRule::DatumDuplicate,
                        format!("datum {letter} is already on the drawing"),
                        false,
                        vec![DrawOp::SetArg {
                            item: i.id.clone(),
                            arg: "2".into(),
                            value: format!("\"{free}\""),
                        }],
                    ));
                }
                letters.push((letter.clone(), fos));
            }
            AnnotKind::DatumTarget { label, .. } => {
                let l: String = label.chars().take_while(|c| c.is_ascii_uppercase()).collect();
                if !letters.iter().any(|x| x.0 == l) {
                    letters.push((l, false));
                }
            }
            _ => {}
        }
    }
    for (i, a) in annots() {
        let AnnotKind::Fcf { to, rows, .. } = &a.kind else {
            continue;
        };
        let std = ctx
            .views
            .iter()
            .find(|v| v.id == a.view)
            .and_then(|v| ctx.sheets.get(v.sheet))
            .map_or(forge_draw::gdt::Standard::Asme, |s| s.1.standard);
        let single = rows.len() == 1;
        for (k, row) in rows.iter().enumerate() {
            let at = if single {
                String::new()
            } else {
                format!("row {}: ", k + 1)
            };
            let word = row.ch.word();
            if !row.ch.in_standard(std) {
                errors.push(gdt_error(
                    i,
                    doc::GdtRule::NotInStandard,
                    format!("{at}{word} is not in ASME Y14.5-2018; use position or profile"),
                    false,
                    if single {
                        vec![DrawOp::SetArg {
                            item: i.id.clone(),
                            arg: "2".into(),
                            value: "\"position\"".into(),
                        }]
                    } else {
                        vec![]
                    },
                ));
            }
            match row.ch.datums() {
                Datums::Never if !row.datums.is_empty() => errors.push(gdt_error(
                    i,
                    doc::GdtRule::NoDatumsAllowed,
                    format!("{at}{word} is a form tolerance: it takes no datums"),
                    false,
                    if single {
                        vec![DrawOp::SetArg {
                            item: i.id.clone(),
                            arg: "4".into(),
                            value: "[]".into(),
                        }]
                    } else {
                        vec![]
                    },
                )),
                Datums::Required if row.datums.is_empty() => errors.push(gdt_error(
                    i,
                    doc::GdtRule::DatumsNeeded,
                    if matches!(row.ch, Char::CircularRunout | Char::TotalRunout) {
                        format!("{at}{word} is measured about a datum axis: give one")
                    } else {
                        format!("{at}{word} is relative to a datum: give one")
                    },
                    false,
                    vec![],
                )),
                _ => {}
            }
            if row.ch == Char::Position && row.datums.is_empty() && (single || k == 0) {
                errors.push(gdt_error(
                    i,
                    doc::GdtRule::PositionNeedsFrame,
                    format!("{at}position with no datum reference frame"),
                    true,
                    vec![],
                ));
            }
            if row.tol.dia && !row.ch.may_be_diameter() {
                let kept: Vec<String> = cell_words(&row.tol)
                    .into_iter()
                    .filter(|w| w != "dia" && w != "Sdia")
                    .collect();
                errors.push(gdt_error(
                    i,
                    doc::GdtRule::DiameterNotAllowed,
                    format!("{at}{word} has no diameter zone"),
                    false,
                    if single {
                        vec![DrawOp::SetArg {
                            item: i.id.clone(),
                            arg: "3".into(),
                            value: format!("\"{}\"", kept.join(" ")),
                        }]
                    } else {
                        vec![]
                    },
                ));
            }
            for dr in &row.datums {
                for l in dr.letters.split('-') {
                    match letters.iter().find(|x| x.0 == l) {
                        None => {
                            let rest: Vec<String> = row
                                .datums
                                .iter()
                                .filter(|x| x.letters != dr.letters)
                                .map(|x| {
                                    let mut w = vec![x.letters.clone()];
                                    w.extend(x.mods.iter().map(|m| match m {
                                        forge_draw::gdt::Modifier::Mmc => "M".to_string(),
                                        forge_draw::gdt::Modifier::Lmc => "L".to_string(),
                                        _ => "S".to_string(),
                                    }));
                                    format!("\"{}\"", w.join(" "))
                                })
                                .collect();
                            errors.push(gdt_error(
                                i,
                                doc::GdtRule::DatumUndefined,
                                format!("{at}datum {l} is not on the drawing: add its datum feature symbol"),
                                false,
                                if single {
                                    vec![DrawOp::SetArg {
                                        item: i.id.clone(),
                                        arg: "4".into(),
                                        value: format!("[{}]", rest.join(", ")),
                                    }]
                                } else {
                                    vec![]
                                },
                            ));
                        }
                        Some((_, fos))
                            if !fos
                                && matches!(row.ch, Char::CircularRunout | Char::TotalRunout)
                                && k == 0
                                && dr == &row.datums[0] =>
                        {
                            errors.push(gdt_error(
                                i,
                                doc::GdtRule::RunoutNeedsAxis,
                                format!("{at}runout is about an axis: datum {l} is not on a feature of size"),
                                true,
                                vec![],
                            ));
                        }
                        _ => {}
                    }
                }
            }
            // a positioned feature (or pattern of like features) is located by basic dimensions,
            // both ways on the view
            if row.ch == Char::Position && k == 0 {
                let target = match to {
                    doc::Attach::Ref(r) => Some(r.clone()),
                    doc::Attach::Item(id) => d.item(id).and_then(|it| match &it.kind {
                        ItemKind::Annot(x) => match &x.kind {
                            AnnotKind::Diameter { e }
                            | AnnotKind::Radius { e }
                            | AnnotKind::HoleNote { e } => Some(e.clone()),
                            _ => None,
                        },
                        _ => None,
                    }),
                };
                let Some(target) = target else { continue };
                let Some(v) = ctx.views.iter().find(|v| v.id == a.view) else {
                    continue;
                };
                let model = &ctx.models[v.model].model;
                // a reference's circle, whatever point of it the dimension takes
                let radius = |r: &Ref| match resolve_ref(
                    model,
                    v,
                    &Ref {
                        at: None,
                        ..r.clone()
                    },
                )
                .map(|x| x.feat)
                {
                    Some(Feat::Circle { r, .. }) => Some(r),
                    _ => None,
                };
                let tr = radius(&target);
                // its pattern: the circles of its size
                let of_pattern = |r: &Ref| {
                    r.name == target.name
                        || tr.is_some_and(|tr| radius(r).is_some_and(|x| (x - tr).abs() < 1e-9))
                };
                let (mut across, mut up) = (false, false);
                // the view itself, or a detail of it: the same model coordinates
                let related = |w: &str| {
                    w == a.view
                        || ctx.views.iter().any(|x| {
                            x.id == w && x.kind == "detail" && x.parent.as_deref() == Some(a.view.as_str())
                        })
                };
                for (j, b) in annots() {
                    if let AnnotKind::Set { refs, dir, .. } = &b.kind
                        && related(&b.view)
                        && b.tol == Some(doc::TolSpec::Basic)
                        && refs.iter().any(of_pattern)
                    {
                        if dir == "vertical" {
                            up = true;
                        } else {
                            across = true;
                        }
                        continue;
                    }
                    let AnnotKind::Linear { a: ra, b: rb, .. } = &b.kind else {
                        continue;
                    };
                    if !related(&b.view) || !(of_pattern(ra) || rb.as_ref().is_some_and(of_pattern)) {
                        continue;
                    }
                    if b.tol == Some(doc::TolSpec::Basic) {
                        if let Some(dir) = items.iter().find(|x| x.id == j.id).and_then(|x| x.dir) {
                            if dir.x.abs() >= dir.y.abs() {
                                across = true;
                            } else {
                                up = true;
                            }
                        }
                        continue;
                    }
                    errors.push(gdt_error(
                        i,
                        doc::GdtRule::PositionNotBasic,
                        format!("{} locates the positioned feature: make it basic", j.id),
                        true,
                        vec![DrawOp::SetArg {
                            item: j.id.clone(),
                            arg: "basic".into(),
                            value: "true".into(),
                        }],
                    ));
                }
                if !(across && up) {
                    let missing = match (across, up) {
                        (false, false) => "either way",
                        (true, false) => "up the view",
                        _ => "across the view",
                    };
                    errors.push(gdt_error(
                        i,
                        doc::GdtRule::PositionNotLocated,
                        format!("no basic dimension locates the positioned feature {missing}"),
                        true,
                        vec![],
                    ));
                }
            }
        }
    }
}

/// Boxes of a page's texts (unturned), low and high corners.
fn text_boxes(p: &Page, pad: f64) -> Vec<(Point2, Point2)> {
    p.texts
        .iter()
        .filter(|t| t.h > 0.0)
        .map(|t| {
            let w = forge_draw::page::text_width(&t.text, t.h);
            let x = match t.align {
                Align::Left => t.at.x,
                Align::Centre => t.at.x - 0.5 * w,
                Align::Right => t.at.x - w,
            };
            (
                Point2::new(x - pad, t.at.y - pad),
                Point2::new(x + w + pad, t.at.y + t.h + pad),
            )
        })
        .collect()
}

fn boxes_meet(a: &(Point2, Point2), b: &(Point2, Point2)) -> bool {
    a.0.x < b.1.x && b.0.x < a.1.x && a.0.y < b.1.y && b.0.y < a.1.y
}

/// Whether segment `p..q` passes through box `b`.
fn segment_meets(p: Point2, q: Point2, b: &(Point2, Point2)) -> bool {
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for (s, d, l, h) in [(p.x, q.x - p.x, b.0.x, b.1.x), (p.y, q.y - p.y, b.0.y, b.1.y)] {
        if d.abs() < 1e-12 {
            if s < l || s > h {
                return false;
            }
            continue;
        }
        let (t0, t1) = ((l - s) / d, (h - s) / d);
        lo = lo.max(t0.min(t1));
        hi = hi.min(t0.max(t1));
    }
    lo < hi
}

fn segments_cross(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let (r, s) = (b - a, d - c);
    let den = r.cross(s);
    if den.abs() < 1e-12 {
        return false;
    }
    let (x, y) = ((c - a).cross(s) / den, (c - a).cross(r) / den);
    x > 1e-6 && x < 1.0 - 1e-6 && y > 1e-6 && y < 1.0 - 1e-6
}

/// A leader-placed annotation moved, if it must, to the nearest place where its text sits clear
/// of the views, the title block and other text, and its leader crosses no other text and no
/// centreline: the drawing's own auto-layout. Dimensions keep where they were put.
#[allow(clippy::too_many_arguments)]
fn place_clear(
    ctx: &Ctx,
    item: &Item,
    a: &AnnotDef,
    page: &Page,
    errors: &[DrawError],
    items: &[ItemOut],
    stack: &std::collections::HashMap<String, f64>,
    hung: f64,
) -> AnnotDef {
    let movable = match &a.kind {
        AnnotKind::HoleNote { .. }
        | AnnotKind::Leader { .. }
        | AnnotKind::Radius { .. }
        | AnnotKind::Diameter { .. }
        | AnnotKind::DatumTarget { .. }
        | AnnotKind::Surface { .. }
        | AnnotKind::Weld { .. }
        | AnnotKind::Edge { .. } => true,
        AnnotKind::Fcf { to, .. } | AnnotKind::Datum { to, .. } => matches!(to, doc::Attach::Ref(_)),
        _ => false,
    };
    let (Some(at), Some(v)) = (a.at, ctx.views.iter().find(|x| x.id == a.view)) else {
        return a.clone();
    };
    if !movable {
        return a.clone();
    }
    let mut busy: Vec<(Point2, Point2)> = text_boxes(page, 1.0);
    busy.extend(
        ctx.views
            .iter()
            .filter(|x| x.sheet == v.sheet)
            .map(|x| (x.lo - Vec2::new(1.0, 1.0), x.hi + Vec2::new(1.0, 1.0))),
    );
    let (_, i1) = forge_draw::sheet::inside(page.w, page.h);
    let title = (
        Point2::new(i1.x - forge_draw::sheet::TITLE_W, 0.0),
        Point2::new(
            page.w,
            forge_draw::sheet::MARGIN + forge_draw::sheet::BAND + forge_draw::sheet::TITLE_H,
        ),
    );
    busy.push(title);
    let (lo, hi) = forge_draw::sheet::inside(page.w, page.h);
    // the sheet's thin lines (dimension, extension and leader lines): text sits off them too
    let lines: Vec<(Point2, Point2)> = page
        .strokes
        .iter()
        .filter(|s| matches!(s.pen, Pen::Thin | Pen::Center))
        .filter_map(|s| match s.geom {
            Geom2::Line { a, b } => Some((a, b)),
            _ => None,
        })
        .collect();
    let start = v.map(at);
    let mut tries = vec![Vec2::ZERO];
    for r in [4.0, 8.0, 12.0, 16.0, 22.0, 30.0, 40.0] {
        for k in 0..12 {
            let (s, c) = dmath::sin_cos(dmath::TAU * k as f64 / 12.0);
            tries.push(Vec2::new(c, s) * r);
        }
    }
    let mut best: Option<(usize, AnnotDef)> = None;
    for off in tries {
        let mut b = a.clone();
        b.at = Some(v.place.inverse_point(start + off));
        let mut scratch = Page {
            w: page.w,
            h: page.h,
            ..Page::default()
        };
        let (mut e, mut it, mut st) = (errors.to_vec(), items.to_vec(), stack.clone());
        annotate(ctx, item, &b, &mut scratch, &mut e, &mut it, &mut st);
        // the label: its text, and the frame or symbol round it (strokes near the text)
        let mut labels = text_boxes(&scratch, 0.5);
        if let Some(first) = labels.first().copied() {
            let mut b = labels.iter().fold(first, |m, l| {
                (
                    Point2::new(m.0.x.min(l.0.x), m.0.y.min(l.0.y)),
                    Point2::new(m.1.x.max(l.1.x), m.1.y.max(l.1.y)),
                )
            });
            let reach = (
                Point2::new(b.0.x - 9.0, b.0.y - 9.0),
                Point2::new(b.1.x + 9.0, b.1.y + 9.0),
            );
            let within =
                |p: Point2| p.x >= reach.0.x && p.x <= reach.1.x && p.y >= reach.0.y && p.y <= reach.1.y;
            for st in &scratch.strokes {
                let (lo, hi) = st.geom.bounds();
                if within(lo) && within(hi) {
                    b = (
                        Point2::new(b.0.x.min(lo.x), b.0.y.min(lo.y)),
                        Point2::new(b.1.x.max(hi.x), b.1.y.max(hi.y)),
                    );
                }
            }
            b.0.y -= hung;
            labels = vec![b];
        }
        let outside = labels
            .iter()
            .filter(|l| !(l.0.x >= lo.x && l.0.y >= lo.y && l.1.x <= hi.x && l.1.y <= hi.y))
            .count();
        let on = labels
            .iter()
            .map(|l| {
                busy.iter().filter(|x| boxes_meet(l, x)).count()
                    + lines.iter().filter(|(p, q)| segment_meets(*p, *q, l)).count()
            })
            .sum::<usize>();
        let texts = text_boxes(page, 0.3);
        let crossed: usize = scratch
            .strokes
            .iter()
            .filter_map(|s| match s.geom {
                Geom2::Line { a, b } if a.distance(b) > 5.0 => Some((a, b)),
                _ => None,
            })
            .map(|(p, q)| {
                texts.iter().filter(|x| segment_meets(p, q, x)).count() * 4
                    + lines.iter().filter(|(c, d)| segments_cross(p, q, *c, *d)).count()
            })
            .sum();
        // what a drafter minds most: text off the sheet, text on lines or text, then crossings;
        // and the nearer the place asked for, the better
        let score = outside * 1000 + on * 20 + crossed;
        if score == 0 {
            return b;
        }
        if best.as_ref().is_none_or(|x| score < x.0) {
            best = Some((score, b));
        }
    }
    best.map_or_else(|| a.clone(), |x| x.1)
}

/// A view cropped to its box and broken at its bands: lines outside the crop and inside a band
/// go, lines past a band close up to it, and the break is drawn as two zigzag lines.
fn cut_view(v: &mut ViewOut, def: &ViewDef, free: bool) {
    if def.crop.is_none() && def.breaks.is_empty() {
        return;
    }
    let keep = |q: Point2| {
        def.crop
            .is_none_or(|(lo, hi)| q.x >= lo.x && q.x <= hi.x && q.y >= lo.y && q.y <= hi.y)
            && !def.breaks.iter().any(|&(x, lo, hi)| {
                let c = if x { q.x } else { q.y };
                c > lo && c < hi
            })
    };
    v.breaks = def.breaks.clone();
    v.gap = 6.0 / v.scale;
    let shift_of = |g: &Geom2| {
        let m = g.at(0.5 * (g.range().0 + g.range().1));
        v.closed(m) - m
    };
    let mut pieces = Vec::new();
    for p in &v.pieces {
        for g in p.geom.runs_where(0.01, |t| keep(p.geom.at(t))) {
            let d = shift_of(&g);
            pieces.push(Piece {
                geom: g.placed(&Place::new(Point2::ORIGIN + d, 1.0)),
                ..p.clone()
            });
        }
    }
    let mut extra = Vec::new();
    for st in &v.extra {
        for g in st.geom.runs_where(0.01, |t| keep(st.geom.at(t))) {
            let d = shift_of(&g);
            extra.push(Stroke {
                pen: st.pen,
                geom: g.placed(&Place::new(Point2::ORIGIN + d, 1.0)),
            });
        }
    }
    let (lo, hi) = bounds(&pieces, &extra);
    // the break lines: a zigzag at each side of the closed-up gap
    for &(x, a, _) in &def.breaks {
        let at = v.closed(Point2::new(a, a));
        for c in [if x { at.x } else { at.y }, if x { at.x } else { at.y } + v.gap] {
            let (s0, s1) = if x {
                (lo.y - 2.0 / v.scale, hi.y + 2.0 / v.scale)
            } else {
                (lo.x - 2.0 / v.scale, hi.x + 2.0 / v.scale)
            };
            let n = 8;
            let amp = 1.2 / v.scale;
            let pts: Vec<Point2> = (0..=n)
                .map(|k| {
                    let t = s0 + (s1 - s0) * k as f64 / n as f64;
                    let w = c + if k % 2 == 0 { -amp } else { amp };
                    if x { Point2::new(w, t) } else { Point2::new(t, w) }
                })
                .collect();
            extra.push(Stroke {
                pen: Pen::Thin,
                geom: Geom2::Poly(pts),
            });
        }
    }
    v.pieces = pieces;
    v.extra = extra;
    let (lo, hi) = bounds(&v.pieces, &v.extra);
    if free && def.crop.is_some() {
        v.place.o += def.at - v.place.point(lo.lerp(hi, 0.5));
    }
    v.lo = v.place.point(lo);
    v.hi = v.place.point(hi);
}
