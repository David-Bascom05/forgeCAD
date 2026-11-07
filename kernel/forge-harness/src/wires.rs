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

/// A wire's place in its segment's cross-section, in the segment's frame; a grouped wire's turns
/// about its group's middle once every lay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub wire: usize,
    pub at: [f64; 2],
    pub spin: Option<([f64; 2], f64)>,
}

/// Each segment's slots, the diameter the packing needs, and how the packing rolls about the
/// centreline from one end to the other (radians, from end a to end b): a rigid turn, so no two
/// wires pass through each other on the way.
#[derive(Clone, Debug, PartialEq)]
pub struct Slots {
    pub at: Vec<Slot>,
    pub d: f64,
    pub roll: f64,
}

/// What a segment's packing is made of: a wire alone, or a twisted group's wires (each with its
/// place in the group), its radius, and the group's lay.
type Item = (Vec<(usize, [f64; 2])>, f64, Option<f64>);

/// A segment's centreline samples and its frame along them, a to b.
fn frames(routed: &Routed) -> Vec<(Samples, Vec<UnitVec3>)> {
    routed
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
        .collect()
}

/// The slots, segment by segment outward from the connector the most wires leave: each packing
/// placed to match where its wires come in (their pins, or their slots in the segment before),
/// then rolled along its run to face where they go out (their pins, or the way their next
/// segments turn off).
pub fn slots(net: &Network, routed: &Routed) -> Vec<Slots> {
    let fr = frames(routed);
    let n_seg = net.segments.len();
    let mut out: Vec<Option<Slots>> = vec![None; n_seg];
    // the root: the connector exit the most wires leave
    let count = |c: usize| {
        net.wires
            .iter()
            .filter(|w| w.from.connector == c || w.to.connector == c)
            .count()
    };
    let root = (0..net.connectors.len()).max_by(|&a, &b| count(a).cmp(&count(b)).then(b.cmp(&a)));
    let Some(root) = root else {
        return vec![
            Slots {
                at: vec![],
                d: 0.0,
                roll: 0.0
            };
            n_seg
        ];
    };
    // breadth first over segments: each with the end it is reached by
    let mut order: Vec<(usize, usize)> = vec![];
    let mut seen_node = vec![false; net.nodes.len()];
    let mut queue = std::collections::VecDeque::from([net.connectors[root].exit]);
    seen_node[net.connectors[root].exit] = true;
    let mut seen_seg = vec![false; n_seg];
    while let Some(nd) = queue.pop_front() {
        for (k, sg) in net.segments.iter().enumerate() {
            if seen_seg[k] || (sg.a != nd && sg.b != nd) {
                continue;
            }
            seen_seg[k] = true;
            let (near, far) = if sg.a == nd { (0, sg.b) } else { (1, sg.a) };
            order.push((k, near));
            if !seen_node[far] {
                seen_node[far] = true;
                queue.push_back(far);
            }
        }
    }
    order.extend(seen_seg.iter().enumerate().filter(|x| !x.1).map(|x| (x.0, 0)));
    // a segment's frame at an end: its point, tangent along a to b, x
    let end_frame = |k: usize, e: usize| {
        let (pts, rs) = &fr[k];
        if e == 0 {
            (pts[0].0, pts[0].1, rs[0])
        } else {
            (pts[pts.len() - 1].0, pts[pts.len() - 1].1, rs[rs.len() - 1])
        }
    };
    let end_node = |k: usize, e: usize| {
        if e == 0 {
            net.segments[k].a
        } else {
            net.segments[k].b
        }
    };
    let rot = |c: [f64; 2], a: f64| {
        let (sn, cs) = dmath::sin_cos(a);
        [c[0] * cs - c[1] * sn, c[0] * sn + c[1] * cs]
    };
    for (k, near) in order {
        let seg = &routed.segments[k];
        if fr[k].0.is_empty() || seg.wires.is_empty() {
            out[k] = Some(Slots {
                at: vec![],
                d: 0.0,
                roll: 0.0,
            });
            continue;
        }
        let far = 1 - near;
        // where each wire is wanted at an end, in this segment's frame there
        let want = |e: usize, out: &[Option<Slots>]| -> std::collections::BTreeMap<usize, [f64; 2]> {
            let node = end_node(k, e);
            let (at, t, u) = end_frame(k, e);
            let v = t.cross(u.get());
            let flat = |d: Vec3| [d.dot(u.get()), d.dot(v)];
            let mut m = std::collections::BTreeMap::new();
            if let Some(c) = net.connectors.iter().position(|c| c.exit == node) {
                for &w in &seg.wires {
                    let wr = &net.wires[w];
                    if let Some(pe) = [wr.from, wr.to].into_iter().find(|x| x.connector == c) {
                        m.insert(w, flat(net.connectors[c].pins[pe.pin].1 - at));
                    }
                }
                return m;
            }
            let rad = bundle_radius(net, &seg.wires).max(0.1);
            let mut turns: Vec<(usize, Option<[f64; 2]>)> = vec![];
            for &w in &seg.wires {
                let Some(o) = &routed.wires[w] else { continue };
                let Some(i) = o.path.iter().position(|x| x.0 == k) else {
                    continue;
                };
                let fwd = o.path[i].1;
                let next = if (e == 1) == fwd {
                    o.path.get(i + 1)
                } else {
                    i.checked_sub(1).and_then(|j| o.path.get(j))
                };
                let Some(&(n, _)) = next else { continue };
                let ne = usize::from(net.segments[n].a != node);
                // its slot in a segment placed already, as it meets this one
                if let Some(Some(sl)) = out.get(n)
                    && let Some(slot) = sl.at.iter().find(|x| x.wire == w)
                {
                    let (_, nt, nu) = end_frame(n, ne);
                    let nv = nt.cross(nu.get());
                    let c = rot(slot.at, if ne == 0 { 0.0 } else { sl.roll });
                    m.insert(w, flat(nu.get() * c[0] + nv * c[1]));
                    continue;
                }
                // else the way the next segment turns off, past its straight
                let lead = routed.segments[n].straight[ne];
                let np = &fr[n].0;
                let mut acc = 0.0;
                let mut prev: Option<Point3> = None;
                let mut aim = None;
                let it: Box<dyn Iterator<Item = &(Point3, UnitVec3)>> = if ne == 0 {
                    Box::new(np.iter())
                } else {
                    Box::new(np.iter().rev())
                };
                for q in it {
                    if let Some(pq) = prev {
                        acc += pq.distance(q.0);
                    }
                    prev = Some(q.0);
                    if acc > lead + 2.0 * rad + 5.0 {
                        aim = Some(q.0);
                        break;
                    }
                }
                let dir = aim.map(|a_| flat(a_ - at)).and_then(|f| {
                    let l = (f[0] * f[0] + f[1] * f[1]).sqrt();
                    (l > 1e-6).then(|| [f[0] / l * rad, f[1] / l * rad])
                });
                turns.push((w, dir));
            }
            // a wire running straight on keeps clear of those turning off
            let off: Vec<[f64; 2]> = turns.iter().filter_map(|x| x.1).collect();
            let away = if off.is_empty() {
                [0.0, 0.0]
            } else {
                let n = off.len() as f64;
                let c = [
                    off.iter().map(|x| x[0]).sum::<f64>() / n,
                    off.iter().map(|x| x[1]).sum::<f64>() / n,
                ];
                let l = (c[0] * c[0] + c[1] * c[1]).sqrt().max(1e-9);
                [-c[0] / l * rad, -c[1] / l * rad]
            };
            for (w, d) in turns {
                m.insert(w, d.unwrap_or(away));
            }
            m
        };
        let (wn, wf) = (want(near, &out), want(far, &out));
        // items: a group's wires packed together, or a wire alone
        let mut items: Vec<Item> = vec![];
        let mut done = vec![];
        for &w in &seg.wires {
            let rad = |x: usize| net.types[net.wires[x].ty].od / 2.0;
            match net.groups.iter().position(|g| g.wires.contains(&w)) {
                Some(g) if done.contains(&g) => {}
                Some(g) => {
                    done.push(g);
                    let ws: Vec<usize> = net.groups[g]
                        .wires
                        .iter()
                        .copied()
                        .filter(|x| seg.wires.contains(x))
                        .collect();
                    let (at, d) = pack(&ws.iter().map(|&x| rad(x)).collect::<Vec<_>>());
                    items.push((ws.into_iter().zip(at).collect(), d / 2.0, Some(net.groups[g].lay)));
                }
                None => items.push((vec![(w, [0.0, 0.0])], rad(w), None)),
            }
        }
        let mean = |m: &std::collections::BTreeMap<usize, [f64; 2]>, it: &[(usize, [f64; 2])]| {
            let got: Vec<[f64; 2]> = it.iter().filter_map(|x| m.get(&x.0).copied()).collect();
            (!got.is_empty()).then(|| {
                let n = got.len() as f64;
                [
                    got.iter().map(|x| x[0]).sum::<f64>() / n,
                    got.iter().map(|x| x[1]).sum::<f64>() / n,
                ]
            })
        };
        let radii: Vec<f64> = items.iter().map(|i| i.1).collect();
        // every item already placed in the segments it comes from (a branch of a bundle): kept
        // where it was, the lot centred and drawn together, so the wires move in as one
        let inherited: Option<Vec<[f64; 2]>> = items.iter().map(|it| mean(&wn, &it.0)).collect();
        let from_pins = net.connectors.iter().any(|c| c.exit == end_node(k, near));
        let (at, d) = match inherited {
            Some(ps) if !from_pins => compact(&ps, &radii),
            _ => pack(&radii),
        };
        // matched at the near end (least squares: moving in, the wires do not cross) ..
        let cost: Vec<Vec<f64>> = items
            .iter()
            .map(|it| {
                let t = mean(&wn, &it.0);
                at.iter()
                    .map(|c| t.map_or(0.0, |t| (c[0] - t[0]).powi(2) + (c[1] - t[1]).powi(2)))
                    .collect()
            })
            .collect();
        let mut place = assign(&cost);
        // .. then the turn at the far end that best faces them where they go (2D Procrustes);
        // and the places chosen again for both ends at once, each end's wants scaled to the
        // bundle, a few times over
        let face = |place: &[usize]| {
            let (mut sx, mut sy) = (0.0, 0.0);
            for (i, it) in items.iter().enumerate() {
                if let Some(t) = mean(&wf, &it.0) {
                    let c = at[place[i]];
                    sx += c[0] * t[0] + c[1] * t[1];
                    sy += c[0] * t[1] - c[1] * t[0];
                }
            }
            if sx.abs() + sy.abs() > 1e-12 {
                dmath::atan2(sy, sx)
            } else {
                0.0
            }
        };
        let mut turn = face(&place);
        {
            let span = |m: &std::collections::BTreeMap<usize, [f64; 2]>| {
                m.values()
                    .map(|t| (t[0] * t[0] + t[1] * t[1]).sqrt())
                    .fold(0.0, f64::max)
                    .max(1e-9)
            };
            let (sn, sf) = (d / 2.0 / span(&wn), d / 2.0 / span(&wf));
            for _ in 0..3 {
                let cost: Vec<Vec<f64>> = items
                    .iter()
                    .map(|it| {
                        let (tn, tf) = (mean(&wn, &it.0), mean(&wf, &it.0));
                        at.iter()
                            .map(|c| {
                                let r_ = rot(*c, turn);
                                tn.map_or(0.0, |t| (c[0] - t[0] * sn).powi(2) + (c[1] - t[1] * sn).powi(2))
                                    + tf.map_or(0.0, |t| {
                                        (r_[0] - t[0] * sf).powi(2) + (r_[1] - t[1] * sf).powi(2)
                                    })
                            })
                            .collect()
                    })
                    .collect();
                place = assign(&cost);
                turn = face(&place);
            }
        }
        let mut slots = vec![];
        for (i, item) in items.iter().enumerate() {
            let c = at[place[i]];
            for &(w, o) in &item.0 {
                slots.push(Slot {
                    wire: w,
                    at: [c[0] + o[0], c[1] + o[1]],
                    spin: item.2.filter(|l| *l > 0.0).map(|l| (c, l)),
                });
            }
        }
        // stored from a to b: placed at a, turned by `roll` at b
        let (slots, roll) = if near == 0 {
            (slots, turn)
        } else {
            let back: Vec<Slot> = slots
                .into_iter()
                .map(|s_| Slot {
                    at: rot(s_.at, -turn),
                    spin: s_.spin.map(|(m, l)| (rot(m, -turn), l)),
                    ..s_
                })
                .collect();
            (back, turn)
        };
        out[k] = Some(Slots { at: slots, d, roll });
    }
    out.into_iter()
        .map(|s_| {
            s_.unwrap_or(Slots {
                at: vec![],
                d: 0.0,
                roll: 0.0,
            })
        })
        .collect()
}

/// Circles where they are, centred on their middle and drawn in toward it, each in turn (nearest
/// first) by small steps while it stays clear of the rest: a branch of a bundle closing up. Each
/// circle's centre, by index, and the enclosing diameter.
pub fn compact(at: &[[f64; 2]], r: &[f64]) -> (Vec<[f64; 2]>, f64) {
    let n = at.len();
    if n == 0 {
        return (vec![], 0.0);
    }
    let m = [
        at.iter().map(|c| c[0]).sum::<f64>() / n as f64,
        at.iter().map(|c| c[1]).sum::<f64>() / n as f64,
    ];
    let mut p: Vec<[f64; 2]> = at.iter().map(|c| [c[0] - m[0], c[1] - m[1]]).collect();
    let clear = |p: &[[f64; 2]], i: usize, c: [f64; 2]| {
        (0..p.len()).all(|j| {
            j == i || ((c[0] - p[j][0]).powi(2) + (c[1] - p[j][1]).powi(2)).sqrt() >= r[i] + r[j] - 1e-9
        })
    };
    for _ in 0..40 {
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            let d = |i: usize| (p[i][0] * p[i][0] + p[i][1] * p[i][1]).sqrt();
            d(a).total_cmp(&d(b)).then(a.cmp(&b))
        });
        let mut moved = false;
        for i in order {
            let d = (p[i][0] * p[i][0] + p[i][1] * p[i][1]).sqrt();
            if d <= 1e-9 {
                continue;
            }
            let step = d.min(r[i] * 0.25);
            let c = [p[i][0] - p[i][0] / d * step, p[i][1] - p[i][1] / d * step];
            if clear(&p, i, c) {
                p[i] = c;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    let reach = (0..n)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]).sqrt() + r[i])
        .fold(0.0, f64::max);
    (p, 2.0 * reach)
}

/// Half the packing diameter of a segment's wires, roughly: its rule's.
fn bundle_radius(net: &Network, wires: &[usize]) -> f64 {
    crate::bundle_d(&crate::item_ods(net, wires), 1.5) / 2.0
}

/// The least-cost assignment of rows to columns of a square cost matrix (Hungarian, O(n^3));
/// row i's column.
pub fn assign(cost: &[Vec<f64>]) -> Vec<usize> {
    let n = cost.len();
    if n == 0 {
        return vec![];
    }
    // potentials and the matching, 1-based with a dummy column 0
    let (mut u, mut v) = (vec![0.0; n + 1], vec![0.0; n + 1]);
    let mut p = vec![0usize; n + 1];
    let mut way = vec![0usize; n + 1];
    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0;
        let mut minv = vec![f64::INFINITY; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = f64::INFINITY;
            let mut j1 = 0;
            for j in 1..=n {
                if !used[j] {
                    let cur = cost[i0 - 1][j - 1] - u[i0] - v[j];
                    if cur < minv[j] {
                        minv[j] = cur;
                        way[j] = j0;
                    }
                    if minv[j] < delta {
                        delta = minv[j];
                        j1 = j;
                    }
                }
            }
            for j in 0..=n {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut out = vec![0; n];
    for j in 1..=n {
        if p[j] > 0 {
            out[p[j] - 1] = j - 1;
        }
    }
    out
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
            let Some(slot) = slots[s].at.iter().find(|x| x.wire == k) else {
                continue;
            };
            let mut run = Run {
                centre: vec![],
                tan: vec![],
                off: vec![],
            };
            let total: f64 = pts
                .windows(2)
                .map(|w_| w_[0].0.distance(w_[1].0))
                .sum::<f64>()
                .max(1e-9);
            let roll = slots[s].roll;
            let mut along = 0.0;
            for (i, ((p, t), r)) in pts.iter().zip(rs).enumerate() {
                if i > 0 {
                    along += p.distance(pts[i - 1].0);
                }
                // a grouped wire turned about its group's middle by how far along it is
                let c = match slot.spin {
                    Some((m, lay)) => {
                        let (sn, cs) = dmath::sin_cos(dmath::TAU * along / lay);
                        let (x, y) = (slot.at[0] - m[0], slot.at[1] - m[1]);
                        [m[0] + x * cs - y * sn, m[1] + x * sn + y * cs]
                    }
                    None => slot.at,
                };
                // the packing's turn so far along
                let (sn, cs) = dmath::sin_cos(roll * along / total);
                let c = [c[0] * cs - c[1] * sn, c[0] * sn + c[1] * cs];
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
        // the next, smoothly, square to the centreline: over `reach` each side; or, where the
        // next leaves along a straight it shares with others at the node (a branch peeling off),
        // holding its slot down that straight and moving after it (and the other way round)
        let shared = |seg: usize, node: usize| {
            let away = |x: usize| {
                let (s_, o_) = (&net.segments[x], &routed.segments[x]);
                if s_.a == node {
                    o_.tangents[0]
                } else if s_.b == node {
                    o_.tangents[1].map(|t| -t)
                } else {
                    None
                }
            };
            let end = |x: usize| usize::from(net.segments[x].a != node);
            let mine = away(seg)?;
            // how far it runs alongside another leaving the same way: the shorter straight
            (0..net.segments.len())
                .filter(|&x| x != seg && away(x).is_some_and(|t| t.dot(mine) > 0.999))
                .map(|x| routed.segments[x].straight[end(x)])
                .reduce(f64::max)
                .map(|other| other.min(routed.segments[seg].straight[end(seg)]))
        };
        for i in 1..runs.len() {
            let (sa, fa) = o.path[i - 1];
            let (sb, _) = o.path[i];
            let node = if fa {
                net.segments[sa].b
            } else {
                net.segments[sa].a
            };
            let reach = 2.0 * (routed.segments[sa].d).max(routed.segments[sb].d);
            let hold_b = shared(sb, node);
            let hold_a = if hold_b.is_none() { shared(sa, node) } else { None };
            let (oa, ob) = (
                *runs[i - 1].off.last().expect("points"),
                *runs[i].off.first().expect("points"),
            );
            let blend = |x: f64| x * x * (3.0 - 2.0 * x);
            // where along the change a point is (0: the old slot, 1: the new), by its distance
            // back from the node on the run before (negative) or on past it (positive)
            let s_at = |x: f64| -> f64 {
                if let Some(h) = hold_b {
                    blend(((x - h) / (2.0 * reach)).clamp(0.0, 1.0))
                } else if let Some(h) = hold_a {
                    blend(((x + h + 2.0 * reach) / (2.0 * reach)).clamp(0.0, 1.0))
                } else {
                    blend((0.5 + 0.5 * x / reach).clamp(0.0, 1.0))
                }
            };
            let mut back = 0.0;
            let a = &mut runs[i - 1];
            for j in (0..a.centre.len()).rev() {
                if j + 1 < a.centre.len() {
                    back += a.centre[j].distance(a.centre[j + 1]);
                }
                let s_ = s_at(-back);
                if s_ <= 0.0 && back > hold_a.unwrap_or(0.0) + 2.0 * reach {
                    break;
                }
                let o_ = a.off[j] * (1.0 - s_) + ob * s_;
                a.off[j] = o_ - a.tan[j].get() * a.tan[j].dot(o_);
            }
            let mut on = 0.0;
            let b_ = &mut runs[i];
            for j in 0..b_.centre.len() {
                if j > 0 {
                    on += b_.centre[j].distance(b_.centre[j - 1]);
                }
                let s_ = s_at(on);
                if s_ >= 1.0 && on > hold_b.unwrap_or(0.0) + 2.0 * reach {
                    break;
                }
                let o_ = oa * (1.0 - s_) + b_.off[j] * s_;
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

/// A cable tie round a bundle: its middle, the bundle's direction and the diameter it holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tie {
    pub at: Point3,
    pub axis: UnitVec3,
    pub d: f64,
}

/// A tie's width along the bundle and its band's thickness (mm).
pub const TIE_W: f64 = 2.5;
pub const TIE_T: f64 = 0.5;

/// Cable ties on every bare segment: past where the wires settle at each end (beside a branch,
/// at a breakout), then every `pitch` between; none where `pitch` is not positive or the segment
/// is sleeved.
pub fn ties(net: &Network, routed: &Routed, slots: &[Slots], pitch: f64) -> Vec<Tie> {
    let mut out = vec![];
    if pitch <= 0.0 {
        return out;
    }
    let exit = |n: usize| net.connectors.iter().any(|c| c.exit == n);
    // how far a node's wires take to settle: as the slot blend reaches
    let settle = |n: usize| {
        let d = net
            .segments
            .iter()
            .zip(&routed.segments)
            .filter(|(s, _)| s.a == n || s.b == n)
            .map(|(_, o)| o.d)
            .fold(0.0, f64::max);
        if exit(n) { 0.75 * d + 2.0 } else { 2.0 * d + 3.0 }
    };
    for (k, (s, o)) in net.segments.iter().zip(&routed.segments).enumerate() {
        if o.wires.is_empty() || s.wall > 0.0 {
            continue;
        }
        let pts = sample(&o.pieces);
        let mut acc = vec![0.0];
        for w in pts.windows(2) {
            acc.push(acc.last().expect("a length") + w[0].0.distance(w[1].0));
        }
        let len = *acc.last().expect("a length");
        let (s0, s1) = (settle(s.a), len - settle(s.b));
        let mut at = vec![];
        if s1 - s0 < TIE_W {
            if len > 2.0 * TIE_W {
                at.push(len / 2.0);
            }
        } else {
            at.push(s0);
            let mut x = s0 + pitch;
            while x < s1 - pitch / 2.0 {
                at.push(x);
                x += pitch;
            }
            at.push(s1);
        }
        let d = slots[k].d + 2.0 * TIE_T;
        for x in at {
            let i = acc.partition_point(|a| *a < x).clamp(1, pts.len() - 1);
            let f = ((x - acc[i - 1]) / (acc[i] - acc[i - 1]).max(TINY)).clamp(0.0, 1.0);
            let (a, b) = (pts[i - 1], pts[i]);
            out.push(Tie {
                at: a.0 + (b.0 - a.0) * f,
                axis: (a.1.get() * (1.0 - f) + b.1.get() * f).normalize().unwrap_or(a.1),
                d,
            });
        }
    }
    out
}

/// Meshes joined into one.
pub fn merge(ms: Vec<TubeMesh>) -> TubeMesh {
    let mut out = TubeMesh::default();
    for m in ms {
        let base = out.positions.len() as u32;
        out.positions.extend(m.positions);
        out.normals.extend(m.normals);
        out.triangles
            .extend(m.triangles.into_iter().map(|t| t.map(|v| v + base)));
    }
    out
}

/// The ties as bands round their bundles, one mesh.
pub fn ties_mesh(ties: &[Tie]) -> TubeMesh {
    merge(
        ties.iter()
            .map(|t| {
                let h = t.axis.get() * (TIE_W / 2.0);
                tube_mesh(&[t.at - h, t.at + h], t.d / 2.0, 24)
            })
            .collect(),
    )
}

/// A box from its middle, its axes (unit) and half sizes, flat-shaded.
fn block(c: Point3, axes: [Vec3; 3], half: [f64; 3]) -> TubeMesh {
    let mut m = TubeMesh::default();
    for k in 0..3 {
        for sign in [-1.0, 1.0] {
            let n = axes[k] * sign;
            let (u, v) = (axes[(k + 1) % 3], axes[(k + 2) % 3]);
            let (hu, hv) = (half[(k + 1) % 3], half[(k + 2) % 3]);
            let o = c + n * half[k];
            let base = m.positions.len() as u32;
            for (a, b) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                m.positions.push(o + u * (a * hu) + v * (b * hv));
                m.normals.push(n);
            }
            let quad = if sign > 0.0 {
                [[0, 1, 2], [0, 2, 3]]
            } else {
                [[0, 2, 1], [0, 3, 2]]
            };
            m.triangles.extend(quad.map(|t| t.map(|x| base + x)));
        }
    }
    m
}

/// A clip: a band round its bundle and a saddle down to where it is seated.
pub fn clip_mesh(c: &crate::ClipOut) -> TubeMesh {
    let w = 6.0;
    let r = c.d / 2.0 + crate::CLIP_WALL;
    let h = c.axis.get() * (w / 2.0);
    let band = tube_mesh(&[c.at - h, c.at + h], r, 32);
    // the saddle: from under the bundle to the seat, as wide as the band
    let foot = c.base;
    let top = c.at - c.seat.get() * (c.d / 2.0);
    let side = c
        .seat
        .cross(c.axis.get())
        .normalize()
        .map_or(Vec3::X, |u| u.get());
    let mid = foot + (top - foot) * 0.5;
    let tall = foot.distance(top) / 2.0;
    let saddle = block(
        mid,
        [c.axis.get(), side, c.seat.get()],
        [w / 2.0, r * 0.8, tall.max(0.1)],
    );
    let base = block(
        foot + c.seat.get() * (crate::CLIP_WALL / 2.0),
        [c.axis.get(), side, c.seat.get()],
        [w / 2.0, r * 1.3, crate::CLIP_WALL / 2.0],
    );
    merge(vec![band, saddle, base])
}

/// Where two wires pass through each other: each pair once, where their tubes first overlap by
/// more than `tol` (mm). Tubes are the paths' polylines swept by the radii.
pub fn clashes(paths: &[Option<WirePath>], tol: f64) -> Vec<(usize, usize, Point3)> {
    struct Piece {
        w: usize,
        a: Point3,
        b: Point3,
        r: f64,
        lo: f64,
        hi: f64,
    }
    let mut pieces: Vec<Piece> = vec![];
    for (w, p) in paths.iter().enumerate() {
        let Some(p) = p else { continue };
        for s in p.points.windows(2) {
            pieces.push(Piece {
                w,
                a: s[0],
                b: s[1],
                r: p.r,
                lo: s[0].x.min(s[1].x) - p.r,
                hi: s[0].x.max(s[1].x) + p.r,
            });
        }
    }
    pieces.sort_by(|a, b| a.lo.total_cmp(&b.lo));
    let mut out: Vec<(usize, usize, Point3)> = vec![];
    for i in 0..pieces.len() {
        let p = &pieces[i];
        for q in &pieces[i + 1..] {
            if q.lo > p.hi {
                break;
            }
            if q.w == p.w || out.iter().any(|c| (c.0, c.1) == (p.w.min(q.w), p.w.max(q.w))) {
                continue;
            }
            let (d, at) = seg_seg(p.a, p.b, q.a, q.b);
            if d < p.r + q.r - tol {
                out.push((p.w.min(q.w), p.w.max(q.w), at));
            }
        }
    }
    out.sort_by_key(|c| (c.0, c.1));
    out
}

/// The least distance between segments ab and cd, and the middle of the two nearest points.
fn seg_seg(a: Point3, b: Point3, c: Point3, d: Point3) -> (f64, Point3) {
    let (u, v, w) = (b - a, d - c, a - c);
    let (aa, bb, cc, dd, ee) = (u.dot(u), u.dot(v), v.dot(v), u.dot(w), v.dot(w));
    let den = aa * cc - bb * bb;
    let clamp = |x: f64| x.clamp(0.0, 1.0);
    let mut s = if den > 1e-12 {
        clamp((bb * ee - cc * dd) / den)
    } else {
        0.0
    };
    let mut t = if cc > 1e-12 { (bb * s + ee) / cc } else { 0.0 };
    if t < 0.0 {
        t = 0.0;
        s = if aa > 1e-12 { clamp(-dd / aa) } else { 0.0 };
    } else if t > 1.0 {
        t = 1.0;
        s = if aa > 1e-12 { clamp((bb - dd) / aa) } else { 0.0 };
    }
    let (p, q) = (a + u * s, c + v * t);
    (p.distance(q), p + (q - p) * 0.5)
}

/// A coloured stripe on a wire's tube: a band just proud of it, `half` radians either side of an
/// angle that starts at `phase` and turns once every `pitch` along the wire (a helix; a pitch of 0
/// runs it straight along).
pub fn stripe_mesh(points: &[Point3], r: f64, phase: f64, half: f64, pitch: f64) -> TubeMesh {
    let mut m = TubeMesh::default();
    // a helix turns between points: points no further apart than a tenth of a turn's length
    let step = if pitch > 0.0 { pitch / 36.0 } else { f64::INFINITY };
    let mut dense: Vec<Point3> = vec![];
    for w in points.windows(2) {
        let n = ((w[0].distance(w[1]) / step).ceil() as usize).max(1);
        for k in 0..n {
            dense.push(w[0] + (w[1] - w[0]) * (k as f64 / n as f64));
        }
    }
    dense.extend(points.last().copied());
    let points = &dense[..];
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
    let frames = rmf(&tan, forge_math::Frame::from_z(tan[0].0, tan[0].1).x());
    const ACROSS: usize = 4;
    // clear of the tube's facets (ten sides: their middles are 5 % in)
    let rr = r * 1.06;
    let mut along = 0.0;
    for (i, ((p, t), u)) in tan.iter().zip(&frames).enumerate() {
        if i > 0 {
            along += p.distance(tan[i - 1].0);
        }
        let mid = phase
            + if pitch > 0.0 {
                dmath::TAU * along / pitch
            } else {
                0.0
            };
        let v = t.cross(u.get());
        for k in 0..=ACROSS {
            let a = mid - half + 2.0 * half * k as f64 / ACROSS as f64;
            let (s, c) = dmath::sin_cos(a);
            let n = u.get() * c + v * s;
            m.positions.push(*p + n * rr);
            m.normals.push(n);
        }
    }
    let w = (ACROSS + 1) as u32;
    for i in 0..tan.len() as u32 - 1 {
        for k in 0..ACROSS as u32 {
            let (a, b) = (i * w + k, i * w + k + 1);
            let (c, d) = (a + w, b + w);
            m.triangles.push([a, b, d]);
            m.triangles.push([a, d, c]);
        }
    }
    m
}
