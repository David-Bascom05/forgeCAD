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
use forge_bool::{BoolError, CpuBatch, Op, Seg2, Shape, boolean_into, boolean_unlinked};
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
    /// The result is no closed solid, first from this feature on: a boolean lost a face.
    NotClosed {
        feature: usize,
        what: String,
    },
    /// A feature that works on what it names was given nothing to work on.
    NoReferences {
        feature: usize,
    },
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
    /// A cut or keep leaves nothing of the part.
    Empty {
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
        closed_body(self, None, &|p, _| {
            nary_regen(p.history(), wasm_one(threads), true, Batching::On(backend))
        })
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
            return bodies::one(self.regenerate_nary_solids(threads)?);
        }
        let batching = batch.map_or(Batching::None, Batching::Given);
        closed_body(self, None, &|p, _| {
            nary_regen(p.history(), wasm_one(threads), shapes, batching)
        })
    }

    /// Every body the part ends with, rebuilt feature by feature.
    pub fn regenerate_solids(&self) -> Result<Vec<Solid>, RegenError> {
        self.regenerate_solids_ids(None)
    }

    /// `regenerate_solids`, each feature named from its id in `ids` (as `regenerate_ids`).
    pub fn regenerate_solids_ids(&self, ids: Option<&[u64]>) -> Result<Vec<Solid>, RegenError> {
        closed_solids(self, ids, &|p, ids| {
            bodies::sequential(History::new(&p.features, ids))
        })
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
        closed_solids(self, ids, &|p, ids| {
            let h = History::new(&p.features, ids);
            if bodies::multi(&p.features) {
                return bodies::nary(h, wasm_one(threads));
            }
            // one body: the same regen as `regenerate_nary`, so the names are its names
            let id = p.features.iter().position(starts).unwrap_or(0);
            let body = nary_regen(h, wasm_one(threads), true, Batching::On(Backend::from_env()))?;
            Ok(vec![Solid { id, body }])
        })
    }

    /// Rebuilds the solid from the first feature. Feature ids are their
    /// positions, so entity names stay stable when nothing upstream changes.
    pub fn regenerate(&self) -> Result<Body, RegenError> {
        self.regenerate_ids(None)
    }

    /// `regenerate`, each feature named from its id in `ids` (one per feature, nonzero,
    /// distinct): ids kept through inserts, deletes and moves keep every name below.
    pub fn regenerate_ids(&self, ids: Option<&[u64]>) -> Result<Body, RegenError> {
        if bodies::multi(&self.features) {
            return bodies::one(self.regenerate_solids_ids(ids)?);
        }
        closed_body(self, ids, &|p, ids| {
            let h = History::new(&p.features, ids);
            in_turn(h)?
                .map(finish)
                .ok_or(RegenError::NothingToCombine { feature: 0 })
        })
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

/// A regen of a part, each feature named from its id in the ids given.
type Regen<'a, T> = &'a dyn Fn(&Part, Option<&[u64]>) -> Result<T, RegenError>;

/// `regen`'s result if it is closed; else NotClosed, naming the first feature from which it is not
/// (found by regenerating shorter histories: only on the way to an error).
fn closed_result<T>(
    part: &Part,
    ids: Option<&[u64]>,
    regen: Regen<'_, T>,
    open: &dyn Fn(&T) -> Option<String>,
) -> Result<T, RegenError> {
    let made = regen(part, ids)?;
    let Some(what) = open(&made) else {
        return Ok(made);
    };
    // (sides of a knife edge within tolerance can leave the body open as well as refused)
    if let Some(all) = all_blunted(
        History::new(&part.features, ids),
        part.features.len().saturating_sub(1),
    ) && let Ok(b) = regen(&Part { features: all }, ids)
        && open(&b).is_none()
    {
        return Ok(b);
    }
    let feature = (0..part.features.len().saturating_sub(1))
        .find(|&k| {
            let shorter = Part {
                features: part.features[..=k].to_vec(),
            };
            regen(&shorter, ids.map(|d| &d[..=k])).is_ok_and(|m| open(&m).is_some())
        })
        .unwrap_or(part.features.len().saturating_sub(1));
    Err(RegenError::NotClosed { feature, what })
}

fn closed_body(part: &Part, ids: Option<&[u64]>, regen: Regen<'_, Body>) -> Result<Body, RegenError> {
    closed_result(part, ids, regen, &forge_topo::check::open_at)
}

fn closed_solids(
    part: &Part,
    ids: Option<&[u64]>,
    regen: Regen<'_, Vec<Solid>>,
) -> Result<Vec<Solid>, RegenError> {
    closed_result(part, ids, regen, &|solids: &Vec<Solid>| {
        solids.iter().find_map(|s| forge_topo::check::open_at(&s.body))
    })
}

/// Applies feature `i` to the part as features `..i` left it: `regenerate` one feature at a
/// time, for callers that keep every stage (each as the next step needs it; `finish` one to
/// show it). Only `features[..=i]` is read.
pub fn step(features: History<'_>, i: usize, body: Option<Body>) -> Result<Option<Body>, RegenError> {
    match step_plain(features, i, body) {
        Err(e) if lost_its_way(&e) => {
            let Some(fs) = blunted(features, i) else {
                return Err(e);
            };
            let Ok(so_far) = careful(features, 0, i, None) else {
                return Err(e);
            };
            step_plain(
                History {
                    features: &fs,
                    ids: features.ids,
                },
                i,
                so_far,
            )
            .map_err(|_| e)
        }
        r => r,
    }
}

pub(crate) fn step_plain(
    features: History<'_>,
    i: usize,
    body: Option<Body>,
) -> Result<Option<Body>, RegenError> {
    let b = step_on(features, i, body)?;
    if let Some(b) = &b {
        no_pinch(features, i, b)?;
    }
    Ok(b)
}

/// Features `from..to` from `body`, each extrude with knife edges tried plain and then blunted, for
/// which a copy is kept. Never nested: at most two tries of each feature, once.
fn careful(
    features: History<'_>,
    from: usize,
    to: usize,
    mut body: Option<Body>,
) -> Result<Option<Body>, RegenError> {
    for i in from..to {
        body = match blunted(features, i) {
            None => step_plain(features, i, body)?,
            Some(fs) => match step_plain(features, i, body.clone()) {
                Err(e) if lost_its_way(&e) => step_plain(
                    History {
                        features: &fs,
                        ids: features.ids,
                    },
                    i,
                    body,
                )
                .map_err(|_| e)?,
                r => r?,
            },
        };
    }
    Ok(body)
}

/// Every feature in turn. Should a boolean refuse with knife edges about: again trying each plain and
/// then blunted, and if that fails too with all of them blunted. Three passes at most, however many
/// knife edges: nothing nests.
fn in_turn(features: History<'_>) -> Result<Option<Body>, RegenError> {
    let plain = |h: History<'_>| (0..h.len()).try_fold(None, |b, i| step_plain(h, i, b));
    match plain(features) {
        Err(e) if lost_its_way(&e) => {
            let Some(all) = all_blunted(features, features.len().saturating_sub(1)) else {
                return Err(e);
            };
            careful(features, 0, features.len(), None)
                .or_else(|_| {
                    plain(History {
                        features: &all,
                        ids: features.ids,
                    })
                })
                .map_err(|_| e)
        }
        r => r,
    }
}

/// Features up to `i` with every knife edge blunted: for a part whose earlier tip a tool trips on.
pub(crate) fn rebuilt_blunted(features: History<'_>, i: usize) -> Option<Result<Option<Body>, RegenError>> {
    let all = all_blunted(features, i)?;
    let h = History {
        features: &all,
        ids: features.ids,
    };
    Some((0..=i).try_fold(None, |b, k| step_plain(h, k, b)))
}

/// The history with feature `i`, an extrude with knife edges in its profile, blunted.
pub(crate) fn blunted(features: History<'_>, i: usize) -> Option<Vec<Feature>> {
    let Feature::Extrude { profile, extent, op } = &features[i] else {
        return None;
    };
    let profile = profile.blunted()?;
    let mut fs = features.features.to_vec();
    fs[i] = Feature::Extrude {
        profile,
        extent: *extent,
        op: *op,
    };
    Some(fs)
}

/// The history with every extrude's knife edges blunted up to feature `to`: a tip already in the part
/// can be what a later tool trips on.
pub(crate) fn all_blunted(features: History<'_>, to: usize) -> Option<Vec<Feature>> {
    let mut fs = features.features.to_vec();
    let mut any = false;
    fn blunt(f: &mut Feature) -> bool {
        match f {
            Feature::Extrude { profile, .. } => profile.blunted().map(|b| *profile = b).is_some(),
            Feature::Body(b) => match &mut **b {
                BodyFeature::In { feature, .. } => blunt(feature),
                _ => false,
            },
            _ => false,
        }
    }
    for f in fs.iter_mut().take(to + 1) {
        any |= blunt(f);
    }
    any.then_some(fs)
}

/// A boolean's refusal that sides within tolerance of each other can cause.
pub(crate) fn lost_its_way(e: &RegenError) -> bool {
    matches!(
        e,
        RegenError::Boolean {
            error: BoolError::ZeroThickness { .. } | BoolError::Internal(_) | BoolError::Unsupported(_),
            ..
        }
    )
}

/// Whether a history has an extrude whose loops touch: its result is judged step by step.
pub(crate) fn any_touching(features: &[Feature]) -> bool {
    fn touches(f: &Feature) -> bool {
        match f {
            Feature::Extrude { profile, .. } => profile.loops.len() > 1 && !profile.touching().is_empty(),
            Feature::Body(b) => matches!(&**b, BodyFeature::In { feature, .. } if touches(feature)),
            _ => false,
        }
    }
    features.iter().any(touches)
}

/// The profile of an extrude whose loops touch, and how far its solid reaches each way.
fn touching_extrude(features: History<'_>, i: usize) -> Option<(Profile, Vec<forge_ops::Touch>, f64, f64)> {
    let Feature::Extrude { profile, .. } = &features[i] else {
        return None;
    };
    if profile.loops.len() < 2 {
        return None;
    }
    if profile.touching().is_empty() {
        return None;
    }
    let p = profile.normalized_touching().ok()?;
    let touches = p.touching();
    let extent::Resolved::Plain(f, _) = extent::resolve(features, i).ok()? else {
        return None;
    };
    let Feature::Extrude {
        extent: Extent::Distance(d),
        profile: at,
        ..
    } = &*f
    else {
        return None;
    };
    let base = profile.plane.z().dot(at.plane.origin - profile.plane.origin);
    Some((p, touches, base.min(base + d), base.max(base + d)))
}

/// An extrude's loops touching leave its result pinched there where material meets itself along
/// that line: refused, as any other touch. Where the line lands in material or in empty space, or
/// only separate solids meet, the result is sound (research/build-failures).
pub(crate) fn no_pinch(features: History<'_>, i: usize, body: &Body) -> Result<(), RegenError> {
    let Some((profile, touches, lo, hi)) = touching_extrude(features, i) else {
        return Ok(());
    };
    // a body the loops leave unsound (meeting twice, they divide the profile into solids one body's
    // faces cannot hold) is refused like the pinch
    if !forge_topo::check(body).is_empty() {
        let at = profile.plane.plane_point(touches[0].at) + profile.plane.z() * (0.5 * (lo + hi));
        return Err(RegenError::Boolean {
            feature: i,
            error: forge_bool::BoolError::ZeroThickness { at },
        });
    }
    let ix = forge_bool::Index::new(body);
    for t in &touches {
        for h in [0.25, 0.5, 0.75] {
            let z = lo + h * (hi - lo);
            let at = |q: Point2| profile.plane.plane_point(q) + profile.plane.z() * z;
            // along the tangent the two wedges between the loops, across it the two sides
            let (holes, other) = (&profile.loops[t.hole], &profile.loops[t.other]);
            let wedge = |sign: f64| {
                let mut r = 0.01;
                loop {
                    let q = t.at + t.tangent * (sign * r);
                    let (a, b) = (holes.nearest(q), other.nearest(q));
                    if a.distance(b) >= 1e-3 || r > 5.0 {
                        return Point2::new(0.5 * (a.x + b.x), 0.5 * (a.y + b.y));
                    }
                    r *= 2.0;
                }
            };
            let (w0, w1) = (wedge(1.0), wedge(-1.0));
            // round the point: a ring, and the two wedges a ring that size would pass between
            let r = 0.5 * w0.distance(t.at).min(w1.distance(t.at)).max(0.04);
            let mut round: Vec<(f64, Point2)> = (0..24)
                .map(|k| {
                    let a = dmath::TAU * k as f64 / 24.0;
                    (a, t.at + Vec2::new(dmath::cos(a), dmath::sin(a)) * r)
                })
                .collect();
            for w in [w0, w1] {
                let d = w - t.at;
                round.push((dmath::atan2(d.y, d.x).rem_euclid(dmath::TAU), w));
            }
            round.sort_by(|x, y| x.0.total_cmp(&y.0));
            let states: Vec<bool> = round
                .iter()
                .filter_map(|&(_, q)| match ix.classify(at(q)) {
                    forge_bool::Where::On => None,
                    w => Some(w == forge_bool::Where::Inside),
                })
                .collect();
            let changes = (0..states.len())
                .filter(|&k| states[k] != states[(k + 1) % states.len()])
                .count();
            // material comes and goes twice round the line: it meets itself there
            if changes >= 4 && connected_through(body, &ix, at(t.at)) {
                return Err(RegenError::Boolean {
                    feature: i,
                    error: forge_bool::BoolError::ZeroThickness { at: at(t.at) },
                });
            }
        }
    }
    Ok(())
}

/// Whether the faces at `p` are all one solid, joined by edges: separate solids only touching
/// there are sound.
fn connected_through(body: &Body, ix: &forge_bool::Index<'_>, p: forge_math::Point3) -> bool {
    let mut lump: Vec<usize> = (0..body.faces.len()).collect();
    fn root(l: &mut [usize], mut i: usize) -> usize {
        while l[i] != i {
            l[i] = l[l[i]];
            i = l[i];
        }
        i
    }
    for e in body.edge_ids() {
        let cs = &body.edge(e).coedges;
        if let [a, b, ..] = cs[..] {
            let (x, y) = (body.coedge_face(a).index(), body.coedge_face(b).index());
            let (x, y) = (root(&mut lump, x), root(&mut lump, y));
            lump[x] = y;
        }
    }
    let mut first = None;
    for f in body.face_ids() {
        if ix.face_box(f).grow(forge_math::tol::LINEAR).contains(p)
            && ix.point_in_face(f, p) != forge_bool::Where::Outside
        {
            let l = root(&mut lump, f.index());
            match first {
                None => first = Some(l),
                Some(o) if o != l => return false,
                _ => {}
            }
        }
    }
    true
}

fn step_on(features: History<'_>, i: usize, mut body: Option<Body>) -> Result<Option<Body>, RegenError> {
    let f = &features[i];
    let id = features.id(i);
    if modifies(f) {
        let b = body.ok_or(RegenError::NothingToCombine { feature: i })?;
        // a part-changing feature works on the finished part
        let b = modify(features, f, i, &finish(b), 1)?;
        if trace::on() {
            trace::step(i, &b, None);
        }
        return Ok(Some(b));
    }
    let err = |error| RegenError::Boolean { feature: i, error };
    if let Some((tool, op)) = unlinked(features, i)
        && let Some(mut b) = body.take()
    {
        boolean_unlinked(&mut b, tool, op_of(op)).map_err(err)?;
        if b.dead > b.faces.len() / 2 {
            b = b.compact();
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
    if let [(tool, op, _)] = &tools[..] {
        // in place: only the faces near the tool change
        boolean_into(&mut b, tool, op_of(*op), id + 1_000_000).map_err(err)?;
        b = not_empty(b, i)?;
        // tombstones are cheap until they outnumber the living
        if b.dead > b.faces.len() / 2 {
            b = b.into_compact();
        }
    } else if !tools.is_empty() {
        // a pattern's copies all in one pass
        let base = if b.dead > 0 { b.into_compact() } else { b };
        let mut all = vec![&base];
        all.extend(tools.iter().map(|t| &t.0));
        let ops: Vec<Op> = tools.iter().map(|t| op_of(t.1)).collect();
        let mut solids = vec![part_shape(&base)];
        solids.extend(tools.iter().map(|t| t.2.clone()));
        b = not_empty(
            forge_bool::nary_with(&all, &ops, Some(&solids), None, 1, cut_root(i)).map_err(err)?,
            i,
        )?;
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

/// Whether a cut or keep among `tools`, applied in turn after `first`, may leave nothing: a cut
/// whose solid's box holds the box of all before it, or a keep whose box misses it. The n-ary
/// pass never sees the part in between, where the feature-by-feature regen refuses an empty
/// one however later tools add to it; such histories are rebuilt feature by feature instead.
fn may_empty(first: &Body, tools: &[Tool]) -> bool {
    let bx = |b: &Body| {
        b.face_ids()
            .fold(forge_math::Box3::EMPTY, |a, f| a.hull(&b.face_box(f)))
    };
    let holds = |a: &forge_math::Box3, b: &forge_math::Box3| {
        let h = a.grow(forge_math::tol::LINEAR);
        h.x.lo <= b.x.lo
            && h.x.hi >= b.x.hi
            && h.y.lo <= b.y.lo
            && h.y.hi >= b.y.hi
            && h.z.lo <= b.z.lo
            && h.z.hi >= b.z.hi
    };
    let mut so_far = bx(first);
    for (b, op, _) in tools {
        let t = bx(b);
        match op {
            Combine::Cut if holds(&t, &so_far) => return true,
            Combine::Keep if !t.grow(forge_math::tol::LINEAR).overlaps(&so_far) => return true,
            Combine::Keep => {
                let cut = |a: forge_math::Interval, b: forge_math::Interval| {
                    forge_math::Interval::new(a.lo.max(b.lo), a.hi.min(b.hi).max(a.lo.max(b.lo)))
                };
                so_far = forge_math::Box3 {
                    x: cut(so_far.x, t.x),
                    y: cut(so_far.y, t.y),
                    z: cut(so_far.z, t.z),
                };
            }
            Combine::Add | Combine::New => so_far = so_far.hull(&t),
            Combine::Cut => {}
        }
    }
    false
}

/// Features `..` rebuilt one at a time (the part between them seen, an empty one refused).
fn one_at_a_time(features: History<'_>) -> Result<Body, RegenError> {
    in_turn(features)?
        .map(Body::into_compact)
        .ok_or(RegenError::NothingToCombine { feature: 0 })
}

/// `b`, unless feature `i` left nothing of it.
pub(crate) fn not_empty(b: Body, i: usize) -> Result<Body, RegenError> {
    if b.face_ids().next().is_none() {
        return Err(RegenError::Empty { feature: i });
    }
    Ok(b)
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

/// Feature `i`'s tool solids as the regen combines them, for checks from outside (forge-proof).
pub fn feature_tools(features: History<'_>, i: usize) -> Result<Vec<(Body, Combine)>, RegenError> {
    Ok(tools(features, i, true)?
        .into_iter()
        .map(|t| (t.0, t.1))
        .collect())
}

/// The part (one body) as features before `i` leave it.
pub(crate) fn part_before(features: History<'_>, i: usize) -> Result<Body, RegenError> {
    let mut body = None;
    for k in 0..i {
        body = step(features.upto(i), k, body)?;
    }
    body.map(finish)
        .ok_or(RegenError::NothingToCombine { feature: i })
}

/// The tools of features `from..` on up to `threads` threads, in order (only their shapes,
/// solids left empty, unless `build`).
fn build_tools(
    features: History<'_>,
    from: usize,
    threads: usize,
    build: bool,
) -> Result<Vec<Tool>, RegenError> {
    Ok(distinct(build_each(features, from, threads, build)?))
}

/// `tools` without a tool equal to an earlier one combined the same way with only tools
/// combined that way between them (the same feature twice, say): two equal solids in one n-ary
/// pass meet everywhere, and the second changes nothing. A tool combined otherwise between
/// them (a ball cut, added back, cut again) makes the second count. Equal means the same
/// analytic shape; a shape known only by its B-rep is kept.
fn distinct(tools: Vec<Tool>) -> Vec<Tool> {
    let how = |c: Combine| if c == Combine::New { Combine::Add } else { c };
    // a cheap key first: only tools that share it are compared whole
    let key = |s: &Shape| -> Option<[u64; 3]> {
        let p = match s {
            Shape::Prism { frame, .. } | Shape::Revolved { frame, .. } | Shape::Coil { frame, .. } => {
                frame.origin
            }
            Shape::Ball { center, .. } => *center,
            Shape::Csg { .. } | Shape::Brep(_) => return None,
        };
        Some([p.x.to_bits(), p.y.to_bits(), p.z.to_bits()])
    };
    // sorted by key, then by place: within a key the earliest is kept
    let mut keyed: Vec<([u64; 3], usize)> = tools
        .iter()
        .enumerate()
        .filter_map(|(i, t)| key(&t.2).map(|k| (k, i)))
        .collect();
    keyed.sort_unstable();
    let mut drop = vec![false; tools.len()];
    for run in keyed.chunk_by(|a, b| a.0 == b.0) {
        for (n, &(_, i)) in run.iter().enumerate() {
            if run[..n].iter().any(|&(_, j)| {
                !drop[j]
                    && tools[j].2 == tools[i].2
                    && tools[j..i].iter().all(|t| how(t.1) == how(tools[i].1))
            }) {
                drop[i] = true;
            }
        }
    }
    tools
        .into_iter()
        .zip(drop)
        .filter_map(|(t, d)| (!d).then_some(t))
        .collect()
}

fn build_each(
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

/// Tools after the part so far up to which a run takes the sequential regen's booleans, one
/// feature at a time: below it the n-ary pass costs more (research/nary-short-runs).
pub(crate) const SHORT_RUN: usize = 16;

/// Whether features `from..` are a run short enough for the sequential booleans: a few features
/// of one tool each (a pattern's copies, a thread's or a text's tools take an n-ary pass over the
/// whole part in the sequential regen too).
fn short(features: History<'_>, from: usize) -> bool {
    let mut n = 0;
    for f in features.iter().skip(from) {
        match f {
            Feature::Pattern { .. } | Feature::Thread(_) | Feature::Text(_) => return false,
            Feature::Plane(_) | Feature::Param(_) | Feature::Sketch(_) | Feature::Work(_) => {}
            _ => n += 1,
        }
    }
    n <= SHORT_RUN
}

/// Features `from..` applied to `b` one at a time, exactly as the sequential regen applies them:
/// the same body as the n-ary pass, for less work on a short run. None where one refuses (the
/// caller takes the n-ary pass from the start).
fn sequential(features: History<'_>, from: usize, b: Body) -> Option<Body> {
    let mut b = Some(b);
    for i in from..features.len() {
        b = step(features, i, b).ok()?;
    }
    b.map(Body::into_compact)
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

thread_local! {
    static NAME_FALLBACKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many n-ary regens on this thread have been rebuilt feature by feature because a later
/// feature's reference was named as only the sequential regen names it (docs/naming.md: a face
/// split into several kept pieces). A part that relies on this costs a sequential regen on top.
pub fn nary_name_fallbacks() -> usize {
    NAME_FALLBACKS.with(std::cell::Cell::get)
}

pub(crate) fn count_name_fallback() {
    NAME_FALLBACKS.with(|c| c.set(c.get() + 1));
}

/// Whether a feature that works on what it names names nothing (a shell's open faces may be none).
pub(crate) fn names_nothing(f: &Feature) -> bool {
    match f {
        Feature::Fillet { edges, .. } => edges.is_empty(),
        Feature::Chamfer { edges, .. } => edges.is_empty(),
        Feature::FaceFillet { first, second, .. } => first.is_empty() || second.is_empty(),
        Feature::Draft { faces, .. } => faces.is_empty(),
        Feature::Edit(
            Edit::Offset { faces, .. }
            | Edit::Thicken { faces, .. }
            | Edit::Delete { faces }
            | Edit::Move { faces, .. },
        ) => faces.is_empty(),
        _ => false,
    }
}

/// Whether a regen failed because a feature's reference names nothing on the part.
pub(crate) fn names_missing(e: &RegenError) -> bool {
    use forge_ops::DirectError;
    matches!(
        e,
        RegenError::MissingEdge { .. }
            | RegenError::Chamfer {
                error: ChamferError::NoSuchEdge { .. },
                ..
            }
            | RegenError::Shell {
                error: ShellError::NoSuchFace { .. },
                ..
            }
            | RegenError::Draft {
                error: DraftError::NoSuchFace { .. },
                ..
            }
            | RegenError::Edit {
                error: EditError::Direct(DirectError::NoSuchFace { .. }),
                ..
            }
    )
}

/// The n-ary regen of a history: each run of solid features after a part-changing one (chamfer,
/// shell, draft, body pattern) is one n-ary boolean starting from the part so far.
fn nary_regen(
    features: History<'_>,
    threads: usize,
    shapes: bool,
    batching: Batching,
) -> Result<Body, RegenError> {
    if any_touching(&features) {
        return one_at_a_time(features);
    }
    nary_regen_on(features, threads, shapes, batching)
}

/// `nary_regen`, its history known to have no touching loops.
fn nary_regen_on(
    features: History<'_>,
    threads: usize,
    shapes: bool,
    batching: Batching,
) -> Result<Body, RegenError> {
    let r = match nary_run(features, threads, shapes, batching) {
        // every point the pass could probe a region at lies on another tool's edge: the tools one
        // at a time never have that question, and answer the same way (or refuse more clearly)
        Err(RegenError::Boolean {
            error: BoolError::Unsupported("region probe on another tool's edge") | BoolError::Internal(_),
            ..
        }) => one_at_a_time(features),
        Err(e) if names_missing(&e) => {
            count_name_fallback();
            one_at_a_time(features)
        }
        // (one at a time, a tool's knife edge can be blunted)
        Err(e) if lost_its_way(&e) && (0..features.len()).any(|i| blunted(features, i).is_some()) => {
            one_at_a_time(features)
        }
        r => r,
    };
    not_empty(r.map(unified)?, features.len() - 1)
}

/// `nary_regen`, its result not yet unified.
fn nary_run(
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
                &nary_regen_on(
                    features.upto(m),
                    threads,
                    shapes,
                    match batching {
                        Batching::Given(_) => Batching::None,
                        b => b,
                    },
                )
                .map(|b| if b.dead > 0 { b.compact_in_shells() } else { b })?,
                threads,
            )
        };
        let mut body = modified()?;
        if short(features, m + 1) {
            if let Some(b) = sequential(features, m + 1, body) {
                return Ok(b);
            }
            // one refused: the n-ary pass from the part as it was
            body = modified()?;
        }
        let built = build_tools(features, m + 1, threads, true)?;
        if may_empty(&body, &built) {
            return one_at_a_time(features);
        }
        let mut tools: Vec<&Body> = vec![&body];
        tools.extend(built.iter().map(|t| &t.0));
        let ops: Vec<Op> = built.iter().map(|t| op_of(t.1)).collect();
        let mut solids = vec![part_shape(&body)];
        solids.extend(built.iter().map(|t| t.2.clone()));
        return forge_bool::nary_with(
            &tools,
            &ops,
            Some(&solids),
            None,
            threads,
            cut_root(features.len() - 1),
        )
        .map_err(err);
    }
    if features.get(start).is_none_or(|f| !starts(f)) {
        return Err(RegenError::NothingToCombine { feature: 0 });
    }
    if short(features, start + 1) {
        let base = tools(features, start, true)?.swap_remove(0).0;
        if let Some(b) = sequential(features, start + 1, base) {
            return Ok(b);
        }
    }
    let built = build_tools(features, start, threads, true)?;
    if built.len() > 1 && may_empty(&built[0].0, &built[1..]) {
        return one_at_a_time(features);
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

/// The part as regeneration hands it out: faces on one surface and edges on one curve joined
/// (`forge_bool::unify`), compacted. `step` leaves this to its caller, so the next step builds
/// on the pieces the n-ary regen also sees.
pub fn finish(mut b: Body) -> Body {
    forge_bool::unify(&mut b);
    b.into_compact()
}

/// `b` unified; compacted only once most of it is dead, as after a boolean (a feature that
/// changes the part compacts its own input).
fn unified(mut b: Body) -> Body {
    forge_bool::unify(&mut b);
    // joining faces leaves shells as they were
    if b.dead > b.faces.len() / 2 {
        b.compact_in_shells()
    } else {
        b
    }
}

/// The part so far as a tool's solid: it has no analytic one, so its B-rep decides every point
/// in its box.
pub(crate) fn part_shape(b: &Body) -> Shape {
    Shape::Brep(forge_bool::Index::new(b).bbox)
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
    if names_nothing(f) {
        return Err(RegenError::NoReferences { feature: i });
    }
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
            // face ids of the body the fillet compacts, not of one with tombstones
            let body = &*live(body);
            let faces = |ns: &[Name]| {
                ns.iter()
                    .map(|&n| face_of(i, body, n))
                    .collect::<Result<Vec<_>, _>>()
            };
            face_fillet(body, &faces(first)?, &faces(second)?, *radius, id)
                .map_err(|error| RegenError::Fillet { feature: i, error })
        }
        Feature::FullRound { side, middle, other } => {
            let body = &*live(body);
            full_round(
                body,
                face_of(i, body, *side)?,
                face_of(i, body, *middle)?,
                face_of(i, body, *other)?,
                id,
            )
            .map_err(|error| RegenError::Fillet { feature: i, error })
        }
        Feature::Chamfer { edges, size } => {
            // a selector gives edges in arena order, which must not name anything
            let mut edges = edges.clone();
            edges.sort();
            chamfer(body, &edges, *size, id).map_err(|error| RegenError::Chamfer { feature: i, error })
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
/// `body` without tombstones.
fn live(body: &Body) -> std::borrow::Cow<'_, Body> {
    if body.dead > 0 {
        std::borrow::Cow::Owned(body.compact())
    } else {
        std::borrow::Cow::Borrowed(body)
    }
}

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
    // a selector gives edges in arena order, which must not name anything
    want.sort_by_key(|e| body.edge(e.edge).name);
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

/// A plain extrude's tool as its entities, for the local imprint, which may need no Body of it.
/// None where the imprint is off, or the extrude is not one tool of its own.
fn unlinked(features: History<'_>, i: usize) -> Option<(forge_topo::Unlinked, Combine)> {
    if forge_bool::imprint::mode() == forge_bool::imprint::Mode::Off || trace::on() {
        return None;
    }
    let Feature::Extrude {
        extent: Extent::Distance(_) | Extent::Through(_) | Extent::Symmetric(_) | Extent::TwoSided { .. },
        op: op @ (Combine::Cut | Combine::Add),
        ..
    } = &features[i]
    else {
        return None;
    };
    let extent::Resolved::Plain(f, None) = extent::resolve(features, i).ok()? else {
        return None;
    };
    let Feature::Extrude {
        profile,
        extent: Extent::Distance(d),
        ..
    } = &*f
    else {
        return None;
    };
    Some((
        forge_ops::extrude_unlinked(profile, *d, features.id(i)).ok()?,
        *op,
    ))
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
            let mut reached = Vec::new();
            for &s in src {
                if s >= i || modifies(&features[s]) {
                    return Err(pe(PatternError::BadSource { feature: s }));
                }
                sources_of(features, s, &mut reached);
            }
            let mut base = Vec::new();
            for s in reached {
                base.extend(tools(features, s, build)?);
            }
            // a sketch or a parameter makes nothing to copy: refused, not a pattern left out
            if base.is_empty() {
                return Err(pe(PatternError::BadSource {
                    feature: src.first().copied().unwrap_or(i),
                }));
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

/// The features whose solids feature `s` stands for, added to `out` once each: a pattern's
/// sources' (a source listed again, or reached through another pattern, would cut twice in one
/// pass), then its own.
fn sources_of(features: History<'_>, s: usize, out: &mut Vec<usize>) {
    if let Feature::Pattern {
        of: Source::Features(src),
        ..
    } = &features[s]
    {
        for &t in src {
            if t < s && !modifies(&features[t]) {
                sources_of(features, t, out);
            }
        }
    }
    if !out.contains(&s) {
        out.push(s);
    }
}

/// The solid an extrude, revolve or ball makes (empty unless `build`), and how it combines
/// with the part.
fn tool(i: usize, id: u64, f: &Feature, build: bool) -> Result<Tool, RegenError> {
    if let Feature::Extrude {
        profile,
        extent: Extent::Tapered { distance, angle },
        op,
    } = f
    {
        let slabs = forge_ops::tapered(profile, *distance, *angle, id).map_err(|e| match e {
            forge_ops::TaperError::Extrude(error) => RegenError::Extrude { feature: i, error },
            forge_ops::TaperError::Draft(error) => RegenError::Draft { feature: i, error },
        })?;
        let mut b = Body::new();
        for (k, s) in slabs.into_iter().enumerate() {
            b = if k == 0 {
                s
            } else {
                forge_bool::boolean(&b, &s, Op::Union, (id + 1_000_000) | ((k as u64) << 40))
                    .map_err(|error| RegenError::Boolean { feature: i, error })?
            };
        }
        // the side leans either way, so it is the built body that bounds it
        let sh = part_shape(&b);
        return Ok((if build { b } else { Body::new() }, *op, sh));
    }
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
