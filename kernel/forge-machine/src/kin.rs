//! Forward kinematics: joint values to the placement of every link, and the tool's tip and axis in
//! the part's frame. Rotations use forge-math's dmath (IEEE + − × ÷ √ only), so a pose is the
//! same bits on every target; whole multiples of 90° are exact.
//!
//! `IPose` is the same over intervals (outward rounded): every link's placement for joint values
//! anywhere in boxes, which the collision check uses to bound motion over a time span.
use crate::def::{JointType, MachineDef, MountKind};
use forge_math::dmath;
use forge_math::interval::Interval;

pub type V3 = [f64; 3];
/// Row-major rotation.
pub type M3 = [[f64; 3]; 3];

pub const I3: M3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}
pub fn mul(m: &M3, v: V3) -> V3 {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}
pub fn mm(a: &M3, b: &M3) -> M3 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    r
}
pub fn transpose(a: &M3) -> M3 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[j][i];
        }
    }
    r
}

/// sin and cos of an angle in degrees; exact at whole multiples of 90.
pub fn sin_cos_deg(q: f64) -> (f64, f64) {
    let r = q % 360.0;
    if r % 90.0 == 0.0 {
        let k = ((r / 90.0) as i64).rem_euclid(4);
        return [(0.0, 1.0), (1.0, 0.0), (0.0, -1.0), (-1.0, 0.0)][k as usize];
    }
    dmath::sin_cos(r * (dmath::PI / 180.0))
}

/// Rotation by q degrees about unit axis a (Rodrigues), right-handed.
pub fn rot(a: V3, q: f64) -> M3 {
    let (s, c) = sin_cos_deg(q);
    let t = 1.0 - c;
    let [x, y, z] = a;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
    ]
}

/// A rigid placement: x ↦ r·x + p.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rigid {
    pub r: M3,
    pub p: V3,
}

impl Rigid {
    pub const ID: Rigid = Rigid { r: I3, p: [0.0; 3] };
    pub fn apply(&self, x: V3) -> V3 {
        add(mul(&self.r, x), self.p)
    }
    pub fn dir(&self, x: V3) -> V3 {
        mul(&self.r, x)
    }
    /// self ∘ o
    pub fn then(&self, o: &Rigid) -> Rigid {
        Rigid {
            r: mm(&self.r, &o.r),
            p: self.apply(o.p),
        }
    }
    pub fn inverse(&self) -> Rigid {
        let rt = transpose(&self.r);
        Rigid {
            r: rt,
            p: scale(mul(&rt, self.p), -1.0),
        }
    }
}

/// The joint's own motion at value q.
pub fn joint_rigid(def: &MachineDef, j: usize, q: f64) -> Rigid {
    let jt = &def.joints[j];
    match jt.ty {
        JointType::Linear => Rigid {
            r: I3,
            p: scale(jt.axis, q),
        },
        JointType::Rotary => {
            let r = rot(jt.axis, q);
            // x ↦ r(x − c) + c
            Rigid {
                r,
                p: sub(jt.pivot, mul(&r, jt.pivot)),
            }
        }
    }
}

/// Every link's placement (indexed as def.joints) for joint values q.
pub fn pose(def: &MachineDef, q: &[f64]) -> Vec<Rigid> {
    let mut out = vec![Rigid::ID; def.joints.len()];
    let mut done = vec![false; def.joints.len()];
    for j in 0..def.joints.len() {
        place(def, q, j, &mut out, &mut done);
    }
    out
}

fn place(def: &MachineDef, q: &[f64], j: usize, out: &mut [Rigid], done: &mut [bool]) {
    if done[j] {
        return;
    }
    let parent = match def.joint_index(&def.joints[j].parent) {
        Some(p) => {
            place(def, q, p, out, done);
            out[p]
        }
        None => Rigid::ID,
    };
    out[j] = parent.then(&joint_rigid(def, j, q[j]));
    done[j] = true;
}

/// A link's placement from a pose ("base" is the identity).
pub fn link<'a>(def: &MachineDef, pose: &'a [Rigid], name: &str) -> &'a Rigid {
    match def.joint_index(name) {
        Some(i) => &pose[i],
        None => &Rigid::ID,
    }
}

/// The part mount's frame: part coordinates to base coordinates, at a pose.
pub fn part_frame(def: &MachineDef, pose: &[Rigid], mount: &str) -> Option<Rigid> {
    let m = def.mount(mount).filter(|m| m.kind == MountKind::Part)?;
    let y = cross(m.dir, m.xdir);
    // columns xdir, y, dir
    let local = Rigid {
        r: [
            [m.xdir[0], y[0], m.dir[0]],
            [m.xdir[1], y[1], m.dir[1]],
            [m.xdir[2], y[2], m.dir[2]],
        ],
        p: m.at,
    };
    Some(link(def, pose, &m.link).then(&local))
}

/// The tool tip and axis (tip toward gauge) in base coordinates, for a tool of gauge length l.
pub fn tool_tip(def: &MachineDef, pose: &[Rigid], mount: &str, l: f64) -> Option<(V3, V3)> {
    let m = def.mount(mount).filter(|m| m.kind == MountKind::Tool)?;
    let t = link(def, pose, &m.link);
    Some((t.apply(add(m.at, scale(m.dir, l))), t.dir(scale(m.dir, -1.0))))
}

/// The tool tip and axis in the part's frame.
pub fn tip_in_part(def: &MachineDef, q: &[f64], tool: &str, part: &str, l: f64) -> Option<(V3, V3)> {
    let p = pose(def, q);
    let (tip, ax) = tool_tip(def, &p, tool, l)?;
    let f = part_frame(def, &p, part)?.inverse();
    Some((f.apply(tip), f.dir(ax)))
}

// ------------------------------------------------------------------------------------------
// Intervals

pub type IV3 = [Interval; 3];
pub type IM3 = [[Interval; 3]; 3];

fn ipt(v: V3) -> IV3 {
    [
        Interval::point(v[0]),
        Interval::point(v[1]),
        Interval::point(v[2]),
    ]
}

fn imul(m: &IM3, v: &IV3) -> IV3 {
    let row = |r: &[Interval; 3]| r[0] * v[0] + r[1] * v[1] + r[2] * v[2];
    [row(&m[0]), row(&m[1]), row(&m[2])]
}

fn imm(a: &IM3, b: &IM3) -> IM3 {
    let mut r = [[Interval::point(0.0); 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    r
}

/// A placement for joint values in boxes: every true placement's r and p lie in these.
#[derive(Clone, Copy, Debug)]
pub struct IRigid {
    pub r: IM3,
    pub p: IV3,
}

impl IRigid {
    pub fn id() -> IRigid {
        let z = Interval::point(0.0);
        let o = Interval::point(1.0);
        IRigid {
            r: [[o, z, z], [z, o, z], [z, z, o]],
            p: [z, z, z],
        }
    }
    pub fn apply(&self, x: &IV3) -> IV3 {
        let a = imul(&self.r, x);
        [a[0] + self.p[0], a[1] + self.p[1], a[2] + self.p[2]]
    }
    pub fn then(&self, o: &IRigid) -> IRigid {
        IRigid {
            r: imm(&self.r, &o.r),
            p: self.apply(&o.p),
        }
    }
}

/// sin and cos of degrees over an interval, enclosing (dmath within 1 ulp, then widened by
/// Interval's own slack; the degree-to-radian product is rounded outward).
fn isin_cos_deg(q: Interval) -> (Interval, Interval) {
    let k = Interval::point(dmath::PI / 180.0);
    // PI/180 as a float is within 1 ulp of π/180: widen it by 2 ulps either side
    let k = Interval::new(k.lo.next_down().next_down(), k.hi.next_up().next_up());
    let r = q * k;
    (r.sin(), r.cos())
}

pub fn ijoint_rigid(def: &MachineDef, j: usize, q: Interval) -> IRigid {
    let jt = &def.joints[j];
    match jt.ty {
        JointType::Linear => {
            let mut r = IRigid::id();
            r.p = [
                q * Interval::point(jt.axis[0]),
                q * Interval::point(jt.axis[1]),
                q * Interval::point(jt.axis[2]),
            ];
            r
        }
        JointType::Rotary => {
            let (s, c) = isin_cos_deg(q);
            let one = Interval::point(1.0);
            let t = one - c;
            let a = ipt(jt.axis);
            let [x, y, z] = a;
            let m: IM3 = [
                [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
                [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
                [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
            ];
            let pv = ipt(jt.pivot);
            let rp = imul(&m, &pv);
            IRigid {
                r: m,
                p: [pv[0] - rp[0], pv[1] - rp[1], pv[2] - rp[2]],
            }
        }
    }
}

/// Every link's placement for joint values in boxes q.
pub fn ipose(def: &MachineDef, q: &[Interval]) -> Vec<IRigid> {
    let n = def.joints.len();
    let mut out: Vec<Option<IRigid>> = vec![None; n];
    fn go(def: &MachineDef, q: &[Interval], j: usize, out: &mut Vec<Option<IRigid>>) -> IRigid {
        if let Some(r) = out[j] {
            return r;
        }
        let parent = match def.joint_index(&def.joints[j].parent) {
            Some(p) => go(def, q, p, out),
            None => IRigid::id(),
        };
        let r = parent.then(&ijoint_rigid(def, j, q[j]));
        out[j] = Some(r);
        r
    }
    for j in 0..n {
        go(def, q, j, &mut out);
    }
    out.into_iter().map(|r| r.unwrap()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::builtin;

    #[test]
    fn vmc_tip_follows_words() {
        let m = &builtin()[0];
        // X 10, Y 20, Z -100 with a 100 mm tool: tip at (10, 20, 500 - 100 - 100) in the part
        let q = [20.0, 10.0, -100.0];
        let (tip, ax) = tip_in_part(m, &q, "spindle", "table", 100.0).unwrap();
        assert_eq!(tip, [10.0, 20.0, 300.0]);
        assert_eq!(ax, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn trunnion_b90_turns_the_part() {
        let m = &builtin()[1];
        // B = 90: the part's +Z lies along base +x; tool straight down at x = 0
        let q = [0.0, 0.0, 0.0, 90.0, 0.0];
        let p = pose(m, &q);
        let f = part_frame(m, &p, "table").unwrap();
        assert_eq!(f.dir([0.0, 0.0, 1.0]), [1.0, 0.0, 0.0]);
        let (_, ax) = tip_in_part(m, &q, "spindle", "table", 100.0).unwrap();
        // the tool's axis (tip to gauge, base +z) in the part: −x
        assert_eq!(ax, [-1.0, 0.0, 0.0]);
    }

    #[test]
    fn interval_pose_encloses_points() {
        let m = &builtin()[1];
        let qs = [[10.0, -5.0, -30.0, 33.3, 71.0], [0.0, 0.0, 0.0, -119.0, 359.0]];
        for q in qs {
            let iq: Vec<Interval> = q.iter().map(|v| Interval::new(v - 0.5, v + 0.5)).collect();
            let ip = ipose(m, &iq);
            let p = pose(m, &q);
            for (a, b) in p.iter().zip(&ip) {
                for i in 0..3 {
                    assert!(b.p[i].contains(a.p[i]));
                    for j in 0..3 {
                        assert!(b.r[i][j].contains(a.r[i][j]));
                    }
                }
            }
        }
    }
}
