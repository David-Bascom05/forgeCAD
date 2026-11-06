//! The maps MOVE, COPY, ROTATE, SCALE, MIRROR and ARRAY place copies by (all `Affine`), and
//! STRETCH, which moves what lies inside a window and keeps the rest.

use super::curve::{Arc, Curve, Ellipse, Poly};
use super::geom::{Affine, Bbox, V2};

/// MOVE or COPY from a base point to a second point.
pub fn move_by(from: V2, to: V2) -> Affine {
    Affine::translate(to - from)
}

/// ROTATE about a base point; the Reference option's angle is `to - from`.
pub fn rotate(base: V2, angle: f64) -> Affine {
    Affine::rotate(base, angle)
}

/// SCALE about a base point.
pub fn scale(base: V2, k: f64) -> Affine {
    Affine::scale(base, k)
}

/// MIRROR in the line through two points.
pub fn mirror(a: V2, b: V2) -> Affine {
    Affine::mirror(a, b)
}

/// ARRAY rectangular: rows by columns at the spacings (the first is the original's place).
pub fn array_rect(rows: usize, cols: usize, row_spacing: f64, col_spacing: f64, angle: f64) -> Vec<Affine> {
    let x = V2::polar(angle, col_spacing);
    let y = V2::polar(angle, row_spacing).perp();
    let mut out = vec![];
    for r in 0..rows {
        for c in 0..cols {
            out.push(Affine::translate(x * c as f64 + y * r as f64));
        }
    }
    out
}

/// ARRAY polar: `count` places about `center` spread over `fill` radians (2 pi a whole turn,
/// which doesn't repeat the first), the items turned with the array or only moved.
pub fn array_polar(center: V2, count: usize, fill: f64, rotate_items: bool, item_base: V2) -> Vec<Affine> {
    if count == 0 {
        return vec![];
    }
    let whole = (fill.abs() - super::geom::TAU).abs() < 1e-12;
    let step = if whole || count == 1 {
        fill / count as f64
    } else {
        fill / (count - 1) as f64
    };
    (0..count)
        .map(|k| {
            let r = Affine::rotate(center, step * k as f64);
            if rotate_items {
                r
            } else {
                Affine::translate(r.apply(item_base) - item_base)
            }
        })
        .collect()
}

/// STRETCH: the curve with what lies in `window` moved by `d`. A curve wholly inside moves;
/// lines and polylines move the vertices inside (a polyline arc keeps its bulge, an arc its
/// included angle); a circle or ellipse moves when its centre is inside; a spline moves the
/// control points inside. None when nothing of it is inside.
pub fn stretch(c: &Curve, window: &Bbox, d: V2) -> Option<Curve> {
    let inside = |p: V2| window.contains(p);
    let mv = |p: V2| if inside(p) { p + d } else { p };
    let bb = c.bbox();
    if window.contains(bb.min) && window.contains(bb.max) {
        return c.transformed(&Affine::translate(d));
    }
    match c {
        Curve::Line { a, b } => (inside(*a) || inside(*b)).then(|| Curve::Line { a: mv(*a), b: mv(*b) }),
        Curve::Ray { at, dir } => inside(*at).then(|| Curve::Ray {
            at: *at + d,
            dir: *dir,
        }),
        Curve::Xline { at, dir } => inside(*at).then(|| Curve::Xline {
            at: *at + d,
            dir: *dir,
        }),
        Curve::Circle { c: o, r } => inside(*o).then(|| Curve::Circle { c: *o + d, r: *r }),
        Curve::Ellipse(e) => inside(e.c).then(|| Curve::Ellipse(Ellipse { c: e.c + d, ..*e })),
        Curve::Arc(a) => {
            let (s, e) = (a.start_pt(), a.end_pt());
            if !inside(s) && !inside(e) {
                return None;
            }
            let b = a.bulge();
            let arc = Arc::from_bulge(mv(s), mv(e), b)?;
            Some(Curve::Arc(arc))
        }
        Curve::Poly(p) => {
            if !p.pts.iter().any(|&q| inside(q)) {
                return None;
            }
            Some(Curve::Poly(Poly {
                pts: p.pts.iter().map(|&q| mv(q)).collect(),
                ..p.clone()
            }))
        }
        Curve::Spline(n) => {
            if !n.ctrl.iter().any(|&q| inside(q)) {
                return None;
            }
            let mut m = n.clone();
            m.ctrl = n.ctrl.iter().map(|&q| mv(q)).collect();
            Some(Curve::Spline(m))
        }
        Curve::Offset(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::geom::{PI, TAU, v2};

    #[test]
    fn arrays() {
        let r = array_rect(2, 3, 5.0, 10.0, 0.0);
        assert_eq!(r.len(), 6);
        assert!(r[5].apply(V2::ZERO).dist(v2(20.0, 5.0)) < 1e-12);
        let p = array_polar(V2::ZERO, 4, TAU, true, v2(1.0, 0.0));
        assert!(p[1].apply(v2(1.0, 0.0)).dist(v2(0.0, 1.0)) < 1e-12);
        let q = array_polar(V2::ZERO, 3, PI, false, v2(1.0, 0.0));
        assert!(q[2].apply(v2(1.0, 0.0)).dist(v2(-1.0, 0.0)) < 1e-12);
        assert!(q[2].similarity_scale(1e-12).is_some() && q[2].m == Affine::IDENTITY.m);
    }

    #[test]
    fn stretches() {
        let w = Bbox {
            min: v2(8.0, -1.0),
            max: v2(12.0, 1.0),
        };
        let l = Curve::line(v2(0.0, 0.0), v2(10.0, 0.0));
        assert_eq!(
            stretch(&l, &w, v2(2.0, 0.0)).unwrap(),
            Curve::line(v2(0.0, 0.0), v2(12.0, 0.0))
        );
        let p = Curve::poly(
            &[v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0)],
            &[0.0, 0.5, 0.0],
            false,
        );
        let s = stretch(&p, &w, v2(0.0, -1.0)).unwrap();
        let Curve::Poly(q) = &s else { panic!() };
        assert!(q.pts[1].dist(v2(10.0, -1.0)) < 1e-12 && q.bulges[1] == 0.5);
        let a = Curve::arc(V2::ZERO, 10.0, 0.0, PI / 2.0);
        let s = stretch(&a, &w, v2(1.0, 0.0)).unwrap();
        assert!(s.start().dist(v2(11.0, 0.0)) < 1e-12 && s.end().dist(v2(0.0, 10.0)) < 1e-12);
        assert!(stretch(&Curve::circle(v2(50.0, 0.0), 1.0), &w, v2(1.0, 0.0)).is_none());
    }
}
