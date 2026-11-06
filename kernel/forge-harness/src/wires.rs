//! Each wire on its own: a slot in its segment's cross-section (a deterministic circle packing
//! inside the bundle), carried along the centreline in its rotation-minimising frame, crossing
//! over to its next segment's slot at a node, and at each end a breakout: two tangent arcs (an
//! S-bend) from the bundle's start to its pin, ending along the connector's exit and on into the
//! pin by its insertion. And the tube round any polyline, as a mesh.

use crate::{Network, Routed};
use forge_math::{Point3, UnitVec3, Vec3, dmath};
use forge_ops::sweep::PathSeg;

/// Arcs are sampled no coarser than this (radians).
const ARC_STEP: f64 = 10.0 * dmath::PI / 180.0;
const TINY: f64 = 1e-9;
/// Lines are sampled no coarser than this (mm), so a wire can move between slots along one.
const LINE_STEP: f64 = 4.0;

/// Circles of radii `r` packed tight about the origin, each placed in turn (larger first, ties by
/// index) where it least grows the enclosing circle: at the middle, against one placed circle at
/// one of 12 angles, or against two. Deterministic; centred on the packing's box at the end.
/// Returns each circle's centre, by index, and the enclosing diameter.
pub fn pack(r: &[f64]) -> (Vec<[f64; 2]>, f64) {
    let mut order: Vec<usize> = (0..r.len()).collect();
    order.sort_by(|&a, &b| r[b].total_cmp(&r[a]).then(a.cmp(&b)));
    let mut at = vec![[0.0f64; 2]; r.len()];
    let mut placed: Vec<usize> = vec![];
    let free = |c: [f64; 2], ri: f64, placed: &[usize], at: &[[f64; 2]]| {
        placed.iter().all(|&j| {
            let (dx, dy) = (c[0] - at[j][0], c[1] - at[j][1]);
            (dx * dx + dy * dy).sqrt() >= r[j] + ri - 1e-9
        })
    };
    for &i in &order {
        let ri = r[i];
        // every candidate place, then the best that is free (most are tried in order and the
        // first free one taken: far cheaper than testing them all)
        let mut cands: Vec<[f64; 2]> = vec![];
        if placed.is_empty() {
            cands.push([0.0, 0.0]);
        }
        for &j in &placed {
            for k in 0..12 {
                let (s, c) = dmath::sin_cos(k as f64 * dmath::PI / 6.0);
                let d = r[j] + ri;
                cands.push([at[j][0] + d * c, at[j][1] + d * s]);
            }
        }
        for (a, &j) in placed.iter().enumerate() {
            for &k in &placed[a + 1..] {
                let (pj, pk) = (at[j], at[k]);
                let (rj, rk) = (r[j] + ri, r[k] + ri);
                let (dx, dy) = (pk[0] - pj[0], pk[1] - pj[1]);
                let d = (dx * dx + dy * dy).sqrt();
                if d <= TINY || d > rj + rk {
                    continue;
                }
                // the two points rj from pj and rk from pk
                let a_ = (rj * rj - rk * rk + d * d) / (2.0 * d);
                let h = (rj * rj - a_ * a_).max(0.0).sqrt();
                let (mx, my) = (pj[0] + a_ * dx / d, pj[1] + a_ * dy / d);
                cands.push([mx - h * dy / d, my + h * dx / d]);
                cands.push([mx + h * dy / d, my - h * dx / d]);
            }
        }
        // nearest the middle (to a nanometre: equal places stay in the order made), first made
        let near = |c: &[f64; 2]| ((c[0] * c[0] + c[1] * c[1]).sqrt() * 1e6).round() as i64;
        let mut order_c: Vec<(i64, usize)> = cands.iter().enumerate().map(|(x, c)| (near(c), x)).collect();
        order_c.sort_unstable();
        let order_c = order_c.into_iter().map(|x| x.1);
        let best = order_c.map(|x| cands[x]).find(|c| free(*c, ri, &placed, &at));
        at[i] = best.unwrap_or([0.0, 0.0]);
        placed.push(i);
    }
    if r.is_empty() {
        return (vec![], 0.0);
    }
    // centred on the smallest circle round them all, near enough: from their middle, steps
    // toward the one reaching farthest while they shorten that reach, halved when they don't
    let worst = |x: [f64; 2]| {
        (0..r.len())
            .map(|i| {
                (
                    ((at[i][0] - x[0]).powi(2) + (at[i][1] - x[1]).powi(2)).sqrt() + r[i],
                    i,
                )
            })
            .fold(
                (f64::NEG_INFINITY, 0),
                |a, b| if b.0 > a.0 + 1e-15 { b } else { a },
            )
    };
    let n = r.len() as f64;
    let mut x = [
        at.iter().map(|c| c[0]).sum::<f64>() / n,
        at.iter().map(|c| c[1]).sum::<f64>() / n,
    ];
    let mut step = worst(x).0 / 2.0;
    while step > 1e-5 {
        let (now, far) = worst(x);
        let (dx, dy) = (at[far][0] - x[0], at[far][1] - x[1]);
        let d = (dx * dx + dy * dy).sqrt();
        if d <= TINY {
            break;
        }
        let y = [x[0] + dx / d * step, x[1] + dy / d * step];
        if worst(y).0 < now - 1e-12 {
            x = y;
        } else {
            step /= 2.0;
        }
    }
    let mut reach = 0.0f64;
    for (i, c) in at.iter_mut().enumerate() {
        c[0] -= x[0];
        c[1] -= x[1];
        reach = reach.max((c[0] * c[0] + c[1] * c[1]).sqrt() + r[i]);
    }
    (at, 2.0 * reach)
}

/// Points along a curve with their unit tangents.
pub type Samples = Vec<(Point3, UnitVec3)>;

/// A centreline's points and unit tangents: line ends, arcs every `ARC_STEP` at most.
pub fn sample(pieces: &[PathSeg]) -> Vec<(Point3, UnitVec3)> {
    let mut out: Vec<(Point3, UnitVec3)> = vec![];
    let push = |p: Point3, t: UnitVec3, out: &mut Vec<(Point3, UnitVec3)>| {
        if out.last().is_none_or(|q| q.0.distance(p) > TINY) {
            out.push((p, t));
        }
    };
    for piece in pieces {
        match *piece {
            PathSeg::Line { a, b } => {
                if let Some(t) = (b - a).normalize() {
                    let n = ((a.distance(b) / LINE_STEP).ceil() as usize).max(1);
                    for k in 0..=n {
                        push(a + (b - a) * (k as f64 / n as f64), t, &mut out);
                    }
                }
            }
            PathSeg::Arc { circle, start, sweep } => {
                let n = ((sweep.abs() / ARC_STEP).ceil() as usize).max(1);
                let (x, y, o, r) = (
                    circle.frame.x().get(),
                    circle.frame.y().get(),
                    circle.frame.origin,
                    circle.radius,
                );
                for k in 0..=n {
                    let a = start + sweep * k as f64 / n as f64;
                    let (s, c) = dmath::sin_cos(a);
                    let p = o + (x * c + y * s) * r;
                    if let Some(t) = ((y * c - x * s) * sweep.signum()).normalize() {
                        push(p, t, &mut out);
                    }
                }
            }
            PathSeg::Spline(_) => {}
        }
    }
    out
}

/// The rotation-minimising frame's normal along points with tangents, from `r0` (double
/// reflection: Wang, Jüttler, Zheng, Liu 2008; exact on lines and on circles' sample points).
pub fn rmf(pts: &[(Point3, UnitVec3)], r0: UnitVec3) -> Vec<UnitVec3> {
    let mut out = Vec::with_capacity(pts.len());
    let mut r = r0;
    for (i, &(x, t)) in pts.iter().enumerate() {
        if i > 0 {
            let (xp, tp) = pts[i - 1];
            let v1 = x - xp;
            let c1 = v1.dot(v1);
            if c1 > TINY * TINY {
                let rl = r.get() - v1 * (2.0 / c1 * v1.dot(r.get()));
                let tl = tp.get() - v1 * (2.0 / c1 * v1.dot(tp.get()));
                let v2 = t.get() - tl;
                let c2 = v2.dot(v2);
                let rn = if c2 > TINY * TINY {
                    rl - v2 * (2.0 / c2 * v2.dot(rl))
                } else {
                    rl
                };
                // kept square to the tangent against rounding
                r = (rn - t.get() * t.dot(rn)).normalize().unwrap_or(r);
            }
        }
        out.push(r);
    }
    out
}

/// One wire's centreline, pin to pin.
#[derive(Clone, Debug, PartialEq)]
pub struct WirePath {
    pub points: Vec<Point3>,
    pub r: f64,
    /// The breakouts' arc radii, at its from and to ends (None: a straight breakout).
    pub breakout: [Option<f64>; 2],
}

/// Each segment's slots: by wire through it, its centre in the segment's frame, and the diameter
/// the packing needs.
#[derive(Clone, Debug, PartialEq)]
pub struct Slots {
    pub at: Vec<(usize, [f64; 2])>,
    pub d: f64,
}

pub fn slots(net: &Network, routed: &Routed) -> Vec<Slots> {
    routed
        .segments
        .iter()
        .map(|s| {
            let r: Vec<f64> = s
                .wires
                .iter()
                .map(|&w| net.types[net.wires[w].ty].od / 2.0)
                .collect();
            let (at, d) = pack(&r);
            Slots {
                at: s.wires.iter().copied().zip(at).collect(),
                d,
            }
        })
        .collect()
}

/// An S-bend from `a` to `b`, both along `t`: two tangent arcs of one radius, or a line where
/// they are not apart along `t`. The points after `a`, and the radius.
fn s_bend(a: Point3, b: Point3, t: UnitVec3) -> (Vec<Point3>, Option<f64>) {
    let ab = b - a;
    let l = ab.dot(t.get());
    let side = ab - t.get() * l;
    let delta = side.length();
    let Some(e) = side.normalize().filter(|_| delta > 1e-6 && l > 1e-6) else {
        return (vec![b], None);
    };
    let r = (delta * delta + l * l) / (4.0 * delta);
    let phi = 2.0 * dmath::atan(delta / l);
    let n = ((phi / ARC_STEP).ceil() as usize).max(2);
    let mut out = vec![];
    for k in 1..=n {
        let th = phi * k as f64 / n as f64;
        let (s, c) = dmath::sin_cos(th);
        out.push(a + e.get() * (r * (1.0 - c)) + t.get() * (r * s));
    }
    for k in (0..n).rev() {
        let th = phi * k as f64 / n as f64;
        let (s, c) = dmath::sin_cos(th);
        out.push(b - e.get() * (r * (1.0 - c)) - t.get() * (r * s));
    }
    (out, Some(r))
}

/// Every routed wire's own centreline, by wire; how far each goes into its pins (`insert`, at
/// most `max_in`).
pub fn paths(net: &Network, routed: &Routed, slots: &[Slots], max_in: f64) -> Vec<Option<WirePath>> {
    // each segment's centreline and its frame, a to b
    let lines: Vec<(Samples, Vec<UnitVec3>)> = routed
        .segments
        .iter()
        .map(|s| {
            let pts = sample(&s.pieces);
            let r0 = pts
                .first()
                .map_or(UnitVec3::X, |p| forge_math::Frame::from_z(p.0, p.1).x());
            let r = rmf(&pts, r0);
            (pts, r)
        })
        .collect();
    let mut out = vec![];
    for (k, w) in net.wires.iter().enumerate() {
        let Some(o) = &routed.wires[k] else {
            out.push(None);
            continue;
        };
        let rad = net.types[w.ty].od / 2.0;
        // the wire through each segment, in the order it runs: the centreline, its tangents,
        // and the slot's offset at each point (the slot carried in the segment's frame)
        struct Run {
            centre: Vec<Point3>,
            tan: Vec<UnitVec3>,
            off: Vec<Vec3>,
        }
        let mut runs: Vec<Run> = vec![];
        for &(s, fwd) in &o.path {
            let (pts, rs) = &lines[s];
            let Some(&(_, c)) = slots[s].at.iter().find(|x| x.0 == k) else {
                continue;
            };
            let mut run = Run {
                centre: vec![],
                tan: vec![],
                off: vec![],
            };
            for ((p, t), r) in pts.iter().zip(rs) {
                run.centre.push(*p);
                run.tan.push(if fwd { *t } else { -*t });
                run.off.push(r.get() * c[0] + t.cross(r.get()) * c[1]);
            }
            if !fwd {
                run.centre.reverse();
                run.tan.reverse();
                run.off.reverse();
            }
            if !run.centre.is_empty() {
                runs.push(run);
            }
        }
        if runs.is_empty() {
            // both ends on one connector's node: pin, node, pin
            let c = &net.connectors[w.from.connector];
            let d = &net.connectors[w.to.connector];
            out.push(Some(WirePath {
                points: vec![c.pins[w.from.pin].1, net.nodes[c.exit].at, d.pins[w.to.pin].1],
                r: rad,
                breakout: [None, None],
            }));
            continue;
        }
        // at each node between two runs the wire moves from its slot in the one to its slot in
        // the next over `reach` each side, smoothly, square to the centreline
        for i in 1..runs.len() {
            let reach = 2.0 * (routed.segments[o.path[i - 1].0].d).max(routed.segments[o.path[i].0].d);
            let (oa, ob) = (
                *runs[i - 1].off.last().expect("points"),
                *runs[i].off.first().expect("points"),
            );
            let blend = |x: f64| x * x * (3.0 - 2.0 * x);
            // along the run before, from the node back; then the run after, from the node on
            let mut back = 0.0;
            let a = &mut runs[i - 1];
            for j in (0..a.centre.len()).rev() {
                if j + 1 < a.centre.len() {
                    back += a.centre[j].distance(a.centre[j + 1]);
                }
                if back >= reach {
                    break;
                }
                let s_ = blend(0.5 - 0.5 * back / reach);
                let o_ = oa * (1.0 - s_) + ob * s_;
                a.off[j] = o_ - a.tan[j].get() * a.tan[j].dot(o_);
            }
            let mut on = 0.0;
            let b_ = &mut runs[i];
            for j in 0..b_.centre.len() {
                if j > 0 {
                    on += b_.centre[j].distance(b_.centre[j - 1]);
                }
                if on >= reach {
                    break;
                }
                let s_ = blend(0.5 + 0.5 * on / reach);
                let o_ = oa * (1.0 - s_) + ob * s_;
                b_.off[j] = o_ - b_.tan[j].get() * b_.tan[j].dot(o_);
            }
        }
        // a run's wire points; the node between two runs once
        let wire_of =
            |r: &Run| -> Vec<Point3> { r.centre.iter().zip(&r.off).map(|(c, o)| *c + *o).collect() };
        let ends = [w.from, w.to];
        let pin = |e: crate::End| net.connectors[e.connector].pins[e.pin].1;
        let ins = |e: crate::End| net.connectors[e.connector].insert.min(max_in);
        let (p, q) = (pin(ends[0]), pin(ends[1]));
        let t_start = runs[0].tan[0];
        let t_end = *runs[runs.len() - 1].tan.last().expect("points");
        let mut pts = vec![p - t_start.get() * ins(ends[0]), p];
        let first = wire_of(&runs[0]);
        let (bend0, r0) = s_bend(p, first[0], t_start);
        pts.extend(bend0);
        for (i, run) in runs.iter().enumerate() {
            pts.extend(wire_of(run).into_iter().skip(usize::from(i == 0)));
        }
        let end = *pts.last().expect("points");
        let (bend1, r1) = s_bend(end, q, t_end);
        pts.extend(bend1);
        pts.push(q + t_end.get() * ins(ends[1]));
        pts.dedup_by(|a, b| a.distance(*b) <= TINY);
        out.push(Some(WirePath {
            points: pts,
            r: rad,
            breakout: [r0, r1],
        }));
    }
    out
}

/// A tube round a polyline: rings of `sides` vertices square to it (in its rotation-minimising
/// frame, so it does not twist), flat caps. Positions, normals and triangles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TubeMesh {
    pub positions: Vec<Point3>,
    pub normals: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

pub fn tube_mesh(points: &[Point3], r: f64, sides: usize) -> TubeMesh {
    let mut m = TubeMesh::default();
    if points.len() < 2 || sides < 3 {
        return m;
    }
    let tan: Vec<(Point3, UnitVec3)> = (0..points.len())
        .filter_map(|i| {
            let a = points[i.saturating_sub(1)];
            let b = points[(i + 1).min(points.len() - 1)];
            Some((points[i], (b - a).normalize()?))
        })
        .collect();
    if tan.len() < 2 {
        return m;
    }
    let r0 = forge_math::Frame::from_z(tan[0].0, tan[0].1).x();
    let frames = rmf(&tan, r0);
    let ring: Vec<(f64, f64)> = (0..sides)
        .map(|k| dmath::sin_cos(dmath::TAU * k as f64 / sides as f64))
        .collect();
    for ((p, t), u) in tan.iter().zip(&frames) {
        let v = t.cross(u.get());
        for &(s, c) in &ring {
            let n = u.get() * c + v * s;
            m.positions.push(*p + n * r);
            m.normals.push(n);
        }
    }
    let n = tan.len() as u32;
    let s = sides as u32;
    for i in 0..n - 1 {
        for k in 0..s {
            let (a, b) = (i * s + k, i * s + (k + 1) % s);
            let (c, d) = (a + s, b + s);
            m.triangles.push([a, b, d]);
            m.triangles.push([a, d, c]);
        }
    }
    // the caps: a centre each
    for (i, sign) in [(0u32, -1.0), (n - 1, 1.0)] {
        let (p, t) = tan[i as usize];
        let centre = m.positions.len() as u32;
        m.positions.push(p);
        m.normals.push(t.get() * sign);
        for k in 0..s {
            let (a, b) = (i * s + k, i * s + (k + 1) % s);
            if sign < 0.0 {
                m.triangles.push([centre, b, a]);
            } else {
                m.triangles.push([centre, a, b]);
            }
        }
    }
    m
}
