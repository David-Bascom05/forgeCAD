//! Where a screw face with a straight section meets a surface at a general placement (a cylinder
//! on an oblique axis, a sphere, a torus): no closed form, so the section line is swept through
//! the face's angles and met with the surface in closed form at each (a line against a quadric or
//! torus). The roots along the section, followed from angle to angle, are the curve: a track ends
//! where its root leaves the section, and two tracks join where their roots meet (the section
//! touches the surface there and the curve turns back in angle). Only the face's own section is
//! met this way; the marching tracer, crossing the corners of the track that carries the section
//! on, turned back there. Open ends go on two tracer steps, so the face's edges are crossed.

use super::{Screw, Section};
use crate::curve::Line;
use crate::intersect::{Contact, LineSurface, SurfaceSurface, line_surface};
use crate::surface::Surface;
use forge_math::{Box3, Interval, Point3, tol};

/// One root followed over consecutive angles, and what each end joins: (track, its end), or open.
struct Track {
    /// (angle, section parameter) of each root.
    pts: Vec<(f64, f64)>,
    ends: [Option<(usize, usize)>; 2],
}

/// Of `more` roots (by section parameter), the ones that went (`k` of them, adjacent when two),
/// leaving the rest nearest `fewer` in order.
fn gone(more: &[f64], fewer: &[f64]) -> Option<usize> {
    let k = more.len().checked_sub(fewer.len())?;
    (0..=more.len().checked_sub(k)?)
        .map(|j| {
            let rest = (0..more.len()).filter(|&i| i < j || i >= j + k);
            (rest.zip(fewer).map(|(i, f)| (more[i] - f).abs()).sum::<f64>(), j)
        })
        .min_by(|x, y| x.0.total_cmp(&y.0))
        .map(|x| x.1)
}

/// The swept closed form for a screw face and another surface, in either order; None for any
/// other pair, or where it cannot follow the roots.
pub fn meet_general(a: &Surface, b: &Surface, region: Box3) -> Option<SurfaceSurface> {
    match (super::coaxial::screw(a), super::coaxial::screw(b)) {
        (Some(s), None) => s.meet_general(b, region),
        (None, Some(s)) => s.meet_general(a, region),
        _ => None,
    }
}

/// Whether a screw face plainly misses a cylinder within `region`: the face lies in a shell about
/// its axis, so over the stretch of the axis the region and the face's turns share, it is farther
/// from the cylinder's axis than the radius plus its shell, or nearer than the radius less it.
pub fn misses(a: &Surface, b: &Surface, region: Box3) -> bool {
    let (s, other) = match (super::coaxial::screw(a), super::coaxial::screw(b)) {
        (Some(s), None) => (s, b),
        (None, Some(s)) => (s, a),
        _ => return false,
    };
    let Surface::Cylinder(c) = other else {
        return false;
    };
    let f = &s.m.frame;
    let z = f.z().get();
    let o = f.origin.to_vec();
    let along = region.x * z.x + region.y * z.y + region.z * z.z + -(o.dot(z));
    let piece = &s.ring.pieces[0];
    let (rr, hh) = (piece.rr + s.u * s.m.a, piece.hh + s.u * s.m.b);
    let (lo, hi) = (along.lo.max(hh.lo), along.hi.min(hh.hi));
    if lo > hi {
        return true;
    }
    let axis = c.frame.z().get();
    // distance from the cylinder's axis, convex along the screw's axis
    let off = |t: f64| {
        let w = f.origin + f.z() * t - c.frame.origin;
        (w - axis * axis.dot(w)).length()
    };
    let (d0, d1) = (off(lo), off(hi));
    // the nearest point of the stretch: where the line through it comes closest
    let w0 = f.origin + f.z() * lo - c.frame.origin;
    let (e, g) = (w0 - axis * axis.dot(w0), z - axis * axis.dot(z));
    let gg = g.dot(g);
    let tmin = if gg > 0.0 {
        (-e.dot(g) / gg).clamp(0.0, hi - lo)
    } else {
        0.0
    };
    let dmin = off(lo + tmin);
    let margin = 4.0 * tol::LINEAR + rr.hi.max(0.0);
    if dmin > c.radius + margin || d0.max(d1) < c.radius - margin {
        return true;
    }
    // seen along the cylinder's axis the screw's axis is a line; along that line a point of the
    // shell strays from its axis point by at most r |a . z|, and its distance from the cylinder's
    // axis is at least that of its place along the line
    if gg <= 1e-18 {
        return false;
    }
    let u = g / gg.sqrt();
    let (x0, x1) = (u.dot(e), u.dot(e) + gg.sqrt() * (hi - lo));
    let along_min = if x0 <= 0.0 && x1 >= 0.0 {
        0.0
    } else {
        x0.abs().min(x1.abs())
    };
    along_min > c.radius + 4.0 * tol::LINEAR + rr.hi.max(0.0) * axis.dot(z).abs()
}

impl Screw {
    fn meet_general(&self, other: &Surface, region: Box3) -> Option<SurfaceSurface> {
        let Section::Line { a: p, b: q } = self.ring.pieces[0].s else {
            return None;
        };
        if matches!(other, Surface::Procedural(_)) {
            return None;
        }
        let d = q - p;
        let len = d.length();
        if len <= tol::LINEAR {
            return None;
        }
        let (f, (a, b)) = (&self.m.frame, (self.m.a, self.m.b));
        let z = f.z().get();
        // the tracer's spacing; the face's angles carried two steps past their ends
        let step = 0.5 * (region.diagonal().max(1e-6) / 64.0).max(10.0 * tol::LINEAR);
        let du = step / (self.rmax * self.rmax + a * a + b * b).sqrt().max(tol::LINEAR);
        // only the turns whose section reaches the region along the axis
        let mut span = self.u;
        if b != 0.0 {
            // the region's extent along the axis
            let o = f.origin.to_vec();
            let along = region.x * z.x + region.y * z.y + region.z * z.z + -(o.dot(z));
            let (zlo, zhi) = (along.lo, along.hi);
            let hh = self.ring.pieces[0].hh;
            let (x, y) = ((zlo - hh.hi) / b, (zhi - hh.lo) / b);
            span = Interval::new(span.lo.max(x.min(y)), span.hi.min(x.max(y)));
        }
        let (u_lo, u_hi) = (span.lo - 2.0 * du, span.hi + 2.0 * du);
        // a budget, as the tracer's march has: a sweep it would not finish is declined
        let samples = (u_hi - u_lo) / du;
        if !(samples.is_finite() && du > 0.0) || samples > 1e5 {
            return None;
        }
        if u_lo >= u_hi {
            return Some(SurfaceSurface::Empty);
        }
        // the section point (s along it) at angle u, and its derivatives in u and s
        let point = |u: f64, s: f64| {
            let (e, ep, _) = self.m.axes(u);
            let r = p.x + d.x * s + a * u;
            (
                f.origin + e * r + z * (p.y + d.y * s + b * u),
                ep * r + e * a + z * b,
                e * d.x + z * d.y,
            )
        };
        let roots = |u: f64| -> Option<Vec<f64>> {
            let (e, _, _) = self.m.axes(u);
            let o = f.origin + e * (p.x + a * u) + z * (p.y + b * u);
            let line = Line::new(o, (e * d.x + z * d.y).normalize()?);
            match line_surface(&line, other) {
                LineSurface::OnSurface => None,
                LineSurface::Hits(h) => Some(
                    h.into_iter()
                        .filter(|h| h.contact == Contact::Crossing)
                        .map(|h| h.t / len)
                        // on the section, in its own half plane
                        .filter(|&s| (0.0..=1.0).contains(&s) && p.x + d.x * s + a * u > 0.0)
                        .collect(),
                ),
            }
        };
        let mut tracks: Vec<Track> = Vec::new();
        // the track of each root at the last angle
        let mut live: Vec<usize> = Vec::new();
        let mut last: Vec<f64> = roots(u_lo)?;
        for &r in &last {
            live.push(tracks.len());
            tracks.push(Track {
                pts: vec![(u_lo, r)],
                ends: [None, None],
            });
        }
        // a root moves on by at most a few steps between samples; where it would jump, or more
        // than a pair of roots changes, the step is halved
        let jump = 4.0 * step / len;
        let follows = |more: &[f64], fewer: &[f64], j: usize, k: usize| {
            (0..more.len())
                .filter(|&i| i < j || i >= j + k)
                .zip(fewer)
                .all(|(i, f)| (more[i] - f).abs() <= jump)
        };
        let (mut u0, mut pending) = (u_lo, vec![u_hi]);
        let mut budget = 4 * samples as usize + 64;
        while let Some(&target) = pending.last() {
            budget = budget.checked_sub(1)?;
            let u = (u0 + du).min(target);
            let now = roots(u)?;
            let diff = now.len().abs_diff(last.len());
            let ok = match now.len().cmp(&last.len()) {
                _ if diff > 2 => false,
                std::cmp::Ordering::Equal => follows(&now, &last, 0, 0),
                std::cmp::Ordering::Less => gone(&last, &now).is_some_and(|j| follows(&last, &now, j, diff)),
                std::cmp::Ordering::Greater => {
                    gone(&now, &last).is_some_and(|j| follows(&now, &last, j, diff))
                }
            };
            if !ok && u - u0 > du * 1e-6 {
                pending.push(0.5 * (u0 + u));
                continue;
            }
            if u >= target {
                pending.pop();
            }
            if now.len() < last.len() {
                // roots that left through a section end (one), or met each other (two, adjacent)
                let j = gone(&last, &now)?;
                if diff == 2 {
                    let (x, y) = (live[j], live[j + 1]);
                    tracks[x].ends[1] = Some((y, 1));
                    tracks[y].ends[1] = Some((x, 1));
                }
                live.drain(j..j + diff);
            } else if now.len() > last.len() {
                let j = gone(&now, &last)?;
                let t = tracks.len();
                for i in 0..diff {
                    tracks.push(Track {
                        pts: Vec::new(),
                        ends: [None, None],
                    });
                    live.insert(j + i, t + i);
                }
                if diff == 2 {
                    tracks[t].ends[0] = Some((t + 1, 0));
                    tracks[t + 1].ends[0] = Some((t, 0));
                }
            }
            for (&t, &r) in live.iter().zip(&now) {
                tracks[t].pts.push((u, r));
            }
            (u0, last) = (u, now);
        }
        // tracks into chains, joined at turns
        let mut used = vec![false; tracks.len()];
        let mut curves = Vec::new();
        let me = self.clone().surface();
        for t in 0..tracks.len() {
            if used[t] {
                continue;
            }
            // back from t to an open end, or round to t (a loop)
            let (mut cur, mut closed) = ((t, 1), false);
            for _ in 0..=tracks.len() {
                match tracks[cur.0].ends[1 - cur.1] {
                    None => break,
                    Some((o, _)) if o == t => {
                        closed = true;
                        break;
                    }
                    Some(next) => cur = next,
                }
            }
            let (mut c, mut en) = if closed { (t, 0) } else { (cur.0, 1 - cur.1) };
            let mut pts: Vec<(f64, f64)> = Vec::new();
            while !used[c] {
                used[c] = true;
                if en == 0 {
                    pts.extend(&tracks[c].pts);
                } else {
                    pts.extend(tracks[c].pts.iter().rev());
                }
                match tracks[c].ends[1 - en] {
                    Some((o, oe)) => (c, en) = (o, oe),
                    None => break,
                }
            }
            // at the tracer's spacing: where the root runs fast along the section, or across a
            // turn, points between are pulled onto the other surface from the screw's parameters
            let onto = |mut u: f64, mut s: f64| {
                for _ in 0..12 {
                    let (q, pu, ps) = point(u, s);
                    let (g, n) = other.implicit_gradient(q);
                    let (ju, js) = (n.dot(pu), n.dot(ps));
                    let jj = ju * ju + js * js;
                    if g.abs() <= 1e-3 * tol::LINEAR || jj == 0.0 {
                        break;
                    }
                    (u, s) = (u - g * ju / jj, s - g * js / jj);
                }
                point(u, s).0
            };
            let mut chain: Vec<Point3> = Vec::with_capacity(pts.len());
            let m = pts.len();
            for i in 0..m {
                let (u0, s0) = pts[i];
                chain.push(point(u0, s0).0);
                if i + 1 == m && !closed {
                    break;
                }
                let (u1, s1) = pts[(i + 1) % m];
                let k = (point(u0, s0).0.distance(point(u1, s1).0) / step).ceil() as usize;
                for j in 1..k.min(64) {
                    let w = j as f64 / k as f64;
                    chain.push(onto(u0 + (u1 - u0) * w, s0 + (s1 - s0) * w));
                }
            }
            if chain.len() < 2 {
                continue;
            }
            // past the section's ends the surface is the track carrying it on, not the section
            // line: open ends go on two steps along both surfaces
            if !closed {
                let me = &me;
                let on = |from: Point3, back: Point3| {
                    let dir = crate::intcurve::direction(me, other, from)?;
                    let dir = if dir.dot(from - back) < 0.0 { -dir } else { dir };
                    crate::intcurve::refine(me, other, from + dir * step)
                };
                for _ in 0..2 {
                    let k = chain.len();
                    if let Some(q) = on(chain[k - 1], chain[k - 2]) {
                        chain.push(q);
                    }
                    if let Some(q) = on(chain[0], chain[1]) {
                        chain.insert(0, q);
                    }
                }
            }
            let pts = chain;
            if pts.len() < 2
                || !region
                    .grow(step)
                    .overlaps(&Box3::from_points(pts.iter().copied()))
            {
                continue;
            }
            let curve = crate::intcurve::finish(&me, other, pts, closed, step);
            curves.push((
                crate::Curve::Intersection(std::sync::Arc::new(curve)),
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
    use crate::surface::{Cylinder, SurfaceGeom};
    use crate::testing::{rand_frame, rand_unit};
    use forge_math::rng::Rng;
    use forge_math::{Interval, Point2, Point3};

    /// Every curve met with an oblique cylinder lies on both surfaces, and some are found.
    #[test]
    fn curves_lie_on_both_surfaces() {
        let mut r = Rng::new(511);
        let mut checked = 0;
        for _ in 0..40 {
            let f = rand_frame(&mut r);
            let c = [
                Point2::new(2.0, 0.0),
                Point2::new(3.0, 0.1),
                Point2::new(3.0, 0.6),
                Point2::new(2.0, 0.7),
            ];
            let loop4: Vec<Section> = (0..4)
                .map(|k| Section::Line {
                    a: c[k],
                    b: c[(k + 1) % 4],
                })
                .collect();
            let Some(faces) = Screw::loop_faces(
                f,
                &loop4,
                0.0,
                1.0 / forge_math::dmath::TAU,
                Interval::new(0.0, 12.0),
            ) else {
                continue;
            };
            let o = f.origin + f.z() * r.range(0.0, 2.0) + f.x() * r.range(-1.0, 1.0);
            let cyl = Surface::Cylinder(Cylinder {
                frame: forge_math::Frame::from_z(o, rand_unit(&mut r)),
                radius: r.range(1.0, 4.0),
            });
            let region = Box3::from_points([o, o]).grow(8.0);
            for face in faces {
                let s = face.surface();
                let Some(SurfaceSurface::Curves(cs)) = meet_general(&s, &cyl, region) else {
                    continue;
                };
                for (curve, _) in cs {
                    let d = curve.domain();
                    for i in 0..=20 {
                        let p = curve.eval(d.lo + (d.hi - d.lo) * i as f64 / 20.0);
                        assert!(
                            cyl.distance(p) < 1e-7 && s.distance(p) < 1e-7,
                            "{} {}",
                            cyl.distance(p),
                            s.distance(p)
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 200, "{checked}");
    }

    /// A long fine thread against a tiny region sweeps only the turns reaching it; a sweep the
    /// region cannot narrow past the budget (a long flat spiral) is declined, not run.
    #[test]
    fn long_sweeps_are_bounded() {
        let f = forge_math::Frame::WORLD;
        let tri = [
            Point2::new(2.0, 0.0),
            Point2::new(2.5, 0.01),
            Point2::new(2.0, 0.02),
        ];
        let sections: Vec<Section> = (0..3)
            .map(|k| Section::Line {
                a: tri[k],
                b: tri[(k + 1) % 3],
            })
            .collect();
        let at = Point3::new(2.2, 0.0, 500.0);
        let cyl = Surface::Cylinder(Cylinder {
            frame: forge_math::Frame::from_z(at, rand_unit(&mut Rng::new(3))),
            radius: 0.1,
        });
        let tiny = Box3::from_points([at, at]).grow(0.05);
        let t = std::time::Instant::now();
        let thread = Screw::loop_faces(
            f,
            &sections,
            0.0,
            0.025 / forge_math::dmath::TAU,
            Interval::new(0.0, 4e6),
        );
        for face in thread.expect("faces") {
            assert!(meet_general(&face.surface(), &cyl, tiny).is_some());
        }
        let spiral = Screw::loop_faces(f, &sections, 1e-6, 0.0, Interval::new(0.0, 4e6));
        for face in spiral.expect("faces") {
            assert!(meet_general(&face.surface(), &cyl, tiny).is_none());
        }
        assert!(t.elapsed().as_secs_f64() < 5.0);
    }
}
