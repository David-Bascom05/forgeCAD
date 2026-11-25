//! The bend simulation as a 3D scene, for a viewer: at any instant of the plan, the part (the
//! exact partial refold of its walls and zones, tessellated), the tools (their exact sections
//! extruded along the bed), and the press's bodies (ram, bed, side housings, backgauge), each
//! with its look and whether a failing check names it. Renderer-independent: triangles,
//! normals, an appearance id. For the eye only: no check reads this.
//!
//! Frame: the press's (x across the bed towards the backgauge, y along the bed from its centre,
//! z up), z = 0 on the top of the working station's die, mm.
use crate::geo::{Region, Seg};
use crate::model::Model;
use crate::num::Sc;
use crate::plan::Plan;
use crate::sim::{
    Check, Contact, HandleTarget, Kind, Moving, Phase, Piece, Rule, Severity, Step, StepCtx, Surface,
};
use crate::tooling::{Punch, Segment, Tooling};
use forge_math::{Interval, Point2, dmath};
use forge_sheet::Strip;

/// Triangles with a normal per vertex.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[f32; 3]>,
    pub tri: Vec<[u32; 3]>,
}

impl Mesh {
    /// Another mesh's triangles added to this one's.
    pub fn append(&mut self, o: &Mesh) {
        let k = self.pos.len() as u32;
        self.pos.extend_from_slice(&o.pos);
        self.nrm.extend_from_slice(&o.nrm);
        self.tri
            .extend(o.tri.iter().map(|t| [t[0] + k, t[1] + k, t[2] + k]));
    }

    fn push_tri(&mut self, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
        let n = normal(a, b, c);
        let k = self.pos.len() as u32;
        for p in [a, b, c] {
            self.pos.push(f3(p));
            self.nrm.push(f3(n));
        }
        self.tri.push([k, k + 1, k + 2]);
    }

    fn push_quad(&mut self, a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) {
        self.push_tri(a, b, c);
        self.push_tri(a, c, d);
    }

    /// An axis-aligned box.
    fn push_box(&mut self, lo: [f64; 3], hi: [f64; 3]) {
        let p = |i: usize| {
            [
                if i & 1 == 0 { lo[0] } else { hi[0] },
                if i & 2 == 0 { lo[1] } else { hi[1] },
                if i & 4 == 0 { lo[2] } else { hi[2] },
            ]
        };
        // outward, counter-clockwise seen from outside
        for f in [
            [0, 2, 3, 1], // z-
            [4, 5, 7, 6], // z+
            [0, 1, 5, 4], // y-
            [2, 6, 7, 3], // y+
            [0, 4, 6, 2], // x-
            [1, 3, 7, 5], // x+
        ] {
            self.push_quad(p(f[0]), p(f[1]), p(f[2]), p(f[3]));
        }
    }

    /// Bounds, or None if empty.
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut it = self.pos.iter();
        let first = *it.next()?;
        Some(it.fold((first, first), |(lo, hi), p| {
            (
                [lo[0].min(p[0]), lo[1].min(p[1]), lo[2].min(p[2])],
                [hi[0].max(p[0]), hi[1].max(p[1]), hi[2].max(p[2])],
            )
        }))
    }
}

/// How far a contact's region is drawn off the part's skin (mm), so it shows over it.
pub const CONTACT_LIFT: f64 = 0.05;

/// Zone pieces of a contact's footprint are drawn no longer than this (mm), so they follow the
/// bend's curve.
const CONTACT_STEP: f64 = 0.5;

/// The (u, v) rectangles of contact cells (any depth, any motion), the same ones once, and merged
/// where they touch: first along u in rows of the same v span, then along v in columns of the
/// same u span. Their union is the cells'.
fn footprint(cells: &[&[Interval; 4]]) -> Vec<(Point2, Point2)> {
    let mut r: Vec<[f64; 4]> = cells
        .iter()
        .map(|c| [c[0].lo, c[0].hi, c[1].lo, c[1].hi])
        .collect();
    r.sort_by(|a, b| {
        a.iter()
            .zip(b)
            .map(|(x, y)| x.total_cmp(y))
            .find(|o| o.is_ne())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    r.dedup();
    // rows: the same v span, touching in u
    let merge = |r: &mut Vec<[f64; 4]>, along: usize, key: [usize; 2]| {
        r.sort_by(|a, b| {
            a[key[0]]
                .total_cmp(&b[key[0]])
                .then(a[key[1]].total_cmp(&b[key[1]]))
                .then(a[along].total_cmp(&b[along]))
        });
        let mut out: Vec<[f64; 4]> = Vec::with_capacity(r.len());
        for x in r.drain(..) {
            if let Some(l) = out.last_mut()
                && l[key[0]] == x[key[0]]
                && l[key[1]] == x[key[1]]
                && x[along] <= l[along + 1]
            {
                l[along + 1] = l[along + 1].max(x[along + 1]);
                continue;
            }
            out.push(x);
        }
        *r = out;
    };
    merge(&mut r, 0, [2, 3]);
    merge(&mut r, 2, [0, 1]);
    r.into_iter()
        .map(|x| (Point2::new(x[0], x[2]), Point2::new(x[1], x[3])))
        .collect()
}

/// The part of the segment a -> b inside the rectangle [lo, hi], as parameters along it.
fn clip_segment(a: Point2, b: Point2, lo: Point2, hi: Point2) -> Option<(f64, f64)> {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, d, l, h) in [(a.x, b.x - a.x, lo.x, hi.x), (a.y, b.y - a.y, lo.y, hi.y)] {
        if d.abs() < 1e-15 {
            if p < l || p > h {
                return None;
            }
            continue;
        }
        let (u, v) = ((l - p) / d, (h - p) / d);
        let (u, v) = if u < v { (u, v) } else { (v, u) };
        t0 = t0.max(u);
        t1 = t1.min(v);
    }
    (t1 > t0).then_some((t0, t1))
}

fn f3(p: [f64; 3]) -> [f32; 3] {
    [p[0] as f32, p[1] as f32, p[2] as f32]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt();
    if l > 0.0 {
        [a[0] / l, a[1] / l, a[2] / l]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn normal(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> [f64; 3] {
    unit(cross(sub(b, a), sub(c, a)))
}

/// What a body looks like: the renderer maps it to an appearance (RENDER-STUDIO's ids).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Look {
    /// The sheet, in its material's appearance.
    Sheet,
    /// Punches and dies: ground tool steel.
    Tool,
    /// The press frame and beams: painted steel.
    Frame,
    /// The backgauge's fingers and crossbar.
    Gauge,
    Floor,
    /// A collision's witness point.
    Witness,
    /// The bend being made: its zone on the sheet, marked so the eye finds it.
    BendZone,
}

impl Look {
    /// The sheet's own bodies, its bend being made among them.
    pub fn is_sheet(self) -> bool {
        matches!(self, Look::Sheet | Look::BendZone)
    }

    /// The appearance id a renderer asks its library for.
    pub fn appearance(self) -> &'static str {
        match self {
            Look::Sheet => "sheet",
            Look::Tool => "tool-steel-ground",
            Look::Frame => "paint-machine-gloss",
            Look::Gauge => "steel-black-oxide",
            Look::Floor => "concrete-floor",
            Look::Witness => "highlight-red",
            Look::BendZone => "highlight-bend",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    /// As the checks name it (`punch:ID/body`, `die:ID`, `ram`, `bed`, `frame:left`,
    /// `backgauge:finger1`, `station2:die:ID`), or `part:<piece>`.
    pub name: String,
    pub look: Look,
    pub mesh: Mesh,
    /// A failing check of the step names it.
    pub hit: bool,
    /// For the part's pieces: the strip.
    pub strip: Option<Strip>,
}

/// Where in a step the instant falls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The operator turns the part to this step's pose (not checked: said so).
    Handle,
    /// The backgauge moves to its stop; the part is pushed against it.
    Gauge,
    Approach,
    Bend,
    /// From the overbend back to the turn.
    Springback,
    Return,
}

impl Stage {
    pub fn word(self) -> &'static str {
        match self {
            Stage::Handle => "handle",
            Stage::Gauge => "gauge",
            Stage::Approach => "approach",
            Stage::Bend => "bend",
            Stage::Springback => "springback",
            Stage::Return => "return",
        }
    }
}

/// A step's stages over t in [0, 1): (stage, from, to).
pub const STAGES: [(Stage, f64, f64); 6] = [
    (Stage::Handle, 0.0, 0.15),
    (Stage::Gauge, 0.15, 0.25),
    (Stage::Approach, 0.25, 0.4),
    (Stage::Bend, 0.4, 0.75),
    (Stage::Springback, 0.75, 0.82),
    (Stage::Return, 0.82, 1.0),
];

/// The instant: the stage, the bend's turn, the punch tip's height, the backgauge's stop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub stage: Stage,
    /// Within the stage, 0..1.
    pub f: f64,
    pub phi: f64,
    pub tip: f64,
    /// The backgauge's stop face x and finger bottom r (when gauged).
    pub gauge: Option<(f64, f64)>,
    /// The part's rigid motion during handling: from the previous step's last pose (1 - f of
    /// the way back), as a lift and turn.
    pub handle: f64,
}

/// The stage at `t` in [0, 1].
pub fn stage_at(t: f64) -> (Stage, f64) {
    let t = t.clamp(0.0, 1.0);
    for (s, a, b) in STAGES {
        if t < b || s == Stage::Return {
            return (s, ((t - a) / (b - a)).clamp(0.0, 1.0));
        }
    }
    (Stage::Return, 1.0)
}

fn smooth(f: f64) -> f64 {
    f * f * (3.0 - 2.0 * f)
}

/// `triangulate`, kept by the region's content and the chords: a piece's region is the same in
/// every step and every reply of an unchanged part, so a replan, a tool change or scrubbing the
/// steps tessellates it once. The key is the region's exact text (its doubles printed to round
/// trip); a hit is the same region, so the same triangles. Looked up by key only, never iterated.
fn tessellated(region: &Region, h: f64, zone: Option<f64>) -> Option<Tess> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static KEPT: Mutex<Option<HashMap<String, Option<Tess>>>> = Mutex::new(None);
    let key = format!("{h:?}|{zone:?}|{region:?}");
    if let Ok(g) = KEPT.lock()
        && let Some(t) = g.as_ref().and_then(|m| m.get(&key))
    {
        return t.clone();
    }
    let t = triangulate(region, h, zone);
    if let Ok(mut g) = KEPT.lock() {
        let m = g.get_or_insert_with(HashMap::new);
        // bounded: a long session of many parts starts afresh
        if m.len() >= 4096 {
            m.clear();
        }
        m.insert(key, t.clone());
    }
    t
}

/// A step's tessellated pieces in their own parameters, kept while the step is shown.
struct PieceMesh {
    /// Boundary loops' points and the triangles over them, in the piece's (u, v).
    pts: Vec<Point2>,
    tris: Vec<[u32; 3]>,
    /// Boundary edges, with the side the region lies on (true: left).
    edges: Vec<(u32, u32)>,
    strip: Strip,
}

/// Samples a segment: lines cut to pieces at most `h` long, arcs every `da` radians.
fn sample(s: &Seg, h: f64, out: &mut Vec<Point2>) {
    let n = match s {
        Seg::Line { a, b } => ((a.distance(*b) / h).ceil() as usize).clamp(1, 200),
        Seg::Arc { r, sweep, .. } => ((sweep.abs() / 0.12).ceil() as usize)
            .max(((r * sweep.abs()) / h).ceil() as usize)
            .clamp(2, 200),
    };
    for i in 0..n {
        out.push(s.point(i as f64 / n as f64));
    }
}

/// The region's boundary chained into loops of points (each segment sampled), or None if its
/// pieces do not close.
fn loops(r: &Region, h: f64) -> Option<Vec<Vec<Point2>>> {
    let eps = 1e-6;
    let mut segs: Vec<Seg> = r.bound.clone();
    let mut out = Vec::new();
    while let Some(first) = segs.pop() {
        let start = first.start();
        let mut pts = Vec::new();
        sample(&first, h, &mut pts);
        let mut end = first.end();
        let mut guard = 0;
        while end.distance(start) > eps {
            guard += 1;
            if guard > 100_000 {
                return None;
            }
            let k = segs.iter().position(|s| s.start().distance(end) < eps);
            match k {
                Some(k) => {
                    let s = segs.swap_remove(k);
                    sample(&s, h, &mut pts);
                    end = s.end();
                }
                None => {
                    // a segment running the other way
                    let k = segs.iter().position(|s| s.end().distance(end) < eps)?;
                    let s = segs.swap_remove(k);
                    let mut back = Vec::new();
                    sample(&s, h, &mut back);
                    back.push(s.end());
                    back.reverse();
                    back.pop();
                    pts.extend(back);
                    end = s.start();
                }
            }
        }
        if pts.len() >= 3 {
            out.push(pts);
        }
    }
    Some(out)
}

/// Points, triangles over them, and the boundary's edges.
type Tess = (Vec<Point2>, Vec<[u32; 3]>, Vec<(u32, u32)>);

/// A 2D region triangulated (with interior points every `grid`, if any): points, triangles and
/// boundary edges.
fn triangulate(r: &Region, h: f64, grid: Option<f64>) -> Option<Tess> {
    let ls = loops(r, h)?;
    let mut pts = Vec::new();
    let mut idx = Vec::new();
    let mut edges = Vec::new();
    for l in &ls {
        let base = pts.len() as u32;
        let n = l.len() as u32;
        pts.extend(l.iter().copied());
        idx.push((0..n).map(|i| base + i).collect::<Vec<u32>>());
        for i in 0..n {
            edges.push((base + i, base + (i + 1) % n));
        }
    }
    let mut interior = Vec::new();
    if let Some(g) = grid {
        let (lo, hi) = r.bbox();
        let nx = (((hi.x - lo.x) / g).ceil() as usize).clamp(1, 400);
        let ny = (((hi.y - lo.y) / g).ceil() as usize).clamp(1, 400);
        for i in 1..nx {
            for j in 1..ny {
                let p = Point2::new(
                    lo.x + (hi.x - lo.x) * i as f64 / nx as f64,
                    lo.y + (hi.y - lo.y) * j as f64 / ny as f64,
                );
                if r.contains(p) && r.sd(p) < -0.25 * g {
                    interior.push(p);
                }
            }
        }
    }
    let t = forge_mesh::cdt::triangulate_within(
        &pts,
        &idx,
        &interior,
        None::<&dyn forge_mesh::cdt::Refine>,
        true,
    )
    .ok()?;
    Some((t.points, t.triangles, edges))
}

/// An instant's part: the step's context placed at a turn, in the die's frame.
fn part_point(ctx: &StepCtx, pc: &Piece, p: Point2, w: f64, phi: f64) -> [f64; 3] {
    let q = ctx.side_point(pc, p.x, p.y, w, phi);
    let r = ctx.rel(q, pc.side, phi);
    [r.x, r.y + ctx.oy, r.z + ctx.ztip(phi)]
}

/// A rigid motion: rows of a 3x4 matrix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rigid3 {
    pub r: [[f64; 3]; 3],
    pub o: [f64; 3],
}

impl Rigid3 {
    pub const ID: Rigid3 = Rigid3 {
        r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        o: [0.0; 3],
    };

    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        let r = &self.r;
        [
            r[0][0] * p[0] + r[0][1] * p[1] + r[0][2] * p[2] + self.o[0],
            r[1][0] * p[0] + r[1][1] * p[1] + r[1][2] * p[2] + self.o[1],
            r[2][0] * p[0] + r[2][1] * p[1] + r[2][2] * p[2] + self.o[2],
        ]
    }

    pub fn vec(&self, p: [f64; 3]) -> [f64; 3] {
        let r = &self.r;
        [
            r[0][0] * p[0] + r[0][1] * p[1] + r[0][2] * p[2],
            r[1][0] * p[0] + r[1][1] * p[1] + r[1][2] * p[2],
            r[2][0] * p[0] + r[2][1] * p[1] + r[2][2] * p[2],
        ]
    }

    /// The frame on three points: origin a, x towards b, z normal to abc.
    fn on(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Rigid3 {
        let x = unit(sub(b, a));
        let z = unit(cross(x, sub(c, a)));
        let y = cross(z, x);
        Rigid3 {
            r: [[x[0], y[0], z[0]], [x[1], y[1], z[1]], [x[2], y[2], z[2]]],
            o: a,
        }
    }

    fn inverse(&self) -> Rigid3 {
        let r = &self.r;
        let t = [
            [r[0][0], r[1][0], r[2][0]],
            [r[0][1], r[1][1], r[2][1]],
            [r[0][2], r[1][2], r[2][2]],
        ];
        let o = self.o;
        let ti = Rigid3 { r: t, o: [0.0; 3] };
        let mo = ti.vec(o);
        Rigid3 {
            r: t,
            o: [-mo[0], -mo[1], -mo[2]],
        }
    }

    fn then(&self, inner: &Rigid3) -> Rigid3 {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x = (0..3).map(|k| self.r[i][k] * inner.r[k][j]).sum();
            }
        }
        Rigid3 {
            r,
            o: self.apply(inner.o),
        }
    }

    /// The rotation's axis and angle.
    fn axis_angle(&self) -> ([f64; 3], f64) {
        let r = &self.r;
        let c = ((r[0][0] + r[1][1] + r[2][2] - 1.0) * 0.5).clamp(-1.0, 1.0);
        let a = dmath::acos(c);
        let v = [r[2][1] - r[1][2], r[0][2] - r[2][0], r[1][0] - r[0][1]];
        let l = dot(v, v).sqrt();
        if l > 1e-9 {
            return ([v[0] / l, v[1] / l, v[2] / l], a);
        }
        if a < 1e-6 {
            return ([0.0, 0.0, 1.0], 0.0);
        }
        // a half turn: the axis is the column of r + I with the largest norm
        let m = [
            [r[0][0] + 1.0, r[0][1], r[0][2]],
            [r[1][0], r[1][1] + 1.0, r[1][2]],
            [r[2][0], r[2][1], r[2][2] + 1.0],
        ];
        let cols: Vec<[f64; 3]> = (0..3).map(|j| [m[0][j], m[1][j], m[2][j]]).collect();
        let best = cols
            .into_iter()
            .max_by(|a, b| dot(*a, *a).total_cmp(&dot(*b, *b)))
            .unwrap();
        (unit(best), a)
    }

    fn about(axis: [f64; 3], a: f64) -> Rigid3 {
        let (s, c) = dmath::sin_cos(a);
        let [x, y, z] = axis;
        let t = 1.0 - c;
        Rigid3 {
            r: [
                [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
                [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
                [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
            ],
            o: [0.0; 3],
        }
    }
}

/// The operator's move between two steps (see `Scene::handling_path`), applied to a point of the
/// part as the new step places it.
#[derive(Clone, Debug, PartialEq)]
pub struct HandlingPath {
    /// The new pose to the old.
    pub h: Rigid3,
    /// The part's centre (new pose) and the radius about it that holds all of it.
    pub c: [f64; 3],
    pub radius: f64,
    /// The lift off the die, and the pulls out towards the operator from the old and new
    /// places.
    pub delta: f64,
    pub d_old: f64,
    pub d_new: f64,
    /// How far the true surface may lie off the samples.
    pub sag: f64,
}

/// The handling's phases over f in [0, 1]: lift, pull out, turn, push in, set down.
pub const HANDLING: [f64; 4] = [0.12, 0.32, 0.68, 0.88];

impl Moving for HandlingPath {
    fn knots(&self) -> &[f64] {
        &KNOTS
    }
    fn place(&self, p: [f64; 3], f: f64) -> [f64; 3] {
        HandlingPath::place(self, p, f)
    }
    /// Each span's formula on intervals: the smooth step is monotone, so its range over an
    /// interval of f is its values at the ends (each outward-rounded); the turn's rotation by
    /// Rodrigues' formula with the interval sine and versine of its angle.
    fn enclose(&self, p: [Interval; 3], f: Interval) -> [Interval; 3] {
        let span = KNOTS
            .windows(2)
            .position(|w| f.mid() >= w[0] && f.mid() <= w[1])
            .unwrap_or(0);
        let (k0, k1) = (KNOTS[span], KNOTS[span + 1]);
        let ends = |x: f64| {
            let x = Interval::point(x);
            let t = (x + (-k0)).div_c(k1 - k0);
            let t = Interval::new(t.lo.max(0.0), t.hi.min(1.0));
            // smooth(t) = t^2 (3 - 2t)
            t.sqr() * (t * -2.0 + 3.0)
        };
        let s = Interval::new(ends(f.lo).lo.max(0.0), ends(f.hi).hi.min(1.0));
        let pt = |v: f64| Interval::point(v);
        let rigid = |h: &Rigid3, p: [Interval; 3]| {
            [0, 1, 2].map(|i| p[0] * h.r[i][0] + p[1] * h.r[i][1] + p[2] * h.r[i][2] + h.o[i])
        };
        let (d, d_old, d_new) = (self.delta, self.d_old, self.d_new);
        match span {
            0 => {
                let q = rigid(&self.h, p);
                [q[0], q[1], q[2] + s * d]
            }
            1 => {
                let q = rigid(&self.h, p);
                [q[0] + s * -d_old, q[1], q[2] + d]
            }
            2 => {
                let (k, ang) = self.h.axis_angle();
                // the angle ang (1 - t)
                let th = s * -ang + ang;
                let (sn, vs) = (Sc::sin(th), th.vers());
                let v = [0, 1, 2].map(|i| p[i] + -self.c[i]);
                let kxv = [
                    v[2] * k[1] - v[1] * k[2],
                    v[0] * k[2] - v[2] * k[0],
                    v[1] * k[0] - v[0] * k[1],
                ];
                let kv = v[0] * k[0] + v[1] * k[1] + v[2] * k[2];
                // R v = v + sin(th) (k x v) + vers(th) (k (k . v) - v)
                let rv = [0, 1, 2].map(|i| v[i] + sn * kxv[i] + vs * (kv * k[i] - v[i]));
                let ca = add(self.h.apply(self.c), [-d_old, 0.0, d]);
                let cb = add(self.c, [-d_new, 0.0, d]);
                [0, 1, 2].map(|i| rv[i] + (pt(ca[i]) + s * (cb[i] - ca[i])))
            }
            3 => [p[0] + (s * -1.0 + 1.0) * -d_new, p[1], p[2] + d],
            _ => [p[0], p[1], p[2] + (s * -1.0 + 1.0) * d],
        }
    }
}

/// The handling's knots: lift, pull out, turn, push in, set down.
pub const KNOTS: [f64; 6] = [0.0, HANDLING[0], HANDLING[1], HANDLING[2], HANDLING[3], 1.0];

impl HandlingPath {
    pub fn place(&self, p: [f64; 3], f: f64) -> [f64; 3] {
        let [a, b, c, d] = HANDLING;
        let s = |lo: f64, hi: f64| smooth(((f - lo) / (hi - lo)).clamp(0.0, 1.0));
        let up = [0.0, 0.0, self.delta];
        if f < b {
            let q = self.h.apply(p);
            let lift = s(0.0, a);
            let pull = s(a, b);
            return add(q, [-self.d_old * pull, 0.0, self.delta * lift]);
        }
        if f < c {
            // turned about the part's centre, the centre moved straight between the two
            // pulled-out places
            let t = s(b, c);
            let (axis, ang) = self.h.axis_angle();
            let r = Rigid3::about(axis, ang * (1.0 - t));
            let ca = add(self.h.apply(self.c), add([-self.d_old, 0.0, 0.0], up));
            let cb = add(self.c, add([-self.d_new, 0.0, 0.0], up));
            let q = r.vec(sub(p, self.c));
            return [0, 1, 2].map(|i| q[i] + ca[i] + (cb[i] - ca[i]) * t);
        }
        let push = s(c, d);
        let down = s(d, 1.0);
        add(p, [-self.d_new * (1.0 - push), 0.0, self.delta * (1.0 - down)])
    }
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn bbox(pts: &[[f64; 3]]) -> Option<([f64; 3], [f64; 3])> {
    let first = *pts.first()?;
    Some(pts.iter().fold((first, first), |(lo, hi), p| {
        (
            [0, 1, 2].map(|i| lo[i].min(p[i])),
            [0, 1, 2].map(|i| hi[i].max(p[i])),
        )
    }))
}

/// The gap between two boxes: their distance if apart, less than zero (the least overlap
/// along an axis, negated) if they meet.
fn box_gap(a: ([f64; 3], [f64; 3]), b: ([f64; 3], [f64; 3])) -> f64 {
    let gaps: [f64; 3] = [0, 1, 2].map(|i| (b.0[i] - a.1[i]).max(a.0[i] - b.1[i]));
    if gaps.iter().any(|g| *g > 0.0) {
        gaps.iter().map(|g| g.max(0.0).powi(2)).sum::<f64>().sqrt()
    } else {
        gaps.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    }
}

/// Every step after the first gets its handling check (a warning when not proven clear).
pub fn annotate_handling(m: &Model, tl: &Tooling, plan: &mut Plan) {
    let checks: Vec<(usize, Check)> = {
        let mut sc = Scene::new(m, tl, plan);
        (1..plan.steps.len())
            .filter_map(|k| sc.handling_check(k).map(|c| (k, c)))
            .collect()
    };
    for (k, c) in checks {
        plan.steps[k].checks.push(c);
    }
}

/// The plan as a scene: one step's meshes kept at a time.
pub struct Scene<'a> {
    pub m: &'a Model,
    pub tl: &'a Tooling,
    pub plan: &'a Plan,
    /// Table height above the floor (the press record's, else 900 mm: drawn only).
    pub table_height: f64,
    /// Chord length for the part's boundaries and zones, mm.
    pub chord: f64,
    kept: Option<(usize, Vec<Option<PieceMesh>>)>,
    kept_path: Option<(usize, Option<HandlingPath>)>,
    ctxs: std::cell::RefCell<Vec<Option<std::rc::Rc<StepCtx<'a>>>>>,
}

impl<'a> Scene<'a> {
    pub fn new(m: &'a Model, tl: &'a Tooling, plan: &'a Plan) -> Scene<'a> {
        Scene {
            m,
            tl,
            plan,
            table_height: 900.0,
            chord: 4.0,
            kept: None,
            kept_path: None,
            ctxs: Default::default(),
        }
    }

    /// The bends done before step `k`.
    pub fn done_before(&self, k: usize) -> Vec<bool> {
        let mut d = vec![false; self.m.bends.len()];
        for st in self.plan.steps.iter().take(k) {
            d[st.bend] = true;
        }
        d
    }

    /// Step `k`'s context, built once while the scene lives (a move reads two steps' several
    /// times, each build costs about a millisecond).
    fn ctx(&self, k: usize) -> std::rc::Rc<StepCtx<'a>> {
        if let Some(Some(c)) = self.ctxs.borrow().get(k) {
            return c.clone();
        }
        let c = std::rc::Rc::new(self.build_ctx(k));
        let mut v = self.ctxs.borrow_mut();
        if v.len() <= k {
            v.resize(k + 1, None);
        }
        v[k] = Some(c.clone());
        c
    }

    fn build_ctx(&self, k: usize) -> StepCtx<'a> {
        let st = &self.plan.steps[k];
        let ctx = StepCtx::new(
            self.m,
            self.tl,
            &self.done_before(k),
            st.bend,
            st.setup,
            st.overbend,
        );
        // the station the plan worked it at (a lock may set one its tools would not pick)
        match st.station {
            Some(i) if ctx.station != Some(i) && i < self.tl.stations.len() => ctx.at_station(i),
            _ => ctx,
        }
    }

    /// The instant at `t` in [0, 1] of step `k`.
    pub fn pose(&self, k: usize, t: f64) -> Pose {
        let st = &self.plan.steps[k];
        let ctx = self.ctx(k);
        let (stage, f) = stage_at(t);
        let g = st.gauge.as_ref().map(|g| (g.x, g.r));
        let prev_g = if k > 0 {
            self.plan.steps[k - 1].gauge.as_ref().map(|g| (g.x, g.r))
        } else {
            None
        };
        let lerp = |a: f64, b: f64, f: f64| a + (b - a) * smooth(f);
        let gauge = match (stage, g, prev_g) {
            // the backgauge moves to this step's stop while the operator turns the part
            (Stage::Handle, _, p) if f < HANDLING[1] => p.or(g),
            (Stage::Handle, Some(b), Some(a)) if f < HANDLING[2] => {
                let u = (f - HANDLING[1]) / (HANDLING[2] - HANDLING[1]);
                Some((lerp(a.0, b.0, u), lerp(a.1, b.1, u)))
            }
            _ => g,
        };
        // the punch's top: top dead centre, but never below the flat sheet it meets (tools taller
        // than the open height leave it there: the open height check fails, and the punch is drawn
        // standing on the sheet, not through it, as the checks' own approach and return do)
        let top = ctx.z_tdc.max(ctx.ztip(ctx.phi0()));
        let (phi, tip) = match stage {
            Stage::Handle | Stage::Gauge => (0.0, top),
            Stage::Approach => (0.0, lerp(top, ctx.ztip(ctx.phi0()), f)),
            Stage::Bend => {
                let phi = ctx.phimax * f;
                (phi, ctx.ztip(phi))
            }
            Stage::Springback => {
                let phi = ctx.phimax + (ctx.theta - ctx.phimax) * smooth(f);
                (phi, ctx.ztip(phi).max(ctx.ztip(ctx.phimax)))
            }
            Stage::Return => {
                let h0 = ctx.ztip(ctx.theta);
                (ctx.theta, lerp(h0, top, f))
            }
        };
        Pose {
            stage,
            f,
            phi,
            tip,
            gauge,
            handle: if stage == Stage::Handle { f } else { 1.0 },
        }
    }

    fn piece_meshes(&mut self, k: usize, ctx: &StepCtx) -> &Vec<Option<PieceMesh>> {
        if self.kept.as_ref().is_none_or(|x| x.0 != k) {
            let h = self.chord;
            let meshes = ctx
                .pieces
                .iter()
                .map(|pc| {
                    let zone = !matches!(pc.kind, Kind::Wall { .. });
                    let (pts, tris, edges) = tessellated(
                        pc.region(),
                        if zone { h.min(2.0) } else { h * 8.0 },
                        zone.then_some(h.min(2.0)),
                    )?;
                    Some(PieceMesh {
                        pts,
                        tris,
                        edges,
                        strip: pc.strip,
                    })
                })
                .collect();
            self.kept = Some((k, meshes));
        }
        &self.kept.as_ref().unwrap().1
    }

    /// The part's rigid motion from step `k - 1`'s last pose to step `k`'s first: a wall's three
    /// material points placed by both (a wall's own coordinates are the same in every step).
    fn handover(&self, k: usize) -> Option<Rigid3> {
        if k == 0 {
            return None;
        }
        let (a, b) = (self.ctx(k - 1), self.ctx(k));
        let ta = a.theta;
        for pb in &b.pieces {
            let Kind::Wall {
                w,
                to_param: tb,
                region,
                ..
            } = &pb.kind
            else {
                continue;
            };
            let Some(pa) = a
                .pieces
                .iter()
                .find(|p| matches!(&p.kind, Kind::Wall { w: x, .. } if x == w))
            else {
                continue;
            };
            let Kind::Wall { to_param: tpa, .. } = &pa.kind else {
                continue;
            };
            let (lo, hi) = region.bbox();
            let own = |p: Point2| tb.inverse().apply(p);
            let ps = [lo, Point2::new(hi.x, lo.y), Point2::new(lo.x, hi.y)].map(own);
            let qa = ps.map(|p| part_point(&a, pa, tpa.apply(p), 0.0, ta));
            let qb = ps.map(|p| part_point(&b, pb, tb.apply(p), 0.0, 0.0));
            let fa = Rigid3::on(qa[0], qa[1], qa[2]);
            let fb = Rigid3::on(qb[0], qb[1], qb[2]);
            // from where step k places the part back to where step k - 1 left it
            return Some(fa.then(&fb.inverse()));
        }
        None
    }

    /// The part's points in step `k`'s first pose, per piece (both skins, boundary and interior
    /// samples), and how far the true surface may lie outside the samples' boxes.
    fn part_samples(&mut self, k: usize) -> (Vec<Vec<[f64; 3]>>, f64) {
        let ctx = self.ctx(k);
        let pcs: Vec<Piece> = ctx.pieces.clone();
        let tess: Vec<Option<Vec<Point2>>> = self
            .piece_meshes(k, &ctx)
            .iter()
            .map(|m| m.as_ref().map(|m| m.pts.clone()))
            .collect();
        let mut out = Vec::new();
        for (pc, pts) in pcs.iter().zip(tess) {
            let Some(pts) = pts else { continue };
            let mut v = Vec::with_capacity(2 * pts.len());
            for w in [0.0, -ctx.t] {
                for p in &pts {
                    v.push(part_point(&ctx, pc, *p, w, 0.0));
                }
            }
            out.push(v);
        }
        // a zone sampled every 2 mm of its neutral length turns 2 / rn between samples: its
        // skins (radius at most r + t) sag at most (r + t) (2 / rn)^2 / 8 off the chord; a wall's
        // outline arcs every 0.12 rad sag at most r (1 - cos 0.06) < 0.0019 r; lines are exact
        let zone = self
            .m
            .bends
            .iter()
            .map(|b| (b.r + self.m.t) / (2.0 * b.rn * b.rn))
            .fold(0.0f64, f64::max);
        let wall = self
            .m
            .pattern
            .iter()
            .flatten()
            .map(|s| match s {
                Seg::Arc { r, .. } => 0.0019 * r,
                Seg::Line { .. } => 0.0,
            })
            .fold(0.0f64, f64::max);
        (out, zone.max(wall) + 1e-6)
    }

    /// The operator's path from step `k - 1`'s last pose to step `k`'s first: lift clear of the
    /// die, pull out towards the operator until the part's sphere is in front of every body,
    /// turn there, push in, set down.
    pub fn handling_path(&mut self, k: usize) -> Option<HandlingPath> {
        if let Some((kk, hp)) = &self.kept_path
            && *kk == k
        {
            return hp.clone();
        }
        let hp = self.make_path(k);
        self.kept_path = Some((k, hp.clone()));
        hp
    }

    fn make_path(&mut self, k: usize) -> Option<HandlingPath> {
        let h = self.handover(k)?;
        let (pieces, sag) = self.part_samples(k);
        let all: Vec<[f64; 3]> = pieces.iter().flatten().copied().collect();
        let (lo, hi) = bbox(&all)?;
        let c = [0, 1, 2].map(|i| 0.5 * (lo[i] + hi[i]));
        let radius = all
            .iter()
            .map(|p| dot(sub(*p, c), sub(*p, c)).sqrt())
            .fold(0.0, f64::max)
            + sag;
        // the bodies' front: the least x of any body (the beams' fronts)
        let front = self
            .ctx(k)
            .handle_targets(None, &[], &[])
            .iter()
            .map(|t| t.bbox().0[0])
            .fold(f64::INFINITY, f64::min);
        let clear = 2.0 * self.tl.clearance;
        let c_old = h.apply(c);
        let out = |cx: f64| (cx + radius - front + clear).max(0.0);
        // lifted until the lowest point (a bend sitting down in the V) is above the die's top
        let low_new = lo[2] - sag;
        let low_old = all.iter().map(|p| h.apply(*p)[2]).fold(f64::INFINITY, f64::min) - sag;
        let delta = (clear + 1.0).max(clear - low_new.min(low_old));
        Some(HandlingPath {
            h,
            c,
            delta,
            d_old: out(c_old[0]),
            d_new: out(c[0]),
            radius,
            sag,
        })
    }

    /// The bodies the move into step `k` is held clear of (`StepCtx::handle_targets`): step
    /// `k`'s press, the ram at top dead centre, through the whole move (the bed and plain dies
    /// left out of the lift and the set-down); the backgauge at the last step's stop while the
    /// part is lifted and pulled out, at this step's while it is pushed in and set down (it moves
    /// while the operator turns the part: both, then).
    pub fn handle_targets(&self, k: usize) -> Vec<HandleTarget> {
        let (a, b) = (self.ctx(k - 1), self.ctx(k));
        let mut targets = b.handle_targets(None, &[0, 1, 2, 3, 4], &[0, 4]);
        if let (_, Some(g)) = a.gauge() {
            targets.extend(a.handle_targets(Some((&g, Phase::Return)), &[0, 1, 2], &[0]));
        }
        if let (_, Some(g)) = b.gauge() {
            targets.extend(b.handle_targets(Some((&g, Phase::Static)), &[2, 3, 4], &[4]));
        }
        targets
    }

    /// The handling into step `k` checked against the press's bodies: proven clear, proven to
    /// meet a body (a real point at a real instant), within the resolution (Near), or undecided.
    /// Each body is first held to the part's envelopes (every piece's box swept through the lift
    /// and the pulls, exact for a translation; the part's sphere through the turn): a body they
    /// clear by the clearance is proven clear at a fraction of the cost; the part itself is
    /// carried through the move (`HandlingPath`, enclosed on intervals) by the same branch and
    /// bound the bends' checks use only against the bodies they do not clear.
    pub fn handling_check(&mut self, k: usize) -> Option<Check> {
        let hp = self.handling_path(k)?;
        let targets = self.handle_targets(k);
        let gaps = self.envelope(k, &hp, &targets);
        let lim = self.tl.clearance;
        let mut cleared: (f64, String) = (f64::INFINITY, String::new());
        let mut open = Vec::new();
        for (t, g) in targets.iter().zip(&gaps) {
            if *g >= lim {
                if *g < cleared.0 {
                    cleared = (*g, t.name().to_string());
                }
            } else {
                open.push(t.only(t.spans().to_vec()));
            }
        }
        let mut c = if open.is_empty() {
            let mut c = Check::rule(Rule::Handling, cleared.0.min(1e9), lim, false);
            c.bound = Some(cleared.0.min(1e9));
            c.certified = true;
            c.pass = true;
            c.severity = Severity::Warning;
            c
        } else {
            self.ctx(k).handling(&hp, &open)
        };
        // a pass: the nearest of the bodies, the searched ones and the envelopes' both
        if c.pass && cleared.0 < c.value {
            c.value = cleared.0;
            c.bound = Some(c.bound.unwrap_or(cleared.0).min(cleared.0));
            c.tool = Some(cleared.1);
        } else if c.pass && c.tool.is_none() {
            c.tool = Some(cleared.1);
        }
        Some(c)
    }

    /// Each target's least gap to the part's envelopes through the spans it stands in: at least
    /// the clearance proves it clear of the move (the true surface lies within the grown boxes
    /// and the sphere); under it says nothing (the envelopes, not the part).
    fn envelope(&mut self, k: usize, hp: &HandlingPath, targets: &[HandleTarget]) -> Vec<f64> {
        let (pieces, _) = self.part_samples(k);
        let g = hp.sag;
        let grow =
            |(lo, hi): ([f64; 3], [f64; 3])| ([0, 1, 2].map(|i| lo[i] - g), [0, 1, 2].map(|i| hi[i] + g));
        // each piece at rest, in the old pose (lift, pull) and the new (push, set down)
        let rest: Vec<[([f64; 3], [f64; 3]); 2]> = pieces
            .iter()
            .filter_map(|pts| {
                let old: Vec<[f64; 3]> = pts.iter().map(|p| hp.h.apply(*p)).collect();
                Some([grow(bbox(&old)?), grow(bbox(pts)?)])
            })
            .collect();
        let lift = |r: &([f64; 3], [f64; 3])| (r.0, [r.1[0], r.1[1], r.1[2] + hp.delta]);
        let pull = |r: &([f64; 3], [f64; 3]), d: f64| {
            (
                [r.0[0] - d, r.0[1], r.0[2] + hp.delta],
                [r.1[0], r.1[1], r.1[2] + hp.delta],
            )
        };
        // the turn: the sphere's centre moves from the old pulled-out place to the new one
        let ca = add(hp.h.apply(hp.c), [-hp.d_old, 0.0, hp.delta]);
        let cb = add(hp.c, [-hp.d_new, 0.0, hp.delta]);
        let r = hp.radius;
        let turn = (
            [0, 1, 2].map(|i| ca[i].min(cb[i]) - r),
            [0, 1, 2].map(|i| ca[i].max(cb[i]) + r),
        );
        targets
            .iter()
            .map(|t| {
                let body = t.bbox();
                let mut gap = f64::INFINITY;
                for &s in t.spans() {
                    let g = match s {
                        0 => rest
                            .iter()
                            .map(|r| box_gap(lift(&r[0]), body))
                            .fold(f64::INFINITY, f64::min),
                        1 => rest
                            .iter()
                            .map(|r| box_gap(pull(&r[0], hp.d_old), body))
                            .fold(f64::INFINITY, f64::min),
                        2 => box_gap(turn, body),
                        3 => rest
                            .iter()
                            .map(|r| box_gap(pull(&r[1], hp.d_new), body))
                            .fold(f64::INFINITY, f64::min),
                        _ => rest
                            .iter()
                            .map(|r| box_gap(lift(&r[1]), body))
                            .fold(f64::INFINITY, f64::min),
                    };
                    gap = gap.min(g);
                }
                gap
            })
            .collect()
    }

    /// Every body at instant `t` of step `k`.
    pub fn bodies(&mut self, k: usize, t: f64) -> Vec<Body> {
        self.bodies_of(k, t, true)
    }

    /// Where step `k`'s contacts are at the instant `t`: for each contact, the part's faces in
    /// its cells whose motion span holds the instant's (the turn in the bend, the ram's height in
    /// approach and return; a static contact's always), posed as `bodies` poses the part and lifted
    /// `CONTACT_LIFT` off the skin so they draw over it. A triangle (or a side's quad) is kept when
    /// its (u, v) box meets a cell's: the drawn region is never smaller than the cells. A
    /// self-collision's other faces are the triangles of those faces inside the contact's box.
    /// One body a contact, `contact:<i>`, its `hit` a proven depth; none for a contact with
    /// nothing in span at `t`.
    pub fn contact_bodies(&mut self, k: usize, t: f64, contacts: &[Contact]) -> Vec<Body> {
        let ctx = self.ctx(k);
        let pose = self.pose(k, t);
        let path = if pose.stage == Stage::Handle {
            self.handling_path(k)
        } else {
            None
        };
        let moved = |p: [f64; 3]| match &path {
            Some(hp) => hp.place(p, pose.handle),
            None => p,
        };
        let phi = pose.phi;
        let ts = ctx.t;
        let pcs: Vec<Piece> = ctx.pieces.clone();
        let meshes: Vec<Option<(Tess, Strip)>> = self
            .piece_meshes(k, &ctx)
            .iter()
            .map(|m| {
                m.as_ref()
                    .map(|m| ((m.pts.clone(), m.tris.clone(), m.edges.clone()), m.strip))
            })
            .collect();
        let value = |ph: Phase| -> Option<f64> {
            match (ph, pose.stage) {
                (Phase::Static, _) => None,
                (Phase::Bend, Stage::Bend | Stage::Springback) => Some(phi),
                (Phase::Approach, Stage::Approach) | (Phase::Return, Stage::Return) => Some(pose.tip),
                _ => Some(f64::NAN),
            }
        };
        let mut out = Vec::new();
        for (i, c) in contacts.iter().enumerate() {
            // a static (and a self) contact holds through the step; another only in its stretch
            let s = if c.rule == Rule::SelfCollision {
                None
            } else {
                value(c.phase)
            };
            let in_span =
                |cell: &[Interval; 4]| s.is_none_or(|s| cell[3].lo - 1e-9 <= s && s <= cell[3].hi + 1e-9);
            let mut mesh = Mesh::default();
            for f in &c.faces {
                let cells: Vec<&[Interval; 4]> = f.cells.iter().filter(|x| in_span(x)).collect();
                if cells.is_empty() {
                    continue;
                }
                // the cells' footprint: their (u, v) rectangles, merged where they touch
                let rects = footprint(&cells);
                for (pc, pm) in pcs.iter().zip(&meshes) {
                    let Some(((pts, _, edges), strip)) = pm else {
                        continue;
                    };
                    if *strip != f.part.strip {
                        continue;
                    }
                    let at = |p: Point2, w: f64| moved(part_point(&ctx, pc, p, w, phi));
                    let region = pc.region();
                    let (plo, phi_) = pts.iter().fold(
                        (
                            Point2::new(f64::INFINITY, f64::INFINITY),
                            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
                        ),
                        |(lo, hi), p| {
                            (
                                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
                            )
                        },
                    );
                    // a zone curves: its pieces short enough to follow it
                    let step = if matches!(pc.kind, Kind::Wall { .. }) {
                        f64::INFINITY
                    } else {
                        CONTACT_STEP
                    };
                    // the region's material, drawn as its shell: both skins over the footprint
                    // and the sides it reaches (a cell lies in the sheet, at any depth; its face
                    // names only where the tool is nearest)
                    for &(lo, hi) in &rects {
                        let lo = Point2::new(lo.x.max(plo.x), lo.y.max(plo.y));
                        let hi = Point2::new(hi.x.min(phi_.x), hi.y.min(phi_.y));
                        if lo.x >= hi.x || lo.y >= hi.y {
                            continue;
                        }
                        let nu = ((hi.x - lo.x) / step).ceil().max(1.0) as usize;
                        let nv = ((hi.y - lo.y) / step).ceil().max(1.0) as usize;
                        let g = |i: usize, j: usize| {
                            Point2::new(
                                lo.x + (hi.x - lo.x) * i as f64 / nu as f64,
                                lo.y + (hi.y - lo.y) * j as f64 / nv as f64,
                            )
                        };
                        for i in 0..nu {
                            for j in 0..nv {
                                let q = [g(i, j), g(i + 1, j), g(i + 1, j + 1), g(i, j + 1)];
                                let mid = Point2::new(0.5 * (q[0].x + q[2].x), 0.5 * (q[0].y + q[2].y));
                                // over a hole or past the outline: nothing to draw
                                if !q.iter().chain([&mid]).any(|p| region.contains(*p)) {
                                    continue;
                                }
                                for w in [CONTACT_LIFT, -ts - CONTACT_LIFT] {
                                    let [a, b, c2, d] = q.map(|p| at(p, w));
                                    // both windings: seen from either side
                                    mesh.push_quad(a, b, c2, d);
                                    mesh.push_quad(a, d, c2, b);
                                }
                            }
                        }
                    }
                    for &(ea, eb) in edges {
                        let (pa, pb) = (pts[ea as usize], pts[eb as usize]);
                        // out of the region by the lift
                        let d = pb - pa;
                        let len = d.length().max(1e-12);
                        let nl = Point2::new(-d.y / len, d.x / len);
                        let mid = Point2::new(0.5 * (pa.x + pb.x), 0.5 * (pa.y + pb.y));
                        let inside_left =
                            region.contains(Point2::new(mid.x + nl.x * 1e-3, mid.y + nl.y * 1e-3));
                        let o = if inside_left { -CONTACT_LIFT } else { CONTACT_LIFT };
                        let off = |p: Point2| Point2::new(p.x + nl.x * o, p.y + nl.y * o);
                        for &(lo, hi) in &rects {
                            // the edge's stretch inside the rectangle
                            let Some((t0, t1)) = clip_segment(pa, pb, lo, hi) else {
                                continue;
                            };
                            let qa = off(Point2::new(pa.x + d.x * t0, pa.y + d.y * t0));
                            let qb = off(Point2::new(pa.x + d.x * t1, pa.y + d.y * t1));
                            mesh.push_quad(at(qa, 0.0), at(qb, 0.0), at(qb, -ts), at(qa, -ts));
                            mesh.push_quad(at(qa, 0.0), at(qa, -ts), at(qb, -ts), at(qb, 0.0));
                        }
                    }
                }
            }
            // the part it meets: those faces' triangles inside the contact's box
            if !c.others.is_empty() && !mesh.tri.is_empty() {
                let (blo, bhi) = c.bbox;
                let inside = |p: [f64; 3]| (0..3).all(|j| blo[j] - 1e-6 <= p[j] && p[j] <= bhi[j] + 1e-6);
                for o in &c.others {
                    let w = match o.surface {
                        Surface::Top => CONTACT_LIFT,
                        Surface::Bottom => -ts - CONTACT_LIFT,
                        Surface::Side => -0.5 * ts,
                    };
                    for (pc, pm) in pcs.iter().zip(&meshes) {
                        let Some(((pts, tris, _), strip)) = pm else {
                            continue;
                        };
                        if *strip != o.strip {
                            continue;
                        }
                        for tr in tris {
                            let q = tr.map(|v| moved(part_point(&ctx, pc, pts[v as usize], w, phi)));
                            if q.iter().any(|p| inside(*p)) {
                                mesh.push_tri(q[0], q[1], q[2]);
                                mesh.push_tri(q[0], q[2], q[1]);
                            }
                        }
                    }
                }
            }
            if mesh.tri.is_empty() {
                continue;
            }
            out.push(Body {
                name: format!("contact:{i}"),
                look: Look::Witness,
                mesh,
                hit: c.depth.is_some(),
                strip: c.faces.first().map(|f| f.part.strip),
            });
        }
        out
    }

    /// The bodies, the part's pieces among them only with `part`.
    fn bodies_of(&mut self, k: usize, t: f64, part: bool) -> Vec<Body> {
        let ctx = self.ctx(k);
        let st: &Step = &self.plan.steps[k];
        let pose = self.pose(k, t);
        let failing: Vec<String> = st.failures().iter().filter_map(|c| c.tool.clone()).collect();
        let hit_parts: Vec<Strip> = st
            .failures()
            .iter()
            .flat_map(|c| [c.part.map(|p| p.strip), c.other.map(|p| p.strip)])
            .flatten()
            .collect();
        let mut out = Vec::new();
        // the part: during handling it follows the operator's path (`HandlingPath`), the same
        // one the handling check proves clear
        let path = if part && pose.stage == Stage::Handle {
            self.handling_path(k)
        } else {
            None
        };
        let phi = pose.phi;
        let t_sheet = ctx.t;
        let moved = |p: [f64; 3]| -> [f64; 3] {
            match &path {
                Some(hp) => hp.place(p, pose.handle),
                None => p,
            }
        };
        let pcs: Vec<Piece> = ctx.pieces.clone();
        let meshes: Vec<Option<(Tess, Strip)>> = self
            .piece_meshes(k, &ctx)
            .iter()
            .map(|m| {
                m.as_ref()
                    .map(|m| ((m.pts.clone(), m.tris.clone(), m.edges.clone()), m.strip))
            })
            .collect();
        for (i, (pc, pm)) in pcs.iter().zip(meshes).enumerate().filter(|_| part) {
            let Some(((pts, tris, edges), strip)) = pm else {
                continue;
            };
            let pphi = phi;
            let top: Vec<[f64; 3]> = pts
                .iter()
                .map(|p| moved(part_point(&ctx, pc, *p, 0.0, pphi)))
                .collect();
            let bot: Vec<[f64; 3]> = pts
                .iter()
                .map(|p| moved(part_point(&ctx, pc, *p, -t_sheet, pphi)))
                .collect();
            let mut mesh = Mesh::default();
            // which way the top skin faces: from the bottom skin to the top
            let up = tris.first().map(|tr| {
                let n = normal(top[tr[0] as usize], top[tr[1] as usize], top[tr[2] as usize]);
                dot(n, sub(top[tr[0] as usize], bot[tr[0] as usize])) >= 0.0
            });
            let up = up.unwrap_or(true);
            // smooth skins: vertices shared, normals averaged
            for (skin, outward) in [(&top, up), (&bot, !up)] {
                let base = mesh.pos.len() as u32;
                let mut nrm = vec![[0.0f64; 3]; skin.len()];
                for tr in &tris {
                    let [a, b, c] = tr.map(|v| skin[v as usize]);
                    let n = cross(sub(b, a), sub(c, a));
                    for v in tr {
                        let e = &mut nrm[*v as usize];
                        e[0] += n[0];
                        e[1] += n[1];
                        e[2] += n[2];
                    }
                }
                for (p, n) in skin.iter().zip(&nrm) {
                    let n = unit(*n);
                    mesh.pos.push(f3(*p));
                    mesh.nrm.push(f3(if outward { n } else { [-n[0], -n[1], -n[2]] }));
                }
                for tr in &tris {
                    mesh.tri.push(if outward {
                        [base + tr[0], base + tr[1], base + tr[2]]
                    } else {
                        [base + tr[0], base + tr[2], base + tr[1]]
                    });
                }
            }
            // sides: each boundary edge's quad, turned out of the region
            let region = pc.region();
            for (a, b) in edges {
                let (pa, pb) = (pts[a as usize], pts[b as usize]);
                let d = pb - pa;
                let mid = Point2::new(0.5 * (pa.x + pb.x), 0.5 * (pa.y + pb.y));
                let len = d.length().max(1e-12);
                let left = Point2::new(mid.x - d.y / len * 1e-3, mid.y + d.x / len * 1e-3);
                let region_left = region.contains(left);
                let (ta, tb, ba, bb) = (top[a as usize], top[b as usize], bot[a as usize], bot[b as usize]);
                // with the region on the left of a -> b and the top skin up, a, b, b', a' winds out
                if region_left == up {
                    mesh.push_quad(ba, bb, tb, ta);
                } else {
                    mesh.push_quad(ta, tb, bb, ba);
                }
            }
            out.push(Body {
                name: format!("part:{i}"),
                look: if strip == Strip::Bend(ctx.bj.joint) {
                    Look::BendZone
                } else {
                    Look::Sheet
                },
                mesh,
                hit: hit_parts.contains(&strip),
                strip: Some(strip),
            });
        }
        // tools
        let (plen, dlen) = ctx.tool_lengths();
        let oy = ctx.oy;
        let half = |l: Option<f64>| 0.5 * l.unwrap_or(ctx.bj.len);
        let hit = |name: &str| {
            failing
                .iter()
                .any(|f| f == name || name.starts_with(f.as_str()) || f.starts_with(name))
        };
        let tip = pose.tip;
        for (name, sec) in [
            (format!("punch:{}/tip", ctx.punch.id), ctx.punch.tip_section()),
            (format!("punch:{}/body", ctx.punch.id), drawn_body(ctx.punch)),
        ] {
            let h = half(plen);
            if let Some(mesh) = stack(&sec, 0.0, tip, oy - h, oy + h, &ctx.punch.segments) {
                out.push(Body {
                    hit: hit(&name),
                    name,
                    look: Look::Tool,
                    mesh,
                    strip: None,
                });
            }
        }
        let dname = format!("die:{}", ctx.die.id);
        let h = half(dlen);
        if let Some(mesh) = stack(&ctx.die.section(), 0.0, 0.0, oy - h, oy + h, &ctx.die.segments) {
            out.push(Body {
                hit: hit(&dname),
                name: dname,
                look: Look::Tool,
                mesh,
                strip: None,
            });
        }
        for (i, s) in self.tl.stations.iter().enumerate() {
            if Some(i) == ctx.station {
                continue;
            }
            let (p, d) = (&self.tl.punches[s.punch], &self.tl.dies[s.die]);
            let lift = ctx.punch.height - p.height;
            for (part, sec) in [("tip", p.tip_section()), ("body", drawn_body(p))] {
                let name = format!("station{}:punch:{}/{part}", i + 1, p.id);
                if let Some(mesh) = stack(
                    &sec,
                    0.0,
                    tip + lift,
                    s.y - 0.5 * s.punch_length,
                    s.y + 0.5 * s.punch_length,
                    &p.segments,
                ) {
                    out.push(Body {
                        hit: hit(&name),
                        name,
                        look: Look::Tool,
                        mesh,
                        strip: None,
                    });
                }
            }
            let name = format!("station{}:die:{}", i + 1, d.id);
            let drop = ctx.die.height - d.height;
            if let Some(mesh) = stack(
                &d.section(),
                0.0,
                drop,
                s.y - 0.5 * s.die_length,
                s.y + 0.5 * s.die_length,
                &d.segments,
            ) {
                out.push(Body {
                    hit: hit(&name),
                    name,
                    look: Look::Tool,
                    mesh,
                    strip: None,
                });
            }
        }
        // the press
        let pr = &self.tl.press;
        let bed = 0.5 * pr.bed_length;
        let ph = ctx.punch.height;
        let dh = ctx.die.height;
        let beam_h = 700.0;
        let mut ram = Mesh::default();
        ram.push_box(
            [-pr.ram.front, -bed, tip + ph],
            [pr.ram.back, bed, tip + ph + beam_h],
        );
        out.push(Body {
            hit: hit("ram"),
            name: "ram".into(),
            look: Look::Frame,
            mesh: ram,
            strip: None,
        });
        let floor_z = -dh - self.table_height;
        let mut bedm = Mesh::default();
        bedm.push_box([-pr.bed.front, -bed, floor_z], [pr.bed.back, bed, -dh]);
        out.push(Body {
            hit: hit("bed"),
            name: "bed".into(),
            look: Look::Frame,
            mesh: bedm,
            strip: None,
        });
        let ram_top_open = ctx.z_tdc.max(ctx.ztip(ctx.phi0())) + ph + beam_h;
        if let Some(f) = pr.frame {
            let g = 0.5 * pr.housing_gap;
            let front = -pr.bed.front.max(pr.ram.front) - 150.0;
            for (side, y0, y1) in [("left", -g - f.thickness, -g), ("right", g, g + f.thickness)] {
                let name = format!("frame:{side}");
                let mut mesh = Mesh::default();
                let top = ram_top_open + 300.0;
                // behind the throat: the collision body; before it, below the bed and above
                // the ram: drawn only
                mesh.push_box([f.throat, y0, floor_z], [f.throat + f.depth, y1, top]);
                mesh.push_box([front, y0, floor_z], [f.throat, y1, -dh - 100.0]);
                mesh.push_box([front, y0, ram_top_open + 50.0], [f.throat, y1, top]);
                out.push(Body {
                    hit: hit(&name),
                    name,
                    look: Look::Frame,
                    mesh,
                    strip: None,
                });
            }
        }
        if let Some((gx, gr)) = pose.gauge {
            let bg = &pr.backgauge;
            let gap = match pose.stage {
                Stage::Handle | Stage::Gauge | Stage::Approach => 2.0 * self.tl.clearance,
                _ => bg.retract.max(2.0 * self.tl.clearance),
            };
            let fingers: Vec<f64> = st.gauge.as_ref().map(|g| g.fingers.clone()).unwrap_or_default();
            for (i, y) in fingers.iter().enumerate() {
                let name = format!("backgauge:finger{}", i + 1);
                let mut mesh = Mesh::default();
                let (x0, x1) = (gx + gap, gx + gap + bg.finger.depth);
                let (y0, y1) = (oy + y - 0.5 * bg.finger.width, oy + y + 0.5 * bg.finger.width);
                mesh.push_box([x0, y0, gr], [x1, y1, gr + bg.finger.height]);
                // the finger's carrier, back to the crossbar
                mesh.push_box(
                    [x1, y0 + 0.2 * bg.finger.width, gr + 0.3 * bg.finger.height],
                    [x1 + 80.0, y1 - 0.2 * bg.finger.width, gr + bg.finger.height],
                );
                out.push(Body {
                    hit: hit(&name),
                    name,
                    look: Look::Gauge,
                    mesh,
                    strip: None,
                });
            }
            let mut bar = Mesh::default();
            let xb = gx + gap + bg.finger.depth + 80.0;
            let span = bg.z.map_or(bed * 0.8, |z| z.1.abs().max(z.0.abs()));
            bar.push_box([xb, -span, gr - 40.0], [xb + 80.0, span, gr + bg.finger.height]);
            out.push(Body {
                hit: false,
                name: "backgauge:crossbar".into(),
                look: Look::Gauge,
                mesh: bar,
                strip: None,
            });
        }
        let mut floor = Mesh::default();
        let ext = bed + 2500.0;
        floor.push_box([-3000.0, -ext, floor_z - 20.0], [3000.0, ext, floor_z]);
        out.push(Body {
            hit: false,
            name: "floor".into(),
            look: Look::Floor,
            mesh: floor,
            strip: None,
        });
        // collision witnesses: a small box at each failing check's place, during its phase
        for c in st.failures() {
            if let (Some(at), Some(ph)) = (c.at, c.phase) {
                let show = matches!(
                    (ph, pose.stage),
                    (Phase::Static, Stage::Gauge)
                        | (Phase::Approach, Stage::Approach)
                        | (Phase::Bend, Stage::Bend)
                        | (Phase::Return, Stage::Return)
                );
                if show {
                    let mut mesh = Mesh::default();
                    let s = 2.0;
                    let at = [at[0], at[1] + oy, at[2]];
                    mesh.push_box(
                        [at[0] - s, at[1] - s, at[2] - s],
                        [at[0] + s, at[1] + s, at[2] + s],
                    );
                    out.push(Body {
                        hit: true,
                        name: format!("witness:{}", c.rule.word()),
                        look: Look::Witness,
                        mesh,
                        strip: None,
                    });
                }
            }
        }
        out
    }
}

/// A tool section (x, z) extruded along y from y0 to y1, shifted by (dx, dz).
/// The section a punch's body is drawn with: at its published width where the checked one is
/// an envelope wider than it (`drawn_width`); the checks never read this.
pub fn drawn_body(p: &Punch) -> Region {
    match p.drawn_width {
        Some(w) if w < p.width => Punch {
            width: w,
            ..p.clone()
        }
        .body_section(),
        _ => p.body_section(),
    }
}

/// A gap drawn between a stack's segments (mm): where one piece ends and the next begins.
pub const STACK_GAP: f64 = 0.8;

/// The pieces a length from `y0` to `y1` is made of: the published segment lengths that make it
/// up exactly (`cover`), laid end to end, longest first; one piece when none do.
pub fn stack_spans(segments: &[Segment], y0: f64, y1: f64) -> Vec<(f64, f64)> {
    let len = y1 - y0;
    let Some((total, pieces)) = crate::tooling::cover(segments, len - 1e-6) else {
        return vec![(y0, y1)];
    };
    if (total - len).abs() > 0.05 || pieces.iter().map(|p| p.1).sum::<u32>() < 2 {
        return vec![(y0, y1)];
    }
    let mut out = Vec::new();
    let mut y = y0;
    for (l, n) in pieces {
        for _ in 0..n {
            out.push((y, (y + l).min(y1)));
            y += l;
        }
    }
    out
}

/// A section extruded as a stack of segments, a small gap between each.
pub fn stack(sec: &Region, dx: f64, dz: f64, y0: f64, y1: f64, segments: &[Segment]) -> Option<Mesh> {
    let spans = stack_spans(segments, y0, y1);
    let g = if spans.len() > 1 { 0.5 * STACK_GAP } else { 0.0 };
    let mut out = Mesh::default();
    for (i, (a, b)) in spans.iter().enumerate() {
        let a = if i == 0 { *a } else { a + g };
        let b = if i + 1 == spans.len() { *b } else { b - g };
        out.append(&extrude(sec, dx, dz, a, b)?);
    }
    Some(out)
}

pub fn extrude(sec: &Region, dx: f64, dz: f64, y0: f64, y1: f64) -> Option<Mesh> {
    let (pts, tris, edges) = triangulate(sec, 20.0, None)?;
    let mut m = Mesh::default();
    let p3 = |p: Point2, y: f64| [p.x + dx, y, p.y + dz];
    // caps: the triangles are counter-clockwise in (x, z); seen from -y that is clockwise
    for tr in &tris {
        let [a, b, c] = tr.map(|v| pts[v as usize]);
        m.push_tri(p3(a, y0), p3(b, y0), p3(c, y0));
        m.push_tri(p3(a, y1), p3(c, y1), p3(b, y1));
    }
    // (x, z) counter-clockwise has normal -y under the right-hand rule with axes x, y, z: fix
    // by checking one triangle
    if let Some(&[a, b, c]) = m.tri.first().map(|t| t.map(|v| m.pos[v as usize])).as_ref() {
        let n = cross(
            [(b[0] - a[0]) as f64, (b[1] - a[1]) as f64, (b[2] - a[2]) as f64],
            [(c[0] - a[0]) as f64, (c[1] - a[1]) as f64, (c[2] - a[2]) as f64],
        );
        if n[1] > 0.0 {
            // the y0 cap faces +y: every triangle so far turned
            for t in &mut m.tri {
                t.swap(1, 2);
            }
            for n in &mut m.nrm {
                *n = [-n[0], -n[1], -n[2]];
            }
        }
    }
    for (a, b) in edges {
        let (pa, pb) = (pts[a as usize], pts[b as usize]);
        let d = pb - pa;
        let len = d.length().max(1e-12);
        let mid = Point2::new(0.5 * (pa.x + pb.x), 0.5 * (pa.y + pb.y));
        let left = Point2::new(mid.x - d.y / len * 1e-3, mid.y + d.x / len * 1e-3);
        let (a0, b0, a1, b1) = (p3(pa, y0), p3(pb, y0), p3(pa, y1), p3(pb, y1));
        if sec.contains(left) {
            // region on the left of a -> b in (x, z): outward is to the right
            m.push_quad(a0, a1, b1, b0);
        } else {
            m.push_quad(a0, b0, b1, a1);
        }
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{Rule, Severity};
    use crate::{Options, library, plan};
    use forge_math::Frame as F3;
    use forge_ops::{ProfileLoop, Segment};
    use forge_sheet::{BendRule, FeatureId, Gauge, Sheet};
    use std::collections::HashMap;

    #[test]
    fn stacks_are_drawn_piece_by_piece() {
        use crate::tooling::Segment as Seg;
        let segs = [
            Seg {
                length: 100.0,
                count: 2,
            },
            Seg {
                length: 50.0,
                count: 1,
            },
        ];
        let s = stack_spans(&segs, -125.0, 125.0);
        assert_eq!(s.len(), 3);
        assert_eq!((s[0].0, s[2].1), (-125.0, 125.0));
        // a length no stack makes up is one piece
        assert_eq!(stack_spans(&segs, 0.0, 77.0), vec![(0.0, 77.0)]);
        // the punch's published width is what is drawn, the envelope what is checked
        let cat = crate::catalog::Catalog::builtin();
        let p = cat
            .punches
            .iter()
            .find_map(|r| r.punch.clone().ok().filter(|p| p.drawn_width.is_some()));
        let p = p.expect("a punch with a published body width");
        let (a, _) = extrude(&drawn_body(&p), 0.0, 0.0, 0.0, 1.0)
            .unwrap()
            .bounds()
            .unwrap();
        let (b, _) = extrude(&p.body_section(), 0.0, 0.0, 0.0, 1.0)
            .unwrap()
            .bounds()
            .unwrap();
        assert!(a[0] > b[0], "drawn {a:?} within checked {b:?}");
    }

    fn ell() -> Model {
        let pts = [(0.0, 0.0), (120.0, 0.0), (120.0, 60.0), (0.0, 60.0)];
        let lp = ProfileLoop::new(
            (0..4)
                .map(|i| {
                    let (a, b) = (pts[i], pts[(i + 1) % 4]);
                    (
                        Segment::Line {
                            a: Point2::new(a.0, a.1),
                            b: Point2::new(b.0, b.1),
                        },
                        i as u64,
                    )
                })
                .collect(),
        );
        let g = Gauge {
            thickness: 2.0,
            radius: 2.0,
            rule: BendRule::K(0.4),
        };
        let (mut s, base) = Sheet::face(FeatureId(1), g, F3::WORLD, &[lp]).unwrap();
        s.flange(base, 1, dmath::FRAC_PI_2, 30.0).unwrap();
        s.flange(base, 3, dmath::FRAC_PI_2, 30.0).unwrap();
        Model::new(&s).unwrap()
    }

    /// Welded by position: every edge is used twice, once each way (closed, consistently wound).
    fn closed(m: &Mesh) -> Result<(), String> {
        let key = |p: [f32; 3]| {
            [
                (p[0] * 1e3).round() as i64,
                (p[1] * 1e3).round() as i64,
                (p[2] * 1e3).round() as i64,
            ]
        };
        let mut ids: HashMap<[i64; 3], usize> = HashMap::new();
        let v: Vec<usize> = m
            .pos
            .iter()
            .map(|p| {
                let n = ids.len();
                *ids.entry(key(*p)).or_insert(n)
            })
            .collect();
        let mut e: HashMap<(usize, usize), i32> = HashMap::new();
        for t in &m.tri {
            let t = t.map(|i| v[i as usize]);
            if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
                continue;
            }
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *e.entry((a, b)).or_default() += 1;
            }
        }
        for (&(a, b), &n) in &e {
            let back = e.get(&(b, a)).copied().unwrap_or(0);
            if n != back {
                return Err(format!("edge {a}-{b} used {n} times, back {back}"));
            }
        }
        Ok(())
    }

    /// A narrow, tall channel on straight punches, its second bend closing the first flange on
    /// the punch: in the bend, at a turn inside a contact's cell, the contact is drawn, its hit
    /// its proven depth, and the cell's centre on the part's skin lies on the drawn region
    /// (within a triangle and the lift); while the operator handles the part, a bend's contact is
    /// not drawn.
    #[test]
    fn contacts_are_drawn_where_and_when_they_are() {
        let pts = [(0.0, 0.0), (30.0, 0.0), (30.0, 200.0), (0.0, 200.0)];
        let lp = ProfileLoop::new(
            (0..4)
                .map(|i| {
                    let (a, b) = (pts[i], pts[(i + 1) % 4]);
                    (
                        Segment::Line {
                            a: Point2::new(a.0, a.1),
                            b: Point2::new(b.0, b.1),
                        },
                        i as u64,
                    )
                })
                .collect(),
        );
        let (mut s, base) = Sheet::face(
            FeatureId(1),
            Gauge {
                thickness: 2.0,
                radius: 2.0,
                rule: BendRule::K(0.4),
            },
            F3::WORLD,
            &[lp],
        )
        .unwrap();
        s.flange(base, 1, dmath::PI / 2.0, 60.0).unwrap();
        s.flange(base, 3, dmath::PI / 2.0, 60.0).unwrap();
        let m = Model::new(&s).unwrap();
        let tl = library::std_eu();
        let straight: Vec<usize> = (0..tl.punches.len())
            .filter(|&k| tl.punches[k].kind == crate::tooling::PunchKind::Straight)
            .collect();
        let p = plan(
            &m,
            &tl,
            &Options {
                punches: Some(straight),
                sequence: Some(vec![0, 1]),
                ..Options::default()
            },
        );
        let k = 1;
        let st = &p.steps[k];
        let done = vec![st.bend != 0, st.bend != 1];
        let cs = crate::sim::contacts_of(&m, &tl, &done, st);
        let bends: Vec<usize> = (0..cs.len()).filter(|&i| cs[i].phase == Phase::Bend).collect();
        assert!(!bends.is_empty(), "{cs:?}");
        let mut sc = Scene::new(&m, &tl, &p);
        let ctx = sc.ctx(k);
        let (_, b0, b1) = STAGES.iter().copied().find(|x| x.0 == Stage::Bend).unwrap();
        let mut drawn = 0;
        for &i in &bends {
            let c = &cs[i];
            for f in &c.faces {
                for cell in f.cells.iter().take(3) {
                    let s = cell[3].mid();
                    let t = b0 + (b1 - b0) * s / ctx.phimax;
                    let bs = sc.contact_bodies(k, t, &cs);
                    let b = bs
                        .iter()
                        .find(|b| b.name == format!("contact:{i}"))
                        .expect("a contact in its stretch is drawn");
                    assert_eq!(b.hit, c.depth.is_some());
                    assert_eq!(b.look, Look::Witness);
                    let phi = sc.pose(k, t).phi;
                    let centre = Point2::new(cell[0].mid(), cell[1].mid());
                    for pc in ctx.pieces.iter().filter(|pc| pc.strip == f.part.strip) {
                        if !pc.region().contains(centre) {
                            continue;
                        }
                        let q = part_point(&ctx, pc, centre, cell[2].mid(), phi);
                        let near = b
                            .mesh
                            .tri
                            .iter()
                            .map(|tr| {
                                let [a, b2, c2] = tr.map(|v| b.mesh.pos[v as usize].map(f64::from));
                                tri_dist(q, a, b2, c2)
                            })
                            .fold(f64::INFINITY, f64::min);
                        // on a skin over it (half the sheet away at most, and the lift)
                        assert!(
                            near <= 0.5 * ctx.t + CONTACT_LIFT + 0.01,
                            "contact {i}: the cell's centre {near} from the drawn region"
                        );
                        drawn += 1;
                    }
                }
            }
        }
        assert!(drawn > 0);
        // a flange's contact is drawn where its cells are, not as the whole flange: in its box,
        // and its box shorter than the flange's by a millimetre or more on some axis
        let t = b0 + 0.5 * (b1 - b0);
        let parts = sc.bodies(k, t);
        let mut walls = 0;
        for b in sc.contact_bodies(k, t, &cs) {
            let strip = b.strip.unwrap();
            if !matches!(strip, Strip::Wall(_)) {
                continue;
            }
            let (lo, hi) = b.mesh.bounds().unwrap();
            let part = parts.iter().find(|p| p.strip == Some(strip)).unwrap();
            let (plo, phi) = part.mesh.bounds().unwrap();
            assert!(
                (0..3).any(|j| (phi[j] - plo[j]) - (hi[j] - lo[j]) > 1.0),
                "{}: {lo:?}..{hi:?}, the flange {plo:?}..{phi:?}",
                b.name
            );
            walls += 1;
        }
        assert!(walls > 0);
        // handling: no bend contact is drawn
        let bs = sc.contact_bodies(k, 0.05, &cs);
        for &i in &bends {
            assert!(bs.iter().all(|b| b.name != format!("contact:{i}")));
        }
        // nothing asked, nothing drawn
        assert!(sc.contact_bodies(k, 0.5, &[]).is_empty());
    }

    /// A point's distance to a triangle.
    fn tri_dist(p: [f64; 3], a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
        let n = cross(sub(b, a), sub(c, a));
        let nn = dot(n, n);
        let seg = |a: [f64; 3], b: [f64; 3]| {
            let d = sub(b, a);
            let t = (dot(sub(p, a), d) / dot(d, d).max(1e-300)).clamp(0.0, 1.0);
            let q = [a[0] + d[0] * t, a[1] + d[1] * t, a[2] + d[2] * t];
            dot(sub(p, q), sub(p, q)).sqrt()
        };
        let edges = seg(a, b).min(seg(b, c)).min(seg(c, a));
        if nn < 1e-300 {
            return edges;
        }
        // the foot on the plane, inside when on the inner side of all three edges
        let h = dot(sub(p, a), n) / nn;
        let f = [p[0] - n[0] * h, p[1] - n[1] * h, p[2] - n[2] * h];
        let inside = [(a, b), (b, c), (c, a)]
            .iter()
            .all(|(u, v)| dot(cross(sub(*v, *u), sub(f, *u)), n) >= 0.0);
        if inside { (h * h * nn).sqrt() } else { edges }
    }

    /// The footprint's union is the cells': every cell's rectangle lies in it, and no point
    /// outside every cell is in it; touching cells merge, the same cell is once.
    #[test]
    fn the_footprint_is_the_cells_union() {
        let c = |u0: f64, u1: f64, v0: f64, v1: f64, w0: f64| {
            [
                Interval::new(u0, u1),
                Interval::new(v0, v1),
                Interval::new(w0, w0 + 0.5),
                Interval::new(0.0, 1.0),
            ]
        };
        let cells = [
            c(0.0, 1.0, 0.0, 1.0, -1.0),
            c(0.0, 1.0, 0.0, 1.0, -0.5),
            c(1.0, 2.0, 0.0, 1.0, -1.0),
            c(0.0, 2.0, 1.0, 2.0, -1.0),
            c(5.0, 5.5, 0.0, 0.25, -1.0),
        ];
        let refs: Vec<&[Interval; 4]> = cells.iter().collect();
        let f = footprint(&refs);
        assert_eq!(f.len(), 2, "{f:?}");
        let inside = |p: Point2, r: &[(Point2, Point2)]| {
            r.iter()
                .any(|(lo, hi)| lo.x <= p.x && p.x <= hi.x && lo.y <= p.y && p.y <= hi.y)
        };
        let cell_rects: Vec<(Point2, Point2)> = cells
            .iter()
            .map(|x| (Point2::new(x[0].lo, x[1].lo), Point2::new(x[0].hi, x[1].hi)))
            .collect();
        for i in 0..=60 {
            for j in 0..=25 {
                let p = Point2::new(i as f64 * 0.1, j as f64 * 0.1);
                assert_eq!(inside(p, &f), inside(p, &cell_rects), "{p:?}");
            }
        }
        // a segment through a rectangle: the stretch inside
        let (t0, t1) = clip_segment(
            Point2::new(-1.0, 0.5),
            Point2::new(3.0, 0.5),
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 1.0),
        )
        .unwrap();
        assert!((t0 - 0.25).abs() < 1e-12 && (t1 - 0.75).abs() < 1e-12);
        assert!(
            clip_segment(
                Point2::new(-1.0, 2.0),
                Point2::new(3.0, 2.0),
                Point2::new(0.0, 0.0),
                Point2::new(2.0, 1.0)
            )
            .is_none()
        );
    }

    #[test]
    fn bodies_through_a_plan() {
        let m = ell();
        let tl = library::std_eu();
        let p = plan(&m, &tl, &Options::default());
        assert!(p.feasible, "{:?}", p.blocking);
        let mut sc = Scene::new(&m, &tl, &p);
        for k in 0..p.steps.len() {
            for t in [0.0, 0.1, 0.2, 0.3, 0.5, 0.7, 0.8, 0.9, 1.0] {
                let bs = sc.bodies(k, t);
                let parts: Vec<&Body> = bs.iter().filter(|b| b.look.is_sheet()).collect();
                assert!(!parts.is_empty(), "step {k} t {t}: no part");
                for b in &bs {
                    assert!(!b.mesh.tri.is_empty(), "{} empty", b.name);
                    if b.look == Look::Tool || b.name == "ram" {
                        closed(&b.mesh).unwrap_or_else(|e| panic!("{}: {e}", b.name));
                    }
                }
                for b in &parts {
                    closed(&b.mesh).unwrap_or_else(|e| panic!("step {k} t {t} {}: {e}", b.name));
                }
                // after handling, the part rests on the die (z >= 0 less a little) or above
                if t >= 0.2 {
                    for b in &parts {
                        let (lo, _) = b.mesh.bounds().unwrap();
                        assert!(lo[2] > -40.0, "step {k} t {t} {} at z {}", b.name, lo[2]);
                    }
                }
            }
        }
        // the handover joins: the part at the end of the handling of step 1 is where step 1
        // places it; at its start it is where step 0 left it
        if p.steps.len() > 1 {
            let end0 = sc.bodies(0, 1.0);
            let start1 = sc.bodies(1, 0.0);
            let bb = |bs: &[Body]| {
                bs.iter()
                    .filter(|b| b.look.is_sheet())
                    .fold((f32::MAX, f32::MIN), |acc, b| {
                        let (lo, hi) = b.mesh.bounds().unwrap();
                        (acc.0.min(lo[2]), acc.1.max(hi[2]))
                    })
            };
            let (a, b) = (bb(&end0), bb(&start1));
            assert!(
                (a.0 - b.0).abs() < 0.05 && (a.1 - b.1).abs() < 0.05,
                "{a:?} vs {b:?}"
            );
        }
    }

    #[test]
    fn handling_runs_from_pose_to_pose_and_is_checked() {
        let m = ell();
        let tl = library::std_eu();
        let p = plan(&m, &tl, &Options::default());
        assert!(p.steps.len() > 1);
        let mut sc = Scene::new(&m, &tl, &p);
        assert!(sc.handling_path(0).is_none());
        let hp = sc.handling_path(1).expect("a handover");
        // the path's ends are the two poses
        for q in [[0.0, 0.0, 0.0], [30.0, -20.0, 5.0], [-60.0, 40.0, 25.0]] {
            let a = hp.place(q, 0.0);
            let b = hp.h.apply(q);
            let e = hp.place(q, 1.0);
            for i in 0..3 {
                assert!((a[i] - b[i]).abs() < 1e-9 && (e[i] - q[i]).abs() < 1e-9);
            }
        }
        // at the turn the part is out in front of every body
        let c = sc.handling_check(1).expect("checked");
        assert_eq!(c.rule, Rule::Handling);
        assert_eq!(c.severity, Severity::Warning);
        assert!(c.value.is_finite());
        eprintln!(
            "handling: {} vs {} pass {} {:?}",
            c.value, c.limit, c.pass, c.tool
        );
    }

    /// The move's enclosure holds every point of the part at every instant of it: boxes of
    /// points over intervals of f within each span, against the move at sampled points of each.
    #[test]
    fn the_handling_enclosure_holds_the_move() {
        let m = ell();
        let tl = library::std_eu();
        let p = plan(&m, &tl, &Options::default());
        let mut sc = Scene::new(&m, &tl, &p);
        let hp = sc.handling_path(1).expect("a handover");
        let mut seed = 7u64;
        let mut rnd = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        for w in KNOTS.windows(2) {
            for _ in 0..200 {
                let (a, b) = (w[0] + (w[1] - w[0]) * rnd(), w[0] + (w[1] - w[0]) * rnd());
                let f = Interval::new(a.min(b), a.max(b));
                let lo = [0, 1, 2].map(|_| -150.0 + 300.0 * rnd());
                let wd = [0, 1, 2].map(|_| 40.0 * rnd());
                let pi = [0, 1, 2].map(|i| Interval::new(lo[i], lo[i] + wd[i]));
                let e = hp.enclose(pi, f);
                for _ in 0..20 {
                    let q = [0, 1, 2].map(|i| lo[i] + wd[i] * rnd());
                    let ff = f.lo + f.width() * rnd();
                    let x = hp.place(q, ff);
                    for i in 0..3 {
                        assert!(
                            e[i].lo <= x[i] && x[i] <= e[i].hi,
                            "span {w:?} f {ff} axis {i}: {} not in {:?}",
                            x[i],
                            e[i]
                        );
                    }
                }
            }
        }
    }

    /// The real handling check against a brute-force sample: every sampled point of the part,
    /// at sampled instants of every span a body stands through, is at least the proven bound
    /// from it (a pass), or the check names a proven hit, Near or undecided.
    #[test]
    fn handling_bound_holds_on_a_sample() {
        let m = ell();
        let tl = library::std_eu();
        let p = plan(&m, &tl, &Options::default());
        let mut sc = Scene::new(&m, &tl, &p);
        for k in 1..p.steps.len() {
            // the envelopes' pass, when they give one, is a proven bound too
            let quick = sc.handling_check(k).expect("checked");
            let hp = sc.handling_path(k).unwrap();
            let b = sc.ctx(k);
            let tg = sc.handle_targets(k);
            // the search itself, whatever the envelopes say
            let c = b.handling(&hp, &tg);
            assert!(!c.undecided, "{c:?}");
            assert!(
                quick.pass <= c.pass,
                "the envelopes pass only what the search does"
            );
            let (pieces, _) = sc.part_samples(k);
            let mut least = f64::INFINITY;
            for t in &tg {
                for &s in t.spans() {
                    for i in 0..=24 {
                        let f = KNOTS[s] + (KNOTS[s + 1] - KNOTS[s]) * i as f64 / 24.0;
                        for pt in pieces.iter().flatten() {
                            least = least.min(t.sd(hp.place(*pt, f)));
                        }
                    }
                }
            }
            if c.pass || c.near.is_some() {
                let bound = c.bound.expect("a proven bound");
                assert!(
                    least >= bound - 1e-6,
                    "step {k}: a sample at {least} under the bound {bound}"
                );
            }
            if c.pass {
                assert!(least >= c.limit - 1e-6, "step {k}: {least}");
            }
            if quick.pass {
                assert!(least >= quick.bound.unwrap() - 1e-6, "step {k}: {least}");
            }
        }
    }

    /// A straight translation, f of the way.
    struct Slide([f64; 3]);
    impl Moving for Slide {
        fn knots(&self) -> &[f64] {
            &[0.0, 1.0]
        }
        fn place(&self, p: [f64; 3], f: f64) -> [f64; 3] {
            [0, 1, 2].map(|i| p[i] + self.0[i] * f)
        }
        fn enclose(&self, p: [Interval; 3], f: Interval) -> [Interval; 3] {
            [0, 1, 2].map(|i| p[i] + f * self.0[i])
        }
    }

    /// The handling check proves both ways: a move driving the part down through the bed is a
    /// proven hit (a real point, certified), a move far out in front of the press and back up
    /// over it is proven clear (bound at least the clearance), and none of it is undecided.
    #[test]
    fn handling_proves_a_hit_and_a_clear_move() {
        let m = ell();
        let tl = library::std_eu();
        let p = plan(&m, &tl, &Options::default());
        let sc = Scene::new(&m, &tl, &p);
        let ctx = sc.ctx(1);
        // the part from where it rests, 400 mm down: into the die and the bed
        let all = ctx.handle_targets(None, &[0], &[]);
        let c = ctx.handling(&Slide([0.0, 0.0, -400.0]), &all);
        assert!(!c.pass && c.certified && !c.undecided, "{c:?}");
        assert!(c.value < c.limit && c.at.is_some() && c.part.is_some(), "{c:?}");
        // the same move with the terrains left out of its span (a vertical move): the bed and
        // the die are not met, so only bodies standing beside the part are
        let up = ctx.handle_targets(None, &[0], &[0]);
        for t in &up {
            let terrain = t.name() == "bed" || t.name().starts_with("die:");
            assert_eq!(t.spans().is_empty(), terrain, "{}", t.name());
        }
        // lifted far over everything the press has (the ram itself rides at most 2 m over the
        // tip, as the section's rectangle has it): clear of the bodies below
        let far = ctx.handling(&Slide([-3000.0, 0.0, 0.0]), &ctx.handle_targets(None, &[0], &[]));
        assert!(!far.undecided, "{far:?}");
        // the part rests on the die at f = 0: that resting contact is the step's own check, so
        // a slide out of it measures under the clearance, a proven hit at the start
        assert!(!far.pass && far.certified, "{far:?}");
    }
}
