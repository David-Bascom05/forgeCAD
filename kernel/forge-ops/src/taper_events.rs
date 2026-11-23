//! The first event of a tapered extrude, exact from the carriers (`taper.rs`).
use crate::profile::{Profile, Segment};
use crate::taper::{ends_dir, unit};
use forge_math::{Point2, Vec2, dmath};

/// The kind of a taper's first event (research/sweep-spec DESIGN.md section 2.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Event {
    /// A side's length or an arc's radius reaches 0: slabs go on without it.
    Edge,
    /// A corner reaches a side it does not end, or two loops touch: the walls cross.
    Cross,
}

/// A loop whose every joint moves straight as the profile offsets: between lines, or where
/// neighbours are tangent (the joint moves square to both). Joint `i`, where segment `i` starts,
/// is `at[i] + vel[i] δ` at offset `δ` the way the taper goes; an arc keeps its angles, its radius
/// `r + dr δ`.
pub(crate) struct Lin<'a> {
    segs: &'a [(Segment, u64)],
    at: Vec<Point2>,
    vel: Vec<Vec2>,
}

impl Lin<'_> {
    /// Err(false) where a joint does not move straight (an arc meeting a neighbour at an angle),
    /// Err(true) where two neighbouring lines run back along each other: their carriers no longer
    /// meet (event J).
    fn new(segs: &[(Segment, u64)], sign: f64) -> Result<Lin<'_>, bool> {
        let n = segs.len();
        if let [(Segment::Circle { .. }, _)] = segs[..] {
            return Ok(Lin {
                segs,
                at: vec![segs[0].0.start()],
                vel: vec![Vec2::new(0.0, 0.0)],
            });
        }
        let mut at = Vec::with_capacity(n);
        let mut vel = Vec::with_capacity(n);
        for i in 0..n {
            let (a, b) = (&cyc(&segs, i + n - 1).0, &cyc(&segs, i).0);
            let (ta, tb) = (unit(ends_dir(a).1), unit(ends_dir(b).0));
            let c = ta.x * tb.y - ta.y * tb.x;
            let w = if c.abs() < 1e-7 && ta.dot(tb) > 0.0 {
                unit(ta + tb).perp()
            } else if matches!((a, b), (Segment::Line { .. }, Segment::Line { .. })) {
                // the meet of the two carriers, each moved 1 along its left normal
                let (na, nb) = (ta.perp(), tb.perp());
                let den = 1.0 + na.dot(nb);
                if den <= 1e-12 {
                    return Err(true);
                }
                (na + nb) * (1.0 / den)
            } else {
                return Err(false);
            };
            at.push(b.start());
            vel.push(w * sign);
        }
        Ok(Lin { segs, at, vel })
    }

    fn joint(&self, i: usize, d: f64) -> Point2 {
        *cyc(&self.at, i) + *cyc(&self.vel, i) * d
    }

    /// An arc's (or a circle's) centre and radius at offset `d`, and the radius's rate.
    fn round(&self, i: usize, sign: f64) -> Option<(Point2, f64, f64)> {
        match cyc(&self.segs, i).0 {
            Segment::Arc {
                center,
                radius,
                sweep,
                ..
            } => Some((center, radius, if sweep > 0.0 { -sign } else { sign })),
            Segment::Circle { center, radius, ccw } => Some((center, radius, if ccw { -sign } else { sign })),
            _ => None,
        }
    }

    /// Whether direction `v` from an arc's centre lies within its sweep (a circle: always).
    fn within(&self, i: usize, v: Vec2) -> bool {
        match cyc(&self.segs, i).0 {
            Segment::Arc { start, sweep, .. } => {
                let a = dmath::atan2(v.y, v.x);
                let (lo, hi) = (start.min(start + sweep), start.max(start + sweep));
                let mut t = a;
                while t < lo - 1e-12 {
                    t += dmath::TAU;
                }
                while t > hi + 1e-12 {
                    t -= dmath::TAU;
                }
                t >= lo - 1e-12 && t <= hi + 1e-12
            }
            _ => true,
        }
    }
}

/// `v[i]`, `i` taken round the loop (`v.len()` and past wrap): a loop's reads, one place to fail.
#[inline]
pub(crate) fn cyc<T>(v: &[T], i: usize) -> &T {
    &v[i % v.len()]
}

/// The least root in (0, reach] of `a δ² + b δ + c` (c the value at 0), or None.
pub(crate) fn least_root(a: f64, b: f64, c: f64, reach: f64) -> Option<f64> {
    let roots: Vec<f64> = if a.abs() <= 1e-15 * (b.abs() + c.abs()).max(f64::MIN_POSITIVE) {
        if b == 0.0 { vec![] } else { vec![-c / b] }
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            vec![]
        } else {
            let q = -0.5 * (b + b.signum() * disc.sqrt());
            if q == 0.0 { vec![0.0] } else { vec![q / a, c / q] }
        }
    };
    roots
        .into_iter()
        .filter(|&r| r > 0.0 && r <= reach)
        .fold(None, |m: Option<f64>, r| Some(m.map_or(r, |m| m.min(r))))
}

/// The first event of tapering `p` by offsets up to `reach` (`sign` the way they go), each
/// computed exactly from the carriers (lines move straight, so lengths and contacts with lines
/// are linear in the offset, contacts with arcs quadratic), or None when the profile's loops are
/// not all `Lin` (`next_event` decides those). `Some(None)`: no event within `reach`; a crossing
/// at 0: neighbouring sides that no longer meet.
pub(crate) fn first_event(p: &Profile, sign: f64, reach: f64) -> Option<Option<(f64, Event)>> {
    let lins: Vec<Lin> = match p.loops.iter().map(|l| Lin::new(&l.segments, sign)).collect() {
        Ok(l) => l,
        // neighbours that no longer meet: the walls cross here
        Err(true) => return Some(Some((0.0, Event::Cross))),
        // an arc meeting a neighbour at an angle: its joints move on a circle
        Err(false) => return crate::taper::curved_event(p, sign, reach),
    };
    let mut edge = f64::INFINITY;
    // E: a side's length, or a radius, running out
    let mut roots: Vec<Vec<Option<f64>>> = Vec::with_capacity(lins.len());
    for l in &lins {
        let n = l.segs.len();
        let mut rs = Vec::with_capacity(n);
        for i in 0..n {
            let r = match cyc(&l.segs, i).0 {
                Segment::Line { a, b } => {
                    let u = unit(b - a);
                    let len0 = (b - a).dot(u);
                    let rate = (*cyc(&l.vel, i + 1) - *cyc(&l.vel, i)).dot(u);
                    least_root(0.0, rate, len0, reach)
                }
                _ => {
                    let (_, r, dr) = l.round(i, sign)?;
                    least_root(0.0, dr, r, reach)
                }
            };
            if let Some(r) = r {
                edge = edge.min(r);
            }
            rs.push(r);
        }
        roots.push(rs);
    }
    let mut cross = f64::INFINITY;
    let same = |d: f64| edge.is_finite() && (d - edge).abs() <= 1e-9 * edge.max(1.0);
    // the sides that run out with the first: their ends come together, which is the edge event
    let gone: Vec<Vec<bool>> = roots
        .iter()
        .map(|rs| rs.iter().map(|r| r.is_some_and(same)).collect())
        .collect();
    // whether joints k and m of loop l come together at the edge event: every side between them,
    // one way round, runs out with it
    let merged = |l: usize, k: usize, m: usize| {
        let n = cyc(&gone, l).len();
        let run = |a: usize, b: usize| {
            (0..n)
                .map(|t| if a + t < n { a + t } else { a + t - n })
                .take_while(|&t| t != b)
                .all(|t| *cyc(cyc(&gone, l), t))
        };
        run(k, m) || run(m, k)
    };
    // a contact at δ is the edge event itself when it comes with it, on the end of a side whose
    // joint there comes together with the corner by sides that run out
    let ends = |f: f64, len: f64| {
        let e = 1e-9 * len.max(1.0);
        (f <= e, f >= len - e)
    };
    for (la, a) in lins.iter().enumerate() {
        let na = a.segs.len();
        for (lb, b) in lins.iter().enumerate() {
            let nb = b.segs.len();
            for j in 0..nb {
                let adjacent = |k: usize| la == lb && (k == j || k == (j + 1) % nb);
                match cyc(&b.segs, j).0 {
                    Segment::Line { a: s0, b: s1 } => {
                        let u = unit(s1 - s0);
                        let nrm = u.perp();
                        let len = |d: f64| (b.joint((j + 1) % nb, d) - b.joint(j, d)).dot(u);
                        // S, C: a corner reaching this side
                        for k in (0..na).filter(|&k| !adjacent(k)) {
                            if a.segs.len() == 1 && matches!(a.segs[0].0, Segment::Circle { .. }) {
                                break;
                            }
                            let g0 = (*cyc(&a.at, k) - *cyc(&b.at, j)).dot(nrm);
                            let g1 = (*cyc(&a.vel, k) - *cyc(&b.vel, j)).dot(nrm);
                            if g0 == 0.0 {
                                continue;
                            }
                            if let Some(d) = least_root(0.0, g1, g0, reach) {
                                let f = (a.joint(k, d) - b.joint(j, d)).dot(u);
                                let l = len(d);
                                let (at0, at1) = ends(f, l);
                                let with_edge = la == lb
                                    && same(d)
                                    && ((at0 && merged(la, k, j)) || (at1 && merged(la, k, (j + 1) % nb)));
                                if f >= -1e-9 * l.max(1.0) && f <= l + 1e-9 * l.max(1.0) && !with_edge {
                                    cross = cross.min(d);
                                }
                            }
                        }
                        // an arc or circle of loop a touching this side along its length
                        for i in (0..na).filter(|&i| !adjacent_seg(la, lb, i, j, na)) {
                            let Some((c, r, dr)) = a.round(i, sign) else {
                                continue;
                            };
                            // the centre's distance from the side's carrier, against the radius
                            let e0 = (c - *cyc(&b.at, j)).dot(nrm);
                            let e1 = -cyc(&b.vel, j).dot(nrm);
                            for side in [1.0, -1.0] {
                                // e(δ) = side r(δ)
                                if let Some(d) = least_root(0.0, e1 - side * dr, e0 - side * r, reach) {
                                    let e = e0 + e1 * d;
                                    let foot = c - nrm * e;
                                    let f = (foot - b.joint(j, d)).dot(u);
                                    let l = len(d);
                                    if f >= 0.0 && f <= l && a.within(i, foot - c) && !same(d) {
                                        cross = cross.min(d);
                                    }
                                }
                            }
                        }
                    }
                    Segment::Arc { .. } | Segment::Circle { .. } => {
                        let (c, r, dr) = b.round(j, sign)?;
                        // S, C: a corner reaching this arc
                        if !(a.segs.len() == 1 && matches!(a.segs[0].0, Segment::Circle { .. })) {
                            for k in (0..na).filter(|&k| !adjacent(k)) {
                                let (p0, v) = (*cyc(&a.at, k) - c, *cyc(&a.vel, k));
                                let qa = v.dot(v) - dr * dr;
                                let qb = 2.0 * (p0.dot(v) - r * dr);
                                let qc = p0.dot(p0) - r * r;
                                if qc == 0.0 {
                                    continue;
                                }
                                if let Some(d) = least_root(qa, qb, qc, reach)
                                    && b.within(j, a.joint(k, d) - c)
                                    && !same(d)
                                {
                                    cross = cross.min(d);
                                }
                            }
                        }
                        // another arc or circle touching this one
                        for i in 0..na {
                            if (la, i) >= (lb, j) || adjacent_seg(la, lb, i, j, na) {
                                continue;
                            }
                            let Some((c2, r2, dr2)) = a.round(i, sign) else {
                                continue;
                            };
                            let dist = (c2 - c).length();
                            for (s1, s2) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0)] {
                                // s1 r + s2 r2 = dist
                                if let Some(d) =
                                    least_root(0.0, s1 * dr + s2 * dr2, s1 * r + s2 * r2 - dist, reach)
                                {
                                    let rj = r + dr * d;
                                    let dir = if dist > 0.0 {
                                        (c2 - c) * (1.0 / dist)
                                    } else {
                                        Vec2::new(1.0, 0.0)
                                    };
                                    // the touch point, seen from each centre
                                    let t = c + dir * (s1 * rj);
                                    if b.within(j, t - c) && a.within(i, t - c2) && !same(d) {
                                        cross = cross.min(d);
                                    }
                                }
                            }
                        }
                    }
                    Segment::Spline(_) => return None,
                }
            }
        }
    }
    Some(if cross <= edge && cross.is_finite() {
        // a loop that closes up here meets itself as it goes (an outline flattening to a ridge, a
        // hole to a slit): that is its collapse, which the slabs end or drop (sweep-spec 2.2,
        // rules 2 and 3), not walls crossing
        if p.loops
            .iter()
            .any(|l| crate::taper::closes(&l.segments, cross * sign))
        {
            Some((cross, Event::Edge))
        } else {
            Some((cross, Event::Cross))
        }
    } else if edge.is_finite() {
        Some((edge, Event::Edge))
    } else {
        None
    })
}

/// Whether segments `i` (of loop `la`) and `j` (of loop `lb`) are one segment or neighbours.
pub(crate) fn adjacent_seg(la: usize, lb: usize, i: usize, j: usize, n: usize) -> bool {
    la == lb && (i == j || (i + 1) % n == j || (j + 1) % n == i)
}
