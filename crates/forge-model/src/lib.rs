//! A part as a history of features, and regenerating it into a solid.

pub mod adversarial;
pub mod bodies;
mod edit;
pub mod edits;
pub mod extent;
pub mod fillets;
pub mod finishing;
pub mod hole;
pub mod names;
pub mod params;
mod path;
pub mod pattern;
mod rib;
mod splitface;
pub mod standard;
pub mod suite;
pub mod swept;
pub mod text;
pub mod thread;
pub mod torture;
mod trace;
mod upto;
pub mod work;

pub use bodies::{BodyError, BodyFeature, Cutter, Keep, Pick, Solid, Step, step_solids};
pub use edit::{Edit, EditError, Motion, Side};
pub use extent::{Extent, ExtentError, Target, Toward, Turn};
use forge_bool::{BoolError, CpuBatch, Op, Seg2, Shape, boolean_into};
use forge_gpu::AutoBatch;
pub use forge_gpu::{Backend, open_in_background as open_gpu};
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_ops::{
    Axis2, ChamferError, ChamferSize, DraftError, ExtrudeError, FilletEdge, FilletError, FilletOptions,
    Profile, ProfileLoop, Radius, RevolveError, Segment, ShellError, chamfer, draft, extrude, face_fillet,
    fillet_with, full_round, revolve, shell,
};
use forge_topo::{Body, Name};
pub use hole::{End, Head, Hole, HoleError, Thread};
pub use params::{On, Param, Sketch, Unit};
pub use pattern::{
    At, Layout, PatternError, Row, Source, body_motion, copy_sources, instance_name, placements,
};
use pattern::{instances, moved, moved_shape};
pub use rib::{Rib, RibError};
pub use standard::{Convention, Fit, Screw, Standard};
pub use swept::{Coil, CoilSize, Loft, PathEnds, Sweep};
pub use text::{FontRef, Text, TextError, TextOn};
pub use thread::{Length as ThreadLength, On as ThreadOn, ScrewThread, Spec as ThreadSpec, ThreadError};
pub use work::{AxisBy, Datum, PlaneBy, PointBy, Ref, UcsBy, Work, WorkError};

/// A part's history as the regen reads it: its features, and the id each is named by (its
/// place from 1, unless the document that holds the part gives ids that stay put when
/// features are inserted, deleted or moved).
#[derive(Clone, Copy, Debug)]
pub struct History<'a> {
    features: &'a [Feature],
    ids: Option<&'a [u64]>,
}

impl<'a> History<'a> {
    /// `ids`, one per feature, nonzero and distinct; None: each feature's place from 1.
    pub fn new(features: &'a [Feature], ids: Option<&'a [u64]>) -> History<'a> {
        if let Some(ids) = ids {
            assert_eq!(ids.len(), features.len(), "one id per feature");
        }
        History { features, ids }
    }

    /// The id feature `i`'s names derive from.
    pub fn id(&self, i: usize) -> u64 {
        self.ids.map_or(i as u64 + 1, |ids| ids[i])
    }

    /// The first `n` features.
    pub fn upto(&self, n: usize) -> History<'a> {
        History {
            features: &self.features[..n],
            ids: self.ids.map(|ids| &ids[..n]),
        }
    }

    /// The same ids on `features`, one for each of these (a copy with some rewritten).
    pub(crate) fn on<'b>(&self, features: &'b [Feature]) -> History<'b>
    where
        'a: 'b,
    {
        History::new(features, self.ids)
    }
}

impl std::ops::Deref for History<'_> {
    type Target = [Feature];
    fn deref(&self) -> &[Feature] {
        self.features
    }
}

/// One step of a part's history.
#[derive(Clone, Debug, PartialEq)]
pub enum Feature {
    /// Extrude a profile into a new solid, or add it to / cut it from the part.
    Extrude {
        profile: Profile,
        extent: Extent,
        op: Combine,
    },
    /// Revolve a profile about an axis in its plane.
    Revolve {
        profile: Profile,
        axis: Axis2,
        extent: Turn,
        op: Combine,
    },
    /// A ball (a shorthand for revolving a half disc).
    Ball { frame: Frame, radius: f64, op: Combine },
    /// Round the named edges of the part as it stands, each with its own radius, in one go.
    /// `starts`: (edge, vertex) where an edge's law runs from that vertex (one of its ends, or
    /// the end of its run of edges) rather than where `forge_ops::law_starts` puts it. `setbacks`: (vertex, one edge there or all, length) how far along
    /// each edge from a corner its blends stop.
    Fillet {
        edges: Vec<(Name, Radius)>,
        starts: Vec<(Name, Name)>,
        setbacks: Vec<(Name, Option<Name>, f64)>,
    },
    /// Round between two sets of faces: the edges where they meet, or the band of strip faces
    /// joining them, taken away.
    FaceFillet {
        first: Vec<Name>,
        second: Vec<Name>,
        radius: f64,
    },
    /// Take face `middle` away between faces `side` and `other` with the ball touching all three.
    FullRound { side: Name, middle: Name, other: Name },
    /// Bevel edges of the part.
    Chamfer { edges: Vec<Name>, size: ChamferSize },
    /// Hollow the part to walls `thickness` thick, inside or outside its skin, `open` faces removed.
    Shell {
        thickness: f64,
        outward: bool,
        open: Vec<Name>,
    },
    /// Taper faces by `angle` from the pull direction (`neutral`'s z) about the neutral plane.
    Draft {
        faces: Vec<Name>,
        neutral: Frame,
        angle: f64,
    },
    /// A drilled hole, cut from the part.
    Hole(Box<Hole>),
    /// Copies of earlier features, or of the whole part, in rows, round an axis, or mirrored.
    Pattern { of: Source, layout: Layout },
    /// A work plane (the frame's xy) for later extents to end at; it makes nothing.
    Plane(Frame),
    /// A profile swept along a path of lines and arcs.
    Sweep(Box<Sweep>),
    /// A profile wound about an axis.
    Coil(Box<Coil>),
    /// Ruled walls through two or more profiles.
    Loft(Box<Loft>),
    /// A rib, offset or thickened faces, or a direct edit of the part's faces.
    Edit(Edit),
    /// A named value; it makes nothing.
    Param(Box<Param>),
    /// A solved sketch; it makes nothing.
    Sketch(Box<Sketch>),
    /// A modeled screw thread cut round or inside a cylinder.
    Thread(Box<ScrewThread>),
    /// Text embossed on or engraved into a plane or a cylinder.
    Text(Box<Text>),
    /// A work plane, axis, point or UCS defined by references; it makes nothing.
    Work(Work),
    /// A feature on chosen bodies, or one that combines, splits or moves bodies (bodies.rs).
    Body(Box<BodyFeature>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combine {
    /// A new body beside any others (Inventor's new solid).
    New,
    Add,
    Cut,
    Keep,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RegenError {
    Extrude {
        feature: usize,
        error: ExtrudeError,
    },
    Revolve {
        feature: usize,
        error: RevolveError,
    },
    Boolean {
        feature: usize,
        error: BoolError,
    },
    Fillet {
        feature: usize,
        error: FilletError,
    },
    /// A feature names an edge (or face, or vertex) the part no longer has.
    MissingEdge {
        feature: usize,
        name: Name,
    },
    Chamfer {
        feature: usize,
        error: ChamferError,
    },
    Shell {
        feature: usize,
        error: ShellError,
    },
    Draft {
        feature: usize,
        error: DraftError,
    },
    Hole {
        feature: usize,
        error: HoleError,
    },
    Extent {
        feature: usize,
        error: ExtentError,
    },
    Pattern {
        feature: usize,
        error: PatternError,
    },
    Sweep {
        feature: usize,
        error: forge_ops::SweepError,
    },
    Coil {
        feature: usize,
        error: forge_ops::CoilError,
    },
    Loft {
        feature: usize,
        error: forge_ops::LoftError,
    },
    Edit {
        feature: usize,
        error: EditError,
    },
    Thread {
        feature: usize,
        error: ThreadError,
    },
    Text {
        feature: usize,
        error: TextError,
    },
    Work {
        feature: usize,
        error: WorkError,
    },
    /// A feature that combines with or changes the part came before any part existed.
    NothingToCombine {
        feature: usize,
    },
    Body {
        feature: usize,
        error: BodyError,
    },
    /// The part ends as several bodies, asked for as one.
    Bodies {
        count: usize,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    pub features: Vec<Feature>,
}

/// Tools in one n-ary run from which the n-ary regen is the faster, measured in instructions on
/// one thread (suite::fuzz histories, 6 seeds each): n-ary costs 5-22% more up to 24 tools,
/// about as much (median within 5%) from 24 to 56, and less from 64 on (the suite parts, ~200
/// tools, 0.5-0.9x except shaft and housing, within 10% over).
const NARY_FROM: usize = 64;

/// A tool: its solid, how it combines with the part, and the solid as an analytic shape.
type Tool = (Body, Combine, Shape);

impl Part {
    fn history(&self) -> History<'_> {
        History::new(&self.features, None)
    }

    /// Rebuilds the solid in one n-ary boolean over every feature's tool (built in parallel),
    /// on up to `threads` threads.
    /// The region batch is classified on the backend FORGE_BATCH names (default Auto). A
    /// long-running app calls `forge_model::open_gpu()` at startup so Auto has the GPU at once.
    pub fn regenerate_nary(&self, threads: usize) -> Result<Body, RegenError> {
        self.regenerate_nary_on(threads, Backend::from_env())
    }

    /// `regenerate_nary`, the region batch classified on `backend`.
    pub fn regenerate_nary_on(&self, threads: usize, backend: Backend) -> Result<Body, RegenError> {
        if bodies::multi(&self.features) {
            return bodies::one(self.regenerate_nary_solids(threads)?);
        }
        nary_regen(self.history(), wasm_one(threads), true, Batching::On(backend))
    }

    /// `regenerate_nary`, placing points against the tools' analytic solids first or not.
    pub fn regenerate_nary_with(&self, threads: usize, shapes: bool) -> Result<Body, RegenError> {
        self.regenerate_nary_batch(threads, shapes, None)
    }

    /// The regen expected to be the faster: the n-ary one from `NARY_FROM` tools in its last run
    /// (after the last part-changing feature) on, the sequential one below. Decided from the
    /// history alone, so the same history always takes the same path.
    pub fn regenerate_auto(&self, threads: usize) -> Result<Body, RegenError> {
        let start = self.features.iter().rposition(starts).unwrap_or(0);
        let start = match self.features.iter().rposition(modifies) {
            Some(m) if m >= start => m + 1,
            _ => start,
        };
        if self.features.len().saturating_sub(start) >= NARY_FROM {
            self.regenerate_nary(threads)
        } else {
            self.regenerate()
        }
    }

    /// The tools' analytic solids and ops, in the order the n-ary regen uses them (for building
    /// a batch classifier).
    pub fn nary_solids(&self) -> Result<(Vec<Shape>, Vec<Op>), RegenError> {
        let start = self.features.iter().rposition(starts).unwrap_or(0);
        let start = match self.features.iter().rposition(modifies) {
            Some(m) if m >= start => m + 1,
            _ => start,
        };
        let built = build_tools(self.history(), start, 1, false)?;
        let ops = built.iter().skip(1).map(|t| op_of(t.1)).collect();
        Ok((built.into_iter().map(|t| t.2).collect(), ops))
    }

    /// `regenerate_nary_with`, regions classified in one batch first (on a GPU, say).
    pub fn regenerate_nary_batch(
        &self,
        threads: usize,
        shapes: bool,
        batch: Option<&dyn forge_bool::Batch>,
    ) -> Result<Body, RegenError> {
        if bodies::multi(&self.features) {
            return bodies::one(bodies::nary(self.history(), wasm_one(threads))?);
        }
        let batching = batch.map_or(Batching::None, Batching::Given);
        nary_regen(self.history(), wasm_one(threads), shapes, batching)
    }

    /// Every body the part ends with, rebuilt feature by feature.
    pub fn regenerate_solids(&self) -> Result<Vec<Solid>, RegenError> {
        self.regenerate_solids_ids(None)
    }

    /// `regenerate_solids`, each feature named from its id in `ids` (as `regenerate_ids`).
    pub fn regenerate_solids_ids(&self, ids: Option<&[u64]>) -> Result<Vec<Solid>, RegenError> {
        bodies::sequential(History::new(&self.features, ids))
    }

    /// `regenerate_solids` in n-ary booleans, one per run of each body's tools.
    pub fn regenerate_nary_solids(&self, threads: usize) -> Result<Vec<Solid>, RegenError> {
        self.regenerate_nary_solids_ids(threads, None)
    }

    /// `regenerate_nary_solids`, each feature named from its id in `ids`.
    pub fn regenerate_nary_solids_ids(
        &self,
        threads: usize,
        ids: Option<&[u64]>,
    ) -> Result<Vec<Solid>, RegenError> {
        bodies::nary(History::new(&self.features, ids), wasm_one(threads))
    }

    /// Rebuilds the solid from the first feature. Feature ids are their
    /// positions, so entity names stay stable when nothing upstream changes.
    pub fn regenerate(&self) -> Result<Body, RegenError> {
        self.regenerate_ids(None)
    }

    /// `regenerate`, each feature named from its id in `ids` (one per feature, nonzero,
    /// distinct): ids kept through inserts, deletes and moves keep every name below.
    pub fn regenerate_ids(&self, ids: Option<&[u64]>) -> Result<Body, RegenError> {
        let h = History::new(&self.features, ids);
        if bodies::multi(&self.features) {
            return bodies::one(bodies::sequential(h)?);
        }
        let mut body = None;
        for i in 0..self.features.len() {
            body = step(h, i, body)?;
        }
        body.map(|b| b.compact())
            .ok_or(RegenError::NothingToCombine { feature: 0 })
    }

    /// Every cosmetic thread (each copy of a patterned hole has its own): the thread, the hole's
    /// axis (origin at the mouth, z down the drill) and the wall's radius.
    pub fn threads(&self) -> Vec<(Thread, Frame, f64)> {
        // each thread and the features whose solids hold it
        let mut out: Vec<(Thread, Frame, f64, Vec<usize>)> = Vec::new();
        for (i, f) in bodies::strip(&self.features).0.iter().enumerate() {
            match f {
                Feature::Hole(h) => {
                    if let Some(t) = h.thread {
                        out.push((t, h.at, h.diameter / 2.0, vec![i]));
                        for (_, m) in h.copies() {
                            out.push((t, m.frame(&h.at, true), h.diameter / 2.0, vec![i]));
                        }
                    }
                }
                Feature::Pattern { of, layout } => {
                    let Ok(copies) = instances(self.history(), i, layout) else {
                        continue;
                    };
                    let mut added = Vec::new();
                    for (t, at, r, owners) in &mut out {
                        let held = match of {
                            Source::Body => true,
                            Source::Features(src) => owners.iter().any(|o| src.contains(o)),
                        };
                        if held {
                            owners.push(i);
                            added.extend(copies.iter().map(|(_, m)| (*t, m.frame(at, true), *r, vec![i])));
                        }
                    }
                    out.extend(added);
                }
                _ => {}
            }
        }
        out.into_iter().map(|(t, at, r, _)| (t, at, r)).collect()
    }

    /// The faces of `body` (this part, regenerated) each thread lies on: the cylinders on its
    /// axis with its radius, however booleans split them.
    pub fn thread_faces(&self, body: &Body) -> Vec<(Thread, Vec<forge_topo::FaceId>)> {
        let eps = forge_math::tol::LINEAR;
        self.threads()
            .into_iter()
            .map(|(t, at, r)| {
                let on = |f: &forge_topo::FaceId| match &body.face(*f).surface {
                    forge_geom::Surface::Cylinder(c) => {
                        let off = at.origin - c.frame.origin;
                        let across = off - c.frame.z() * c.frame.z().dot(off);
                        (c.radius - r).abs() <= eps
                            && c.frame.z().dot(at.z().get()).abs() >= 1.0 - eps
                            && across.length() <= eps
                    }
                    _ => false,
                };
                (t, body.face_ids().filter(on).collect())
            })
            .collect()
    }
}

/// Applies feature `i` to the part as features `..i` left it: `regenerate` one feature at a
/// time, for callers that keep every stage. Only `features[..=i]` is read.
pub fn step(features: History<'_>, i: usize, body: Option<Body>) -> Result<Option<Body>, RegenError> {
    let f = &features[i];
    let id = features.id(i);
    if modifies(f) {
        let b = body.ok_or(RegenError::NothingToCombine { feature: i })?;
        let b = modify(features, f, i, &b, 1)?;
        if trace::on() {
            trace::step(i, &b, None);
        }
        return Ok(Some(b));
    }
    let mut tools = tools(features, i, true)?;
    if starts(f) {
        let b = tools.swap_remove(0).0;
        if trace::on() {
            trace::step(i, &b, None);
        }
        return Ok(Some(b));
    }
    if matches!(f, Feature::Plane(_) | Feature::Param(_) | Feature::Sketch(_)) {
        return Ok(body);
    }
    if let Feature::Work(_) = f {
        work::datum(features, i)?;
        return Ok(body);
    }
    let mut b = body.ok_or(RegenError::NothingToCombine { feature: i })?;
    let err = |error| RegenError::Boolean { feature: i, error };
    if let [(tool, op, _)] = &tools[..] {
        // in place: only the faces near the tool change
        boolean_into(&mut b, tool, op_of(*op), id + 1_000_000).map_err(err)?;
        // tombstones are cheap until they outnumber the living
        if b.dead > b.faces.len() / 2 {
            b = b.compact();
        }
    } else if !tools.is_empty() {
        // a pattern's copies all in one pass
        let base = if b.dead > 0 { b.compact() } else { b };
        let mut all = vec![&base];
        all.extend(tools.iter().map(|t| &t.0));
        let ops: Vec<Op> = tools.iter().map(|t| op_of(t.1)).collect();
        b = forge_bool::nary_with(&all, &ops, None, None, 1, cut_root(i)).map_err(err)?;
    }
    if trace::on() {
        trace::step(i, &b, tools.first().map(|t| &t.0));
    }
    Ok(Some(b))
}

/// The analytic solids of feature `i`'s tools and how each combines, in the order they are
/// applied: for checking against another kernel.
pub fn tool_shapes(features: History<'_>, i: usize) -> Result<Vec<(Shape, Combine)>, RegenError> {
    Ok(tools(features, i, false)?
        .into_iter()
        .map(|t| (t.2, t.1))
        .collect())
}

/// Whether a feature makes a new body.
fn starts(f: &Feature) -> bool {
    matches!(
        f,
        Feature::Extrude { op: Combine::New, .. }
            | Feature::Revolve { op: Combine::New, .. }
            | Feature::Ball { op: Combine::New, .. }
    ) || swept::op(f) == Some(Combine::New)
}

/// Whether a feature changes the part as a whole, rather than making tools to combine with it.
fn modifies(f: &Feature) -> bool {
    matches!(
        f,
        Feature::Fillet { .. }
            | Feature::FaceFillet { .. }
            | Feature::FullRound { .. }
            | Feature::Chamfer { .. }
            | Feature::Shell { .. }
            | Feature::Draft { .. }
            | Feature::Pattern { of: Source::Body, .. }
            | Feature::Edit(_)
            | Feature::Body(_)
    )
}

/// The tools of features `from..` on up to `threads` threads, in order (only their shapes,
/// solids left empty, unless `build`).
fn build_tools(
    features: History<'_>,
    from: usize,
    threads: usize,
    build: bool,
) -> Result<Vec<Tool>, RegenError> {
    let some = |r: std::ops::Range<usize>| -> Result<Vec<Tool>, RegenError> {
        let mut out = Vec::new();
        for i in r {
            out.extend(tools(features, i, build)?);
        }
        Ok(out)
    };
    let n = features.len().saturating_sub(from);
    if threads <= 1 || n <= 1 {
        return some(from..features.len());
    }
    // one feature at a time from a shared counter: tools differ widely in cost (a hole up to the
    // next face against a plain extrude), so fixed chunks left threads idle
    let next = std::sync::atomic::AtomicUsize::new(from);
    type Built = (usize, Result<Vec<Tool>, RegenError>);
    let done: Vec<Vec<Built>> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads.min(n))
            .map(|_| {
                s.spawn(|| {
                    let mut got = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if i >= features.len() {
                            break got;
                        }
                        got.push((i, some(i..i + 1)));
                    }
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().expect("tool builder")).collect()
    });
    let mut built: Vec<Built> = done.into_iter().flatten().collect();
    built.sort_unstable_by_key(|b| b.0);
    let built = built.into_iter().map(|b| b.1);
    let mut out = Vec::new();
    for b in built {
        out.extend(b?);
    }
    Ok(out)
}

/// wasm has no threads.
fn wasm_one(threads: usize) -> usize {
    if cfg!(target_family = "wasm") { 1 } else { threads }
}

/// How an n-ary regen classifies its regions in one batch.
#[derive(Clone, Copy)]
enum Batching<'a> {
    None,
    Given(&'a dyn forge_bool::Batch),
    /// The tools' own solids, on this backend.
    On(Backend),
}

/// The n-ary regen of a history: each run of solid features after a part-changing one (chamfer,
/// shell, draft, body pattern) is one n-ary boolean starting from the part so far.
fn nary_regen(
    features: History<'_>,
    threads: usize,
    shapes: bool,
    batching: Batching,
) -> Result<Body, RegenError> {
    let start = features.iter().rposition(starts).unwrap_or(0);
    let err = |error| RegenError::Boolean {
        feature: features.len() - 1,
        error,
    };
    if let Some(m) = features.iter().rposition(modifies)
        && m >= start
    {
        if m == 0 {
            return Err(RegenError::NothingToCombine { feature: 0 });
        }
        let modified = || {
            modify(
                features,
                &features[m],
                m,
                // a given batch holds the tools after the last modifier (nary_solids), not these
                &nary_regen(
                    features.upto(m),
                    threads,
                    shapes,
                    match batching {
                        Batching::Given(_) => Batching::None,
                        b => b,
                    },
                )?,
                threads,
            )
        };
        let mut body = modified()?;
        let built = build_tools(features, m + 1, threads, true)?;
        if built.is_empty() {
            return Ok(body);
        }
        if let [(tool, c, _)] = &built[..] {
            if single(&mut body, tool, *c) {
                return Ok(body);
            }
            // the boolean refused, perhaps halfway: the n-ary pass from the part as it was
            body = modified()?;
        }
        let mut tools: Vec<&Body> = vec![&body];
        tools.extend(built.iter().map(|t| &t.0));
        let ops: Vec<Op> = built.iter().map(|t| op_of(t.1)).collect();
        return forge_bool::nary_with(&tools, &ops, None, None, threads, cut_root(features.len() - 1))
            .map_err(err);
    }
    if features.get(start).is_none_or(|f| !starts(f)) {
        return Err(RegenError::NothingToCombine { feature: 0 });
    }
    let mut built = build_tools(features, start, threads, true)?;
    if let [(base, _, _), (tool, c, _)] = &mut built[..] {
        if single(base, tool, *c) {
            return Ok(built.swap_remove(0).0);
        }
        built = build_tools(features, start, threads, true)?;
    }
    let tools: Vec<&Body> = built.iter().map(|t| &t.0).collect();
    let ops: Vec<Op> = built[1..].iter().map(|t| op_of(t.1)).collect();
    let solids: Vec<Shape> = built.iter().map(|t| t.2.clone()).collect();
    let auto = match batching {
        Batching::On(backend) => Some(AutoBatch::new(
            CpuBatch {
                shapes: &solids,
                ops: &ops,
                threads,
            },
            backend,
        )),
        _ => None,
    };
    let batch: Option<&dyn forge_bool::Batch> = match (&auto, batching) {
        (Some(a), _) => Some(a),
        (None, Batching::Given(b)) => Some(b),
        _ => None,
    };
    forge_bool::nary_with(
        &tools,
        &ops,
        shapes.then_some(&solids[..]),
        batch,
        threads,
        cut_root(features.len() - 1),
    )
    .map_err(err)
}

/// A run of one tool: the sequential regen's boolean, the same body for less work. False where
/// that fails (`body` may be left halfway; the caller takes the n-ary pass from the start).
fn single(body: &mut Body, tool: &Body, c: Combine) -> bool {
    if boolean_into(body, tool, op_of(c), 0).is_err() {
        return false;
    }
    *body = body.compact();
    true
}

/// The root of the names a boolean gives the vertices it makes: the same for every boolean, so
/// the n-ary and the feature-by-feature regen name them alike.
fn cut_root(_i: usize) -> Name {
    Name::feature(0)
}

fn op_of(c: Combine) -> Op {
    match c {
        Combine::Add => Op::Union,
        Combine::Cut => Op::Subtract,
        _ => Op::Intersect,
    }
}

/// Applies part-changing feature `i` to `body`.
fn modify(
    features: History<'_>,
    f: &Feature,
    i: usize,
    body: &Body,
    threads: usize,
) -> Result<Body, RegenError> {
    let id = features.id(i);
    match f {
        Feature::Fillet {
            edges,
            starts,
            setbacks,
        } => apply_fillet(i, id, body, edges, starts, setbacks),
        Feature::FaceFillet {
            first,
            second,
            radius,
        } => {
            let faces = |ns: &[Name]| {
                ns.iter()
                    .map(|&n| face_of(i, body, n))
                    .collect::<Result<Vec<_>, _>>()
            };
            face_fillet(body, &faces(first)?, &faces(second)?, *radius, id)
                .map_err(|error| RegenError::Fillet { feature: i, error })
        }
        Feature::FullRound { side, middle, other } => full_round(
            body,
            face_of(i, body, *side)?,
            face_of(i, body, *middle)?,
            face_of(i, body, *other)?,
            id,
        )
        .map_err(|error| RegenError::Fillet { feature: i, error }),
        Feature::Chamfer { edges, size } => {
            chamfer(body, edges, *size, id).map_err(|error| RegenError::Chamfer { feature: i, error })
        }
        Feature::Shell {
            thickness,
            outward,
            open,
        } => shell(body, *thickness, *outward, open, id)
            .map_err(|error| RegenError::Shell { feature: i, error }),
        Feature::Draft {
            faces,
            neutral,
            angle,
        } => draft(body, faces, neutral, *angle).map_err(|error| RegenError::Draft { feature: i, error }),
        Feature::Pattern { layout, .. } => {
            let pe = |error| RegenError::Pattern { feature: i, error };
            let copies = instances(features, i, layout)?;
            let copies = copies
                .iter()
                .map(|(key, m)| moved(body, m, |n| instance_name(features.id(i), *key, n)))
                .collect::<Result<Vec<Body>, _>>()
                .map_err(pe)?;
            if copies.is_empty() {
                return Ok(body.clone());
            }
            let mut all = vec![body];
            all.extend(copies.iter());
            forge_bool::nary_with(
                &all,
                &vec![Op::Union; copies.len()],
                None,
                None,
                threads,
                cut_root(i),
            )
            .map_err(|error| RegenError::Boolean { feature: i, error })
        }
        Feature::Edit(e) => edit::apply(features, i, e, body),
        _ => unreachable!("solid features make tools"),
    }
}

/// The face named `n` on `body`.
fn face_of(i: usize, body: &Body, name: Name) -> Result<forge_topo::FaceId, RegenError> {
    body.face_by_name(name)
        .ok_or(RegenError::MissingEdge { feature: i, name })
}

/// Feature `i`, a fillet of the named edges, applied to `body`.
fn apply_fillet(
    i: usize,
    id: u64,
    body: &Body,
    edges: &[(Name, Radius)],
    starts: &[(Name, Name)],
    setbacks: &[(Name, Option<Name>, f64)],
) -> Result<Body, RegenError> {
    let body = if body.dead > 0 {
        body.compact()
    } else {
        body.clone()
    };
    let edge = |name: Name| {
        body.edge_by_name(name)
            .ok_or(RegenError::MissingEdge { feature: i, name })
    };
    let vertex = |name: Name| {
        body.vertex_ids()
            .find(|&v| body.vertex(v).name == name)
            .ok_or(RegenError::MissingEdge { feature: i, name })
    };
    let mut want: Vec<FilletEdge> = Vec::with_capacity(edges.len());
    for (name, radius) in edges {
        want.push(FilletEdge {
            edge: edge(*name)?,
            radius: radius.clone(),
        });
    }
    let opts = FilletOptions {
        starts: starts
            .iter()
            .map(|&(e, v)| Ok((edge(e)?, vertex(v)?)))
            .collect::<Result<_, RegenError>>()?,
        setbacks: setbacks
            .iter()
            .map(|&(v, e, d)| Ok((vertex(v)?, e.map(edge).transpose()?, d)))
            .collect::<Result<_, RegenError>>()?,
    };
    fillet_with(&body, &want, &opts, id).map_err(|error| RegenError::Fillet { feature: i, error })
}

/// The tools feature `i` makes: one, or a pattern's copies (each named after its place).
fn tools(features: History<'_>, i: usize, build: bool) -> Result<Vec<Tool>, RegenError> {
    match &features[i] {
        Feature::Hole(h) => {
            let (h, up, next) = extent::hole_end(features, i, h)?;
            let cut = h
                .revolve(&hole::reach(features.upto(i)))
                .map_err(|error| RegenError::Hole { feature: i, error })?;
            let base = tool(i, features.id(i), &cut, build)?;
            let names = upto::names(&base.0);
            let t = match up {
                None => base,
                Some(u) => upto::build(i, features.id(i), base, &u.plan, &u.start, u.far, build)?,
            };
            if let (true, Some(m)) = (build, next) {
                // the flat bottom (or drill point) is the end
                let bottom = Name::feature(features.id(i)).with_str("side").with(hole::BOTTOM);
                upto::verify_next(
                    features,
                    i,
                    (&t.0, &t.2),
                    &names,
                    &[bottom.with(0), bottom.with(1)],
                    &m,
                )?;
            }
            let pe = |error| RegenError::Pattern { feature: i, error };
            let mut out = Vec::new();
            for (key, m) in h.copies() {
                let body = if build {
                    moved(&t.0, &m, |n| instance_name(features.id(i), key, n)).map_err(pe)?
                } else {
                    Body::new()
                };
                out.push((body, t.1, moved_shape(&t.2, &m)));
            }
            out.insert(0, t);
            Ok(out)
        }
        Feature::Pattern {
            of: Source::Features(src),
            layout,
        } => {
            let pe = |error| RegenError::Pattern { feature: i, error };
            let copies = instances(features, i, layout)?;
            let mut base = Vec::new();
            for &s in src {
                if s >= i || modifies(&features[s]) {
                    return Err(pe(PatternError::BadSource { feature: s }));
                }
                base.extend(solids_of(features, s, build)?);
            }
            let mut out = Vec::with_capacity(copies.len() * base.len());
            for (key, m) in &copies {
                for (b, op, sh) in &base {
                    let body = moved(b, m, |n| instance_name(features.id(i), *key, n)).map_err(pe)?;
                    let op = if *op == Combine::New { Combine::Add } else { *op };
                    out.push((body, op, moved_shape(sh, m)));
                }
            }
            Ok(out)
        }
        Feature::Extrude { .. } | Feature::Revolve { .. } => {
            let (t, names, next) = match extent::resolve(features, i)? {
                extent::Resolved::Plain(f, next) => {
                    let t = tool(i, features.id(i), &f, build)?;
                    let names = upto::names(&t.0);
                    (t, names, next)
                }
                extent::Resolved::UpTo(f, u) => {
                    let base = tool(i, features.id(i), &f, build)?;
                    let names = upto::names(&base.0);
                    (
                        upto::build(i, features.id(i), base, &u.plan, &u.start, u.far, build)?,
                        names,
                        u.next,
                    )
                }
            };
            if let (true, Some(m)) = (build, next) {
                let end = Name::feature(features.id(i)).with_str("end");
                upto::verify_next(features, i, (&t.0, &t.2), &names, &[end], &m)?;
            }
            Ok(vec![t])
        }
        Feature::Plane(_) | Feature::Param(_) | Feature::Sketch(_) | Feature::Work(_) => Ok(vec![]),
        Feature::Sweep(_) | Feature::Coil(_) | Feature::Loft(_) => {
            Ok(vec![swept::tool_of(features, i, build)?])
        }
        Feature::Thread(_) => thread::tools(features, i, build),
        Feature::Text(_) => text::tools(features, i),
        f => Ok(vec![tool(i, features.id(i), f, build)?]),
    }
}

/// The solids feature `s` stands for: its tool, or a pattern's sources' and its copies.
fn solids_of(features: History<'_>, s: usize, build: bool) -> Result<Vec<Tool>, RegenError> {
    let mut out = Vec::new();
    if let Feature::Pattern {
        of: Source::Features(src),
        ..
    } = &features[s]
    {
        for &t in src {
            if t < s && !modifies(&features[t]) {
                out.extend(solids_of(features, t, build)?);
            }
        }
    }
    out.extend(tools(features, s, build)?);
    Ok(out)
}

/// The solid an extrude, revolve or ball makes (empty unless `build`), and how it combines
/// with the part.
fn tool(i: usize, id: u64, f: &Feature, build: bool) -> Result<Tool, RegenError> {
    if !build {
        let (Feature::Extrude { op, .. } | Feature::Revolve { op, .. } | Feature::Ball { op, .. }) = f else {
            unreachable!("only extrudes, revolves and balls are built directly")
        };
        return Ok((Body::new(), *op, shape(f)));
    }
    Ok(match f {
        Feature::Extrude {
            profile,
            extent: Extent::Distance(distance),
            op,
        } => (
            extrude(profile, *distance, id).map_err(|error| RegenError::Extrude { feature: i, error })?,
            *op,
            shape(f),
        ),
        Feature::Revolve {
            profile,
            axis,
            extent: Turn::Angle(angle),
            op,
        } => (
            revolve(profile, *axis, *angle, id).map_err(|error| RegenError::Revolve { feature: i, error })?,
            *op,
            shape(f),
        ),
        Feature::Ball { frame, radius, op } => {
            let q = dmath::FRAC_PI_2;
            let half = ProfileLoop::new(vec![
                (
                    Segment::Arc {
                        center: Point2::ORIGIN,
                        radius: *radius,
                        start: -q,
                        sweep: 2.0 * q,
                    },
                    0,
                ),
                (
                    Segment::Line {
                        a: Point2::new(0.0, *radius),
                        b: Point2::new(0.0, -*radius),
                    },
                    1,
                ),
            ]);
            let profile = Profile {
                plane: *frame,
                loops: vec![half],
            };
            let axis = Axis2 {
                origin: Point2::ORIGIN,
                dir: Vec2::Y,
            };
            (
                revolve(&profile, axis, dmath::TAU, id)
                    .map_err(|error| RegenError::Revolve { feature: i, error })?,
                *op,
                shape(f),
            )
        }
        _ => unreachable!("only extrudes, revolves and balls are built directly"),
    })
}

fn seg2(s: &Segment) -> Seg2 {
    match *s {
        Segment::Line { a, b } => Seg2::Line { a, b },
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => Seg2::Arc {
            c: center,
            r: radius,
            start,
            sweep,
        },
        Segment::Circle { center, radius, ccw } => Seg2::Arc {
            c: center,
            r: radius,
            start: 0.0,
            sweep: if ccw { dmath::TAU } else { -dmath::TAU },
        },
    }
}

fn segs(profile: &Profile) -> Vec<Seg2> {
    profile
        .loops
        .iter()
        .flat_map(|l| l.segments.iter().map(|(s, _)| seg2(s)))
        .collect()
}

/// The solid a feature's tool is, analytically (as `extrude` and `revolve` build it).
fn shape(f: &Feature) -> Shape {
    match f {
        Feature::Extrude {
            profile,
            extent: Extent::Distance(distance),
            ..
        } => Shape::Prism {
            frame: profile.plane,
            segs: segs(profile),
            lo: distance.min(0.0),
            hi: distance.max(0.0),
        },
        Feature::Revolve {
            profile,
            axis,
            extent: Turn::Angle(angle),
            ..
        } => {
            let dir = axis.dir.normalize().unwrap_or(Vec2::Y);
            let perp = dir.perp();
            let mut side = 0.0f64;
            for l in &profile.loops {
                for (s, _) in &l.segments {
                    for p in s.samples(32) {
                        let r = (p - axis.origin).dot(perp);
                        side = if r.abs() > side.abs() { r } else { side };
                    }
                }
            }
            let side = if side < 0.0 { -1.0 } else { 1.0 };
            let plane = profile.plane;
            let vec3 = |v: Vec2| plane.x() * v.x + plane.y() * v.y;
            let frame = vec3(dir)
                .normalize()
                .and_then(|z| Frame::from_zx(plane.plane_point(axis.origin), z, vec3(perp * side)))
                .unwrap_or(plane);
            Shape::Revolved {
                frame,
                origin: axis.origin,
                dir,
                perp: perp * side,
                segs: segs(profile),
                angle: (*angle < dmath::TAU - forge_math::tol::ANGULAR).then_some(*angle),
            }
        }
        Feature::Ball { frame, radius, .. } => Shape::Ball {
            center: frame.origin,
            radius: *radius,
        },
        _ => unreachable!("only extrudes, revolves and balls have a shape of their own"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_bool::{Index, Member, Where};
    use forge_math::rng::Rng;

    /// A tool's analytic solid agrees with its B-rep wherever it answers.
    #[test]
    fn shapes_agree_with_their_tools() {
        let mut parts: Vec<Part> = suite::parts().into_iter().map(|p| p.1).collect();
        parts.extend((1..=10).map(|s| suite::fuzz(s, 80)));
        parts.push(torture::part(221));
        let mut r = Rng::new(77);
        let (mut answered, mut near) = (0, 0);
        for part in &parts {
            let solid = part.features.iter().enumerate().filter(|f| !modifies(f.1));
            // a few copies of each pattern, mirrored ones among them
            for ((i, f), (body, _, sh)) in solid.flat_map(|(i, f)| {
                let ts = tools(History::new(&part.features, None), i, true).expect("tools build");
                ts.into_iter().take(6).map(move |t| ((i, f), t))
            }) {
                if matches!(sh, Shape::Brep(_)) {
                    continue;
                }
                let index = Index::new(&body);
                let b = index.bbox.grow(1.0);
                for _ in 0..200 {
                    let p = forge_math::Point3::new(
                        r.range(b.x.lo, b.x.hi),
                        r.range(b.y.lo, b.y.hi),
                        r.range(b.z.lo, b.z.hi),
                    );
                    let want = match sh.member(p, 1e-4) {
                        Member::Near => {
                            near += 1;
                            continue;
                        }
                        Member::In => Where::Inside,
                        Member::Out => Where::Outside,
                    };
                    answered += 1;
                    assert_eq!(index.classify(p), want, "feature {i} {f:?} at {p:?}");
                }
            }
        }
        eprintln!("{answered} answered, {near} near");
        assert!(answered > 100 * near, "{answered} answered, {near} near");
    }
}
