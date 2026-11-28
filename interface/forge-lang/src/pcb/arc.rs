//! Three-point circular arcs are flattened to the same nanometre-grid polyline
//! for checking, connectivity and plotting. The authored point splits the arc
//! and is retained exactly. Maximum chord sag is 1 µm before grid rounding.
use super::Point;
use crate::electronics::{Fault, Result};
use crate::syntax::Span;
use forge_math::dmath::{acos_clamped, atan2, sin_cos};
pub(super) fn append(path: &mut Vec<Point>, mid: Point, end: Point, span: Span) -> Result<()> {
    let invalid = || {
        Fault::new(
            "InvalidBoardArc",
            span,
            "an arc requires three distinct non-collinear points within the board grid",
        )
    };
    let &a = path.last().ok_or_else(invalid)?;
    let (bx, by) = (mid[0] - a[0], mid[1] - a[1]);
    let (cx, cy) = (end[0] - a[0], end[1] - a[1]);
    let det = i128::from(bx) * i128::from(cy) - i128::from(by) * i128::from(cx);
    if det == 0 {
        return Err(invalid());
    }
    let (bx, by, cx, cy) = (bx as f64, by as f64, cx as f64, cy as f64);
    let b2 = bx * bx + by * by;
    let c2 = cx * cx + cy * cy;
    let d = 2. * det as f64;
    let ox = (cy * b2 - by * c2) / d;
    let oy = (bx * c2 - cx * b2) / d;
    let r = (ox * ox + oy * oy).sqrt();
    let centre = [a[0] as f64 + ox, a[1] as f64 + oy];
    let angle = |p: Point| atan2(p[1] as f64 - centre[1], p[0] as f64 - centre[0]);
    let sign = det.signum() as f64;
    let step = 2. * acos_clamped((1. - 1000. / r).max(0.));
    for (from, to) in [(a, mid), (mid, end)] {
        let start = angle(from);
        let sweep = ((angle(to) - start) * sign).rem_euclid(std::f64::consts::TAU);
        let count = (sweep / step).ceil().max(1.);
        if !count.is_finite() || count > 65536. {
            return Err(invalid());
        }
        for i in 1..count as usize {
            let (s, c) = sin_cos(start + sign * sweep * i as f64 / count);
            let p = [centre[0] + r * c, centre[1] + r * s];
            if p.iter().any(|v| v.abs() > 500_000_000.) {
                return Err(invalid());
            }
            let q = p.map(|v| v.round() as i64);
            if path.last() != Some(&q) && q != to {
                path.push(q);
            }
        }
        path.push(to);
    }
    Ok(())
}
