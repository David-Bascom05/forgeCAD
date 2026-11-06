//! Where a screw face with a straight section meets a plane square to its axis, or a cylinder or
//! cone about it, in closed form. At each turn angle u the section meets the other surface's
//! profile (a line in (r, h)) in one point, which moves linearly with u: the curve is a helix
//! (on a cylinder), a conical helix (on a cone) or a flat spiral (on a plane), all the screw's
//! own helix curve. Where the section runs along the profile it meets the surface at one angle,
//! along the section line there. Two screw faces are met in `pair`.

use super::{Helix, Motion, Screw, Section};
use crate::curve::{CurveDeriv, Domain1, Line, ProcCurve, ProceduralCurve};
use crate::intersect::{Contact, SurfaceSurface};
use crate::surface::Surface;
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec2, dmath, tol};
use std::sync::Arc;

/// The closed form for a screw face and a coaxial plane, cylinder or cone, in either order;
/// None for any other pair.
pub fn meet_coaxial(a: &Surface, b: &Surface) -> Option<SurfaceSurface> {
    match (screw(a), screw(b)) {
        (Some(s), None) => s.meet_coaxial(b),
        (None, Some(s)) => s.meet_coaxial(a),
        (Some(s), Some(t)) => s.meet_screw(t),
        _ => None,
    }
}

impl Screw {
    /// A plane through the axis holds the section at every angle it passes, a half turn apart
    /// (its two half planes): there the face meets it along the section line.
    fn meet_axial_plane(&self, p: &crate::surface::Plane, a: Point2, b: Point2) -> Option<SurfaceSurface> {
        let f = &self.m.frame;
        let n = p.frame.z();
        if n.dot(f.z().get()).abs() > tol::ANGULAR {
            return None;
        }
        // the half plane at angle th has e(th) square to n
        let th = forge_math::dmath::atan2(-n.dot(f.x().get()), n.dot(f.y().get()));
        let half = forge_math::dmath::PI;
        let k0 = ((self.ue.lo - th) / half).ceil() as i64;
        let k1 = ((self.ue.hi - th) / half).floor() as i64;
        let mut curves = Vec::new();
        for k in k0..=k1 {
            let u = th + half * k as f64;
            let (e, _, _) = self.m.axes(u);
            let at = |q: Point2| f.origin + e * (q.x + self.m.a * u) + f.z() * (q.y + self.m.b * u);
            if let Some(l) = Line::through(at(a), at(b)) {
                curves.push((crate::Curve::Line(l), Contact::Crossing));
            }
        }
        Some(if curves.is_empty() {
            SurfaceSurface::Empty
        } else {
            SurfaceSurface::Curves(curves)
        })
    }
}

pub(super) fn screw(s: &Surface) -> Option<&Screw> {
    match s {
        Surface::Procedural(p) => p.as_any().and_then(|x| x.downcast_ref::<Screw>()),
        _ => None,
    }
}

impl Screw {
    fn meet_coaxial(&self, other: &Surface) -> Option<SurfaceSurface> {
        let f = &self.m.frame;
        let z = f.z();
        let h = |p: Point3| z.dot(p - f.origin);
        let coaxial = |o: Point3, d: forge_math::UnitVec3| {
            let q = o - f.origin;
            d.cross(z).length() <= tol::ANGULAR && (q - z * z.dot(q)).length() <= tol::LINEAR
        };
        // the other surface's profile in (r, h): n . q = c
        let (n, c) = match other {
            Surface::Plane(p) if p.frame.z().cross(z).length() <= tol::ANGULAR => {
                (Vec2::new(0.0, 1.0), h(p.frame.origin))
            }
            Surface::Cylinder(cy) if coaxial(cy.frame.origin, cy.frame.z()) => {
                (Vec2::new(1.0, 0.0), cy.radius)
            }
            Surface::Cone(co) if coaxial(co.frame.origin, co.frame.z()) => {
                let up = co.frame.z().dot(z.get()).signum();
                let n = Vec2::new(up * co.cos_a(), -co.sin_a());
                (n, n.dot(Vec2::new(co.radius, h(co.frame.origin))))
            }
            Surface::Sphere(sp) if coaxial(sp.frame.origin, f.z()) => {
                return self.meet_round(Point2::new(0.0, h(sp.frame.origin)), sp.radius);
            }
            Surface::Torus(t) if t.major > t.minor + tol::LINEAR && coaxial(t.frame.origin, t.frame.z()) => {
                return self.meet_round(Point2::new(t.major, h(t.frame.origin)), t.minor);
            }
            Surface::Plane(p)
                if coaxial(p.frame.origin, p.frame.x()) || coaxial(p.frame.origin, p.frame.y()) =>
            {
                let Section::Line { a: p0, b: p1 } = self.ring.pieces[0].s else {
                    return None;
                };
                return self.meet_axial_plane(p, p0, p1);
            }
            _ => return None,
        };
        // the section's box carried over the face's angles: a profile clear of it is never met
        // (a coil round a rod it does not touch, whatever its section)
        let piece = &self.ring.pieces[0];
        let (rr, hh) = (piece.rr + self.u * self.m.a, piece.hh + self.u * self.m.b);
        let reach = rr * n.x + hh * n.y;
        if c < reach.lo - tol::LINEAR || c > reach.hi + tol::LINEAR {
            return Some(SurfaceSurface::Empty);
        }
        let Section::Line { a: p0, b: p1 } = piece.s else {
            return None;
        };
        let p0 = Vec2::new(p0.x, p0.y);
        let d = Vec2::new(p1.x, p1.y) - p0;
        let ab = Vec2::new(self.m.a, self.m.b);
        let (nd, nab, gap) = (n.dot(d), n.dot(ab), c - n.dot(p0));
        if nd.abs() <= 1e-12 * d.length() {
            // the section along the profile: met at the one angle where the motion brings it on
            if nab.abs() <= 1e-12 {
                return if gap.abs() <= tol::LINEAR {
                    None
                } else {
                    Some(SurfaceSurface::Empty)
                };
            }
            let u = gap / nab;
            if !self.ue.contains(u) {
                return Some(SurfaceSurface::Empty);
            }
            let (e, _, _) = self.m.axes(u);
            let at = |q: Vec2| f.origin + e * (q.x + self.m.a * u) + z * (q.y + self.m.b * u);
            let line = Line::through(at(p0), at(p0 + d))?;
            return Some(SurfaceSurface::Curves(vec![(
                crate::Curve::Line(line),
                Contact::Crossing,
            )]));
        }
        // s(u) = (gap - nab u) / nd along the section, the point q0 + v u in (r, h)
        let q0 = p0 + d * (gap / nd);
        let v = ab - d * (nab / nd);
        // the angles where s stays on the section, carried a quarter on past its ends
        let (s0, s1) = (-0.25, 1.25);
        let (mut lo, mut hi) = if nab == 0.0 {
            if !(s0..=s1).contains(&(gap / nd)) {
                return Some(SurfaceSurface::Empty);
            }
            (self.ue.lo, self.ue.hi)
        } else {
            let at = |s: f64| (gap - s * nd) / nab;
            let (x, y) = (at(s0), at(s1));
            (x.min(y).max(self.ue.lo), x.max(y).min(self.ue.hi))
        };
        // and off the axis (past it the point is the mirror image, on no face)
        if v.x > 0.0 {
            lo = lo.max(-q0.x / v.x);
        } else if v.x < 0.0 {
            hi = hi.min(-q0.x / v.x);
        } else if q0.x <= 0.0 {
            return Some(SurfaceSurface::Empty);
        }
        if lo >= hi {
            return Some(SurfaceSurface::Empty);
        }
        let range = Interval::new(lo, hi);
        let helix = Helix {
            m: Motion {
                frame: *f,
                a: v.x,
                b: v.y,
            },
            p: Point2::new(q0.x, q0.y),
            u: range,
            fence: None,
        };
        Some(SurfaceSurface::Curves(vec![(
            crate::Curve::Procedural(ProcCurve(Arc::new(helix))),
            Contact::Crossing,
        )]))
    }
}

impl Screw {
    /// A straight section against a coaxial round surface (a sphere centred on the axis, a ring
    /// torus about it) whose meridian in (r, h) is the circle about `c` of radius `r`: each arc of
    /// the circle the carried section passes over is a `Round` curve (research/thread-math 7).
    fn meet_round(&self, c: Point2, r: f64) -> Option<SurfaceSurface> {
        let piece = &self.ring.pieces[0];
        let Section::Line { a: p0, b: p1 } = piece.s else {
            return None;
        };
        // the section's box carried over the face's angles: a meridian clear of it is never met
        let (rr, hh) = (piece.rr + self.u * self.m.a, piece.hh + self.u * self.m.b);
        let g = tol::LINEAR;
        if c.x + r < rr.lo - g || c.x - r > rr.hi + g || c.y + r < hh.lo - g || c.y - r > hh.hi + g {
            return Some(SurfaceSurface::Empty);
        }
        let d = p1 - p0;
        let dd = d.length_sq();
        let n = Vec2::new(-d.y, d.x) / dd.sqrt();
        let ab = Vec2::new(self.m.a, self.m.b);
        let nm = n.dot(ab);
        if ab == Vec2::ZERO {
            return None;
        }
        if nm.abs() <= 1e-12 * ab.length() {
            return self.meet_round_along(p0, d, c, r);
        }
        let w = c - p0;
        let u = (n.dot(w) / nm, r * n.x / nm, r * n.y / nm);
        // s(t) along the section, a quarter past its ends as `meet_coaxial` carries it
        let dm = d.dot(ab);
        let s = (
            (d.dot(w) - dm * u.0) / dd,
            (r * d.x - dm * u.1) / dd,
            (r * d.y - dm * u.2) / dd,
        );
        let limits = [
            (u.0 - self.ue.lo, u.1, u.2),
            (self.ue.hi - u.0, -u.1, -u.2),
            (s.0 + 0.25, s.1, s.2),
            (1.25 - s.0, -s.1, -s.2),
            // off the axis: past it the point is the mirror image, on no face
            (c.x, r, 0.0),
        ];
        let arcs = arcs_where(&limits)?;
        let curves: Vec<_> = arcs
            .into_iter()
            .map(|t| {
                let curve = Round {
                    m: self.m,
                    c,
                    r,
                    u,
                    t,
                };
                (
                    crate::Curve::Procedural(ProcCurve(Arc::new(curve))),
                    Contact::Crossing,
                )
            })
            .collect();
        Some(if curves.is_empty() {
            SurfaceSurface::Empty
        } else {
            SurfaceSurface::Curves(curves)
        })
    }
}

impl Screw {
    /// `meet_round` where the motion runs along the section line: the face is a piece of the
    /// surface of revolution of that line, so it meets the circle in the line's points on it, each
    /// a circle about the axis, over the angles the section passes it at.
    fn meet_round_along(&self, p0: Point2, d: Vec2, c: Point2, r: f64) -> Option<SurfaceSurface> {
        let dd = d.length_sq();
        let ab = Vec2::new(self.m.a, self.m.b);
        // the section point at s and angle u lies on the line at sigma = s + lambda u
        let lambda = ab.dot(d) / dd;
        // |p0 + d sigma - c|^2 = r^2
        let w = p0 - c;
        let (bq, cq) = (w.dot(d) / dd, (w.dot(w) - r * r) / dd);
        let disc = bq * bq - cq;
        if disc < 0.0 {
            return Some(SurfaceSurface::Empty);
        }
        let root = disc.sqrt();
        let f = &self.m.frame;
        let mut curves = Vec::new();
        for sigma in [-bq - root, -bq + root] {
            if root == 0.0 && !curves.is_empty() {
                break;
            }
            let q = p0 + d * sigma;
            if q.x <= 0.0 {
                continue;
            }
            // s = sigma - lambda u within a quarter past the section's ends
            let (x, y) = ((sigma + 0.25) / lambda, (sigma - 1.25) / lambda);
            let (lo, hi) = (x.min(y).max(self.ue.lo), x.max(y).min(self.ue.hi));
            if lo >= hi {
                continue;
            }
            let circle = Helix {
                m: Motion {
                    frame: *f,
                    a: 0.0,
                    b: 0.0,
                },
                p: q,
                u: Interval::new(lo, hi),
                fence: None,
            };
            curves.push((
                crate::Curve::Procedural(ProcCurve(Arc::new(circle))),
                Contact::Crossing,
            ));
        }
        Some(if curves.is_empty() {
            SurfaceSurface::Empty
        } else {
            SurfaceSurface::Curves(curves)
        })
    }
}

/// The arcs of t in one turn where every `alpha + beta cos t + gamma sin t >= 0`, as increasing
/// intervals; None when they all hold all round (a closed curve, left to the tracer).
fn arcs_where(limits: &[(f64, f64, f64)]) -> Option<Vec<Interval>> {
    let held = |t: f64| {
        let (sn, cs) = dmath::sin_cos(t);
        limits.iter().all(|&(a, b, g)| a + b * cs + g * sn >= 0.0)
    };
    // start where the limit that fails worst is least: no arc runs across the start
    let mut start = None;
    for &(a, b, g) in limits {
        let rho = (b * b + g * g).sqrt();
        let low = a - rho;
        if low < 0.0 && start.is_none_or(|(l, _)| low < l) {
            let phi = if rho > 0.0 { dmath::atan2(g, b) } else { 0.0 };
            start = Some((low, phi + dmath::PI));
        }
    }
    let (_, t0) = start?;
    let wrap = |t: f64| t0 + (t - t0).rem_euclid(dmath::TAU);
    let mut cuts = vec![t0, t0 + dmath::TAU];
    for &(a, b, g) in limits {
        let rho = (b * b + g * g).sqrt();
        if rho > a.abs() {
            let phi = dmath::atan2(g, b);
            let half = dmath::acos(-a / rho);
            cuts.extend([wrap(phi - half), wrap(phi + half)]);
        }
    }
    cuts.sort_by(f64::total_cmp);
    let mut out: Vec<Interval> = Vec::new();
    for w in cuts.windows(2) {
        let (lo, hi) = (w[0], w[1]);
        if hi - lo <= 1e-12 || !held(0.5 * (lo + hi)) {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.hi == lo => last.hi = hi,
            _ => out.push(Interval::new(lo, hi)),
        }
    }
    Some(out)
}

/// A straight screw section's meeting with a coaxial round surface: the meridian circle's point
/// Q(t) = c + r (cos t, sin t) in the half plane at angle u(t) = u0 + uc cos t + us sin t.
#[derive(Clone, Debug)]
struct Round {
    m: Motion,
    c: Point2,
    r: f64,
    /// u0, uc, us.
    u: (f64, f64, f64),
    t: Interval,
}

impl Round {
    /// A bound on |u'| and |u''|.
    fn turn_rate(&self) -> f64 {
        (self.u.1 * self.u.1 + self.u.2 * self.u.2).sqrt()
    }
}

impl ProceduralCurve for Round {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn exact_d1(&self) -> bool {
        true
    }
    fn deriv(&self, t: f64) -> CurveDeriv {
        let (sn, cs) = dmath::sin_cos(t);
        let (u0, uc, us) = self.u;
        let u = u0 + uc * cs + us * sn;
        let (u1, u2) = (us * cs - uc * sn, -(uc * cs + us * sn));
        let (qr, qh) = (self.c.x + self.r * cs, self.c.y + self.r * sn);
        let (q1r, q1h) = (-self.r * sn, self.r * cs);
        let (e, ep, z) = self.m.axes(u);
        CurveDeriv {
            p: self.m.frame.origin + e * qr + z * qh,
            d1: ep * (u1 * qr) + e * q1r + z * q1h,
            d2: e * (-(u1 * u1) * qr - qr + self.c.x) + ep * (u2 * qr + 2.0 * u1 * q1r) - z * (qh - self.c.y),
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: self.t.lo,
            hi: self.t.hi,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let (_, phi, _) = self.m.cylindrical(p);
        let mut best = (f64::INFINITY, range.lo);
        let mut take = |t: f64| {
            let d = self.deriv(t).p.distance_sq(p);
            if d < best.0 {
                best = (d, t);
            }
        };
        take(range.lo);
        take(range.hi);
        // in p's half plane: u(t) = phi + 2 pi k, u0 + rho cos(t - psi) = phi + 2 pi k, in closed
        // form for each turn the curve reaches
        let (u0, uc, us) = self.u;
        let rho = self.turn_rate();
        if rho > 0.0 {
            let psi = dmath::atan2(us, uc);
            let k0 = ((u0 - rho - phi) / dmath::TAU).ceil() as i64;
            let k1 = ((u0 + rho - phi) / dmath::TAU).floor() as i64;
            for k in k0..=k1 {
                let c = ((phi + dmath::TAU * k as f64 - u0) / rho).clamp(-1.0, 1.0);
                let half = dmath::acos(c);
                for t in [psi - half, psi + half] {
                    // the turn of t in range nearest it
                    let lo = range.lo + (t - range.lo).rem_euclid(dmath::TAU);
                    for t in [lo, lo + dmath::TAU] {
                        if range.contains(t) {
                            take(t);
                        }
                    }
                }
            }
        }
        // and a few even steps, for a point near the axis whose angle says little
        for i in 1..8 {
            take(range.lo + range.width() * i as f64 / 8.0);
        }
        // Newton on (P - p).P' = 0, kept inside the range while it gets closer
        let (mut t, mut f) = (best.1, best.0);
        for _ in 0..16 {
            let d = self.deriv(t);
            let e = d.p - p;
            let (g, h) = (e.dot(d.d1), d.d1.length_sq() + e.dot(d.d2));
            let h = if h > 0.0 { h } else { d.d1.length_sq() };
            if h <= 0.0 {
                break;
            }
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
        let (cs, sn) = (range.cos(), range.sin());
        let (u0, uc, us) = self.u;
        let u = cs * uc + sn * us + u0;
        let r = cs * self.r + self.c.x;
        let h = sn * self.r + self.c.y;
        self.m.bbox(u, r, h).grow(tol::LINEAR)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        let mut c = self.clone();
        c.m.frame = f.compose(&self.m.frame);
        ProcCurve(Arc::new(c))
    }
    fn speed_bound(&self, _range: Interval) -> f64 {
        // |P'|^2 = (u' Q_r)^2 + r^2
        let k = self.turn_rate() * (self.c.x + self.r);
        (k * k + self.r * self.r).sqrt() * (1.0 + 1e-12)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        // equal steps h keep each chord within |P''| h^2 / 8 of the curve
        let (k, q) = (self.turn_rate(), self.c.x + self.r);
        let acc = k * k * q + k * q + 2.0 * k * self.r + self.r;
        let n = range.width() * (acc / (8.0 * chord.max(tol::LINEAR))).sqrt();
        (n.ceil() as usize).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::CurveGeom;
    use crate::surface::{Cone, Cylinder, Plane, SurfaceGeom};
    use crate::testing::rand_frame;
    use forge_math::rng::Rng;
    use forge_math::{Box2, Frame, Point2};

    /// Every point of every closed-form curve lies on both surfaces.
    #[test]
    fn curves_lie_on_both_surfaces() {
        let mut r = Rng::new(733);
        let mut checked = 0;
        for _ in 0..60 {
            let f = rand_frame(&mut r);
            let (a, b) = (r.range(-0.2, 0.2), r.range(-0.5, 0.5));
            let (r0, h0) = (r.range(2.0, 4.0), r.range(-1.0, 1.0));
            let (w, h) = (r.range(0.5, 1.5), r.range(0.3, 1.0));
            let c = [
                Point2::new(r0, h0),
                Point2::new(r0 + w, h0 + r.range(-0.3, 0.3)),
                Point2::new(r0 + w, h0 + h),
                Point2::new(r0, h0 + h),
            ];
            let loop4: Vec<Section> = (0..4)
                .map(|k| Section::Line {
                    a: c[k],
                    b: c[(k + 1) % 4],
                })
                .collect();
            let Some(faces) = Screw::loop_faces(f, &loop4, a, b, Interval::new(0.0, r.range(2.0, 9.0)))
            else {
                continue;
            };
            let up = |o: f64| f.with_origin(f.origin + f.z() * o);
            let others = [
                Surface::Plane(Plane::new(up(r.range(-1.0, 3.0)))),
                Surface::Cylinder(Cylinder {
                    frame: up(r.range(-2.0, 2.0)),
                    radius: r.range(2.0, 6.0),
                }),
                Surface::Cone(
                    Cone::new(up(r.range(-1.0, 1.0)), r.range(2.0, 5.0), r.range(0.2, 1.2)).expect("a cone"),
                ),
                Surface::Cone(
                    Cone::new(
                        Frame::from_zx(f.origin, -f.z(), f.x().get()).expect("axes"),
                        r.range(2.0, 5.0),
                        r.range(0.2, 1.2),
                    )
                    .expect("a cone"),
                ),
            ];
            for face in faces {
                let s = face.clone().surface();
                for o in &others {
                    let Some(SurfaceSurface::Curves(cs)) = meet_coaxial(&s, o) else {
                        continue;
                    };
                    for (curve, _) in cs {
                        // a section line at one angle meets it too; the helices are checked here
                        let crate::Curve::Procedural(pc) = &curve else {
                            continue;
                        };
                        let d = pc.domain();
                        for i in 0..=10 {
                            let t = d.lo + (d.hi - d.lo) * i as f64 / 10.0;
                            let p = curve.eval(t);
                            assert!(o.distance(p) < 1e-9, "off the other surface by {}", o.distance(p));
                            // the point at angle t: its section point lies on the section's line
                            let l = f.point_to_local(p);
                            let (sn, cs) = forge_math::dmath::sin_cos(t);
                            let rho = l.x * cs + l.y * sn;
                            assert!((l.y * cs - l.x * sn).abs() < 1e-9 && rho > 0.0);
                            let q = Point2::new(rho - a * t, l.z - b * t);
                            let Section::Line { a: s0, b: s1 } = face.ring.pieces[0].s else {
                                unreachable!()
                            };
                            let dir = (s1 - s0).normalize().expect("a section");
                            assert!(
                                (q - s0).cross(dir).abs() < 1e-9,
                                "off the screw by {}",
                                (q - s0).cross(dir)
                            );
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert!(checked > 300, "{checked}");
    }

    /// A straight section against a sphere centred on the axis or a ring torus about it: every
    /// point of every closed-form curve lies on both surfaces, and every point the tracer finds on
    /// the face lies on one of them.
    #[test]
    fn round_meets_lie_on_both_and_match_the_tracer() {
        use crate::surface::{Sphere, Torus};
        let mut r = Rng::new(9127);
        let (mut checked, mut traced) = (0, 0);
        for _ in 0..40 {
            let f = rand_frame(&mut r);
            let (a, b) = (
                if r.range(0.0, 1.0) < 0.5 {
                    0.0
                } else {
                    r.range(-0.1, 0.1)
                },
                r.range(0.1, 0.4),
            );
            let (r0, h0) = (r.range(2.0, 3.0), r.range(-0.5, 0.5));
            let (w, h) = (r.range(0.5, 1.0), r.range(0.3, 0.6));
            let c = [
                Point2::new(r0, h0),
                Point2::new(r0 + w, h0 + r.range(-0.2, 0.2)),
                Point2::new(r0 + w, h0 + h),
                Point2::new(r0, h0 + h),
            ];
            let loop4: Vec<Section> = (0..4)
                .map(|k| Section::Line {
                    a: c[k],
                    b: c[(k + 1) % 4],
                })
                .collect();
            let Some(faces) = Screw::loop_faces(f, &loop4, a, b, Interval::new(0.0, r.range(6.0, 14.0)))
            else {
                continue;
            };
            let up = |o: f64| f.with_origin(f.origin + f.z() * o);
            let others = [
                Surface::Sphere(Sphere {
                    frame: up(r.range(-3.0, 3.0)),
                    radius: r.range(2.5, 6.0),
                }),
                Surface::Torus(
                    Torus::new(up(r.range(-1.0, 3.0)), r.range(2.0, 4.0), r.range(0.4, 1.2))
                        .expect("a torus"),
                ),
            ];
            for face in faces {
                let s = face.clone().surface();
                for o in &others {
                    let Some(found) = meet_coaxial(&s, o) else {
                        continue;
                    };
                    let cs = match &found {
                        SurfaceSurface::Curves(cs) => cs.clone(),
                        SurfaceSurface::Empty => vec![],
                        other => panic!("{other:?}"),
                    };
                    for (curve, _) in &cs {
                        let crate::Curve::Procedural(pc) = curve else {
                            panic!("a procedural curve");
                        };
                        let d = pc.domain();
                        for i in 0..=20 {
                            let t = d.lo + (d.hi - d.lo) * i as f64 / 20.0;
                            let p = curve.eval(t);
                            assert!(o.distance(p) < 1e-9, "off the round surface by {}", o.distance(p));
                            // at its angle u, the point's section point lies on the section's line
                            let u = match pc.as_any().and_then(|x| x.downcast_ref::<Round>()) {
                                Some(rc) => {
                                    let (sn, cs) = forge_math::dmath::sin_cos(t);
                                    rc.u.0 + rc.u.1 * cs + rc.u.2 * sn
                                }
                                None => t,
                            };
                            let l = f.point_to_local(p);
                            let (sn, cs) = forge_math::dmath::sin_cos(u);
                            let rho = l.x * cs + l.y * sn;
                            assert!((l.y * cs - l.x * sn).abs() < 1e-9 && rho > 0.0, "off the angle");
                            let q = Point2::new(rho - a * u, l.z - b * u);
                            let Section::Line { a: s0, b: s1 } = face.ring.pieces[0].s else {
                                unreachable!()
                            };
                            let dir = (s1 - s0).normalize().expect("a section");
                            let off = (q - s0).cross(dir);
                            assert!(off.abs() < 1e-9, "off the screw by {off}");
                            let along = (q - s0).dot(dir) / s0.distance(s1);
                            assert!(
                                (-0.25 - 1e-9..=1.25 + 1e-9).contains(&along),
                                "past the section: {along}"
                            );
                            // the derivative is the curve's own
                            let e = 1e-6;
                            let fd = (curve.eval(t + e) - curve.eval(t - e)) / (2.0 * e);
                            let d1 = pc.deriv(t).d1;
                            assert!(
                                (fd - d1).length() < 1e-5 * (1.0 + d1.length()),
                                "d1 {fd:?} {d1:?}"
                            );
                            checked += 1;
                        }
                    }
                    // the tracer's points on the face lie on a closed-form curve
                    let region = s.bbox(Box2 {
                        u: Interval::new(face.u.lo, face.u.hi),
                        v: Interval::new(0.0, 1.0),
                    });
                    let Some(chains) = crate::intcurve::trace(&s, o, region) else {
                        continue;
                    };
                    let on_face = |p: Point3| {
                        let uv = s.project(p);
                        uv.y > 0.01 && uv.y < 0.99 && uv.x > face.u.lo + 0.01 && uv.x < face.u.hi - 0.01
                    };
                    for ch in &chains {
                        for &p in &ch.pts {
                            if !on_face(p) {
                                continue;
                            }
                            let near = cs
                                .iter()
                                .map(|(c, _)| {
                                    let crate::Curve::Procedural(pc) = c else {
                                        unreachable!()
                                    };
                                    let d = pc.domain();
                                    let t = pc.closest(p, Interval::new(d.lo, d.hi));
                                    c.eval(t).distance(p)
                                })
                                .fold(f64::INFINITY, f64::min);
                            assert!(near < 1e-6, "a traced point {near} off every closed-form curve");
                            traced += 1;
                        }
                    }
                }
            }
        }
        assert!(checked > 500 && traced > 200, "{checked} {traced}");
    }
}
