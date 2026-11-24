//! 3+2 orientation: the rotary joint values that put the tool along a work plane's normal.
//!
//! A work plane is given in the part's frame at home pose (its axes as the columns of a rotation;
//! its z the normal the tool points out along, tip toward gauge). On a machine with two rotary
//! joints the tool's axis reaches it when
//!
//! - table-table: R_a(θa) R_b(θb) d = u (d the normal in the base frame at home, u the tool axis,
//!   a the joint nearer the base),
//! - head-head:   R_a(θa) R_b(θb) u = d,
//! - head-table:  R_t(−θt) R_h(θh) u = d,
//!
//! each the Paden–Kahan subproblem 2, exp(ω₁θ₁) exp(ω₂θ₂) p = q (Murray, Li, Sastry, "A
//! Mathematical Introduction to Robotic Manipulation", 1994, §3.2.2): up to two solutions, one
//! where they coincide, none when the normal is out of the joints' reach. A free joint (the normal
//! along its axis) is held at 0. Each solution is checked through the forward kinematics
//! (kin::tip_in_part), widened by whole turns on cyclic joints, and kept if every value is within
//! the joint's range. The chosen one honours each joint's `preference` (the sign it prefers, as
//! Fusion's createAxis), then has the smallest tilt (the non-cyclic joints' |angles|), then the
//! smallest cyclic travel; the rest are reported as alternatives.
use crate::def::{JointType, MachineDef};
use crate::kin::{self, Rigid, M3, V3};
use forge_math::dmath;

/// One rotary solution: each rotary joint's value (deg), the tilt (sum of the non-cyclic joints'
/// |values|), and the spin: the angle (deg) from the base's x to the work plane's x, seen along
/// the tool axis.
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    pub joints: Vec<(String, f64)>,
    pub tilt: f64,
    pub spin: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Orient {
    pub chosen: Pose,
    pub alternatives: Vec<Pose>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrientError {
    /// The plane's axes are not orthonormal and right-handed.
    Plane(String),
    /// The machine cannot orient (fewer than two rotary joints, parallel axes, no mounts).
    Machine(String),
    /// No rotation reaches the normal.
    Unreachable(String),
    /// Solutions exist but none within the joints' ranges: each (joint values, the joint out).
    OutOfRange(Vec<(Vec<(String, f64)>, String)>),
}

impl std::fmt::Display for OrientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrientError::Plane(s) => write!(f, "work plane: {s}"),
            OrientError::Machine(s) => write!(f, "machine: {s}"),
            OrientError::Unreachable(s) => write!(f, "unreachable: {s}"),
            OrientError::OutOfRange(c) => {
                write!(f, "every solution is outside a joint's range:")?;
                for (js, j) in c {
                    let v: Vec<String> = js.iter().map(|(n, x)| format!("{n} {x:.3}")).collect();
                    write!(f, " [{}: {j} out]", v.join(", "))?;
                }
                Ok(())
            }
        }
    }
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: V3) -> V3 {
    let l = norm(a);
    [a[0] / l, a[1] / l, a[2] / l]
}
fn col(m: &M3, c: usize) -> V3 {
    [m[0][c], m[1][c], m[2][c]]
}

/// Subproblem 1: the angle (rad) turning p into q about unit ω (None if their components off ω
/// differ, or vanish: then any angle).
fn sp1(w: V3, p: V3, q: V3) -> Option<Option<f64>> {
    let pp = kin::sub(p, kin::scale(w, dot(w, p)));
    let qq = kin::sub(q, kin::scale(w, dot(w, q)));
    let (np, nq) = (norm(pp), norm(qq));
    if (dot(w, p) - dot(w, q)).abs() > 1e-9 || (np - nq).abs() > 1e-9 {
        return None;
    }
    if np < 1e-12 {
        return Some(None);
    }
    Some(Some(dmath::atan2(dot(w, kin::cross(pp, qq)), dot(pp, qq))))
}

/// Subproblem 2: (θ1, θ2) with exp(ω1 θ1) exp(ω2 θ2) p = q, unit vectors, ω1 ∦ ω2. A free angle
/// (p along ω2, or q along ω1) is returned as 0.
fn sp2(w1: V3, w2: V3, p: V3, q: V3) -> Vec<(f64, f64)> {
    let c = dot(w1, w2);
    let den = c * c - 1.0;
    let a = (c * dot(w2, p) - dot(w1, q)) / den;
    let b = (c * dot(w1, q) - dot(w2, p)) / den;
    let x = kin::cross(w1, w2);
    let g2 = (dot(p, p) - a * a - b * b - 2.0 * a * b * c) / dot(x, x);
    if g2 < -1e-12 {
        return vec![];
    }
    let g = g2.max(0.0).sqrt();
    let mut out = Vec::new();
    for s in if g < 1e-12 { vec![0.0] } else { vec![g, -g] } {
        let z = kin::add(kin::add(kin::scale(w1, a), kin::scale(w2, b)), kin::scale(x, s));
        // exp(ω2 θ2) p = z, exp(ω1 θ1) z = q
        let (Some(t2), Some(t1)) = (sp1(w2, p, z), sp1(w1, z, q)) else {
            continue;
        };
        out.push((t1.unwrap_or(0.0), t2.unwrap_or(0.0)));
    }
    out
}

/// Degrees, snapped to a whole degree within 1e-9 (the solutions at right angles are then
/// exact) and to (−180, 180].
fn deg(r: f64) -> f64 {
    let mut d = r * (180.0 / dmath::PI);
    if (d - d.round()).abs() < 1e-9 {
        d = d.round();
    }
    while d <= -180.0 {
        d += 360.0;
    }
    while d > 180.0 {
        d -= 360.0;
    }
    if d == -0.0 {
        0.0
    } else {
        d
    }
}

/// The rotary joint values orienting the tool along `plane`'s z (the plane's axes as columns,
/// in the part's frame at home pose), and the alternatives.
pub fn solve(
    def: &MachineDef,
    part_mount: &str,
    tool_mount: &str,
    plane: &M3,
) -> Result<Orient, OrientError> {
    // the plane: orthonormal, right-handed
    let (px, py, pz) = (col(plane, 0), col(plane, 1), col(plane, 2));
    let ortho = [dot(px, py), dot(py, pz), dot(pz, px)]
        .iter()
        .all(|d| d.abs() < 1e-9)
        && [norm(px), norm(py), norm(pz)]
            .iter()
            .all(|n| (n - 1.0).abs() < 1e-9)
        && dot(kin::cross(px, py), pz) > 0.0;
    if !ortho {
        return Err(OrientError::Plane(
            "its axes are not orthonormal and right-handed".into(),
        ));
    }
    let rot: Vec<usize> = (0..def.joints.len())
        .filter(|j| def.joints[*j].ty == JointType::Rotary)
        .collect();
    if rot.len() != 2 {
        return Err(OrientError::Machine(format!(
            "{} rotary joints: 3+2 is solved for two",
            rot.len()
        )));
    }
    let home: Vec<f64> = vec![0.0; def.joints.len()];
    let pose0 = kin::pose(def, &home);
    let pf = kin::part_frame(def, &pose0, part_mount).ok_or(OrientError::Machine("no part mount".into()))?;
    let (_, u) =
        kin::tool_tip(def, &pose0, tool_mount, 0.0).ok_or(OrientError::Machine("no tool mount".into()))?;
    let d = unit(pf.dir(pz));
    let u = unit(u);
    // the joints in chain order (the one nearer the base first) and their sides
    let part_link = def.mount(part_mount).map(|m| m.link.clone()).unwrap_or_default();
    let tool_link = def.mount(tool_mount).map(|m| m.link.clone()).unwrap_or_default();
    let on = |link: &str, j: usize| link != "base" && def.chain(link).is_some_and(|c| c.contains(&j));
    let (j0, j1) = (rot[0], rot[1]);
    let axis = |j: usize| unit(def.joints[j].axis);
    // the axes in the base frame at home: a child's axis as its parent chain carries it at home
    // pose (every joint at 0: the record's axes)
    let in_part = |j: usize| on(&part_link, j);
    let in_tool = |j: usize| on(&tool_link, j);
    let order = |a: usize, b: usize, link: &str| -> (usize, usize) {
        let c = def.chain(link).unwrap_or_default();
        let ia = c.iter().position(|x| *x == a).unwrap_or(0);
        let ib = c.iter().position(|x| *x == b).unwrap_or(0);
        if ia <= ib {
            (a, b)
        } else {
            (b, a)
        }
    };
    // (ω1, ω2, p, q, map from (θ1, θ2) to (joint, value) pairs)
    type Map = Box<dyn Fn(f64, f64) -> [(usize, f64); 2]>;
    let (w1, w2, p, q, map): (V3, V3, V3, V3, Map) = if in_part(j0) && in_part(j1) {
        let (a, b) = order(j0, j1, &part_link);
        (axis(a), axis(b), d, u, Box::new(move |t1, t2| [(a, t1), (b, t2)]))
    } else if in_tool(j0) && in_tool(j1) {
        let (a, b) = order(j0, j1, &tool_link);
        (axis(a), axis(b), u, d, Box::new(move |t1, t2| [(a, t1), (b, t2)]))
    } else {
        let (t, h) = if in_part(j0) { (j0, j1) } else { (j1, j0) };
        if !(in_part(t) && in_tool(h)) {
            return Err(OrientError::Machine(
                "a rotary joint on neither the tool's nor the part's chain".into(),
            ));
        }
        // R_t(θt) d = R_h(θh) u  ⇔  exp(ωt (−θt)) exp(ωh θh) u = d
        (
            axis(t),
            axis(h),
            u,
            d,
            Box::new(move |t1, t2| [(t, -t1), (h, t2)]),
        )
    };
    if kin::norm(kin::cross(w1, w2)) < 1e-9 {
        return Err(OrientError::Machine("the two rotary axes are parallel".into()));
    }
    let raw = sp2(w1, w2, p, q);
    if raw.is_empty() {
        return Err(OrientError::Unreachable(format!(
            "the normal {pz:?} is out of the rotary joints' reach on {}",
            def.name
        )));
    }
    // candidates: each solution with whole turns on cyclic joints, within range
    let mut cands: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut out_of_range: Vec<(Vec<(String, f64)>, String)> = Vec::new();
    for (t1, t2) in raw {
        let base = map(t1, t2);
        let opts = |j: usize, v: f64| -> Vec<f64> {
            let v = deg(v);
            let jt = &def.joints[j];
            let mut o: Vec<f64> = if jt.cyclic {
                (-2..=2).map(|k| v + 360.0 * k as f64).collect()
            } else {
                vec![v]
            };
            if let Some([lo, hi]) = jt.range {
                o.retain(|x| *x >= lo - 1e-9 && *x <= hi + 1e-9);
            }
            o
        };
        let (a, va) = base[0];
        let (b, vb) = base[1];
        let (oa, ob) = (opts(a, va), opts(b, vb));
        if oa.is_empty() || ob.is_empty() {
            let named = vec![
                (def.joints[a].name.clone(), deg(va)),
                (def.joints[b].name.clone(), deg(vb)),
            ];
            let which = if oa.is_empty() {
                &def.joints[a].name
            } else {
                &def.joints[b].name
            };
            out_of_range.push((named, which.clone()));
            continue;
        }
        for x in &oa {
            for y in &ob {
                cands.push(vec![(a, *x), (b, *y)]);
            }
        }
    }
    // verify through the forward kinematics, and describe
    let mut poses: Vec<(Pose, (i32, f64, f64))> = Vec::new();
    for c in cands {
        let mut qv = home.clone();
        for (j, v) in &c {
            qv[*j] = *v;
        }
        let pz_now = kin::pose(def, &qv);
        let Some(f) = kin::part_frame(def, &pz_now, part_mount) else {
            continue;
        };
        let Some((_, ax)) = kin::tool_tip(def, &pz_now, tool_mount, 0.0) else {
            continue;
        };
        let n = f.dir(pz);
        if dot(unit(n), unit(ax)) < 1.0 - 1e-9 {
            continue;
        }
        // spin: the plane's x in the base, seen along the tool axis (base +z for a vertical tool)
        let xb = f.dir(px);
        let spin = deg(dmath::atan2(xb[1], xb[0]));
        let tilt: f64 = c
            .iter()
            .filter(|(j, _)| !def.joints[*j].cyclic)
            .map(|(_, v)| v.abs())
            .sum();
        let travel: f64 = c
            .iter()
            .filter(|(j, _)| def.joints[*j].cyclic)
            .map(|(_, v)| v.abs())
            .sum();
        let pref_miss: i32 = c
            .iter()
            .filter(|(j, v)| {
                let p = def.joints[*j].preference;
                p != 0 && *v != 0.0 && (v.signum() as i8) != p
            })
            .count() as i32;
        let pose = Pose {
            joints: c.iter().map(|(j, v)| (def.joints[*j].name.clone(), *v)).collect(),
            tilt,
            spin,
        };
        if !poses.iter().any(|(p, _)| p.joints == pose.joints) {
            poses.push((pose, (pref_miss, tilt, travel)));
        }
    }
    if poses.is_empty() {
        return Err(if out_of_range.is_empty() {
            OrientError::Unreachable("no solution survives the forward kinematics".into())
        } else {
            OrientError::OutOfRange(out_of_range)
        });
    }
    poses.sort_by(|a, b| {
        (a.1 .0, a.1 .1, a.1 .2)
            .partial_cmp(&(b.1 .0, b.1 .1, b.1 .2))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut it = poses.into_iter().map(|(p, _)| p);
    let chosen = it.next().unwrap();
    Ok(Orient {
        chosen,
        alternatives: it.collect(),
    })
}

/// The work plane's frame in the base at joint values `q`: plane coordinates (its origin `o` in
/// the part's frame, its axes `plane`) to base coordinates.
pub fn plane_frame(def: &MachineDef, q: &[f64], part_mount: &str, plane: &M3, o: V3) -> Option<Rigid> {
    let f = kin::part_frame(def, &kin::pose(def, q), part_mount)?;
    Some(f.then(&Rigid { r: *plane, p: o }))
}
