//! A part as a history of features, and regenerating it into a solid.

pub mod adversarial;
pub mod extent;
pub mod fillets;
pub mod finishing;
pub mod hole;
pub mod pattern;
pub mod suite;
pub mod swept;
pub mod torture;
mod trace;
mod upto;

pub use extent::{Extent, ExtentError, Target, Toward, Turn};
use forge_bool::{BoolError, CpuBatch, Op, Seg2, Shape, boolean_into};
use forge_gpu::AutoBatch;
pub use forge_gpu::{Backend, open_in_background as open_gpu};
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_ops::{
    Axis2, ChamferError, ChamferSize, DraftError, ExtrudeError, FilletEdge, FilletError, Profile,
    ProfileLoop, Radius, RevolveError, Segment, ShellError, chamfer, draft, extrude, fillet, revolve, shell,
};
use forge_topo::{Body, Name};
pub use hole::{End, Head, Hole, HoleError, Thread};
pub use pattern::{Layout, PatternError, Row, Source, instance_name};
use pattern::{instances, moved, moved_shape};
pub use swept::{Coil, CoilSize, Loft, PathEnds, Sweep};

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
    Fillet { edges: Vec<(Name, Radius)> },
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combine {
    /// Start the part (only valid as the first feature).
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
    /// A feature names an edge the part no longer has.
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
    /// A feature that combines with or changes the part came before any part existed.
    NothingToCombine {
        feature: usize,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    pub features: Vec<Feature>,
}

/// A tool: its solid, how it combines with the part, and the solid as an analytic shape.
type Tool = (Body, Combine, Shape);

impl Part {
    /// Rebuilds the solid in one n-ary boolean over every feature's tool (built in parallel),
    /// on up to `threads` threads.
    /// The region batch is classified on the backend FORGE_BATCH names (default Auto). A
    /// long-running app calls `forge_model::open_gpu()` at startup so Auto has the GPU at once.
    pub fn regenerate_nary(&self, threads: usize) -> Result<Body, RegenError> {
        self.regenerate_nary_on(threads, Backend::from_env())
    }

    /// `regenerate_nary`, the region batch classified on `backend`.
    pub fn regenerate_nary_on(&self, threads: usize, backend: Backend) -> Result<Body, RegenError> {
        let (shapes, ops) = self.nary_solids()?;
        let cpu = CpuBatch {
            shapes: &shapes,
            ops: &ops,
            threads: if cfg!(target_family = "wasm") { 1 } else { threads },
        };
        self.regenerate_nary_batch(threads, true, Some(&AutoBatch::new(cpu, backend)))
    }

    /// `regenerate_nary`, placing points against the tools' analytic solids first or not.
    pub fn regenerate_nary_with(&self, threads: usize, shapes: bool) -> Result<Body, RegenError> {
        self.regenerate_nary_batch(threads, shapes, None)
    }

    /// The tools' analytic solids and ops, in the order the n-ary regen uses them (for building
    /// a batch classifier).
    pub fn nary_solids(&self) -> Result<(Vec<Shape>, Vec<Op>), RegenError> {
        let start = self.features.iter().rposition(starts).unwrap_or(0);
        let start = match self.features.iter().rposition(modifies) {
            Some(m) if m >= start => m + 1,
            _ => start,
        };
        let built = build_tools(&self.features, start, 1, false)?;
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
        // wasm has no threads
        let threads = if cfg!(target_family = "wasm") { 1 } else { threads };
        nary_regen(&self.features, threads, shapes, batch)
    }

    /// Rebuilds the solid from the first feature. Feature ids are their
    /// positions, so entity names stay stable when nothing upstream changes.
    pub fn regenerate(&self) -> Result<Body, RegenError> {
        let mut body = None;
        for i in 0..self.features.len() {
            body = step(&self.features, i, body)?;
        }
        body.map(|b| b.compact())
            .ok_or(RegenError::NothingToCombine { feature: 0 })
    }

    /// Every cosmetic thread (each copy of a patterned hole has its own): the thread, the hole's
    /// axis (origin at the mouth, z down the drill) and the wall's radius.
    pub fn threads(&self) -> Vec<(Thread, Frame, f64)> {
        // each thread and the features whose solids hold it
        let mut out: Vec<(Thread, Frame, f64, Vec<usize>)> = Vec::new();
        for (i, f) in self.features.iter().enumerate() {
            match f {
                Feature::Hole(h) => {
                    if let Some(t) = h.thread {
                        out.push((t, h.at, h.diameter / 2.0, vec![i]));
                    }
                }
                Feature::Pattern { of, layout } => {
                    let Ok(copies) = instances(layout) else {
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
pub fn step(features: &[Feature], i: usize, body: Option<Body>) -> Result<Option<Body>, RegenError> {
    let f = &features[i];
    let id = i as u64 + 1;
    if modifies(f) {
        let b = body.ok_or(RegenError::NothingToCombine { feature: i })?;
        let b = modify(features, i, &b, 1)?;
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
    if matches!(f, Feature::Plane(_)) {
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
        b = forge_bool::nary_with(&all, &ops, None, None, 1, cut_root(i))
            .map_err(err)?
            .compact();
    }
    if trace::on() {
        trace::step(i, &b, tools.first().map(|t| &t.0));
    }
    Ok(Some(b))
}

/// Whether a feature starts the part afresh.
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
            | Feature::Chamfer { .. }
            | Feature::Shell { .. }
            | Feature::Draft { .. }
            | Feature::Pattern { of: Source::Body, .. }
    )
}

/// The tools of features `from..` on up to `threads` threads, in order (only their shapes,
/// solids left empty, unless `build`).
fn build_tools(
    features: &[Feature],
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
    let chunk = n.div_ceil(threads);
    let built: Vec<Result<Vec<Tool>, RegenError>> = std::thread::scope(|s| {
        let hs: Vec<_> = (from..features.len())
            .step_by(chunk)
            .map(|a| {
                let some = &some;
                s.spawn(move || some(a..(a + chunk).min(features.len())))
            })
            .collect();
        hs.into_iter().map(|h| h.join().expect("tool builder")).collect()
    });
    let mut out = Vec::new();
    for b in built {
        out.extend(b?);
    }
    Ok(out)
}

/// The n-ary regen of a history: each run of solid features after a part-changing one (chamfer,
/// shell, draft, body pattern) is one n-ary boolean starting from the part so far.
fn nary_regen(
    features: &[Feature],
    threads: usize,
    shapes: bool,
    batch: Option<&dyn forge_bool::Batch>,
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
        let body = modify(
            features,
            m,
            // the batch holds the tools after the last modifier (nary_solids), not these
            &nary_regen(&features[..m], threads, shapes, None)?,
            threads,
        )?;
        let built = build_tools(features, m + 1, threads, true)?;
        if built.is_empty() {
            return Ok(body);
        }
        let mut tools: Vec<&Body> = vec![&body];
        tools.extend(built.iter().map(|t| &t.0));
        let ops: Vec<Op> = built.iter().map(|t| op_of(t.1)).collect();
        return forge_bool::nary_with(&tools, &ops, None, None, threads, cut_root(features.len() - 1))
            .map(|b| b.compact())
            .map_err(err);
    }
    if features.get(start).is_none_or(|f| !starts(f)) {
        return Err(RegenError::NothingToCombine { feature: 0 });
    }
    let built = build_tools(features, start, threads, true)?;
    let tools: Vec<&Body> = built.iter().map(|t| &t.0).collect();
    let ops: Vec<Op> = built[1..].iter().map(|t| op_of(t.1)).collect();
    let solids: Vec<Shape> = built.iter().map(|t| t.2.clone()).collect();
    forge_bool::nary_with(
        &tools,
        &ops,
        shapes.then_some(&solids[..]),
        batch,
        threads,
        cut_root(features.len() - 1),
    )
    .map(|b| b.compact())
    .map_err(err)
}

/// The root of the names a boolean ending at feature `i` gives the vertices it makes.
fn cut_root(i: usize) -> Name {
    Name::feature(i as u64 + 1 + 1_000_000)
}

fn op_of(c: Combine) -> Op {
    match c {
        Combine::Add => Op::Union,
        Combine::Cut => Op::Subtract,
        _ => Op::Intersect,
    }
}

/// Applies part-changing feature `i` to `body`.
fn modify(features: &[Feature], i: usize, body: &Body, threads: usize) -> Result<Body, RegenError> {
    let id = i as u64 + 1;
    match &features[i] {
        Feature::Fillet { edges } => apply_fillet(i, body, edges),
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
            let copies = instances(layout).map_err(pe)?;
            let copies = copies
                .iter()
                .map(|(key, m)| moved(body, m, |n| instance_name(i, *key, n)))
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
            .map(|b| b.compact())
            .map_err(|error| RegenError::Boolean { feature: i, error })
        }
        _ => unreachable!("solid features make tools"),
    }
}

/// Feature `i`, a fillet of the named edges, applied to `body`.
fn apply_fillet(i: usize, body: &Body, edges: &[(Name, Radius)]) -> Result<Body, RegenError> {
    let body = if body.dead > 0 {
        body.compact()
    } else {
        body.clone()
    };
    let mut want: Vec<FilletEdge> = Vec::with_capacity(edges.len());
    for (name, radius) in edges {
        let name = *name;
        let edge = body
            .edge_by_name(name)
            .ok_or(RegenError::MissingEdge { feature: i, name })?;
        want.push(FilletEdge {
            edge,
            radius: radius.clone(),
        });
    }
    fillet(&body, &want, i as u64 + 1).map_err(|error| RegenError::Fillet { feature: i, error })
}

/// The tools feature `i` makes: one, or a pattern's copies (each named after its place).
fn tools(features: &[Feature], i: usize, build: bool) -> Result<Vec<Tool>, RegenError> {
    match &features[i] {
        Feature::Hole(h) => {
            let (h, up, next) = extent::hole_end(features, i, h)?;
            let cut = h
                .revolve(&hole::reach(&features[..i]))
                .map_err(|error| RegenError::Hole { feature: i, error })?;
            let base = tool(i, &cut, build)?;
            let names = upto::names(&base.0);
            let t = match up {
                None => base,
                Some(u) => upto::build(i, base, &u.plan, &u.start, u.far, build)?,
            };
            if let (true, Some(m)) = (build, next) {
                // the flat bottom (or drill point) is the end
                let bottom = Name::feature(i as u64 + 1).with_str("side").with(hole::BOTTOM);
                upto::verify_next(
                    features,
                    i,
                    (&t.0, &t.2),
                    &names,
                    &[bottom.with(0), bottom.with(1)],
                    &m,
                )?;
            }
            Ok(vec![t])
        }
        Feature::Pattern {
            of: Source::Features(src),
            layout,
        } => {
            let pe = |error| RegenError::Pattern { feature: i, error };
            let copies = instances(layout).map_err(pe)?;
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
                    let body = moved(b, m, |n| instance_name(i, *key, n)).map_err(pe)?;
                    let op = if *op == Combine::New { Combine::Add } else { *op };
                    out.push((body, op, moved_shape(sh, m)));
                }
            }
            Ok(out)
        }
        Feature::Extrude { .. } | Feature::Revolve { .. } => {
            let (t, names, next) = match extent::resolve(features, i)? {
                extent::Resolved::Plain(f, next) => {
                    let t = tool(i, &f, build)?;
                    let names = upto::names(&t.0);
                    (t, names, next)
                }
                extent::Resolved::UpTo(f, u) => {
                    let base = tool(i, &f, build)?;
                    let names = upto::names(&base.0);
                    (
                        upto::build(i, base, &u.plan, &u.start, u.far, build)?,
                        names,
                        u.next,
                    )
                }
            };
            if let (true, Some(m)) = (build, next) {
                let end = Name::feature(i as u64 + 1).with_str("end");
                upto::verify_next(features, i, (&t.0, &t.2), &names, &[end], &m)?;
            }
            Ok(vec![t])
        }
        Feature::Plane(_) => Ok(vec![]),
        Feature::Sweep(_) | Feature::Coil(_) | Feature::Loft(_) => {
            Ok(vec![swept::tool_of(features, i, build)?])
        }
        f => Ok(vec![tool(i, f, build)?]),
    }
}

/// The solids feature `s` stands for: its tool, or a pattern's sources' and its copies.
fn solids_of(features: &[Feature], s: usize, build: bool) -> Result<Vec<Tool>, RegenError> {
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
fn tool(i: usize, f: &Feature, build: bool) -> Result<Tool, RegenError> {
    let id = i as u64 + 1;
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
                let ts = tools(&part.features, i, true).expect("tools build");
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
