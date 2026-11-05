//! Inventor's drag in an assembly: a press and drag on a component moves it within what its joints
//! leave free. The drag is a transient pose: its joint values (params) and moved poses are built
//! in the file's place, the file kept, till Keep Position writes them (one undo step) or Return to
//! Design (Esc) drops them. The Drive dialog's values go through the same pose.
//!
//! What a drag moves: the grabbed occurrence's top-level instance and everything held rigidly to
//! it (fastened, or by joints whose every value is set), as one body; the joint that ties that
//! body towards ground is the one dragged. Its freedoms driven by a param of their own move by
//! that param; freedoms nothing drives move by the body's written pose; a freedom set by anything
//! else stays. A body with no way to ground (unjointed) moves freely; a grounded one stays.

use super::fg::{FeatRow, From, ParamRow, short};
use super::fgasm::AsmPanel;
use forge_lang::asm::LinkFamily;
use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::values::Dim;
use forge_math::{Frame, Point3, Vec3};

/// How one of a joint's freedoms answers a drag.
#[derive(Clone, Debug, PartialEq)]
pub enum Hold {
    /// Its own param drives it: the drag sets the param.
    Param(ParamKey),
    /// Nothing drives it: the drag moves the body's written pose.
    Free,
    /// Set by what a drag must not change; why.
    Held(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dof {
    /// The joint's word for it: turn, along, x, y.
    pub word: &'static str,
    pub angle: bool,
    /// Solved, when the drag began (radians, mm).
    pub value: f64,
    pub hold: Hold,
    pub limits: Option<(f64, f64)>,
}

impl Dof {
    fn moves(&self) -> bool {
        !matches!(self.hold, Hold::Held(_))
    }
}

/// The joint a drag turns or slides.
#[derive(Clone, Debug, PartialEq)]
pub struct JointDrag {
    pub joint: String,
    pub kind: String,
    /// The grabbed body is the joint's A end (the moving one); else its B.
    pub child_a: bool,
    /// B's frame in the world as the drag began: values are measured in it.
    pub b: Frame,
    pub dofs: Vec<Dof>,
    /// The body's top-level instances and their solved poses as the drag began.
    pub body: Vec<(String, Frame)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Plan {
    Joint(JointDrag),
    /// Nothing ties it to ground: the body follows the pointer.
    Free {
        body: Vec<(String, Frame)>,
    },
    /// It doesn't move, and what holds it.
    Locked(String),
}

/// A drag under way: what it moves, where it was taken, the values it has set.
#[derive(Clone, Debug, PartialEq)]
pub struct Grab {
    pub occ: String,
    pub plan: Plan,
    /// The world point pressed on.
    pub at: Point3,
    /// The joint values now (the plan's dofs' order), or the free body's motion.
    pub values: Vec<f64>,
    pub motion: Frame,
    /// A value stopped at its limit.
    pub at_limit: bool,
}

/// A joint value set by a drag: the joint's id, its freedom's word, the value (radians, mm).
pub type Drive = (String, String, f64);

/// The transient pose: joint values and instance poses built in the file's place.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Posing {
    pub params: Vec<(ParamKey, Literal)>,
    pub poses: Vec<(String, Frame)>,
    /// The file's text this is a pose of: when the file changes the pose is gone.
    pub over: Option<String>,
    pub grab: Option<Grab>,
    /// The grab's plan is made (from a build that's up to date).
    pub planned: bool,
    /// Drives for the host to set straight in the kept solve (the Drive dialog's slider, held).
    pub live: Vec<Drive>,
    /// Instances moved in the view only, till the pose's build comes.
    pub settling: Vec<String>,
    /// The status bar says the pose once its build is in (the build's own line would hide it).
    pub say: bool,
}

impl Posing {
    pub fn is_empty(&self) -> bool {
        self.params.is_empty() && self.poses.is_empty()
    }

    pub fn set_param(&mut self, key: ParamKey, value: Literal) {
        match self.params.iter_mut().find(|p| p.0 == key) {
            Some(p) => p.1 = value,
            None => self.params.push((key, value)),
        }
    }

    pub fn set_pose(&mut self, inst: &str, pose: Frame) {
        match self.poses.iter_mut().find(|p| p.0 == inst) {
            Some(p) => p.1 = pose,
            None => self.poses.push((inst.to_string(), pose)),
        }
    }

    /// What builds it: a SetParam each value, a SetArg `at` each pose.
    pub fn ops(&self) -> Vec<Op> {
        let mut ops: Vec<Op> = self
            .params
            .iter()
            .map(|(k, v)| Op::SetParam {
                param: k.clone(),
                value: *v,
            })
            .collect();
        ops.extend(self.poses.iter().map(|(id, f)| Op::SetArg {
            id: id.clone(),
            arg: "at".into(),
            text: super::asmtools::pose_text(f),
        }));
        ops
    }

    /// The status bar's words for it: `f_door_f.turn = 63°, f_lid moved`.
    pub fn words(&self) -> String {
        let mut w: Vec<String> = self
            .params
            .iter()
            .map(|(k, v)| format!("{k} = {}", literal_text(v)))
            .collect();
        w.extend(self.poses.iter().map(|p| format!("{} moved", p.0)));
        w.join(", ")
    }

    pub fn clear(&mut self) {
        *self = Posing::default();
    }
}

/// A value as a person reads it: degrees, mm.
pub fn literal_text(v: &Literal) -> String {
    let r = (v.value * 100.0).round() / 100.0;
    match v.dim {
        Dim::Angle => format!("{}°", short(r)),
        Dim::Length => format!("{} mm", short(r)),
        _ => short(r),
    }
}

/// An occurrence's top-level instance.
pub fn top(path: &str) -> &str {
    path.split('.').next().unwrap_or(path)
}

/// A joint's freedoms and how each answers a drag (the agent's `drive_joint` asks this too).
pub fn joint_dofs(a: &AsmPanel, feats: &[FeatRow], params: &[ParamRow], id: &str) -> Vec<Dof> {
    dofs(a, feats, params, id)
}

fn dofs(a: &AsmPanel, feats: &[FeatRow], params: &[ParamRow], id: &str) -> Vec<Dof> {
    let Some(j) = a.joints.iter().find(|j| j.id == id) else {
        return vec![];
    };
    let row = feats.iter().find(|f| f.id == id);
    j.values
        .iter()
        .map(|&(word, value, angle)| {
            let dim = row.and_then(|f| f.dims.iter().find(|d| d.what == word));
            let hold = match dim.map(|d| &d.from) {
                None => Hold::Free,
                Some(From::Const) => Hold::Held(format!("{id}.{word} is written in its statement")),
                Some(From::Driven(ks)) => Hold::Held(format!(
                    "{id}.{word} is set by {}",
                    ks.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(", ")
                )),
                Some(From::Driving(k)) => {
                    let p = params.iter().find(|p| p.key == *k);
                    // its own (`param joint.turn`), or a shared one nothing else reads
                    let own = k.owner.len() == 1 && k.owner[0].name == id;
                    let alone = p.is_some_and(|p| p.used_by.iter().all(|u| u == id) && !p.used_by.is_empty());
                    match p {
                        Some(p) if !p.driving() => Hold::Held(format!("{k} = {}", p.text.trim())),
                        Some(p) if p.structural => Hold::Held(format!("{k} changes what the assembly holds")),
                        Some(_) if own || alone => Hold::Param(k.clone()),
                        _ => Hold::Held(format!("{k} is shared with other relationships")),
                    }
                }
            };
            Dof {
                word,
                angle,
                value,
                hold,
                limits: j.limits.iter().find(|l| l.0 == word).map(|l| (l.1, l.2)),
            }
        })
        .collect()
}

struct Sets(Vec<usize>);

impl Sets {
    fn find(&mut self, k: usize) -> usize {
        let mut r = k;
        while self.0[r] != r {
            r = self.0[r];
        }
        let mut k = k;
        while self.0[k] != r {
            let n = self.0[k];
            self.0[k] = r;
            k = n;
        }
        r
    }
    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0[a.max(b)] = a.min(b);
        }
    }
}

/// What a press on occurrence `occ` would drag.
pub fn plan(a: &AsmPanel, feats: &[FeatRow], params: &[ParamRow], occ: &str) -> Plan {
    let ids: Vec<&str> = a
        .insts
        .iter()
        .filter(|i| !i.path.contains('.') && !i.suppressed)
        .map(|i| i.path.as_str())
        .collect();
    let at = |p: &str| ids.iter().position(|i| *i == top(p));
    let Some(me) = at(occ) else {
        return Plan::Locked(format!("{occ} is not an instance of this assembly"));
    };
    let mut sets = Sets((0..ids.len()).collect());
    // the relationships that leave freedom: (link, its two ends' instances, its freedoms)
    let mut loose: Vec<(usize, usize, usize, Vec<Dof>)> = vec![];
    for (k, l) in a.links.iter().enumerate() {
        if !l.owner.is_empty() || l.suppressed {
            continue;
        }
        let (Some(x), Some(y)) = (at(&l.a.0), at(&l.b.0)) else {
            continue;
        };
        if x == y {
            continue;
        }
        match l.family {
            LinkFamily::Joint => {
                let d = dofs(a, feats, params, &l.id);
                if d.iter().any(Dof::moves) {
                    loose.push((k, x, y, d));
                } else {
                    sets.join(x, y);
                }
            }
            LinkFamily::Constraint => loose.push((k, x, y, vec![])),
            _ => {}
        }
    }
    let n = ids.len();
    let body_of = |sets: &mut Sets, c: usize| -> Vec<(String, Frame)> {
        (0..n)
            .filter(|&i| sets.find(i) == c)
            .filter_map(|i| {
                let pose = a.poses.iter().find(|p| p.0 == ids[i])?.1;
                Some((ids[i].to_string(), pose))
            })
            .collect()
    };
    let mine = sets.find(me);
    // grounded: what holds it
    let grounded: Vec<usize> = ids
        .iter()
        .enumerate()
        .filter(|(_, i)| a.grounded.iter().any(|g| g == *i))
        .map(|(k, _)| k)
        .collect();
    if let Some(&g) = grounded.iter().find(|&&g| sets.find(g) == mine) {
        let name = top(occ);
        return Plan::Locked(if ids[g] == name {
            format!("{name} is grounded")
        } else {
            let set: Vec<&str> = a
                .links
                .iter()
                .filter(|l| l.owner.is_empty() && !l.suppressed && l.family == LinkFamily::Joint)
                .filter(|l| top(&l.a.0) == name || top(&l.b.0) == name)
                .map(|l| l.id.as_str())
                .take(3)
                .collect();
            format!(
                "{name} is held to {}, which is grounded, by {}",
                ids[g],
                set.join(", ")
            )
        });
    }
    // steps from ground over what's loose, by body
    let mut depth = vec![usize::MAX; n];
    let mut front: Vec<usize> = grounded.iter().map(|&g| sets.find(g)).collect();
    for &c in &front {
        depth[c] = 0;
    }
    while !front.is_empty() {
        let mut next = vec![];
        for &c in &front {
            for (_, x, y, _) in &loose {
                let (x, y) = (sets.find(*x), sets.find(*y));
                for (from, to) in [(x, y), (y, x)] {
                    if from == c && depth[to] == usize::MAX {
                        depth[to] = depth[c] + 1;
                        next.push(to);
                    }
                }
            }
        }
        front = next;
    }
    let body = body_of(&mut sets, mine);
    if depth[mine] == usize::MAX {
        return Plan::Free { body };
    }
    // the relationship towards ground: a joint with freedom first
    let towards: Vec<&(usize, usize, usize, Vec<Dof>)> = loose
        .iter()
        .filter(|(_, x, y, _)| {
            let (x, y) = (sets.find(*x), sets.find(*y));
            (x == mine && depth[y] + 1 == depth[mine]) || (y == mine && depth[x] + 1 == depth[mine])
        })
        .collect();
    let joint = towards.iter().find(|t| !t.3.is_empty());
    let Some((k, x, _, d)) = joint.copied() else {
        // a constraint holds it: its written pose moves, the solve keeps what the constraint holds
        return Plan::Free { body };
    };
    let l = &a.links[*k];
    let Some(b) = a.link_at.get(*k).and_then(|e| e[1]) else {
        return Plan::Locked(format!("{} has no solved frame to drag in", l.id));
    };
    Plan::Joint(JointDrag {
        joint: l.id.clone(),
        kind: a
            .joints
            .iter()
            .find(|j| j.id == l.id)
            .map_or(l.kind.clone(), |j| j.kind.to_string()),
        child_a: sets.find(*x) == mine,
        b,
        dofs: d.clone(),
        body,
    })
}

/// A joint kind's motion `B⁻¹ ∘ A` for its values by word (forge-asm's joint.rs).
fn motion(kind: &str, value: &dyn Fn(&str) -> f64) -> Frame {
    let rz = |t: f64| Frame::WORLD.rotated_about_z(t);
    let at = |f: Frame, x: f64, y: f64, z: f64| f.with_origin(Point3::new(x, y, z));
    match kind {
        "revolute" => rz(value("turn")),
        "slider" => at(Frame::WORLD, 0.0, 0.0, value("along")),
        "cylindrical" => at(rz(value("turn")), 0.0, 0.0, value("along")),
        "pin_slot" => at(rz(value("turn")), value("along"), 0.0, 0.0),
        "planar" => at(rz(value("turn")), value("x"), value("y"), 0.0),
        _ => Frame::WORLD,
    }
}

impl JointDrag {
    /// The body's motion in the world when the dofs have these values.
    pub fn world_motion(&self, values: &[f64]) -> Frame {
        let of = |vals: &dyn Fn(usize) -> f64| {
            motion(&self.kind, &|w: &str| {
                self.dofs.iter().position(|d| d.word == w).map_or(0.0, vals)
            })
        };
        let m0 = of(&|i| self.dofs[i].value);
        let m = of(&|i| values[i]);
        let rel = if self.child_a {
            m.compose(&m0.inverse())
        } else {
            m0.compose(&m.inverse())
        };
        self.b.compose(&rel).compose(&self.b.inverse())
    }

    /// The dofs a drag moves: a planar joint's turn only with the modifier, else its x and y.
    pub fn moving(&self, rotate: bool) -> Vec<usize> {
        let all: Vec<usize> = (0..self.dofs.len()).filter(|&i| self.dofs[i].moves()).collect();
        if self.kind == "planar" {
            let (turn, slide): (Vec<usize>, Vec<usize>) =
                all.iter().partition(|&&i| self.dofs[i].word == "turn");
            return if rotate || slide.is_empty() { turn } else { slide };
        }
        all
    }

    /// Why the dofs that don't move don't.
    pub fn held(&self) -> Vec<String> {
        self.dofs
            .iter()
            .filter_map(|d| match &d.hold {
                Hold::Held(w) => Some(w.clone()),
                _ => None,
            })
            .collect()
    }
}

/// The joint values that bring the pressed point nearest the pointer on the screen, from `from`:
/// damped Gauss-Newton over the moving dofs, each held within its limits. True: one is at a limit.
pub fn follow(
    j: &JointDrag,
    moving: &[usize],
    at: Point3,
    from: &[f64],
    pointer: (f64, f64),
    pixel: &dyn Fn(Point3) -> Option<(f64, f64)>,
    scale: f64,
) -> (Vec<f64>, bool) {
    let mut v = from.to_vec();
    let clamp = |v: &mut Vec<f64>| {
        let mut hit = false;
        for &i in moving {
            if let Some((lo, hi)) = j.dofs[i].limits {
                if v[i] < lo || v[i] > hi {
                    hit = true;
                }
                v[i] = v[i].clamp(lo, hi);
            }
        }
        hit
    };
    let resid = |v: &[f64]| -> Option<(f64, f64)> {
        let p = j.world_motion(v).point_to_world(at);
        let q = pixel(p)?;
        Some((q.0 - pointer.0, q.1 - pointer.1))
    };
    let mut hit = false;
    let Some(mut r) = resid(&v) else {
        return (v, false);
    };
    let norm = |r: (f64, f64)| r.0 * r.0 + r.1 * r.1;
    let mut damp = 1e-3;
    for _ in 0..30 {
        // the derivative of the point's pixel by each moving value
        let cols: Vec<(f64, f64)> = moving
            .iter()
            .map(|&i| {
                // pixels are f32: a step that moves the point a good part of one
                let h = if j.dofs[i].angle {
                    2e-3
                } else {
                    2e-3 * scale.max(1.0)
                };
                let (mut lo, mut hi) = (v.clone(), v.clone());
                lo[i] -= h;
                hi[i] += h;
                match (resid(&lo), resid(&hi)) {
                    (Some(a), Some(b)) => ((b.0 - a.0) / (2.0 * h), (b.1 - a.1) / (2.0 * h)),
                    _ => (0.0, 0.0),
                }
            })
            .collect();
        let m = cols.len();
        let mut jtj = vec![vec![0.0; m]; m];
        let mut jtr = vec![0.0; m];
        for p in 0..m {
            for q in 0..m {
                jtj[p][q] = cols[p].0 * cols[q].0 + cols[p].1 * cols[q].1;
            }
            jtr[p] = cols[p].0 * r.0 + cols[p].1 * r.1;
        }
        let trace: f64 = (0..m).map(|p| jtj[p][p]).sum::<f64>().max(1e-12);
        // Levenberg-Marquardt: a step that makes it worse is tried again, shorter
        let mut took = false;
        for _ in 0..10 {
            let mut a = jtj.clone();
            for (p, row) in a.iter_mut().enumerate() {
                row[p] += damp * trace / m as f64;
            }
            let Some(step) = solve_small(a, jtr.iter().map(|x| -x).collect()) else {
                break;
            };
            let mut w = v.clone();
            for (s, &i) in step.iter().zip(moving) {
                // a turn moves at most half a radian a step: it stays on the side it's on
                w[i] += if j.dofs[i].angle { s.clamp(-0.5, 0.5) } else { *s };
            }
            let hit_w = clamp(&mut w);
            match resid(&w) {
                Some(rw) if norm(rw) < norm(r) => {
                    let moved = w.iter().zip(&v).any(|(a, b)| (a - b).abs() > 1e-9);
                    (v, r, hit) = (w, rw, hit_w);
                    damp = (damp / 3.0).max(1e-6);
                    took = moved;
                    break;
                }
                _ => damp *= 10.0,
            }
        }
        if !took || norm(r) < 1e-4 {
            break;
        }
    }
    // pressed against a limit: the clamp held it there
    let mut w = v.clone();
    for &i in moving {
        if let Some((lo, hi)) = j.dofs[i].limits
            && ((v[i] - lo).abs() < 1e-9 || (v[i] - hi).abs() < 1e-9)
        {
            w[i] = v[i];
            hit = true;
        }
    }
    v = w;
    (v, hit)
}

/// `A x = b` for a few unknowns, by elimination with pivoting.
fn solve_small(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| a[x][c].abs().total_cmp(&a[y][c].abs()))?;
        if a[p][c].abs() < 1e-18 {
            return None;
        }
        a.swap(c, p);
        b.swap(c, p);
        let (top, rest) = a.split_at_mut(c + 1);
        let pivot = &top[c];
        for (k, row) in rest.iter_mut().enumerate() {
            let f = row[c] / pivot[c];
            for (x, y) in row[c..].iter_mut().zip(&pivot[c..]) {
                *x -= f * y;
            }
            b[c + 1 + k] -= f * b[c];
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let s: f64 = (r + 1..n).map(|k| a[r][k] * x[k]).sum();
        x[r] = (b[r] - s) / a[r][r];
    }
    Some(x)
}

/// A dof's value as the param writes it: degrees for a turn.
pub fn literal(d: &Dof, v: f64) -> Literal {
    if d.angle {
        let deg = (v.to_degrees() * 1e4).round() / 1e4;
        Literal {
            value: deg,
            dim: Dim::Angle,
        }
    } else {
        Literal {
            value: (v * 1e4).round() / 1e4,
            dim: Dim::Length,
        }
    }
}

impl Grab {
    /// The pose these values make: params set, a free dof's body moved.
    pub fn pose_into(&self, p: &mut Posing) {
        match &self.plan {
            Plan::Joint(j) => {
                let mut free = false;
                for (d, &v) in j.dofs.iter().zip(&self.values) {
                    match &d.hold {
                        Hold::Param(k) => p.set_param(k.clone(), literal(d, v)),
                        Hold::Free => free |= (v - d.value).abs() > 1e-12,
                        Hold::Held(_) => {}
                    }
                }
                if free {
                    let w = j.world_motion(&self.values);
                    for (id, pose) in &j.body {
                        p.set_pose(id, w.compose(pose));
                    }
                }
            }
            Plan::Free { body } => {
                for (id, pose) in body {
                    p.set_pose(id, self.motion.compose(pose));
                }
            }
            Plan::Locked(_) => {}
        }
    }

    /// The status bar while it's dragged.
    pub fn words(&self) -> String {
        match &self.plan {
            Plan::Joint(j) => {
                let vals: Vec<String> = j
                    .dofs
                    .iter()
                    .zip(&self.values)
                    .filter(|(d, _)| d.moves())
                    .map(|(d, &v)| {
                        let l = literal(d, v);
                        format!("{}.{} = {}", j.joint, d.word, literal_text(&l))
                    })
                    .collect();
                format!(
                    "Dragging: {}{}",
                    vals.join(", "),
                    if self.at_limit { " (at its limit)" } else { "" }
                )
            }
            Plan::Free { body } => format!(
                "Dragging: {} (free)",
                body.iter().map(|b| b.0.as_str()).collect::<Vec<_>>().join(", ")
            ),
            Plan::Locked(why) => format!("Locked: {why}"),
        }
    }
}

/// A free body's motion: along the screen at the pressed point's depth, or (rotating) turned about
/// it, up and across.
pub fn free_motion(
    at: Point3,
    from: (f64, f64),
    to: (f64, f64),
    rotate: bool,
    point_at: &dyn Fn((f64, f64)) -> Point3,
    up: Vec3,
    right: Vec3,
) -> Frame {
    let about = |axis: Vec3, angle: f64| {
        let Some(z) = axis.normalize() else {
            return Frame::WORLD;
        };
        let f = Frame::from_z(at, z);
        f.rotated_about_z(angle).compose(&f.inverse())
    };
    if rotate {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        about(up, dx * 0.01).compose(&about(right, dy * 0.01))
    } else {
        let d = point_at(to) - point_at(from);
        Frame::WORLD.with_origin(Point3::ORIGIN + d)
    }
}

/// Keep Position and Return to Design, from the ribbon or the view's menu: the status bar's words.
pub fn command(fg: &mut super::fg::FgPanel, keep: bool) -> String {
    let Some(a) = &mut fg.asm else {
        return String::new();
    };
    let p = &mut a.posing;
    if p.is_empty() {
        p.clear();
        return "Nothing dragged: the assembly is as designed".into();
    }
    let words = p.words();
    let ops = p.ops();
    for t in std::mem::take(&mut p.settling) {
        a.moved.remove(&t);
    }
    let p = &mut a.posing;
    p.clear();
    if keep {
        fg.out.push(super::fg::FgAct::AsmOps(ops));
        format!("Kept: {words}")
    } else {
        fg.out.push(super::fg::FgAct::AsmPreview(None));
        "Returned to the design".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn door(child_a: bool) -> JointDrag {
        JointDrag {
            joint: "hinge".into(),
            kind: "revolute".into(),
            child_a,
            b: Frame::WORLD.with_origin(Point3::new(10.0, 0.0, 0.0)),
            dofs: vec![Dof {
                word: "turn",
                angle: true,
                value: 0.0,
                hold: Hold::Free,
                limits: Some((-1.0, 1.0)),
            }],
            body: vec![],
        }
    }

    #[test]
    fn a_turn_follows_the_pointer_round_its_axis_and_stops_at_its_limit() {
        // looking down z: a pixel is the point's x, y
        let pixel = |p: Point3| Some((p.x, p.y));
        let at = Point3::new(20.0, 0.0, 0.0);
        for child_a in [true, false] {
            let j = door(child_a);
            let (v, hit) = follow(
                &j,
                &[0],
                at,
                &[0.0],
                (
                    10.0 + 10.0 * forge_math::dmath::cos(0.6),
                    10.0 * forge_math::dmath::sin(0.6),
                ),
                &pixel,
                10.0,
            );
            let p = j.world_motion(&v).point_to_world(at);
            assert!(
                (p.x - (10.0 + 10.0 * forge_math::dmath::cos(0.6))).abs() < 1e-2,
                "{child_a}: {v:?} {p:?}"
            );
            assert!(!hit);
            // past the limit: held at it
            let (v, hit) = follow(&j, &[0], at, &v, (10.0, 10.0), &pixel, 10.0);
            assert!(hit && (v[0].abs() - 1.0).abs() < 1e-9, "{child_a}: {v:?}");
        }
    }

    #[test]
    fn a_child_b_turns_the_other_way_for_the_same_value() {
        let at = Point3::new(20.0, 0.0, 0.0);
        let a = door(true).world_motion(&[0.5]).point_to_world(at);
        let b = door(false).world_motion(&[0.5]).point_to_world(at);
        assert!((a.y + b.y).abs() < 1e-9 && a.y > 0.0, "{a:?} {b:?}");
    }

    #[test]
    fn the_pose_is_ops_and_words() {
        let mut p = Posing::default();
        let k = ParamKey::parse("hinge.turn").unwrap();
        p.set_param(
            k.clone(),
            Literal {
                value: 30.0,
                dim: Dim::Angle,
            },
        );
        p.set_param(
            k,
            Literal {
                value: 63.0,
                dim: Dim::Angle,
            },
        );
        assert_eq!(p.words(), "hinge.turn = 63°");
        assert_eq!(p.ops().len(), 1);
    }
}
