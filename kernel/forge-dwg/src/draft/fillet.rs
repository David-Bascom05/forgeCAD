//! FILLET and CHAMFER, as AutoCAD does them: between two lines, arcs or circles (or an open
//! polyline's end segment), keeping the ends the picks are on; radius 0 (or distances 0) trims or
//! extends both to their corner; parallel lines get a half circle; and a polyline's vertices
//! one at a time or all at once.

use super::curve::{Arc, Curve, Poly, Seg};
use super::geom::{Bbox, PI, Tol, V2, wrap, wrap_pi};
use super::intersect::{Ext, intersect_tol};
use forge_math::dmath;

/// A FILLET or CHAMFER: the two curves as they are now (a circle unchanged), and the arc or line
/// added between them (none at radius or distance 0).
#[derive(Clone, Debug, PartialEq)]
pub struct Corner {
    pub a: Curve,
    pub b: Curve,
    pub added: Option<Curve>,
}

/// A CHAMFER's size: distances from the corner along each curve, or a distance along the first
/// and the angle (radians) the chamfer makes with it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Chamfer {
    Distances(f64, f64),
    Angle(f64, f64),
}

/// The line or circle a fillet works on.
#[derive(Clone, Copy, Debug)]
enum Car {
    Lin { p: V2, u: V2 },
    Circ { c: V2, r: f64 },
}

impl Car {
    fn of(c: &Curve) -> Option<Car> {
        match c {
            Curve::Line { a, b } => Some(Car::Lin {
                p: *a,
                u: (*b - *a).unit()?,
            }),
            Curve::Ray { at, dir } | Curve::Xline { at, dir } => Some(Car::Lin { p: *at, u: *dir }),
            Curve::Arc(a) => Some(Car::Circ { c: a.c, r: a.r }),
            Curve::Circle { c, r } => Some(Car::Circ { c: *c, r: *r }),
            _ => None,
        }
    }
    /// The curves a fillet's centre lies on: this carrier moved `r` either way.
    fn offsets(&self, r: f64) -> Vec<Curve> {
        match *self {
            Car::Lin { p, u } => vec![
                Curve::Xline {
                    at: p + u.perp() * r,
                    dir: u,
                },
                Curve::Xline {
                    at: p - u.perp() * r,
                    dir: u,
                },
            ],
            Car::Circ { c, r: rc } => {
                let mut v = vec![Curve::Circle { c, r: rc + r }];
                if rc - r > 0.0 {
                    v.push(Curve::Circle { c, r: rc - r });
                }
                if r - rc > 0.0 {
                    v.push(Curve::Circle { c, r: r - rc });
                }
                v
            }
        }
    }
    fn foot(&self, q: V2) -> V2 {
        match *self {
            Car::Lin { p, u } => p + u * (q - p).dot(u),
            Car::Circ { c, r } => c + (q - c).unit_or_zero() * r,
        }
    }
    fn curve(&self) -> Curve {
        match *self {
            Car::Lin { p, u } => Curve::Xline { at: p, dir: u },
            Car::Circ { c, r } => Curve::Circle { c, r },
        }
    }
    /// Whether, leaving `t` (on the carrier) in direction `k`, the carrier reaches `pick` before
    /// coming back round (lines: pick ahead; circles: within half a turn).
    fn ahead(&self, t: V2, k: V2, pick: V2) -> bool {
        match *self {
            Car::Lin { u, .. } => {
                let s = if k.dot(u) >= 0.0 { 1.0 } else { -1.0 };
                (pick - t).dot(u) * s > 0.0
            }
            Car::Circ { c, .. } => {
                let s = if (t - c).cross(k) >= 0.0 { 1.0 } else { -1.0 };
                let travel = wrap(s * ((pick - c).angle() - (t - c).angle()));
                travel > 0.0 && travel < PI
            }
        }
    }
}

/// The curve cut back or run on to `x` (a point on it or its extension), keeping the side `pick`
/// is on. A circle stays whole.
pub fn cut_to(c: &Curve, x: V2, pick: V2) -> Option<Curve> {
    if matches!(c, Curve::Circle { .. }) {
        return Some(c.clone());
    }
    let (lo, hi) = c.domain();
    let (tx, _) = c.closest_unbounded(x);
    let tp = c.param_of(pick);
    let tol = Tol::of_box(&c.rough_bbox().union(Bbox::of([x]))).lin;
    let e = tol / c.deriv(tx).len().max(1e-300);
    if tx > lo + e && tx < hi - e {
        if tp < tx { c.sub(lo, tx) } else { c.sub(tx, hi) }
    } else if tx >= hi - e {
        c.sub(lo, tx)
    } else {
        c.sub(tx, hi)
    }
}

/// An open polyline's end segment under a pick: (the segment as a curve, its index, polyline).
fn end_piece(c: &Curve, pick: V2) -> Option<(Curve, Option<(usize, Poly)>)> {
    match c {
        Curve::Poly(p) => {
            let (t, _) = c.closest(pick);
            let n = p.nseg();
            if n == 0 {
                return None;
            }
            let k = (t.floor() as usize).min(n - 1);
            if p.closed || (k != 0 && k != n - 1) {
                return None;
            }
            Some((p.seg(k).curve(), Some((k, p.clone()))))
        }
        _ => Some((c.clone(), None)),
    }
}

/// Puts a changed end segment back: its free end moved, the joint kept.
fn put_back(piece: Curve, at: Option<(usize, Poly)>, tol: f64) -> Option<Curve> {
    let Some((k, p)) = at else { return Some(piece) };
    let old = p.seg(k);
    let seg = match piece {
        Curve::Line { a, b } => Seg::Line(a, b),
        Curve::Arc(a) => Seg::Arc(a),
        _ => return None,
    };
    let n = p.nseg();
    // the joint with the rest of the polyline must stay
    if n > 1 {
        let ok = if k == 0 {
            seg.end().dist(old.end()) <= tol
        } else {
            seg.start().dist(old.start()) <= tol
        };
        if !ok {
            return None;
        }
    }
    let mut segs = p.segs();
    segs[k] = seg;
    let mut q = Poly::from_segs(&segs, false);
    q.const_width = p.const_width;
    Some(Curve::Poly(q))
}

/// FILLET with radius `r` between `a` and `b`, keeping the parts the picks are on. Lines, rays,
/// arcs, circles (left whole) and an open polyline's end segments; parallel lines get a half
/// circle; `r = 0` makes a sharp corner. None when there is no such fillet (radius too large, no
/// corner).
pub fn fillet(a: &Curve, pick_a: V2, b: &Curve, pick_b: V2, r: f64) -> Option<Corner> {
    if let (Curve::Poly(p), Curve::Poly(q)) = (a, b)
        && p == q
    {
        // two segments of one polyline: the vertex between them
        let (ka, kb) = (
            a.param_of(pick_a).floor() as usize,
            b.param_of(pick_b).floor() as usize,
        );
        let n = p.nseg();
        let (ka, kb) = (ka.min(n - 1), kb.min(n - 1));
        let v = if ka + 1 == kb {
            kb
        } else if kb + 1 == ka {
            ka
        } else if p.closed && ((ka == n - 1 && kb == 0) || (kb == n - 1 && ka == 0)) {
            0
        } else {
            return None;
        };
        let out = Curve::Poly(fillet_poly_vertex(p, v, r)?);
        return Some(Corner {
            a: out.clone(),
            b: out,
            added: None,
        });
    }
    let (pa, at_a) = end_piece(a, pick_a)?;
    let (pb, at_b) = end_piece(b, pick_b)?;
    let tol = Tol::of_box(&pa.bbox().union(pb.bbox())).lin * 16.0;
    let (ca, cb) = (Car::of(&pa)?, Car::of(&pb)?);
    if r <= tol {
        let (na, nb) = sharp_corner(&pa, pick_a, &pb, pick_b, tol)?;
        return Some(Corner {
            a: put_back(na, at_a, tol)?,
            b: put_back(nb, at_b, tol)?,
            added: None,
        });
    }
    if let (Car::Lin { p: p1, u: u1 }, Car::Lin { p: p2, u: u2 }) = (ca, cb)
        && u1.cross(u2).abs() <= 1e-12
    {
        let (na, nb, arc) = parallel_fillet(&pa, pick_a, &pb, pick_b, p1, u1, p2, tol)?;
        return Some(Corner {
            a: put_back(na, at_a, tol)?,
            b: put_back(nb, at_b, tol)?,
            added: Some(arc),
        });
    }
    if let (Car::Lin { p: p1, u: u1 }, Car::Lin { p: p2, u: u2 }) = (ca, cb) {
        // two lines: the picks say which side of their corner each keeps (a pick may be where
        // the fillet will cut, as AutoCAD's may)
        let (s, _) = super::curve::line_line(p1, u1, p2, u2)?;
        let x = p1 + u1 * s;
        let ka = side_of_corner(&pa, x, u1, pick_a, tol)?;
        let kb = side_of_corner(&pb, x, u2, pick_b, tol)?;
        let th = dmath::acos_clamped(ka.dot(kb));
        if th <= 1e-9 || th >= PI - 1e-9 {
            return None;
        }
        // the centre where the lines' offsets toward the kept sides meet; the tangent points
        // its feet
        let na_ = if u1.perp().dot(kb) >= 0.0 {
            u1.perp()
        } else {
            -u1.perp()
        };
        let nb_ = if u2.perp().dot(ka) >= 0.0 {
            u2.perp()
        } else {
            -u2.perp()
        };
        let (t, _) = super::curve::line_line(p1 + na_ * r, u1, p2 + nb_ * r, u2)?;
        let c = p1 + na_ * r + u1 * t;
        let (ta, tb) = (ca.foot(c), cb.foot(c));
        let d = ta.dist(x);
        let na = keep_from(&pa, x, ta, ka, d, tol)?;
        let nb = keep_from(&pb, x, tb, kb, d, tol)?;
        let arc = Arc {
            c,
            r,
            start: (ta - c).angle(),
            sweep: wrap_pi((tb - c).angle() - (ta - c).angle()),
        };
        return Some(Corner {
            a: put_back(na, at_a, tol)?,
            b: put_back(nb, at_b, tol)?,
            added: Some(Curve::Arc(arc)),
        });
    }
    // centres: where the carriers' offsets meet
    let mut best: Option<(f64, Curve, Curve, Arc)> = None;
    for oa in ca.offsets(r) {
        for ob in cb.offsets(r) {
            for h in intersect_tol(&oa, &ob, Ext::None, tol).points {
                let c = h.p;
                let (ta, tb) = (ca.foot(c), cb.foot(c));
                if ta.dist(tb) <= tol {
                    continue;
                }
                let sweep = wrap_pi((tb - c).angle() - (ta - c).angle());
                let dir = if sweep < 0.0 { -1.0 } else { 1.0 };
                // leaving the fillet the curves run on away from it
                let da = (ta - c).perp() * dir;
                let db = (tb - c).perp() * dir;
                if !ca.ahead(ta, -da, pick_a) || !cb.ahead(tb, db, pick_b) {
                    continue;
                }
                let Some(na) = cut_to(&pa, ta, pick_a) else {
                    continue;
                };
                let Some(nb) = cut_to(&pb, tb, pick_b) else {
                    continue;
                };
                // the kept parts hold the picks
                if !holds(&pa, &na, pick_a, tol) || !holds(&pb, &nb, pick_b, tol) {
                    continue;
                }
                let score = ta.dist(pick_a) + tb.dist(pick_b);
                if best.as_ref().is_none_or(|x| score < x.0) {
                    let arc = Arc {
                        c,
                        r,
                        start: (ta - c).angle(),
                        sweep,
                    };
                    best = Some((score, na, nb, arc));
                }
            }
        }
    }
    let (_, na, nb, arc) = best?;
    Some(Corner {
        a: put_back(na, at_a, tol)?,
        b: put_back(nb, at_b, tol)?,
        added: Some(Curve::Arc(arc)),
    })
}

/// The way from a corner `x` along a line (direction `u`) that a pick keeps: toward the pick,
/// or, picked at the corner itself, toward the line's far end.
fn side_of_corner(line: &Curve, x: V2, u: V2, pick: V2, tol: f64) -> Option<V2> {
    let s = (pick - x).dot(u);
    if s.abs() > tol {
        return Some(if s > 0.0 { u } else { -u });
    }
    let far = [line.start(), line.end()]
        .into_iter()
        .max_by(|a, b| a.dist(x).total_cmp(&b.dist(x)))?;
    Some(if (far - x).dot(u) >= 0.0 { u } else { -u })
}

/// A line from `t` (`d` from the corner `x` along `k`) on to its end that way: cut back, or run on
/// when it stops short of `t`. None when nothing of it lies that way past `t`.
fn keep_from(line: &Curve, x: V2, t: V2, k: V2, d: f64, tol: f64) -> Option<Curve> {
    let far = [line.start(), line.end()]
        .into_iter()
        .max_by(|a, b| (*a - x).dot(k).total_cmp(&(*b - x).dot(k)))?;
    if (far - x).dot(k) <= d + tol {
        return None;
    }
    cut_to(line, t, far)
}

/// The cut curve still holds the pick (its foot on the original's carrier), and a line still
/// runs the way it did.
fn holds(orig: &Curve, new: &Curve, pick: V2, tol: f64) -> bool {
    let foot = orig.closest_unbounded(pick).1;
    let same_way = match (orig, new) {
        (Curve::Line { a, b }, Curve::Line { a: c, b: d }) => (*b - *a).dot(*d - *c) > 0.0,
        _ => true,
    };
    same_way && new.dist(foot) <= tol * 10.0
}

fn sharp_corner(a: &Curve, pick_a: V2, b: &Curve, pick_b: V2, tol: f64) -> Option<(Curve, Curve)> {
    let hits = intersect_tol(&Car::of(a)?.curve(), &Car::of(b)?.curve(), Ext::None, tol);
    let x = hits
        .points
        .iter()
        .map(|h| h.p)
        .min_by(|p, q| (p.dist(pick_a) + p.dist(pick_b)).total_cmp(&(q.dist(pick_a) + q.dist(pick_b))))?;
    Some((cut_to(a, x, pick_a)?, cut_to(b, x, pick_b)?))
}

#[allow(clippy::too_many_arguments)]
fn parallel_fillet(
    a: &Curve,
    pick_a: V2,
    b: &Curve,
    pick_b: V2,
    p1: V2,
    u: V2,
    p2: V2,
    tol: f64,
) -> Option<(Curve, Curve, Curve)> {
    let gap = u.cross(p2 - p1);
    if gap.abs() <= tol {
        return None;
    }
    // the first line's end nearer its pick
    let (s, e) = (a.start(), a.end());
    if !a.is_bounded() {
        return None;
    }
    let ta = if s.dist(pick_a) < e.dist(pick_a) { s } else { e };
    let other = if ta == s { e } else { s };
    let out = (ta - other).unit()?;
    let tb = ta + u.perp() * gap;
    let c = ta.mid(tb);
    let rr = gap.abs() / 2.0;
    let a0 = (ta - c).angle();
    // the half circle bulges outward, past the ends
    let sweep = if (V2::polar(a0 + PI / 2.0, 1.0)).dot(out) > 0.0 {
        PI
    } else {
        -PI
    };
    let nb = cut_to(b, tb, pick_b)?;
    Some((
        a.clone(),
        nb,
        Curve::Arc(Arc {
            c,
            r: rr,
            start: a0,
            sweep,
        }),
    ))
}

/// CHAMFER between two lines (or open polylines' end segments).
pub fn chamfer(a: &Curve, pick_a: V2, b: &Curve, pick_b: V2, how: Chamfer) -> Option<Corner> {
    if let (Curve::Poly(p), Curve::Poly(q)) = (a, b)
        && p == q
    {
        let (ka, kb) = (
            a.param_of(pick_a).floor() as usize,
            b.param_of(pick_b).floor() as usize,
        );
        let n = p.nseg();
        let (ka, kb) = (ka.min(n - 1), kb.min(n - 1));
        let v = if ka + 1 == kb {
            kb
        } else if kb + 1 == ka {
            ka
        } else if p.closed && ((ka == n - 1 && kb == 0) || (kb == n - 1 && ka == 0)) {
            0
        } else {
            return None;
        };
        let out = Curve::Poly(chamfer_poly_vertex(p, v, how)?);
        return Some(Corner {
            a: out.clone(),
            b: out,
            added: None,
        });
    }
    let (pa, at_a) = end_piece(a, pick_a)?;
    let (pb, at_b) = end_piece(b, pick_b)?;
    let (Car::Lin { p: p1, u: u1 }, Car::Lin { p: p2, u: u2 }) = (Car::of(&pa)?, Car::of(&pb)?) else {
        return None;
    };
    let tol = Tol::of_box(&pa.bbox().union(pb.bbox())).lin * 16.0;
    let (s, _) = super::curve::line_line(p1, u1, p2, u2)?;
    let x = p1 + u1 * s;
    // from the corner toward each pick
    let ka = if (pick_a - x).dot(u1) >= 0.0 { u1 } else { -u1 };
    let kb = if (pick_b - x).dot(u2) >= 0.0 { u2 } else { -u2 };
    let (d1, d2) = match how {
        Chamfer::Distances(d1, d2) => (d1, d2),
        Chamfer::Angle(d, ang) => {
            let th = dmath::acos_clamped(ka.dot(kb));
            let den = dmath::sin(th + ang);
            if den.abs() < 1e-12 {
                return None;
            }
            (d, d * dmath::sin(ang) / den)
        }
    };
    if d1 < 0.0 || d2 < 0.0 {
        return None;
    }
    let (qa, qb) = (x + ka * d1, x + kb * d2);
    // each kept from its chamfer point on, the way its pick says (a pick may be in the part cut)
    let na = keep_from(&pa, x, qa, ka, d1, tol)?;
    let nb = keep_from(&pb, x, qb, kb, d2, tol)?;
    let added = (qa.dist(qb) > tol).then_some(Curve::Line { a: qa, b: qb });
    Some(Corner {
        a: put_back(na, at_a, tol)?,
        b: put_back(nb, at_b, tol)?,
        added,
    })
}

/// The two segments either side of vertex `v` (v >= 1, or 0 on a closed polyline).
fn around(p: &Poly, v: usize) -> Option<(usize, usize)> {
    let n = p.nseg();
    if p.closed {
        (n >= 2).then(|| ((v + n - 1) % n, v % n))
    } else {
        (v >= 1 && v < n).then(|| (v - 1, v))
    }
}

/// Replaces the corner at vertex v: segment i ends at `ea`, `mid` (if any) follows, segment j
/// starts at `sb`.
fn rebuild(p: &Poly, i: usize, j: usize, ea: Seg, mid: Option<Seg>, sb: Seg) -> Poly {
    let mut segs = p.segs();
    segs[i] = ea;
    segs[j] = sb;
    let _ = j;
    // a closing corner's segment goes last, from the last segment's end to the first's start
    let mut out: Vec<Seg> = vec![];
    for (k, s) in segs.iter().enumerate() {
        out.push(*s);
        if k == i
            && let Some(m) = mid
        {
            out.push(m);
        }
    }
    let mut q = Poly::from_segs(&out, p.closed);
    q.const_width = p.const_width;
    q
}

fn as_seg(c: &Curve) -> Option<Seg> {
    match c {
        Curve::Line { a, b } => Some(Seg::Line(*a, *b)),
        Curve::Arc(a) => Some(Seg::Arc(*a)),
        _ => None,
    }
}

/// FILLET at one polyline vertex (between its two segments, lines or arcs). None when the
/// segments are too short for the radius or meet without a corner.
pub fn fillet_poly_vertex(p: &Poly, v: usize, r: f64) -> Option<Poly> {
    let (i, j) = around(p, v)?;
    let (s1, s2) = (p.seg(i), p.seg(j));
    let (c1, c2) = (s1.curve(), s2.curve());
    let tol = Tol::of_box(&Curve::Poly(p.clone()).bbox()).lin * 16.0;
    // no corner: tangent continuous
    if c1.tangent(c1.domain().1).cross(c2.tangent(0.0)).abs() <= 1e-9 {
        return None;
    }
    let f = fillet(&c1, s1.point(0.01), &c2, s2.point(0.99), r)?;
    let (a, b) = (as_seg(&f.a)?, as_seg(&f.b)?);
    // each keeps its far end
    if a.start().dist(s1.start()) > tol || b.end().dist(s2.end()) > tol {
        return None;
    }
    let mid = match &f.added {
        Some(Curve::Arc(arc)) => Some(Seg::Arc(*arc)),
        _ => None,
    };
    Some(rebuild(p, i, j, a, mid, b))
}

/// FILLET at every corner of a polyline (Polyline option) that the radius fits; the others are
/// left as they are.
pub fn fillet_polyline(p: &Poly, r: f64) -> Poly {
    corners(p, |q, v| fillet_poly_vertex(q, v, r))
}

/// CHAMFER at one polyline vertex between two line segments.
pub fn chamfer_poly_vertex(p: &Poly, v: usize, how: Chamfer) -> Option<Poly> {
    let (i, j) = around(p, v)?;
    let (s1, s2) = (p.seg(i), p.seg(j));
    if !matches!((s1, s2), (Seg::Line(..), Seg::Line(..))) {
        return None;
    }
    let tol = Tol::of_box(&Curve::Poly(p.clone()).bbox()).lin * 16.0;
    let f = chamfer(&s1.curve(), s1.point(0.01), &s2.curve(), s2.point(0.99), how)?;
    let (a, b) = (as_seg(&f.a)?, as_seg(&f.b)?);
    if a.start().dist(s1.start()) > tol || b.end().dist(s2.end()) > tol {
        return None;
    }
    let mid = match &f.added {
        Some(Curve::Line { a, b }) => Some(Seg::Line(*a, *b)),
        _ => None,
    };
    Some(rebuild(p, i, j, a, mid, b))
}

/// CHAMFER at every corner of a polyline between line segments.
pub fn chamfer_polyline(p: &Poly, how: Chamfer) -> Poly {
    corners(p, |q, v| chamfer_poly_vertex(q, v, how))
}

/// Applies a vertex operation at each original corner in turn.
fn corners(p: &Poly, f: impl Fn(&Poly, usize) -> Option<Poly>) -> Poly {
    let mut q = p.clone();
    let tol = Tol::of_box(&Curve::Poly(p.clone()).bbox()).lin * 16.0;
    // walk the original vertices, finding each in the changed polyline by position
    let verts: Vec<V2> = if p.closed {
        p.pts.clone()
    } else {
        p.pts[1..p.pts.len().saturating_sub(1)].to_vec()
    };
    for vp in verts {
        let Some(v) = q.pts.iter().position(|x| x.dist(vp) <= tol) else {
            continue;
        };
        if let Some(r) = f(&q, v) {
            q = r;
        }
    }
    q
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::geom::v2;

    fn close(a: V2, b: V2) -> bool {
        a.dist(b) < 1e-9
    }

    #[test]
    fn line_line() {
        let a = Curve::line(v2(0.0, 0.0), v2(10.0, 0.0));
        let b = Curve::line(v2(12.0, 2.0), v2(12.0, 10.0));
        let f = fillet(&a, v2(5.0, 0.0), &b, v2(12.0, 8.0), 2.0).unwrap();
        assert_eq!(f.a, Curve::line(v2(0.0, 0.0), v2(10.0, 0.0)));
        assert_eq!(f.b, Curve::line(v2(12.0, 2.0), v2(12.0, 10.0)));
        let Some(Curve::Arc(arc)) = f.added else { panic!() };
        assert!(close(arc.c, v2(10.0, 2.0)) && (arc.r - 2.0).abs() < 1e-12);
        assert!(close(arc.start_pt(), v2(10.0, 0.0)) && close(arc.end_pt(), v2(12.0, 2.0)));
        // crossing lines: the picks choose the quadrant
        let h = Curve::line(v2(-10.0, 0.0), v2(10.0, 0.0));
        let v = Curve::line(v2(0.0, -10.0), v2(0.0, 10.0));
        for (pa, pb, want) in [
            (v2(5.0, 0.0), v2(0.0, 5.0), v2(1.0, 1.0)),
            (v2(-5.0, 0.0), v2(0.0, 5.0), v2(-1.0, 1.0)),
            (v2(-5.0, 0.0), v2(0.0, -5.0), v2(-1.0, -1.0)),
            (v2(5.0, 0.0), v2(0.0, -5.0), v2(1.0, -1.0)),
        ] {
            let f = fillet(&h, pa, &v, pb, 1.0).unwrap();
            let Some(Curve::Arc(arc)) = f.added else { panic!() };
            assert!(close(arc.c, want), "{want:?} {:?}", arc.c);
            assert!(f.a.dist(pa) < 1e-9 && f.b.dist(pb) < 1e-9);
            assert!((f.a.length() - 9.0).abs() < 1e-9 && (f.b.length() - 9.0).abs() < 1e-9);
        }
        // radius 0: a sharp corner
        let f = fillet(&a, v2(5.0, 0.0), &b, v2(12.0, 8.0), 0.0).unwrap();
        assert!(f.added.is_none() && close(f.a.end(), v2(12.0, 0.0)) && close(f.b.start(), v2(12.0, 0.0)));
        // too large
        assert!(fillet(&a, v2(5.0, 0.0), &b, v2(12.0, 8.0), 50.0).is_none());
    }

    #[test]
    fn parallel_lines() {
        let a = Curve::line(v2(0.0, 0.0), v2(10.0, 0.0));
        let b = Curve::line(v2(2.0, 4.0), v2(8.0, 4.0));
        let f = fillet(&a, v2(9.0, 0.0), &b, v2(7.0, 4.0), 1.0).unwrap();
        let Some(Curve::Arc(arc)) = f.added else { panic!() };
        assert!(
            close(arc.c, v2(10.0, 2.0))
                && (arc.r - 2.0).abs() < 1e-12
                && (arc.sweep.abs() - PI).abs() < 1e-12,
            "{arc:?}"
        );
        assert!(arc.bbox().max.x > 11.9);
        assert!(close(f.b.end(), v2(10.0, 4.0)) && close(f.b.start(), v2(2.0, 4.0)));
    }

    #[test]
    fn line_arc_all_quadrants() {
        // a horizontal line through a circle's middle; each quadrant's fillet by the picks
        let l = Curve::line(v2(-20.0, 0.0), v2(20.0, 0.0));
        let c = Curve::arc(V2::ZERO, 5.0, 0.0, 2.0 * PI - 1e-3);
        for (pl, pc) in [
            (v2(15.0, 0.0), v2(0.0, 5.0)),
            (v2(-15.0, 0.0), v2(0.0, 5.0)),
            (v2(-15.0, 0.0), v2(0.0, -5.0)),
            (v2(15.0, 0.0), v2(0.0, -5.0)),
        ] {
            let f = fillet(&l, pl, &c, pc, 1.0).unwrap();
            let Some(Curve::Arc(arc)) = f.added else { panic!() };
            // tangent to both: 1 from the line, 6 from the centre (outside the circle)
            assert!(
                (arc.c.y.abs() - 1.0).abs() < 1e-9 && (arc.c.len() - 6.0).abs() < 1e-9,
                "{:?}",
                arc.c
            );
            assert!(arc.c.x.signum() == pl.x.signum() && arc.c.y.signum() == pc.y.signum());
            assert!(f.a.dist(pl) < 1e-9 && f.b.dist(pc) < 1e-9);
            assert!(f.a.dist(arc.start_pt()) < 1e-9 || f.a.dist(arc.end_pt()) < 1e-9);
            assert!(f.b.dist(arc.start_pt()) < 1e-9 || f.b.dist(arc.end_pt()) < 1e-9);
            // the line now stops at the fillet
            assert!(f.a.length() < 20.0);
        }
    }

    #[test]
    fn arc_arc_and_circles() {
        let a = Curve::arc(v2(0.0, 0.0), 5.0, 0.0, PI);
        let b = Curve::arc(v2(12.0, 0.0), 5.0, 0.0, PI);
        let f = fillet(&a, v2(-3.0, 4.0), &b, v2(15.0, 4.0), 2.0).unwrap();
        let Some(Curve::Arc(arc)) = f.added else { panic!() };
        assert!(
            (arc.c.dist(v2(0.0, 0.0)) - 7.0).abs() < 1e-9 && (arc.c.dist(v2(12.0, 0.0)) - 7.0).abs() < 1e-9
        );
        assert!(arc.c.y > 0.0);
        // a circle stays whole
        let c = Curve::circle(v2(0.0, 0.0), 5.0);
        let l = Curve::line(v2(-20.0, 6.0), v2(20.0, 6.0));
        let f = fillet(&c, v2(3.0, 4.0), &l, v2(10.0, 6.0), 1.0).unwrap();
        assert_eq!(f.a, c);
        let Some(Curve::Arc(arc)) = f.added else { panic!() };
        assert!(close(arc.c, v2(11.0f64.sqrt(), 5.0)));
        assert!(close(f.b.start(), v2(11.0f64.sqrt(), 6.0)));
    }

    #[test]
    fn chamfers() {
        let a = Curve::line(v2(0.0, 0.0), v2(10.0, 0.0));
        let b = Curve::line(v2(10.0, 0.0), v2(10.0, 10.0));
        let f = chamfer(&a, v2(5.0, 0.0), &b, v2(10.0, 5.0), Chamfer::Distances(2.0, 3.0)).unwrap();
        assert!(close(f.a.end(), v2(8.0, 0.0)) && close(f.b.start(), v2(10.0, 3.0)));
        assert_eq!(f.added, Some(Curve::line(v2(8.0, 0.0), v2(10.0, 3.0))));
        let f = chamfer(&a, v2(5.0, 0.0), &b, v2(10.0, 5.0), Chamfer::Angle(2.0, PI / 4.0)).unwrap();
        assert!(close(f.b.start(), v2(10.0, 2.0)));
        // lines that stop short are extended to the chamfer
        let c = Curve::line(v2(0.0, 0.0), v2(5.0, 0.0));
        let d = Curve::line(v2(10.0, 5.0), v2(10.0, 10.0));
        let f = chamfer(&c, v2(1.0, 0.0), &d, v2(10.0, 9.0), Chamfer::Distances(1.0, 1.0)).unwrap();
        assert!(close(f.a.end(), v2(9.0, 0.0)) && close(f.b.start(), v2(10.0, 1.0)));
    }

    #[test]
    fn polyline_vertices() {
        let p = Poly::new(
            vec![v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0), v2(0.0, 10.0)],
            vec![],
            true,
        );
        let q = fillet_polyline(&p, 2.0);
        assert_eq!(q.nseg(), 8);
        let len = Curve::Poly(q.clone()).length();
        assert!((len - (4.0 * 6.0 + 2.0 * PI * 2.0)).abs() < 1e-9, "{len}");
        // the arcs bulge as a quarter turn each
        assert_eq!(q.bulges.iter().filter(|b| b.abs() > 0.0).count(), 4);
        let one = fillet_poly_vertex(&p, 1, 2.0).unwrap();
        assert_eq!(one.nseg(), 5);
        assert!(fillet_poly_vertex(&p, 1, 20.0).is_none());
        let c = chamfer_polyline(&p, Chamfer::Distances(1.0, 1.0));
        assert_eq!(c.nseg(), 8);
        assert!((Curve::Poly(c).length() - (4.0 * 8.0 + 4.0 * 2.0f64.sqrt())).abs() < 1e-9);
        // fillet two segments of one polyline by picks
        let open = Curve::poly(&[v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0)], &[], false);
        let f = fillet(&open, v2(5.0, 0.0), &open, v2(10.0, 5.0), 3.0).unwrap();
        let Curve::Poly(x) = &f.a else { panic!() };
        assert_eq!(x.nseg(), 3);
        // a polyline's end segment and a line
        let l = Curve::line(v2(0.0, 15.0), v2(5.0, 15.0));
        let f = fillet(&open, v2(10.0, 8.0), &l, v2(2.0, 15.0), 1.0).unwrap();
        assert!(close(f.a.end(), v2(10.0, 14.0)) && close(f.b.end(), v2(9.0, 15.0)));
    }

    #[test]
    fn a_pick_where_the_fillet_cuts() {
        // the top line picked between the corner and where the arc will meet it (AutoCAD keeps
        // that side of the corner all the same), the side line picked just off it
        let a = Curve::line(v2(176.0, 150.0), v2(176.0, 186.0));
        let b = Curve::line(v2(224.00004, 186.0), v2(175.99996, 186.0));
        let k = fillet(&a, v2(175.68, 167.45), &b, v2(190.98, 185.92), 15.0).unwrap();
        let Some(Curve::Arc(arc)) = &k.added else { panic!() };
        assert!(arc.c.dist(v2(191.0, 171.0)) < 1e-9 && (arc.r - 15.0).abs() < 1e-12);
        assert!(k.a.end().dist(v2(176.0, 171.0)) < 1e-9, "{:?}", k.a);
        assert!(
            k.b.end().dist(v2(191.0, 186.0)) < 1e-9 || k.b.start().dist(v2(191.0, 186.0)) < 1e-9,
            "{:?}",
            k.b
        );
        // and a chamfer picked inside its distances
        let c = chamfer(
            &a,
            v2(176.0, 184.0),
            &b,
            v2(178.0, 186.0),
            Chamfer::Distances(5.0, 5.0),
        )
        .unwrap();
        assert!(c.a.end().dist(v2(176.0, 181.0)) < 1e-9, "{:?}", c.a);
    }
}
