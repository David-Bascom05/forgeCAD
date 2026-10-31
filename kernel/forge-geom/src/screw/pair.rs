//! Where two screw faces with straight sections about one axis, rising the same way, meet in
//! closed form. In the half plane at angle u (of the first face's turn) each face shows its
//! section, carried out and up linearly in u, once per turn of the other face; two such lines
//! cross at a point that moves linearly with u, so each crossing is a (conical) helix. Sections
//! that lie along each other meet at the one angle where they come together, along a line.

use super::{Helix, Motion, Screw, Section};
use crate::curve::{Line, ProcCurve};
use crate::intersect::{Contact, SurfaceSurface};
use forge_math::{Interval, Point2, Vec2, dmath, tol};
use std::sync::Arc;

impl Screw {
    pub(super) fn meet_screw(&self, o: &Screw) -> Option<SurfaceSurface> {
        let (Section::Line { a: p1, b: q1 }, Section::Line { a: p2, b: q2 }) =
            (self.ring.pieces[0].s, o.ring.pieces[0].s)
        else {
            return None;
        };
        let (f, g) = (&self.m.frame, &o.m.frame);
        let z = f.z();
        let off = g.origin - f.origin;
        if (z.get() - g.z().get()).length() > tol::ANGULAR || (off - z * z.dot(off)).length() > tol::LINEAR {
            return None;
        }
        // the other face's turn u2 = u - phi0 + 2 pi k, its sections raised by h0
        let phi0 = dmath::atan2(g.x().dot(f.y().get()), g.x().dot(f.x().get()));
        let h0 = z.dot(off);
        let (d1, d2) = (q1 - p1, q2 - p2);
        let (ab1, ab2) = (Vec2::new(self.m.a, self.m.b), Vec2::new(o.m.a, o.m.b));
        let p2 = Point2::new(p2.x, p2.y + h0);
        let det = d1.cross(d2);
        let (lo, hi) = (self.ue.lo, self.ue.hi);
        // a quarter of the section, but a section along its own turn's rise (a cylinder's wall)
        // stops well short of its next turn, or a point there would be found again on that turn
        let past = |d: Vec2, ab: Vec2| {
            let turn = ab * dmath::TAU;
            if d.cross(turn).abs() <= 1e-9 * d.length() * turn.length() && d.dot(turn) != 0.0 {
                (0.25 * (turn.length() / d.length() - 1.0)).clamp(0.0, 0.25)
            } else {
                0.25
            }
        };
        let (e1, e2) = (past(d1, ab1), past(d2, ab2));
        let k0 = ((o.ue.lo - hi + phi0) / dmath::TAU).floor() as i64;
        let k1 = ((o.ue.hi - lo + phi0) / dmath::TAU).ceil() as i64;
        let mut curves = Vec::new();
        for k in k0..=k1 {
            // u2 = u + c2 with c2 the turn's shift; the gap between the sections' starts is
            // w0 + wu u
            let c2 = -phi0 + dmath::TAU * k as f64;
            let w0 = (p2 - p1) + ab2 * c2;
            let wu = ab2 - ab1;
            let (mut ulo, mut uhi) = (lo.max(o.ue.lo - c2), hi.min(o.ue.hi - c2));
            if ulo >= uhi {
                continue;
            }
            if det.abs() <= 1e-12 * d1.length() * d2.length() {
                // parallel sections: on one line where n.(w0 + wu u) = 0
                let n = Vec2::new(-d1.y, d1.x) / d1.length();
                let (c, m) = (n.dot(w0), n.dot(wu));
                if m.abs() <= 1e-12 {
                    // the same surface crosses itself nowhere (the tracer's seeds only found that
                    // when none landed exactly on it)
                    if c.abs() <= tol::LINEAR {
                        return Some(SurfaceSurface::Empty);
                    }
                    continue;
                }
                let u = -c / m;
                if !(ulo..=uhi).contains(&u) {
                    continue;
                }
                // the overlap of the two sections along d1
                let (e, _, _) = self.m.axes(u);
                let base = p1 + ab1 * u;
                let s_of = |q: Point2| (q - base).dot(d1) / d1.length_sq();
                let (t0, t1) = (s_of(p2 + ab2 * (u + c2)), s_of(p2 + ab2 * (u + c2) + d2));
                let (a, b) = (t0.min(t1).max(0.0), t0.max(t1).min(1.0));
                if a >= b {
                    continue;
                }
                let at = |s: f64| {
                    let q = base + d1 * s;
                    f.origin + e * q.x + z * q.y
                };
                if let Some(l) = Line::through(at(a), at(b)) {
                    curves.push((crate::Curve::Line(l), Contact::Crossing));
                }
                continue;
            }
            // s d1 - t d2 = w0 + wu u: s and t linear in u
            let solve = |w: Vec2| (w.cross(d2) / det, w.cross(d1) / det);
            let (s0, t0) = solve(w0);
            let (su, tu) = solve(wu);
            // both sections carried a little past their ends, and off the axis
            for (x0, xu, e) in [(s0, su, e1), (t0, tu, e2)] {
                if xu == 0.0 {
                    if !(-e..=1.0 + e).contains(&x0) {
                        (ulo, uhi) = (1.0, 0.0);
                    }
                } else {
                    let (a, b) = ((-e - x0) / xu, (1.0 + e - x0) / xu);
                    (ulo, uhi) = (ulo.max(a.min(b)), uhi.min(a.max(b)));
                }
            }
            let q0 = p1 + d1 * s0;
            let v = ab1 + d1 * su;
            if v.x > 0.0 {
                ulo = ulo.max(-q0.x / v.x);
            } else if v.x < 0.0 {
                uhi = uhi.min(-q0.x / v.x);
            } else if q0.x <= 0.0 {
                continue;
            }
            if ulo >= uhi {
                continue;
            }
            let helix = Helix {
                m: Motion {
                    frame: *f,
                    a: v.x,
                    b: v.y,
                },
                p: q0,
                u: Interval::new(ulo, uhi),
                fence: None,
            };
            curves.push((
                crate::Curve::Procedural(ProcCurve(Arc::new(helix))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::CurveGeom;
    use crate::testing::rand_frame;
    use forge_math::rng::Rng;

    fn rect(r0: f64, h0: f64, w: f64, h: f64, tilt: f64) -> Vec<Section> {
        let c = [
            Point2::new(r0, h0),
            Point2::new(r0 + w, h0 + tilt),
            Point2::new(r0 + w, h0 + h - tilt),
            Point2::new(r0, h0 + h),
        ];
        (0..4)
            .map(|k| Section::Line {
                a: c[k],
                b: c[(k + 1) % 4],
            })
            .collect()
    }

    /// Every point of every crossing lies on both faces' sections at its angle.
    #[test]
    fn screws_meet_screws_in_closed_form() {
        let mut r = Rng::new(744);
        let mut checked = 0;
        for _ in 0..80 {
            let f = rand_frame(&mut r);
            let b = r.range(0.3, 0.6);
            let g = f
                .rotated_about_z(r.range(0.0, 6.0))
                .with_origin(f.origin + f.z() * r.range(-1.0, 1.0));
            let one = Screw::loop_faces(f, &rect(3.0, 0.0, 1.0, 0.8, 0.1), 0.0, b, Interval::new(0.0, 7.0));
            let two = Screw::loop_faces(
                g,
                &rect(
                    r.range(2.5, 3.5),
                    r.range(-0.4, 0.4),
                    1.0,
                    0.7,
                    if r.bool() { 0.1 } else { 0.15 },
                ),
                r.range(-0.1, 0.1),
                b,
                Interval::new(0.0, r.range(2.0, 7.0)),
            );
            let (Some(one), Some(two)) = (one, two) else {
                continue;
            };
            for s in &one {
                for t in &two {
                    let Some(SurfaceSurface::Curves(cs)) = s.meet_screw(t) else {
                        continue;
                    };
                    for (c, _) in cs {
                        // a helix over its domain; a section line from where the overlap starts
                        let (lo, hi) = match &c {
                            crate::Curve::Procedural(pc) => (pc.domain().lo, pc.domain().hi),
                            _ => (0.0, 0.05),
                        };
                        for i in 0..=8 {
                            let u = lo + (hi - lo) * i as f64 / 8.0;
                            let p = c.eval(u);
                            for (sc, at) in [(s, f), (t, g)] {
                                // the point's section point at its angle in that face's frame
                                let l = at.point_to_local(p);
                                let ang = forge_math::dmath::atan2(l.y, l.x);
                                let rho = (l.x * l.x + l.y * l.y).sqrt();
                                let Section::Line { a, b: e } = sc.ring.pieces[0].s else {
                                    unreachable!()
                                };
                                let dir = (e - a).normalize().expect("a section");
                                // some turn of the face's motion puts it on the section's line
                                let on = (-12..=12).any(|k| {
                                    let uu = ang + dmath::TAU * k as f64;
                                    let q = Point2::new(rho - sc.m.a * uu, l.z - sc.m.b * uu);
                                    (q - a).cross(dir).abs() < 1e-8
                                });
                                assert!(on, "off a face's section");
                            }
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert!(checked > 200, "{checked}");
    }
}
