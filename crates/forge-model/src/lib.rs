//! A part as a history of features, and regenerating it into a solid.

pub mod adversarial;
pub mod fillets;
pub mod finishing;
pub mod suite;
pub mod torture;

use forge_bool::{BoolError, Op, Seg2, Shape, boolean_into};
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_ops::{
    Axis2, ChamferError, ChamferSize, DraftError, ExtrudeError, FilletEdge, FilletError, Profile,
    ProfileLoop, RevolveError, Segment, ShellError, chamfer, draft, extrude, fillet, revolve, shell,
};
use forge_topo::{Body, Name};

/// One step of a part's history.
#[derive(Clone, Debug, PartialEq)]
pub enum Feature {
    /// Extrude a profile into a new solid, or add it to / cut it from the part.
    Extrude {
        profile: Profile,
        distance: f64,
        op: Combine,
    },
    /// Revolve a profile about an axis in its plane.
    Revolve {
        profile: Profile,
        axis: Axis2,
        angle: f64,
        op: Combine,
    },
    /// A ball (a shorthand for revolving a half disc).
    Ball { frame: Frame, radius: f64, op: Combine },
    /// Round the named edges of the part as it stands, each with its own radius, in one go.
    Fillet { edges: Vec<(Name, f64)> },
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
    /// A feature that combines with or changes the part came before any part existed.
    NothingToCombine {
        feature: usize,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    pub features: Vec<Feature>,
}

impl Part {
    /// Rebuilds the solid from the first feature. Feature ids are their
    /// positions, so entity names stay stable when nothing upstream changes.
    /// Rebuilds the solid in one n-ary boolean over every feature's tool (built in parallel),
    /// on up to `threads` threads.
    pub fn regenerate_nary(&self, threads: usize) -> Result<Body, RegenError> {
        self.regenerate_nary_with(threads, true)
    }

    /// `regenerate_nary`, placing points against the tools' analytic solids first or not.
    pub fn regenerate_nary_with(&self, threads: usize, shapes: bool) -> Result<Body, RegenError> {
        self.regenerate_nary_batch(threads, shapes, None)
    }

    /// The tools' analytic solids and ops, in the order the n-ary regen uses them (for building
    /// a batch classifier).
    pub fn nary_solids(&self) -> (Vec<Shape>, Vec<Op>) {
        let start = self
            .features
            .iter()
            .rposition(|f| feature_op(f) == Some(Combine::New))
            .unwrap_or(0);
        let start = match self.features.iter().rposition(|f| feature_op(f).is_none()) {
            Some(m) if m >= start => m + 1,
            _ => start,
        };
        let feats = &self.features[start.min(self.features.len())..];
        let ops = feats[1.min(feats.len())..]
            .iter()
            .map(|f| match feature_op(f) {
                Some(Combine::Add) => Op::Union,
                Some(Combine::Cut) => Op::Subtract,
                _ => Op::Intersect,
            })
            .collect();
        (feats.iter().map(shape).collect(), ops)
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

    pub fn regenerate(&self) -> Result<Body, RegenError> {
        let mut body: Option<Body> = None;
        for (i, f) in self.features.iter().enumerate() {
            let id = i as u64 + 1;
            if feature_op(f).is_none() {
                let b = body.ok_or(RegenError::NothingToCombine { feature: i })?;
                body = Some(modify(i, f, &b)?);
                continue;
            }
            let (tool, op) = tool(i, f)?;
            body = Some(match (body, op) {
                (_, Combine::New) => tool,
                (None, _) => return Err(RegenError::NothingToCombine { feature: i }),
                (Some(mut b), op) => {
                    // in place: only the faces near the tool change
                    boolean_into(&mut b, &tool, op_of(op), id + 1_000_000)
                        .map_err(|error| RegenError::Boolean { feature: i, error })?;
                    // tombstones are cheap until they outnumber the living
                    if b.dead > b.faces.len() / 2 {
                        b.compact()
                    } else {
                        b
                    }
                }
            });
        }
        body.map(|b| b.compact())
            .ok_or(RegenError::NothingToCombine { feature: 0 })
    }
}

/// Builds the tools of `feats` (feature ids from `start`) on up to `threads` threads.
fn build_tools(feats: &[Feature], start: usize, threads: usize) -> Result<Vec<(Body, Combine)>, RegenError> {
    if threads <= 1 {
        return feats
            .iter()
            .enumerate()
            .map(|(k, f)| tool(start + k, f))
            .collect();
    }
    let built: Vec<Result<(Body, Combine), RegenError>> = std::thread::scope(|s| {
        let chunk = feats.len().div_ceil(threads.max(1));
        let hs: Vec<_> = feats
            .chunks(chunk.max(1))
            .enumerate()
            .map(|(c, fs)| {
                s.spawn(move || {
                    fs.iter()
                        .enumerate()
                        .map(|(k, f)| tool(start + c * chunk + k, f))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter()
            .flat_map(|h| h.join().expect("tool builder"))
            .collect()
    });
    built.into_iter().collect()
}

/// The n-ary regen of a history: each run of solid features after a part-changing one (chamfer,
/// shell, draft) is one n-ary boolean starting from the part so far.
fn nary_regen(
    features: &[Feature],
    threads: usize,
    shapes: bool,
    batch: Option<&dyn forge_bool::Batch>,
) -> Result<Body, RegenError> {
    let start = features
        .iter()
        .rposition(|f| feature_op(f) == Some(Combine::New))
        .unwrap_or(0);
    if let Some(m) = features.iter().rposition(|f| feature_op(f).is_none())
        && m >= start
    {
        if m == 0 {
            return Err(RegenError::NothingToCombine { feature: 0 });
        }
        let body = modify(
            m,
            &features[m],
            &nary_regen(&features[..m], threads, shapes, batch)?,
        )?;
        if m + 1 == features.len() {
            return Ok(body);
        }
        let built = build_tools(&features[m + 1..], m + 1, threads)?;
        let mut tools: Vec<&Body> = vec![&body];
        tools.extend(built.iter().map(|t| &t.0));
        let ops: Vec<Op> = built.iter().map(|t| op_of(t.1)).collect();
        return forge_bool::nary_with(&tools, &ops, None, None, threads)
            .map(|b| b.compact())
            .map_err(|error| RegenError::Boolean {
                feature: features.len() - 1,
                error,
            });
    }
    if start == 0
        && features
            .first()
            .is_some_and(|f| feature_op(f) != Some(Combine::New))
    {
        return Err(RegenError::NothingToCombine { feature: 0 });
    }
    let feats = &features[start..];
    let built = build_tools(feats, start, threads)?;
    let tools: Vec<&Body> = built.iter().map(|t| &t.0).collect();
    let ops: Vec<Op> = built[1..].iter().map(|t| op_of(t.1)).collect();
    let solids: Vec<Shape> = feats.iter().map(shape).collect();
    forge_bool::nary_with(&tools, &ops, shapes.then_some(&solids[..]), batch, threads)
        .map(|b| b.compact())
        .map_err(|error| RegenError::Boolean {
            feature: features.len() - 1,
            error,
        })
}

fn op_of(c: Combine) -> Op {
    match c {
        Combine::Add => Op::Union,
        Combine::Cut => Op::Subtract,
        _ => Op::Intersect,
    }
}

/// Applies part-changing feature `i` to `body`.
fn modify(i: usize, f: &Feature, body: &Body) -> Result<Body, RegenError> {
    let id = i as u64 + 1;
    match f {
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
        _ => unreachable!("solid features make tools"),
    }
}

/// Feature `i`, a fillet of the named edges, applied to `body`.
fn apply_fillet(i: usize, body: &Body, edges: &[(Name, f64)]) -> Result<Body, RegenError> {
    let body = if body.dead > 0 {
        body.compact()
    } else {
        body.clone()
    };
    let mut want: Vec<FilletEdge> = Vec::with_capacity(edges.len());
    for &(name, radius) in edges {
        let edge = body
            .edge_by_name(name)
            .ok_or(RegenError::MissingEdge { feature: i, name })?;
        want.push(FilletEdge { edge, radius });
    }
    fillet(&body, &want, i as u64 + 1).map_err(|error| RegenError::Fillet { feature: i, error })
}

/// The solid feature `i` makes, and how it combines with the part.
fn tool(i: usize, f: &Feature) -> Result<(Body, Combine), RegenError> {
    let id = i as u64 + 1;
    Ok(match f {
        Feature::Extrude {
            profile,
            distance,
            op,
        } => (
            extrude(profile, *distance, id).map_err(|error| RegenError::Extrude { feature: i, error })?,
            *op,
        ),
        Feature::Revolve {
            profile,
            axis,
            angle,
            op,
        } => (
            revolve(profile, *axis, *angle, id).map_err(|error| RegenError::Revolve { feature: i, error })?,
            *op,
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
            )
        }
        Feature::Fillet { .. } | Feature::Chamfer { .. } | Feature::Shell { .. } | Feature::Draft { .. } => {
            unreachable!("part-changing features are no tool")
        }
    })
}

/// How a solid feature combines with the part; None for one that changes the part itself.
fn feature_op(f: &Feature) -> Option<Combine> {
    match f {
        Feature::Extrude { op, .. } | Feature::Revolve { op, .. } | Feature::Ball { op, .. } => Some(*op),
        Feature::Fillet { .. } | Feature::Chamfer { .. } | Feature::Shell { .. } | Feature::Draft { .. } => {
            None
        }
    }
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
            profile, distance, ..
        } => Shape::Prism {
            frame: profile.plane,
            segs: segs(profile),
            lo: distance.min(0.0),
            hi: distance.max(0.0),
        },
        Feature::Revolve {
            profile, axis, angle, ..
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
        Feature::Fillet { .. } | Feature::Chamfer { .. } | Feature::Shell { .. } | Feature::Draft { .. } => {
            unreachable!("part-changing features are no tool")
        }
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
            for (i, f) in part.features.iter().enumerate() {
                let (body, _) = tool(i, f).expect("tool builds");
                let index = Index::new(&body);
                let sh = shape(f);
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
