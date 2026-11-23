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
            let (a, b) = (&segs[(i + n - 1) % n].0, &segs[i].0);
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
        self.at[i] + self.vel[i] * d
    }

    /// An arc's (or a circle's) centre and radius at offset `d`, and the radius's rate.
    fn round(&self, i: usize, sign: f64) -> Option<(Point2, f64, f64)> {
        match self.segs[i].0 {
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
        match self.segs[i].0 {
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
        Err(false) => return curved_event(p, sign, reach),
    };
    let mut edge = f64::INFINITY;
    // E: a side's length, or a radius, running out
    let mut roots: Vec<Vec<Option<f64>>> = Vec::with_capacity(lins.len());
    for l in &lins {
        let n = l.segs.len();
        let mut rs = Vec::with_capacity(n);
        for i in 0..n {
            let r = match l.segs[i].0 {
                Segment::Line { a, b } => {
                    let u = unit(b - a);
                    let len0 = (b - a).dot(u);
                    let rate = (l.vel[(i + 1) % n] - l.vel[i]).dot(u);
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
        let n = gone[l].len();
        let run = |a: usize, b: usize| {
            (0..n)
                .map(|t| (a + t) % n)
                .take_while(|&t| t != b)
                .all(|t| gone[l][t])
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
                match b.segs[j].0 {
                    Segment::Line { a: s0, b: s1 } => {
                        let u = unit(s1 - s0);
                        let nrm = u.perp();
                        let len = |d: f64| (b.joint((j + 1) % nb, d) - b.joint(j, d)).dot(u);
                        // S, C: a corner reaching this side
                        for k in (0..na).filter(|&k| !adjacent(k)) {
                            if a.segs.len() == 1 && matches!(a.segs[0].0, Segment::Circle { .. }) {
                                break;
                            }
                            let g0 = (a.at[k] - b.at[j]).dot(nrm);
                            let g1 = (a.vel[k] - b.vel[j]).dot(nrm);
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
                            let e0 = (c - b.at[j]).dot(nrm);
                            let e1 = -b.vel[j].dot(nrm);
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
                                let (p0, v) = (a.at[k] - c, a.vel[k]);
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

/// A polynomial in the offset δ, coefficients ascending.
type Poly = Vec<f64>;

fn padd(a: &[f64], b: &[f64]) -> Poly {
    (0..a.len().max(b.len()))
        .map(|i| a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0))
        .collect()
}

fn pscale(a: &[f64], k: f64) -> Poly {
    a.iter().map(|x| x * k).collect()
}

fn pmul(a: &[f64], b: &[f64]) -> Poly {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += x * y;
        }
    }
    out
}

/// A joint at offset δ: `p(δ) + v σ sqrt(r(δ))`, p a pair of polynomials; a joint between lines,
/// or tangent neighbours, has v = 0.
struct Joint {
    p: [Poly; 2],
    v: Vec2,
    r: Poly,
    sg: f64,
}

impl Joint {
    fn root(&self, d: f64) -> f64 {
        forge_math::poly::eval(&self.r, d).max(0.0).sqrt() * self.sg
    }
    fn at(&self, d: f64) -> Point2 {
        let w = self.root(d);
        Point2::new(
            forge_math::poly::eval(&self.p[0], d) + self.v.x * w,
            forge_math::poly::eval(&self.p[1], d) + self.v.y * w,
        )
    }
    /// `(J - o) . n` as `a(δ) + b σ sqrt(r(δ))`, with `o` and `n` fixed.
    fn along(&self, o: Point2, n: Vec2) -> (Poly, f64) {
        let a = padd(&pscale(&self.p[0], n.x), &pscale(&self.p[1], n.y));
        (padd(&a, &[-(o.x * n.x + o.y * n.y)]), self.v.dot(n) * self.sg)
    }
}

/// A loop's segments and its joints as they move.
type Moving<'a> = (&'a [(Segment, u64)], Vec<Joint>);

/// A line's carrier at δ: the points x with `(x - a) . n = sign δ`, `u` along it.
struct Carrier {
    a: Point2,
    u: Vec2,
    n: Vec2,
}

/// The roots in (0, reach] of `a + b1 sqrt(r1) + b2 sqrt(r2)` (b1, b2 already signed), found
/// on the polynomial its squares give and kept where the expression itself vanishes.
fn radical_roots(a: &[f64], (b1, r1): (f64, &[f64]), (b2, r2): (f64, &[f64]), reach: f64) -> Vec<f64> {
    let poly = if b1 == 0.0 && b2 == 0.0 {
        a.to_vec()
    } else if b2 == 0.0 || b1 == 0.0 {
        let (b, r) = if b1 == 0.0 { (b2, r2) } else { (b1, r1) };
        padd(&pmul(a, a), &pscale(&pmul(&[b * b], r), -1.0))
    } else {
        // (a + b1 w1)^2 = b2^2 r2, then the w1 term squared away
        let k = padd(&padd(&pmul(a, a), &pscale(r1, b1 * b1)), &pscale(r2, -b2 * b2));
        padd(&pmul(&k, &k), &pscale(&pmul(&pmul(a, a), r1), -4.0 * b1 * b1))
    };
    let lead = poly.iter().rposition(|x| x.abs() > 0.0).map_or(0, |i| i + 1);
    if lead <= 1 {
        return vec![];
    }
    let f = |d: f64| {
        let w = |r: &[f64]| forge_math::poly::eval(r, d).max(0.0).sqrt();
        forge_math::poly::eval(a, d) + b1 * w(r1) + b2 * w(r2)
    };
    let scale = a.iter().fold(0.0f64, |m, x| m.max(x.abs())) + b1.abs() + b2.abs() + 1.0;
    forge_math::poly::real_roots(&poly[..lead], 0.0, reach)
        .into_iter()
        .map(|r| r.x)
        .filter(|&d| d > 0.0 && f(d).abs() <= 1e-9 * scale * (1.0 + d))
        .collect()
}

/// `first_event` where some joint is an arc meeting its neighbour at an angle: the joint moves
/// on the circle where the other's carrier meets it, so a side's length, a corner's distance from
/// a line and its distance from a circle are radicals of polynomials in the offset, whose roots
/// are those of the polynomial squaring gives (kept where the radical form vanishes). The joint is
/// lost (event J) where its radicand reaches 0. Arcs meeting each other at an angle: None (the
/// search decides).
fn curved_event(p: &Profile, sign: f64, reach: f64) -> Option<Option<(f64, Event)>> {
    let mut loops: Vec<Moving> = Vec::new();
    let line = |s: &Segment| match *s {
        Segment::Line { a, b } => {
            let u = unit(b - a);
            Some(Carrier { a, u, n: u.perp() })
        }
        _ => None,
    };
    // a circle's centre, radius at 0 and its rate in δ
    let round = |s: &Segment| match *s {
        Segment::Arc {
            center,
            radius,
            sweep,
            ..
        } => Some((center, radius, if sweep > 0.0 { -sign } else { sign })),
        Segment::Circle { center, radius, ccw } => Some((center, radius, if ccw { -sign } else { sign })),
        _ => None,
    };
    for l in &p.loops {
        let segs = &l.segments[..];
        let n = segs.len();
        if n == 1 {
            loops.push((
                segs,
                vec![Joint {
                    p: [vec![segs[0].0.start().x], vec![segs[0].0.start().y]],
                    v: Vec2::new(0.0, 0.0),
                    r: vec![0.0],
                    sg: 0.0,
                }],
            ));
            continue;
        }
        let mut js = Vec::with_capacity(n);
        for i in 0..n {
            let (a, b) = (&segs[(i + n - 1) % n].0, &segs[i].0);
            let (ta, tb) = (unit(ends_dir(a).1), unit(ends_dir(b).0));
            let at = b.start();
            let c = ta.x * tb.y - ta.y * tb.x;
            let still = |w: Vec2| Joint {
                p: [vec![at.x, w.x * sign], vec![at.y, w.y * sign]],
                v: Vec2::new(0.0, 0.0),
                r: vec![0.0],
                sg: 0.0,
            };
            js.push(if c.abs() < 1e-7 && ta.dot(tb) > 0.0 {
                still(unit(ta + tb).perp())
            } else if let (Some(ca), Some(cb)) = (line(a), line(b)) {
                let den = 1.0 + ca.n.dot(cb.n);
                if den <= 1e-12 {
                    return Some(Some((0.0, Event::Cross)));
                }
                still((ca.n + cb.n) * (1.0 / den))
            } else {
                // (arcs meeting each other at an angle: the search decides)
                let l = line(a).or_else(|| line(b))?;
                let (cc, r0, dr) = round(a).or_else(|| round(b))?;
                // the foot of the centre on the carrier, d(δ) from it, and along it sqrt(ρ² - d²)
                let d = [(cc - l.a).dot(l.n), -sign];
                let foot = [
                    vec![cc.x - l.n.x * d[0], -l.n.x * d[1]],
                    vec![cc.y - l.n.y * d[0], -l.n.y * d[1]],
                ];
                let rho = [r0, dr];
                let r = padd(&pmul(&rho, &rho), &pscale(&pmul(&d, &d), -1.0));
                let side = (at - Point2::new(foot[0][0], foot[1][0])).dot(l.u);
                Joint {
                    p: foot,
                    v: l.u,
                    r,
                    sg: if side < 0.0 { -1.0 } else { 1.0 },
                }
            });
        }
        loops.push((segs, js));
    }
    let mut edge = f64::INFINITY;
    let mut lost = f64::INFINITY;
    let least = |m: &mut f64, ds: Vec<f64>| {
        for d in ds {
            *m = m.min(d);
        }
    };
    // E and J
    for (segs, js) in &loops {
        let n = segs.len();
        for j in js {
            if j.v != Vec2::new(0.0, 0.0) {
                // where the joint's radicand runs out, if it goes through 0
                let lostd: Vec<f64> = forge_math::poly::real_roots(&j.r, 0.0, reach)
                    .into_iter()
                    .map(|r| r.x)
                    .filter(|&d| d > 0.0)
                    .collect();
                least(&mut lost, lostd);
            }
        }
        for i in 0..n {
            let (a, b) = (&js[i], &js[(i + 1) % n]);
            match segs[i].0 {
                Segment::Line { .. } => {
                    let c = line(&segs[i].0)?;
                    // (b - a) . u
                    let (pa, wa) = a.along(Point2::new(0.0, 0.0), c.u);
                    let (pb, wb) = b.along(Point2::new(0.0, 0.0), c.u);
                    let len = padd(&pb, &pscale(&pa, -1.0));
                    least(&mut edge, radical_roots(&len, (-wa, &a.r), (wb, &b.r), reach));
                }
                _ => {
                    let (_, r0, dr) = round(&segs[i].0)?;
                    least(&mut edge, least_root(0.0, dr, r0, reach).into_iter().collect());
                    if n > 1 {
                        // its sweep running out: its start reaching the next side's carrier
                        if let Some(cb) = line(&segs[(i + 1) % n].0) {
                            let (g, w) = a.along(cb.a, cb.n);
                            let g = padd(&g, &[0.0, -sign]);
                            for d in radical_roots(&g, (w, &a.r), (0.0, &[]), reach) {
                                if a.at(d).distance(b.at(d)) <= 1e-7 * (1.0 + r0) {
                                    edge = edge.min(d);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let same = |d: f64| edge.is_finite() && (d - edge).abs() <= 1e-9 * edge.max(1.0);
    let mut cross = lost;
    // the arc's sweep from its start joint to `q`, against its whole sweep, at δ
    let on_arc = |segs: &[(Segment, u64)], js: &[Joint], j: usize, q: Point2, d: f64| -> bool {
        let Segment::Arc { center, sweep, .. } = segs[j].0 else {
            return true;
        };
        let n = segs.len();
        let ang = |x: Point2| dmath::atan2(x.y - center.y, x.x - center.x);
        let k = if sweep > 0.0 { 1.0 } else { -1.0 };
        let turn = |from: f64, to: f64| (k * (to - from)).rem_euclid(dmath::TAU);
        let a0 = ang(js[j].at(d));
        let whole = turn(a0, ang(js[(j + 1) % n].at(d)));
        let t = turn(a0, ang(q));
        t <= whole + 1e-9 || t >= dmath::TAU - 1e-9
    };
    for (la, (sa, ja)) in loops.iter().enumerate() {
        let na = sa.len();
        for (lb, (sb, jb)) in loops.iter().enumerate() {
            let nb = sb.len();
            for j in 0..nb {
                let near = |k: usize| la == lb && (k == j || k == (j + 1) % nb);
                if let Some(c) = line(&sb[j].0) {
                    let len = |d: f64| (jb[(j + 1) % nb].at(d) - jb[j].at(d)).dot(c.u);
                    for k in (0..na).filter(|&k| !near(k) && na > 1) {
                        let (g, w) = ja[k].along(c.a, c.n);
                        let g = padd(&g, &[0.0, -sign]);
                        if forge_math::poly::eval(&g, 0.0)
                            + w * forge_math::poly::eval(&ja[k].r, 0.0).max(0.0).sqrt()
                            == 0.0
                        {
                            continue;
                        }
                        for d in radical_roots(&g, (w, &ja[k].r), (0.0, &[]), reach) {
                            let f = (ja[k].at(d) - jb[j].at(d)).dot(c.u);
                            let l = len(d);
                            let e = 1e-9 * l.abs().max(1.0);
                            if f >= -e && f <= l + e && !(same(d) && (f <= e || f >= l - e)) {
                                cross = cross.min(d);
                            }
                        }
                    }
                    for i in (0..na).filter(|&i| !adjacent_seg(la, lb, i, j, na)) {
                        let Some((cc, r0, dr)) = round(&sa[i].0) else {
                            continue;
                        };
                        let e0 = (cc - c.a).dot(c.n);
                        for side in [1.0, -1.0] {
                            if let Some(d) = least_root(0.0, -sign - side * dr, e0 - side * r0, reach) {
                                let e = e0 - sign * d;
                                let foot = cc - c.n * e;
                                let f = (foot - jb[j].at(d)).dot(c.u);
                                if f >= 0.0 && f <= len(d) && on_arc(sa, ja, i, foot, d) && !same(d) {
                                    cross = cross.min(d);
                                }
                            }
                        }
                    }
                } else if let Some((cc, r0, dr)) = round(&sb[j].0) {
                    for k in (0..na).filter(|&k| !near(k) && na > 1) {
                        // |J - c|² - ρ² = A + B w, w² = r
                        let jk = &ja[k];
                        let px = padd(&jk.p[0], &[-cc.x]);
                        let py = padd(&jk.p[1], &[-cc.y]);
                        let rho = [r0, dr];
                        let a = padd(
                            &padd(
                                &padd(&pmul(&px, &px), &pmul(&py, &py)),
                                &pscale(&jk.r, jk.v.dot(jk.v)),
                            ),
                            &pscale(&pmul(&rho, &rho), -1.0),
                        );
                        let b = padd(
                            &pscale(&px, 2.0 * jk.v.x * jk.sg),
                            &pscale(&py, 2.0 * jk.v.y * jk.sg),
                        );
                        if forge_math::poly::eval(&a, 0.0).abs() <= 0.0 {
                            continue;
                        }
                        // A + B w = 0: A² - B² r = 0, kept where A + B w vanishes
                        let poly = padd(&pmul(&a, &a), &pscale(&pmul(&pmul(&b, &b), &jk.r), -1.0));
                        let lead = poly.iter().rposition(|x| x.abs() > 0.0).map_or(0, |i| i + 1);
                        if lead <= 1 {
                            continue;
                        }
                        for root in forge_math::poly::real_roots(&poly[..lead], 0.0, reach) {
                            let d = root.x;
                            let q = jk.at(d);
                            let rho_d = r0 + dr * d;
                            if d > 0.0
                                && ((q - cc).length() - rho_d).abs() <= 1e-9 * (1.0 + rho_d)
                                && on_arc(sb, jb, j, q, d)
                                && !same(d)
                            {
                                cross = cross.min(d);
                            }
                        }
                    }
                    for i in 0..na {
                        if (la, i) >= (lb, j) || adjacent_seg(la, lb, i, j, na) {
                            continue;
                        }
                        let Some((c2, r2, dr2)) = round(&sa[i].0) else {
                            continue;
                        };
                        let dist = (c2 - cc).length();
                        for (s1, s2) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0)] {
                            if let Some(d) =
                                least_root(0.0, s1 * dr + s2 * dr2, s1 * r0 + s2 * r2 - dist, reach)
                            {
                                let dir = if dist > 0.0 {
                                    (c2 - cc) * (1.0 / dist)
                                } else {
                                    Vec2::new(1.0, 0.0)
                                };
                                let t = cc + dir * (s1 * (r0 + dr * d));
                                if on_arc(sb, jb, j, t, d) && on_arc(sa, ja, i, t, d) && !same(d) {
                                    cross = cross.min(d);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Some(if cross <= edge && cross.is_finite() {
        // (a joint lost where its arc and line turn tangent: the loop closes to that point, its
        // area going as the joint goes, so it is closed just before)
        let shut = |d: f64| {
            p.loops
                .iter()
                .any(|l| crate::taper::closes(&l.segments, d * sign))
        };
        if shut(cross) || (cross == lost && shut(cross * (1.0 - 1e-9))) {
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
