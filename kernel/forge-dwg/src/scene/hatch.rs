//! HATCH: boundary paths as rings, and pattern lines cut to the area inside them (even-odd, so
//! islands stay clear).

use super::geom::{P2, P3, bulge_pts, ellipse_pts, interp_pts, nurbs_pts};
use crate::types::{HatchEdge, HatchLine, HatchPath};
use forge_math::dmath;

/// The most pattern lines one family gets before it is left out (AutoCAD calls such a hatch too
/// dense, and so would anyone looking at it).
const MAX_LINES: f64 = 20_000.0;
/// The most dashes one hatch gets.
const MAX_DASHES: usize = 400_000;

/// A boundary path as a ring in the hatch's plane (OCS x, y).
pub fn ring(p: &HatchPath, chord: f64) -> Vec<P2> {
    let mut out: Vec<P3> = vec![];
    if p.flag & 2 != 0 {
        let v = &p.vertices;
        if v.is_empty() {
            return vec![];
        }
        out.push([v[0][0], v[0][1], 0.0]);
        let n = v.len();
        // a polyline path always closes
        for i in 0..n {
            let a = [v[i][0], v[i][1], 0.0];
            let b = [v[(i + 1) % n][0], v[(i + 1) % n][1], 0.0];
            let bulge = if p.has_bulges { v[i][2] } else { 0.0 };
            bulge_pts(a, b, bulge, chord, &mut out);
        }
    } else {
        for e in &p.edges {
            let mut pts: Vec<P3> = vec![];
            edge_pts(e, chord, &mut pts);
            // edges can come in either direction: join at the nearer end
            if let (Some(last), Some(f), Some(l)) = (out.last(), pts.first(), pts.last()) {
                let d0 = (last[0] - f[0]).abs() + (last[1] - f[1]).abs();
                let d1 = (last[0] - l[0]).abs() + (last[1] - l[1]).abs();
                if d1 < d0 {
                    pts.reverse();
                }
            }
            out.extend(pts);
        }
    }
    let mut r: Vec<P2> = Vec::with_capacity(out.len());
    for q in out {
        let q2 = [q[0], q[1]];
        if r.last() != Some(&q2) {
            r.push(q2);
        }
    }
    while r.len() > 1 && r.first() == r.last() {
        r.pop();
    }
    r
}

fn edge_pts(e: &HatchEdge, chord: f64, out: &mut Vec<P3>) {
    match e {
        HatchEdge::Line { a, b } => {
            out.push([a[0], a[1], 0.0]);
            out.push([b[0], b[1], 0.0]);
        }
        HatchEdge::Arc {
            center,
            radius,
            start,
            end,
            ccw,
        } => {
            let c = [center[0], center[1], 0.0];
            let mut sweep = end - start;
            if sweep.abs() >= dmath::TAU - 1e-12 {
                sweep = dmath::TAU;
            } else {
                sweep = dmath::wrap_tau(sweep);
                if sweep == 0.0 {
                    sweep = dmath::TAU;
                }
            }
            if *ccw {
                super::geom::arc_pts(c, *radius, *start, sweep, chord, out);
            } else {
                // clockwise: the angles are stored mirrored
                super::geom::arc_pts(c, *radius, -start, -sweep, chord, out);
            }
        }
        HatchEdge::Ellipse {
            center,
            major,
            ratio,
            start,
            end,
            ccw,
        } => {
            let c = [center[0], center[1], 0.0];
            let mj = [major[0], major[1], 0.0];
            let mn = [-major[1] * ratio, major[0] * ratio, 0.0];
            let mut sweep = dmath::wrap_tau(end - start);
            if sweep == 0.0 {
                sweep = dmath::TAU;
            }
            if *ccw {
                ellipse_pts(c, mj, mn, *start, start + sweep, chord, out);
            } else {
                let mut tmp = vec![];
                ellipse_pts(c, mj, mn, -start - sweep, -start, chord, &mut tmp);
                tmp.reverse();
                out.extend(tmp);
            }
        }
        HatchEdge::Spline(s) => {
            let ctrl: Vec<P3> = s.ctrl.iter().map(|p| [p[0], p[1], 0.0]).collect();
            if ctrl.len() >= 2 {
                out.extend(nurbs_pts(
                    s.degree.max(1) as usize,
                    &s.knots,
                    &ctrl,
                    &s.weights,
                    chord,
                ));
            } else {
                let fit: Vec<P3> = s.fit.iter().map(|p| [p[0], p[1], 0.0]).collect();
                let t = |v: [f64; 2]| (v != [0.0, 0.0]).then_some([v[0], v[1], 0.0]);
                out.extend(interp_pts(&fit, t(s.start_tangent), t(s.end_tangent), 0, chord));
            }
        }
    }
}

/// The pattern's lines cut to the rings (even-odd): segments in the hatch's plane.
pub fn pattern(lines: &[HatchLine], rings: &[Vec<P2>], dot: f64) -> Vec<[P2; 2]> {
    let mut out = vec![];
    let edges: Vec<(P2, P2)> = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .filter(|(a, b)| a != b)
        .collect();
    if edges.is_empty() {
        return out;
    }
    for hl in lines {
        let (s, c) = dmath::sin_cos(hl.angle);
        let d = [c, s];
        let n = [-s, c];
        let o = hl.offset;
        let spacing = o[0] * n[0] + o[1] * n[1];
        if spacing.abs() < 1e-12 || !spacing.is_finite() {
            continue;
        }
        let shift = o[0] * d[0] + o[1] * d[1];
        let b = hl.base;
        // each edge's range across the lines, sorted, swept
        let mut es: Vec<(f64, f64, P2, P2)> = edges
            .iter()
            .map(|&(p, q)| {
                let sp = (p[0] - b[0]) * n[0] + (p[1] - b[1]) * n[1];
                let sq = (q[0] - b[0]) * n[0] + (q[1] - b[1]) * n[1];
                (sp.min(sq), sp.max(sq), p, q)
            })
            .collect();
        es.sort_by(|x, y| x.0.total_cmp(&y.0));
        let smin = es[0].0;
        let smax = es.iter().map(|e| e.1).fold(f64::NEG_INFINITY, f64::max);
        let (k0, k1) = if spacing > 0.0 {
            ((smin / spacing).floor(), (smax / spacing).ceil())
        } else {
            ((smax / spacing).floor(), (smin / spacing).ceil())
        };
        if k1 - k0 > MAX_LINES {
            continue;
        }
        let period: f64 = hl.dashes.iter().map(|x| x.abs()).sum();
        let mut next = 0;
        let mut active: Vec<usize> = vec![];
        let mut ts: Vec<f64> = vec![];
        // walk the lines in increasing s
        let mut ks: Vec<f64> = (k0 as i64..=k1 as i64).map(|k| k as f64).collect();
        if spacing < 0.0 {
            ks.reverse();
        }
        for k in ks {
            let sl = k * spacing;
            while next < es.len() && es[next].0 <= sl {
                active.push(next);
                next += 1;
            }
            active.retain(|&i| es[i].1 >= sl);
            if active.is_empty() {
                continue;
            }
            // the line: q + t d
            let q = [b[0] + k * o[0], b[1] + k * o[1]];
            ts.clear();
            for &i in &active {
                let (_, _, p1, p2) = es[i];
                let s1 = (p1[0] - q[0]) * n[0] + (p1[1] - q[1]) * n[1];
                let s2 = (p2[0] - q[0]) * n[0] + (p2[1] - q[1]) * n[1];
                if (s1 < 0.0) != (s2 < 0.0) {
                    let f = s1 / (s1 - s2);
                    let x = [p1[0] + (p2[0] - p1[0]) * f, p1[1] + (p2[1] - p1[1]) * f];
                    ts.push((x[0] - q[0]) * d[0] + (x[1] - q[1]) * d[1]);
                }
            }
            ts.sort_by(f64::total_cmp);
            let at = |t: f64| [q[0] + d[0] * t, q[1] + d[1] * t];
            for pair in ts.as_chunks::<2>().0 {
                let (ta, tb) = (pair[0], pair[1]);
                if tb - ta <= 0.0 {
                    continue;
                }
                if hl.dashes.is_empty() || period <= 0.0 || hl.dashes.iter().all(|x| *x > 0.0) {
                    out.push([at(ta), at(tb)]);
                    continue;
                }
                if (tb - ta) / period > 1e5 {
                    continue;
                }
                // the pattern runs from the line's origin (shifted along by `shift` each line,
                // which q already includes)
                let _ = shift;
                let mut start = (ta / period).floor() * period;
                'run: loop {
                    for &dl in &hl.dashes {
                        let end = start + dl.abs();
                        if start >= tb {
                            break 'run;
                        }
                        if dl > 0.0 {
                            let (x0, x1) = (start.max(ta), end.min(tb));
                            if x1 > x0 {
                                out.push([at(x0), at(x1)]);
                            }
                        } else if dl == 0.0 && start >= ta {
                            out.push([at(start), at((start + dot).min(tb))]);
                        }
                        start = end;
                    }
                    if out.len() > MAX_DASHES {
                        return out;
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(c: f64, h: f64) -> Vec<P2> {
        vec![[c - h, c - h], [c + h, c - h], [c + h, c + h], [c - h, c + h]]
    }

    #[test]
    fn lines_skip_islands() {
        // horizontal lines one apart, through a 10 square with a 2 square hole in the middle
        let lines = [HatchLine {
            angle: 0.0,
            base: [0.0, 0.5],
            offset: [0.0, 1.0],
            dashes: vec![],
        }];
        let rings = vec![square(5.0, 5.0), square(5.0, 1.0)];
        let segs = pattern(&lines, &rings, 0.0);
        // y = 0.5 .. 9.5: ten lines, the two through the hole in two pieces
        assert_eq!(segs.len(), 12);
        for s in &segs {
            let mid = [(s[0][0] + s[1][0]) / 2.0, s[0][1]];
            assert!(!(mid[0] > 4.0 && mid[0] < 6.0 && mid[1] > 4.0 && mid[1] < 6.0));
            assert!(s[0][0] >= -1e-9 && s[1][0] <= 10.0 + 1e-9);
        }
        let through: Vec<_> = segs.iter().filter(|s| s[0][1] == 4.5).collect();
        assert_eq!(through.len(), 2);
        assert!((through[0][1][0] - 4.0).abs() < 1e-12 && (through[1][0][0] - 6.0).abs() < 1e-12);
    }

    #[test]
    fn dashed_and_slanted() {
        let lines = [HatchLine {
            angle: dmath::FRAC_PI_2,
            base: [0.0, 0.0],
            offset: [-2.0, 0.0],
            dashes: vec![1.0, -1.0],
        }];
        let rings = vec![square(2.0, 2.0)];
        let segs = pattern(&lines, &rings, 0.0);
        // x = 0, 2, 4 (the edges hit at their ends count once), each dashed 0-1, 2-3
        assert!(!segs.is_empty());
        for s in &segs {
            assert!((s[1][1] - s[0][1] - 1.0).abs() < 1e-9);
            assert!(s[0][1].rem_euclid(2.0) < 1e-9);
        }
    }

    #[test]
    fn polyline_ring_with_bulge() {
        let p = HatchPath {
            flag: 2,
            edges: vec![],
            has_bulges: true,
            closed: true,
            vertices: vec![[0.0, 0.0, 1.0], [2.0, 0.0, 0.0]],
            boundary: vec![],
        };
        let r = ring(&p, 1e-3);
        assert!(r.len() > 10);
        assert!(r.iter().all(|q| q[1] <= 1e-9 && q[1] >= -1.0 - 1e-9));
    }
}
