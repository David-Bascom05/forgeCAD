//! OFFSET: a curve moved a distance to one side, exactly. Lines, circles and arcs offset to lines,
//! circles and arcs; polylines segment by segment, joined at their corners by extending them to
//! meet (AutoCAD's OFFSETGAPTYPE 0) or, where they can't meet, by an arc round the corner; an
//! ellipse or spline to its exact offset curve (`Curve::Offset`), which is fitted to a SPLINE only
//! when written.

use super::curve::{Arc, Curve, Offset, Poly, Seg};
use super::geom::{Bbox, PI, TAU, Tol, V2, wrap, wrap_pi};
use super::intersect::{Ext, intersect_tol};

/// Which side of a curve a point is: +1 left of its direction, -1 right, 0 on it.
pub fn side_of(c: &Curve, p: V2) -> f64 {
    let (t, q) = c.closest_unbounded(p);
    let tol = Tol::of_box(&c.bbox().union(Bbox::of([p]))).lin;
    if q.dist(p) <= tol {
        return 0.0;
    }
    let s = c.tangent(t).cross(p - q);
    if s > 0.0 {
        1.0
    } else if s < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// OFFSET through a point: `dist` to the side `through` is on. None when the point is on the
/// curve or the offset vanishes (an arc offset inward past its centre).
pub fn offset(c: &Curve, dist: f64, through: V2) -> Option<Curve> {
    let s = side_of(c, through);
    if s == 0.0 || dist <= 0.0 {
        return None;
    }
    offset_signed(c, dist * s)
}

/// OFFSET through a point with the distance it sets (OFFSET's Through option).
pub fn offset_through(c: &Curve, through: V2) -> Option<Curve> {
    let (_, q) = c.closest(through);
    offset(c, q.dist(through), through)
}

/// The offset by `d`, positive to the left of the curve's direction.
pub fn offset_signed(c: &Curve, d: f64) -> Option<Curve> {
    if d == 0.0 {
        return Some(c.clone());
    }
    match c {
        Curve::Line { a, b } => {
            let n = (*b - *a).unit()?.perp() * d;
            Some(Curve::Line { a: *a + n, b: *b + n })
        }
        Curve::Ray { at, dir } => Some(Curve::Ray {
            at: *at + dir.perp() * d,
            dir: *dir,
        }),
        Curve::Xline { at, dir } => Some(Curve::Xline {
            at: *at + dir.perp() * d,
            dir: *dir,
        }),
        Curve::Circle { c, r } => {
            // counter-clockwise: the left is inside
            let r2 = r - d;
            (r2 > 0.0).then_some(Curve::Circle { c: *c, r: r2 })
        }
        Curve::Arc(a) => {
            let r2 = a.r - d * a.dir();
            (r2 > 0.0).then_some(Curve::Arc(Arc { r: r2, ..*a }))
        }
        Curve::Ellipse(_) | Curve::Spline(_) => Some(Curve::Offset(Offset {
            base: Box::new(c.clone()),
            d,
        })),
        Curve::Offset(o) => Some(Curve::Offset(Offset {
            base: o.base.clone(),
            d: o.d + d,
        })),
        Curve::Poly(p) => offset_poly(p, d).map(Curve::Poly),
    }
}

/// An offset segment: its carrier and its (moving) ends.
#[derive(Clone, Copy, Debug)]
struct Piece {
    /// Line: None; arc: centre, radius, direction and the original sweep's size.
    arc: Option<(V2, f64, f64, f64)>,
    s: V2,
    e: V2,
    /// The original direction of a line piece.
    dir: V2,
}

impl Piece {
    fn seg(&self) -> Seg {
        match self.arc {
            None => Seg::Line(self.s, self.e),
            Some((c, r, dir, _)) => {
                let a0 = (self.s - c).angle();
                let travel = wrap(dir * ((self.e - c).angle() - a0));
                let travel = if travel == 0.0 && self.s != self.e {
                    TAU
                } else {
                    travel
                };
                Seg::Arc(Arc {
                    c,
                    r,
                    start: a0,
                    sweep: dir * travel,
                })
            }
        }
    }
    /// The carrier as a curve whose extension `intersect` follows.
    fn carrier(&self) -> Curve {
        match self.arc {
            None => Curve::Line {
                a: self.s,
                b: if self.s == self.e {
                    self.s + self.dir
                } else {
                    self.e
                },
            },
            Some((c, r, dir, _)) => Curve::Arc(Arc {
                c,
                r,
                start: (self.s - c).angle(),
                sweep: dir * PI,
            }),
        }
    }
    /// Whether the piece has turned inside out (its ends crossed).
    fn inverted(&self, tol: f64) -> bool {
        match self.arc {
            None => (self.e - self.s).dot(self.dir) <= tol * 0.5,
            Some((c, _, dir, orig)) => {
                if self.s.dist(self.e) <= tol {
                    return orig < PI;
                }
                let travel = wrap(dir * ((self.e - c).angle() - (self.s - c).angle()));
                travel > orig + 0.5 * (TAU - orig)
            }
        }
    }
}

fn offset_piece(s: &Seg, d: f64, tol: f64) -> Option<Piece> {
    match s {
        Seg::Line(a, b) => {
            let u = (*b - *a).unit()?;
            let n = u.perp() * d;
            Some(Piece {
                arc: None,
                s: *a + n,
                e: *b + n,
                dir: u,
            })
        }
        Seg::Arc(a) => {
            let r = a.r - d * a.dir();
            if r <= tol {
                return None;
            }
            let o = Arc { r, ..*a };
            Some(Piece {
                arc: Some((a.c, r, a.dir(), a.len())),
                s: o.start_pt(),
                e: o.end_pt(),
                dir: V2::ZERO,
            })
        }
    }
}

/// Joins piece `a`'s end to piece `b`'s start, round original vertex `v`: at their carriers'
/// meeting nearest the gap, else with an arc of radius |d| about v.
fn join_corner(a: &mut Piece, b: &mut Piece, v: V2, d: f64, tol: f64) -> Result<Option<Piece>, ()> {
    if a.e.dist(b.s) <= tol {
        let m = a.e.mid(b.s);
        a.e = m;
        b.s = m;
        return Ok(None);
    }
    let (ca, cb) = (a.carrier(), b.carrier());
    let gap = a.e.mid(b.s);
    let hits = intersect_tol(&ca, &cb, Ext::Both, tol);
    let best = hits
        .points
        .iter()
        .map(|h| h.p)
        .min_by(|x, y| x.dist(gap).total_cmp(&y.dist(gap)));
    // a meeting much further than the offset is a near-tangent glancing one: round it instead
    if let Some(x) = best
        && (a.arc.is_none() && b.arc.is_none() || x.dist(v) <= 4.0 * d.abs() + tol)
    {
        a.e = x;
        b.s = x;
        return Ok(None);
    }
    if a.arc.is_none() && b.arc.is_none() {
        // parallel straights apart: what was between them has vanished
        return Err(());
    }
    // round the corner about the vertex
    let a0 = (a.e - v).angle();
    let sweep = wrap_pi((b.s - v).angle() - a0);
    Ok(Some(Piece {
        arc: Some((v, d.abs(), if sweep < 0.0 { -1.0 } else { 1.0 }, sweep.abs())),
        s: a.e,
        e: b.s,
        dir: V2::ZERO,
    }))
}

fn offset_poly(p: &Poly, d: f64) -> Option<Poly> {
    let segs = p.segs();
    if segs.is_empty() {
        return None;
    }
    let tol = Tol::of_box(&Curve::Poly(p.clone()).bbox()).lin * 16.0;
    // the original vertex each piece starts at
    let mut items: Vec<(V2, V2, Piece)> = segs
        .iter()
        .filter_map(|s| offset_piece(s, d, tol).map(|o| (s.start(), s.end(), o)))
        .collect();
    let mut guard = 0;
    loop {
        guard += 1;
        if items.is_empty() || guard > 4 * segs.len() + 4 {
            return None;
        }
        let n = items.len();
        let mut pieces: Vec<Piece> = items.iter().map(|x| x.2).collect();
        let mut fillers: Vec<Option<Piece>> = vec![None; n];
        let pairs = if p.closed { n } else { n - 1 };
        for k in 0..pairs {
            let j = (k + 1) % n;
            // the vertex between them: the first's end (or the second's start when one vanished)
            let v = if items[k].1.dist(items[j].0) <= tol {
                items[k].1
            } else {
                items[k].1.mid(items[j].0)
            };
            let (mut a, mut b) = (pieces[k], pieces[j]);
            fillers[k] = join_corner(&mut a, &mut b, v, d, tol).ok()?;
            pieces[k].e = a.e;
            if j == k {
                pieces[k].s = b.s;
            } else {
                pieces[j].s = b.s;
            }
        }
        // drop inside-out pieces, then join again
        let bad: Vec<usize> = (0..n).filter(|&k| pieces[k].inverted(tol)).collect();
        if !bad.is_empty() && n > 1 {
            let mut keep = vec![];
            for (k, it) in items.iter().enumerate() {
                if !bad.contains(&k) {
                    keep.push(*it);
                }
            }
            if keep.is_empty() {
                return None;
            }
            items = keep;
            continue;
        }
        if !bad.is_empty() {
            return None;
        }
        let mut out_segs = vec![];
        for k in 0..n {
            out_segs.push(pieces[k].seg());
            if let Some(f) = fillers[k] {
                out_segs.push(f.seg());
            }
        }
        let mut q = Poly::from_segs(&out_segs, p.closed);
        q.const_width = p.const_width;
        if out_segs.len() == p.nseg() && p.widths.len() == p.pts.len() {
            q.widths = p.widths.clone();
        }
        return Some(q);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::curve::Ellipse;
    use crate::draft::geom::v2;

    fn close(a: V2, b: V2) -> bool {
        a.dist(b) < 1e-9
    }

    #[test]
    fn lines_arcs_circles() {
        let l = Curve::line(v2(0.0, 0.0), v2(10.0, 0.0));
        let o = offset(&l, 2.0, v2(5.0, -3.0)).unwrap();
        assert_eq!(o, Curve::line(v2(0.0, -2.0), v2(10.0, -2.0)));
        let c = Curve::circle(V2::ZERO, 5.0);
        assert_eq!(
            offset(&c, 2.0, v2(1.0, 0.0)).unwrap(),
            Curve::circle(V2::ZERO, 3.0)
        );
        assert_eq!(
            offset(&c, 2.0, v2(9.0, 0.0)).unwrap(),
            Curve::circle(V2::ZERO, 7.0)
        );
        assert!(offset(&c, 6.0, v2(1.0, 0.0)).is_none());
        let a = Curve::arc(V2::ZERO, 5.0, 0.0, PI / 2.0);
        let Curve::Arc(x) = offset(&a, 1.0, v2(1.0, 1.0)).unwrap() else {
            panic!()
        };
        assert!((x.r - 4.0).abs() < 1e-12 && x.sweep == PI / 2.0);
        // a clockwise arc: the same side, the same result
        let Curve::Arc(y) = offset(&a.reversed(), 1.0, v2(1.0, 1.0)).unwrap() else {
            panic!()
        };
        assert!((y.r - 4.0).abs() < 1e-12);
        let t = offset_through(&l, v2(3.0, 4.0)).unwrap();
        assert!(close(t.start(), v2(0.0, 4.0)));
    }

    #[test]
    fn ellipse_offsets_exactly() {
        let e = Curve::Ellipse(Ellipse {
            c: V2::ZERO,
            major: v2(6.0, 0.0),
            ratio: 0.5,
            start: 0.0,
            sweep: TAU,
        });
        let o = offset(&e, 1.0, v2(20.0, 0.0)).unwrap();
        assert!(matches!(o, Curve::Offset(_)));
        for k in 0..50 {
            let p = o.point(TAU * k as f64 / 50.0);
            assert!((e.dist(p) - 1.0).abs() < 1e-9);
        }
        assert!(close(o.point(0.0), v2(7.0, 0.0)));
    }

    #[test]
    fn rectangle_in_and_out() {
        let r = Curve::poly(
            &[v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 6.0), v2(0.0, 6.0)],
            &[],
            true,
        );
        let out = offset(&r, 1.0, v2(20.0, 3.0)).unwrap();
        let Curve::Poly(p) = &out else { panic!() };
        assert_eq!(p.pts.len(), 4);
        assert!(close(p.pts[0], v2(-1.0, -1.0)) && close(p.pts[2], v2(11.0, 7.0)));
        let inn = offset(&r, 1.0, v2(5.0, 3.0)).unwrap();
        assert!((inn.length() - 2.0 * (8.0 + 4.0)).abs() < 1e-9);
        // too far in: the short sides vanish
        assert!(offset(&r, 3.5, v2(5.0, 3.0)).is_none());
    }

    #[test]
    fn closed_polyline_with_bulges() {
        // a slot: two straights and two half circles of radius 5, counter-clockwise
        let s = Curve::poly(
            &[v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0), v2(0.0, 10.0)],
            &[0.0, 1.0, 0.0, 1.0],
            true,
        );
        assert!((s.length() - (20.0 + 10.0 * PI)).abs() < 1e-9);
        let inn = offset(&s, 2.0, v2(5.0, 5.0)).unwrap();
        assert!(
            (inn.length() - (20.0 + 6.0 * PI)).abs() < 1e-9,
            "{}",
            inn.length()
        );
        assert!(close(inn.start(), v2(0.0, 2.0)));
        let out = offset(&s, 2.0, v2(5.0, -5.0)).unwrap();
        assert!((out.length() - (20.0 + 14.0 * PI)).abs() < 1e-9);
        let Curve::Poly(p) = &out else { panic!() };
        assert!(p.closed && p.pts.len() == 4 && (p.bulges[1] - 1.0).abs() < 1e-12);
        // inward past the ends' radius, the arcs vanish and the straights meet
        let deep = offset(&s, 5.0, v2(5.0, 5.0));
        assert!(deep.is_none() || deep.unwrap().length() <= 20.0 + 1e-9);
    }

    #[test]
    fn line_into_arc_corners() {
        // a line into an arc at a corner: extended to meet outside, trimmed to meet inside
        let p = Curve::poly(
            &[v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0)],
            &[0.0, 0.5, 0.0],
            false,
        );
        for side in [v2(5.0, -5.0), v2(5.0, 2.0)] {
            let o = offset(&p, 1.0, side).unwrap();
            let Curve::Poly(q) = &o else { panic!() };
            // at least 1 from the original everywhere, exactly 1 along each piece's middle
            for k in 0..=100 {
                let t = q.nseg() as f64 * k as f64 / 100.0;
                assert!(p.dist(o.point(t)) > 1.0 - 1e-6, "{side:?} {t}");
            }
            assert_eq!(q.nseg(), 2);
            for k in 0..2 {
                assert!((p.dist(o.point(k as f64 + 0.5)) - 1.0).abs() < 1e-9);
            }
        }
    }
}
