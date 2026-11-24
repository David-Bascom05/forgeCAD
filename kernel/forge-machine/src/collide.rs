//! Collision of every body through the simulated motion.
//!
//! The bodies: the machine's (each on its link), the tool in the spindle (stacked cylinders on
//! the tool mount), and the workpiece's (stock and fixture, on the part mount). Two bodies on one
//! link never move relative to each other and are not checked; nor are pairs a body `ignore`s.
//! The tool's cutting cylinder is not checked against the stock on feed moves (that is cutting).
//!
//! For a span of a segment's parameter s ∈ [s0, s1] every link's placement is enclosed with
//! interval arithmetic (kin::ipose over the joint values' enclosure on the span: a line's hull,
//! an arc's circle over the angle interval). A body's primitives are carried by those enclosures,
//! and for a direction n the gap between two primitives is bounded below by the separation of
//! their projections on n: min over B of y·n minus max over A of x·n, each from the primitive's
//! support function evaluated in intervals (a box: c·n + Σ hᵢ|eᵢ·n|; a cylinder of unit axis a:
//! c·n + (h/2)|a·n| plus r√(|n|² − (a·n)²)). Any n gives a lower bound on the distance (divided by |n|), so
//! the directions (face normals, axes, their cross products, centre to centre) are chosen in
//! floating point at the span's middle without affecting soundness.
//!
//! Verdicts per pair and segment:
//! - **Clear**: every span's bound is ≥ the clearance: proven, given forge-math's interval
//!   contracts (outward rounding; dmath sin/cos within 1 ulp).
//! - **Hit**: at some s, a point of one body lies inside the other by more than 1e-6 mm, found in
//!   floating point (a witness, checked with a margin far above the rounding of mm-scale numbers;
//!   not an interval proof).
//! - **Near**: not proven clear down to the resolution, and no witness: the bodies come within the
//!   clearance (or overlap without a vertex inside). Treated as a failure by the caller.
use crate::def::{MachineDef, MountKind, Prim};
use crate::kin::{self, IRigid, Rigid, V3};
use crate::sim::{Path, SegKind, Segment, Setup, Timeline};
use forge_math::interval::Interval;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Machine,
    /// The tool: `cutting` for its first cylinder (the flutes or insert).
    Tool {
        cutting: bool,
    },
    Stock,
    Fixture,
}

/// A body ready for the check: primitives in the base frame at home, carried by `link`.
#[derive(Clone, Debug, PartialEq)]
pub struct CBody {
    pub name: String,
    /// def.joints index, or None for the base.
    pub link: Option<usize>,
    pub prims: Vec<Prim>,
    pub ignore: Vec<String>,
    pub role: Role,
    /// Its own least gap (a machine body's `clearance`), else the machine's.
    pub clearance: Option<f64>,
    /// The stock runs through it (a guide bushing): no contact with Role::Stock.
    pub passes_stock: bool,
}

/// The least gap a pair keeps: the smaller of their own, else the machine's.
fn pair_clearance(def: &MachineDef, a: &CBody, b: &CBody) -> f64 {
    match (a.clearance, b.clearance) {
        (Some(x), Some(y)) => x.min(y),
        (Some(x), None) | (None, Some(x)) => x,
        (None, None) => def.clearance,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Clear,
    Hit { s: f64, witness: V3 },
    Near { s0: f64, s1: f64, gap_lo: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Contact {
    pub line: usize,
    pub segment: usize,
    pub a: String,
    pub b: String,
    pub verdict: Verdict,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Report {
    pub pairs_checked: usize,
    /// Spans evaluated (the work).
    pub spans: usize,
    pub contacts: Vec<Contact>,
}

/// The machine's bodies (a body with no envelope, cited unknown, is left out: sim::unchecked
/// reports it).
pub fn machine_bodies(def: &MachineDef) -> Vec<CBody> {
    def.bodies
        .iter()
        .filter(|b| !b.prims.is_empty())
        .map(|b| CBody {
            name: b.name.clone(),
            link: def.joint_index(&b.link),
            prims: b.prims.clone(),
            ignore: b.ignore.clone(),
            role: Role::Machine,
            clearance: b.clearance,
            passes_stock: b.passes_stock,
        })
        .collect()
}

/// A tool's bodies on the setup's tool mount: one cylinder per stacked piece.
pub fn tool_bodies(s: &Setup, t: u32) -> Vec<CBody> {
    let def = s.def;
    let Some(tool) = s.tool(t) else { return Vec::new() };
    let Some(m) = s.mount_for(t) else {
        return Vec::new();
    };
    let tip = kin::add(m.at, tool.tip);
    let u = kin::scale(m.dir, -1.0);
    tool.body
        .iter()
        .enumerate()
        .map(|(i, (r, a, b))| CBody {
            name: if i == 0 {
                format!("T{t} cutter")
            } else {
                format!("T{t} body {i}")
            },
            link: def.joint_index(&m.link),
            prims: vec![Prim::Cyl {
                base: kin::add(tip, kin::scale(u, *a)),
                axis: u,
                r: *r,
                h: b - a,
            }],
            ignore: Vec::new(),
            role: Role::Tool { cutting: i == 0 },
            clearance: None,
            passes_stock: false,
        })
        .collect()
}

/// A workpiece body given in the part's coordinates, carried by the setup's part mount.
pub fn part_body(s: &Setup, name: &str, prims_in_part: &[Prim], role: Role) -> CBody {
    let def = s.def;
    let m = def
        .mounts
        .iter()
        .find(|m| m.name == s.part_mount && m.kind == MountKind::Part)
        .unwrap();
    let pose0 = kin::pose(def, &vec![0.0; def.joints.len()]);
    // the part frame at the home pose, relative to its link (the link is the identity at home)
    let f = kin::part_frame(def, &pose0, &m.name).unwrap();
    let prims = prims_in_part
        .iter()
        .flat_map(|p| match p {
            Prim::Box { min, max } => {
                // a box in the part frame is an oriented box in the base frame: a box again when
                // the frame is axis-aligned, else an oriented box
                let a = f.apply(*min);
                let b = f.apply(*max);
                let axis_aligned =
                    f.r.iter()
                        .all(|row| row.iter().filter(|x| **x != 0.0).count() == 1);
                if axis_aligned {
                    vec![Prim::Box {
                        min: [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])],
                        max: [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])],
                    }]
                } else {
                    vec![Prim::OBox {
                        o: a,
                        axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|d| f.dir(d)),
                        size: [max[0] - min[0], max[1] - min[1], max[2] - min[2]],
                    }]
                }
            }
            Prim::Cyl { base, axis, r, h } => vec![Prim::Cyl {
                base: f.apply(*base),
                axis: f.dir(*axis),
                r: *r,
                h: *h,
            }],
            Prim::OBox { o, axes, size } => vec![Prim::OBox {
                o: f.apply(*o),
                axes: axes.map(|a| f.dir(a)),
                size: *size,
            }],
        })
        .collect();
    CBody {
        name: name.into(),
        link: def.joint_index(&m.link),
        prims,
        ignore: Vec::new(),
        role,
        clearance: None,
        passes_stock: false,
    }
}

// ------------------------------------------------------------------------------------------

/// A primitive placed by an interval placement.
#[derive(Clone, Copy, Debug)]
enum IPrim {
    /// centre, the three (unit) edge directions, half-lengths
    Box {
        c: [Interval; 3],
        e: [[Interval; 3]; 3],
        h: [f64; 3],
    },
    /// centre of the axis segment, unit axis, radius, half height
    Cyl {
        c: [Interval; 3],
        a: [Interval; 3],
        r: f64,
        hh: f64,
    },
}

fn ip(v: V3) -> [Interval; 3] {
    [
        Interval::point(v[0]),
        Interval::point(v[1]),
        Interval::point(v[2]),
    ]
}

fn place(p: &Prim, t: &IRigid) -> IPrim {
    match p {
        Prim::Box { min, max } => {
            let c = [
                (min[0] + max[0]) * 0.5,
                (min[1] + max[1]) * 0.5,
                (min[2] + max[2]) * 0.5,
            ];
            let h = [
                (max[0] - min[0]) * 0.5,
                (max[1] - min[1]) * 0.5,
                (max[2] - min[2]) * 0.5,
            ];
            // the halves and centre of a box with float corners: exact (a halving and a sum of two
            // floats of like size can round; widen by an ulp each side to stay sound)
            let cc = ip(c).map(|x| Interval::new(x.lo.next_down(), x.hi.next_up()));
            let h = h.map(|x| x.next_up());
            let e = [0, 1, 2].map(|i| [t.r[0][i], t.r[1][i], t.r[2][i]]);
            IPrim::Box {
                c: t.apply(&cc),
                e,
                h,
            }
        }
        Prim::OBox { o, axes, size } => {
            // the centre in interval arithmetic (sound for float axes); the axes as given, so the
            // support function Σ h_i |e_i · n| is exact for the parallelepiped they span
            let mut c = ip(*o);
            for k in 0..3 {
                let half = Interval::point(size[k]) * Interval::point(0.5);
                for i in 0..3 {
                    c[i] = c[i] + half * Interval::point(axes[k][i]);
                }
            }
            let e = axes.map(|a| {
                let ia = ip(a);
                [0, 1, 2].map(|i| t.r[i][0] * ia[0] + t.r[i][1] * ia[1] + t.r[i][2] * ia[2])
            });
            IPrim::Box {
                c: t.apply(&c),
                e,
                h: size.map(|x| (x * 0.5).next_up()),
            }
        }
        Prim::Cyl { base, axis, r, h } => {
            let mid = kin::add(*base, kin::scale(*axis, h * 0.5));
            let mc = ip(mid).map(|x| Interval::new(x.lo.next_down().next_down(), x.hi.next_up().next_up()));
            let a = ip(*axis);
            let ra = [0, 1, 2].map(|i| t.r[i][0] * a[0] + t.r[i][1] * a[1] + t.r[i][2] * a[2]);
            IPrim::Cyl {
                c: t.apply(&mc),
                a: ra,
                r: *r,
                hh: (h * 0.5).next_up(),
            }
        }
    }
}

fn idot(a: &[Interval; 3], n: &[Interval; 3]) -> Interval {
    a[0] * n[0] + a[1] * n[1] + a[2] * n[2]
}

fn iabs_hi(x: Interval) -> f64 {
    x.lo.abs().max(x.hi.abs())
}

/// Upper bound of max over the primitive of x·n.
fn support_hi(p: &IPrim, n: &[Interval; 3], n2: Interval) -> f64 {
    match p {
        IPrim::Box { c, e, h } => {
            let mut s = idot(c, n).hi;
            for i in 0..3 {
                s = (s + (h[i] * iabs_hi(idot(&e[i], n))).next_up()).next_up();
            }
            s
        }
        IPrim::Cyl { c, a, r, hh } => {
            let an = idot(a, n);
            // |n|² − (a·n)² over the enclosures; the true value is ≥ 0 for the true unit a
            let rad = (n2 - an.sqr()).sqrt();
            let s = idot(c, n).hi;
            let s = (s + (hh * iabs_hi(an)).next_up()).next_up();
            (s + (r * rad.hi).next_up()).next_up()
        }
    }
}

/// Lower bound of min over the primitive of x·n.
fn support_lo(p: &IPrim, n: &[Interval; 3], n2: Interval) -> f64 {
    let m = [-n[0], -n[1], -n[2]];
    -support_hi(p, &m, n2)
}

/// A lower bound on the distance between two placed primitives along direction n (f64, nonzero).
fn sep(a: &IPrim, b: &IPrim, n: V3) -> f64 {
    let ni = ip(n);
    let n2 = idot(&ni, &ni);
    let nn = n2.sqrt();
    let g1 = (support_lo(b, &ni, n2) - support_hi(a, &ni, n2)).next_down();
    let g2 = (support_lo(a, &ni, n2) - support_hi(b, &ni, n2)).next_down();
    let g = g1.max(g2);
    // divide by |n|: by its upper bound when g > 0, by its lower when g < 0 (a negative bound
    // only says "no separation shown" and is never used to clear)
    if g > 0.0 {
        (g / nn.hi).next_down()
    } else {
        g / nn.lo.max(f64::MIN_POSITIVE)
    }
}

fn mid3(v: &[Interval; 3]) -> V3 {
    [v[0].mid(), v[1].mid(), v[2].mid()]
}

/// The candidate directions for a pair, from the enclosures' midpoints.
fn axes(a: &IPrim, b: &IPrim) -> Vec<V3> {
    let dirs = |p: &IPrim| -> (V3, Vec<V3>) {
        match p {
            IPrim::Box { c, e, .. } => (mid3(c), e.iter().map(mid3).collect()),
            IPrim::Cyl { c, a, .. } => (mid3(c), vec![mid3(a)]),
        }
    };
    let (ca, da) = dirs(a);
    let (cb, db) = dirs(b);
    let mut out: Vec<V3> = Vec::new();
    out.extend(&da);
    out.extend(&db);
    for x in &da {
        for y in &db {
            let c = kin::cross(*x, *y);
            if kin::norm(c) > 1e-9 {
                out.push(c);
            }
        }
    }
    let d = kin::sub(cb, ca);
    if kin::norm(d) > 0.0 {
        out.push(d);
        // a cylinder's direction from its axis toward the other centre, square to the axis
        for (p, u, w) in [(a, da[0], d), (b, db[0], kin::scale(d, -1.0))] {
            if let IPrim::Cyl { .. } = p {
                let perp = kin::sub(w, kin::scale(u, kin::dot(w, u)));
                if kin::norm(perp) > 1e-9 {
                    out.push(perp);
                }
            }
        }
    }
    out
}

fn gap_lo(a: &IPrim, b: &IPrim) -> f64 {
    axes(a, b)
        .into_iter()
        .map(|n| sep(a, b, n))
        .fold(f64::NEG_INFINITY, f64::max)
}

/// Joint values over s ∈ [s0, s1] of a segment, enclosed.
fn q_span(seg: &Segment, s0: f64, s1: f64) -> Vec<Interval> {
    let n = seg.q0.len();
    let mut q: Vec<Interval> = (0..n)
        .map(|j| {
            let (a, b) = (seg.q0[j], seg.q1[j]);
            let x0 = a + s0 * (b - a);
            let x1 = a + s1 * (b - a);
            // the lerp's rounding: widen by a relative 4 ulps of the magnitude
            let w = 4.0 * f64::EPSILON * (a.abs().max(b.abs()));
            Interval::new(x0.min(x1) - w, x0.max(x1) + w)
        })
        .collect();
    if let Path::Arc {
        a,
        b,
        c,
        r,
        r1,
        a0,
        sweep,
    } = &seg.path
    {
        // Segment::at: angle a0 + s·sweep, radius r + s·(r1 − r); each computed in f64, so widen
        // by a few ulps of the magnitudes involved
        let t0 = a0 + s0 * sweep;
        let t1 = a0 + s1 * sweep;
        let w = 4.0 * f64::EPSILON * (a0.abs() + sweep.abs());
        let th = Interval::new(t0.min(t1) - w, t0.max(t1) + w);
        let (ra, rb) = (r + s0 * (r1 - r), r + s1 * (r1 - r));
        let wr = 4.0 * f64::EPSILON * r.abs().max(r1.abs());
        let ri = Interval::new(ra.min(rb) - wr, ra.max(rb) + wr);
        let wc = 4.0 * f64::EPSILON * (c[0].abs().max(c[1].abs()) + r.abs().max(r1.abs()));
        q[*a] = (Interval::point(c[0]) + ri * th.cos()).grow(wc);
        q[*b] = (Interval::point(c[1]) + ri * th.sin()).grow(wc);
    }
    q
}

fn ilink(pose: &[IRigid], link: Option<usize>) -> IRigid {
    match link {
        Some(i) => pose[i],
        None => IRigid::id(),
    }
}

/// Points of a primitive (f64, placed) that witness penetration if inside the other.
fn samples(p: &Prim, t: &Rigid) -> Vec<V3> {
    let mut out = Vec::new();
    match p {
        Prim::Box { min, max } => {
            for i in 0..27 {
                let pick = |k: usize, d: usize| match (i / [1, 3, 9][d]) % 3 {
                    0 => min[k],
                    1 => (min[k] + max[k]) * 0.5,
                    _ => max[k],
                };
                out.push(t.apply([pick(0, 0), pick(1, 1), pick(2, 2)]));
            }
        }
        Prim::OBox { o, axes, size } => {
            for i in 0..27 {
                let f = [0, 1, 2].map(|d| ((i / [1, 3, 9][d]) % 3) as f64 * 0.5);
                out.push(t.apply(Prim::obox_at(o, axes, size, f)));
            }
        }
        Prim::Cyl { base, axis, r, h } => {
            let u = if axis[0].abs() < 0.9 {
                kin::cross(*axis, [1.0, 0.0, 0.0])
            } else {
                kin::cross(*axis, [0.0, 1.0, 0.0])
            };
            let u = kin::scale(u, 1.0 / kin::norm(u));
            let v = kin::cross(*axis, u);
            for hk in [0.0, 0.5, 1.0] {
                let c = kin::add(*base, kin::scale(*axis, h * hk));
                out.push(t.apply(c));
                for k in 0..8 {
                    let (sn, cs) = kin::sin_cos_deg(45.0 * k as f64);
                    out.push(t.apply(kin::add(
                        c,
                        kin::add(kin::scale(u, r * cs), kin::scale(v, r * sn)),
                    )));
                }
            }
        }
    }
    out
}

/// Depth of x inside a placed primitive (positive inside).
fn depth(p: &Prim, t: &Rigid, x: V3) -> f64 {
    let l = t.inverse().apply(x);
    match p {
        Prim::Box { min, max } => (0..3)
            .map(|k| (l[k] - min[k]).min(max[k] - l[k]))
            .fold(f64::INFINITY, f64::min),
        Prim::OBox { o, axes, size } => {
            let w = kin::sub(l, *o);
            (0..3)
                .map(|k| {
                    let s = kin::dot(w, axes[k]);
                    s.min(size[k] - s)
                })
                .fold(f64::INFINITY, f64::min)
        }
        Prim::Cyl { base, axis, r, h } => {
            let w = kin::sub(l, *base);
            let z = kin::dot(w, *axis);
            let radial = kin::norm(kin::sub(w, kin::scale(*axis, z)));
            (r - radial).min(z).min(h - z)
        }
    }
}

fn witness(a: &CBody, b: &CBody, pose: &[Rigid]) -> Option<V3> {
    let tl = |l: Option<usize>| match l {
        Some(i) => pose[i],
        None => Rigid::ID,
    };
    let (ta, tb) = (tl(a.link), tl(b.link));
    for (x, tx, y, ty) in [(a, ta, b, tb), (b, tb, a, ta)] {
        for px in &x.prims {
            for pt in samples(px, &tx) {
                for py in &y.prims {
                    if depth(py, &ty, pt) > 1e-6 {
                        return Some(pt);
                    }
                }
            }
        }
    }
    None
}

/// Resolution: a span is not split once every enclosure is this narrow (mm).
pub const RES: f64 = 0.01;
const MAX_DEPTH: u32 = 30;

fn width(p: &IPrim) -> f64 {
    let wc = |c: &[Interval; 3]| c.iter().map(|x| x.width()).fold(0.0, f64::max);
    match p {
        IPrim::Box { c, e, h } => {
            let we = e
                .iter()
                .flat_map(|v| v.iter().map(|x| x.width()))
                .fold(0.0, f64::max);
            wc(c) + we * (h[0] + h[1] + h[2])
        }
        IPrim::Cyl { c, a, r, hh } => wc(c) + a.iter().map(|x| x.width()).fold(0.0, f64::max) * (r + hh),
    }
}

#[allow(clippy::too_many_arguments)]
fn check_span(
    def: &MachineDef,
    seg: &Segment,
    a: &CBody,
    b: &CBody,
    s0: f64,
    s1: f64,
    clearance: f64,
    depth_left: u32,
    spans: &mut usize,
) -> Verdict {
    *spans += 1;
    let q = q_span(seg, s0, s1);
    let pose = kin::ipose(def, &q);
    let (ta, tb) = (ilink(&pose, a.link), ilink(&pose, b.link));
    let pa: Vec<IPrim> = a.prims.iter().map(|p| place(p, &ta)).collect();
    let pb: Vec<IPrim> = b.prims.iter().map(|p| place(p, &tb)).collect();
    let mut worst = f64::INFINITY;
    let mut wide = 0.0f64;
    for x in &pa {
        for y in &pb {
            let g = gap_lo(x, y);
            worst = worst.min(g);
            if g < clearance {
                wide = wide.max(width(x) + width(y));
            }
        }
    }
    if worst >= clearance {
        return Verdict::Clear;
    }
    if wide <= RES || depth_left == 0 {
        let sm = 0.5 * (s0 + s1);
        let p = kin::pose(def, &seg.at(sm));
        if let Some(w) = witness(a, b, &p) {
            return Verdict::Hit { s: sm, witness: w };
        }
        return Verdict::Near {
            s0,
            s1,
            gap_lo: worst,
        };
    }
    let sm = 0.5 * (s0 + s1);
    // the worse of the two halves: a Hit anywhere beats a Near found earlier (a Near is often the
    // approach to a Hit)
    let first = check_span(def, seg, a, b, s0, sm, clearance, depth_left - 1, spans);
    if matches!(first, Verdict::Hit { .. }) {
        return first;
    }
    let second = check_span(def, seg, a, b, sm, s1, clearance, depth_left - 1, spans);
    match (&first, &second) {
        (_, Verdict::Hit { .. }) | (Verdict::Clear, _) => second,
        _ => first,
    }
}

fn may_pair(a: &CBody, b: &CBody, kind: SegKind) -> bool {
    if a.link == b.link || a.ignore.contains(&b.name) || b.ignore.contains(&a.name) {
        return false;
    }
    let cut = |x: &CBody, y: &CBody| {
        matches!(x.role, Role::Tool { cutting: true }) && y.role == Role::Stock && kind != SegKind::Rapid
    };
    let through = |x: &CBody, y: &CBody| x.passes_stock && y.role == Role::Stock;
    !(cut(a, b) || cut(b, a) || through(a, b) || through(b, a))
}

/// Checks every pair of bodies through every segment. `work` are the workpiece bodies (stock,
/// fixture); the tool's bodies follow the tool each segment carries.
pub fn check(s: &Setup, tl: &Timeline, work: &[CBody]) -> Report {
    let def = s.def;
    let machine = machine_bodies(def);
    let mut rep = Report::default();
    let mut tool_cache: Vec<(u32, Vec<CBody>)> = Vec::new();
    for (si, seg) in tl.segments.iter().enumerate() {
        let tools: Vec<CBody> = match seg.tool {
            Some(t) => {
                if !tool_cache.iter().any(|(n, _)| *n == t) {
                    tool_cache.push((t, tool_bodies(s, t)));
                }
                tool_cache.iter().find(|(n, _)| *n == t).unwrap().1.clone()
            }
            None => Vec::new(),
        };
        let all: Vec<&CBody> = machine.iter().chain(tools.iter()).chain(work.iter()).collect();
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                let (a, b) = (all[i], all[j]);
                if !may_pair(a, b, seg.kind) {
                    continue;
                }
                rep.pairs_checked += 1;
                let v = check_span(
                    def,
                    seg,
                    a,
                    b,
                    0.0,
                    1.0,
                    pair_clearance(def, a, b),
                    MAX_DEPTH,
                    &mut rep.spans,
                );
                if v != Verdict::Clear {
                    rep.contacts.push(Contact {
                        line: seg.line,
                        segment: si,
                        a: a.name.clone(),
                        b: b.name.clone(),
                        verdict: v,
                    });
                }
            }
        }
    }
    rep
}

/// The machine's bodies at one pose, checked pairwise (a static pose: home, a tool-change
/// position). Used to validate a definition: at home nothing may collide.
pub fn check_pose(def: &MachineDef, q: &[f64]) -> Vec<(String, String, Verdict)> {
    let seg = Segment {
        line: 0,
        kind: SegKind::Rapid,
        q0: q.to_vec(),
        q1: q.to_vec(),
        path: Path::Line,
        t0: 0.0,
        t_min: 0.0,
        t_stop: 0.0,
        tool: None,
        spindle: None,
    };
    let bodies = machine_bodies(def);
    let mut out = Vec::new();
    let mut spans = 0;
    for i in 0..bodies.len() {
        for j in i + 1..bodies.len() {
            let (a, b) = (&bodies[i], &bodies[j]);
            if !may_pair(a, b, SegKind::Rapid) {
                continue;
            }
            let v = check_span(
                def,
                &seg,
                a,
                b,
                0.0,
                1.0,
                pair_clearance(def, a, b),
                4,
                &mut spans,
            );
            if v != Verdict::Clear {
                out.push((a.name.clone(), b.name.clone(), v));
            }
        }
    }
    out
}

/// What `check_box` found for a pair over a box of joint values.
#[derive(Clone, Debug, PartialEq)]
pub enum BoxVerdict {
    /// The gap is at least the clearance for every joint value in the box: proven, as `Clear`
    /// is for a segment (interval kinematics and separating-axis lower bounds).
    Clear,
    /// At these joint values a point of one body lies inside the other (a witness, as `Hit`).
    Hit { q: Vec<f64>, witness: V3 },
    /// Not proven clear down to the resolution and no witness: within the clearance near here.
    Near { q: Vec<f64>, gap_lo: f64 },
    /// The budget of boxes ran out before every box was decided.
    Undecided { gap_lo: f64 },
    /// The pair is never checked (`pair_rule`: same link, `ignore`d, an exempt pass).
    NotChecked,
}

/// Bodies `a` and `b` checked for every joint value in the box `q` (one interval per def.joints;
/// a joint that does not move is a point interval), splitting the box in half across its
/// relatively widest joint until each piece is proven clear, a witness is found, or the
/// primitives' enclosures are narrower than `RES`. The clearance is the pair's own
/// (`pair_rule`, as a rapid); a pair it never checks is `NotChecked`. At most `budget` boxes;
/// returns the verdict and the boxes evaluated. A witness at any box's middle ends the search
/// with Hit; the first piece at the resolution that is neither proven clear nor witnessed ends it
/// with Near (a Hit elsewhere is then not looked for).
///
/// With `outside_zones`, the zones' open interiors are left out (the control forbids them) and
/// their faces kept (they are travel): a box wholly inside a zone is skipped, a box reaching into
/// one is split at the zone's face, and a piece within the zone's closed box keeps only its
/// faces. Clear then means "clear for all travel outside the zones' interiors".
pub fn check_box(
    def: &MachineDef,
    a: &CBody,
    b: &CBody,
    q: &[Interval],
    budget: usize,
    outside_zones: bool,
) -> (BoxVerdict, usize) {
    let Some(clearance) = pair_rule(def, a, b, SegKind::Rapid) else {
        return (BoxVerdict::NotChecked, 0);
    };
    // a zone's (joint index, min, max) per joint it names
    let zones: Vec<Vec<(usize, f64, f64)>> = if outside_zones {
        def.zones
            .iter()
            .map(|z| {
                z.joints
                    .iter()
                    .zip(z.min.iter().zip(&z.max))
                    .filter_map(|(j, (lo, hi))| def.joint_index(j).map(|i| (i, *lo, *hi)))
                    .collect::<Vec<_>>()
            })
            .filter(|z| !z.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    let w0: Vec<f64> = q.iter().map(|x| x.width()).collect();
    let mut stack: Vec<Vec<Interval>> = vec![q.to_vec()];
    let mut boxes = 0;
    // the least gap bound among boxes not proven clear
    let mut open = f64::INFINITY;
    while let Some(qb) = stack.pop() {
        if boxes == budget {
            return (BoxVerdict::Undecided { gap_lo: open }, boxes);
        }
        boxes += 1;
        // zones are open boxes whose faces are travel: a box inside one is skipped; a box that
        // reaches into one is split at the zone's face, so the pieces keep the faces and drop
        // only the open interior
        if let Some(z) = zones
            .iter()
            .find(|z| z.iter().all(|&(i, lo, hi)| qb[i].lo < hi && qb[i].hi > lo))
        {
            if z.iter().all(|&(i, lo, hi)| qb[i].lo > lo && qb[i].hi < hi) {
                continue;
            }
            let cut = z.iter().find_map(|&(i, lo, hi)| {
                [lo, hi]
                    .into_iter()
                    .find(|&f| qb[i].lo < f && f < qb[i].hi)
                    .map(|f| (i, f))
            });
            match cut {
                Some((i, f)) => {
                    let (mut x, mut y) = (qb.clone(), qb);
                    x[i] = Interval::new(x[i].lo, f);
                    y[i] = Interval::new(f, y[i].hi);
                    stack.push(y);
                    stack.push(x);
                }
                None => {
                    // within the zone's closed box, touching a face: only the faces are travel
                    for &(i, lo, hi) in z {
                        for f in [lo, hi] {
                            if qb[i].lo == f || qb[i].hi == f {
                                let mut x = qb.clone();
                                x[i] = Interval::new(f, f);
                                stack.push(x);
                            }
                        }
                    }
                }
            }
            continue;
        }
        let pose = kin::ipose(def, &qb);
        let (ta, tb) = (ilink(&pose, a.link), ilink(&pose, b.link));
        let pa: Vec<IPrim> = a.prims.iter().map(|p| place(p, &ta)).collect();
        let pb: Vec<IPrim> = b.prims.iter().map(|p| place(p, &tb)).collect();
        let mut worst = f64::INFINITY;
        let mut wide = 0.0f64;
        for x in &pa {
            for y in &pb {
                let g = gap_lo(x, y);
                worst = worst.min(g);
                if g < clearance {
                    wide = wide.max(width(x) + width(y));
                }
            }
        }
        if worst >= clearance {
            continue;
        }
        open = open.min(worst);
        let k = (0..qb.len())
            .filter(|&j| w0[j] > 0.0)
            .max_by(|&i, &j| (qb[i].width() / w0[i]).total_cmp(&(qb[j].width() / w0[j])));
        let mid: Vec<f64> = qb.iter().map(|x| 0.5 * (x.lo + x.hi)).collect();
        if wide <= RES || k.is_none_or(|j| qb[j].width() <= RES * 1e-3) {
            let p = kin::pose(def, &mid);
            if let Some(w) = witness(a, b, &p) {
                return (BoxVerdict::Hit { q: mid, witness: w }, boxes);
            }
            // a piece at the resolution, not proven clear and no witness: the pair is Near. The
            // search ends here: the leaves along a contact surface are too many to visit, and
            // one is enough to say the pair is not clear
            return (
                BoxVerdict::Near {
                    q: mid,
                    gap_lo: worst,
                },
                boxes,
            );
        }
        // a witness at the middle ends the search early
        let p = kin::pose(def, &mid);
        if let Some(w) = witness(a, b, &p) {
            return (BoxVerdict::Hit { q: mid, witness: w }, boxes);
        }
        let j = k.unwrap();
        let m = mid[j];
        let (mut lo, mut hi) = (qb.clone(), qb);
        lo[j] = Interval::new(lo[j].lo, m);
        hi[j] = Interval::new(m, hi[j].hi);
        stack.push(hi);
        stack.push(lo);
    }
    (BoxVerdict::Clear, boxes)
}

/// A dense sampling test for overlap (tests only: a grid of points through each primitive of a,
/// checked inside b, and the other way). Not a proof of anything; the soundness test uses it to
/// look for a contradiction of `Clear`.
pub fn dense_overlap(a: &CBody, b: &CBody, pose: &[Rigid]) -> bool {
    let tl = |l: Option<usize>| match l {
        Some(i) => pose[i],
        None => Rigid::ID,
    };
    let (ta, tb) = (tl(a.link), tl(b.link));
    let grid = |p: &Prim, t: &Rigid| -> Vec<V3> {
        let n = 9;
        let mut out = Vec::new();
        match p {
            Prim::Box { min, max } => {
                for i in 0..n {
                    for j in 0..n {
                        for k in 0..n {
                            let f = |m: usize, lo: f64, hi: f64| lo + (hi - lo) * m as f64 / (n - 1) as f64;
                            out.push(t.apply([
                                f(i, min[0], max[0]),
                                f(j, min[1], max[1]),
                                f(k, min[2], max[2]),
                            ]));
                        }
                    }
                }
            }
            Prim::OBox { o, axes, size } => {
                for i in 0..n {
                    for j in 0..n {
                        for k in 0..n {
                            let f = [i, j, k].map(|m| m as f64 / (n - 1) as f64);
                            out.push(t.apply(Prim::obox_at(o, axes, size, f)));
                        }
                    }
                }
            }
            Prim::Cyl { base, axis, r, h } => {
                let u = if axis[0].abs() < 0.9 {
                    kin::cross(*axis, [1.0, 0.0, 0.0])
                } else {
                    kin::cross(*axis, [0.0, 1.0, 0.0])
                };
                let u = kin::scale(u, 1.0 / kin::norm(u));
                let v = kin::cross(*axis, u);
                for i in 0..n {
                    for j in 0..n {
                        for k in 0..2 * n {
                            let z = h * i as f64 / (n - 1) as f64;
                            let rr = r * j as f64 / (n - 1) as f64;
                            let (sn, cs) = kin::sin_cos_deg(360.0 * k as f64 / (2 * n) as f64);
                            let p = kin::add(
                                kin::add(*base, kin::scale(*axis, z)),
                                kin::add(kin::scale(u, rr * cs), kin::scale(v, rr * sn)),
                            );
                            out.push(t.apply(p));
                        }
                    }
                }
            }
        }
        out
    };
    for (x, tx, y, ty) in [(a, ta, b, tb), (b, tb, a, ta)] {
        for px in &x.prims {
            for pt in grid(px, &tx) {
                if y.prims.iter().any(|py| depth(py, &ty, pt) > 0.0) {
                    return true;
                }
            }
        }
    }
    false
}

// ------------------------------------------------------------------------------------------
// Two channels (sim::multi): a pair of bodies, each moved by one channel's segment, with the two
// segments' parameters independent (any relative timing).

/// The channels (joint letter digits: Z2 is channel 2, Z channel 1) whose joints move a body.
pub fn body_channels(def: &MachineDef, b: &CBody) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let Some(l) = b.link else { return out };
    for j in def.chain(&def.joints[l].name).unwrap_or_default() {
        let letter = &def.joints[j].letter;
        let base = letter.trim_end_matches(|c: char| c.is_ascii_digit());
        let ch: u8 = letter[base.len()..].parse().unwrap_or(1);
        if !out.contains(&ch) {
            out.push(ch);
        }
    }
    out
}

/// The joints channel `ch` drives.
pub fn channel_joints(def: &MachineDef, ch: u8) -> Vec<usize> {
    (0..def.joints.len())
        .filter(|&j| {
            let letter = &def.joints[j].letter;
            let base = letter.trim_end_matches(|c: char| c.is_ascii_digit());
            letter[base.len()..].parse::<u8>().unwrap_or(1) == ch
        })
        .collect()
}

/// Body `a` moved by segment `sa` (channel A's joints `ja`) against body `b` moved by `sb` (the
/// other joints), over every pair of times: Clear only if the pair keeps `clearance` for all
/// sa ∈ [0, 1] and sb ∈ [0, 1] independently. A Hit's `s` is sa's.
#[allow(clippy::too_many_arguments)]
pub fn check_two(
    def: &MachineDef,
    sa: &Segment,
    sb: &Segment,
    ja: &[usize],
    a: &CBody,
    b: &CBody,
    clearance: f64,
    spans: &mut usize,
) -> Verdict {
    let mut budget = TWO_BUDGET;
    two_span(
        def,
        sa,
        sb,
        ja,
        a,
        b,
        [0.0, 1.0],
        [0.0, 1.0],
        clearance,
        TWO_DEPTH,
        spans,
        &mut budget,
    )
}

/// Two parameters split one at a time: this deep reaches 2^-12 of each segment. Past the
/// budget of spans a pair is left Near (not proven clear), never Clear.
const TWO_DEPTH: u32 = 24;
const TWO_BUDGET: usize = 4096;

#[allow(clippy::too_many_arguments)]
fn two_span(
    def: &MachineDef,
    sa: &Segment,
    sb: &Segment,
    ja: &[usize],
    a: &CBody,
    b: &CBody,
    ra: [f64; 2],
    rb: [f64; 2],
    clearance: f64,
    depth_left: u32,
    spans: &mut usize,
    budget: &mut usize,
) -> Verdict {
    *spans += 1;
    let qa = q_span(sa, ra[0], ra[1]);
    let qb = q_span(sb, rb[0], rb[1]);
    let q: Vec<Interval> = (0..qa.len())
        .map(|j| if ja.contains(&j) { qa[j] } else { qb[j] })
        .collect();
    let pose = kin::ipose(def, &q);
    let (ta, tb) = (ilink(&pose, a.link), ilink(&pose, b.link));
    let pa: Vec<IPrim> = a.prims.iter().map(|p| place(p, &ta)).collect();
    let pb: Vec<IPrim> = b.prims.iter().map(|p| place(p, &tb)).collect();
    let mut worst = f64::INFINITY;
    let mut wide = 0.0f64;
    for x in &pa {
        for y in &pb {
            let g = gap_lo(x, y);
            worst = worst.min(g);
            if g < clearance {
                wide = wide.max(width(x) + width(y));
            }
        }
    }
    if worst >= clearance {
        return Verdict::Clear;
    }
    let (ma, mb) = (0.5 * (ra[0] + ra[1]), 0.5 * (rb[0] + rb[1]));
    *budget = budget.saturating_sub(1);
    if wide <= RES || depth_left == 0 || *budget == 0 {
        let (xa, xb) = (sa.at(ma), sb.at(mb));
        let qm: Vec<f64> = (0..xa.len())
            .map(|j| if ja.contains(&j) { xa[j] } else { xb[j] })
            .collect();
        if let Some(w) = witness(a, b, &kin::pose(def, &qm)) {
            return Verdict::Hit { s: ma, witness: w };
        }
        return Verdict::Near {
            s0: ra[0],
            s1: ra[1],
            gap_lo: worst,
        };
    }
    // split the parameter whose segment moves more over its span
    let move_of = |sg: &Segment, r: [f64; 2]| -> f64 {
        sg.q0
            .iter()
            .zip(&sg.q1)
            .map(|(x, y)| (y - x).abs())
            .fold(0.0, f64::max)
            * (r[1] - r[0])
    };
    let halves: [([f64; 2], [f64; 2]); 2] = if move_of(sa, ra) >= move_of(sb, rb) {
        [([ra[0], ma], rb), ([ma, ra[1]], rb)]
    } else {
        [(ra, [rb[0], mb]), (ra, [mb, rb[1]])]
    };
    let first = two_span(
        def,
        sa,
        sb,
        ja,
        a,
        b,
        halves[0].0,
        halves[0].1,
        clearance,
        depth_left - 1,
        spans,
        budget,
    );
    if matches!(first, Verdict::Hit { .. }) {
        return first;
    }
    let second = two_span(
        def,
        sa,
        sb,
        ja,
        a,
        b,
        halves[1].0,
        halves[1].1,
        clearance,
        depth_left - 1,
        spans,
        budget,
    );
    match (&first, &second) {
        (_, Verdict::Hit { .. }) | (Verdict::Clear, _) => second,
        _ => first,
    }
}

/// `check` over the pairs `keep` admits (a multi-channel run checks each channel's own pairs
/// with this, the cross-channel ones with check_two).
pub fn check_where(
    s: &Setup,
    tl: &Timeline,
    work: &[CBody],
    keep: &dyn Fn(&CBody, &CBody) -> bool,
) -> Report {
    let def = s.def;
    let machine = machine_bodies(def);
    let mut rep = Report::default();
    for (si, seg) in tl.segments.iter().enumerate() {
        let tools: Vec<CBody> = seg.tool.map(|t| tool_bodies(s, t)).unwrap_or_default();
        let all: Vec<&CBody> = machine.iter().chain(tools.iter()).chain(work.iter()).collect();
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                let (a, b) = (all[i], all[j]);
                if !may_pair(a, b, seg.kind) || !keep(a, b) {
                    continue;
                }
                rep.pairs_checked += 1;
                let v = check_span(
                    def,
                    seg,
                    a,
                    b,
                    0.0,
                    1.0,
                    pair_clearance(def, a, b),
                    MAX_DEPTH,
                    &mut rep.spans,
                );
                if v != Verdict::Clear {
                    rep.contacts.push(Contact {
                        line: seg.line,
                        segment: si,
                        a: a.name.clone(),
                        b: b.name.clone(),
                        verdict: v,
                    });
                }
            }
        }
    }
    rep
}

/// may_pair and the pair's clearance, for sim::multi.
pub fn pair_rule(def: &MachineDef, a: &CBody, b: &CBody, kind: SegKind) -> Option<f64> {
    may_pair(a, b, kind).then(|| pair_clearance(def, a, b))
}
