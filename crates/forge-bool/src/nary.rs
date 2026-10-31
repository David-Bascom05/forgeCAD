//! A whole CSG history evaluated in one pass (after QuickCSG, Douze & Franco 2017), instead
//! of one boolean per feature.
//!
//! 1. Every pair of overlapping tools is intersected; each curve is trimmed to both faces.
//! 2. Each new edge is cut where a third tool's face crosses it (a triple point).
//! 3. Cut points within tolerance are one vertex.
//! 4. Every tool face is split by the new edges on it.
//! 5. Each region is classified once: which tools hold a point just either side of it, and
//!    whether the expression changes across it.
//! 6. The regions where it does are sewn into one body.
//!
//! Every pair, edge, face and region is worked on its own, so each step runs across threads.
//! Classification only ever tests points against the tools (a few faces each), never against
//! a part grown to thousands of faces.

use crate::boolean::{
    BoolError, Cut, CutOn, Imprint, Op, Piece, R, Seg, Side, Touch, VKey, Walk, branches_meet,
    crossings_near, edge_split, grown_tols, imprint, lean, local_class, on_edge, overlap, settle, split_face,
    touches, trim,
};
use crate::classify::{Index, Where};
use crate::shape::{Member, Shape};
use crate::ssi;
use forge_geom::intersect::{Contact, CurveSurface, SurfaceSurface, curve_surface};
use forge_geom::{Curve, CurveGeom, SurfaceGeom};
use forge_math::{Interval, Point2, Point3, tol};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Time spent per step of the last n-ary boolean on this thread (ns): intersect, triple
/// points, vertices and pieces, split, classify, build, (inside classify) choosing points, merging, the batch.
pub fn nary_phase_times() -> [u64; 9] {
    PHASES.with(|p| *p.borrow())
}

thread_local! {
    static PHASES: std::cell::RefCell<[u64; 9]> = const { std::cell::RefCell::new([0; 9]) };
}

fn lap(t: &mut std::time::Instant, k: usize) {
    let now = std::time::Instant::now();
    PHASES.with(|p| p.borrow_mut()[k] = (now - *t).as_nanos() as u64);
    *t = now;
}

static PICK: AtomicUsize = AtomicUsize::new(0);

/// `f(0..n)` on up to `threads` threads, each given at least `per` items; results in index
/// order. The calling thread starts alone and the others join only once the work has run long
/// enough to pay for starting them (tens of microseconds each).
fn par_map<T: Send>(n: usize, threads: usize, per: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = threads.min(n / per.max(1));
    if threads <= 1 || n < 2 {
        return (0..n).map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let work = || {
        let mut got = Vec::new();
        loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= n {
                break got;
            }
            got.push((i, f(i)));
        }
    };
    let parts: Vec<Vec<(usize, T)>> = std::thread::scope(|s| {
        let start = std::time::Instant::now();
        let mut mine = Vec::new();
        let mut helpers = Vec::new();
        loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= n {
                break;
            }
            mine.push((i, f(i)));
            if helpers.is_empty()
                && next.load(Ordering::Relaxed) < n
                && start.elapsed() > std::time::Duration::from_micros(200)
            {
                helpers = (1..threads).map(|_| s.spawn(work)).collect();
            }
        }
        let mut parts = vec![mine];
        parts.extend(helpers.into_iter().map(|w| w.join().expect("worker")));
        parts
    });
    let mut out: Vec<Option<T>> = (0..n).map(|_| None).collect();
    for (i, t) in parts.into_iter().flatten() {
        out[i] = Some(t);
    }
    out.into_iter().map(|t| t.expect("every item done")).collect()
}

/// How far clear of a tool's boundary a point must be for its analytic solid to answer.
const SHAPE_MARGIN: f64 = tol::LINEAR;

/// The part of `range` along which `c` may come within any of `boxes` (None: nowhere): for a
/// traced curve, the span of its chain segments (each within its chord and sag of its ends) that
/// meet one; for any other curve, the whole range.
fn stretch(c: &Curve, range: Interval, boxes: &[forge_math::Box3]) -> Option<Interval> {
    let Curve::Intersection(ic) = c else {
        return Some(range);
    };
    let (n, segs) = (ic.pts.len() as i64, ic.segments() as i64);
    let mut out: Option<Interval> = None;
    let mut k = range.lo.floor() as i64;
    while (k as f64) < range.hi {
        let (a, b) = (range.lo.max(k as f64), range.hi.min((k + 1) as f64));
        let i = if ic.closed {
            k.rem_euclid(segs)
        } else {
            k.clamp(0, segs - 1)
        };
        let (p, q) = (ic.pts[i as usize], ic.pts[((i + 1) % n) as usize]);
        let bx = forge_math::Box3::from_points([p, q]).grow(ic.sag + tol::LINEAR);
        if b > a && boxes.iter().any(|x| x.overlaps(&bx)) {
            out = Some(out.map_or(Interval::new(a, b), |o| Interval::new(o.lo, b)));
        }
        k += 1;
    }
    out
}

/// Whether surface `s` may pass through box `b` (false only when the box surely lies on one side
/// of it, clear by more than a tolerance).
fn may_meet(s: &forge_geom::Surface, b: &forge_math::Box3) -> bool {
    use forge_geom::Surface;
    let m = 10.0 * tol::LINEAR;
    let corners = || {
        [b.x.lo, b.x.hi].into_iter().flat_map(move |x| {
            [b.y.lo, b.y.hi]
                .into_iter()
                .flat_map(move |y| [b.z.lo, b.z.hi].into_iter().map(move |z| Point3::new(x, y, z)))
        })
    };
    // the box's radii about a frame's z axis
    let radii = |f: &forge_math::Frame| {
        let mut l = forge_math::Box3::EMPTY;
        for c in corners() {
            l.include(f.point_to_local(c));
        }
        let gap = |lo: f64, hi: f64| {
            if lo > 0.0 {
                lo
            } else if hi < 0.0 {
                -hi
            } else {
                0.0
            }
        };
        let (gx, gy) = (gap(l.x.lo, l.x.hi), gap(l.y.lo, l.y.hi));
        let (ax, ay) = (l.x.lo.abs().max(l.x.hi.abs()), l.y.lo.abs().max(l.y.hi.abs()));
        ((gx * gx + gy * gy).sqrt(), (ax * ax + ay * ay).sqrt())
    };
    match s {
        Surface::Plane(p) => {
            let (o, z) = (p.frame.origin, p.frame.z().get());
            let (lo, hi) = corners()
                .map(|c| (c - o).dot(z))
                .fold((f64::INFINITY, f64::NEG_INFINITY), |a, d| {
                    (a.0.min(d), a.1.max(d))
                });
            lo <= m && hi >= -m
        }
        Surface::Cylinder(c) => {
            let (lo, hi) = radii(&c.frame);
            lo <= c.radius + m && hi >= c.radius - m
        }
        Surface::Sphere(sp) => {
            let o = sp.frame.origin;
            let gap = |c: f64, lo: f64, hi: f64| (lo - c).max(c - hi).max(0.0);
            let near = forge_math::Vec3::new(
                gap(o.x, b.x.lo, b.x.hi),
                gap(o.y, b.y.lo, b.y.hi),
                gap(o.z, b.z.lo, b.z.hi),
            );
            let far = corners().map(|c| c.distance(o)).fold(0.0, f64::max);
            near.length() <= sp.radius + m && far >= sp.radius - m
        }
        _ => true,
    }
}

/// Tool boxes binned on a coarse grid: for a point, the tools whose boxes might hold it, in
/// order (every tool whose box holds it is listed).
struct Grid {
    all: forge_math::Box3,
    k: usize,
    cells: Vec<Vec<u32>>,
}

impl Grid {
    fn new(boxes: &[forge_math::Box3]) -> Grid {
        let all = boxes.iter().fold(forge_math::Box3::EMPTY, |a, b| a.hull(b));
        // about one tool a cell
        let k = (1..16).find(|k| k * k * k >= boxes.len()).unwrap_or(16);
        let mut g = Grid {
            all,
            k,
            cells: vec![Vec::new(); k * k * k],
        };
        for (t, b) in boxes.iter().enumerate() {
            let (lo, hi) = (g.cell([b.x.lo, b.y.lo, b.z.lo]), g.cell([b.x.hi, b.y.hi, b.z.hi]));
            for x in lo[0]..=hi[0] {
                for y in lo[1]..=hi[1] {
                    for z in lo[2]..=hi[2] {
                        g.cells[(x * k + y) * k + z].push(t as u32);
                    }
                }
            }
        }
        g
    }

    /// The cell of a point, per axis; monotone in each coordinate, so a box's cells hold
    /// every point of it.
    fn cell(&self, c: [f64; 3]) -> [usize; 3] {
        let a = [self.all.x, self.all.y, self.all.z];
        std::array::from_fn(|i| {
            let w = a[i].width();
            let f = if w > 0.0 {
                (c[i] - a[i].lo) / w * self.k as f64
            } else {
                0.0
            };
            (f.max(0.0) as usize).min(self.k - 1)
        })
    }

    /// The tools whose boxes might meet `b`, in order.
    fn near(&self, b: &forge_math::Box3) -> Vec<u32> {
        if !self.all.overlaps(b) {
            return Vec::new();
        }
        let (lo, hi) = (
            self.cell([b.x.lo, b.y.lo, b.z.lo]),
            self.cell([b.x.hi, b.y.hi, b.z.hi]),
        );
        let mut out = Vec::new();
        for x in lo[0]..=hi[0] {
            for y in lo[1]..=hi[1] {
                for z in lo[2]..=hi[2] {
                    out.extend_from_slice(&self.cells[(x * self.k + y) * self.k + z]);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    fn at(&self, p: Point3) -> &[u32] {
        if !self.all.contains(p) {
            return &[];
        }
        let [x, y, z] = self.cell([p.x, p.y, p.z]);
        &self.cells[(x * self.k + y) * self.k + z]
    }
}

/// Which tools hold a point just inside and just outside a region, and whether another tool's
/// face lies on it.
type Seen = (Vec<bool>, Vec<bool>, bool);
/// A region's loops and its verdict: None dropped, Some(flip) kept.
type Judged = (Vec<Walk>, Option<bool>);

/// FORGE_NARY is set (read once).
fn nary_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("FORGE_NARY").is_some())
}

/// The region FORGE_NARY_REG asks to show.
fn nary_reg() -> Option<usize> {
    static REG: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    *REG.get_or_init(|| std::env::var("FORGE_NARY_REG").ok().and_then(|v| v.parse().ok()))
}

/// A cut of a new edge by a third tool's face.
struct SegCut {
    seg: usize,
    t: f64,
    point: Point3,
    tol: f64,
    /// The face whose surface it crosses (None where it meets that face's boundary).
    by: Option<(Side, FaceId)>,
}

/// What intersecting one pair of tools gives.
#[derive(Default)]
struct PairOut {
    cuts: Vec<Cut>,
    segs: Vec<Seg>,
    imprints: Vec<Imprint>,
    coplanar: Vec<[(Side, FaceId); 2]>,
    touching: Vec<Touch>,
    /// Face pairs whose meeting is not wholly in the new edges (points, coincident, touches,
    /// curves along an edge, left out as outside the result or dividing neither face).
    loose: Vec<FacePair>,
}

/// Two tools' faces, the lower tool first.
type FacePair = ((Side, FaceId), (Side, FaceId));

/// A region of a tool face, before classification.
struct Reg {
    side: Side,
    face: FaceId,
    /// Empty for a face nothing touches (its own loops, made into pieces only if kept).
    loops: Vec<Walk>,
    name: Name,
    /// The split job it came from (None for an untouched face).
    job: Option<usize>,
}

/// The pieces of the tools' edges and the new edges, split once each.
struct Pieces<'a> {
    tools: &'a [&'a Body],
    cuts: &'a [Cut],
    by_edge: &'a [(Side, EdgeId, usize)],
    cut_vertex: &'a [VKey],
    alias: &'a [(VKey, VKey)],
    list: Vec<Piece>,
    /// Per tool, per edge: (first piece, count), count 0 if not split yet.
    of_edge: Vec<Vec<(u32, u32)>>,
}

impl Pieces<'_> {
    fn ensure(&mut self, side: Side, e: EdgeId) -> std::ops::Range<usize> {
        let (first, n) = self.of_edge[side][e.index()];
        if n > 0 {
            return first as usize..(first + n) as usize;
        }
        let r = edge_split(
            self.tools[side],
            side,
            e,
            self.cuts,
            self.by_edge,
            self.cut_vertex,
            self.alias,
            &mut self.list,
        );
        self.of_edge[side][e.index()] = (r.start as u32, r.len() as u32);
        r
    }

    /// The face's own loops as walks of pieces.
    fn walks(&mut self, side: Side, f: FaceId) -> Vec<Walk> {
        let body = self.tools[side];
        let mut out = Vec::with_capacity(body.face(f).loops.len());
        for &l in &body.face(f).loops {
            let mut w = Vec::with_capacity(body.lp(l).coedges().len());
            for c in body.lp(l).coedges() {
                let co = body.coedge(c);
                let ps = self.ensure(side, co.edge);
                if co.forward {
                    w.extend(ps.map(|p| (p, true)));
                } else {
                    w.extend(ps.rev().map(|p| (p, false)));
                }
            }
            out.push(w);
        }
        out
    }
}

/// Whether a point is in the result, given which tools hold it: tool 0, then each later
/// tool combined by `ops[k - 1]`.
fn in_result(ops: &[Op], inside: &[bool], any_intersect: bool) -> bool {
    if !any_intersect {
        // a tool not holding the point changes nothing: the last one holding it decides
        return match (0..inside.len()).rev().find(|&k| inside[k]) {
            None => false,
            Some(0) => true,
            Some(k) => ops[k - 1] == Op::Union,
        };
    }
    let mut v = inside[0];
    for k in 1..inside.len() {
        v = match ops[k - 1] {
            Op::Union => v || inside[k],
            Op::Subtract => v && !inside[k],
            Op::Intersect => v && inside[k],
        };
    }
    v
}

/// Whether the region on tool `side`'s face, with these tools holding a point just inside and
/// just outside it, is the face the sequential regen keeps there: the boundary through the point
/// is that of the tool that last made it (as the part stood before each tool, a face already
/// there keeps its place against a coincident one of a later tool).
fn owned(ops: &[Op], inner: &[bool], outer: &[bool], side: Side) -> bool {
    let (mut vin, mut vout) = (inner[0], outer[0]);
    let mut was = (vin != vout).then_some(vin);
    let mut owner = 0;
    for k in 1..inner.len() {
        (vin, vout) = match ops[k - 1] {
            Op::Union => (vin || inner[k], vout || outer[k]),
            Op::Subtract => (vin && !inner[k], vout && !outer[k]),
            Op::Intersect => (vin && inner[k], vout && outer[k]),
        };
        let now = (vin != vout).then_some(vin);
        if now.is_some() && now != was {
            owner = k;
        }
        was = now;
    }
    owner == side
}

/// Whether the verdict on tool `side`'s face differs across tool `other`'s surface, at a place
/// held by the tools `holds` (in order) and no others: whether a new edge of the two divides it.
fn divides(ops: &[Op], any_intersect: bool, holds: &[Side], side: Side, other: Side) -> bool {
    let n = ops.len() + 1;
    let result = |own: bool, held: bool| -> bool {
        if any_intersect {
            let mut inside = vec![false; n];
            for &k in holds {
                inside[k] = true;
            }
            inside[side] = own;
            inside[other] = held;
            return in_result(ops, &inside, true);
        }
        // as in_result: the last tool holding the point decides
        let last = holds
            .last()
            .copied()
            .into_iter()
            .chain(own.then_some(side))
            .chain(held.then_some(other))
            .max();
        match last {
            None => false,
            Some(0) => true,
            Some(k) => ops[k - 1] == Op::Union,
        }
    };
    let verdict = |held: bool| {
        let (fin, fout) = (result(true, held), result(false, held));
        (fin != fout).then_some(!fin)
    };
    verdict(true) != verdict(false)
}

/// The solid `tools[0] ops[0] tools[1] ops[1] tools[2] ...`, folded left, in one pass.
pub fn nary(tools: &[&Body], ops: &[Op], threads: usize) -> R<Body> {
    nary_with(tools, ops, None, None, threads, Name::feature(0))
}

/// Verdicts for many region points at once (on a GPU, say), given each point's own tool and
/// every tool's box: 0 dropped, 1 kept, 2 kept flipped, 3 undecided (the full way decides).
pub trait Batch: Sync {
    fn verdicts(&self, points: &[Point3], sides: &[u32], boxes: &[forge_math::Box3]) -> Vec<u8>;
    /// Called while the points are being placed, before `verdicts` (a GPU wakes up).
    fn warm(&self) {}
}

/// The batch on the CPU, from the tools' analytic solids: what a GPU batch must match.
pub struct CpuBatch<'a> {
    pub shapes: &'a [Shape],
    pub ops: &'a [Op],
    pub threads: usize,
}

impl Batch for CpuBatch<'_> {
    fn verdicts(&self, points: &[Point3], sides: &[u32], boxes: &[forge_math::Box3]) -> Vec<u8> {
        // with no intersect, a tool whose box misses the point changes nothing: only the tools
        // the grid lists (and the point's own) are walked
        let grid = (!self.ops.contains(&Op::Intersect)).then(|| Grid::new(boxes));
        par_map(points.len(), self.threads, 256, |i| {
            let p = points[i];
            let own = sides[i] as usize;
            let (mut vin, mut vout) = (false, false);
            // false where tool k's boundary is too near to tell
            let mut apply = |k: usize| -> bool {
                let (bin, bout) = if k == own {
                    (true, false)
                } else if !boxes[k].contains(p) {
                    (false, false)
                } else {
                    match self.shapes[k].member(p, SHAPE_MARGIN) {
                        Member::Near => return false,
                        Member::In => (true, true),
                        Member::Out => (false, false),
                    }
                };
                if k == 0 {
                    (vin, vout) = (bin, bout);
                } else {
                    match self.ops[k - 1] {
                        Op::Union => (vin, vout) = (vin || bin, vout || bout),
                        Op::Subtract => (vin, vout) = (vin && !bin, vout && !bout),
                        Op::Intersect => (vin, vout) = (vin && bin, vout && bout),
                    }
                }
                true
            };
            match &grid {
                None => {
                    for k in 0..self.shapes.len() {
                        if !apply(k) {
                            return 3;
                        }
                    }
                }
                Some(g) => {
                    // the listed tools with `own` in its place
                    let mut own_left = own < self.shapes.len();
                    for &k in g.at(p) {
                        let k = k as usize;
                        if own_left && own <= k {
                            own_left = false;
                            if own < k && !apply(own) {
                                return 3;
                            }
                        }
                        if !apply(k) {
                            return 3;
                        }
                    }
                    if own_left && !apply(own) {
                        return 3;
                    }
                }
            }
            if vin == vout {
                0
            } else if vin {
                1
            } else {
                2
            }
        })
    }
}

/// `nary`, with each tool also given as an analytic solid: a point clear of a tool's boundary
/// is placed from its profile, and only points near it go to the B-rep. New vertices are named
/// from `root`. The body comes out compact (as `Body::compact` leaves it).
pub fn nary_with(
    tools: &[&Body],
    ops: &[Op],
    shapes: Option<&[Shape]>,
    batch: Option<&dyn Batch>,
    threads: usize,
    root: Name,
) -> R<Body> {
    let mut out = nary_body(tools, ops, shapes, batch, threads, root)?;
    // pieces left touching at a point keep a vertex each: where walking round a vertex from one
    // coedge leaving it (back along its loop, across to the partner) misses some of the others
    let nv = out.vertices.len();
    let from = |x: &forge_topo::Edge, c: forge_topo::CoedgeId| {
        if out.coedges[c.index()].forward {
            x.start
        } else {
            x.end
        }
        .index()
    };
    // the coedges leaving vertex v are leaving[at[v]..at[v + 1]]
    let mut at = vec![0u32; nv + 1];
    for x in out.edges.iter().filter(|x| x.alive) {
        for &c in x.coedges.iter() {
            at[from(x, c) + 1] += 1;
        }
    }
    for v in 0..nv {
        at[v + 1] += at[v];
    }
    let mut leaving = vec![forge_topo::CoedgeId(0); at[nv] as usize];
    let mut fill = at.clone();
    for x in out.edges.iter().filter(|x| x.alive) {
        for &c in x.coedges.iter() {
            let v = from(x, c);
            leaving[fill[v] as usize] = c;
            fill[v] += 1;
        }
    }
    let mut before = vec![forge_topo::CoedgeId(0); out.coedges.len()];
    for l in &out.loops {
        let mut last = l.coedges().next_back().unwrap_or_default();
        for c in l.coedges() {
            before[c.index()] = last;
            last = c;
        }
    }
    let mut prev = Vec::new();
    for v in 0..nv {
        let from = &leaving[at[v] as usize..at[v + 1] as usize];
        let Some(&c0) = from.first() else { continue };
        let (mut c, mut k) = (c0, 1);
        let one = loop {
            match out.partner(before[c.index()]) {
                Some(n) if n == c0 => break k == from.len(),
                Some(n) if k < from.len() => (c, k) = (n, k + 1),
                _ => break false,
            }
        };
        if !one {
            out.split_pinched(VertexId(v as u32), from, &mut prev);
        }
    }
    Ok(out)
}

fn nary_body(
    tools: &[&Body],
    ops: &[Op],
    shapes: Option<&[Shape]>,
    batch: Option<&dyn Batch>,
    threads: usize,
    root: Name,
) -> R<Body> {
    assert_eq!(ops.len() + 1, tools.len(), "one op per tool after the first");
    let n = tools.len();
    let mut clock = std::time::Instant::now();
    let index: Vec<Index> = par_map(n, threads, 16, |k| Index::new(tools[k]));
    let debug = crate::boolean::debug_on();

    // the result lies within the first tool and those added to it: what is surely outside all of
    // them bounds none of it (its regions are all dropped, however they are split)
    let outside = |b: &forge_math::Box3| -> bool {
        let Some(sh) = shapes else { return false };
        (0..n)
            .filter(|&k| k == 0 || ops[k - 1] == Op::Union)
            .all(|k| !index[k].bbox.overlaps(b) || sh[k].box_out(b, SHAPE_MARGIN))
    };

    let any_intersect = ops.contains(&Op::Intersect);
    // whether a new edge of tools i and j in box b could divide a face of either: false only
    // when every other tool surely holds all of b or none of it
    let meets_matter = |b: &forge_math::Box3, i: Side, j: Side| -> bool {
        let Some(sh) = shapes else { return true };
        let (lo, hi) = (
            Point3::new(b.x.lo, b.y.lo, b.z.lo),
            Point3::new(b.x.hi, b.y.hi, b.z.hi),
        );
        // (with no intersect only the last holder matters)
        let mut holds: Vec<Side> = Vec::new();
        let mut last: Option<Side> = None;
        for k in 0..n {
            if k == i || k == j || !index[k].bbox.overlaps(b) || sh[k].box_out(b, SHAPE_MARGIN) {
                continue;
            }
            if index[k].bbox.contains(lo) && index[k].bbox.contains(hi) && sh[k].box_in(b, SHAPE_MARGIN) {
                if any_intersect {
                    holds.push(k);
                }
                last = Some(k);
            } else {
                return true;
            }
        }
        let holds = if any_intersect {
            &holds[..]
        } else {
            last.as_slice()
        };
        divides(ops, any_intersect, holds, i, j) || divides(ops, any_intersect, holds, j, i)
    };

    // 1: pairs of tools whose boxes meet, then their faces
    let mut pairs: Vec<(Side, Side)> = Vec::new();
    // tool pairs whose boxes meet but were left out as surely outside the result: whether their
    // faces meet is not known
    let mut unknown: Vec<(Side, Side)> = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            if index[i].bbox.overlaps(&index[j].bbox) {
                if outside(&overlap(&index[i].bbox, &index[j].bbox).grow(tol::LINEAR)) {
                    unknown.push((i, j));
                } else {
                    pairs.push((i, j));
                }
            }
        }
    }
    let outs = par_map(pairs.len(), threads, 2, |pi| -> R<PairOut> {
        let (i, j) = pairs[pi];
        let (a, b) = (tools[i], tools[j]);
        let (ia, ib) = (&index[i], &index[j]);
        let mut out = PairOut::default();
        // a face surely clear of the other tool's solid meets none of its faces
        let clear = |f: &forge_math::Box3, k: Side| shapes.is_some_and(|sh| sh[k].box_out(f, SHAPE_MARGIN));
        let (ba, bb) = (ia.boxes(), ib.boxes());
        let b_clear: Vec<bool> = b
            .face_ids()
            .map(|fb| bb[fb.index()].overlaps(&ia.bbox) && clear(&bb[fb.index()], i))
            .collect();
        for fa in a.face_ids() {
            if !ba[fa.index()].overlaps(&ib.bbox) || clear(&ba[fa.index()], j) {
                continue;
            }
            for fb in b.face_ids() {
                if !ba[fa.index()].overlaps(&bb[fb.index()]) || b_clear[fb.index()] {
                    continue;
                }
                let region = overlap(&ba[fa.index()], &bb[fb.index()]).grow(tol::LINEAR);
                // an edge here would be dropped below, with both its sides (not known to meet: kept
                // as meeting for the triple points)
                if outside(&region) {
                    out.loose.push(((i, fa), (j, fb)));
                    continue;
                }
                let (sa, sb) = (&a.face(fa).surface, &b.face(fb).surface);
                // a pair with no closed form is searched, at a cost: not where the tools holding all
                // of the region (by bounds) say its curves would divide neither face
                let found = match ssi::closed_form(sa, sb) {
                    SurfaceSurface::NotAnalytic if !meets_matter(&region, i, j) => {
                        out.loose.push(((i, fa), (j, fb)));
                        continue;
                    }
                    SurfaceSurface::NotAnalytic => ssi::searched(sa, sb, region),
                    r => r,
                };
                out.touching
                    .extend(touches(&found, [ia, ib], [i, j], [fa, fb], region));
                let curves = match found {
                    SurfaceSurface::Empty => continue,
                    SurfaceSurface::Points(_) => {
                        out.loose.push(((i, fa), (j, fb)));
                        continue;
                    }
                    SurfaceSurface::Coincident => {
                        out.coplanar.push([(i, fa), (j, fb)]);
                        let both = [a, b];
                        imprint(
                            both,
                            [ia, ib],
                            [i, j],
                            1,
                            fb,
                            fa,
                            &mut out.cuts,
                            &mut out.imprints,
                        )?;
                        imprint(
                            both,
                            [ia, ib],
                            [i, j],
                            0,
                            fa,
                            fb,
                            &mut out.cuts,
                            &mut out.imprints,
                        )?;
                        continue;
                    }
                    SurfaceSurface::NotAnalytic => {
                        return Err(BoolError::Unsupported("surface pair without a closed form"));
                    }
                    SurfaceSurface::Curves(cs) => cs,
                };
                // branches of one intersection may meet where the surfaces touch
                let branches = [
                    curves.iter().filter(|c| c.1 == Contact::Crossing).count() > 1,
                    branches_meet(&curves),
                ];
                // where the faces' meeting is not all in new edges (a touch, a curve along an
                // edge), the triple points search the whole of every edge; a curve the trim leaves
                // out lies outside a face, where no triple point of the two faces can be
                let imprints_before = out.imprints.len();
                let mut whole = true;
                for (ci, (curve, contact)) in curves.into_iter().enumerate() {
                    if contact == Contact::Touching {
                        whole = false;
                        continue;
                    }
                    let name = Name::feature(0)
                        .with_str("cut")
                        .with_set(&mut [a.face(fa).name, b.face(fb).name])
                        .with(ci as u64);
                    trim(
                        [a, b],
                        [ia, ib],
                        [i, j],
                        [fa, fb],
                        curve,
                        name,
                        branches,
                        &mut out.cuts,
                        &mut out.segs,
                        &mut out.imprints,
                    )?;
                }
                if !whole || out.imprints.len() > imprints_before {
                    out.loose.push(((i, fa), (j, fb)));
                }
            }
        }
        Ok(out)
    });
    let mut cuts: Vec<Cut> = Vec::new();
    let mut segs: Vec<Seg> = Vec::new();
    let mut imprints: Vec<Imprint> = Vec::new();
    let mut coplanar: Vec<[(Side, FaceId); 2]> = Vec::new();
    let mut touching: Vec<Touch> = Vec::new();
    let mut loose: Vec<FacePair> = Vec::new();
    for out in outs {
        let out = out?;
        loose.extend(out.loose);
        let base = cuts.len();
        cuts.extend(out.cuts);
        segs.extend(out.segs.into_iter().map(|mut s| {
            s.ends = s.ends.map(|(x, y)| (x + base, y + base));
            s
        }));
        imprints.extend(out.imprints);
        coplanar.extend(out.coplanar);
        touching.extend(out.touching);
    }
    // every new edge's box by its face pair (before those outside the result go): a triple point
    // on an edge and a third tool's face h lies on the edge of the edge's face and h
    let key = |x: (Side, FaceId), y: (Side, FaceId)| if x.0 < y.0 { (x, y) } else { (y, x) };
    // each new edge's box, once (a traced curve's costs evaluations)
    let mut seg_box: Vec<forge_math::Box3> = segs.iter().map(|sg| sg.curve.bbox(sg.range)).collect();
    let keep: Vec<bool> = seg_box
        .iter()
        .map(|b| shapes.is_none() || !outside(&b.grow(tol::LINEAR)))
        .collect();
    let is_traced = |sg: &Seg| matches!(sg.curve, Curve::Intersection(_));
    // only a traced edge's triple points are narrowed by them, so only the faces those lie on
    let mut traced: Vec<(Side, FaceId)> = segs
        .iter()
        .filter(|sg| is_traced(sg))
        .flat_map(|sg| [(sg.sides[0], sg.faces[0]), (sg.sides[1], sg.faces[1])])
        .collect();
    traced.sort_unstable();
    traced.dedup();
    let on_traced = |f: (Side, FaceId)| traced.binary_search(&f).is_ok();
    // (face pair, box, whether the edge is kept, whether it meets surfaces in closed form)
    let mut along: Vec<(FacePair, forge_math::Box3, bool, bool)> = segs
        .iter()
        .zip(&seg_box)
        .zip(&keep)
        .filter(|((sg, _), _)| !traced.is_empty() && (0..2).any(|e| on_traced((sg.sides[e], sg.faces[e]))))
        .map(|((sg, b), &kept)| {
            (
                key((sg.sides[0], sg.faces[0]), (sg.sides[1], sg.faces[1])),
                b.grow(10.0 * tol::LINEAR),
                kept,
                !is_traced(sg),
            )
        })
        .collect();
    along.sort_unstable_by_key(|x| x.0);
    if !traced.is_empty() {
        // coincident faces too (kept in `coplanar` already)
        loose.extend(coplanar.iter().map(|&[x, y]| key(x, y)));
        loose.sort_unstable();
    }
    if shapes.is_some() {
        let mut flags = keep.iter();
        segs.retain(|_| *flags.next().expect("a flag a seg"));
        let mut flags = keep.iter();
        seg_box.retain(|_| *flags.next().expect("a flag a seg"));
    }
    // where two tools touch, both faces kept would be material meeting itself
    let kept_at = |side: Side, q: Point3| -> Option<bool> {
        let mut inner = vec![false; n];
        inner[side] = true;
        for k in 0..n {
            if k == side || !index[k].bbox.contains(q) {
                continue;
            }
            match index[k].classify(q) {
                Where::Inside => inner[k] = true,
                Where::Outside => {}
                Where::On => return None,
            }
        }
        let mut outer = inner.clone();
        outer[side] = false;
        Some(in_result(ops, &inner, any_intersect) != in_result(ops, &outer, any_intersect))
    };
    for t in &touching {
        if debug {
            let got: Vec<_> = (0..2)
                .map(|k| {
                    let (sd, f) = (t.sides[k], t.faces[k]);
                    local_class(tools[sd], &index[sd], f, t.at, t.dir, |q| kept_at(sd, q))
                })
                .collect();
            eprintln!(
                "nary: touch {:?} {:?} at {:?}: kept {got:?}",
                t.sides, t.faces, t.at
            );
        }
        let both = (0..2).all(|k| {
            let (sd, f) = (t.sides[k], t.faces[k]);
            local_class(tools[sd], &index[sd], f, t.at, t.dir, |q| kept_at(sd, q)) == Some(true)
        });
        if both {
            return Err(BoolError::ZeroThickness { at: t.at });
        }
    }
    lap(&mut clock, 0);

    // 2: each new edge is cut where a face of a third tool crosses it
    let grid = Grid::new(&index.iter().map(|x| x.bbox).collect::<Vec<_>>());
    // `f` of each face of another tool that may cross edge si
    let third = |si: usize, f: &mut dyn FnMut((Side, FaceId))| {
        let sg = &segs[si];
        let reach = seg_box[si].grow(tol::LINEAR);
        for k in grid.near(&reach) {
            let k = k as usize;
            // the edge surely clear of tool k's solid crosses none of its faces
            if sg.sides.contains(&k)
                || !index[k].bbox.overlaps(&reach)
                || shapes.is_some_and(|sh| sh[k].box_out(&reach, SHAPE_MARGIN))
            {
                continue;
            }
            for h in tools[k].face_ids() {
                if index[k].boxes()[h.index()].overlaps(&reach) && may_meet(&tools[k].face(h).surface, &reach)
                {
                    f((k, h));
                }
            }
        }
    };
    // (t, point, tol, the face whose surface it crosses: None where it meets that face's boundary)
    type Hit = (f64, Point3, f64, Option<(Side, FaceId)>);
    // edge si's cuts by tool k's face h over `range` (false: it lies on h's surface)
    let cut_by = |si: usize, (k, h): (Side, FaceId), range: Interval, got: &mut Vec<Hit>| -> bool {
        let sg = &segs[si];
        let s = &tools[k].face(h).surface;
        let CurveSurface::Hits(hits) = curve_surface(&sg.curve, range, s) else {
            return false;
        };
        for hit in hits
            .into_iter()
            .flat_map(|x| crossings_near(&sg.curve, sg.range, s, x))
        {
            if index[k].point_in_face(h, hit.point) != Where::Outside {
                got.push((hit.t, hit.point, tol::LINEAR, Some((k, h))));
            }
        }
        true
    };
    // the new edge lies on this face's surface: it is cut where it crosses the face's boundary
    let on_boundary = |si: usize, (k, h): (Side, FaceId), got: &mut Vec<Hit>| {
        let sg = &segs[si];
        let body = tools[k];
        let reach = seg_box[si].grow(tol::LINEAR);
        for &l in &body.face(h).loops {
            for c in body.lp(l).coedges() {
                let e = body.coedge(c).edge;
                if !body.edge_box(e).overlaps(&reach) {
                    continue;
                }
                let Ok(across) = crate::boolean::across(body, h, c) else {
                    continue;
                };
                if let CurveSurface::Hits(hits) = curve_surface(&sg.curve, sg.range, &across) {
                    for hit in hits {
                        if let Some((_, tol)) = on_edge(body, e, hit.point) {
                            got.push((hit.t, hit.point, tol, None));
                        }
                    }
                }
            }
        }
    };
    // A traced edge's triple points cost evaluations to search for. Every triple point of edge
    // (a, b) and face h lies on the new edges of (a, h) and of (b, h), where those pairs meet only
    // in new edges: where one pair's edges all met the edge's other face in closed form, the points
    // are where they met it, placed on this edge by its nearest point; where all three pairs' edges
    // are traced, the pair of the two lowest faces searches and the other two take its points.
    // Otherwise only the stretch of the edge near the face pairs' own edges is searched. So what
    // each edge on a traced edge's face met is noted: its face pair, the face, where (None: the
    // edge lies on that face's surface)
    type Met = (FacePair, (Side, FaceId), Option<Point3>);
    enum Way {
        None,
        Take(FacePair, (Side, FaceId)),
        Later,
        Search(Option<Vec<forge_math::Box3>>),
    }
    let whole =
        |fp: FacePair| loose.binary_search(&fp).is_err() && unknown.binary_search(&(fp.0.0, fp.1.0)).is_err();
    let edges_of =
        |fp: FacePair| &along[along.partition_point(|x| x.0 < fp)..along.partition_point(|x| x.0 <= fp)];
    let met_of = |met: &'_ [Met], fp: FacePair, other: (Side, FaceId)| -> Option<Vec<Point3>> {
        let from = met.partition_point(|x| (x.0, x.1) < (fp, other));
        met[from..]
            .iter()
            .take_while(|x| (x.0, x.1) == (fp, other))
            .map(|x| x.2)
            .collect()
    };
    // `late`: the lowest pairs' searches are done
    let way = |si: usize, kh: (Side, FaceId), met: &[Met], late: bool| -> Way {
        let sg = &segs[si];
        if !is_traced(sg) {
            return Way::Search(None);
        }
        let fs = [(sg.sides[0], sg.faces[0]), (sg.sides[1], sg.faces[1])];
        let top = fs[0].max(fs[1]).max(kh);
        let mut near: Option<Vec<forge_math::Box3>> = None;
        for e in 0..2 {
            let fp = key(fs[e], kh);
            if !whole(fp) {
                continue;
            }
            let edges = edges_of(fp);
            if edges.is_empty() {
                return Way::None;
            }
            if edges.iter().all(|x| x.2 && x.3) && met_of(met, fp, fs[1 - e]).is_some() {
                return Way::Take(fp, fs[1 - e]);
            }
            if fs[1 - e] == top && edges.iter().all(|x| x.2) {
                if !late {
                    return Way::Later;
                }
                if met_of(met, fp, top).is_some() {
                    return Way::Take(fp, top);
                }
            }
            near.get_or_insert_with(|| edges.iter().map(|x| x.1).collect());
        }
        Way::Search(near.filter(|_| !late))
    };
    // edge si's triple points with face kh found that way (false: it lies on kh's surface)
    let cuts_with = |si: usize, kh: (Side, FaceId), met: &[Met], way: Way, got: &mut Vec<Hit>| -> bool {
        let sg = &segs[si];
        let crossed = match way {
            Way::None | Way::Later => true,
            Way::Take(fp, other) => {
                for p in met_of(met, fp, other).expect("all met at points") {
                    let t = sg.curve.closest(p, sg.range);
                    if sg.curve.eval(t).distance(p) <= tol::LINEAR
                        && index[kh.0].point_in_face(kh.1, p) != Where::Outside
                    {
                        got.push((t, p, tol::LINEAR, Some(kh)));
                    }
                }
                true
            }
            Way::Search(Some(boxes)) => match stretch(&sg.curve, sg.range, &boxes) {
                None => true,
                Some(r) => cut_by(si, kh, r, got) || cut_by(si, kh, sg.range, got),
            },
            Way::Search(None) => cut_by(si, kh, sg.range, got),
        };
        if !crossed {
            on_boundary(si, kh, got);
        }
        crossed
    };
    // edge si's cuts, what it met, and the faces left until the lowest pairs' searches are done
    let run = |si: usize, met: &[Met]| {
        let sg = &segs[si];
        let fs = [(sg.sides[0], sg.faces[0]), (sg.sides[1], sg.faces[1])];
        let (mut got, mut mine, mut later): (Vec<Hit>, Vec<Met>, Vec<(Side, FaceId)>) = Default::default();
        // an edge on no traced edge's face: what it met is never asked
        if !fs.iter().any(|&f| on_traced(f)) {
            third(si, &mut |kh| {
                if !cut_by(si, kh, sg.range, &mut got) {
                    on_boundary(si, kh, &mut got);
                }
            });
            return (got, mine, later);
        }
        third(si, &mut |kh| {
            let w = way(si, kh, met, false);
            if matches!(w, Way::Later) {
                later.push(kh);
                return;
            }
            let at = got.len();
            let fp = key(fs[0], fs[1]);
            if cuts_with(si, kh, met, w, &mut got) {
                mine.extend(got[at..].iter().map(|x| (fp, kh, Some(x.1))));
            } else {
                mine.push((fp, kh, None));
            }
        });
        (got, mine, later)
    };
    // edges met in closed form first, then traced ones, then those left
    let mut seg_cuts: Vec<Vec<Hit>> = vec![Vec::new(); segs.len()];
    let mut met: Vec<Met> = Vec::new();
    let mut later: Vec<(usize, (Side, FaceId))> = Vec::new();
    for traced in [false, true] {
        let these: Vec<usize> = (0..segs.len())
            .filter(|&si| is_traced(&segs[si]) == traced)
            .collect();
        let done = par_map(these.len(), threads, if traced { 4 } else { 32 }, |i| {
            run(these[i], &met)
        });
        for (&si, (got, mine, l)) in these.iter().zip(done) {
            seg_cuts[si] = got;
            met.extend(mine);
            later.extend(l.into_iter().map(|kh| (si, kh)));
        }
        met.sort_unstable_by_key(|x| (x.0, x.1));
    }
    let late = par_map(later.len(), threads, 16, |i| {
        let (si, kh) = later[i];
        let mut got = Vec::new();
        cuts_with(si, kh, &met, way(si, kh, &met, true), &mut got);
        got
    });
    for (&(si, _), got) in later.iter().zip(late) {
        seg_cuts[si].extend(got);
    }
    let mut triple: Vec<SegCut> = Vec::new();
    for (si, cs) in seg_cuts.into_iter().enumerate() {
        triple.extend(cs.into_iter().map(|(t, point, tol, by)| SegCut {
            seg: si,
            t,
            point,
            tol,
            by,
        }));
    }
    lap(&mut clock, 1);

    // 3: one vertex per cluster of cut points (an existing tool vertex where a cut sits at an
    // edge's end; vertices of different tools at one place are one)
    // points within reach of each other (transitively) are one; found by a sweep along x
    let pts: Vec<Point3> = cuts
        .iter()
        .map(|c| c.point)
        .chain(triple.iter().map(|c| c.point))
        .collect();
    let tols: Vec<f64> = cuts
        .iter()
        .map(|c| c.tol)
        .chain(triple.iter().map(|c| c.tol))
        .collect();
    let widest = tols.iter().fold(tol::LINEAR, |a, &b| a.max(b));
    let mut parent: Vec<usize> = (0..pts.len()).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    // sorted by slabs of x as wide as the widest reach, then by y: a point's partners lie in its
    // slab or the next, within that reach in y (a sweep along x alone met whole planes of points
    // at one x). Pairs join in any order: a cluster's root is its least index
    // + 0.0: a -0.0 slab sorts apart from 0.0 but compares equal to it, which split the scan
    let slab = |i: usize| (pts[i].x / widest).floor() + 0.0;
    let mut order: Vec<(f64, f64, usize)> = (0..pts.len()).map(|i| (slab(i), pts[i].y, i)).collect();
    order.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut join = |i: usize, j: usize| {
        if pts[i].distance(pts[j]) <= tols[i].max(tols[j]) {
            let (x, y) = (find(&mut parent, i), find(&mut parent, j));
            if x != y {
                parent[x.max(y)] = x.min(y);
            }
        }
    };
    for a in 0..order.len() {
        let (sx, y, i) = order[a];
        for &(tx, ty, j) in &order[a + 1..] {
            if tx != sx || ty - y > widest {
                break;
            }
            join(i, j);
        }
        let from = order.partition_point(|o| o.0 < sx + 1.0 || (o.0 == sx + 1.0 && o.1 < y - widest));
        for &(tx, ty, j) in &order[from..] {
            if tx != sx + 1.0 || ty > y + widest {
                break;
            }
            join(i, j);
        }
    }
    // clusters numbered in order of their first point; each sits at that point
    let mut cluster: Vec<usize> = Vec::with_capacity(pts.len());
    let mut heads: Vec<Point3> = Vec::new();
    let mut id_of_root: Vec<usize> = vec![usize::MAX; pts.len()];
    for i in 0..pts.len() {
        let r = find(&mut parent, i);
        if id_of_root[r] == usize::MAX {
            id_of_root[r] = heads.len();
            heads.push(pts[r]);
        }
        cluster.push(id_of_root[r]);
    }
    let mut olds: Vec<Vec<VKey>> = vec![Vec::new(); heads.len()];
    for (i, c) in cuts.iter().enumerate() {
        let Some((sd, e)) = c.on.edge() else { continue };
        let edge = tools[sd].edge(e);
        for v in [edge.start, edge.end] {
            let vx = tools[sd].vertex(v);
            if vx.point.distance(c.point) <= vx.tol.max(c.tol) {
                olds[cluster[i]].push(VKey::Old(sd, v));
                break;
            }
        }
    }
    let mut points: Vec<Point3> = Vec::new();
    let mut point_tols: Vec<f64> = Vec::new();
    let mut alias: Vec<(VKey, VKey)> = Vec::new();
    let mut key_of: Vec<VKey> = Vec::with_capacity(heads.len());
    for (k, os) in olds.iter_mut().enumerate() {
        os.sort();
        os.dedup();
        let key = match os.first() {
            Some(&v) => v,
            None => {
                points.push(heads[k]);
                VKey::New(points.len() as u32 - 1)
            }
        };
        let mut sides: Vec<Side> = Vec::new();
        for &o in os.iter() {
            let VKey::Old(sd, _) = o else { unreachable!() };
            if sides.contains(&sd) {
                return Err(BoolError::Unsupported("two vertices of one tool meet at a cut"));
            }
            sides.push(sd);
            if o != key {
                alias.push((o, key));
            }
        }
        key_of.push(key);
    }
    alias.sort();
    let cut_vertex: Vec<VKey> = cluster[..cuts.len()].iter().map(|&k| key_of[k]).collect();
    let triple_vertex: Vec<VKey> = cluster[cuts.len()..].iter().map(|&k| key_of[k]).collect();
    let mut marks = crate::boolean::curve_ends(&segs, &cut_vertex);
    for (tc, &k) in triple.iter().zip(&triple_vertex) {
        if let VKey::New(i) = k {
            marks.push((i, segs[tc.seg].name));
        }
    }
    let mut ended = vec![false; points.len()];
    for m in &marks {
        ended[m.0 as usize] = true;
    }
    for (c, &k) in cuts.iter().zip(&cut_vertex) {
        if let VKey::New(i) = k
            && !ended[i as usize]
            && let Some((sd, e)) = c.on.edge()
        {
            marks.push((i, tools[sd].edge(e).name));
        }
    }
    let vnames = crate::boolean::vertex_names(&points, marks, root);
    // closed new edges' own vertices, added after those
    let mut seams: Vec<Name> = Vec::new();
    // a new vertex settles on the point nearest every curve through it; parameters follow it
    {
        let mut through: Vec<Vec<(&Curve, Interval)>> = vec![Vec::new(); points.len()];
        for (c, &k) in cuts.iter().zip(&cut_vertex) {
            if let VKey::New(i) = k
                && let Some((sd, e)) = c.on.edge()
            {
                let edge = tools[sd].edge(e);
                through[i as usize].push((&edge.curve, edge.range));
            }
        }
        for sg in &segs {
            let Some((i0, i1)) = sg.ends else { continue };
            let w = 1e-3 * sg.range.width();
            for (t, k) in [(sg.range.lo, cut_vertex[i0]), (sg.range.hi, cut_vertex[i1])] {
                if let VKey::New(i) = k {
                    through[i as usize].push((&sg.curve, Interval::new(t - w, t + w)));
                }
            }
        }
        for (tc, &k) in triple.iter().zip(&triple_vertex) {
            if let VKey::New(i) = k {
                let sg = &segs[tc.seg];
                let w = 1e-3 * sg.range.width();
                through[i as usize].push((&sg.curve, Interval::new(tc.t - w, tc.t + w)));
            }
        }
        // a vertex only triple points of three faces' surfaces make lies where those meet
        let mut three: Vec<Option<[(Side, FaceId); 3]>> = vec![None; points.len()];
        let mut mixed = vec![false; points.len()];
        for &k in &cut_vertex {
            if let VKey::New(i) = k {
                mixed[i as usize] = true;
            }
        }
        for (tc, &k) in triple.iter().zip(&triple_vertex) {
            let VKey::New(i) = k else { continue };
            let i = i as usize;
            let sg = &segs[tc.seg];
            match tc.by {
                Some(kh) => {
                    let mut fs = [(sg.sides[0], sg.faces[0]), (sg.sides[1], sg.faces[1]), kh];
                    fs.sort_unstable();
                    if three[i].is_some_and(|x| x != fs) {
                        mixed[i] = true;
                    }
                    three[i] = Some(fs);
                }
                None => mixed[i] = true,
            }
        }
        let settled = par_map(points.len(), threads, 64, |i| {
            if !mixed[i]
                && let Some(fs) = three[i]
                && let Some(p) = meet3(fs.map(|(sd, f)| &tools[sd].face(f).surface), points[i])
            {
                return Ok((p, tol::LINEAR));
            }
            settle(points[i], &through[i])
        });
        points.clear();
        for r in settled {
            let (p, t) = r?;
            points.push(p);
            point_tols.push(t);
        }
    }
    let key_point = |k: VKey| match k {
        VKey::Old(sd, v) => tools[sd].vertex(v).point,
        VKey::New(i) => points[i as usize],
    };
    // parameters follow the settled points (each its own Newton solve: across threads)
    let at_cut = par_map(cuts.len(), threads, 64, |i| {
        let c = &cuts[i];
        let p = key_point(cut_vertex[i]);
        let s = match c.on {
            CutOn::Free => c.s,
            CutOn::Edge(sd, e) => {
                let edge = tools[sd].edge(e);
                edge.curve.closest(p, edge.range)
            }
        };
        (p, s)
    });
    for (c, (p, s)) in cuts.iter_mut().zip(at_cut) {
        c.point = p;
        c.s = s;
    }
    let at_triple = par_map(triple.len(), threads, 64, |i| {
        let tc = &triple[i];
        let sg = &segs[tc.seg];
        let w = 1e-3 * sg.range.width();
        let p = key_point(triple_vertex[i]);
        // a vertex where the cut was found is there already
        if p == tc.point {
            return (p, tc.t);
        }
        (p, sg.curve.closest(p, Interval::new(tc.t - w, tc.t + w)))
    });
    for (tc, (p, t)) in triple.iter_mut().zip(at_triple) {
        tc.point = p;
        tc.t = t;
    }
    let ranges = par_map(segs.len(), threads, 64, |si| {
        let sg = &segs[si];
        let (i0, i1) = sg.ends?;
        let w = 1e-3 * sg.range.width();
        let lo = sg.curve.closest(
            key_point(cut_vertex[i0]),
            Interval::new(sg.range.lo - w, sg.range.lo + w),
        );
        let hi = sg.curve.closest(
            key_point(cut_vertex[i1]),
            Interval::new(sg.range.hi - w, sg.range.hi + w),
        );
        Some(Interval::new(lo, hi))
    });
    for (sg, r) in segs.iter_mut().zip(ranges) {
        if let Some(r) = r {
            sg.range = r;
        }
    }
    let mut grown = grown_tols(tools, &cuts, &cut_vertex, &segs, &alias, key_point);
    for (tc, &k) in triple.iter().zip(&triple_vertex) {
        if let VKey::Old(sd, v) = k {
            let d = segs[tc.seg].curve.eval(tc.t).distance(tc.point);
            if d > tools[sd].vertex(v).tol {
                grown.push((k, d));
            }
        }
    }
    grown.sort_unstable_by_key(|g| g.0);

    // edge pieces on demand, new-edge pieces now
    // ways out of a vertex share a tangent only where a curve met an edge without crossing it
    let mut ties: Vec<VKey> = cuts
        .iter()
        .zip(&cut_vertex)
        .filter(|(c, _)| c.touch)
        .map(|(_, &k)| k)
        .collect();
    if !ties.is_empty() {
        ties.sort_unstable();
        let aliased: Vec<VKey> = alias
            .iter()
            .filter(|(_, t)| ties.binary_search(t).is_ok())
            .map(|&(f, _)| f)
            .collect();
        ties.extend(aliased);
        ties.sort_unstable();
    }
    let mut by_edge: Vec<(Side, EdgeId, usize)> = cuts
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.on.edge().map(|(sd, e)| (sd, e, i)))
        .collect();
    by_edge.sort_unstable();
    let mut pieces = Pieces {
        tools,
        cuts: &cuts,
        by_edge: &by_edge,
        cut_vertex: &cut_vertex,
        alias: &alias,
        list: Vec::new(),
        of_edge: tools.iter().map(|b| vec![(0, 0); b.edges.len()]).collect(),
    };
    let mut at_seg: Vec<Vec<(f64, VKey)>> = vec![Vec::new(); segs.len()];
    for (tc, &k) in triple.iter().zip(&triple_vertex) {
        at_seg[tc.seg].push((tc.t, k));
    }
    let mut seg_pieces: Vec<std::ops::Range<usize>> = Vec::with_capacity(segs.len());
    for (si, sg) in segs.iter().enumerate() {
        let first = pieces.list.len();
        let mut marks = std::mem::take(&mut at_seg[si]);
        // a piece between marks is named by its ends, not its place along the curve
        let vname = |v: VKey| match v {
            VKey::Old(sd, id) => tools[sd].vertex(id).name,
            VKey::New(i) => vnames[i as usize],
        };
        let push = |list: &mut Vec<Piece>, lo: (f64, VKey), hi: (f64, VKey), part: bool| {
            list.push(Piece {
                curve: sg.curve.clone(),
                range: Interval::new(lo.0, hi.0),
                start: lo.1,
                end: hi.1,
                name: if part {
                    sg.name
                        .with_str("part")
                        .with_name(vname(lo.1))
                        .with_name(vname(hi.1))
                } else {
                    sg.name
                },
                seg: Some(si),
                whole: None,
                side: None,
                of: None,
            });
        };
        match sg.ends {
            Some((i0, i1)) => {
                let (v0, v1) = (cut_vertex[i0], cut_vertex[i1]);
                let w = sg.range.width();
                marks.retain(|&(t, v)| {
                    v != v0 && v != v1 && t > sg.range.lo + 1e-9 * w && t < sg.range.hi - 1e-9 * w
                });
                marks.sort_by(|x, y| x.0.total_cmp(&y.0));
                marks.dedup_by(|x, y| x.1 == y.1);
                if marks.is_empty() {
                    push(&mut pieces.list, (sg.range.lo, v0), (sg.range.hi, v1), false);
                } else {
                    let all: Vec<(f64, VKey)> = std::iter::once((sg.range.lo, v0))
                        .chain(marks)
                        .chain(std::iter::once((sg.range.hi, v1)))
                        .collect();
                    for w in all.windows(2) {
                        push(&mut pieces.list, w[0], w[1], true);
                    }
                }
            }
            None => {
                let period = sg.range.width();
                for m in &mut marks {
                    m.0 = sg.range.lo + (m.0 - sg.range.lo).rem_euclid(period);
                }
                marks.sort_by(|x, y| x.0.total_cmp(&y.0));
                marks.dedup_by(|x, y| x.1 == y.1);
                if marks.len() > 1 && marks[0].1 == marks[marks.len() - 1].1 {
                    marks.pop();
                }
                if marks.is_empty() {
                    // a closed new edge carries its own vertex
                    points.push(sg.curve.eval(sg.range.lo));
                    point_tols.push(tol::LINEAR);
                    seams.push(sg.name.with_str("seam"));
                    let v = VKey::New(points.len() as u32 - 1);
                    push(&mut pieces.list, (sg.range.lo, v), (sg.range.hi, v), false);
                } else {
                    let m = marks.len();
                    for k in 0..m {
                        let hi = if k + 1 == m {
                            (marks[0].0 + period, marks[0].1)
                        } else {
                            marks[k + 1]
                        };
                        push(&mut pieces.list, marks[k], hi, true);
                    }
                }
            }
        }
        seg_pieces.push(first..pieces.list.len());
    }
    lap(&mut clock, 2);

    // 4: split each touched tool face by the new edges and imprints on it
    let mut face_segs: Vec<Vec<Vec<usize>>> = tools.iter().map(|b| vec![Vec::new(); b.faces.len()]).collect();
    for (si, sg) in segs.iter().enumerate() {
        for k in 0..2 {
            face_segs[sg.sides[k]][sg.faces[k].index()].push(si);
        }
    }
    let mut face_imprints: Vec<Vec<Vec<usize>>> =
        tools.iter().map(|b| vec![Vec::new(); b.faces.len()]).collect();
    for (ii, im) in imprints.iter().enumerate() {
        face_imprints[im.into_side][im.into.index()].push(ii);
    }
    let mut cut_faces: Vec<Vec<bool>> = tools.iter().map(|b| vec![false; b.faces.len()]).collect();
    for &(sd, e, _) in &by_edge {
        for &c in &tools[sd].edge(e).coedges {
            cut_faces[sd][tools[sd].coedge_face(c).index()] = true;
        }
    }
    // the edges the jobs below walk, split across threads up front, in the order the walk would
    // first ask for them (so each edge's pieces land where they would have)
    {
        let mut seen: Vec<Vec<bool>> = tools.iter().map(|b| vec![false; b.edges.len()]).collect();
        let mut order: Vec<(Side, EdgeId)> = Vec::new();
        let mut ask = |sd: Side, e: EdgeId| {
            if !std::mem::replace(&mut seen[sd][e.index()], true) {
                order.push((sd, e));
            }
        };
        for (side, &body) in tools.iter().enumerate() {
            for f in body.face_ids() {
                let ims = &face_imprints[side][f.index()];
                if face_segs[side][f.index()].is_empty() && ims.is_empty() && !cut_faces[side][f.index()] {
                    continue;
                }
                for &ii in ims {
                    ask(imprints[ii].side, imprints[ii].edge);
                }
                for &l in &body.face(f).loops {
                    for c in body.lp(l).coedges() {
                        ask(side, body.coedge(c).edge);
                    }
                }
            }
        }
        // on one thread, straight into the list
        if threads.min(order.len() / 32) <= 1 {
            for (sd, e) in order {
                pieces.ensure(sd, e);
            }
        } else {
            let split = par_map(order.len(), threads, 32, |k| {
                let (sd, e) = order[k];
                let mut list = Vec::new();
                edge_split(tools[sd], sd, e, &cuts, &by_edge, &cut_vertex, &alias, &mut list);
                list
            });
            for ((sd, e), got) in order.into_iter().zip(split) {
                let first = pieces.list.len();
                pieces.of_edge[sd][e.index()] = (first as u32, got.len() as u32);
                pieces.list.extend(got);
            }
        }
    }
    // (side, face, boundary, new edges) per touched face
    type Job = (Side, FaceId, Vec<Walk>, Vec<usize>);
    let mut jobs: Vec<Job> = Vec::new();
    let mut regs: Vec<Reg> = Vec::new();
    for (side, &body) in tools.iter().enumerate() {
        for f in body.face_ids() {
            let mine = &face_segs[side][f.index()];
            let ims = &face_imprints[side][f.index()];
            if mine.is_empty() && ims.is_empty() && !cut_faces[side][f.index()] {
                regs.push(Reg {
                    side,
                    face: f,
                    loops: Vec::new(),
                    name: body.face(f).name,
                    job: None,
                });
                continue;
            }
            let mut new_edges: Vec<usize> = mine.iter().flat_map(|&si| seg_pieces[si].clone()).collect();
            for &ii in ims {
                let im = &imprints[ii];
                for p in pieces.ensure(im.side, im.edge) {
                    if im.range.contains(pieces.list[p].range.mid()) && !new_edges.contains(&p) {
                        new_edges.push(p);
                    }
                }
            }
            let boundary = pieces.walks(side, f);
            jobs.push((side, f, boundary, new_edges));
        }
    }
    // two tools' faces on one surface crossing a third give the same new edge twice: one copy
    let mut all: Vec<usize> = jobs
        .iter()
        .flat_map(|j| j.2.iter().flatten().map(|h| h.0).chain(j.3.iter().copied()))
        .collect();
    all.sort_unstable();
    all.dedup();
    let canon = coincident(&pieces.list, &all);
    if canon.iter().any(Option::is_some) {
        for job in &mut jobs {
            for h in job.2.iter_mut().flatten() {
                if let Some((q, f)) = canon[h.0] {
                    *h = (q, h.1 != f);
                }
            }
            let bound: Vec<usize> = job.2.iter().flatten().map(|h| h.0).collect();
            for p in &mut job.3 {
                *p = canon[*p].map_or(*p, |c| c.0);
            }
            job.3.sort_unstable();
            job.3.dedup();
            job.3.retain(|p| !bound.contains(p));
        }
    }
    // a new edge on a face divides it only where the face's verdict differs across it: the
    // sides differ just in the other tool of the edge, so its midpoint (clear of every other tool)
    // shows whether it can (the merge below drops what this leaves)
    if let Some(sh) = shapes {
        let list = &pieces.list;
        let mut shared: Vec<(Side, FaceId)> = coplanar.iter().flatten().copied().collect();
        shared.sort_unstable();
        // the tools other than the edge's own two holding the middle of piece p, in order (with
        // no intersect only the last matters; None: one is too near to tell), once a piece
        let held: Vec<std::sync::OnceLock<Option<Vec<Side>>>> =
            (0..list.len()).map(|_| std::sync::OnceLock::new()).collect();
        let needed = |side: Side, p: usize| -> bool {
            let Some(si) = list[p].seg else { return true };
            let sg = &segs[si];
            let Some(at) = sg.sides.iter().position(|&s| s == side) else {
                return true;
            };
            let other = sg.sides[1 - at];
            let holds = held[p].get_or_init(|| {
                let m = list[p].curve.eval(list[p].range.mid());
                let mut holds: Vec<Side> = Vec::new();
                // with no intersect the last holder decides, whatever the tools before it: walked
                // from the last tool, the first that holds the point ends the walk
                for &k in grid.at(m).iter().rev() {
                    let k = k as usize;
                    if sg.sides.contains(&k) || !index[k].bbox.contains(m) {
                        continue;
                    }
                    match sh[k].member(m, SHAPE_MARGIN) {
                        Member::In if any_intersect => holds.push(k),
                        Member::In => return Some(vec![k]),
                        Member::Out => {}
                        Member::Near => return None,
                    }
                }
                holds.reverse();
                Some(holds)
            });
            holds
                .as_ref()
                .is_none_or(|h| divides(ops, any_intersect, h, side, other))
        };
        let kept: Vec<Vec<usize>> = par_map(jobs.len(), threads, 8, |j| {
            let (side, f, ref boundary, ref new_edges) = jobs[j];
            if new_edges.is_empty() || shared.binary_search(&(side, f)).is_ok() {
                return new_edges.clone();
            }
            let mut keep: Vec<usize> = new_edges.iter().copied().filter(|&p| needed(side, p)).collect();
            if keep.len() == new_edges.len() {
                return keep;
            }
            // an edge kept where its midpoint was too near a tool to tell may now end in the open:
            // it divides nothing, so it goes, and so may the one it hung from
            let ends = |p: usize| [list[p].start, list[p].end];
            let mut degree: Vec<(VKey, u32)> = boundary
                .iter()
                .flatten()
                .flat_map(|&(p, _)| ends(p))
                .chain(keep.iter().flat_map(|&p| ends(p)))
                .map(|v| (v, 1))
                .collect();
            degree.sort_unstable_by_key(|x| x.0);
            degree.dedup_by(|x, y| {
                let same = x.0 == y.0;
                if same {
                    y.1 += x.1;
                }
                same
            });
            let at = |d: &[(VKey, u32)], v: VKey| d.binary_search_by_key(&v, |x| x.0).expect("an end");
            loop {
                let before = keep.len();
                keep.retain(|&p| {
                    let [a, b] = ends(p);
                    let (ia, ib) = (at(&degree, a), at(&degree, b));
                    let dangles = a != b && (degree[ia].1 == 1 || degree[ib].1 == 1);
                    if dangles {
                        degree[ia].1 -= 1;
                        degree[ib].1 -= 1;
                    }
                    !dangles
                });
                if keep.len() == before {
                    break keep;
                }
            }
        });
        for (job, k) in jobs.iter_mut().zip(kept) {
            job.3 = k;
        }
    }
    let list = &pieces.list;
    // with no intersect, a face's region inside a later tool is dropped whatever the ops: that
    // tool's material (or its absence) lies on both sides of it
    let any_intersect = ops.contains(&Op::Intersect);
    let seg_list = &segs;
    let gone = move |side: Side, f: FaceId| {
        move |h: (usize, bool)| {
            let (other, l) = lean(tools, seg_list, list, side, f, h)?;
            (!any_intersect && other > side && l < -1e-3).then_some(true)
        }
    };
    let splits = par_map(jobs.len(), threads, 8, |j| {
        let (side, f, ref boundary, ref new_edges) = jobs[j];
        match (!cut_faces[side][f.index()])
            .then(|| holes_only(tools[side], side, f, list, boundary, new_edges, &ties))
            .flatten()
        {
            Some(regions) => Ok(regions),
            None => split_face(
                tools[side],
                f,
                list,
                boundary,
                new_edges,
                true,
                &ties,
                &gone(side, f),
            )
            .map(|s| s.regions),
        }
    });
    let mut job_regs: Vec<std::ops::Range<usize>> = Vec::with_capacity(jobs.len());
    for (j, ((side, f, _, _), split)) in jobs.iter().zip(splits).enumerate() {
        let first = regs.len();
        for loops in split? {
            regs.push(Reg {
                side: *side,
                face: *f,
                // named once kept
                name: Name::NONE,
                loops,
                job: Some(j),
            });
        }
        job_regs.push(first..regs.len());
    }
    lap(&mut clock, 3);

    // 5: a point of each region, just outside and just inside its tool: kept where the
    // expression differs between them
    // each face's coincident partners, in the order they were found
    let mut partners: Vec<FacePair> = coplanar.iter().flat_map(|&[x, y]| [(x, y), (y, x)]).collect();
    partners.sort_by_key(|x| x.0);
    // whether a region lies inside tool k, by how it leans off k's face along a new edge they share
    let geom: Vec<std::sync::OnceLock<(f64, Reach)>> =
        (0..list.len()).map(|_| std::sync::OnceLock::new()).collect();
    let lean = |r: &Reg, k: Side| -> Option<bool> {
        let body = tools[r.side];
        for &(p, fwd) in r.loops.iter().flatten() {
            let Some(si) = list[p].seg else { continue };
            let sg = &segs[si];
            let Some(at) = sg.sides.iter().position(|&s| s == k) else {
                continue;
            };
            let piece = &list[p];
            let d = piece.curve.deriv(piece.range.mid());
            let along = if fwd { d.d1 } else { -d.d1 };
            let face = body.face(r.face);
            let n_here = body.face_normal(r.face, face.surface.project(d.p))?;
            let inward = n_here.cross(along);
            let (of, ob) = (sg.faces[at], tools[k]);
            let n_other = ob.face_normal(of, ob.face(of).surface.project(d.p))?;
            let l = inward.dot(n_other.get());
            if l.abs() > 1e-9 * inward.length() {
                return Some(l < 0.0);
            }
        }
        None
    };
    // `first`: the region's first point, if already placed
    let full = |ri: usize, first: Option<Point3>| -> R<Option<bool>> {
        let r = &regs[ri];
        let body = tools[r.side];
        let face = body.face(r.face);
        let t0 = std::time::Instant::now();
        let mut candidates = match first {
            Some(p) => vec![p],
            None => region_points(body, r.face, &r.loops, list, &geom, 1),
        };
        if candidates
            .first()
            .is_some_and(|&p| outside(&forge_math::Box3::from_point(p)))
        {
            return Ok(None);
        }
        PICK.fetch_add(t0.elapsed().as_nanos() as usize, Ordering::Relaxed);
        // which tools hold a point just inside / just outside the region there, and whether a lower
        // tool's face lies on it; Err(k) when the point is on tool k's boundary but not on a face of
        // k lying on this one (a tangency or an edge of k): another point must decide
        let judge = |p: Point3| -> R<Result<Seen, Side>> {
            let normal = body
                .face_normal(r.face, face.surface.project(p))
                .ok_or(BoolError::Internal("no normal at a region's point"))?;
            let mut inner = vec![false; n];
            let mut outer = vec![false; n];
            inner[r.side] = true;
            let mut on_face = false;
            for k in 0..n {
                if k == r.side || !index[k].bbox.contains(p) {
                    continue;
                }
                if let Some(sh) = shapes {
                    match sh[k].member(p, SHAPE_MARGIN) {
                        Member::In => {
                            inner[k] = true;
                            outer[k] = true;
                            continue;
                        }
                        Member::Out => continue,
                        Member::Near => {}
                    }
                }
                match index[k].classify(p) {
                    Where::Inside => {
                        inner[k] = true;
                        outer[k] = true;
                    }
                    Where::Outside => {}
                    Where::On => {
                        let from = partners.partition_point(|x| x.0 < (r.side, r.face));
                        let on = partners[from..]
                            .iter()
                            .take_while(|x| x.0 == (r.side, r.face))
                            .map(|x| x.1)
                            .find(|&(sd, g)| sd == k && index[k].point_in_face(g, p) != Where::Outside);
                        let Some((_, g)) = on else {
                            return Ok(Err(k));
                        };
                        let other = tools[k];
                        let m = other
                            .face_normal(g, other.face(g).surface.project(p))
                            .ok_or(BoolError::Internal("no normal at a region's point"))?;
                        on_face = true;
                        // the other tool's material is behind its face
                        if m.dot(normal.get()) > 0.0 {
                            inner[k] = true;
                        } else {
                            outer[k] = true;
                        }
                    }
                }
            }
            Ok(Ok((inner, outer, on_face)))
        };
        let mut found = None;
        let mut stuck: Vec<Side> = Vec::new();
        // the first point nearly always settles it; the rest are worked out only if not
        for pass in 0..2 {
            if pass == 1 {
                candidates = region_points(body, r.face, &r.loops, list, &geom, usize::MAX);
            }
            for &p in candidates.iter().skip(pass) {
                match judge(p)? {
                    Ok(v) => {
                        found = Some((p, v));
                        break;
                    }
                    Err(k) => stuck.push(k),
                }
            }
            if found.is_some() || candidates.is_empty() {
                break;
            }
        }
        let (p, (inner, outer, on_face)) = match found {
            Some(x) => x,
            None => {
                // near-tangent faces: the whole region is within tolerance of tool k; which side of
                // k it is on shows where it meets k, by its lean off k's face there
                let p = *candidates
                    .first()
                    .ok_or(BoolError::Internal("no interior point"))?;
                let mut v = None;
                if let Ok(Err(k)) = judge(p)
                    && let Some(inside) = lean(r, k)
                {
                    // the rest of the tools as seen from p, tool k by the lean
                    let mut inner = vec![false; n];
                    let mut outer = vec![false; n];
                    inner[r.side] = true;
                    let mut ok = true;
                    for j in 0..n {
                        if j == r.side || !index[j].bbox.contains(p) {
                            continue;
                        }
                        if j == k {
                            inner[j] = inside;
                            outer[j] = inside;
                            continue;
                        }
                        match index[j].classify(p) {
                            Where::Inside => {
                                inner[j] = true;
                                outer[j] = true;
                            }
                            Where::Outside => {}
                            Where::On => ok = false,
                        }
                    }
                    if ok {
                        v = Some((inner, outer, false));
                    }
                }
                match v {
                    Some(v) => (p, v),
                    None => {
                        if debug {
                            eprintln!(
                                "nary: region {ri} of tool {} face {:?} probes {candidates:?} on tools {stuck:?}",
                                r.side, r.face
                            );
                        }
                        return Err(BoolError::Unsupported("region probe on another tool's edge"));
                    }
                }
            }
        };
        let (fin, fout) = (
            in_result(ops, &inner, any_intersect),
            in_result(ops, &outer, any_intersect),
        );
        if nary_reg() == Some(ri) {
            for (li, l) in r.loops.iter().enumerate() {
                for &(pc, fwd) in l {
                    let q = &list[pc];
                    eprintln!(
                        "  loop {li} piece {pc} fwd {fwd} {:?}->{:?} seg {:?} side {:?} mid {:?}",
                        q.start,
                        q.end,
                        q.seg,
                        q.side,
                        q.curve.eval(q.range.mid())
                    );
                }
            }
        }
        if nary_on() {
            let on = |b: &[bool]| (0..n).filter(|&k| b[k]).collect::<Vec<_>>();
            eprintln!(
                "REG {ri}: tool {} face {:?} probe {p:?} in {:?} out {:?} -> {fin} {fout}",
                r.side,
                r.face,
                on(&inner),
                on(&outer)
            );
        }
        // only where faces coincide can another tool's face be the one kept
        Ok((fin != fout && (!on_face || owned(ops, &inner, &outer, r.side))).then_some(!fin))
    };
    // regions of one tool meeting across a piece of its own edge lie in the same other tools (a
    // third tool's face crossing the edge cuts it there), so share a verdict: only the first of
    // each group is placed. Faces that meet another tool's face otherwise than across a new edge
    // (coincident, touching, left out unintersected) are each placed on their own
    let mut unclean: Vec<(Side, FaceId)> = coplanar
        .iter()
        .flatten()
        .copied()
        .chain(loose.iter().flat_map(|&(x, y)| [x, y]))
        .chain(
            touching
                .iter()
                .flat_map(|t| [(t.sides[0], t.faces[0]), (t.sides[1], t.faces[1])]),
        )
        .collect();
    unclean.sort_unstable();
    unclean.dedup();
    let mut unpaired = vec![false; n];
    for &(i, j) in &unknown {
        unpaired[i] = true;
        unpaired[j] = true;
    }
    let mut links: Vec<((Side, EdgeId, u32), usize)> = Vec::new();
    for (ri, r) in regs.iter().enumerate() {
        if unpaired[r.side] || unclean.binary_search(&(r.side, r.face)).is_ok() {
            continue;
        }
        let body = tools[r.side];
        if r.loops.is_empty() {
            // an untouched face's edges are uncut: one piece each
            for &l in &body.face(r.face).loops {
                for c in body.lp(l).coedges() {
                    links.push(((r.side, body.coedge(c).edge, 0), ri));
                }
            }
            continue;
        }
        for &(p, _) in r.loops.iter().flatten() {
            if let (None, Some(sd), Some(e)) = (list[p].seg, list[p].side, list[p].of)
                && sd == r.side
            {
                let first = pieces.of_edge[sd][e.index()].0 as usize;
                links.push(((sd, e, (p - first) as u32), ri));
            }
        }
    }
    links.sort_unstable();
    let mut group: Vec<usize> = (0..regs.len()).collect();
    for g in links.chunk_by(|x, y| x.0 == y.0) {
        for w in g.windows(2) {
            let (x, y) = (find(&mut group, w[0].1), find(&mut group, w[1].1));
            group[x.max(y)] = x.min(y);
        }
    }
    let group: Vec<usize> = (0..regs.len()).map(|ri| find(&mut group, ri)).collect();
    let class = match batch {
        None => par_map(regs.len(), threads, 32, |ri| {
            if group[ri] != ri {
                return Ok(None);
            }
            full(ri, None)
        }),
        Some(batch) => {
            // one point per region, all placed at once; the rest decided the full way
            batch.warm();
            let first: Vec<Option<Point3>> = par_map(regs.len(), threads, 32, |ri| {
                if group[ri] != ri {
                    return None;
                }
                let r = &regs[ri];
                region_points(tools[r.side], r.face, &r.loops, list, &geom, 1)
                    .first()
                    .copied()
            });
            let mut pts = Vec::with_capacity(regs.len());
            let mut sides = Vec::with_capacity(regs.len());
            let mut at: Vec<Option<usize>> = vec![None; regs.len()];
            for (ri, p) in first.iter().enumerate() {
                if let Some(p) = p {
                    at[ri] = Some(pts.len());
                    pts.push(*p);
                    sides.push(regs[ri].side as u32);
                }
            }
            let boxes: Vec<forge_math::Box3> = index.iter().map(|x| x.bbox).collect();
            let t = std::time::Instant::now();
            let codes = batch.verdicts(&pts, &sides, &boxes);
            PHASES.with(|p| p.borrow_mut()[8] = t.elapsed().as_nanos() as u64);
            par_map(regs.len(), threads, 32, |ri| match at[ri].map(|i| codes[i]) {
                _ if group[ri] != ri => Ok(None),
                Some(0) => Ok(None),
                Some(1) => Ok(Some(false)),
                Some(2) => Ok(Some(true)),
                _ => full(ri, first[ri]),
            })
        }
    };
    let mut class: Vec<Option<bool>> = class.into_iter().collect::<R<_>>()?;
    for ri in 0..regs.len() {
        class[ri] = class[group[ri]];
    }
    let merge_start = std::time::Instant::now();

    // a new edge with the same verdict on both sides divides nothing: the face is split again
    // without it, each new region taking the verdict of the old ones it is made of. The edge of
    // a later tool's coincident face imprinted between two kept regions stays, as in the
    // sequential regen (the face was there first and took the imprint)
    let merged = par_map(jobs.len(), threads, 16, |j| -> R<Option<Vec<Judged>>> {
        let rr = job_regs[j].clone();
        // nothing of a face all dropped is sewn, however it is split
        if rr.len() < 2 || rr.clone().all(|ri| class[ri].is_none()) {
            return Ok(None);
        }
        let (side, f, ref boundary, ref new_edges) = jobs[j];
        // the region each new edge's half bounds (and each half of the face's edges not of its
        // tool), sorted for lookup
        let mut owners: Vec<((usize, bool), usize)> = rr
            .clone()
            .flat_map(|ri| regs[ri].loops.iter().flatten().map(move |&h| (h, ri)))
            .filter(|(h, _)| list[h.0].seg.is_some() || list[h.0].side != Some(side))
            .collect();
        owners.sort_unstable();
        let owner = |h: (usize, bool)| owners.binary_search_by(|x| x.0.cmp(&h)).ok().map(|i| owners[i].1);
        let keep: Vec<usize> = new_edges
            .iter()
            .copied()
            .filter(|&p| match (owner((p, true)), owner((p, false))) {
                (Some(a), Some(b)) => {
                    a != b
                        && (class[a] != class[b]
                            || class[a].is_some()
                                && list[p].seg.is_none()
                                && list[p].side.is_some_and(|k| k > side))
                }
                _ => true,
            })
            .collect();
        if keep.len() == new_edges.len() {
            return Ok(None);
        }
        let regions = match (!cut_faces[side][f.index()])
            .then(|| holes_only(tools[side], side, f, list, boundary, &keep, &ties))
            .flatten()
        {
            Some(regions) => regions,
            None => split_face(tools[side], f, list, boundary, &keep, true, &ties, &gone(side, f))?.regions,
        };
        regions
            .into_iter()
            .map(|loops| {
                let first = *loops
                    .first()
                    .and_then(|l| l.first())
                    .ok_or(BoolError::Internal("empty region"))?;
                let old = owner(first)
                    .or_else(|| {
                        rr.clone()
                            .find(|&ri| regs[ri].loops.iter().flatten().any(|&h| h == first))
                    })
                    .ok_or(BoolError::Internal("merged region of no old one"))?;
                Ok((loops, class[old]))
            })
            .collect::<R<Vec<_>>>()
            .map(Some)
    });
    let mut replaced = vec![false; jobs.len()];
    let mut extra: Vec<(Reg, Option<bool>)> = Vec::new();
    for (j, m) in merged.into_iter().enumerate() {
        let Some(m) = m? else { continue };
        replaced[j] = true;
        let (side, f, _, _) = jobs[j];
        for (loops, c) in m {
            extra.push((
                Reg {
                    side,
                    face: f,
                    name: Name::NONE,
                    loops,
                    job: Some(j),
                },
                c,
            ));
        }
    }
    if !extra.is_empty() {
        let old = std::mem::take(&mut regs);
        let old_class = std::mem::take(&mut class);
        for (r, c) in old.into_iter().zip(old_class) {
            if !r.job.is_some_and(|j| replaced[j]) {
                regs.push(r);
                class.push(c);
            }
        }
        for (r, c) in extra {
            regs.push(r);
            class.push(c);
        }
    }
    let merge_ns = merge_start.elapsed().as_nanos() as u64;
    lap(&mut clock, 4);
    PHASES.with(|p| {
        let mut p = p.borrow_mut();
        p[6] = PICK.swap(0, Ordering::Relaxed) as u64;
        p[7] = merge_ns;
    });

    // 6: sew the kept regions; pieces with the same ends along the same curve are one edge
    let mut kept: Vec<(usize, bool)> = Vec::new();
    for (ri, c) in class.iter().enumerate() {
        if let Some(flip) = *c {
            if nary_on() {
                eprintln!("KEPT {} = reg {ri} flip {flip}", kept.len());
            }
            kept.push((ri, flip));
        }
    }
    for &(ri, _) in &kept {
        if regs[ri].loops.is_empty() {
            let walks = pieces.walks(regs[ri].side, regs[ri].face);
            regs[ri].loops = walks;
        }
    }
    let mut list = pieces.list;
    let mut used: Vec<usize> = kept
        .iter()
        .flat_map(|&(ri, _)| regs[ri].loops.iter().flatten().map(|h| h.0))
        .collect();
    used.sort_unstable();
    used.dedup();
    let canon = coincident(&list, &used);
    // an edge four kept faces meet at is where two solids' material touches along a line
    let mut uses = vec![0u8; list.len()];
    for &(ri, _) in &kept {
        for &(p, _) in regs[ri].loops.iter().flatten() {
            let p = canon[p].map_or(p, |c| c.0);
            uses[p] = uses[p].saturating_add(1);
            if uses[p] > 2 {
                let pc = &list[p];
                return Err(BoolError::ZeroThickness {
                    at: pc.curve.eval(pc.range.mid()),
                });
            }
        }
    }
    // a face or edge only one piece of which is kept keeps its own name: it was only trimmed;
    // other pieces are named by their ends, and regions by their boundaries
    let vname = |v: VKey| match v {
        VKey::Old(sd, id) => tools[sd].vertex(id).name,
        VKey::New(i) => vnames
            .get(i as usize)
            .copied()
            .unwrap_or_else(|| seams[i as usize - vnames.len()]),
    };
    let split: Vec<(usize, (Side, EdgeId))> = used
        .iter()
        .filter_map(|&p| Some((p, (list[p].side?, list[p].of?))))
        .collect();
    let keys: Vec<(Side, EdgeId)> = split.iter().map(|x| x.1).collect();
    let mut wholes = Vec::new();
    for (&(p, (sd, e)), sole) in split.iter().zip(crate::boolean::alone(&keys)) {
        let edge = tools[sd].edge(e).name;
        list[p].name = if sole {
            edge
        } else {
            let n = crate::boolean::piece_name(edge, &list[p], &vname);
            wholes.push((n, edge));
            n
        };
    }
    let split: Vec<usize> = kept
        .iter()
        .map(|k| k.0)
        .filter(|&ri| regs[ri].job.is_some())
        .collect();
    let faces: Vec<(Side, FaceId)> = split.iter().map(|&ri| (regs[ri].side, regs[ri].face)).collect();
    for (&ri, sole) in split.iter().zip(crate::boolean::alone(&faces)) {
        let face = tools[regs[ri].side].face(regs[ri].face).name;
        regs[ri].name = if sole {
            face
        } else {
            let n = crate::boolean::region_name(face, &regs[ri].loops, &list);
            wholes.push((n, face));
            n
        };
    }
    let mut out = Body::new();
    // sized up front: the body grows by pushes alone
    let (loops, uses): (usize, usize) = kept.iter().fold((0, 0), |a, &(ri, _)| {
        let ls = &regs[ri].loops;
        (a.0 + ls.len(), a.1 + ls.iter().map(Vec::len).sum::<usize>())
    });
    out.faces.reserve_exact(kept.len());
    out.loops.reserve_exact(loops);
    out.coedges.reserve_exact(uses);
    out.edges.reserve_exact(uses / 2);
    out.vertices.reserve_exact(uses / 2);
    for t in tools {
        out.joins.extend(&t.joins);
    }
    out.joins.pieces.extend(wholes);
    let shell = out.add_shell();
    // the body's vertex of each tool vertex (per tool, made on first use) and each new one
    let mut old_v: Vec<Vec<Option<VertexId>>> = vec![Vec::new(); n];
    let mut new_v: Vec<Option<VertexId>> = vec![None; points.len()];
    let mut emap: Vec<Option<EdgeId>> = vec![None; list.len()];
    for &(ri, flip) in &kept {
        let r = &regs[ri];
        let src = tools[r.side].face(r.face);
        let face = out.add_face(shell, src.surface.clone(), src.same_sense != flip, r.name);
        for lp in &r.loops {
            let mut uses: Vec<(EdgeId, bool)> = Vec::with_capacity(lp.len());
            // a flipped face walks its loops backwards; entities are made in the order they are
            // used, so the body comes out compact
            for k in 0..lp.len() {
                let (p, fwd) = lp[if flip { lp.len() - 1 - k } else { k }];
                let fwd = fwd != flip;
                let (p, fwd) = match canon[p] {
                    Some((q, f)) => (q, fwd != f),
                    None => (p, fwd),
                };
                let e = match emap[p] {
                    Some(e) => e,
                    None => {
                        let pc = &list[p];
                        let mut vertex = |out: &mut Body, v: VKey| -> VertexId {
                            let slot = match v {
                                VKey::Old(sd, id) => {
                                    if old_v[sd].is_empty() {
                                        old_v[sd] = vec![None; tools[sd].vertices.len()];
                                    }
                                    &mut old_v[sd][id.index()]
                                }
                                VKey::New(i) => &mut new_v[i as usize],
                            };
                            *slot.get_or_insert_with(|| match v {
                                VKey::Old(sd, id) => {
                                    let x = tools[sd].vertex(id);
                                    let from = grown.partition_point(|g| g.0 < v);
                                    let t = grown[from..]
                                        .iter()
                                        .take_while(|g| g.0 == v)
                                        .fold(x.tol, |t, g| t.max(g.1));
                                    out.add_tolerant_vertex(x.point, t, x.name)
                                }
                                VKey::New(i) => out.add_tolerant_vertex(
                                    points[i as usize],
                                    point_tols[i as usize],
                                    vnames
                                        .get(i as usize)
                                        .copied()
                                        .unwrap_or_else(|| seams[i as usize - vnames.len()]),
                                ),
                            })
                        };
                        let (v0, v1) = (vertex(&mut out, pc.start), vertex(&mut out, pc.end));
                        let e = out.add_edge(pc.curve.clone(), pc.range, v0, v1, pc.name);
                        emap[p] = Some(e);
                        e
                    }
                };
                uses.push((e, fwd));
            }
            out.add_loop(face, &uses);
        }
    }
    out.rebuild_shells();
    lap(&mut clock, 5);
    Ok(out)
}

/// The point where three surfaces meet, by Newton's method from `p` (None where they meet at a
/// shallow angle there, or not within tolerance of it).
fn meet3(s: [&forge_geom::Surface; 3], p: Point3) -> Option<Point3> {
    if s.iter().any(|x| matches!(x, forge_geom::Surface::Procedural(_))) {
        return None;
    }
    let mut q = p;
    for _ in 0..8 {
        let [(f0, g0), (f1, g1), (f2, g2)] = s.map(|x| x.implicit_gradient(q));
        if f0.abs().max(f1.abs()).max(f2.abs()) <= 1e-12 {
            return Some(q);
        }
        let (c0, c1, c2) = (g1.cross(g2), g2.cross(g0), g0.cross(g1));
        let det = g0.dot(c0);
        if det.abs() <= 1e-3 * g0.length() * g1.length() * g2.length() {
            return None;
        }
        q = q - (c0 * f0 + c1 * f1 + c2 * f2) / det;
        if q.distance(p) > tol::LINEAR {
            return None;
        }
    }
    None
}

/// The regions `split_face` gives a planar face, none of whose edges is cut, whose new edges
/// close into loops clear of each other and of the face's own loops (no shared vertex, no box of
/// one holding another or a vertex of the face): the face with each as a hole, then each as a
/// disk (None otherwise).
fn holes_only(
    body: &Body,
    side: Side,
    f: FaceId,
    list: &[Piece],
    boundary: &[Walk],
    new_edges: &[usize],
    ties: &[VKey],
) -> Option<Vec<Vec<Walk>>> {
    let face = body.face(f);
    if new_edges.is_empty() || !matches!(face.surface, forge_geom::Surface::Plane(_)) {
        return None;
    }
    // each end of a new edge by vertex: every vertex two of them, none the face's tool's or a tie
    let mut ends: Vec<(VKey, usize)> = new_edges
        .iter()
        .enumerate()
        .flat_map(|(k, &p)| [(list[p].start, k), (list[p].end, k)])
        .collect();
    ends.sort_unstable();
    if !ends.chunks(2).all(|c| c[0].0 == c[1].0)
        || ends.windows(3).any(|w| w[0].0 == w[2].0)
        || ends
            .iter()
            .any(|(v, _)| matches!(v, VKey::Old(sd, _) if *sd == side) || ties.binary_search(v).is_ok())
    {
        return None;
    }
    // the other new edge at v
    let other = |v: VKey, k: usize| {
        let i = ends.partition_point(|e| e.0 < v);
        if ends[i].1 == k { ends[i + 1].1 } else { ends[i].1 }
    };
    // the loops as split_face walks them (each from its first edge forwards)
    let mut used = vec![false; new_edges.len()];
    let mut cycles: Vec<Walk> = Vec::new();
    for k in 0..new_edges.len() {
        if used[k] {
            continue;
        }
        used[k] = true;
        let p = new_edges[k];
        let mut cyc = vec![(p, true)];
        let (mut at, mut j) = (list[p].end, k);
        while at != list[p].start {
            j = other(at, j);
            if std::mem::replace(&mut used[j], true) {
                return None;
            }
            let q = new_edges[j];
            let fwd = list[q].start == at;
            at = if fwd { list[q].end } else { list[q].start };
            cyc.push((q, fwd));
        }
        cycles.push(cyc);
    }
    // each loop's box in the plane (from its edges' boxes, grown a little): none may meet another
    // or hold a vertex of the face (a sweep along u)
    let mirror = |p: Point2| {
        if face.same_sense {
            p
        } else {
            Point2::new(-p.x, p.y)
        }
    };
    let forge_geom::Surface::Plane(plane) = &face.surface else {
        return None;
    };
    let (eu, ev) = (plane.frame.x().get(), plane.frame.y().get());
    let boxes: Vec<(f64, f64, f64, f64)> = cycles
        .iter()
        .map(|cyc| {
            let (mut lo, mut hi) = (
                Point2::new(f64::INFINITY, f64::INFINITY),
                Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            );
            let mut add = |c: Point2, du: f64, dv: f64| {
                lo = Point2::new(lo.x.min(c.x - du), lo.y.min(c.y - dv));
                hi = Point2::new(hi.x.max(c.x + du), hi.y.max(c.y + dv));
            };
            for &(q, _) in cyc {
                let pc = &list[q];
                // a conic by its whole outline (o + a cos t x + b sin t y reaches |a x.e| + ..
                // along e, at most), a line by its ends, else by its box's corners
                let conic = |f: &forge_math::Frame, a: f64, b: f64| {
                    let (x, y) = (f.x().get(), f.y().get());
                    let r = |e: forge_math::Vec3| ((a * x.dot(e)).powi(2) + (b * y.dot(e)).powi(2)).sqrt();
                    (mirror(face.surface.project(f.origin)), r(eu), r(ev))
                };
                match &pc.curve {
                    Curve::Circle(k) => {
                        let (c, du, dv) = conic(&k.frame, k.radius, k.radius);
                        add(c, du, dv);
                    }
                    Curve::Ellipse(e) => {
                        let (c, du, dv) = conic(&e.frame, e.major, e.minor);
                        add(c, du, dv);
                    }
                    Curve::Line(_) => {
                        for t in [pc.range.lo, pc.range.hi] {
                            add(mirror(face.surface.project(pc.curve.eval(t))), 0.0, 0.0);
                        }
                    }
                    c => {
                        let b = c.bbox(pc.range);
                        for x in [b.x.lo, b.x.hi] {
                            for y in [b.y.lo, b.y.hi] {
                                for z in [b.z.lo, b.z.hi] {
                                    add(mirror(face.surface.project(Point3::new(x, y, z))), 0.0, 0.0);
                                }
                            }
                        }
                    }
                }
            }
            let g = 1e-3 * (hi.x - lo.x + hi.y - lo.y);
            (lo.x - g, hi.x + g, lo.y - g, hi.y + g)
        })
        .collect();
    let corners: Vec<Point2> = face
        .loops
        .iter()
        .map(|&l| {
            let v = body.edge(body.coedge(body.lp(l).coedge(0)).edge).start;
            mirror(face.surface.project(body.vertex(v).point))
        })
        .collect();
    // boxes before corners at one u
    let mut marks: Vec<(f64, usize)> = boxes.iter().enumerate().map(|(i, b)| (b.0, i)).collect();
    marks.extend(corners.iter().enumerate().map(|(i, q)| (q.x, boxes.len() + i)));
    marks.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut open: Vec<usize> = Vec::new();
    for &(x, i) in &marks {
        open.retain(|&o| boxes[o].1 >= x);
        let (lo, hi) = match boxes.get(i) {
            Some(b) => (b.2, b.3),
            None => (corners[i - boxes.len()].y, corners[i - boxes.len()].y),
        };
        if open.iter().any(|&o| boxes[o].2 <= hi && lo <= boxes[o].3) {
            return None;
        }
        if i < boxes.len() {
            open.push(i);
        }
    }
    // which way round each runs, drawn as split_face draws it
    let mut samples = Vec::new();
    let mut disks: Vec<Walk> = Vec::with_capacity(cycles.len());
    for cyc in cycles {
        samples.clear();
        for &(q, fwd) in &cyc {
            crate::boolean::piece_samples(&list[q], fwd, &mut samples);
        }
        let poly: Vec<Point2> = forge_topo::uv::draw_loop(body, f, &samples)
            .0
            .into_iter()
            .map(mirror)
            .collect();
        let area = forge_topo::uv::signed_area(&poly);
        if area.abs() <= 1e-12 {
            return None;
        }
        disks.push(if area > 0.0 {
            cyc
        } else {
            std::iter::once((cyc[0].0, false))
                .chain(cyc[1..].iter().rev().map(|&(q, fwd)| (q, !fwd)))
                .collect()
        });
    }
    let mut outer: Vec<Walk> = boundary.to_vec();
    let mut regions = Vec::with_capacity(disks.len() + 1);
    regions.push(Vec::new());
    for disk in disks {
        // the hole is the loop walked the other way: its first edge, then the rest backwards
        outer.push(
            std::iter::once((disk[0].0, !disk[0].1))
                .chain(disk[1..].iter().rev().map(|&(q, fwd)| (q, !fwd)))
                .collect(),
        );
        regions.push(vec![disk]);
    }
    regions[0] = outer;
    Some(regions)
}

/// Pieces among `ps` with the same ends along the same curve as a lower one: by piece, (the
/// lower one, whether it runs the other way).
fn coincident(list: &[Piece], ps: &[usize]) -> Vec<Option<(usize, bool)>> {
    let key = |pc: &Piece| (pc.start.min(pc.end), pc.start.max(pc.end));
    let mut by_ends: Vec<((VKey, VKey), usize)> = ps.iter().map(|&p| (key(&list[p]), p)).collect();
    by_ends.sort_unstable();
    let mut canon = vec![None; list.len()];
    for g in by_ends.chunk_by(|x, y| x.0 == y.0) {
        for (k, &(_, p)) in g.iter().enumerate().skip(1) {
            let pc = &list[p];
            let mid = pc.curve.eval(pc.range.mid());
            for &(_, q) in &g[..k] {
                if canon[q].is_some() {
                    continue;
                }
                let o = &list[q];
                if o.curve.eval(o.curve.closest(mid, o.range)).distance(mid) <= tol::LINEAR {
                    canon[p] = Some((q, pc.start != o.start));
                    break;
                }
            }
        }
    }
    canon
}

/// A point on face `f` a step to the left of a curve running along its boundary (walked
/// `fwd`), at fraction `frac` of the range.
fn step_in(
    body: &Body,
    f: FaceId,
    curve: &Curve,
    range: Interval,
    fwd: bool,
    frac: f64,
    step: f64,
) -> Option<Point3> {
    let face = body.face(f);
    let d = curve.deriv(range.lo + range.width() * frac);
    let along = if fwd { d.d1 } else { -d.d1 };
    let n = body.face_normal(f, face.surface.project(d.p))?;
    let inward = n.cross(along).normalize()?;
    let q = d.p + inward * step;
    Some(face.surface.eval(face.surface.project(q)))
}

/// Each piece's chord and reach (as `chord_of`), worked out once.
type Geom = [std::sync::OnceLock<(f64, Reach)>];

/// Where a piece may come near a point: within a distance of a point, or in a box.
#[derive(Clone, Copy)]
enum Reach {
    Ball(Point3, f64),
    Box(forge_math::Box3),
}

/// The curve's chord over `r` (the longer half of it, at least 1e-3) and where it reaches: a
/// line or circle arc lies within its chord of its middle (no point of it is farther than its
/// ends), any other curve in its box.
fn chord_of(c: &Curve, r: Interval) -> (f64, Reach) {
    let m = c.eval(r.mid());
    let d = c.eval(r.lo).distance(m).max(m.distance(c.eval(r.hi)));
    let reach = match c {
        Curve::Line(_) | Curve::Circle(_) => Reach::Ball(m, d),
        _ => Reach::Box(c.bbox(r)),
    };
    (d.max(1e-3), reach)
}

/// Points inside a region of face `f` (its loops of pieces, or the face's own loops when
/// `loops` is empty), best first: a step in from its longest edges at a few places along
/// them, each step short enough that no other edge comes nearer than it (so the way in
/// crosses none).
fn region_points(
    body: &Body,
    f: FaceId,
    loops: &[Walk],
    list: &[Piece],
    geom: &Geom,
    max: usize,
) -> Vec<Point3> {
    // (curve, range, forward, chord, reach) of each edge
    let mut edges: Vec<(&Curve, Interval, bool, f64, Reach)> = Vec::new();
    if loops.is_empty() {
        for &l in &body.face(f).loops {
            for c in body.lp(l).coedges() {
                let co = body.coedge(c);
                let e = body.edge(co.edge);
                let (len, _) = chord_of(&e.curve, e.range);
                edges.push((
                    &e.curve,
                    e.range,
                    co.forward,
                    len,
                    Reach::Box(body.edge_box(co.edge)),
                ));
            }
        }
    } else {
        for &(p, fwd) in loops.iter().flatten() {
            let pc = &list[p];
            let &(len, reach) = geom[p].get_or_init(|| chord_of(&pc.curve, pc.range));
            edges.push((&pc.curve, pc.range, fwd, len, reach));
        }
    }
    let mut order: Vec<(f64, usize)> = edges.iter().enumerate().map(|(i, e)| (e.3, i)).collect();
    order.sort_by(|a, b| b.0.total_cmp(&a.0));
    // clear of every other edge by the step, less a slack under the tolerance: a probe that has
    // crossed an edge a hair away (a near-tolerance sliver) lies a hair nearer it than the step
    let clear = |p: Point3, step: f64, own: usize| {
        let slack = (1e-3 * step).min(0.1 * tol::LINEAR);
        edges.iter().enumerate().all(|(i, &(c, r, _, _, reach))| {
            let far = match reach {
                Reach::Ball(m, d) => m.distance(p) > d + step,
                Reach::Box(b) => !b.grow(step).contains(p),
            };
            i == own || far || c.eval(c.closest(p, r)).distance(p) >= step - slack
        })
    };
    let mut out = Vec::new();
    for &(len, i) in order.iter().take(4) {
        let (c, r, fwd, _, _) = edges[i];
        for frac in [0.5, 0.3, 0.7] {
            let mut step = (0.01 * len).min(0.05);
            while step > tol::LINEAR {
                if let Some(p) = step_in(body, f, c, r, fwd, frac, step)
                    && clear(p, step, i)
                {
                    out.push(p);
                    if out.len() == max {
                        return out;
                    }
                    break;
                }
                step *= 0.25;
            }
        }
    }
    // nothing clear (a sliver thinner than the steps tried): halfway across to the nearest
    // other edge from the longest one's middle
    if out.is_empty()
        && let Some(&(len, i)) = order.first()
    {
        let (c, r, fwd, _, _) = edges[i];
        let at = c.eval(r.mid());
        let gap = edges
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != i)
            .map(|(_, &(o, ro, _, _, _))| o.eval(o.closest(at, ro)).distance(at))
            .fold(f64::INFINITY, f64::min);
        let step = if gap.is_finite() && gap > 0.0 {
            0.5 * gap
        } else {
            0.01 * len
        };
        out.extend(step_in(body, f, c, r, fwd, 0.5, step.min(0.05)));
    }
    out
}
