//! Tilted planes: convex box-like solids one of whose faces is turned by theta off the other's face
//! (examples/tilt_sweep.rs sweeps them), with the exact answer by convex clipping.
use forge_bool::Op;
use forge_math::dmath;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3};
use forge_ops::extrude;
use forge_ops::profile::{Profile, ProfileLoop, Segment};
use forge_topo::Body;

pub type P = [f64; 3];

thread_local! {
    /// A's prism is feature 1, B's 2 (each family builds A first).
    static NEXT: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
}

/// A convex polyhedron as its faces (outward, counter-clockwise seen from outside).
#[derive(Clone)]
pub struct Poly {
    pub faces: Vec<Vec<P>>,
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

impl Poly {
    pub fn volume(&self) -> f64 {
        let mut v = 0.0;
        for f in &self.faces {
            for i in 1..f.len().saturating_sub(1) {
                v += dot(f[0], cross(f[i], f[i + 1])) / 6.0;
            }
        }
        v
    }
    /// The outward planes (n, d): n.x <= d inside.
    pub fn planes(&self) -> Vec<(P, f64)> {
        self.faces
            .iter()
            .map(|f| {
                let mut n = [0.0; 3];
                for i in 0..f.len() {
                    let (a, b) = (f[i], f[(i + 1) % f.len()]);
                    n[0] += (a[1] - b[1]) * (a[2] + b[2]);
                    n[1] += (a[2] - b[2]) * (a[0] + b[0]);
                    n[2] += (a[0] - b[0]) * (a[1] + b[1]);
                }
                let l = dot(n, n).sqrt();
                let n = [n[0] / l, n[1] / l, n[2] / l];
                (n, dot(n, f[0]))
            })
            .collect()
    }
    /// The part with n.x <= d.
    pub fn clip(&self, n: P, d: f64) -> Poly {
        let mut faces = Vec::new();
        let mut cut: Vec<P> = Vec::new();
        // a face already on the plane facing its way is the cap: none is added
        let mut capped = false;
        for (f, (m, _)) in self.faces.iter().zip(self.planes()) {
            if dot(m, n) > 0.0
                && f.iter()
                    .all(|&a| (dot(n, a) - d).abs() <= 1e-12 * (1.0 + d.abs()))
            {
                capped = true;
            }
        }
        for f in &self.faces {
            let mut out = Vec::new();
            for i in 0..f.len() {
                let (a, b) = (f[i], f[(i + 1) % f.len()]);
                let eps = 1e-12 * (1.0 + d.abs());
                let snap = |v: f64| if v.abs() <= eps { 0.0 } else { v };
                let (sa, sb) = (snap(dot(n, a) - d), snap(dot(n, b) - d));
                if sa <= 0.0 {
                    out.push(a);
                }
                if (sa < 0.0 && sb > 0.0) || (sa > 0.0 && sb < 0.0) {
                    let t = sa / (sa - sb);
                    let p = [
                        a[0] + t * (b[0] - a[0]),
                        a[1] + t * (b[1] - a[1]),
                        a[2] + t * (b[2] - a[2]),
                    ];
                    out.push(p);
                    cut.push(p);
                } else if sa == 0.0 {
                    cut.push(a);
                }
            }
            if out.len() >= 3 {
                faces.push(out);
            }
        }
        if cut.len() >= 3 && !capped {
            let c = cut
                .iter()
                .fold([0.0; 3], |s, p| [s[0] + p[0], s[1] + p[1], s[2] + p[2]]);
            let c = [
                c[0] / cut.len() as f64,
                c[1] / cut.len() as f64,
                c[2] / cut.len() as f64,
            ];
            let u = {
                let far = cut.iter().copied().fold(cut[0], |m, p| {
                    if dot(sub(p, c), sub(p, c)) > dot(sub(m, c), sub(m, c)) {
                        p
                    } else {
                        m
                    }
                });
                sub(far, c)
            };
            let w = cross(n, u);
            cut.sort_by(|a, b| {
                let (pa, pb) = (sub(*a, c), sub(*b, c));
                dmath::atan2(dot(pa, w), dot(pa, u)).total_cmp(&dmath::atan2(dot(pb, w), dot(pb, u)))
            });
            cut.dedup_by(|a, b| dot(sub(*a, *b), sub(*a, *b)) < 1e-30);
            if cut.len() >= 3 {
                faces.push(cut);
            }
        }
        Poly { faces }
    }
    pub fn meet(&self, other: &Poly) -> Poly {
        let mut p = self.clone();
        for (n, d) in other.planes() {
            p = p.clip(n, d);
        }
        p
    }
}

/// A prism: the polygon `pts` (counter-clockwise in the frame's xy) extruded `h` along its z.
pub fn prism(fr: &Frame, pts: &[(f64, f64)], h: f64) -> (Body, Poly) {
    let feature = NEXT.with(|n| {
        let v = n.get();
        n.set(v % 2 + 1);
        v
    });
    let at = |x: f64, y: f64, z: f64| -> P {
        let p = fr.origin + fr.x().get() * x + fr.y().get() * y + fr.z().get() * z;
        [p.x, p.y, p.z]
    };
    let n = pts.len();
    let mut faces = vec![
        pts.iter().rev().map(|&(x, y)| at(x, y, 0.0)).collect::<Vec<_>>(),
        pts.iter().map(|&(x, y)| at(x, y, h)).collect::<Vec<_>>(),
    ];
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        faces.push(vec![
            at(a.0, a.1, 0.0),
            at(b.0, b.1, 0.0),
            at(b.0, b.1, h),
            at(a.0, a.1, h),
        ]);
    }
    let lp = ProfileLoop::new(
        (0..n)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                (
                    Segment::Line {
                        a: Point2::new(a.0, a.1),
                        b: Point2::new(b.0, b.1),
                    },
                    feature * 100 + i as u64,
                )
            })
            .collect(),
    );
    let body = extrude(
        &Profile {
            plane: *fr,
            loops: vec![lp],
        },
        h,
        feature,
    )
    .expect("a prism extrudes");
    (body, Poly { faces })
}

fn frame(o: P, z: P, x: P) -> Frame {
    let zu = UnitVec3::new_unchecked(Vec3::new(z[0], z[1], z[2]));
    Frame::from_zx(Point3::new(o[0], o[1], o[2]), zu, Vec3::new(x[0], x[1], x[2])).expect("a frame")
}

fn rect(x0: f64, y0: f64, w: f64, h: f64) -> Vec<(f64, f64)> {
    vec![(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]
}

fn cube() -> (Body, Poly) {
    prism(
        &frame([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        &rect(0.0, 0.0, 10.0, 10.0),
        10.0,
    )
}

/// The families: (A, B) at tilt theta (radians), each as a body and as the exact polyhedron.
pub fn family(name: &str, th: f64) -> ((Body, Poly), (Body, Poly)) {
    let (s, c) = dmath::sin_cos(th);
    match name {
        // B on A's top, its bottom turned about the line y = 5 in A's top face: half dips in
        "lid" => (
            cube(),
            prism(
                &frame([2.0, 5.0, 10.0], [0.0, -s, c], [1.0, 0.0, 0.0]),
                &rect(0.0, -3.0, 6.0, 6.0),
                5.0,
            ),
        ),
        // B in A's top, its top turned about the line y = 5 in A's top face (a pocket's floor... its lid)
        "plug" => (
            cube(),
            prism(
                &frame([2.0, 5.0, 10.0], [0.0, s, -c], [1.0, 0.0, 0.0]),
                &rect(0.0, -3.0, 6.0, 6.0),
                5.0,
            ),
        ),
        // B on A's top over its whole footprint, turned about the top's middle line y = 5
        "cap" => (
            cube(),
            prism(
                &frame([0.0, 5.0, 10.0], [0.0, -s, c], [1.0, 0.0, 0.0]),
                &rect(0.0, -5.0, 10.0, 10.0),
                5.0,
            ),
        ),
        // `cap` with B overhanging A in y: no end face of B near one of A's (no sub-LINEAR sliver)
        "cap_over" => (
            cube(),
            prism(
                &frame([0.0, 5.0, 10.0], [0.0, -s, c], [1.0, 0.0, 0.0]),
                &rect(0.0, -7.0, 10.0, 14.0),
                5.0,
            ),
        ),
        // `wall_flush` with B overhanging A in x
        "wall_over" => (
            cube(),
            prism(
                &frame([5.0, 10.0, 0.0], [0.0, 0.0, 1.0], [c, s, 0.0]),
                &rect(-7.0, 0.0, 14.0, 10.0),
                10.0,
            ),
        ),
        // B beside A (y = 10 face), turned about the vertical line x = 5 in that face
        "wall" => (
            cube(),
            prism(
                &frame([5.0, 10.0, 2.0], [0.0, 0.0, 1.0], [c, s, 0.0]),
                &rect(-3.0, 0.0, 6.0, 10.0),
                6.0,
            ),
        ),
        // B beside A over the whole face, turned about the vertical line x = 5: its edges all but run
        // along A's (the run-on test), their ends 5 theta off
        "wall_flush" => (
            cube(),
            prism(
                &frame([5.0, 10.0, 0.0], [0.0, 0.0, 1.0], [c, s, 0.0]),
                &rect(-5.0, 0.0, 10.0, 10.0),
                10.0,
            ),
        ),
        // B on A's top, its bottom flush, turned about A's vertical axis x = y = 5 by theta: no tilt of
        // planes, edges crossing at theta (a control: the flat path at an angle in the plane)
        "twist" => (
            cube(),
            prism(
                &frame([5.0, 5.0, 10.0], [0.0, 0.0, 1.0], [c, s, 0.0]),
                &rect(-3.0, -3.0, 6.0, 6.0),
                5.0,
            ),
        ),
        // A a roof: its top two faces meeting at the ridge x = 5 at 2 theta from flat; B a block across
        // the ridge through A's top (the across `smooth` test at the ridge)
        "ridge" => {
            let t = {
                let (s, c) = dmath::sin_cos(th);
                s / c
            };
            let a = prism(
                &frame([0.0, 0.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
                &[
                    (0.0, 0.0),
                    (10.0, 0.0),
                    (10.0, 10.0),
                    (5.0, 10.0 + 5.0 * t),
                    (0.0, 10.0),
                ],
                10.0,
            );
            // the frame's z is -y: the prism runs y = 0 .. -10; move B accordingly
            let b = prism(
                &frame([2.0, -7.0, 8.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                &rect(0.0, 0.0, 6.0, 4.0),
                4.0,
            );
            (a, b)
        }
        _ => panic!("family {name}"),
    }
}

pub const FAMILIES: [&str; 9] = [
    "lid",
    "plug",
    "cap",
    "cap_over",
    "wall",
    "wall_flush",
    "wall_over",
    "twist",
    "ridge",
];

pub fn reading(a: &Poly, b: &Poly, op: Op) -> f64 {
    let i = a.meet(b).volume();
    match op {
        Op::Union => a.volume() + b.volume() - i,
        Op::Subtract => a.volume() - i,
        Op::Intersect => i,
    }
}
