//! Surfaces swept by a screw motion: a planar section turning about an axis while it rises along
//! it and drifts away from it, both in step with the angle. A coil's faces are these, and so is a
//! bend whose section reaches past the bend's axis (no torus fits there).
//!
//! In the axis frame (z the axis, x through the section's half plane at u = 0) the section point
//! (r, h) at angle u lies at ((r + a u) cos u, (r + a u) sin u, h + b u): `b` is the rise and `a`
//! the drift per radian. Points come straight from that formula (one sin_cos for u, one more for
//! an arc).
//!
//! A face's section is carried on around a smooth closed track that follows it closely (see
//! `track`), and a turn past each end of the face's angles: v in [0, 1] is the face's section,
//! v in [j, j + 1] the track's piece j. The implicit value reads the point in the half planes
//! through the axis: the track's signed distance there, scaled by the motion's slant (first order
//! exact). Its zero set is exactly the carried-on surface, and projection searches all of it, so
//! a point of the zero set off the face projects to parameters off the face: no rim or crease for
//! an intersection to run off, and no sheet of the zero set that projection would mistake for
//! the face.

use crate::curve::{CurveDeriv, Domain1, ProcCurve, ProceduralCurve};
use crate::surface::{Domain2, Proc, Procedural, SurfDeriv, Surface};
use forge_math::linalg::solve2;
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, tol};
use std::sync::Arc;

mod coaxial;
mod general;
mod pair;
pub use coaxial::meet_coaxial;
pub use general::meet_general;

/// A planar section in (r, h): distance from the axis, height along it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Section {
    Line {
        a: Point2,
        b: Point2,
    },
    /// Counter-clockwise in (r, h) when `sweep` > 0.
    Arc {
        c: Point2,
        r: f64,
        start: f64,
        sweep: f64,
    },
}

impl Section {
    /// Point and first two derivatives at fraction v.
    pub fn at(&self, v: f64) -> (Point2, Vec2, Vec2) {
        match *self {
            Section::Line { a, b } => (a.lerp(b, v), b - a, Vec2::ZERO),
            Section::Arc { c, r, start, sweep } => {
                let (s, co) = dmath::sin_cos(start + sweep * v);
                let k = r * sweep;
                (
                    Point2::new(c.x + r * co, c.y + r * s),
                    Vec2::new(-s * k, co * k),
                    Vec2::new(-co * k * sweep, -s * k * sweep),
                )
            }
        }
    }

    /// The fraction of the point nearest q within [0, 1].
    fn frac(&self, q: Point2) -> f64 {
        match *self {
            Section::Line { a, b } => {
                let d = b - a;
                ((q - a).dot(d) / d.length_sq()).clamp(0.0, 1.0)
            }
            Section::Arc { c, start, sweep, .. } => {
                let w = q - c;
                if w.length_sq() == 0.0 {
                    return 0.5;
                }
                let t = (dmath::atan2(w.y, w.x) - start) / sweep;
                let per = dmath::TAU / sweep.abs();
                let t = t + per * ((0.5 - t) / per).round();
                if (0.0..=1.0).contains(&t) {
                    return t;
                }
                let (p0, p1) = (self.at(0.0).0, self.at(1.0).0);
                if q.distance(p0) <= q.distance(p1) {
                    0.0
                } else {
                    1.0
                }
            }
        }
    }

    /// Boxes of r and h over fractions v.
    fn bbox(&self, v: Interval) -> (Interval, Interval) {
        match *self {
            Section::Line { a, b } => (
                Interval::point(a.x) + v * (b.x - a.x),
                Interval::point(a.y) + v * (b.y - a.y),
            ),
            Section::Arc { c, r, start, sweep } => {
                let t = Interval::point(start) + v * sweep;
                (t.cos() * r + c.x, t.sin() * r + c.y)
            }
        }
    }

    /// The exact ranges of r and of h over the section.
    pub fn extent(&self) -> ((f64, f64), (f64, f64)) {
        let (p0, p1) = (self.at(0.0).0, self.at(1.0).0);
        let (mut r, mut h) = ((p0.x.min(p1.x), p0.x.max(p1.x)), (p0.y.min(p1.y), p0.y.max(p1.y)));
        if let Section::Arc {
            c,
            r: rad,
            start,
            sweep,
        } = *self
        {
            let (a, b) = (start.min(start + sweep), start.max(start + sweep));
            // the arc's extremes where it passes the axes' directions
            for k in 0..4 {
                let angle = k as f64 * dmath::FRAC_PI_2;
                if angle + ((a - angle) / dmath::TAU).ceil() * dmath::TAU <= b {
                    let (sn, cs) = dmath::sin_cos(angle);
                    let q = Point2::new(c.x + rad * cs, c.y + rad * sn);
                    r = (r.0.min(q.x), r.1.max(q.x));
                    h = (h.0.min(q.y), h.1.max(q.y));
                }
            }
        }
        (r, h)
    }

    pub fn length(&self) -> f64 {
        match *self {
            Section::Line { a, b } => a.distance(b),
            Section::Arc { r, sweep, .. } => r * sweep.abs(),
        }
    }
}

/// A section with what the distance and side tests reuse.
#[derive(Clone, Copy, Debug)]
struct Piece {
    s: Section,
    /// An arc's end directions from its centre, and its end points.
    d0: Vec2,
    d1: Vec2,
    p0: Point2,
    p1: Point2,
    rr: Interval,
    hh: Interval,
}

impl Piece {
    fn new(s: Section) -> Piece {
        let (rr, hh) = s.bbox(Interval::new(0.0, 1.0));
        let (p0, p1) = (s.at(0.0).0, s.at(1.0).0);
        let (d0, d1) = match s {
            Section::Line { .. } => (Vec2::ZERO, Vec2::ZERO),
            Section::Arc { c, r, .. } => ((p0 - c) / r, (p1 - c) / r),
        };
        Piece {
            s,
            d0,
            d1,
            p0,
            p1,
            rr,
            hh,
        }
    }

    /// Whether direction w from an arc's centre falls on the arc.
    fn spans(&self, w: Vec2, sweep: f64) -> bool {
        let s = if sweep > 0.0 { 1.0 } else { -1.0 };
        if sweep.abs() <= dmath::PI {
            self.d0.cross(w) * s >= 0.0 && w.cross(self.d1) * s >= 0.0
        } else {
            !(self.d1.cross(w) * s > 0.0 && w.cross(self.d0) * s > 0.0)
        }
    }

    /// Squared distance from q to the piece, the nearest point, and the unit normal to the
    /// right of travel there when that point is inside the piece (not an end).
    fn dist(&self, q: Point2) -> (f64, Point2, Option<Vec2>) {
        let end = |q: Point2| {
            let (e0, e1) = ((q - self.p0).length_sq(), (q - self.p1).length_sq());
            if e0 <= e1 {
                (e0, self.p0, None)
            } else {
                (e1, self.p1, None)
            }
        };
        match self.s {
            Section::Line { a, b } => {
                let d = b - a;
                let t = (q - a).dot(d) / d.length_sq();
                if !(t > 0.0 && t < 1.0) {
                    return end(q);
                }
                let p = a.lerp(b, t);
                ((q - p).length_sq(), p, Some(Vec2::new(d.y, -d.x) / d.length()))
            }
            Section::Arc { c, r, sweep, .. } => {
                let w = q - c;
                let l = w.length();
                if l > 0.0 && self.spans(w, sweep) {
                    let u = w / l;
                    return (
                        (l - r) * (l - r),
                        c + u * r,
                        Some(if sweep > 0.0 { u } else { -u }),
                    );
                }
                end(q)
            }
        }
    }

    /// Crossings of the ray from q towards +r.
    fn crossings(&self, q: Point2) -> usize {
        match self.s {
            Section::Line { a, b } => {
                usize::from((a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x))
            }
            Section::Arc { c, r, sweep, .. } => {
                let dy = q.y - c.y;
                if dy.abs() >= r {
                    return 0;
                }
                let dx = (r * r - dy * dy).sqrt();
                [-dx, dx]
                    .into_iter()
                    .filter(|&x| c.x + x > q.x && self.spans(Vec2::new(x, dy), sweep))
                    .count()
            }
        }
    }
}

/// A face's track: its section carried on `w` past each end, turned back to its left by a
/// semicircle of radius `w` at each end, and joined by the section offset `2 w` to its left. A
/// closed curve, smooth (C1) all round, that follows the section closely.
fn track(s: Section, w: f64) -> Vec<Section> {
    let unit = |v: Vec2| v.normalize().unwrap_or(Vec2::X);
    let (p0, t0) = (s.at(0.0).0, unit(s.at(0.0).1));
    let (p1, t1) = (s.at(1.0).0, unit(s.at(1.0).1));
    let left = |t: Vec2| Vec2::new(-t.y, t.x);
    let (n0, n1) = (left(t0), left(t1));
    let (q0, q1) = (p0 - t0 * w, p1 + t1 * w);
    let (p0b, p1b) = (p0 + n0 * (2.0 * w), p1 + n1 * (2.0 * w));
    let (q0b, q1b) = (p0b - t0 * w, q1 + n1 * (2.0 * w));
    let angle = |v: Vec2| dmath::atan2(v.y, v.x);
    let back = match s {
        Section::Line { .. } => Section::Line { a: p1b, b: p0b },
        Section::Arc { c, r, start, sweep } => Section::Arc {
            c,
            r: if sweep > 0.0 { r - 2.0 * w } else { r + 2.0 * w },
            start: start + sweep,
            sweep: -sweep,
        },
    };
    vec![
        s,
        Section::Line { a: p1, b: q1 },
        Section::Arc {
            c: q1 + n1 * w,
            r: w,
            start: angle(-n1),
            sweep: dmath::PI,
        },
        Section::Line { a: q1b, b: p1b },
        back,
        Section::Line { a: p0b, b: q0b },
        Section::Arc {
            c: q0 + n0 * w,
            r: w,
            start: angle(n0),
            sweep: dmath::PI,
        },
        Section::Line { a: q0, b: p0 },
    ]
}

/// The screw motion's frame and rates, shared by the surface and its helices.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Motion {
    frame: Frame,
    /// Drift from the axis and rise along it, per radian.
    a: f64,
    b: f64,
}

impl Motion {
    /// e(u), e'(u) and the axis, in world coordinates.
    fn axes(&self, u: f64) -> (Vec3, Vec3, Vec3) {
        let (s, c) = dmath::sin_cos(u);
        let (x, y) = (self.frame.x().get(), self.frame.y().get());
        (x * c + y * s, y * c - x * s, self.frame.z().get())
    }

    /// Box of the points (r cos u, r sin u, h) for u, r, h in the given intervals.
    fn bbox(&self, u: Interval, r: Interval, h: Interval) -> Box3 {
        let (cx, cy) = (u.cos() * r, u.sin() * r);
        let f = &self.frame;
        let (x, y, z, o) = (f.x().get(), f.y().get(), f.z().get(), f.origin);
        Box3 {
            x: cx * x.x + cy * y.x + h * z.x + o.x,
            y: cx * x.y + cy * y.y + h * z.y + o.y,
            z: cx * x.z + cy * y.z + h * z.z + o.z,
        }
    }

    /// Distance from the axis, angle and height of p.
    fn cylindrical(&self, p: Point3) -> (f64, f64, f64) {
        let l = self.frame.point_to_local(p);
        let rho = (l.x * l.x + l.y * l.y).sqrt();
        let phi = if rho > 0.0 { dmath::atan2(l.y, l.x) } else { 0.0 };
        (rho, phi, l.z)
    }
}

/// A face's track, closed and counter-clockwise in (r, h).
#[derive(Debug)]
struct Ring {
    pieces: Vec<Piece>,
    rr: Interval,
    hh: Interval,
}

/// The track point nearest a point, found in one turn's half plane.
struct Near {
    d: f64,
    /// The turn's angle and the point in (r, h) there.
    u: f64,
    q: Point2,
    /// The track piece, its nearest point, and the normal there unless that is a piece's end.
    j: usize,
    np: Point2,
    nr: Option<Vec2>,
}

/// One section of a profile loop swept by a screw motion over an angle range.
#[derive(Clone, Debug)]
pub struct Screw {
    m: Motion,
    ring: Arc<Ring>,
    u: Interval,
    /// The angles the surface is carried over (a turn past each end; one turn about the face
    /// for a plain revolution).
    ue: Interval,
    /// The direction across the profile loop at the face's start (in (r, h)).
    start_across: Vec2,
    /// No point of the carried-on surface is nearer the axis than this.
    floor: f64,
    rmin: f64,
    rmax: f64,
}

impl Screw {
    /// One face per piece of the closed loop `sections`, swept through angles `u` about `frame`'s
    /// z, drifting `a` and rising `b` per radian. None when a section reaches the axis on the face.
    pub fn loop_faces(frame: Frame, sections: &[Section], a: f64, b: f64, u: Interval) -> Option<Vec<Screw>> {
        let ue = if a == 0.0 && b == 0.0 {
            Interval::new(u.mid() - dmath::PI, u.mid() + dmath::PI)
        } else {
            Interval::new(u.lo - dmath::TAU, u.hi + dmath::TAU)
        };
        let drift = |i: Interval| (a * i.lo).min(a * i.hi);
        // how far the loop clears its next turn, along the axis
        let height = sections
            .iter()
            .fold(Interval::EMPTY, |h, s| h.hull(&s.bbox(Interval::new(0.0, 1.0)).1));
        let gap = if b == 0.0 {
            f64::INFINITY
        } else {
            dmath::TAU * b.abs() - height.width()
        };
        let right = |d: Vec2| Vec2::new(d.y, -d.x).normalize().unwrap_or(Vec2::X);
        let n = sections.len();
        sections
            .iter()
            .enumerate()
            .map(|(k, &sec)| {
                // across the loop where the face starts: between its normal and the one before's
                let before = right(sections[(k + n - 1) % n].at(1.0).1);
                let own = right(sec.at(0.0).1);
                let start_across = (before + own).normalize().unwrap_or(own);
                let (sr, _) = sec.bbox(Interval::new(0.0, 1.0));
                let rmin = sr.lo + drift(u);
                let rmax = sr.hi + (a * u.lo).max(a * u.hi);
                if rmin <= tol::LINEAR {
                    return None;
                }
                // the track stays this close to the face's section
                let inward = match sec {
                    Section::Arc { r, sweep, .. } if sweep > 0.0 => r,
                    _ => f64::INFINITY,
                };
                let w = (0.25 * sec.length())
                    .min((sr.lo + drift(ue)) / 8.0)
                    .min(0.25 * gap)
                    .min(0.25 * inward);
                let pieces: Vec<Piece> = track(sec, w).into_iter().map(Piece::new).collect();
                let (mut rr, mut hh) = (Interval::EMPTY, Interval::EMPTY);
                for p in &pieces {
                    rr = rr.hull(&p.rr);
                    hh = hh.hull(&p.hh);
                }
                Some(Screw {
                    m: Motion { frame, a, b },
                    ring: Arc::new(Ring { pieces, rr, hh }),
                    u,
                    ue,
                    start_across,
                    floor: (rr.lo + drift(ue)).max(0.0),
                    rmin,
                    rmax,
                })
            })
            .collect()
    }

    pub fn surface(self) -> Surface {
        Surface::Procedural(Proc(Arc::new(self)))
    }

    /// The piece and fraction at track parameter v (the face's own piece at its upper end).
    fn piece_at(&self, v: f64) -> (usize, f64) {
        let n = self.ring.pieces.len();
        let j = v.floor();
        if j == 1.0 && v == j {
            return (0, 1.0);
        }
        ((j as i64).rem_euclid(n as i64) as usize, v - j)
    }

    fn point(&self, uv: Point2) -> SurfDeriv {
        let (e, ep, z) = self.m.axes(uv.x);
        let (j, t) = self.piece_at(uv.y);
        let (p, d1, d2) = self.ring.pieces[j].s.at(t);
        let r = p.x + self.m.a * uv.x;
        let h = p.y + self.m.b * uv.x;
        SurfDeriv {
            p: self.m.frame.origin + e * r + z * h,
            du: e * self.m.a + ep * r + z * self.m.b,
            dv: e * d1.x + z * d1.y,
            duu: ep * (2.0 * self.m.a) - e * r,
            duv: ep * d1.x,
            dvv: e * d2.x + z * d2.y,
        }
    }

    /// The nearest point of the track over the turns whose half plane through p can hold it, and
    /// p's distance from the axis. The gap from p to a turn's box only grows on from the nearest
    /// turn (it is convex in the turn), so the search stops either way once it climbs past the
    /// best distance.
    fn nearest_turn(&self, p: Point3) -> (f64, Option<Near>) {
        let (rho, phi, z) = self.m.cylindrical(p);
        let (a, b) = (self.m.a, self.m.b);
        let ring = &self.ring;
        let at = |m: i64| {
            let u = phi + dmath::TAU * m as f64;
            let q = Point2::new(rho - a * u, z - b * u);
            let gap = (ring.hh.lo - q.y)
                .max(q.y - ring.hh.hi)
                .max(ring.rr.lo - q.x)
                .max(q.x - ring.rr.hi)
                .max(0.0);
            (u, q, gap)
        };
        let lo = ((self.ue.lo - phi) / dmath::TAU).ceil() as i64;
        let hi = ((self.ue.hi - phi) / dmath::TAU).floor() as i64;
        let start = if b != 0.0 {
            ((z - b * phi - ring.hh.mid()) / (dmath::TAU * b)).round() as i64
        } else {
            lo
        };
        let start = start.clamp(lo, hi.max(lo));
        let mut best: Option<Near> = None;
        for (mut m, step) in [(start, 1), (start - 1, -1)] {
            let mut last = f64::INFINITY;
            while m >= lo && m <= hi {
                let (u, q, gap) = at(m);
                let bound = best.as_ref().map_or(f64::INFINITY, |b| b.d);
                if gap >= bound && gap >= last {
                    break;
                }
                if gap < bound
                    && let Some((j, d2, np, nr)) = self.nearest_piece(q, bound)
                {
                    best = Some(Near {
                        d: d2.sqrt(),
                        u,
                        q,
                        j,
                        np,
                        nr,
                    });
                }
                last = gap;
                m += step;
            }
        }
        (rho, best)
    }

    /// The track piece nearest q: its index, squared distance, nearest point and normal there.
    fn nearest_piece(&self, q: Point2, bound: f64) -> Option<(usize, f64, Point2, Option<Vec2>)> {
        let mut best: Option<(usize, f64, Point2, Option<Vec2>)> = None;
        let mut b2 = bound * bound;
        for (j, pc) in self.ring.pieces.iter().enumerate() {
            let gap = (pc.hh.lo - q.y)
                .max(q.y - pc.hh.hi)
                .max(pc.rr.lo - q.x)
                .max(q.x - pc.rr.hi);
            if gap > 0.0 && gap * gap >= b2 {
                continue;
            }
            let (d2, np, nr) = pc.dist(q);
            if d2 < b2 {
                b2 = d2;
                best = Some((j, d2, np, nr));
            }
        }
        best
    }

    /// Parameters of the carried-on surface's point nearest p.
    fn nearest(&self, p: Point3) -> Point2 {
        let Some(Near { u, q, j, .. }) = self.nearest_turn(p).1 else {
            return Point2::new(self.u.mid(), 0.5);
        };
        let solve = |j: usize, t: f64| {
            let v0 = j as f64;
            let (uv, d) = polish(
                &|uv| self.point(uv),
                p,
                Point2::new(u, v0 + t),
                Box2::new(self.ue, Interval::new(v0, v0 + 1.0)),
                None,
            );
            (d.p.distance_sq(p), uv)
        };
        let mut out = solve(j, self.ring.pieces[j].s.frac(q));
        // stopped at a joint: the next piece may carry on downhill
        let nj = self.ring.pieces.len();
        for (side, nb, t) in [(j, (j + nj - 1) % nj, 1.0), (j + 1, (j + 1) % nj, 0.0)] {
            if out.1.y == side as f64 {
                let other = solve(nb, t);
                if other.0 < out.0 {
                    out = other;
                }
            }
        }
        // the track's turn that puts v nearest the face's piece, [0, 1]
        let n = nj as f64;
        let off = |v: f64| (-v).max(v - 1.0).max(0.0);
        let v = [out.1.y, out.1.y - n, out.1.y + n]
            .into_iter()
            .fold(out.1.y, |b, v| if off(v) < off(b) { v } else { b });
        Point2::new(out.1.x, v)
    }

    /// The implicit value at p and its gradient.
    fn value(&self, p: Point3) -> (f64, Vec3) {
        let (rho, near) = self.nearest_turn(p);
        let Some(Near { d, q, np, nr, .. }) = near else {
            return (self.floor.max(tol::LINEAR), Vec3::Z);
        };
        // the side from the piece's normal, or at a corner by parity
        let (s, g) = match nr {
            Some(nr) => (if (q - np).dot(nr) >= 0.0 { 1.0 } else { -1.0 }, nr),
            None => {
                // the track runs counter-clockwise: the natural normal's side is outside it
                let inside = self.ring.pieces.iter().map(|pc| pc.crossings(q)).sum::<usize>() % 2 == 1;
                let s = if inside { -1.0 } else { 1.0 };
                (s, if d > 0.0 { (q - np) * (s / d) } else { Vec2::X })
            }
        };
        let (a, b) = (self.m.a, self.m.b);
        let k = a * g.x + b * g.y;
        let n = if rho > 0.0 {
            (1.0 + k * k / (rho * rho)).sqrt()
        } else if k == 0.0 {
            1.0
        } else {
            f64::INFINITY
        };
        // near the axis the surface is no nearer than its floor
        let f = (d / n).max(d.min(self.floor - rho));
        let l = self.m.frame.point_to_local(p);
        let (er, ephi) = if rho > 0.0 {
            (
                Vec3::new(l.x / rho, l.y / rho, 0.0),
                Vec3::new(-l.y / rho, l.x / rho, 0.0),
            )
        } else {
            (Vec3::X, Vec3::Y)
        };
        let kr = if rho > 0.0 { k / rho } else { 0.0 };
        let grad = (er * g.x + Vec3::Z * g.y - ephi * kr) / n;
        (s * f, self.m.frame.vec_to_world(grad))
    }

    /// A helix: the face's section point at fraction t carried through the face's angles.
    /// Its fence runs across the profile there (at the face's start, between the two faces'
    /// normals).
    pub fn helix(&self, t: f64) -> crate::Curve {
        let (p, d1, _) = self.ring.pieces[0].s.at(t);
        let across = if t == 0.0 {
            self.start_across
        } else {
            Vec2::new(d1.y, -d1.x).normalize().unwrap_or(Vec2::X)
        };
        let size = (self.ring.rr.width() + self.ring.hh.width()).max(tol::LINEAR);
        crate::Curve::Procedural(ProcCurve(Arc::new(Helix {
            m: self.m,
            p,
            u: self.u,
            fence: Some((across, size, self.ue)),
        })))
    }
}

impl Procedural for Screw {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        self.point(uv)
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: Domain1 {
                lo: self.u.lo,
                hi: self.u.hi,
                period: None,
            },
            v: Domain1 {
                lo: 0.0,
                hi: 1.0,
                period: None,
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        self.nearest(p)
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        // piece by piece over the track parameters the box spans
        let mut b = Box3::EMPTY;
        let mut v = uv.v.lo;
        let n = self.ring.pieces.len() as i64;
        while v <= uv.v.hi {
            let (j, t0) = ((v.floor() as i64).rem_euclid(n) as usize, v - v.floor());
            let end = (v.floor() + 1.0).min(uv.v.hi);
            let t1 = t0 + (end - v);
            let (r, h) = self.ring.pieces[j].s.bbox(Interval::new(t0, t1.min(1.0)));
            b = b.hull(&self.m.bbox(uv.u, r + uv.u * self.m.a, h + uv.u * self.m.b));
            if end >= uv.v.hi {
                break;
            }
            v = end;
        }
        b.grow(tol::LINEAR)
    }
    fn implicit(&self, p: Point3) -> f64 {
        self.value(p).0
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.value(p).1
    }
    fn transformed(&self, f: &Frame) -> Proc {
        let mut s = self.clone();
        s.m.frame = f.compose(&self.m.frame);
        Proc(Arc::new(s))
    }
    fn uv_scale(&self) -> (f64, f64) {
        let (a, b) = (self.m.a, self.m.b);
        (
            (self.rmax * self.rmax + a * a + b * b).sqrt(),
            self.ring.pieces[0].s.length().max(tol::LINEAR),
        )
    }
    fn second_bounds(&self, uv: Box2) -> Option<[f64; 3]> {
        // S = O + e(u) r + z h, r = x(v) + a u, h = y(v) + b u for section point (x, y)(v):
        // S_uu = 2 a e' - r e, S_uv = x'(v) e', S_vv = x''(v) e + y''(v) z
        let (mut r, mut d1, mut d2) = (0.0f64, 0.0f64, 0.0f64);
        let n = self.ring.pieces.len() as i64;
        let mut v = uv.v.lo;
        loop {
            let (j, t0) = ((v.floor() as i64).rem_euclid(n) as usize, v - v.floor());
            let end = (v.floor() + 1.0).min(uv.v.hi);
            let s = self.ring.pieces[j].s;
            let rr = s.bbox(Interval::new(t0, (t0 + (end - v)).min(1.0))).0 + uv.u * self.m.a;
            r = r.max(rr.lo.abs()).max(rr.hi.abs());
            let (l1, l2) = match s {
                Section::Line { a, b } => (a.distance(b), 0.0),
                Section::Arc { r, sweep, .. } => (r * sweep.abs(), r * sweep * sweep),
            };
            (d1, d2) = (d1.max(l1), d2.max(l2));
            if end >= uv.v.hi {
                break;
            }
            v = end;
        }
        let a = self.m.a;
        Some([(4.0 * a * a + r * r).sqrt(), d1, d2])
    }
    fn min_radius(&self) -> f64 {
        let r = match self.ring.pieces[0].s {
            Section::Line { .. } => f64::INFINITY,
            Section::Arc { r, .. } => r,
        };
        0.8 * self.rmin.min(r)
    }
    fn feature(&self) -> f64 {
        // the track's end turns are its tightest, and no two turns' tracks come closer
        match self.ring.pieces[2].s {
            Section::Arc { r, .. } => r,
            Section::Line { .. } => f64::INFINITY,
        }
    }
}

/// A section point carried by the screw motion: a (conical) helix.
#[derive(Clone, Debug)]
struct Helix {
    m: Motion,
    /// (r, h) at u = 0.
    p: Point2,
    u: Interval,
    /// The line through p across its faces (direction along it, half length) and the angles
    /// to carry it over: the helix's fence.
    fence: Option<(Vec2, f64, Interval)>,
}

impl Helix {
    fn rmax(&self, range: Interval) -> f64 {
        (self.p.x + self.m.a * range.lo)
            .abs()
            .max((self.p.x + self.m.a * range.hi).abs())
    }
}

impl ProceduralCurve for Helix {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let (e, ep, z) = self.m.axes(t);
        let r = self.p.x + self.m.a * t;
        CurveDeriv {
            p: self.m.frame.origin + e * r + z * (self.p.y + self.m.b * t),
            d1: e * self.m.a + ep * r + z * self.m.b,
            d2: ep * (2.0 * self.m.a) - e * r,
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: self.u.lo,
            hi: self.u.hi,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let (rho, phi, z) = self.m.cylindrical(p);
        let (a, b) = (self.m.a, self.m.b);
        let mut best = (f64::INFINITY, range.lo);
        for t in [range.lo, range.hi] {
            let d = self.deriv(t).p.distance_sq(p);
            if d < best.0 {
                best = (d, t);
            }
        }
        // in p's half plane the distance needs no trigonometry
        let k0 = ((range.lo - phi) / dmath::TAU).ceil() as i64;
        let k1 = ((range.hi - phi) / dmath::TAU).floor() as i64;
        for k in k0..=k1 {
            let t = phi + dmath::TAU * k as f64;
            let (dr, dh) = (rho - self.p.x - a * t, z - self.p.y - b * t);
            let d = dr * dr + dh * dh;
            if d < best.0 {
                best = (d, t);
            }
        }
        // Newton on (H - p).H' = 0, kept inside the range while it gets closer
        let (mut t, mut f) = (best.1, best.0);
        for _ in 0..12 {
            let d = self.deriv(t);
            let e = d.p - p;
            let (g, h) = (e.dot(d.d1), d.d1.length_sq() + e.dot(d.d2));
            let h = if h > 0.0 { h } else { d.d1.length_sq() };
            let next = (t - g / h).clamp(range.lo, range.hi);
            if next == t {
                break;
            }
            let fnext = self.deriv(next).p.distance_sq(p);
            if fnext > f {
                break;
            }
            (t, f) = (next, fnext);
        }
        t
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let r = range * self.m.a + self.p.x;
        let h = range * self.m.b + self.p.y;
        self.m.bbox(range, r, h).grow(tol::LINEAR)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        let mut c = self.clone();
        c.m.frame = f.compose(&self.m.frame);
        ProcCurve(Arc::new(c))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        let r = self.rmax(range);
        (self.m.a * self.m.a + r * r + self.m.b * self.m.b).sqrt() * (1.0 + 1e-12)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        // equal steps h keep each chord within |H''| h^2 / 8 of the curve
        let acc = 2.0 * self.m.a.abs() + self.rmax(range);
        let n = range.width() * (acc / (8.0 * chord.max(tol::LINEAR))).sqrt();
        (n.ceil() as usize).max(1)
    }
    fn across(&self) -> Option<Surface> {
        let (d, half, ue) = self.fence?;
        Some(Surface::Procedural(Proc(Arc::new(Fence {
            m: self.m,
            p: self.p,
            d,
            half,
            ue,
        }))))
    }
}

/// The screw sweep of a line through a helix's point, across the faces the helix bounds: where
/// another curve crosses it, that curve passes from one face to the other.
#[derive(Clone, Debug)]
struct Fence {
    m: Motion,
    p: Point2,
    /// Unit direction of the line, and how far it runs either way of p.
    d: Vec2,
    half: f64,
    ue: Interval,
}

impl Fence {
    fn point(&self, uv: Point2) -> SurfDeriv {
        let (e, ep, z) = self.m.axes(uv.x);
        let q = self.p + self.d * uv.y;
        let r = q.x + self.m.a * uv.x;
        SurfDeriv {
            p: self.m.frame.origin + e * r + z * (q.y + self.m.b * uv.x),
            du: e * self.m.a + ep * r + z * self.m.b,
            dv: e * self.d.x + z * self.d.y,
            duu: ep * (2.0 * self.m.a) - e * r,
            duv: ep * self.d.x,
            dvv: Vec3::ZERO,
        }
    }

    /// Signed distance to the line in the nearest turn's half plane, slant-scaled, with its
    /// gradient, and that turn's angle and place along the line.
    fn value(&self, p: Point3) -> (f64, Vec3, Point2) {
        let (rho, phi, z) = self.m.cylindrical(p);
        let (a, b) = (self.m.a, self.m.b);
        let nl = Vec2::new(self.d.y, -self.d.x);
        let lo = ((self.ue.lo - phi) / dmath::TAU).ceil() as i64;
        let hi = ((self.ue.hi - phi) / dmath::TAU).floor() as i64;
        let mut best = (f64::INFINITY, Point2::new(phi, 0.0));
        for m in lo..=hi {
            let u = phi + dmath::TAU * m as f64;
            let q = Point2::new(rho - a * u, z - b * u);
            let s = (q - self.p).dot(nl);
            if s.abs() < best.0.abs() {
                best = (s, Point2::new(u, (q - self.p).dot(self.d)));
            }
        }
        let k = a * nl.x + b * nl.y;
        let n = if rho > 0.0 {
            (1.0 + k * k / (rho * rho)).sqrt()
        } else {
            f64::INFINITY
        };
        let l = self.m.frame.point_to_local(p);
        let (er, ephi) = if rho > 0.0 {
            (
                Vec3::new(l.x / rho, l.y / rho, 0.0),
                Vec3::new(-l.y / rho, l.x / rho, 0.0),
            )
        } else {
            (Vec3::X, Vec3::Y)
        };
        let kr = if rho > 0.0 { k / rho } else { 0.0 };
        let g = (er * nl.x + Vec3::Z * nl.y - ephi * kr) / n;
        (best.0 / n, self.m.frame.vec_to_world(g), best.1)
    }
}

impl Procedural for Fence {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        self.point(uv)
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: Domain1 {
                lo: self.ue.lo,
                hi: self.ue.hi,
                period: None,
            },
            v: Domain1 {
                lo: -self.half,
                hi: self.half,
                period: None,
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let seed = self.value(p).2;
        let dom = Box2::new(self.ue, Interval::new(-self.half, self.half));
        polish(&|uv| self.point(uv), p, seed, dom, None).0
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        let r = uv.v * self.d.x + self.p.x + uv.u * self.m.a;
        let h = uv.v * self.d.y + self.p.y + uv.u * self.m.b;
        self.m.bbox(uv.u, r, h).grow(tol::LINEAR)
    }
    fn implicit(&self, p: Point3) -> f64 {
        self.value(p).0
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.value(p).1
    }
    fn transformed(&self, f: &Frame) -> Proc {
        let mut s = self.clone();
        s.m.frame = f.compose(&self.m.frame);
        Proc(Arc::new(s))
    }
    fn min_radius(&self) -> f64 {
        0.8 * (self.p.x - self.half).max(tol::LINEAR)
    }
}

/// A fresh id for a surface the last query is remembered against.
pub(crate) fn fresh_id() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// The implicit value and gradient of surface `id` at p, solved by `solve` unless this thread
/// just asked for the same (tracing asks for both, one after the other).
pub(crate) fn remembered(id: u64, p: Point3, solve: impl FnOnce() -> (f64, Vec3)) -> (f64, Vec3) {
    thread_local! {
        static LAST: std::cell::Cell<(u64, Point3, f64, Vec3)> = const {
            std::cell::Cell::new((0, Point3::ORIGIN, 0.0, Vec3::ZERO))
        };
    }
    LAST.with(|last| {
        let (k, q, f, g) = last.get();
        if k == id && q == p {
            return (f, g);
        }
        let (f, g) = solve();
        last.set((id, p, f, g));
        (f, g)
    })
}

/// Parameters of the point of a patch nearest p, and the point, by Newton steps on |S - p|^2
/// from `uv` (whose point `at` may already be known), kept inside `dom` (at a side, only along
/// it) and taken only while they get closer.
pub(crate) fn polish(
    s: &impl Fn(Point2) -> SurfDeriv,
    p: Point3,
    uv: Point2,
    dom: Box2,
    at: Option<SurfDeriv>,
) -> (Point2, SurfDeriv) {
    let clamp = |q: Point2| Point2::new(q.x.clamp(dom.u.lo, dom.u.hi), q.y.clamp(dom.v.lo, dom.v.hi));
    let start = uv;
    let mut uv = clamp(uv);
    let mut d = match at {
        Some(d) if uv == start => d,
        _ => s(uv),
    };
    let mut f = (d.p - p).length_sq();
    for _ in 0..32 {
        let e = d.p - p;
        let (gu, gv) = (d.du.dot(e), d.dv.dot(e));
        let (a, b, c) = (d.du.length_sq(), d.du.dot(d.dv), d.dv.length_sq());
        let (huu, huv, hvv) = (a + d.duu.dot(e), b + d.duv.dot(e), c + d.dvv.dot(e));
        // Newton where the Hessian is positive definite, then Gauss-Newton where Newton's step
        // overshoots (the Hessian nearly vanishing)
        let newton = huu > 0.0 && huu * hvv > huv * huv;
        let mut taken = false;
        let mut tiny = false;
        for (k, (h00, h01, h11)) in [(huu, huv, hvv), (a, b, c)].into_iter().enumerate() {
            if k == 0 && !newton {
                continue;
            }
            let Some((mut x, mut y)) = solve2(h00, h01, h01, h11, -gu, -gv) else {
                continue;
            };
            // held at a side, move along it only
            if (uv.x <= dom.u.lo && x < 0.0) || (uv.x >= dom.u.hi && x > 0.0) {
                (x, y) = (0.0, -gv / h11);
            } else if (uv.y <= dom.v.lo && y < 0.0) || (uv.y >= dom.v.hi && y > 0.0) {
                (x, y) = (-gu / h00, 0.0);
            }
            if !(x.is_finite() && y.is_finite()) {
                continue;
            }
            let moves = (a * x * x + 2.0 * b * x * y + c * y * y).max(0.0).sqrt();
            let overshoots = k == 0 && moves > 2.0 * f.sqrt();
            // done when the step is lost in rounding, or (off the surface) when moving the foot
            // that little changes the distance by a part in 1e12 (the foot's error enters the
            // distance squared, scaled by the curvature); on the surface the first test decides
            tiny = moves <= 1e-13 * (1.0 + d.p.to_vec().abs_max()) || moves * moves <= 1e-12 * f;
            for _ in 0..8 {
                let next = clamp(Point2::new(uv.x + x, uv.y + y));
                if next == uv {
                    break;
                }
                let dn = s(next);
                let fnext = (dn.p - p).length_sq();
                if fnext < f || (tiny && fnext <= f) {
                    (uv, d, f, taken) = (next, dn, fnext, true);
                    break;
                }
                (x, y) = (0.5 * x, 0.5 * y);
            }
            if taken || tiny || !overshoots {
                break;
            }
        }
        if !taken || tiny {
            break;
        }
    }
    (uv, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SurfaceGeom;
    use crate::testing::rand_frame;
    use forge_math::rng::Rng;

    /// A random closed loop (a rectangle with one side bowed, or a circle in two halves) and
    /// one face of its sweep.
    fn random_screw(r: &mut Rng) -> Screw {
        let f = rand_frame(r);
        let rc = r.range(8.0, 20.0);
        let (w, h) = (r.range(1.0, 4.0), r.range(0.5, 2.5));
        let ccw = r.bool();
        let mut loop_: Vec<Section> = if r.bool() {
            let c = [
                Point2::new(rc - w, -h),
                Point2::new(rc + w, -h),
                Point2::new(rc + w, h),
                Point2::new(rc - w, h),
            ];
            let mut s: Vec<Section> = (0..4)
                .map(|i| Section::Line {
                    a: c[i],
                    b: c[(i + 1) % 4],
                })
                .collect();
            if r.bool() {
                // the top bowed up into an arc through its corners
                let bulge = r.range(0.2, 1.0);
                let rad = (w * w + bulge * bulge) / (2.0 * bulge);
                let cc = Point2::new(rc, h + bulge - rad);
                let a0 = dmath::atan2(h - cc.y, w);
                s[2] = Section::Arc {
                    c: cc,
                    r: rad,
                    start: a0,
                    sweep: dmath::PI - 2.0 * a0,
                };
            }
            s
        } else {
            let rad = h.min(w);
            let c = Point2::new(rc, 0.0);
            vec![
                Section::Arc {
                    c,
                    r: rad,
                    start: 0.0,
                    sweep: dmath::PI,
                },
                Section::Arc {
                    c,
                    r: rad,
                    start: dmath::PI,
                    sweep: dmath::PI,
                },
            ]
        };
        if !ccw {
            loop_ = loop_
                .iter()
                .rev()
                .map(|s| match *s {
                    Section::Line { a, b } => Section::Line { a: b, b: a },
                    Section::Arc { c, r, start, sweep } => Section::Arc {
                        c,
                        r,
                        start: start + sweep,
                        sweep: -sweep,
                    },
                })
                .collect();
        }
        let turns = r.range(0.3, 4.0);
        let pitch = r.range(6.0, 9.0);
        let b = pitch / dmath::TAU * if r.below(4) == 0 { 0.0 } else { 1.0 };
        let a = if b == 0.0 || r.bool() {
            0.0
        } else {
            r.range(-0.2, 0.2)
        };
        let turns = if b == 0.0 { turns.min(0.9) } else { turns };
        let faces = Screw::loop_faces(f, &loop_, a, b, Interval::new(0.0, turns * dmath::TAU))
            .expect("clear of the axis");
        let k = r.below(faces.len());
        faces[k].clone()
    }

    fn rand_uv(r: &mut Rng, s: &Screw) -> Point2 {
        Point2::new(r.range(s.u.lo, s.u.hi), r.range(0.0, 1.0))
    }

    #[test]
    fn derivatives_match_finite_differences() {
        let mut r = Rng::new(510);
        for _ in 0..300 {
            let s = random_screw(&mut r);
            let uv = Point2::new(r.range(s.u.lo, s.u.hi), r.range(0.01, 0.99));
            let h = 1e-6;
            let d = s.point(uv);
            let at = |du: f64, dv: f64| s.point(Point2::new(uv.x + du, uv.y + dv));
            let close =
                |a: Vec3, b: Vec3| assert!((a - b).length() < 1e-5 * (1.0 + b.length()), "{a:?} {b:?}");
            close((at(h, 0.0).p - at(-h, 0.0).p) / (2.0 * h), d.du);
            close((at(0.0, h).p - at(0.0, -h).p) / (2.0 * h), d.dv);
            close((at(h, 0.0).du - at(-h, 0.0).du) / (2.0 * h), d.duu);
            close((at(0.0, h).du - at(0.0, -h).du) / (2.0 * h), d.duv);
            close((at(0.0, h).dv - at(0.0, -h).dv) / (2.0 * h), d.dvv);
        }
    }

    /// Nearest over a fine grid of the carried-on surface.
    fn grid_nearest(s: &Screw, p: Point3) -> f64 {
        let n = s.ring.pieces.len();
        let (nu, nv) = (((s.ue.width() / 0.05) as usize).max(50), 20 * n);
        let mut best = f64::INFINITY;
        for i in 0..=nu {
            let u = s.ue.lo + s.ue.width() * i as f64 / nu as f64;
            for j in 0..=nv {
                best = best.min(s.point(Point2::new(u, j as f64 / 20.0)).p.distance(p));
            }
        }
        best
    }

    #[test]
    fn project_inverts_eval_and_is_nearest() {
        let mut r = Rng::new(511);
        for _ in 0..60 {
            let s = random_screw(&mut r);
            let surf = s.clone().surface();
            for _ in 0..4 {
                let uv = rand_uv(&mut r, &s);
                let d = s.point(uv);
                let back = s.nearest(d.p);
                assert!(surf.eval(back).distance(d.p) < 1e-10, "{s:?} {uv:?}");
                assert!((back.y - uv.y).abs() < 1e-9 || s.point(back).p.distance(d.p) < 1e-10);
                let off = d.p + Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
                let q = s.point(s.nearest(off)).p;
                assert!(q.distance(off) <= grid_nearest(&s, off) + 1e-9, "{s:?} {uv:?}");
            }
        }
    }

    #[test]
    fn implicit_is_zero_on_the_surface_and_below_the_distance() {
        let mut r = Rng::new(512);
        for _ in 0..200 {
            let s = random_screw(&mut r);
            for _ in 0..10 {
                let uv = Point2::new(r.range(s.u.lo, s.u.hi), r.range(0.05, 0.95));
                let d = s.point(uv);
                let (f, g) = s.value(d.p);
                assert!(f.abs() < 1e-9, "{f} {s:?}");
                let n = d.du.cross(d.dv).normalize().unwrap();
                assert!(g.normalize().unwrap().dot(n) > 1.0 - 1e-9, "{g:?} {n:?}");
                // a small step along the natural normal reads (about) its length, with its sign
                let t = r.range(-0.01, 0.01);
                let (ft, _) = s.value(d.p + n * t);
                assert!((ft - t).abs() < 1e-2 * t.abs() + 1e-12, "{ft} {t} {s:?} {uv:?}");
            }
            // anywhere: never more than the distance to the face
            for _ in 0..5 {
                let b = Procedural::bbox(&s, Box2::new(s.u, Interval::new(0.0, s.ring.pieces.len() as f64)))
                    .grow(3.0);
                let p = Point3::new(
                    r.range(b.x.lo, b.x.hi),
                    r.range(b.y.lo, b.y.hi),
                    r.range(b.z.lo, b.z.hi),
                );
                let near = s.point(s.nearest(p)).p.distance(p);
                let f = s.value(p).0.abs();
                if f > near {
                    let nu = (s.u.width() / 0.02) as usize;
                    let face = (0..=nu)
                        .flat_map(|i| (0..=40).map(move |j| (i, j)))
                        .map(|(i, j)| {
                            s.point(Point2::new(
                                s.u.lo + s.u.width() * i as f64 / nu as f64,
                                j as f64 / 40.0,
                            ))
                            .p
                            .distance(p)
                        })
                        .fold(f64::INFINITY, f64::min);
                    assert!(f <= face * (1.0 + 1e-3), "{f} {near} {face} {p:?}");
                }
            }
        }
    }

    #[test]
    fn bbox_contains_samples() {
        let mut r = Rng::new(513);
        for _ in 0..200 {
            let s = random_screw(&mut r);
            let (a, b) = (rand_uv(&mut r, &s), rand_uv(&mut r, &s));
            let bx = Box2::new(Interval::new(a.x, b.x), Interval::new(a.y, b.y));
            let bb = Procedural::bbox(&s, bx);
            for i in 0..=10 {
                for j in 0..=10 {
                    let q = Point2::new(
                        (bx.u.lo + bx.u.width() * i as f64 / 10.0).min(bx.u.hi),
                        (bx.v.lo + bx.v.width() * j as f64 / 10.0).min(bx.v.hi),
                    );
                    assert!(bb.contains(s.point(q).p));
                }
            }
        }
    }

    #[test]
    fn helices_lie_on_the_surface_and_find_their_points() {
        let mut r = Rng::new(514);
        for _ in 0..200 {
            let s = random_screw(&mut r);
            let surf = s.clone().surface();
            let crate::Curve::Procedural(h) = s.helix(r.range(0.0, 1.0)) else {
                unreachable!()
            };
            let range = s.u;
            let b = h.bbox(range);
            for i in 0..=50 {
                let t = range.lo + range.width() * i as f64 / 50.0;
                let d = h.deriv(t);
                assert!(surf.distance(d.p) < 1e-9);
                assert!(b.contains(d.p));
                assert!(d.d1.length() <= h.speed_bound(range));
                let back = h.closest(d.p, range);
                assert!(h.deriv(back).p.distance(d.p) < 1e-9, "{t} {back}");
            }
        }
    }

    #[test]
    fn transform_moves_points() {
        let mut r = Rng::new(515);
        for _ in 0..100 {
            let s = random_screw(&mut r);
            let f = rand_frame(&mut r);
            let moved = Procedural::transformed(&s, &f);
            let uv = rand_uv(&mut r, &s);
            assert!(moved.deriv(uv).p.distance(f.point_to_world(s.point(uv).p)) < 1e-9);
        }
    }
}
