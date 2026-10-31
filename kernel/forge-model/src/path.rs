//! Where the copies of a pattern along a path, or at a set of points, go.

use crate::pattern::{At, Motion, PatternError};
use crate::work::{self, Ref, at_length, length};
use crate::{History, RegenError};
use forge_geom::{Curve, CurveGeom};
use forge_math::{Frame, Interval, Point3, UnitVec3, Vec3, dmath, tol};

/// One piece of a path: a curve over a range, walked up the range or down it.
struct Piece {
    c: Curve,
    range: Interval,
    forward: bool,
    len: f64,
}

impl Piece {
    fn t(&self, s: f64) -> f64 {
        if self.forward {
            at_length(&self.c, self.range, s)
        } else {
            at_length(&self.c, self.range, self.len - s)
        }
    }
    fn ends(&self) -> (Point3, Point3) {
        let (a, b) = (self.c.eval(self.range.lo), self.c.eval(self.range.hi));
        if self.forward { (a, b) } else { (b, a) }
    }
    fn tangent(&self, t: f64) -> Option<UnitVec3> {
        let d = self.c.tangent(t)?;
        Some(if self.forward { d } else { -d })
    }
    /// The turn of a frame carried along the piece from parameter `a` to `b` without twisting.
    fn turn(&self, a: f64, b: f64) -> Option<Frame> {
        match &self.c {
            Curve::Line(_) => Some(Frame::WORLD),
            Curve::Circle(k) => Some(about(k.frame.z(), b - a)),
            _ => {
                let n = 16;
                let mut r = Frame::WORLD;
                for j in 0..n {
                    let (u, v) = (
                        a + (b - a) * j as f64 / n as f64,
                        a + (b - a) * (j + 1) as f64 / n as f64,
                    );
                    r = least(self.tangent(u)?, self.tangent(v)?).compose(&r);
                }
                Some(r)
            }
        }
    }
}

/// A turn by `angle` about `axis` through the origin.
fn about(axis: UnitVec3, angle: f64) -> Frame {
    let a = Frame::from_z(Point3::ORIGIN, axis);
    a.rotated_about_z(angle).compose(&a.inverse())
}

/// The least turn taking `a` to `b`.
fn least(a: UnitVec3, b: UnitVec3) -> Frame {
    let x = a.cross(b);
    match x.normalize() {
        Some(axis) if x.length() > 1e-15 => about(axis, dmath::atan2(x.length(), a.dot(b.get()))),
        _ if a.dot(b.get()) > 0.0 => Frame::WORLD,
        _ => about(a.any_perpendicular(), dmath::PI),
    }
}

/// The pieces in path order, end to end, and whether the path closes.
fn chain(mut free: Vec<(Curve, Interval)>) -> Option<(Vec<Piece>, bool)> {
    let close = |p: Point3, q: Point3| p.distance(q) <= 10.0 * tol::LINEAR;
    let piece = |(c, range): (Curve, Interval), forward| {
        let len = length(&c, range);
        Piece {
            c,
            range,
            forward,
            len,
        }
    };
    let mut out = vec![piece(free.remove(0), true)];
    // grow the tail, then the head
    for head in [false, true] {
        loop {
            let at = if head {
                out[0].ends().0
            } else {
                out[out.len() - 1].ends().1
            };
            let hit = free.iter().enumerate().find_map(|(k, (c, r))| {
                let (a, b) = (c.eval(r.lo), c.eval(r.hi));
                match (close(a, at), close(b, at)) {
                    (true, _) => Some((k, !head)),
                    (_, true) => Some((k, head)),
                    _ => None,
                }
            });
            let Some((k, forward)) = hit else { break };
            let p = piece(free.remove(k), forward);
            if head {
                out.insert(0, p);
            } else {
                out.push(p);
            }
        }
    }
    let closed = close(out[0].ends().0, out[out.len() - 1].ends().1);
    free.is_empty().then_some((out, closed))
}

/// Copies along a path, keyed by their place on it.
pub(crate) fn along(
    features: History<'_>,
    i: usize,
    refs: &[Ref],
    count: usize,
    spacing: Option<f64>,
    turn: bool,
    reverse: bool,
) -> Result<Vec<([u64; 2], Motion)>, RegenError> {
    let bad = || RegenError::Pattern {
        feature: i,
        error: PatternError::BadLayout,
    };
    let curves = work::curves(features, i, refs)?;
    if count == 0 || curves.is_empty() || spacing.is_some_and(|s| !(s.is_finite() && s > 0.0)) {
        return Err(bad());
    }
    let (mut pieces, closed) = match &curves[..] {
        // an axis: from its origin on, as far as the copies go
        [(c @ Curve::Line(_), None)] => {
            let s = spacing.ok_or_else(bad)?;
            let range = Interval::new(0.0, s * (count - 1) as f64);
            (
                vec![Piece {
                    c: c.clone(),
                    range,
                    forward: true,
                    len: range.hi,
                }],
                false,
            )
        }
        _ => {
            let edges = curves
                .into_iter()
                .map(|(c, r)| r.map(|r| (c, r)))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(bad)?;
            chain(edges).ok_or_else(bad)?
        }
    };
    if reverse {
        pieces.reverse();
        pieces.iter_mut().for_each(|p| p.forward = !p.forward);
    }
    let total: f64 = pieces.iter().map(|p| p.len).sum();
    let step = match spacing {
        Some(s) => s,
        None if closed => total / count as f64,
        None if count > 1 => total / (count - 1) as f64,
        None => 0.0,
    };
    let p0 = pieces[0].ends().0;
    let mut out = Vec::with_capacity(count.saturating_sub(1));
    // the turn carried to the start of piece `k`
    let (mut k, mut gone, mut turned) = (0, 0.0, Frame::WORLD);
    for n in 1..count {
        let mut s = step * n as f64;
        if closed {
            s = s.rem_euclid(total);
            if s < gone {
                (k, gone, turned) = (0, 0.0, Frame::WORLD);
            }
        } else if s > total + tol::LINEAR {
            return Err(bad());
        }
        while k + 1 < pieces.len() && s > gone + pieces[k].len {
            let p = &pieces[k];
            let end = p.t(p.len);
            let r = p.turn(p.t(0.0), end).ok_or_else(bad)?;
            let next = &pieces[k + 1];
            let joint = least(
                p.tangent(end).ok_or_else(bad)?,
                next.tangent(next.t(0.0)).ok_or_else(bad)?,
            );
            turned = joint.compose(&r.compose(&turned));
            gone += p.len;
            k += 1;
        }
        let p = &pieces[k];
        let t = p.t((s - gone).clamp(0.0, p.len));
        let at = p.c.eval(t);
        let m = if turn {
            let r = p.turn(p.t(0.0), t).ok_or_else(bad)?.compose(&turned);
            r.with_origin(at - r.vec_to_world(p0.to_vec()))
        } else {
            Frame::WORLD.with_origin(Point3::ORIGIN + (at - p0))
        };
        out.push(([n as u64, 0], Motion::Rigid(m)));
    }
    Ok(out)
}

/// Copies moved from the first point to each of the others.
pub(crate) fn at_points(
    features: History<'_>,
    i: usize,
    at: &[At],
) -> Result<Vec<([u64; 2], Motion)>, RegenError> {
    let mut pts = Vec::new();
    for a in at {
        match *a {
            At::Sketch { ref at, .. } => pts.extend(at),
            At::Xyz(p) if p.is_finite() => pts.push(p),
            At::Xyz(_) => {
                return Err(RegenError::Pattern {
                    feature: i,
                    error: PatternError::BadLayout,
                });
            }
            At::Ref(r) => pts.extend(work::points(features, i, r)?),
        }
    }
    let Some(&p0) = pts.first() else {
        return Err(RegenError::Pattern {
            feature: i,
            error: PatternError::BadLayout,
        });
    };
    Ok(pts[1..]
        .iter()
        .enumerate()
        .map(|(k, &p)| {
            let d: Vec3 = p - p0;
            (
                [k as u64 + 1, 0],
                Motion::Rigid(Frame::WORLD.with_origin(Point3::ORIGIN + d)),
            )
        })
        .collect())
}
