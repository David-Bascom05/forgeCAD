//! A sketch's plane arrangement: curves split where they meet, the regions they bound, and any set
//! of regions' boundary as loops. Lines and arcs are exact; splines are fine polylines (approx).
//! Ported from tools/find-parts/recreate/sketch2d.py, operation for operation, so a sketch makes
//! the same loops to the last digit.

use crate::Fail;
use crate::py::{self, PI, TAU, V2, dist2 as dist, pmax, pmin, pmod};
use std::collections::{BTreeSet, HashMap};

/// Points this close are one point (mm).
pub const EPS: f64 = 1e-5;
/// How far a sampled spline can lie off the true one (mm).
pub const SAG: f64 = 2e-3;

#[derive(Clone, Debug)]
pub enum Shape {
    Line { start: V2, end: V2 },
    Circle { center: V2, radius: f64 },
    Arc { start: V2, mid: V2, end: V2 },
    Poly { points: Vec<V2> },
}

#[derive(Clone, Debug)]
pub struct Ent {
    pub id: String,
    pub shape: Shape,
    pub construction: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Line,
    Circle,
    Arc,
    Poly,
}

/// A stretch of one entity between arrangement vertices, along the entity's direction.
#[derive(Clone, Debug)]
pub struct Piece {
    pub ent: usize,
    pub kind: Kind,
    pub a: V2,
    pub b: V2,
    pub c: V2,
    pub r: f64,
    pub ccw: bool,
    pub pts: Vec<V2>,
    /// Entities drawn over this one, and whether they run the same way.
    pub others: Vec<(usize, bool)>,
    pub va: usize,
    pub vb: usize,
}

fn sub(a: V2, b: V2) -> V2 {
    (a.0 - b.0, a.1 - b.1)
}

impl Piece {
    fn new(ent: usize, kind: Kind, a: V2, b: V2) -> Piece {
        Piece {
            ent,
            kind,
            a,
            b,
            c: (0.0, 0.0),
            r: 0.0,
            ccw: true,
            pts: vec![],
            others: vec![],
            va: 0,
            vb: 0,
        }
    }

    fn round(ent: usize, kind: Kind, a: V2, b: V2, c: V2, r: f64, ccw: bool) -> Piece {
        Piece {
            c,
            r,
            ccw,
            ..Piece::new(ent, kind, a, b)
        }
    }

    fn poly(ent: usize, pts: Vec<V2>) -> Piece {
        Piece {
            pts,
            ..Piece::new(ent, Kind::Poly, (0.0, 0.0), (0.0, 0.0))
        }
        .ends()
    }

    fn ends(mut self) -> Piece {
        self.a = self.pts[0];
        self.b = self.pts[self.pts.len() - 1];
        self
    }

    pub fn sweep(&self) -> (f64, f64) {
        let a0 = py::atan2(self.a.1 - self.c.1, self.a.0 - self.c.0);
        let a1 = py::atan2(self.b.1 - self.c.1, self.b.0 - self.c.0);
        let mut d = a1 - a0;
        if self.ccw {
            while d <= 1e-12 {
                d += TAU;
            }
        } else {
            while d >= -1e-12 {
                d -= TAU;
            }
        }
        (a0, d)
    }

    pub fn sample(&self, n: usize) -> Vec<V2> {
        match self.kind {
            Kind::Line => vec![self.a, self.b],
            Kind::Poly => self.pts.clone(),
            _ => {
                let (a0, d) = self.sweep();
                let mut out = vec![self.a];
                for i in 1..n {
                    let t = a0 + d * i as f64 / n as f64;
                    out.push((self.c.0 + self.r * py::cos(t), self.c.1 + self.r * py::sin(t)));
                }
                out.push(self.b);
                out
            }
        }
    }

    fn near(&self, p: V2, tol: f64) -> bool {
        curve_dist(self, p) <= tol
    }
}

pub fn curve_dist(pc: &Piece, p: V2) -> f64 {
    match pc.kind {
        Kind::Circle => return (dist(p, pc.c) - pc.r).abs(),
        Kind::Arc => {
            let (a0, d) = pc.sweep();
            let t = py::atan2(p.1 - pc.c.1, p.0 - pc.c.0);
            let off = if d > 0.0 {
                pmod(t - a0, TAU)
            } else {
                pmod(a0 - t, TAU)
            };
            if off <= d.abs() {
                return (dist(p, pc.c) - pc.r).abs();
            }
            return pmin(dist(p, pc.a), dist(p, pc.b));
        }
        _ => {}
    }
    let pts = pc.sample(24);
    let mut best = f64::INFINITY;
    for w in pts.windows(2) {
        let (u, v) = (w[0], w[1]);
        let e = sub(v, u);
        let l2 = e.0 * e.0 + e.1 * e.1;
        let t = if l2 == 0.0 {
            0.0
        } else {
            pmax(0.0, pmin(1.0, ((p.0 - u.0) * e.0 + (p.1 - u.1) * e.1) / l2))
        };
        best = pmin(best, py::hypot(&[u.0 + t * e.0 - p.0, u.1 + t * e.1 - p.1]));
    }
    best
}

/// Centre, radius and turn through three points; None when they are in line.
pub fn circle_from(a: V2, m: V2, b: V2) -> Option<(V2, f64, bool)> {
    let (ax, ay, bx, by, cx, cy) = (a.0, a.1, m.0, m.1, b.0, b.1);
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < 1e-14 {
        return None;
    }
    let (s1, s2, s3) = (ax * ax + ay * ay, bx * bx + by * by, cx * cx + cy * cy);
    let ux = (s1 * (by - cy) + s2 * (cy - ay) + s3 * (ay - by)) / d;
    let uy = (s1 * (cx - bx) + s2 * (ax - cx) + s3 * (bx - ax)) / d;
    Some(((ux, uy), py::hypot(&[ax - ux, ay - uy]), d > 0.0))
}

/// Non-construction curves as whole-entity pieces (a circle split later), each naming its entity
/// by index; and whether any is approximate.
fn entities(ents: &[Ent]) -> (Vec<Piece>, bool) {
    let mut out = vec![];
    let mut approx = false;
    for (k, e) in ents.iter().enumerate() {
        let p = match &e.shape {
            Shape::Line { start, end } => Piece::new(k, Kind::Line, *start, *end),
            Shape::Circle { center: c, radius } => {
                let a = (c.0 + radius, c.1);
                Piece::round(k, Kind::Circle, a, a, *c, *radius, true)
            }
            Shape::Arc { start, mid, end } => match circle_from(*start, *mid, *end) {
                None => Piece::new(k, Kind::Line, *start, *end),
                Some((c, r, ccw)) => Piece::round(k, Kind::Arc, *start, *end, c, r, ccw),
            },
            Shape::Poly { points } => {
                approx = approx || !e.construction;
                Piece::poly(k, points.clone())
            }
        };
        if !e.construction {
            out.push(p);
        }
    }
    (out, approx)
}

fn line_x(a: &Piece, b: &Piece) -> Vec<V2> {
    let (r, s) = (sub(a.b, a.a), sub(b.b, b.a));
    let d = r.0 * s.1 - r.1 * s.0;
    let w = sub(b.a, a.a);
    if d.abs() < 1e-12 {
        let mut out = vec![];
        for p in [b.a, b.b] {
            if a.near(p, EPS) {
                out.push(p);
            }
        }
        for p in [a.a, a.b] {
            if b.near(p, EPS) {
                out.push(p);
            }
        }
        return out;
    }
    let (t, u) = ((w.0 * s.1 - w.1 * s.0) / d, (w.0 * r.1 - w.1 * r.0) / d);
    let (la, lb) = (
        pmax(py::hypot(&[r.0, r.1]), 1e-12),
        pmax(py::hypot(&[s.0, s.1]), 1e-12),
    );
    if -EPS / la <= t && t <= 1.0 + EPS / la && -EPS / lb <= u && u <= 1.0 + EPS / lb {
        return vec![(a.a.0 + t * r.0, a.a.1 + t * r.1)];
    }
    vec![]
}

fn on_arc(pc: &Piece, p: V2) -> bool {
    pc.kind == Kind::Circle || curve_dist(pc, p) <= EPS * 5.0
}

fn line_circle(l: &Piece, c: &Piece) -> Vec<V2> {
    let d = sub(l.b, l.a);
    let f = sub(l.a, c.c);
    let a = d.0 * d.0 + d.1 * d.1;
    let b = 2.0 * (f.0 * d.0 + f.1 * d.1);
    let cc = f.0 * f.0 + f.1 * f.1 - c.r * c.r;
    if a == 0.0 {
        return vec![];
    }
    let disc = b * b - 4.0 * a * cc;
    if disc < -1e-9 * pmax(a * c.r * c.r, 1e-12) {
        return vec![];
    }
    let disc = pmax(disc, 0.0).sqrt();
    let la = a.sqrt();
    let mut out = vec![];
    for t in [(-b - disc) / (2.0 * a), (-b + disc) / (2.0 * a)] {
        if -EPS / la <= t && t <= 1.0 + EPS / la {
            let p = (l.a.0 + t * d.0, l.a.1 + t * d.1);
            if on_arc(c, p) {
                out.push(p);
            }
        }
    }
    out
}

fn circle_circle(a: &Piece, b: &Piece) -> Vec<V2> {
    let d = dist(a.c, b.c);
    if d < 1e-12 || d > a.r + b.r + EPS || d < (a.r - b.r).abs() - EPS {
        return vec![];
    }
    let x = (d * d + a.r * a.r - b.r * b.r) / (2.0 * d);
    let h = pmax(a.r * a.r - x * x, 0.0).sqrt();
    let u = ((b.c.0 - a.c.0) / d, (b.c.1 - a.c.1) / d);
    let m = (a.c.0 + x * u.0, a.c.1 + x * u.1);
    [(m.0 - h * u.1, m.1 + h * u.0), (m.0 + h * u.1, m.1 - h * u.0)]
        .into_iter()
        .filter(|&p| on_arc(a, p) && on_arc(b, p))
        .collect()
}

fn segs(p: &Piece) -> Vec<Piece> {
    if p.kind == Kind::Poly {
        return p
            .pts
            .windows(2)
            .filter(|w| dist(w[0], w[1]) > 0.0)
            .map(|w| Piece::new(p.ent, Kind::Line, w[0], w[1]))
            .collect();
    }
    vec![p.clone()]
}

fn bbox(p: &Piece) -> [f64; 4] {
    if matches!(p.kind, Kind::Circle | Kind::Arc) {
        // an arc's whole circle: enough to rule out most pairs
        return [p.c.0 - p.r, p.c.1 - p.r, p.c.0 + p.r, p.c.1 + p.r];
    }
    let ps = if p.kind == Kind::Poly {
        p.pts.clone()
    } else {
        vec![p.a, p.b]
    };
    let lo = |f: fn(&V2) -> f64| ps.iter().map(f).fold(f64::INFINITY, pmin);
    let hi = |f: fn(&V2) -> f64| ps.iter().map(f).fold(f64::NEG_INFINITY, pmax);
    [lo(|q| q.0), lo(|q| q.1), hi(|q| q.0), hi(|q| q.1)]
}

fn apart(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a[2] < b[0] - EPS || b[2] < a[0] - EPS || a[3] < b[1] - EPS || b[3] < a[1] - EPS
}

pub fn meets(a: &Piece, b: &Piece) -> Vec<V2> {
    let mut out = vec![];
    let (sa, sb) = (segs(a), segs(b));
    for x in &sa {
        for y in &sb {
            out.extend(match (x.kind == Kind::Line, y.kind == Kind::Line) {
                (true, true) => line_x(x, y),
                (true, false) => line_circle(x, y),
                (false, true) => line_circle(y, x),
                _ => circle_circle(x, y),
            });
        }
    }
    // a curve ending on a spline: the true spline passes through its end, the sampled one by up to the sag
    for (x, y, sx) in [(a, b, &sa), (b, a, &sb)] {
        if x.kind == Kind::Poly && matches!(y.kind, Kind::Line | Kind::Arc) {
            for q in [y.a, y.b] {
                let known = out.iter().chain([&x.a, &x.b]).any(|&o| dist(q, o) <= SAG);
                if !known && sx.iter().any(|s| curve_dist(s, q) <= SAG) {
                    out.push(q);
                }
            }
        }
    }
    out
}

/// Python's `sorted(set)` of floats, the earlier of equal values kept.
fn sorted_set(xs: impl IntoIterator<Item = f64>) -> Vec<f64> {
    let mut out: Vec<f64> = vec![];
    for x in xs {
        if !out.contains(&x) {
            out.push(x);
        }
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// p cut at the points on it, in order along it.
fn split(p: &Piece, pts: &[V2]) -> Vec<Piece> {
    match p.kind {
        Kind::Line => {
            let d = sub(p.b, p.a);
            let l2 = d.0 * d.0 + d.1 * d.1;
            let mut ts = vec![0.0, 1.0];
            if l2 != 0.0 {
                ts.extend(
                    pts.iter()
                        .map(|q| pmax(0.0, pmin(1.0, ((q.0 - p.a.0) * d.0 + (q.1 - p.a.1) * d.1) / l2))),
                );
            }
            let ts = sorted_set(ts);
            let ps: Vec<V2> = ts.iter().map(|t| (p.a.0 + t * d.0, p.a.1 + t * d.1)).collect();
            ps.windows(2)
                .filter(|w| dist(w[0], w[1]) > EPS)
                .map(|w| Piece::new(p.ent, Kind::Line, w[0], w[1]))
                .collect()
        }
        Kind::Arc | Kind::Circle => {
            let (mut a0, d) = if p.kind == Kind::Circle {
                (py::atan2(p.a.1 - p.c.1, p.a.0 - p.c.0), TAU)
            } else {
                p.sweep()
            };
            let mut off = vec![];
            for q in pts {
                let t = py::atan2(q.1 - p.c.1, q.0 - p.c.0);
                let o = if d > 0.0 {
                    pmod(t - a0, TAU)
                } else {
                    pmod(a0 - t, TAU)
                };
                if o <= d.abs() && !off.contains(&o) {
                    off.push(o);
                }
            }
            let s = if d > 0.0 { 1.0 } else { -1.0 };
            if p.kind == Kind::Circle && !off.is_empty() {
                let m = off.iter().copied().fold(f64::INFINITY, pmin);
                a0 += m * s;
                off = sorted_set(off.iter().map(|x| x - m));
            }
            let ts = sorted_set([0.0, d.abs()].into_iter().chain(off));
            let ps: Vec<V2> = ts
                .iter()
                .map(|t| {
                    (
                        p.c.0 + p.r * py::cos(a0 + s * t),
                        p.c.1 + p.r * py::sin(a0 + s * t),
                    )
                })
                .collect();
            if ts.len() == 2 && p.kind == Kind::Circle {
                return vec![Piece::round(p.ent, Kind::Circle, ps[0], ps[0], p.c, p.r, true)];
            }
            let mut out = vec![];
            for i in 0..ps.len() - 1 {
                if p.r * (ts[i + 1] - ts[i]) > EPS {
                    out.push(Piece::round(
                        p.ent,
                        Kind::Arc,
                        ps[i],
                        ps[i + 1],
                        p.c,
                        p.r,
                        d > 0.0,
                    ));
                }
            }
            out
        }
        Kind::Poly => {
            let segs: Vec<Piece> = p
                .pts
                .windows(2)
                .map(|w| Piece::new(p.ent, Kind::Line, w[0], w[1]))
                .collect();
            // a point may lie off the chords by the sampling's sag: it goes on the nearest one
            let mut at: HashMap<usize, Vec<V2>> = HashMap::new();
            for &q in pts {
                let mut k = 0;
                let mut best = f64::INFINITY;
                for (i, s) in segs.iter().enumerate() {
                    let c = curve_dist(s, q);
                    if c < best {
                        (k, best) = (i, c);
                    }
                }
                if !segs.is_empty() && curve_dist(&segs[k], q) <= SAG {
                    at.entry(k).or_default().push(q);
                }
            }
            let mut out = vec![];
            let mut cur = vec![p.pts[0]];
            let last = p.pts[p.pts.len() - 1];
            for (k, w) in p.pts.windows(2).enumerate() {
                let (u, v) = (w[0], w[1]);
                let mut hits: Vec<(f64, V2)> = at
                    .get(&k)
                    .map(|qs| {
                        qs.iter()
                            .filter(|&&q| dist(u, q) > EPS && dist(v, q) > EPS)
                            .map(|&q| (dist(u, q), q))
                            .collect()
                    })
                    .unwrap_or_default();
                hits.sort_by(|a, b| {
                    (a.0, a.1.0, a.1.1)
                        .partial_cmp(&(b.0, b.1.0, b.1.1))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                for (_, q) in hits {
                    cur.push(q);
                    out.push(Piece::poly(p.ent, std::mem::replace(&mut cur, vec![q])));
                }
                cur.push(v);
                if pts.iter().any(|&q| dist(v, q) <= EPS * 2.0) && v != last {
                    out.push(Piece::poly(p.ent, std::mem::replace(&mut cur, vec![v])));
                }
            }
            if cur.len() > 1 {
                out.push(Piece::poly(p.ent, cur));
            }
            out
        }
    }
}

pub struct Face {
    pub outer: usize,
    pub holes: Vec<usize>,
    pub area: f64,
}

pub type He = (usize, bool);

pub struct Arrangement {
    pub pieces: Vec<Piece>,
    pub verts: Vec<V2>,
    pub loops: Vec<Vec<He>>,
    pub faces: Vec<Face>,
    pub left: HashMap<He, usize>,
    pub live: BTreeSet<usize>,
    pub approx: bool,
    angles: std::cell::RefCell<HashMap<He, f64>>,
    polys: std::cell::RefCell<HashMap<Vec<He>, std::rc::Rc<Vec<V2>>>>,
}

#[derive(PartialEq, Eq, Hash)]
enum Round {
    No,
    R(u64),
}

impl Arrangement {
    pub fn new(ents: &[Ent]) -> Result<Arrangement, Fail> {
        let (curves, approx) = entities(ents);
        if curves.len() > 400 {
            return Err(Fail::Unsupported("sketch of over 400 curves".into()));
        }
        let mut hits: Vec<Vec<V2>> = vec![vec![]; curves.len()];
        let boxes: Vec<[f64; 4]> = curves.iter().map(bbox).collect();
        for i in 0..curves.len() {
            for j in i + 1..curves.len() {
                if apart(&boxes[i], &boxes[j]) {
                    continue;
                }
                let ps = meets(&curves[i], &curves[j]);
                hits[i].extend(&ps);
                hits[j].extend(&ps);
            }
        }
        let mut a = Arrangement {
            pieces: vec![],
            verts: vec![],
            loops: vec![],
            faces: vec![],
            left: HashMap::new(),
            live: BTreeSet::new(),
            approx,
            angles: Default::default(),
            polys: Default::default(),
        };
        let mut keep = vec![];
        for (c, h) in curves.iter().zip(&hits) {
            for mut p in split(c, h) {
                p.va = a.vert(p.a);
                p.vb = a.vert(p.b);
                if p.va != p.vb || matches!(p.kind, Kind::Circle | Kind::Poly) {
                    keep.push(p);
                }
            }
        }
        // curves drawn over each other are one curve; each entity still names it
        let mut once: HashMap<(usize, usize, Round, String, String), usize> = HashMap::new();
        let mut pieces: Vec<Piece> = vec![];
        for p in keep {
            let mid = match p.kind {
                Kind::Line => ((p.a.0 + p.b.0) / 2.0, (p.a.1 + p.b.1) / 2.0),
                Kind::Circle => p.c,
                _ => {
                    let s = p.sample(8);
                    s[s.len() / 2]
                }
            };
            let r = if p.kind == Kind::Circle {
                Round::R((p.r + 0.0).to_bits())
            } else {
                Round::No
            };
            let k = (
                p.va.min(p.vb),
                p.va.max(p.vb),
                r,
                py::key4(mid.0),
                py::key4(mid.1),
            );
            match once.get(&k) {
                Some(&q) => {
                    let same = p.va == pieces[q].va;
                    pieces[q].others.push((p.ent, same));
                }
                None => {
                    once.insert(k, pieces.len());
                    pieces.push(p);
                }
            }
        }
        a.pieces = pieces;
        a.walk()?;
        Ok(a)
    }

    fn vert(&mut self, q: V2) -> usize {
        if let Some(i) = self.verts.iter().position(|&v| dist(v, q) <= EPS * 2.0) {
            return i;
        }
        self.verts.push(q);
        self.verts.len() - 1
    }

    /// Prunes what bounds nothing: a free end, or a bridge with one region on both sides.
    fn walk(&mut self) -> Result<(), Fail> {
        let mut live: BTreeSet<usize> = (0..self.pieces.len()).collect();
        loop {
            loop {
                let mut deg: HashMap<usize, usize> = HashMap::new();
                for &i in &live {
                    let p = &self.pieces[i];
                    if p.kind == Kind::Circle {
                        continue;
                    }
                    *deg.entry(p.va).or_default() += 1;
                    *deg.entry(p.vb).or_default() += 1;
                }
                let dead: Vec<usize> = live
                    .iter()
                    .copied()
                    .filter(|&i| {
                        let p = &self.pieces[i];
                        p.kind != Kind::Circle && (deg[&p.va] == 1 || deg[&p.vb] == 1)
                    })
                    .collect();
                if dead.is_empty() {
                    break;
                }
                dead.iter().for_each(|i| {
                    live.remove(i);
                });
            }
            self.live = live.clone();
            self.polys.borrow_mut().clear();
            let hes: Vec<He> = live.iter().flat_map(|&i| [(i, false), (i, true)]).collect();
            let loops = trace(&hes, self)?;
            self.faces_from(loops);
            let bridge: Vec<usize> = live
                .iter()
                .copied()
                .filter(|&i| self.left.get(&(i, true)) == self.left.get(&(i, false)))
                .collect();
            if bridge.is_empty() {
                return Ok(());
            }
            bridge.iter().for_each(|i| {
                live.remove(i);
            });
        }
    }

    fn faces_from(&mut self, loops: Vec<Vec<He>>) {
        self.loops = loops;
        self.left = HashMap::new();
        let (mut outers, mut holes) = (vec![], vec![]);
        for (k, lp) in self.loops.iter().enumerate() {
            if self.polygon(lp).len() < 3 && self.pieces[lp[0].0].kind != Kind::Circle {
                // a curve on another with the same ends: encloses nothing
                continue;
            }
            if self.area(lp) > 0.0 {
                outers.push(k);
            } else {
                holes.push(k);
            }
        }
        let mut faces: Vec<Face> = outers
            .iter()
            .map(|&k| Face {
                outer: k,
                holes: vec![],
                area: self.area(&self.loops[k]),
            })
            .collect();
        for k in holes {
            let pt = self.point_of(&self.loops[k]);
            let h = -self.area(&self.loops[k]);
            let mut best: Option<usize> = None;
            for (i, f) in faces.iter().enumerate() {
                if h < f.area * (1.0 - 1e-6)
                    && self.inside(pt, &self.loops[f.outer])
                    && best.is_none_or(|b| f.area < faces[b].area)
                {
                    best = Some(i);
                }
            }
            if let Some(b) = best {
                faces[b].holes.push(k);
            }
        }
        for (fi, f) in faces.iter().enumerate() {
            for k in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for &he in &self.loops[k] {
                    self.left.insert(he, fi);
                }
            }
        }
        self.faces = faces;
    }

    /// Is the point in the region (inside its outside, outside its holes)?
    pub fn holds(&self, region: usize, q: V2) -> bool {
        let f = &self.faces[region];
        self.inside(q, &self.loops[f.outer]) && !f.holes.iter().any(|&h| self.inside(q, &self.loops[h]))
    }

    /// The boundary of a set of regions as profiles: [outer loop, hole loop..], each of half-edges.
    pub fn merged(&self, regions: &[usize]) -> Result<Vec<Vec<Vec<He>>>, Fail> {
        let rs: BTreeSet<usize> = regions.iter().copied().collect();
        if rs.len() == 1 {
            let f = &self.faces[regions[0]];
            let mut p = vec![self.loops[f.outer].clone()];
            p.extend(f.holes.iter().map(|&h| self.loops[h].clone()));
            return Ok(vec![p]);
        }
        let mut hes = BTreeSet::new();
        for &r in &rs {
            let f = &self.faces[r];
            for k in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for &he in &self.loops[k] {
                    if !self.left.get(&(he.0, !he.1)).is_some_and(|x| rs.contains(x)) {
                        hes.insert(he);
                    }
                }
            }
        }
        let hes: Vec<He> = hes.into_iter().collect();
        let loops = trace(&hes, self)?;
        let (mut pos, mut neg) = (vec![], vec![]);
        for l in loops {
            if self.area(&l) > 0.0 {
                pos.push(l)
            } else {
                neg.push(l)
            }
        }
        pos.sort_by(|a, b| {
            (-self.area(a))
                .partial_cmp(&-self.area(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut groups: Vec<Vec<Vec<He>>> = pos.into_iter().map(|o| vec![o]).collect();
        for h in neg {
            let pt = self.point_of(&h);
            let mut best: Option<usize> = None;
            for (i, g) in groups.iter().enumerate() {
                if -self.area(&h) < self.area(&g[0]) * (1.0 - 1e-6)
                    && self.inside(pt, &g[0])
                    && best.is_none_or(|b| self.area(&g[0]) < self.area(&groups[b][0]))
                {
                    best = Some(i);
                }
            }
            match best {
                Some(b) => groups[b].push(h),
                None => return Err(Fail::Unsupported("a hole outside its region".into())),
            }
        }
        Ok(groups)
    }

    fn tangent_angle(&self, he: He) -> f64 {
        if let Some(&t) = self.angles.borrow().get(&he) {
            return t;
        }
        let mut pts = self.pieces[he.0].sample(48);
        if !he.1 {
            pts.reverse();
        }
        let (a, b) = (pts[0], pts[1]);
        let t = py::atan2(b.1 - a.1, b.0 - a.0);
        self.angles.borrow_mut().insert(he, t);
        t
    }

    pub fn polygon(&self, lp: &[He]) -> std::rc::Rc<Vec<V2>> {
        if let Some(p) = self.polys.borrow().get(lp) {
            return p.clone();
        }
        let p = std::rc::Rc::new(self.polygon_of(lp));
        self.polys.borrow_mut().insert(lp.to_vec(), p.clone());
        p
    }

    fn polygon_of(&self, lp: &[He]) -> Vec<V2> {
        let mut pts = vec![];
        for &(i, fwd) in lp {
            let p = &self.pieces[i];
            if p.kind == Kind::Circle {
                let n = 96.0;
                let mut s: Vec<V2> = (0..96)
                    .map(|k| {
                        let t = 2.0 * PI * k as f64 / n;
                        (p.c.0 + p.r * py::cos(t), p.c.1 + p.r * py::sin(t))
                    })
                    .collect();
                if !fwd {
                    s.reverse();
                }
                return s;
            }
            let mut s = p.sample(24);
            if !fwd {
                s.reverse();
            }
            s.pop();
            pts.extend(s);
        }
        pts
    }

    pub fn area(&self, lp: &[He]) -> f64 {
        let ps = self.polygon(lp);
        let n = ps.len();
        py::sum((0..n).map(|i| {
            let (a, b) = (ps[i], ps[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })) / 2.0
    }

    pub fn inside(&self, q: V2, lp: &[He]) -> bool {
        let ps = self.polygon(lp);
        let n = ps.len();
        let mut c = false;
        for i in 0..n {
            let (a, b) = (ps[i], ps[(i + 1) % n]);
            if (a.1 > q.1) != (b.1 > q.1) && q.0 < (b.0 - a.0) * (q.1 - a.1) / (b.1 - a.1) + a.0 {
                c = !c;
            }
        }
        c
    }

    /// A point just inside a counter-clockwise loop: off the boundary any neighbouring region shares.
    pub fn point_within(&self, lp: &[He]) -> V2 {
        let ps = self.polygon(lp);
        let (a, b) = (ps[0], ps[1]);
        let l = pmax(dist(a, b), 1e-12);
        let d = pmin(1e-4, l / 8.0);
        (
            (a.0 + b.0) / 2.0 - d * (b.1 - a.1) / l,
            (a.1 + b.1) / 2.0 + d * (b.0 - a.0) / l,
        )
    }

    /// The middle of a boundary stretch: off any vertex it may share with a loop around it.
    fn point_of(&self, lp: &[He]) -> V2 {
        let ps = self.polygon(lp);
        ((ps[0].0 + ps[1].0) / 2.0, (ps[0].1 + ps[1].1) / 2.0)
    }
}

/// Cycles of half-edges with the face on their left: at each vertex turn most clockwise. `hes`
/// sorted.
fn trace(hes: &[He], arr: &Arrangement) -> Result<Vec<Vec<He>>, Fail> {
    let mut out_at: HashMap<usize, Vec<He>> = HashMap::new();
    for &he in hes {
        let p = &arr.pieces[he.0];
        if p.kind == Kind::Circle {
            continue;
        }
        let v = if he.1 { p.va } else { p.vb };
        out_at.entry(v).or_default().push(he);
    }
    for hs in out_at.values_mut() {
        hs.sort_by(|a, b| {
            arr.tangent_angle(*a)
                .partial_cmp(&arr.tangent_angle(*b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    let mut loops = vec![];
    let mut used = std::collections::HashSet::new();
    for &he in hes {
        if used.contains(&he) {
            continue;
        }
        if arr.pieces[he.0].kind == Kind::Circle {
            used.insert(he);
            loops.push(vec![he]);
            continue;
        }
        let (mut lp, mut cur) = (vec![], he);
        while !used.contains(&cur) {
            used.insert(cur);
            lp.push(cur);
            let q = &arr.pieces[cur.0];
            let v = if cur.1 { q.vb } else { q.va };
            let opts = out_at.get(&v).map(Vec::as_slice).unwrap_or_default();
            if opts.is_empty() {
                return Err(Fail::Unsupported("an open boundary".into()));
            }
            // the arriving edge's reverse direction, then the next clockwise from it
            let ra = arr.tangent_angle((cur.0, !cur.1));
            let turn = |h: He| {
                let d = pmod(ra - arr.tangent_angle(h), TAU);
                if d > 1e-9 { d } else { TAU }
            };
            let mut best = (opts[0], turn(opts[0]));
            for &h in &opts[1..] {
                let t = turn(h);
                if t < best.1 {
                    best = (h, t);
                }
            }
            cur = best.0;
        }
        loops.push(lp);
    }
    Ok(loops)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, s: f64) -> Vec<Ent> {
        let c = [(x, y), (x + s, y), (x + s, y + s), (x, y + s)];
        (0..4)
            .map(|i| Ent {
                id: String::new(),
                shape: Shape::Line {
                    start: c[i],
                    end: c[(i + 1) % 4],
                },
                construction: false,
            })
            .collect()
    }

    #[test]
    fn overlapping_squares_and_a_circle() {
        // as sketch2d.py makes them: 4 regions (the circle's a 96-gon), 13 pieces, 11 vertices
        let mut ents = square(0.0, 0.0, 10.0);
        ents.extend(square(5.0, 5.0, 10.0));
        ents.push(Ent {
            id: String::new(),
            shape: Shape::Circle {
                center: (30.0, 0.0),
                radius: 2.0,
            },
            construction: false,
        });
        let a = Arrangement::new(&ents).expect("builds");
        let areas: Vec<String> = a.faces.iter().map(|f| format!("{:.9}", f.area)).collect();
        assert_eq!(
            areas,
            ["75.000000000", "75.000000000", "25.000000000", "12.557400812"]
        );
        assert_eq!((a.pieces.len(), a.verts.len()), (13, 11));
        let all: Vec<usize> = (0..a.faces.len()).collect();
        let m = a.merged(&all).expect("merges");
        let sizes: Vec<(usize, usize)> = m.iter().map(|p| (p.len(), p[0].len())).collect();
        assert_eq!(sizes, [(1, 8), (1, 1)]);
    }
}
