use forge_math::{Point2, Vec2, dmath};

/// A drawn curve on a view or sheet, exactly where the projection has a closed form.
#[derive(Clone, Debug, PartialEq)]
pub enum Geom2 {
    Line {
        a: Point2,
        b: Point2,
    },
    /// `c + cos t u + sin t v` for t from `t0` to `t1` (`t0 < t1`): a circle when u and v are
    /// square and equal, else an ellipse given by two conjugate semi-diameters.
    Conic {
        c: Point2,
        u: Vec2,
        v: Vec2,
        t0: f64,
        t1: f64,
    },
    /// Points on the curve, within the drawing tolerance between them.
    Poly(Vec<Point2>),
}

/// A map from one plane to another: `x' = o + s (cos r x - sin r y, sin r x + cos r y)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub o: Point2,
    pub s: f64,
    pub cos: f64,
    pub sin: f64,
}

impl Place {
    pub const IDENTITY: Place = Place {
        o: Point2::ORIGIN,
        s: 1.0,
        cos: 1.0,
        sin: 0.0,
    };
    pub fn new(o: Point2, s: f64) -> Place {
        Place {
            o,
            s,
            cos: 1.0,
            sin: 0.0,
        }
    }
    pub fn vec(&self, v: Vec2) -> Vec2 {
        Vec2::new(
            self.s * (self.cos * v.x - self.sin * v.y),
            self.s * (self.sin * v.x + self.cos * v.y),
        )
    }
    pub fn point(&self, p: Point2) -> Point2 {
        self.o + self.vec(p.to_vec())
    }
    /// The point this map takes to `q`.
    pub fn inverse_point(&self, q: Point2) -> Point2 {
        let d = (q - self.o) * (1.0 / self.s);
        Point2::new(self.cos * d.x + self.sin * d.y, -self.sin * d.x + self.cos * d.y)
    }
}

impl Geom2 {
    /// The curve at parameter `t`: 0..1 on a line, the angle on a conic, the index on a polyline.
    pub fn at(&self, t: f64) -> Point2 {
        match self {
            Geom2::Line { a, b } => a.lerp(*b, t),
            Geom2::Conic { c, u, v, .. } => {
                let (s, co) = dmath::sin_cos(t);
                *c + *u * co + *v * s
            }
            Geom2::Poly(p) => {
                let n = p.len();
                if n == 1 {
                    return p[0];
                }
                let i = (t.floor().max(0.0) as usize).min(n - 2);
                p[i].lerp(p[i + 1], t - i as f64)
            }
        }
    }

    pub fn range(&self) -> (f64, f64) {
        match self {
            Geom2::Line { .. } => (0.0, 1.0),
            Geom2::Conic { t0, t1, .. } => (*t0, *t1),
            Geom2::Poly(p) => (0.0, p.len().saturating_sub(1) as f64),
        }
    }

    pub fn start(&self) -> Point2 {
        self.at(self.range().0)
    }
    pub fn end(&self) -> Point2 {
        self.at(self.range().1)
    }

    /// A circle's centre and radius, when the conic is one.
    pub fn circle(&self) -> Option<(Point2, f64)> {
        let Geom2::Conic { c, u, v, .. } = self else {
            return None;
        };
        let (lu, lv) = (u.length(), v.length());
        let r = lu.max(lv);
        ((lu - lv).abs() <= 1e-9 * r && u.dot(*v).abs() <= 1e-9 * r * r).then_some((*c, r))
    }

    /// Points within `chord` of the curve, both ends included.
    pub fn points(&self, chord: f64) -> Vec<Point2> {
        match self {
            Geom2::Line { a, b } => vec![*a, *b],
            Geom2::Conic { u, v, t0, t1, .. } => {
                let r = u.length().max(v.length());
                let n = conic_steps(r, t1 - t0, chord);
                (0..=n)
                    .map(|k| self.at(t0 + (t1 - t0) * k as f64 / n as f64))
                    .collect()
            }
            Geom2::Poly(p) => p.clone(),
        }
    }

    /// Parameters whose points, joined, stay within `chord` of the curve.
    pub fn params(&self, chord: f64) -> Vec<f64> {
        let (t0, t1) = self.range();
        match self {
            Geom2::Line { .. } => vec![0.0, 1.0],
            Geom2::Conic { u, v, .. } => {
                let n = conic_steps(u.length().max(v.length()), t1 - t0, chord);
                (0..=n).map(|k| t0 + (t1 - t0) * k as f64 / n as f64).collect()
            }
            Geom2::Poly(p) => (0..p.len()).map(|k| k as f64).collect(),
        }
    }

    /// The unit direction of travel at `t`.
    pub fn tangent(&self, t: f64) -> Option<Vec2> {
        match self {
            Geom2::Line { a, b } => (*b - *a).normalize(),
            Geom2::Conic { u, v, .. } => {
                let (s, c) = dmath::sin_cos(t);
                (*v * c - *u * s).normalize()
            }
            Geom2::Poly(p) => {
                let i = (t.floor().max(0.0) as usize).min(p.len().saturating_sub(2));
                (*p.get(i + 1)? - p[i]).normalize()
            }
        }
    }

    /// The parameter of the curve's point nearest `q`, starting from `guess` (Newton's method on a
    /// conic; exact on lines and polylines' segments), kept in range.
    pub fn nearest(&self, q: Point2, guess: f64) -> f64 {
        let (t0, t1) = self.range();
        match self {
            Geom2::Line { a, b } => {
                let ab = *b - *a;
                ((q - *a).dot(ab) / ab.dot(ab).max(1e-300)).clamp(0.0, 1.0)
            }
            Geom2::Conic { c, u, v, .. } => {
                let mut t = guess;
                for _ in 0..6 {
                    let (s, co) = dmath::sin_cos(t);
                    let r = *c + *u * co + *v * s - q;
                    let d1 = *v * co - *u * s;
                    let d2 = (*u * co + *v * s) * -1.0;
                    let den = d1.dot(d1) + r.dot(d2);
                    if den.abs() <= 1e-300 {
                        break;
                    }
                    let next = t - r.dot(d1) / den;
                    // a step that changes nothing: every step after it would be the same one
                    if next == t {
                        break;
                    }
                    t = next;
                }
                t.clamp(t0, t1)
            }
            Geom2::Poly(p) => {
                let i = (guess.floor().max(0.0) as usize).min(p.len().saturating_sub(2));
                let (a, b) = (p[i], p[(i + 1).min(p.len() - 1)]);
                let ab = b - a;
                i as f64 + ((q - a).dot(ab) / ab.dot(ab).max(1e-300)).clamp(0.0, 1.0)
            }
        }
    }

    pub fn length(&self) -> f64 {
        let p = self.points(1e-4);
        p.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// The part of the curve between parameters `a < b`.
    pub fn sub(&self, a: f64, b: f64) -> Geom2 {
        match self {
            Geom2::Line { .. } => Geom2::Line {
                a: self.at(a),
                b: self.at(b),
            },
            Geom2::Conic { c, u, v, .. } => Geom2::Conic {
                c: *c,
                u: *u,
                v: *v,
                t0: a,
                t1: b,
            },
            Geom2::Poly(p) => {
                let mut out = vec![self.at(a)];
                let (i, j) = (a.floor() as usize + 1, b.ceil() as usize);
                out.extend(p.iter().take(j.min(p.len())).skip(i).copied());
                out.push(self.at(b));
                out.dedup();
                Geom2::Poly(out)
            }
        }
    }

    pub fn placed(&self, m: &Place) -> Geom2 {
        match self {
            Geom2::Line { a, b } => Geom2::Line {
                a: m.point(*a),
                b: m.point(*b),
            },
            Geom2::Conic { c, u, v, t0, t1 } => Geom2::Conic {
                c: m.point(*c),
                u: m.vec(*u),
                v: m.vec(*v),
                t0: *t0,
                t1: *t1,
            },
            Geom2::Poly(p) => Geom2::Poly(p.iter().map(|q| m.point(*q)).collect()),
        }
    }

    /// The box round the curve: its low and high corners.
    pub fn bounds(&self) -> (Point2, Point2) {
        let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for q in self.points(1e-3) {
            lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
            hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
        }
        (lo, hi)
    }

    /// The runs of the curve where `inside` (of the parameter) holds, found on its points within
    /// `chord` and their ends settled by bisection.
    pub fn runs_where(&self, chord: f64, inside: impl Fn(f64) -> bool) -> Vec<Geom2> {
        self.runs_where_also(chord, &[], inside)
    }

    /// `runs_where`, looking at the parameters `also` (a hair either side, and halfway to the
    /// next) too: where the condition may change between samples.
    pub fn runs_where_also(&self, chord: f64, also: &[f64], inside: impl Fn(f64) -> bool) -> Vec<Geom2> {
        let (t0, t1) = self.range();
        let mut ts: Vec<f64> = match self {
            Geom2::Line { .. } => (0..=64).map(|k| k as f64 / 64.0).collect(),
            Geom2::Conic { u, v, .. } => {
                let n = conic_steps(u.length().max(v.length()), t1 - t0, chord).max(64);
                (0..=n).map(|k| t0 + (t1 - t0) * k as f64 / n as f64).collect()
            }
            Geom2::Poly(p) => {
                let mut ts = Vec::new();
                for i in 0..p.len().saturating_sub(1) {
                    for k in 0..4 {
                        ts.push(i as f64 + k as f64 / 4.0);
                    }
                }
                ts.push(t1);
                ts
            }
        };
        let hair = 1e-9 * (t1 - t0);
        for &t in also {
            ts.extend([t - hair, t, t + hair].into_iter().filter(|x| *x > t0 && *x < t1));
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup();
        if !also.is_empty() {
            // and between each two: a run may lie wholly between two of them
            let mids: Vec<f64> = ts.windows(2).map(|w| 0.5 * (w[0] + w[1])).collect();
            ts.extend(mids);
            ts.sort_by(f64::total_cmp);
        }
        let ins: Vec<bool> = ts.iter().map(|&t| inside(t)).collect();
        let edge = |a: f64, b: f64, ia: bool| {
            let (mut a, mut b) = (a, b);
            for _ in 0..50 {
                let m = 0.5 * (a + b);
                if inside(m) == ia {
                    a = m;
                } else {
                    b = m;
                }
            }
            0.5 * (a + b)
        };
        let mut out = Vec::new();
        let mut start: Option<f64> = ins[0].then_some(ts[0]);
        for k in 1..ts.len() {
            if ins[k] != ins[k - 1] {
                let x = edge(ts[k - 1], ts[k], ins[k - 1]);
                match start.take() {
                    Some(s) => {
                        if x > s {
                            out.push(self.sub(s, x));
                        }
                    }
                    None => start = Some(x),
                }
            }
        }
        if let Some(s) = start
            && t1 > s
        {
            out.push(self.sub(s, t1));
        }
        out
    }
}

/// Chords for a conic of largest radius `r` over `sweep` radians, each sagging at most `chord`.
pub fn conic_steps(r: f64, sweep: f64, chord: f64) -> usize {
    let x = (1.0 - chord / r.max(1e-12)).clamp(-1.0, 1.0);
    let step = (2.0 * dmath::acos(x)).max(1e-3);
    ((sweep.abs() / step).ceil() as usize).clamp(2, 4096)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conic_runs_inside_a_box() {
        let g = Geom2::Conic {
            c: Point2::ORIGIN,
            u: Vec2::new(10.0, 0.0),
            v: Vec2::new(0.0, 10.0),
            t0: 0.0,
            t1: dmath::TAU,
        };
        assert_eq!(g.circle(), Some((Point2::ORIGIN, 10.0)));
        let runs = g.runs_where(0.01, |t| g.at(t).x > 5.0);
        // x > 5 is the arc from -60 to 60 degrees, split at the seam
        let total: f64 = runs.iter().map(Geom2::length).sum();
        assert!((total - 10.0 * dmath::TAU / 3.0).abs() < 1e-3, "{total}");
    }
}
