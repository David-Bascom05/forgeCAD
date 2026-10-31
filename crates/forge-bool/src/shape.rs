//! Tools as analytic solids: whether a point is inside one answered from the profile it was
//! made from (a prism, a solid of revolution, a ball), with no B-rep in the loop.
//!
//! The answer is In or Out only when the point is clear of the boundary by a margin; nearer,
//! it is Near and the caller asks the B-rep.

use crate::Op;
use forge_math::{Frame, Point2, Point3, Vec2, dmath};

/// A piece of a closed 2D profile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg2 {
    Line {
        a: Point2,
        b: Point2,
    },
    /// From angle `start` through `sweep` (negative: clockwise); a whole circle has |sweep| 2pi.
    Arc {
        c: Point2,
        r: f64,
        start: f64,
        sweep: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// The profile (in `frame`'s xy) swept along its z from `lo` to `hi`.
    Prism {
        frame: Frame,
        segs: Vec<Seg2>,
        lo: f64,
        hi: f64,
    },
    /// The profile (in its own plane's coordinates) turned about `frame`'s z, from its x through
    /// `angle` (None: a whole turn). A point at height h and radius r lies over the profile point
    /// `origin + dir * h + perp * r`.
    Revolved {
        frame: Frame,
        origin: Point2,
        dir: Vec2,
        perp: Vec2,
        segs: Vec<Seg2>,
        angle: Option<f64>,
    },
    Ball {
        center: Point3,
        radius: f64,
    },
    /// `first`, then each later solid combined with it in turn (a tool cut off at a surface:
    /// its sweep, kept on the near side of a plane, less the solid beyond the surface). Parts
    /// are plain solids, never combinations.
    Csg {
        first: Box<Shape>,
        rest: Vec<(Op, Shape)>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Member {
    In,
    Out,
    Near,
}

/// Whether a point is inside a closed profile: by the parity of a ray along +x, with the
/// distance to the boundary; None where the ray grazes a vertex or an arc's top or bottom.
fn in_profile(segs: &[Seg2], q: Point2) -> Option<(bool, f64)> {
    let mut inside = false;
    let mut dist = f64::INFINITY;
    let graze = 1e-9;
    for s in segs {
        match *s {
            Seg2::Line { a, b } => {
                dist = dist.min(segment_distance(q, a, b));
                if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                    inside = !inside;
                }
            }
            Seg2::Arc { c, r, start, sweep } => {
                let d = q - c;
                let len = d.length();
                let at = dmath::atan2(d.y, d.x);
                dist = dist.min(if on_sweep(at, start, sweep) {
                    (len - r).abs()
                } else {
                    let p0 = c + Vec2::new(dmath::cos(start), dmath::sin(start)) * r;
                    let p1 = c + Vec2::new(dmath::cos(start + sweep), dmath::sin(start + sweep)) * r;
                    q.distance(p0).min(q.distance(p1))
                });
                let dy = q.y - c.y;
                if dy.abs() >= r - graze {
                    if dy.abs() <= r + graze {
                        return None;
                    }
                    continue;
                }
                if sweep.abs() < dmath::TAU - 1e-12 {
                    for e in [start, start + sweep] {
                        if (c.y + r * dmath::sin(e) - q.y).abs() <= graze {
                            return None;
                        }
                    }
                }
                let dx = (r * r - dy * dy).sqrt();
                for x in [c.x - dx, c.x + dx] {
                    if x > q.x && on_sweep(dmath::atan2(dy, x - c.x), start, sweep) {
                        inside = !inside;
                    }
                }
            }
        }
    }
    Some((inside, dist))
}

/// Whether angle `a` lies on the arc from `start` through `sweep`.
fn on_sweep(a: f64, start: f64, sweep: f64) -> bool {
    if sweep.abs() >= dmath::TAU - 1e-12 {
        return true;
    }
    let (lo, w) = if sweep >= 0.0 {
        (start, sweep)
    } else {
        (start + sweep, -sweep)
    };
    (a - lo).rem_euclid(dmath::TAU) <= w
}

fn segment_distance(q: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let t = ((q - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
    q.distance(a + ab * t)
}

/// In, Out, or Near from a profile test and how far along an extent (`z` against `lo..hi`) the
/// point lies.
fn combine(prof: Option<(bool, f64)>, z: f64, lo: f64, hi: f64, m: f64) -> Member {
    let Some((in2, d2)) = prof else {
        return Member::Near;
    };
    if z < lo - m || z > hi + m || (!in2 && d2 > m) {
        return Member::Out;
    }
    let dz = (z - lo).min(hi - z);
    if in2 && d2 > m && dz > m {
        Member::In
    } else {
        Member::Near
    }
}

impl Shape {
    /// Where `p` is, clear of the boundary by more than `m`, or Near.
    pub fn member(&self, p: Point3, m: f64) -> Member {
        match self {
            Shape::Prism { frame, segs, lo, hi } => {
                let l = frame.point_to_local(p);
                if l.z < lo - m || l.z > hi + m {
                    return Member::Out;
                }
                combine(in_profile(segs, Point2::new(l.x, l.y)), l.z, *lo, *hi, m)
            }
            Shape::Revolved {
                frame,
                origin,
                dir,
                perp,
                segs,
                angle,
            } => {
                let l = frame.point_to_local(p);
                let r = (l.x * l.x + l.y * l.y).sqrt();
                let q = *origin + *dir * l.z + *perp * r;
                let prof = in_profile(segs, q);
                let Some(angle) = angle else {
                    return combine(prof, 0.0, -1.0, 1.0, m.min(0.5));
                };
                // a part turn: the sector between the two profile planes
                let a = dmath::atan2(l.y, l.x).rem_euclid(dmath::TAU);
                let sector = if a <= *angle {
                    // distance to the nearer bounding half plane
                    let to = |b: f64| {
                        let d = (a - b).abs();
                        if d >= dmath::FRAC_PI_2 {
                            r
                        } else {
                            r * dmath::sin(d)
                        }
                    };
                    (true, to(0.0).min(to(*angle)))
                } else {
                    let to = |b: f64| {
                        let d = (a - b).abs().min(dmath::TAU - (a - b).abs());
                        if d >= dmath::FRAC_PI_2 {
                            r
                        } else {
                            r * dmath::sin(d)
                        }
                    };
                    (false, to(0.0).min(to(*angle)))
                };
                match combine(prof, 0.0, -1.0, 1.0, m.min(0.5)) {
                    Member::Out => Member::Out,
                    _ if !sector.0 && sector.1 > m => Member::Out,
                    Member::In if sector.0 && sector.1 > m => Member::In,
                    _ => Member::Near,
                }
            }
            Shape::Csg { first, rest } => {
                // a combination's boundary lies on its parts': clear of all of theirs is clear of it
                let mut v = first.member(p, m);
                for (op, sh) in rest {
                    if v == Member::Out && *op != Op::Union {
                        continue;
                    }
                    let w = sh.member(p, m);
                    v = match op {
                        Op::Union if v == Member::In || w == Member::In => Member::In,
                        Op::Union if v == Member::Out && w == Member::Out => Member::Out,
                        Op::Subtract if v == Member::Out || w == Member::In => Member::Out,
                        Op::Subtract if v == Member::In && w == Member::Out => Member::In,
                        Op::Intersect if v == Member::Out || w == Member::Out => Member::Out,
                        Op::Intersect if v == Member::In && w == Member::In => Member::In,
                        _ => Member::Near,
                    };
                }
                v
            }
            Shape::Ball { center, radius } => {
                let d = p.distance(*center);
                if d < radius - m {
                    Member::In
                } else if d > radius + m {
                    Member::Out
                } else {
                    Member::Near
                }
            }
        }
    }
}
