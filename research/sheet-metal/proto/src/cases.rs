//! The trim test set: a wall with n cuts, of a mix of kinds or all of one, seeded. Every number is
//! a function of the seed; the same case goes to every option.
use forge_geom::spline::PlaneSpline;
use forge_math::rng::Rng;
use forge_math::{Point2, dmath};
use forge_ops::{Profile, ProfileLoop, Segment};
use std::sync::Arc;

pub const W: f64 = 300.0;
pub const H: f64 = 150.0;
pub const WALL_R: f64 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Isolated,
    Overlap,
    Boundary,
    Tangent,
    Coincident,
    Mix,
    /// `Overlap` with no splines, to see what they cost.
    OverlapLines,
    /// `Coincident`, each cut moved 1e-8 to 1e-3 mm, log-uniform: near the tolerance, on both sides.
    Nudged,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::Isolated,
        Kind::Overlap,
        Kind::Boundary,
        Kind::Tangent,
        Kind::Coincident,
        Kind::Mix,
        Kind::OverlapLines,
        Kind::Nudged,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Kind::Isolated => "isolated",
            Kind::Overlap => "overlap",
            Kind::Boundary => "boundary",
            Kind::Tangent => "tangent",
            Kind::Coincident => "coincident",
            Kind::Mix => "mix",
            Kind::OverlapLines => "overlap-lines",
            Kind::Nudged => "nudged",
        }
    }
    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.name() == s)
    }
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Circle(Point2, f64),
    Rect(f64, f64, f64, f64),
    Other(Point2, f64),
}

pub struct Case {
    pub wall: Vec<ProfileLoop>,
    pub cuts: Vec<ProfileLoop>,
}

fn p(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}

fn lines(pts: &[Point2]) -> ProfileLoop {
    let n = pts.len();
    ProfileLoop::new(
        (0..n)
            .map(|i| {
                (
                    Segment::Line {
                        a: pts[i],
                        b: pts[(i + 1) % n],
                    },
                    i as u64,
                )
            })
            .collect(),
    )
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> ProfileLoop {
    lines(&[p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)])
}

fn rotated(c: Point2, w: f64, h: f64, a: f64) -> ProfileLoop {
    let (s, k) = dmath::sin_cos(a);
    let at = |x: f64, y: f64| p(c.x + k * x - s * y, c.y + s * x + k * y);
    lines(&[
        at(-w / 2.0, -h / 2.0),
        at(w / 2.0, -h / 2.0),
        at(w / 2.0, h / 2.0),
        at(-w / 2.0, h / 2.0),
    ])
}

/// A slot: two half circles of radius r on the ends of a straight run.
fn slot(c: Point2, len: f64, r: f64, a: f64) -> ProfileLoop {
    let (s, k) = dmath::sin_cos(a);
    let at = |x: f64, y: f64| p(c.x + k * x - s * y, c.y + s * x + k * y);
    let (e0, e1) = (at(-len / 2.0, 0.0), at(len / 2.0, 0.0));
    let q = dmath::PI / 2.0;
    ProfileLoop::new(vec![
        (
            Segment::Line {
                a: at(-len / 2.0, -r),
                b: at(len / 2.0, -r),
            },
            0,
        ),
        (
            Segment::Arc {
                center: e1,
                radius: r,
                start: a - q,
                sweep: 2.0 * q,
            },
            1,
        ),
        (
            Segment::Line {
                a: at(len / 2.0, r),
                b: at(-len / 2.0, r),
            },
            2,
        ),
        (
            Segment::Arc {
                center: e0,
                radius: r,
                start: a + q,
                sweep: 2.0 * q,
            },
            3,
        ),
    ])
}

/// A closed spline blob through a wobbling ring of points.
fn blob(c: Point2, r: f64, rng: &mut Rng) -> ProfileLoop {
    let n = 6 + rng.below(4);
    let mut pts: Vec<Point2> = (0..n)
        .map(|i| {
            let a = dmath::TAU * i as f64 / n as f64;
            let (s, k) = dmath::sin_cos(a);
            let q = r * rng.range(0.7, 1.0);
            p(c.x + q * k, c.y + q * s)
        })
        .collect();
    pts.push(pts[0]);
    let s = PlaneSpline::through(pts, None, None).expect("a ring of distinct points");
    ProfileLoop::new(vec![(Segment::Spline(Arc::new(s)), 0)])
}

fn circle(c: Point2, r: f64) -> ProfileLoop {
    Profile::circle(c, r, 0)
}

struct Gen {
    rng: Rng,
    s: f64,
    splines: bool,
    shapes: Vec<Shape>,
    cuts: Vec<ProfileLoop>,
}

impl Gen {
    fn add(&mut self, l: ProfileLoop, sh: Shape) {
        self.cuts.push(l);
        self.shapes.push(sh);
    }
    fn here(&mut self) -> Point2 {
        let m = 8.0 + self.s;
        p(self.rng.range(m, W - m), self.rng.range(m, H - m))
    }
    fn size(&mut self) -> f64 {
        self.s * self.rng.range(0.4, 1.0)
    }
    fn clear(&self, c: Point2, r: f64) -> bool {
        self.shapes.iter().all(|s| {
            let (q, rr) = match *s {
                Shape::Circle(q, r) | Shape::Other(q, r) => (q, r),
                Shape::Rect(x, y, w, h) => (p(x + w / 2.0, y + h / 2.0), 0.5 * (w * w + h * h).sqrt()),
            };
            q.distance(c) > r + rr + 0.5
        })
    }
    fn any_shape(&mut self, c: Point2, r: f64) {
        match self.rng.below(5) {
            0 | 1 => self.add(circle(c, r), Shape::Circle(c, r)),
            2 => {
                let a = self.rng.range(0.0, 3.0);
                let (w, h) = (r * 1.4, r);
                self.add(rotated(c, w, h, a), Shape::Other(c, r));
            }
            3 => {
                let a = self.rng.range(0.0, 3.0);
                let q = r * 0.4;
                self.add(slot(c, r, q, a), Shape::Other(c, r));
            }
            _ if !self.splines => self.add(circle(c, r), Shape::Circle(c, r)),
            _ => {
                let l = blob(c, r, &mut self.rng);
                self.add(l, Shape::Other(c, r));
            }
        }
    }
    fn isolated(&mut self) {
        for _ in 0..200 {
            let (c, r) = (self.here(), self.size());
            if self.clear(c, r) {
                self.any_shape(c, r);
                return;
            }
        }
        // full: a small one anywhere
        let c = self.here();
        self.add(circle(c, 1.0), Shape::Circle(c, 1.0));
    }
    fn anchor(&mut self) -> Shape {
        if self.shapes.is_empty() {
            let (c, r) = (self.here(), self.size());
            self.add(circle(c, r), Shape::Circle(c, r));
        }
        let i = self.rng.below(self.shapes.len());
        self.shapes[i]
    }
    fn overlap(&mut self) {
        let a = self.anchor();
        let (c, r) = match a {
            Shape::Circle(c, r) | Shape::Other(c, r) => (c, r),
            Shape::Rect(x, y, w, h) => (p(x + w / 2.0, y + h / 2.0), 0.5 * w.max(h)),
        };
        let ang = self.rng.range(0.0, dmath::TAU);
        let (s, k) = dmath::sin_cos(ang);
        let d = r * self.rng.range(0.5, 1.3);
        let q = p(
            (c.x + d * k).clamp(2.0, W - 2.0),
            (c.y + d * s).clamp(2.0, H - 2.0),
        );
        let rr = self.size();
        self.any_shape(q, rr);
    }
    fn boundary(&mut self) {
        let r = self.size();
        let c = match self.rng.below(5) {
            0 => p(self.rng.range(20.0, W - 20.0), 0.0),
            1 => p(self.rng.range(20.0, W - 20.0), H),
            2 => p(0.0, self.rng.range(20.0, H - 20.0)),
            3 => p(W, self.rng.range(20.0, H - 20.0)),
            _ => p(
                if self.rng.bool() { 1.0 } else { W - 1.0 },
                if self.rng.bool() { 1.0 } else { H - 1.0 },
            ),
        };
        self.any_shape(c, r);
    }
    fn tangent(&mut self) {
        let r = self.size();
        match self.rng.below(5) {
            // a hole tangent to the wall's edge from inside: the wall pinches
            0 => {
                let x = self.rng.range(20.0, W - 20.0);
                let c = p(x, r);
                self.add(circle(c, r), Shape::Circle(c, r));
            }
            // tangent to the edge from outside: touches, removes nothing
            1 => {
                let x = self.rng.range(20.0, W - 20.0);
                let c = p(x, H + r);
                self.add(circle(c, r), Shape::Circle(c, r));
            }
            // two circles touching from outside
            2 => {
                let (c0, r0) = self.a_circle();
                let a = self.rng.range(0.0, dmath::TAU);
                let (s, k) = dmath::sin_cos(a);
                let c = p(c0.x + (r0 + r) * k, c0.y + (r0 + r) * s);
                self.add(circle(c, r), Shape::Circle(c, r));
            }
            // one inside another, touching
            3 => {
                let (c0, r0) = self.a_circle();
                let a = self.rng.range(0.0, dmath::TAU);
                let (s, k) = dmath::sin_cos(a);
                let q = r0 * 0.5;
                let c = p(c0.x + (r0 - q) * k, c0.y + (r0 - q) * s);
                self.add(circle(c, q), Shape::Circle(c, q));
            }
            // a square whose side touches a circle
            _ => {
                let (c0, r0) = self.a_circle();
                let y = c0.y + r0;
                let (w, h) = (2.0 * r, r);
                let x = c0.x - w * self.rng.range(0.2, 0.8);
                self.add(rect(x, y, w, h), Shape::Rect(x, y, w, h));
            }
        }
    }
    fn a_circle(&mut self) -> (Point2, f64) {
        let found: Vec<(Point2, f64)> = self
            .shapes
            .iter()
            .filter_map(|s| match *s {
                Shape::Circle(c, r) => Some((c, r)),
                _ => None,
            })
            .collect();
        if !found.is_empty() {
            return found[self.rng.below(found.len())];
        }
        let (c, r) = (self.here(), self.size());
        self.add(circle(c, r), Shape::Circle(c, r));
        (c, r)
    }
    fn a_rect(&mut self) -> (f64, f64, f64, f64) {
        let found: Vec<_> = self
            .shapes
            .iter()
            .filter_map(|s| match *s {
                Shape::Rect(x, y, w, h) => Some((x, y, w, h)),
                _ => None,
            })
            .collect();
        if !found.is_empty() {
            return found[self.rng.below(found.len())];
        }
        let c = self.here();
        let (w, h) = (self.size() * 1.5, self.size());
        self.add(rect(c.x, c.y, w, h), Shape::Rect(c.x, c.y, w, h));
        (c.x, c.y, w, h)
    }
    fn coincident(&mut self) {
        let (w, h) = (self.size() * 1.5, self.size());
        match self.rng.below(6) {
            // flush with the wall's edge from inside: a notch
            0 => {
                let x = self.rng.range(20.0, W - 20.0 - w);
                self.add(rect(x, 0.0, w, h), Shape::Rect(x, 0.0, w, h));
            }
            // flush from outside: removes nothing
            1 => {
                let x = self.rng.range(20.0, W - 20.0 - w);
                self.add(rect(x, H, w, h), Shape::Rect(x, H, w, h));
            }
            // sharing part of a side with another rectangle, outside it
            2 => {
                let (x, y, rw, rh) = self.a_rect();
                let dy = rh * self.rng.range(-0.5, 0.5);
                self.add(rect(x + rw, y + dy, w, h), Shape::Rect(x + rw, y + dy, w, h));
            }
            // the same again
            3 => {
                let i = self.rng.below(self.cuts.len().max(1));
                if self.cuts.is_empty() {
                    let (x, y, rw, rh) = self.a_rect();
                    self.add(rect(x, y, rw, rh), Shape::Rect(x, y, rw, rh));
                } else {
                    let (l, s) = (self.cuts[i].clone(), self.shapes[i]);
                    self.add(l, s);
                }
            }
            // a slot whose end shares its circle
            4 => {
                let (c, r) = self.a_circle();
                let a = self.rng.range(0.0, dmath::TAU);
                let (s, k) = dmath::sin_cos(a);
                let len = 2.0 * r;
                let q = p(c.x + 0.5 * len * k, c.y + 0.5 * len * s);
                self.add(slot(q, len, r, a), Shape::Other(q, len));
            }
            // overlapping another rectangle with one side along its side
            _ => {
                let (x, y, rw, _) = self.a_rect();
                let dx = rw * self.rng.range(0.2, 0.8);
                self.add(rect(x + dx, y, w, h), Shape::Rect(x + dx, y, w, h));
            }
        }
    }
}

/// The case for this kind, count and seed.
pub fn case(kind: Kind, n: usize, seed: u64) -> Case {
    let mut g = Gen {
        rng: Rng::new(0x5e7a_0000 ^ (seed * 7919 + n as u64 * 31 + kind as u64)),
        s: (W * H / (8.0 * n as f64)).sqrt().clamp(3.0, 28.0),
        splines: kind != Kind::OverlapLines,
        shapes: Vec::new(),
        cuts: Vec::new(),
    };
    while g.cuts.len() < n {
        let k = match kind {
            Kind::Mix => match g.rng.below(100) {
                0..40 => Kind::Isolated,
                40..65 => Kind::Overlap,
                65..80 => Kind::Boundary,
                80..90 => Kind::Tangent,
                _ => Kind::Coincident,
            },
            k => k,
        };
        match k {
            Kind::Isolated => g.isolated(),
            Kind::Overlap | Kind::OverlapLines => g.overlap(),
            Kind::Boundary => g.boundary(),
            Kind::Tangent => g.tangent(),
            _ => g.coincident(),
        }
    }
    g.cuts.truncate(n);
    if kind == Kind::Nudged {
        for c in &mut g.cuts {
            let size = [1e-8, 1e-7, 1e-6, 1e-5, 1e-4][g.rng.below(5)] * g.rng.range(1.0, 10.0);
            let a = g.rng.range(0.0, dmath::TAU);
            let (s, k) = dmath::sin_cos(a);
            let (dx, dy) = (size * k, size * s);
            let by = |q: Point2| p(q.x + dx, q.y + dy);
            for (seg, _) in &mut c.segments {
                *seg = match seg.clone() {
                    Segment::Line { a, b } => Segment::Line { a: by(a), b: by(b) },
                    Segment::Arc {
                        center,
                        radius,
                        start,
                        sweep,
                    } => Segment::Arc {
                        center: by(center),
                        radius,
                        start,
                        sweep,
                    },
                    Segment::Circle { center, radius, ccw } => Segment::Circle {
                        center: by(center),
                        radius,
                        ccw,
                    },
                    other => other,
                };
            }
        }
    }
    Case {
        wall: vec![Profile::rounded_rectangle(p(0.0, 0.0), W, H, WALL_R, 0)],
        cuts: g.cuts,
    }
}
