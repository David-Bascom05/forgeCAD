//! Shared builders and the independent residual oracle (prior-art §11.1): plain frame geometry,
//! written apart from the solver's rows.
#![allow(dead_code)]

use forge_asm::*;
use forge_math::{Frame, Point3, Vec3, dmath};

pub fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}
pub fn v(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z)
}
pub fn frame(o: Point3, z: Vec3, x: Vec3) -> Frame {
    Frame::from_zx(o, z.normalize().unwrap(), x).unwrap()
}
pub fn at(x: f64, y: f64, z: f64) -> Frame {
    Frame::WORLD.with_origin(p(x, y, z))
}
/// A frame at `o` turned by `a` about world z.
pub fn rz(o: Point3, a: f64) -> Frame {
    Frame::WORLD.rotated_about_z(a).with_origin(o)
}

pub fn part(n: &str) -> PartRef {
    PartRef::new(n, 0)
}

pub fn inst(id: u64, pose: Frame) -> Instance {
    Instance::part(id, part("p"), pose)
}

/// A catalog with one anchorless part "p" (joints use implicit anchors).
pub fn catalog() -> Catalog {
    let mut c = Catalog::default();
    c.parts.insert((DocId("p".into()), 0), PartInfo::default());
    c
}

pub fn joint(id: u64, kind: JointKind, a: (u64, Frame), b: (u64, Frame)) -> Joint {
    Joint::new(id, kind, End::local(&[a.0], a.1), End::local(&[b.0], b.1))
}

/// The world frame of a joint end from the solved poses (local anchors only).
pub fn end_world(sol: &Solution, e: &End) -> Frame {
    let pose = sol.pose(e.path[0]).expect("a placed instance");
    match &e.anchor {
        AnchorRef::Local(f) => pose.compose(&f.frame),
        AnchorRef::Named(_) => panic!("oracle takes local anchors"),
    }
}

fn angle(rel_x: Vec3) -> f64 {
    dmath::atan2(rel_x.y, rel_x.x)
}

/// The largest violation of a joint's defining relations, from world frames (mm; angles × 100).
pub fn oracle(j: &Joint, sol: &Solution) -> f64 {
    let a0 = end_world(sol, &j.a);
    let a = if j.flip { a0.flipped() } else { a0 };
    let b = end_world(sol, &j.b);
    let d = a.origin - b.origin;
    let (xa, za) = (a.x().get(), a.z().get());
    let (xb, yb, zb) = (b.x().get(), b.y().get(), b.z().get());
    let s = 100.0;
    let parallel = || {
        za.cross(zb)
            .length()
            .max(if za.dot(zb) > 0.0 { 0.0 } else { 1.0 })
            * s
    };
    let mut e: f64 = match j.kind {
        JointKind::Fastened => d.length().max((xa - xb).length() * s).max((za - zb).length() * s),
        JointKind::Revolute => d.length().max(parallel()),
        JointKind::Slider => d.cross(zb).length().max(parallel()).max((xa - xb).length() * s),
        JointKind::Cylindrical => d.cross(zb).length().max(parallel()),
        JointKind::PinSlot => d.cross(xb).length().max(parallel()),
        JointKind::Planar => d.dot(zb).abs().max(parallel()),
        JointKind::Ball => d.length(),
    };
    for &(f, want) in &j.drive {
        let local = v(xa.dot(xb), xa.dot(yb), 0.0);
        let got = match f {
            Freedom::Tx => d.dot(xb),
            Freedom::Ty => d.dot(yb),
            Freedom::Tz => d.dot(zb),
            _ => angle(local),
        };
        let gap = if f.is_angle() {
            let t = got - want;
            (t - dmath::TAU * (t / dmath::TAU).round()).abs() * s
        } else {
            (got - want).abs()
        };
        e = e.max(gap);
    }
    e
}

pub fn worst(asm: &Assembly, sol: &Solution) -> f64 {
    asm.joints.iter().map(|j| oracle(j, sol)).fold(0.0, f64::max)
}

/// A planar four-bar in the xy plane: ground pivots at the origin and (d, 0), crank a, coupler
/// b, rocker c; crank at `t2`, open branch. Instances: 1 ground, 2 crank, 3 coupler, 4 rocker.
pub fn four_bar(a: f64, b: f64, c: f64, d: f64, t2: f64) -> Assembly {
    let (t3, t4) = four_bar_angles(a, b, c, d, t2).expect("a closing four-bar");
    let (s2, c2) = dmath::sin_cos(t2);
    let pp = p(a * c2, a * s2, 0.0);
    let instances = vec![
        inst(1, Frame::WORLD).grounded(),
        inst(2, rz(p(0.0, 0.0, 0.0), t2)),
        inst(3, rz(pp, t3)),
        inst(4, rz(p(d, 0.0, 0.0), t4)),
    ];
    let joints = vec![
        joint(1, JointKind::Revolute, (2, Frame::WORLD), (1, Frame::WORLD)),
        joint(2, JointKind::Revolute, (3, Frame::WORLD), (2, at(a, 0.0, 0.0))),
        joint(3, JointKind::Revolute, (4, at(c, 0.0, 0.0)), (3, at(b, 0.0, 0.0))),
        joint(4, JointKind::Revolute, (4, Frame::WORLD), (1, at(d, 0.0, 0.0))),
    ];
    Assembly {
        instances,
        joints,
        ..Assembly::default()
    }
}

/// Coupler and rocker angles of the open four-bar at crank angle `t2`.
pub fn four_bar_angles(a: f64, b: f64, c: f64, d: f64, t2: f64) -> Option<(f64, f64)> {
    let (s2, c2) = dmath::sin_cos(t2);
    let pp = v(a * c2, a * s2, 0.0);
    let o4 = v(d, 0.0, 0.0);
    let e = o4 - pp;
    let l = e.length();
    if l > b + c || l < (b - c).abs() {
        return None;
    }
    // the coupler's far end: on the circle of radius b about P and radius c about O4
    let x = (b * b - c * c + l * l) / (2.0 * l);
    let h = (b * b - x * x).max(0.0).sqrt();
    let u = e / l;
    let n = v(-u.y, u.x, 0.0);
    let q = pp + u * x + n * h;
    let t3 = dmath::atan2(q.y - pp.y, q.x - pp.x);
    let t4 = dmath::atan2(q.y - o4.y, q.x - o4.x);
    Some((t3, t4))
}

/// Grounded instance 1 with a crank (2) on a revolute at the origin, a rod (3) of length `l`, and
/// a slider block (4) on a revolute to the rod and a slider along world x.
pub fn slider_crank(r: f64, l: f64, t: f64) -> Assembly {
    let (s, c) = dmath::sin_cos(t);
    let pp = p(r * c, r * s, 0.0);
    let bx = r * c + (l * l - r * r * s * s).sqrt();
    let t3 = dmath::atan2(-pp.y, bx - pp.x);
    let along_x = frame(p(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 1.0, 0.0));
    Assembly {
        instances: vec![
            inst(1, Frame::WORLD).grounded(),
            inst(2, rz(p(0.0, 0.0, 0.0), t)),
            inst(3, rz(pp, t3)),
            inst(4, at(bx, 0.0, 0.0)),
        ],
        joints: vec![
            joint(1, JointKind::Revolute, (2, Frame::WORLD), (1, Frame::WORLD)),
            joint(2, JointKind::Revolute, (3, Frame::WORLD), (2, at(r, 0.0, 0.0))),
            joint(3, JointKind::Revolute, (4, Frame::WORLD), (3, at(l, 0.0, 0.0))),
            joint(4, JointKind::Slider, (4, along_x), (1, along_x)),
        ],
        ..Assembly::default()
    }
}

/// Rank of a dense row-major matrix by full pivoting (the oracle's own, not the solver's QR).
pub fn rank(a: &[f64], m: usize, n: usize, tol: f64) -> usize {
    let mut a = a.to_vec();
    let mut r = 0;
    let mut rows: Vec<usize> = (0..m).collect();
    let mut cols: Vec<usize> = (0..n).collect();
    while r < m.min(n) {
        let (mut bi, mut bj, mut best) = (r, r, 0.0);
        for i in r..m {
            for j in r..n {
                let x = a[rows[i] * n + cols[j]].abs();
                if x > best {
                    best = x;
                    bi = i;
                    bj = j;
                }
            }
        }
        if best <= tol {
            break;
        }
        rows.swap(r, bi);
        cols.swap(r, bj);
        let pr = rows[r];
        let piv = a[pr * n + cols[r]];
        for &ri in &rows[r + 1..] {
            let f = a[ri * n + cols[r]] / piv;
            for &cj in &cols[r..] {
                a[ri * n + cj] -= f * a[pr * n + cj];
            }
        }
        r += 1;
    }
    r
}

/// Degrees of freedom by finite differences of the oracle's own constraint functions (every
/// joint's defining vector relations, unsigned), over 6 motions per ungrounded instance.
pub fn fd_mobility(asm: &Assembly, sol: &Solution) -> usize {
    let free: Vec<InstanceId> = asm.instances.iter().filter(|i| !i.ground).map(|i| i.id).collect();
    let n = 6 * free.len();
    let base: Vec<(InstanceId, Frame)> = sol.poses.clone();
    let eval = |poses: &[(InstanceId, Frame)]| -> Vec<f64> {
        let get = |id: InstanceId| poses.iter().find(|p| p.0 == id).unwrap().1;
        let w = |e: &End| match &e.anchor {
            AnchorRef::Local(f) => get(e.path[0]).compose(&f.frame),
            _ => unreachable!(),
        };
        let mut out = Vec::new();
        for j in &asm.joints {
            let a0 = w(&j.a);
            let a = if j.flip { a0.flipped() } else { a0 };
            let b = w(&j.b);
            let d = a.origin - b.origin;
            let (xa, za) = (a.x().get(), a.z().get());
            let (xb, zb) = (b.x().get(), b.z().get());
            let mut put = |u: Vec3| out.extend([u.x, u.y, u.z]);
            match j.kind {
                JointKind::Fastened => {
                    put(d);
                    put(za.cross(zb));
                    put(xa.cross(xb));
                }
                JointKind::Revolute => {
                    put(d);
                    put(za.cross(zb));
                }
                JointKind::Slider => {
                    put(d.cross(zb));
                    put(za.cross(zb));
                    put(xa.cross(xb));
                }
                JointKind::Cylindrical => {
                    put(d.cross(zb));
                    put(za.cross(zb));
                }
                JointKind::PinSlot => {
                    put(d.cross(b.x().get()));
                    put(za.cross(zb));
                }
                JointKind::Planar => {
                    put(v(d.dot(zb), 0.0, 0.0));
                    put(za.cross(zb));
                }
                JointKind::Ball => put(d),
            }
            for &(f, _) in &j.drive {
                let yb = b.y().get();
                out.push(match f {
                    Freedom::Tx => d.dot(xb),
                    Freedom::Ty => d.dot(yb),
                    Freedom::Tz => d.dot(zb),
                    _ => dmath::atan2(xa.dot(yb), xa.dot(xb)) * 100.0,
                });
            }
        }
        out
    };
    let moved = |k: usize, h: f64| {
        let mut ps = base.clone();
        let id = free[k / 6];
        let e = ps.iter_mut().find(|p| p.0 == id).unwrap();
        let f = e.1;
        let c = k % 6;
        e.1 = if c < 3 {
            let mut w = [0.0; 3];
            w[c] = h / 100.0;
            let rot = |u: Vec3| {
                let t = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
                let ax = v(w[0], w[1], w[2]) / t;
                let (s, co) = dmath::sin_cos(t);
                u * co + ax.cross(u) * s + ax * (ax.dot(u) * (1.0 - co))
            };
            Frame::from_zx(f.origin, rot(f.z().get()).normalize().unwrap(), rot(f.x().get())).unwrap()
        } else {
            let mut t = [0.0; 3];
            t[c - 3] = h;
            f.with_origin(f.origin + v(t[0], t[1], t[2]))
        };
        ps
    };
    let h = 1e-6;
    let m = eval(&base).len();
    let mut jac = vec![0.0; m * n];
    for k in 0..n {
        let (fp, fm) = (eval(&moved(k, h)), eval(&moved(k, -h)));
        for i in 0..m {
            jac[i * n + k] = (fp[i] - fm[i]) / (2.0 * h);
        }
    }
    n - rank(&jac, m, n, 1e-6)
}

pub fn count(sol: &Solution, name: &str) -> usize {
    sol.diagnostics.iter().filter(|d| d.name() == name).count()
}

pub fn redundant_rows(sol: &Solution) -> usize {
    sol.diagnostics
        .iter()
        .map(|d| match d {
            Diagnostic::Redundant { rows, .. } => *rows,
            _ => 0,
        })
        .sum()
}
