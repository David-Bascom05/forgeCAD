//! The in-process stock for 2.5D: the stock box minus voids, each void a line/arc region cleared
//! from its floor upward (everything above a cleared floor is air, since 2.5D tools come from
//! above). A void is only recorded where the op provably cleared it:
//! - facing: the whole box down to its height;
//! - a pocket with stepover ≤ the tool radius: the opening R ∘ B_r = (R ⊖ B_r) ⊕ B_r (every point
//!   of R ⊖ B_r is within one stepover of a pass, every pass offset from the original boundary);
//! - a contour: the band its loop sweeps, (C ⊕ B_r) − (C ⊖ B_r), C the tool-centre loop's region;
//! - a drilled hole: its circle, down to where the drill's full diameter reaches.
//! So the modelled stock contains the true in-process stock, and a clearance proven against it
//! holds against the true one (G2, G3).
use forge_cam_geom::boolean::{Op, boolean, region_of};
use forge_cam_geom::check::{Face, Surf, TSeg, Trim};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::region::{Loop, Region, Seg, p2};
use forge_math::dmath;

#[derive(Clone, Debug)]
pub struct InProcess {
    pub min: [f64; 3],
    pub max: [f64; 3],
    /// (region, floor): cleared from the floor up.
    pub voids: Vec<(Region, f64)>,
}

fn rect(a: [f64; 3], b: [f64; 3]) -> Region {
    let (p, q, r, s) = (p2(a[0], a[1]), p2(b[0], a[1]), p2(b[0], b[1]), p2(a[0], b[1]));
    Region {
        loops: vec![Loop {
            segs: vec![
                Seg::Line { a: p, b: q },
                Seg::Line { a: q, b: r },
                Seg::Line { a: r, b: s },
                Seg::Line { a: s, b: p },
            ],
        }],
    }
}

/// Arcs as pieces of at most a quarter turn (the checker's trims take less than a half).
fn arc_pieces(a: [f64; 2], b: [f64; 2], c: [f64; 2], ccw: bool, full: bool) -> Vec<([f64; 2], [f64; 2])> {
    let r = ((a[0] - c[0]).powi(2) + (a[1] - c[1]).powi(2)).sqrt();
    let t0 = dmath::atan2(a[1] - c[1], a[0] - c[0]);
    let mut t1 = if full {
        t0 + if ccw {
            std::f64::consts::TAU
        } else {
            -std::f64::consts::TAU
        }
    } else {
        dmath::atan2(b[1] - c[1], b[0] - c[0])
    };
    if !full {
        if ccw {
            while t1 <= t0 {
                t1 += std::f64::consts::TAU;
            }
        } else {
            while t1 >= t0 {
                t1 -= std::f64::consts::TAU;
            }
        }
    }
    let n = (((t1 - t0).abs() / (0.5 * std::f64::consts::PI)).ceil() as usize).max(1);
    let mut out = vec![];
    let mut prev = a;
    for k in 1..=n {
        let p = if k == n {
            b
        } else {
            let t = t0 + (t1 - t0) * k as f64 / n as f64;
            let (s, co) = dmath::sin_cos(t);
            [c[0] + r * co, c[1] + r * s]
        };
        out.push((prev, p));
        prev = p;
    }
    out
}

impl InProcess {
    pub fn new(min: [f64; 3], max: [f64; 3]) -> InProcess {
        InProcess {
            min,
            max,
            voids: vec![],
        }
    }

    pub fn cut(&mut self, region: Region, floor: f64) {
        if floor < self.max[2] {
            self.voids.push((region, floor));
        }
    }

    /// Facing the whole box to z.
    pub fn face_to(&mut self, z: f64) {
        let g = 1.0;
        self.cut(
            rect(
                [self.min[0] - g, self.min[1] - g, 0.0],
                [self.max[0] + g, self.max[1] + g, 0.0],
            ),
            z,
        );
    }

    /// The material's xy region in the band just above height z (None: no material).
    pub fn material(&self, z: f64) -> Result<Option<Region>, String> {
        let mut m = rect(self.min, self.max);
        for (v, floor) in &self.voids {
            if *floor <= z {
                let d = boolean(&m, v, Op::Diff).map_err(|e| format!("{e:?}"))?;
                if d.is_empty() {
                    return Ok(None);
                }
                m = region_of(&d);
            }
        }
        Ok(Some(m))
    }

    /// The heights where the stock's section changes.
    pub fn levels(&self) -> Vec<f64> {
        let mut zs: Vec<f64> = vec![self.min[2], self.max[2]];
        for (_, f) in &self.voids {
            if *f > self.min[2] && *f < self.max[2] {
                zs.push(*f);
            }
        }
        zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        zs.dedup();
        zs
    }

    /// The stock's boundary as the checker's faces (all prisms along z).
    pub fn faces(&self) -> Result<Vec<Face>, String> {
        let mu = 1e-9 * (1.0 + self.max[0].abs().max(self.max[1].abs()).max(self.max[2].abs()));
        let mut zs: Vec<f64> = vec![self.min[2], self.max[2]];
        for (_, f) in &self.voids {
            if *f > self.min[2] && *f < self.max[2] {
                zs.push(*f);
            }
        }
        zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        zs.dedup();
        let mut faces = vec![];
        let mut id = 2000u32;
        let mut below: Option<Region> = None;
        for k in 0..zs.len() - 1 {
            let (z0, z1) = (zs[k], zs[k + 1]);
            let m = self.material(z0)?;
            // the horizontal face at z0: material below that ends here, or material above that
            // starts here (the box's bottom)
            let exposed = match (&below, &m) {
                (None, Some(m)) => Some(m.clone()),
                (Some(b), Some(m)) => boolean(b, m, Op::Diff)
                    .ok()
                    .filter(|d| !d.is_empty())
                    .map(|d| region_of(&d)),
                (Some(b), None) => Some(b.clone()),
                (None, None) => None,
            };
            if let Some(e) = exposed {
                faces.push(flat(&e, z0, mu, id));
                id += 1;
            }
            if let Some(m) = &m {
                for l in &m.loops {
                    for s in &l.segs {
                        walls(s, z0, z1, mu, &mut id, &mut faces);
                    }
                }
            }
            below = m;
        }
        if let Some(b) = below {
            faces.push(flat(&b, *zs.last().unwrap(), mu, id));
        }
        Ok(faces)
    }
}

fn flat(r: &Region, z: f64, mu: f64, id: u32) -> Face {
    let mut segs = vec![];
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for l in &r.loops {
        for s in &l.segs {
            match *s {
                Seg::Line { a, b } => segs.push(TSeg::Line {
                    a: [a.x, a.y],
                    b: [b.x, b.y],
                }),
                Seg::Arc { a, b, c, ccw } => {
                    let rad = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
                    for (p, q) in arc_pieces([a.x, a.y], [b.x, b.y], [c.x, c.y], ccw, a == b) {
                        segs.push(TSeg::Arc {
                            a: p,
                            b: q,
                            c: [c.x, c.y],
                            r: rad,
                            ccw,
                        });
                    }
                    lo = [lo[0].min(c.x - rad), lo[1].min(c.y - rad)];
                    hi = [hi[0].max(c.x + rad), hi[1].max(c.y + rad)];
                }
            }
            let a = s.start();
            lo = [lo[0].min(a.x), lo[1].min(a.y)];
            hi = [hi[0].max(a.x), hi[1].max(a.y)];
        }
    }
    Face::new(
        Surf::Plane {
            o: [0.0, 0.0, z],
            u: [1.0, 0.0, 0.0],
            v: [0.0, 1.0, 0.0],
        },
        Ival::new(lo[0] - mu, hi[0] + mu),
        Ival::new(lo[1] - mu, hi[1] + mu),
        Some(Trim { segs }),
        mu,
        id,
    )
}

fn walls(s: &Seg, z0: f64, z1: f64, mu: f64, id: &mut u32, out: &mut Vec<Face>) {
    match *s {
        Seg::Line { a, b } => {
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let len = (dx * dx + dy * dy).sqrt();
            if len == 0.0 {
                return;
            }
            let u = [dx / len, dy / len, 0.0];
            let h = z1 - z0;
            let trim = Trim {
                segs: vec![
                    TSeg::Line {
                        a: [0.0, 0.0],
                        b: [len, 0.0],
                    },
                    TSeg::Line {
                        a: [len, 0.0],
                        b: [len, h],
                    },
                    TSeg::Line {
                        a: [len, h],
                        b: [0.0, h],
                    },
                    TSeg::Line {
                        a: [0.0, h],
                        b: [0.0, 0.0],
                    },
                ],
            };
            // the unit direction is rounded: the wall is within mu of the true one
            out.push(Face::new(
                Surf::Plane {
                    o: [a.x, a.y, z0],
                    u,
                    v: [0.0, 0.0, 1.0],
                },
                Ival::new(0.0, len),
                Ival::new(0.0, h),
                Some(trim),
                mu + 4.0 * f64::EPSILON * len,
                *id,
            ));
            *id += 1;
        }
        Seg::Arc { a, b, c, ccw } => {
            let rad = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
            for (p, q) in arc_pieces([a.x, a.y], [b.x, b.y], [c.x, c.y], ccw, a == b) {
                let (t0, t1) = (
                    dmath::atan2(p[1] - c.y, p[0] - c.x),
                    dmath::atan2(q[1] - c.y, q[0] - c.x),
                );
                let mut d = t1 - t0;
                if ccw && d < 0.0 {
                    d += std::f64::consts::TAU;
                }
                if !ccw && d > 0.0 {
                    d -= std::f64::consts::TAU;
                }
                let um = t0 + 0.5 * d;
                let (sm, cm) = dmath::sin_cos(um);
                let half = 0.5 * d.abs();
                let th = dmath::sin(0.5 * half) / dmath::cos(0.5 * half) * (1.0 + 1e-12);
                out.push(Face::new(
                    Surf::Cyl {
                        o: [c.x, c.y, z0],
                        x: [cm, sm, 0.0],
                        y: [-sm, cm, 0.0],
                        ax: [0.0, 0.0, 1.0],
                        r: rad,
                    },
                    Ival::new(-th, th),
                    Ival::new(0.0, z1 - z0),
                    None,
                    mu + 1e-12 * rad,
                    *id,
                ));
                *id += 1;
            }
        }
    }
}
