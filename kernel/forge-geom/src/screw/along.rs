//! Where a curve meets a surface about its axis, in closed form (research/thread-math). In the
//! half plane of a screw surface's turn k a helix, a coaxial circle or a line in an axial plane is
//! a point moving linearly with its parameter, and the surface shows a fixed section: a line or
//! an arc. So each turn is one low-degree solve, and only the turns that can reach the section are
//! looked at. A plane square to the axis or a coaxial cylinder or cone is one such section, its
//! motion none. A meeting clear of everything decides the answer here; anything within a few
//! tolerances of a section's end, a turn's end, the range's end, or another meeting, or that
//! grazes, is searched for in just that stretch, so a touch stays what the search calls a touch.

use super::{Fence, Helix, Screw};
use crate::curve::Curve;
use crate::intersect::{Contact, CurveHit, CurveSurface, speed_bound, subdivide_stretches};
use crate::surface::Surface;
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec2, dmath, tol};

/// How far from a meeting, in section distance per tolerance, it is no longer clear of anything:
/// past the search's reach of 1.5 tolerances from a leaf.
const CLEAR: f64 = 4.0;

/// A curve this much further than a tolerance from a section (as a fraction of the band) touches
/// nothing: the search keeps nothing past 1.5 tolerances.
const MISS: f64 = 1.6 / CLEAR;

/// More turns than this and the search is no slower.
const MAX_TURNS: f64 = 1e5;

/// A curve in a surface's axis coordinates: distance from the axis r0 + r1 t, angle u0 + w t and
/// height h0 + h1 t.
#[derive(Clone, Copy)]
struct Moving {
    r0: f64,
    r1: f64,
    u0: f64,
    w: f64,
    h0: f64,
    h1: f64,
}

fn coaxial(g: &Frame, f: &Frame) -> Option<f64> {
    let z = f.z();
    let off = g.origin - f.origin;
    let s = z.dot(g.z().get());
    (z.cross(g.z().get()).length() <= 1e-12 && (off - z * z.dot(off)).length() <= 1e-9).then_some(s.signum())
}

fn moving(c: &Curve, range: Interval, f: &Frame) -> Option<Moving> {
    let carried = |g: &Frame, r0: f64, r1: f64, h0: f64, h1: f64| {
        let s = coaxial(g, f)?;
        Some(Moving {
            r0,
            r1,
            u0: dmath::atan2(f.y().dot(g.x().get()), f.x().dot(g.x().get())),
            w: s,
            h0: s * h0 + f.z().dot(g.origin - f.origin),
            h1: s * h1,
        })
    };
    match c {
        Curve::Circle(k) => carried(&k.frame, k.radius, 0.0, 0.0, 0.0),
        Curve::Procedural(p) => {
            let h = p.as_any()?.downcast_ref::<Helix>()?;
            carried(&h.m.frame, h.p.x, h.m.a, h.p.y, h.m.b)
        }
        Curve::Line(l) => {
            let (o, d) = (f.point_to_local(l.origin), f.vec_to_local(l.dir.get()));
            // its half plane, by its middle, which it must keep to
            let (mx, my) = (o.x + d.x * range.mid(), o.y + d.y * range.mid());
            let r = (mx * mx + my * my).sqrt();
            if r <= tol::LINEAR {
                return None;
            }
            let (ex, ey) = (mx / r, my / r);
            if (o.y * ex - o.x * ey).abs() > 1e-10 || (d.y * ex - d.x * ey).abs() > tol::ANGULAR {
                return None;
            }
            Some(Moving {
                r0: o.x * ex + o.y * ey,
                r1: d.x * ex + d.y * ey,
                u0: dmath::atan2(ey, ex),
                w: 0.0,
                h0: o.z,
                h1: d.z,
            })
        }
        _ => None,
    }
}

/// What the solves found: meetings, each with how far either side of it counts as near it.
#[derive(Default)]
struct Found {
    /// (t, half width, a clear crossing rather than a stretch to search)
    at: Vec<(f64, f64, bool)>,
    /// A stretch too long to say anything about in closed form: the whole search it is.
    whole: bool,
    /// The curve's speed bound, and whether it runs along a section further than the search's
    /// 20000 leaves.
    speed: f64,
    along: bool,
}

/// One turn's motion in the section plane: q0 + v t, while the turn lasts.
#[derive(Clone, Copy)]
struct Turn {
    q0: Point2,
    v: Vec2,
    x0: f64,
    x1: f64,
}

/// A meeting of s(t) = a + b t with zero, over [x0, x1], all of whose ends count as near.
fn linear(a: f64, b: f64, x0: f64, x1: f64, band: f64, out: &mut Found) {
    let (s0, s1) = (a + b * x0, a + b * x1);
    if s0.abs() > MISS * band && s1.abs() > MISS * band && (s0 < 0.0) == (s1 < 0.0) {
        return;
    }
    if b.abs() * (x1 - x0) <= 4.0 * band {
        // no clear crossing: the curve keeps near the section over all of this
        let far = (x1 - x0) * out.speed;
        if far > 0.1 {
            if s0.abs().max(s1.abs()) <= 1.4 * tol::LINEAR && far > 0.4 {
                out.along = true;
            } else {
                out.whole = true;
            }
            return;
        }
        out.at.push((0.5 * (x0 + x1), 0.5 * (x1 - x0), false));
        return;
    }
    let (t, w) = (-a / b, band / b.abs());
    let clear = t - x0 > w && x1 - t > w;
    out.at.push((t, w, clear));
}

fn line_piece(tu: Turn, pa: Point2, pb: Point2, band: f64, out: &mut Found) {
    let d = pb - pa;
    let l = d.length();
    let (e, n) = (d / l, Vec2::new(d.y, -d.x) / l);
    let rel = tu.q0 - pa;
    // the stretch where the point is beside the section, its ends near the next piece
    let (along0, along1) = (e.dot(rel), e.dot(tu.v));
    let (mut x0, mut x1) = (tu.x0, tu.x1);
    if along1 != 0.0 {
        let (y0, y1) = ((-band - along0) / along1, (l + band - along0) / along1);
        (x0, x1) = (x0.max(y0.min(y1)), x1.min(y0.max(y1)));
    } else if along0 < -band || along0 > l + band {
        return;
    }
    if x0 < x1 {
        linear(n.dot(rel), n.dot(tu.v), x0, x1, band, out);
    }
}

fn arc_piece(tu: Turn, p: &super::Piece, band: f64, out: &mut Found) {
    let super::Section::Arc { c, r, sweep, .. } = p.s else {
        return;
    };
    let rel = tu.q0 - c;
    let vv = tu.v.dot(tu.v);
    if vv == 0.0 {
        let l = rel.length();
        if (l - r).abs() <= band && p.spans(rel, sweep) {
            out.whole = true;
        }
        return;
    }
    let speed = vv.sqrt();
    let t0 = -rel.dot(tu.v) / vv;
    let h = rel.cross(tu.v / speed).abs();
    if h > r + MISS * band {
        return;
    }
    if h >= r - band {
        // grazing: all of the stretch within the band of the circle
        let w = (2.0 * (2.0 * r + band) * band).sqrt() / speed;
        if t0 + w > tu.x0 && t0 - w < tu.x1 {
            out.at.push((t0, w, false));
        }
        return;
    }
    let half = (r * r - h * h).sqrt() / speed;
    let cos = half * speed / r;
    let w = band / (speed * cos);
    for t in [t0 - half, t0 + half] {
        if t < tu.x0 - w || t > tu.x1 + w {
            continue;
        }
        let at = tu.q0 + tu.v * t;
        let near_end = at.distance(p.p0) <= 2.0 * band || at.distance(p.p1) <= 2.0 * band;
        let inside = t - tu.x0 > w && tu.x1 - t > w;
        if near_end {
            out.at.push((t, w, false));
        } else if p.spans(at - c, sweep) {
            out.at.push((t, w, inside));
        }
    }
}

/// The turns k where base - step k can lie in `band`; None for no step, or too many turns.
fn turns_between(base: Interval, step: f64, band: Interval) -> Option<(i64, i64)> {
    if step == 0.0 {
        return None;
    }
    let (x, y) = ((base.lo - band.hi) / step, (base.hi - band.lo) / step);
    let (lo, hi) = (x.min(y), x.max(y));
    (hi - lo < MAX_TURNS).then_some((lo.ceil() as i64, hi.floor() as i64))
}

/// Each turn of screw face `t` that `m` can reach in `range` within band `w`, with its stretch.
fn each_turn(
    m: Moving,
    range: Interval,
    t: &Screw,
    w: f64,
    out: &mut Found,
    mut f: impl FnMut(Turn, &mut Found),
) {
    let (a, b, ue) = (t.m.a, t.m.b, t.ue);
    let (rr, hh) = (t.ring.rr.grow(w), t.ring.hh.grow(w));
    let at = |g: f64, g1: f64| {
        let (x, y) = (g + g1 * range.lo, g + g1 * range.hi);
        Interval::new(x.min(y), x.max(y))
    };
    let v = Vec2::new(m.r1 - a * m.w, m.h1 - b * m.w);
    let (r_of, h_of) = (at(m.r0 - a * m.u0, v.x), at(m.h0 - b * m.u0, v.y));
    let tau = dmath::TAU;
    let (ks0, ks1) = (turns_between(r_of, tau * a, rr), turns_between(h_of, tau * b, hh));
    let (lo, hi) = match (ks0, ks1) {
        (Some(p), Some(q)) => (p.0.max(q.0), p.1.min(q.1)),
        (Some(p), None) | (None, Some(p)) => (p.0, p.1),
        (None, None) => {
            // no motion: the one turn, all the way round. Too many turns is the search's
            if a != 0.0 || b != 0.0 {
                out.whole = true;
                return;
            }
            f(
                Turn {
                    q0: Point2::new(m.r0, m.h0),
                    v,
                    x0: range.lo,
                    x1: range.hi,
                },
                out,
            );
            return;
        }
    };
    if (a != 0.0 && ks0.is_none()) || (b != 0.0 && ks1.is_none()) {
        out.whole = true;
        return;
    }
    for k in lo..=hi {
        let shift = m.u0 + tau * k as f64;
        let (mut x0, mut x1) = (range.lo, range.hi);
        if m.w != 0.0 {
            let (y0, y1) = ((ue.lo - shift) / m.w, (ue.hi - shift) / m.w);
            (x0, x1) = (x0.max(y0.min(y1)), x1.min(y0.max(y1)));
        } else if !ue.contains(shift) {
            continue;
        }
        if x0 >= x1 {
            continue;
        }
        let q0 = Point2::new(m.r0 - a * shift, m.h0 - b * shift);
        f(Turn { q0, v, x0, x1 }, out);
    }
}

/// `c` against a surface about the same axis: None if the pair is not one of these. `near` as in
/// `curve_surface_in`.
pub(crate) fn curve_along(
    c: &Curve,
    range: Interval,
    s: &Surface,
    near: &dyn Fn() -> Option<Box3>,
) -> Option<CurveSurface> {
    if let (Curve::Line(l), Surface::Procedural(p)) = (c, s) {
        let any = p.as_any()?;
        let f = match (any.downcast_ref::<Fence>(), any.downcast_ref::<Screw>()) {
            (Some(x), _) => &x.m.frame,
            (_, Some(x)) => &x.m.frame,
            _ => return None,
        };
        // through the axis it is two half lines, one in each half plane
        if let Some(t0) = through_axis(l, f, range) {
            let side = |r: Interval| {
                if r.lo >= r.hi {
                    return Some(Vec::new());
                }
                match solve(c, r, s, near)? {
                    CurveSurface::Hits(h) => Some(h),
                    _ => None,
                }
            };
            let mut hits = side(Interval::new(range.lo, t0.lo))?;
            hits.extend(side(Interval::new(t0.hi, range.hi))?);
            return Some(CurveSurface::Hits(hits));
        }
    }
    solve(c, range, s, near)
}

/// The stretch of the line `l` so near `f`'s axis that no half plane holds it, if it runs across.
fn through_axis(l: &crate::curve::Line, f: &Frame, range: Interval) -> Option<Interval> {
    let (o, d) = (f.point_to_local(l.origin), f.vec_to_local(l.dir.get()));
    let dd = d.x * d.x + d.y * d.y;
    if dd <= 1e-12 {
        return None;
    }
    let t0 = -(o.x * d.x + o.y * d.y) / dd;
    let (px, py) = (o.x + d.x * t0, o.y + d.y * t0);
    if !range.contains(t0) || px * px + py * py > 1e-20 {
        return None;
    }
    let w = 2.0 * tol::LINEAR / dd.sqrt();
    Some(Interval::new(t0 - w, t0 + w))
}

fn solve(c: &Curve, range: Interval, s: &Surface, near: &dyn Fn() -> Option<Box3>) -> Option<CurveSurface> {
    let mut out = Found {
        speed: speed_bound(c, range),
        ..Found::default()
    };
    // the curve's nearest approach to the axis sets how far the slant scales a section distance
    let of = |m: &Moving| (m.r0 + m.r1 * range.lo).min(m.r0 + m.r1 * range.hi);
    let band =
        |rmin: f64, a: f64, b: f64| CLEAR * tol::LINEAR * (1.0 + (a * a + b * b) / (rmin * rmin)).sqrt();
    match s {
        Surface::Plane(p) => {
            let Some(m) = moving(c, range, &p.frame) else {
                return helix_plane(c, range, p, near);
            };
            if of(&m) <= tol::LINEAR {
                return None;
            }
            linear(m.h0, m.h1, range.lo, range.hi, CLEAR * tol::LINEAR, &mut out);
        }
        Surface::Cylinder(y) => {
            let m = moving(c, range, &y.frame)?;
            if of(&m) <= tol::LINEAR {
                return None;
            }
            linear(
                m.r0 - y.radius,
                m.r1,
                range.lo,
                range.hi,
                CLEAR * tol::LINEAR,
                &mut out,
            );
        }
        Surface::Cone(k) => {
            let m = moving(c, range, &k.frame)?;
            if of(&m) <= tol::LINEAR {
                return None;
            }
            let (ca, sa) = (k.cos_a(), k.sin_a());
            let (a, b) = ((m.r0 - k.radius) * ca - m.h0 * sa, m.r1 * ca - m.h1 * sa);
            linear(a, b, range.lo, range.hi, CLEAR * tol::LINEAR, &mut out);
        }
        Surface::Procedural(pr) => {
            let any = pr.as_any()?;
            if let Some(f) = any.downcast_ref::<Fence>() {
                let m = moving(c, range, &f.m.frame)?;
                let rmin = of(&m);
                if rmin <= tol::LINEAR {
                    return None;
                }
                let (a, b) = (f.m.a, f.m.b);
                // (nearer the axis than its faces reach, what it meets is on no face)
                let w = band(rmin.max(0.25 * f.p.x).max(f.p.x - f.half), a, b);
                let n = Vec2::new(f.d.y, -f.d.x);
                // the section's line, the turns that reach it
                let v = Vec2::new(m.r1 - a * m.w, m.h1 - b * m.w);
                let s0 = (Point2::new(m.r0 - a * m.u0, m.h0 - b * m.u0) - f.p).dot(n);
                let (x, y) = (s0 + n.dot(v) * range.lo, s0 + n.dot(v) * range.hi);
                let reach = Interval::new(x.min(y), x.max(y));
                let step = dmath::TAU * (a * n.x + b * n.y);
                if step == 0.0 {
                    // every turn shows the line in one place
                    linear(s0, n.dot(v), range.lo, range.hi, w, &mut out);
                } else {
                    let (lo, hi) = turns_between(reach, step, Interval::new(-w, w))?;
                    for k in lo..=hi {
                        let shift = m.u0 + dmath::TAU * k as f64;
                        let (mut x0, mut x1) = (range.lo, range.hi);
                        if m.w != 0.0 {
                            let (y0, y1) = ((f.ue.lo - shift) / m.w, (f.ue.hi - shift) / m.w);
                            (x0, x1) = (x0.max(y0.min(y1)), x1.min(y0.max(y1)));
                        } else if !f.ue.contains(shift) {
                            continue;
                        }
                        if x0 < x1 {
                            linear(s0 - step * k as f64, n.dot(v), x0, x1, w, &mut out);
                        }
                    }
                }
            } else {
                let t = any.downcast_ref::<Screw>()?;
                let m = moving(c, range, &t.m.frame)?;
                let rmin = of(&m);
                if rmin <= tol::LINEAR {
                    return None;
                }
                let w = band(rmin.max(t.floor), t.m.a, t.m.b);
                each_turn(m, range, t, w, &mut out, |tu, out| {
                    for p in &t.ring.pieces {
                        match p.s {
                            super::Section::Line { a: pa, b: pb } => line_piece(tu, pa, pb, w, out),
                            super::Section::Arc { .. } => arc_piece(tu, p, w, out),
                        }
                    }
                });
            }
        }
        _ => return None,
    }
    if out.along {
        let probes = 9;
        let on = (0..probes).all(|i| {
            let t = range.lo + range.width() * i as f64 / (probes - 1) as f64;
            s.implicit(crate::curve::CurveGeom::eval(c, t)).abs() <= tol::LINEAR
        });
        return Some(if on {
            CurveSurface::OnSurface
        } else {
            CurveSurface::Overlap
        });
    }
    if out.whole {
        return None;
    }
    // meetings that run together are one stretch for the search
    out.at.retain(|x| x.0 + x.1 > range.lo && x.0 - x.1 < range.hi);
    out.at.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut hits: Vec<CurveHit> = Vec::with_capacity(out.at.len());
    let mut stretches: Vec<Interval> = Vec::new();
    let mut i = 0;
    while i < out.at.len() {
        let (t, w, mut clear) = out.at[i];
        let (lo, mut hi) = (t - 2.0 * w, t + 2.0 * w);
        let mut j = i + 1;
        while j < out.at.len() && out.at[j].0 - 2.0 * out.at[j].1 <= hi {
            hi = hi.max(out.at[j].0 + 2.0 * out.at[j].1);
            clear = false;
            j += 1;
        }
        if clear {
            let p: Point3 = crate::curve::CurveGeom::eval(c, t);
            hits.push(CurveHit {
                t,
                point: p,
                contact: Contact::Crossing,
            });
        } else {
            stretches.push(Interval::new(lo.max(range.lo), hi.min(range.hi)));
        }
        i = j;
    }
    // near the surface most of the way, it may lie on it: that is the search's to say
    if stretches.iter().map(|x| x.width()).sum::<f64>() >= 0.5 * range.width() {
        return None;
    }
    if !stretches.is_empty() {
        let found = subdivide_stretches(c, range, &stretches, s, near());
        match found {
            CurveSurface::Hits(h) => hits.extend(h),
            _ => return None,
        }
        hits.sort_by(|a, b| a.t.total_cmp(&b.t));
    }
    Some(CurveSurface::Hits(hits))
}

/// A helix round a cylinder against any plane: f(t) = A cos(t - th) + G t + D, which turns at
/// most twice a turn where sin(t - th) = G / A; only the turns where G t + D is within A of zero
/// can hold a root, and each root is alone between two turning points.
fn helix_plane(
    c: &Curve,
    range: Interval,
    p: &crate::surface::Plane,
    near: &dyn Fn() -> Option<Box3>,
) -> Option<CurveSurface> {
    let Curve::Procedural(pc) = c else {
        return None;
    };
    let h = pc.as_any()?.downcast_ref::<Helix>()?;
    if h.m.a != 0.0 {
        return None;
    }
    let f = &h.m.frame;
    let n = p.frame.z();
    let (nx, ny, nz) = (n.dot(f.x().get()), n.dot(f.y().get()), n.dot(f.z().get()));
    let (m, th) = ((nx * nx + ny * ny).sqrt(), dmath::atan2(ny, nx));
    let (a, g) = (h.p.x * m, nz * h.m.b);
    let d = nz * h.p.y + n.dot(f.origin - p.frame.origin);
    let band = CLEAR * tol::LINEAR;
    let tau = dmath::TAU;
    let val = |t: f64| {
        let (sn, cs) = dmath::sin_cos(t - th);
        (a * cs + g * t + d, -a * sn + g)
    };
    // the stretch that can hold a root or a near miss
    let (mut lo, mut hi) = (range.lo, range.hi);
    if g != 0.0 {
        let (x, y) = ((-d - a - band) / g, (-d + a + band) / g);
        (lo, hi) = (lo.max(x.min(y)), hi.min(x.max(y)));
    } else if d.abs() > a + band {
        return Some(CurveSurface::Hits(vec![]));
    }
    if lo >= hi {
        return Some(CurveSurface::Hits(vec![]));
    }
    // the turning points in it, or none on a steep helix
    let mut knots = vec![lo];
    if g.abs() < a {
        let q = dmath::asin(g / a);
        for base in [th + q, th + dmath::PI - q] {
            let k0 = ((lo - base) / tau).ceil() as i64;
            let k1 = ((hi - base) / tau).floor() as i64;
            if (k1 - k0) as f64 > MAX_TURNS {
                return None;
            }
            knots.extend((k0..=k1).map(|k| base + tau * k as f64));
        }
        knots.sort_by(f64::total_cmp);
    }
    knots.push(hi);
    // how far from t f stays within the band: by its slope there, or by its bend at a turning point
    let reach = |t: f64| {
        let bend = if a > 0.0 {
            (8.0 * band / a).sqrt()
        } else {
            f64::INFINITY
        };
        bend.min(2.0 * band / val(t).1.abs())
    };
    let w = 2.0 * band / (h.p.x * h.p.x + h.m.b * h.m.b).sqrt().max(1e-300);
    let (mut hits, mut stretches): (Vec<CurveHit>, Vec<Interval>) = (Vec::new(), Vec::new());
    let mut stretch = |t: f64, half: f64| {
        let i = Interval::new((t - half).max(range.lo), (t + half).min(range.hi));
        match stretches.last_mut() {
            Some(l) if l.hi >= i.lo => l.hi = l.hi.max(i.hi),
            _ => stretches.push(i),
        }
    };
    let mut prev = val(knots[0]).0;
    for ks in knots.windows(2) {
        let (x0, x1) = (ks[0], ks[1]);
        let next = val(x1).0;
        // a turning point or an end near the plane: a graze, or a root the search must judge
        if prev.abs() <= band {
            stretch(x0, reach(x0));
        }
        if next.abs() > band && prev.abs() > band && (prev < 0.0) != (next < 0.0) {
            let (mut u, mut v) = (x0, x1);
            let up = next > 0.0;
            let mut t = 0.5 * (x0 + x1);
            for _ in 0..64 {
                let (y, dy) = val(t);
                if (y > 0.0) == up {
                    v = t;
                } else {
                    u = t;
                }
                let nt = t - y / dy;
                let (nx, done) = if nt > u && nt < v {
                    (nt, (nt - t).abs() <= 1e-15 * t.abs().max(1.0))
                } else {
                    (0.5 * (u + v), v - u <= 1e-15 * t.abs().max(1.0))
                };
                t = nx;
                if done {
                    break;
                }
            }
            hits.push(CurveHit {
                t,
                point: crate::curve::CurveGeom::eval(c, t),
                contact: Contact::Crossing,
            });
        }
        prev = next;
    }
    if prev.abs() <= band {
        stretch(hi, reach(hi));
    }
    if !stretches.is_empty() {
        // grazing turn after turn is the search's, budget and all
        if stretches.len() > 2 || stretches.iter().map(|x| x.width()).sum::<f64>() >= 0.5 * range.width() {
            return None;
        }
        // a clear crossing beside a stretch is the stretch's
        hits.retain(|x| !stretches.iter().any(|i| x.t >= i.lo - w && x.t <= i.hi + w));
        match subdivide_stretches(c, range, &stretches, &Surface::Plane(*p), near()) {
            CurveSurface::Hits(h) => hits.extend(h),
            _ => return None,
        }
        hits.sort_by(|a, b| a.t.total_cmp(&b.t));
    }
    Some(CurveSurface::Hits(hits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::{Circle, CurveGeom, Line, ProcCurve};
    use crate::intersect::curve_surface_subdivide;
    use crate::screw::{Motion, Section};
    use crate::surface::{Cone, Cylinder, Plane};
    use crate::testing::rand_frame;
    use forge_math::rng::Rng;
    use std::sync::Arc;

    fn helix(frame: Frame, a: f64, b: f64, p: Point2, u: Interval) -> Curve {
        Curve::Procedural(ProcCurve(Arc::new(Helix {
            m: Motion { frame, a, b },
            p,
            u,
            fence: None,
        })))
    }

    /// The same answer as the search: as many hits, of the same kind, in the same places. Returns
    /// whether the closed form took it.
    fn agree(f: &Frame, c: &Curve, range: Interval, s: &Surface, what: &str) -> bool {
        let Some(new) = curve_along(c, range, s, &|| None) else {
            return false;
        };
        let old = curve_surface_subdivide(c, range, s, None);
        if std::env::var_os("ALONGDBG").is_some()
            && let (CurveSurface::Hits(x), CurveSurface::Hits(y)) = (&new, &old)
            && x.len() != y.len()
        {
            eprintln!("c {c:?}\ns {s:?}\nrange {range:?}");
        }
        match (&new, &old) {
            (CurveSurface::Hits(x), CurveSurface::Hits(y)) => {
                // right by the axis a turn's half plane jumps and the implicit with it, and no
                // face comes near there
                let off = |h: &&CurveHit| {
                    let l = f.point_to_local(h.point);
                    l.x * l.x + l.y * l.y > 1.0
                };
                let (x, y): (Vec<CurveHit>, Vec<CurveHit>) = (
                    x.iter().filter(off).cloned().collect(),
                    y.iter().filter(off).cloned().collect(),
                );
                let (x, y) = (&x, &y);
                // past its budget the search keeps only sign changes: it may have dropped a true
                // touch, but it finds nothing the closed form does not
                let touching = |h: &&CurveHit| h.contact == Contact::Touching;
                let graze = |h: &&CurveHit| s.implicit(h.point).abs() <= 2.0 * tol::LINEAR;
                assert!(
                    x.iter().filter(touching).all(|h| graze(&h)),
                    "{what}: {x:?}\n vs {y:?}"
                );
                let same = |p: &CurveHit, q: &CurveHit| {
                    p.contact == q.contact && (p.point.distance(q.point) < 2e-5 || (p.t - q.t).abs() < 1e-2)
                };
                let crossings = |v: &[CurveHit]| v.iter().filter(|h| h.contact == Contact::Crossing).count();
                assert_eq!(
                    crossings(x),
                    crossings(y),
                    "{what}: {x:?}\n vs {y:?}\n range {range:?}"
                );
                for q in y {
                    assert!(
                        x.iter().any(|p| same(p, q)),
                        "{what}: {x:?}\n vs {y:?}\n range {range:?}"
                    );
                }
            }
            // grazing a plane every turn, the search gives up on the count of its leaves; the
            // turning points say where the touches are
            (CurveSurface::Hits(_), CurveSurface::Overlap) => {}
            _ => assert_eq!(
                std::mem::discriminant(&new),
                std::mem::discriminant(&old),
                "{what}: {new:?}\n vs {old:?}"
            ),
        }
        true
    }

    /// A groove's faces against the curves a thread makes, near each other by chance and by
    /// construction: exactly on, within and just past a tolerance, ending on the surface.
    #[test]
    fn agrees_with_the_search() {
        let mut r = Rng::new(9177);
        let (mut asked, mut closed) = (0, 0);
        let pick = |r: &mut Rng, xs: &[f64]| xs[(r.range(0.0, xs.len() as f64) as usize).min(xs.len() - 1)];
        let eps = [0.0, 0.0, 1e-9, 0.5e-5, 1e-5, 1.2e-5, 2e-5, 4e-5, 8e-5, 1e-3];
        for _ in 0..500 {
            let f = rand_frame(&mut r);
            let up = r.range(0.0, 1.0) < 0.5;
            let g = if up {
                f.with_origin(f.origin + f.z() * r.range(-2.0, 2.0))
            } else {
                Frame::from_zx(f.origin, -f.z(), f.y().get()).expect("axes")
            };
            let b = r.range(0.2, 0.5) * if r.range(0.0, 1.0) < 0.5 { 1.0 } else { -1.0 };
            let a = pick(&mut r, &[0.0, 0.0, 0.0, 0.05, -0.03]);
            let (r0, wide, h) = (r.range(2.0, 6.0), r.range(0.2, 0.8), r.range(0.1, 0.25));
            let c = [
                Point2::new(r0, -h),
                Point2::new(r0 + wide, -2.0 * h),
                Point2::new(r0 + wide, 2.0 * h),
                Point2::new(r0, h),
            ];
            let sections: Vec<Section> = (0..4)
                .map(|k| Section::Line {
                    a: c[k],
                    b: c[(k + 1) % 4],
                })
                .collect();
            // the curves stay well within the surface's turns: where a turn ends the implicit
            // jumps, and the search's Lipschitz bound can lose a crossing beside the jump
            let u = Interval::new(-60.0, 60.0);
            let Some(faces) = Screw::loop_faces(f, &sections, a, b, u) else {
                continue;
            };
            let face = &faces[(r.range(0.0, 4.0) as usize).min(3)];
            let (rh, hh) = (
                r0 + pick(&mut r, &[0.0, 0.1, wide]) + pick(&mut r, &eps),
                r.range(-0.5, 0.5),
            );
            let rh_tangent = rh;
            let surfaces: Vec<Surface> = {
                let helix0 = face.helix(0.0);
                let Curve::Procedural(p) = &helix0 else {
                    unreachable!()
                };
                vec![
                    face.clone().surface(),
                    p.across().expect("a fence"),
                    Surface::Plane(Plane::new(f.with_origin(f.origin + f.z() * r.range(-1.0, 6.0)))),
                    Surface::Cylinder(Cylinder {
                        frame: g,
                        radius: r0 + pick(&mut r, &[0.0, 0.3, wide]) + pick(&mut r, &eps),
                    }),
                    Surface::Cone(Cone::new(g, r0 + r.range(-1.0, 1.0), r.range(0.1, 1.0)).expect("a cone")),
                    // any plane, near enough to meet the helices
                    Surface::Plane(Plane::new(rand_frame(&mut r).with_origin(
                        f.origin + f.x() * r.range(-5.0, 5.0) + f.z() * r.range(-1.0, 8.0),
                    ))),
                    // along the axis, tangent to the helices' cylinder or a tolerance from it
                    Surface::Plane(Plane::new(
                        Frame::from_zx(
                            f.origin + f.x() * (rh_tangent + pick(&mut r, &eps)),
                            f.x(),
                            f.y().get(),
                        )
                        .expect("axes"),
                    )),
                ]
            };
            // the curves: others' helices, the same motion or not; circles; axial lines
            let mut curves: Vec<(Curve, Interval)> = Vec::new();
            let span = Interval::new(r.range(-3.0, 3.0), r.range(4.0, 40.0));
            curves.push((helix(g, 0.0, b, Point2::new(rh, hh), span), span));
            curves.push((helix(g, a, b, Point2::new(rh, hh), span), span));
            curves.push((
                helix(
                    g,
                    r.range(-0.02, 0.02),
                    r.range(-0.3, 0.3),
                    Point2::new(rh + 2.0, hh),
                    span,
                ),
                span,
            ));
            let lift = g.with_origin(g.origin + g.z() * (r.range(-1.0, 8.0)));
            let turn = Interval::new(0.0, dmath::TAU);
            curves.push((
                Curve::Circle(Circle {
                    frame: lift,
                    radius: rh,
                }),
                turn,
            ));
            let (sn, cs) = dmath::sin_cos(r.range(0.0, 6.0));
            let e = f.x() * cs + f.y() * sn;
            let o = f.origin + e * rh + f.z() * r.range(-1.0, 1.0);
            let slant = pick(&mut r, &[0.0, 0.0, 0.3, -0.5]);
            let d = (f.z().get() + e * slant).normalize().expect("a direction");
            let reach = Interval::new(-r.range(0.0, 3.0), r.range(0.0, 10.0));
            curves.push((Curve::Line(Line { origin: o, dir: d }), reach));
            // through the axis
            let (o, d) = (
                f.origin + f.z() * r.range(-1.0, 8.0),
                e.normalize().expect("a direction"),
            );
            let cross = Interval::new(-r.range(1.0, 9.0), r.range(1.0, 9.0));
            curves.push((Curve::Line(Line { origin: o, dir: d }), cross));
            for (curve, range) in &curves {
                for s in &surfaces {
                    asked += 1;
                    closed += agree(&f, curve, *range, s, "any") as usize;
                    // ending on the surface, or a tolerance away
                    let t = range.lo + range.width() * r.range(0.1, 0.9);
                    let g = s.implicit(curve.eval(t));
                    if g.abs() < 0.5 {
                        let cut = Interval::new(range.lo, t + pick(&mut r, &eps) * 0.1);
                        closed += agree(&f, curve, cut, s, "cut") as usize;
                    }
                    if let CurveSurface::Hits(h) = curve_surface_subdivide(curve, *range, s, None)
                        && let Some(x) = h.first()
                        && x.t - range.lo > 1e-3
                    {
                        // a range ending exactly at, or within the band of, a crossing
                        let e = pick(&mut r, &eps);
                        let cut = Interval::new(range.lo, x.t + e);
                        closed += agree(&f, curve, cut, s, "at a hit") as usize;
                        let cut = Interval::new(x.t - e - 1e-9, range.hi);
                        closed += agree(&f, curve, cut, s, "from a hit") as usize;
                    }
                }
            }
        }
        assert!(asked > 5000 && closed > asked / 2, "{closed} of {asked} closed");
    }
}
