//! Tier A of the fillet reference (research/fillets/general-method.md §5.4): whether a point outside
//! the body lies under no admissible ball, decided by branch and bound over the ball's possible
//! centres, never by the kernel.
//!
//! The obstacle is a [`Field`]: a signed distance (negative inside), 1-Lipschitz, with an error
//! bound on each value. For a box of centres with middle m and half-diagonal h, the distance anywhere
//! in the box lies within `sd(m) ± (err(m) + h)`. So:
//!
//! - a box whose upper bound is below r − τ holds no clear centre;
//! - a centre c with `sd(c) − err(c) ≥ r − τ` and |c − x| ≤ r is a witness that a clear ball covers
//!   x.
//!
//! Each answer is In (under no clear ball: filled), Out (a witness covers it) or Unknown (the search
//! reached its smallest box, or its box budget, without either). τ is the spec's tolerance
//! (§1.4): a distance within τ of the radius counts as touching.
//!
//! The error bounds come from counting flops. Each primitive's distance takes at most 40 operations
//! on values no larger than |p|∞ + its own size, each rounded once (unit roundoff u = 2⁻⁵³), so its
//! error is at most 40 u (|p|∞ + size). [`ERR_FLOPS`] doubles that. This is a forward-error count,
//! not yet a machine-checked one: discharging it the way research/loft-ssi's Lean files do is the
//! O9 obligation.
//!
//! The vertex table (§1.3, row 2: some edge at v unpicked) is a [`Zone`]: near such a vertex a point
//! is filled when every picked edge's wedge closing fills it, within the cap's half-spaces.

use forge_math::{Point3, Vec3};
use std::collections::BinaryHeap;

/// The slack on each primitive's error, in units of u (|p|∞ + size): twice a 40-flop count.
pub const ERR_FLOPS: f64 = 80.0;
const U: f64 = f64::EPSILON * 0.5;

fn inf_norm(p: Point3) -> f64 {
    p.x.abs().max(p.y.abs()).max(p.z.abs())
}

/// A solid by its signed distance: the kinds an analytic part's faces and their carried-on
/// surfaces make.
#[derive(Clone, Debug)]
pub enum Field {
    /// Material where (p − o)·n ≤ 0.
    Half { o: Point3, n: Vec3 },
    /// An infinite round bar about the line o + t d (material inside), or the hole of one
    /// (`solid` false: material outside).
    Bar {
        o: Point3,
        d: Vec3,
        radius: f64,
        solid: bool,
    },
    /// A capped cylinder: base centre o, axis d (unit), height h, radius.
    Rod { o: Point3, d: Vec3, h: f64, radius: f64 },
    /// A box: corner o, edges along x, y, z (unit, square to each other) of lengths w, l, h.
    Block {
        o: Point3,
        x: Vec3,
        y: Vec3,
        z: Vec3,
        w: f64,
        l: f64,
        h: f64,
    },
    /// The union of solids: the least distance.
    Union(Vec<Field>),
}

impl Field {
    /// The signed distance at p (negative inside).
    pub fn sd(&self, p: Point3) -> f64 {
        match self {
            Field::Half { o, n } => (p - *o).dot(*n) / n.length(),
            Field::Bar { o, d, radius, solid } => {
                let q = p - *o;
                let a = q.dot(*d) / d.length_sq();
                let rad = (q - *d * a).length();
                if *solid { rad - radius } else { radius - rad }
            }
            Field::Rod { o, d, h, radius } => {
                let v = p - *o;
                let a = v.dot(*d);
                let rad = (v - *d * a).length();
                let e = if a < 0.0 {
                    -a
                } else if a > *h {
                    a - h
                } else {
                    0.0
                };
                let q = (rad - radius).max(0.0);
                if e == 0.0 && q == 0.0 {
                    -(radius - rad).min(a).min(h - a)
                } else {
                    (e * e + q * q).sqrt()
                }
            }
            Field::Block { o, x, y, z, w, l, h } => {
                let v = p - *o;
                let (a, b, c) = (v.dot(*x), v.dot(*y), v.dot(*z));
                let ex = |t: f64, hi: f64| {
                    if t < 0.0 {
                        -t
                    } else if t > hi {
                        t - hi
                    } else {
                        0.0
                    }
                };
                let (ea, eb, ec) = (ex(a, *w), ex(b, *l), ex(c, *h));
                if ea == 0.0 && eb == 0.0 && ec == 0.0 {
                    -(a.min(w - a).min(b).min(l - b).min(c).min(h - c))
                } else {
                    (ea * ea + eb * eb + ec * ec).sqrt()
                }
            }
            Field::Union(fs) => fs.iter().map(|f| f.sd(p)).fold(f64::INFINITY, f64::min),
        }
    }

    /// A bound on |sd(p) as computed − sd(p) exact|.
    pub fn err(&self, p: Point3) -> f64 {
        let size = match self {
            Field::Half { o, .. } => inf_norm(*o),
            Field::Bar { o, radius, .. } => inf_norm(*o) + radius,
            Field::Rod { o, h, radius, .. } => inf_norm(*o) + h + radius,
            Field::Block { o, w, l, h, .. } => inf_norm(*o) + w + l + h,
            Field::Union(fs) => return fs.iter().map(|f| f.err(p)).fold(0.0, f64::max),
        };
        ERR_FLOPS * U * (inf_norm(p) + size + 1.0)
    }
}

/// An obstacle the search can ask: a 1-Lipschitz signed distance and a bound on its error. A
/// [`Field`] is one with its bounds counted; another implementation (a body's faces, say) states
/// its own, and a label from it is only as certified as that statement.
pub trait Obstacle: Sync {
    fn sd(&self, p: Point3) -> f64;
    fn err(&self, p: Point3) -> f64;
    /// An upper bound on sd(p), its error included: an obstacle that knows its distance but not
    /// cheaply its side may answer the unsigned distance here.
    fn upper(&self, p: Point3) -> f64 {
        self.sd(p) + self.err(p)
    }
    /// Whether the ball of radius `need` (τ off) at p certainly misses the solid.
    fn clear_at(&self, p: Point3, need: f64) -> bool {
        self.sd(p) - self.err(p) >= need
    }
}

impl Obstacle for Field {
    fn sd(&self, p: Point3) -> f64 {
        Field::sd(self, p)
    }
    fn err(&self, p: Point3) -> f64 {
        Field::err(self, p)
    }
}

/// How the search is bounded.
#[derive(Clone, Copy, Debug)]
pub struct Search {
    /// The spec's tolerance: a centre within τ of clear is clear.
    pub tau: f64,
    /// Boxes of centres no wider than this (half a side) are not split further.
    pub smallest: f64,
    /// Boxes looked at before giving up.
    pub budget: usize,
}

impl Search {
    pub fn for_radius(r: f64) -> Self {
        Search {
            tau: 1e-7,
            smallest: 1e-6 * r.max(1e-3),
            budget: 400_000,
        }
    }
}

/// Whether a clear ball of radius r covers x.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Covered {
    /// A witness: its centre.
    Yes(Point3),
    /// No clear centre lies within r of x.
    No,
    Unknown,
}

struct Cell {
    ub: f64,
    m: Point3,
    half: f64,
}
impl PartialEq for Cell {
    fn eq(&self, o: &Self) -> bool {
        self.ub == o.ub
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cell {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.ub.total_cmp(&o.ub)
    }
}

/// Branch and bound over the centres within r of x, largest possible clearance first.
pub fn covered(f: &(impl Obstacle + ?Sized), x: Point3, r: f64, s: &Search) -> Covered {
    let bound = |m: Point3, half: f64| f.upper(m) + half * 3f64.sqrt();
    let mut heap = BinaryHeap::new();
    heap.push(Cell {
        ub: bound(x, r),
        m: x,
        half: r,
    });
    let mut unknown = false;
    let mut looked = 0usize;
    let need = r - s.tau;
    while let Some(c) = heap.pop() {
        if c.ub < need {
            // every box left holds no clear centre
            break;
        }
        looked += 1;
        if looked > s.budget {
            return Covered::Unknown;
        }
        // the box's nearest point to x: past r, no centre in it reaches x
        let near = |m: f64, xv: f64| (xv - m).clamp(-c.half, c.half) + m;
        let q = Point3::new(near(c.m.x, x.x), near(c.m.y, x.y), near(c.m.z, x.z));
        if q.distance(x) > r * (1.0 + 4.0 * U) {
            continue;
        }
        // a witness: the box's middle, pulled into the ball
        let dm = c.m.distance(x);
        let w = if dm <= r {
            c.m
        } else {
            x + (c.m - x) * (r / dm * (1.0 - 1e-12))
        };
        if w.distance(x) <= r && f.clear_at(w, need) {
            return Covered::Yes(w);
        }
        if c.half <= s.smallest {
            unknown = true;
            continue;
        }
        let h = 0.5 * c.half;
        for i in 0..8 {
            let o = Vec3::new(
                if i & 1 == 0 { -h } else { h },
                if i & 2 == 0 { -h } else { h },
                if i & 4 == 0 { -h } else { h },
            );
            let m = c.m + o;
            heap.push(Cell {
                ub: bound(m, h),
                m,
                half: h,
            });
        }
    }
    if unknown { Covered::Unknown } else { Covered::No }
}

/// Near a vertex of the table's second row (some edge at it unpicked): within `rho` of `at`, a
/// point is filled when no wedge's clear ball covers it and it lies within every cap half-space.
#[derive(Clone, Debug)]
pub struct Zone {
    pub at: Point3,
    pub rho: f64,
    /// Each picked edge's wedge: its two faces' surfaces carried on, as solids.
    pub wedges: Vec<Field>,
    pub cap: Vec<Field>,
}

/// The spec for one radius class of concave picks whose reach is the whole closing away from the
/// zones (every concave edge picked).
#[derive(Clone, Debug)]
pub struct Spec {
    pub body: Field,
    pub r: f64,
    pub zones: Vec<Zone>,
    pub search: Search,
}

/// Where a point lies under the spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    /// In the body before the fillet.
    Body,
    /// Filled by the fillet.
    Fill,
    /// Left as air.
    Air,
    Unknown,
}

impl Spec {
    pub fn classify(&self, x: Point3) -> Where {
        let (d, e) = (self.body.sd(x), self.body.err(x));
        if d + e < 0.0 {
            return Where::Body;
        }
        if d - e <= 0.0 {
            return Where::Unknown;
        }
        let zone = self.zones.iter().find(|z| z.at.distance(x) < z.rho);
        let Some(z) = zone else {
            return match covered(&self.body, x, self.r, &self.search) {
                Covered::Yes(_) => Where::Air,
                Covered::No => Where::Fill,
                Covered::Unknown => Where::Unknown,
            };
        };
        if z.cap.iter().any(|c| c.sd(x) - c.err(x) > 0.0) {
            return Where::Air;
        }
        let mut unknown = z
            .cap
            .iter()
            .any(|c| c.sd(x) + c.err(x) >= 0.0 && c.sd(x) - c.err(x) <= 0.0);
        for w in &z.wedges {
            match covered(w, x, self.r, &self.search) {
                Covered::Yes(_) => return Where::Air,
                Covered::Unknown => unknown = true,
                Covered::No => {}
            }
        }
        if unknown { Where::Unknown } else { Where::Fill }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two half-spaces at a right angle (an L's inside corner along z): the fill under a ball of
    /// radius r is the strip between the walls and the ball's arc, r²(1 − π/4) per unit length.
    fn corner() -> Field {
        Field::Union(vec![
            Field::Half {
                o: Point3::new(0.0, 0.0, 0.0),
                n: Vec3::new(1.0, 0.0, 0.0),
            },
            Field::Half {
                o: Point3::new(0.0, 0.0, 0.0),
                n: Vec3::new(0.0, 1.0, 0.0),
            },
        ])
    }

    #[test]
    fn right_angle_corner_points() {
        let f = corner();
        let r = 2.0;
        let s = Search::for_radius(r);
        // the corner's own neighbourhood is filled; the ball's centre (2, 2) clears; along the
        // diagonal the arc lies at 2 − √2 from each wall
        let edge = 2.0 - 2f64.sqrt();
        for (t, filled) in [
            (0.1, true),
            (edge - 1e-3, true),
            (edge + 1e-3, false),
            (1.0, false),
        ] {
            let x = Point3::new(t, t, 5.0);
            let got = covered(&f, x, r, &s);
            assert_eq!(matches!(got, Covered::No), filled, "{t}: {got:?}");
            assert_eq!(matches!(got, Covered::Yes(_)), !filled, "{t}: {got:?}");
        }
        // along a wall, past the ball's touch point, the wall is open
        assert!(matches!(
            covered(&f, Point3::new(0.05, 2.5, 0.0), r, &s),
            Covered::Yes(_)
        ));
    }

    #[test]
    fn strip_area_by_sampling() {
        // the fill's section by a grid of points: r²(1 − π/4) within the grid's own error
        let f = corner();
        let r = 1.0;
        let s = Search::for_radius(r);
        let n = 120;
        let h = r / n as f64;
        let mut filled = 0usize;
        for i in 0..n {
            for j in 0..n {
                let x = Point3::new((i as f64 + 0.5) * h, (j as f64 + 0.5) * h, 0.0);
                if covered(&f, x, r, &s) == Covered::No {
                    filled += 1;
                }
            }
        }
        let area = filled as f64 * h * h;
        let exact = r * r * (1.0 - std::f64::consts::FRAC_PI_4);
        assert!((area - exact).abs() < 4.0 * h * r, "{area} against {exact}");
    }

    #[test]
    fn rod_and_block_distances() {
        let rod = Field::Rod {
            o: Point3::new(0.0, 0.0, 0.0),
            d: Vec3::new(0.0, 0.0, 1.0),
            h: 10.0,
            radius: 2.0,
        };
        assert!((rod.sd(Point3::new(5.0, 0.0, 5.0)) - 3.0).abs() < 1e-15);
        assert!((rod.sd(Point3::new(0.0, 0.0, 12.0)) - 2.0).abs() < 1e-15);
        assert!((rod.sd(Point3::new(1.0, 0.0, 5.0)) + 1.0).abs() < 1e-15);
        let block = Field::Block {
            o: Point3::new(0.0, 0.0, 0.0),
            x: Vec3::new(1.0, 0.0, 0.0),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(0.0, 0.0, 1.0),
            w: 2.0,
            l: 2.0,
            h: 2.0,
        };
        assert!((block.sd(Point3::new(3.0, 3.0, 1.0)) - 2f64.sqrt()).abs() < 1e-15);
        assert!((block.sd(Point3::new(1.0, 1.0, 1.0)) + 1.0).abs() < 1e-15);
    }
}
