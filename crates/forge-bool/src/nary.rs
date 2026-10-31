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
    BoolError, Cut, Imprint, Op, Piece, R, Seg, Side, Touch, VKey, Walk, crossings_near, edge_split,
    grown_tols, imprint, local_class, on_edge, overlap, settle, split_face, touches, trim,
};
use crate::classify::{Index, Where};
use crate::shape::{Member, Shape};
use crate::ssi;
use forge_geom::intersect::{Contact, CurveSurface, SurfaceSurface, curve_surface};
use forge_geom::{Curve, CurveGeom, SurfaceGeom};
use forge_math::{Interval, Point3, tol};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};
use std::collections::HashMap;
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

/// `f(0..n)` on up to `threads` threads, each given at least `per` items (a thread costs
/// tens of microseconds to start); results in index order.
fn par_map<T: Send>(n: usize, threads: usize, per: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = threads.min(n / per.max(1));
    if threads <= 1 || n < 2 {
        return (0..n).map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let parts: Vec<Vec<(usize, T)>> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads.min(n))
            .map(|_| {
                s.spawn(|| {
                    let mut got = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= n {
                            break got;
                        }
                        got.push((i, f(i)));
                    }
                })
            })
            .collect();
        workers.into_iter().map(|w| w.join().expect("worker")).collect()
    });
    let mut out: Vec<Option<T>> = (0..n).map(|_| None).collect();
    for (i, t) in parts.into_iter().flatten() {
        out[i] = Some(t);
    }
    out.into_iter().map(|t| t.expect("every item done")).collect()
}

/// How far clear of a tool's boundary a point must be for its analytic solid to answer.
const SHAPE_MARGIN: f64 = tol::LINEAR;

/// Which tools hold a point just inside and just outside a region, and whether a lower tool's
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
}

/// What intersecting one pair of tools gives.
#[derive(Default)]
struct PairOut {
    cuts: Vec<Cut>,
    segs: Vec<Seg>,
    imprints: Vec<Imprint>,
    coplanar: Vec<[(Side, FaceId); 2]>,
    touching: Vec<Touch>,
}

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
            let mut w = Vec::with_capacity(body.lp(l).coedges.len());
            for &c in &body.lp(l).coedges {
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

/// The solid `tools[0] ops[0] tools[1] ops[1] tools[2] ...`, folded left, in one pass.
pub fn nary(tools: &[&Body], ops: &[Op], threads: usize) -> R<Body> {
    nary_with(tools, ops, None, None, threads)
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
        par_map(points.len(), self.threads, 256, |i| {
            let p = points[i];
            let own = sides[i] as usize;
            let (mut vin, mut vout) = (false, false);
            for (k, sh) in self.shapes.iter().enumerate() {
                let (bin, bout) = if k == own {
                    (true, false)
                } else if !boxes[k].contains(p) {
                    (false, false)
                } else {
                    match sh.member(p, SHAPE_MARGIN) {
                        Member::Near => return 3,
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
/// is placed from its profile, and only points near it go to the B-rep.
pub fn nary_with(
    tools: &[&Body],
    ops: &[Op],
    shapes: Option<&[Shape]>,
    batch: Option<&dyn Batch>,
    threads: usize,
) -> R<Body> {
    assert_eq!(ops.len() + 1, tools.len(), "one op per tool after the first");
    let n = tools.len();
    let mut clock = std::time::Instant::now();
    let index: Vec<Index> = tools.iter().map(|b| Index::new(b)).collect();
    let debug = crate::boolean::debug_on();

    // 1: pairs of tools whose boxes meet, then their faces
    let mut pairs: Vec<(Side, Side)> = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            if index[i].bbox.overlaps(&index[j].bbox) {
                pairs.push((i, j));
            }
        }
    }
    let outs = par_map(pairs.len(), threads, 2, |pi| -> R<PairOut> {
        let (i, j) = pairs[pi];
        let (a, b) = (tools[i], tools[j]);
        let (ia, ib) = (&index[i], &index[j]);
        let mut out = PairOut::default();
        for fa in a.face_ids() {
            if !ia.boxes[fa.index()].overlaps(&ib.bbox) {
                continue;
            }
            for fb in b.face_ids() {
                if !ia.boxes[fa.index()].overlaps(&ib.boxes[fb.index()]) {
                    continue;
                }
                let (sa, sb) = (&a.face(fa).surface, &b.face(fb).surface);
                let region = overlap(&ia.boxes[fa.index()], &ib.boxes[fb.index()]).grow(tol::LINEAR);
                let found = ssi::surfaces_in(sa, sb, region);
                out.touching
                    .extend(touches(&found, [ia, ib], [i, j], [fa, fb], region));
                let curves = match found {
                    SurfaceSurface::Empty | SurfaceSurface::Points(_) => continue,
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
                for (ci, (curve, contact)) in curves.into_iter().enumerate() {
                    if contact == Contact::Touching {
                        continue;
                    }
                    let name = Name::feature(0)
                        .with_str("cut")
                        .with_set(vec![a.face(fa).name, b.face(fb).name])
                        .with(ci as u64);
                    trim(
                        [a, b],
                        [ia, ib],
                        [i, j],
                        [fa, fb],
                        curve,
                        name,
                        &mut out.cuts,
                        &mut out.segs,
                        &mut out.imprints,
                    )?;
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
    for out in outs {
        let out = out?;
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
    // where two tools touch, both faces kept would be material meeting itself
    let any_intersect = ops.contains(&Op::Intersect);
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
    let seg_cuts: Vec<Vec<(f64, Point3, f64)>> = par_map(segs.len(), threads, 32, |si| {
        let sg = &segs[si];
        let reach = sg.curve.bbox(sg.range).grow(tol::LINEAR);
        let mut got = Vec::new();
        for k in 0..n {
            if sg.sides.contains(&k) || !index[k].bbox.overlaps(&reach) {
                continue;
            }
            let body = tools[k];
            for h in body.face_ids() {
                if !index[k].boxes[h.index()].overlaps(&reach) {
                    continue;
                }
                let s = &body.face(h).surface;
                match curve_surface(&sg.curve, sg.range, s) {
                    CurveSurface::Hits(hits) => {
                        for hit in hits
                            .into_iter()
                            .flat_map(|x| crossings_near(&sg.curve, sg.range, s, x))
                        {
                            if index[k].point_in_face(h, hit.point) != Where::Outside {
                                got.push((hit.t, hit.point, tol::LINEAR));
                            }
                        }
                    }
                    // the new edge lies on this face's surface: it is cut where it crosses the
                    // face's boundary
                    CurveSurface::OnSurface | CurveSurface::Overlap => {
                        for &l in &body.face(h).loops {
                            for &c in &body.lp(l).coedges {
                                let e = body.coedge(c).edge;
                                if !body.edge_box(e).overlaps(&reach) {
                                    continue;
                                }
                                let Ok(across) = crate::boolean::across(body, h, c) else {
                                    continue;
                                };
                                if let CurveSurface::Hits(hits) = curve_surface(&sg.curve, sg.range, &across)
                                {
                                    for hit in hits {
                                        if let Some((_, tol)) = on_edge(body, e, hit.point) {
                                            got.push((hit.t, hit.point, tol));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        got
    });
    let mut triple: Vec<SegCut> = Vec::new();
    for (si, cs) in seg_cuts.into_iter().enumerate() {
        triple.extend(cs.into_iter().map(|(t, point, tol)| SegCut {
            seg: si,
            t,
            point,
            tol,
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
    let mut order: Vec<usize> = (0..pts.len()).collect();
    order.sort_unstable_by(|&i, &j| pts[i].x.total_cmp(&pts[j].x));
    for a in 0..order.len() {
        let i = order[a];
        for &j in &order[a + 1..] {
            if pts[j].x - pts[i].x > widest {
                break;
            }
            if pts[i].distance(pts[j]) <= tols[i].max(tols[j]) {
                let (x, y) = (find(&mut parent, i), find(&mut parent, j));
                if x != y {
                    parent[x.max(y)] = x.min(y);
                }
            }
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
        let edge = tools[c.side].edge(c.edge);
        for v in [edge.start, edge.end] {
            let vx = tools[c.side].vertex(v);
            if vx.point.distance(c.point) <= vx.tol.max(c.tol) {
                olds[cluster[i]].push(VKey::Old(c.side, v));
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
    // a new vertex settles on the point nearest every curve through it; parameters follow it
    {
        let mut through: Vec<Vec<(&Curve, Interval)>> = vec![Vec::new(); points.len()];
        for (c, &k) in cuts.iter().zip(&cut_vertex) {
            if let VKey::New(i) = k {
                let edge = tools[c.side].edge(c.edge);
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
        let settled = par_map(points.len(), threads, 64, |i| settle(points[i], &through[i]));
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
    for (c, &k) in cuts.iter_mut().zip(&cut_vertex) {
        let edge = tools[c.side].edge(c.edge);
        c.point = key_point(k);
        c.s = edge.curve.closest(c.point, edge.range);
    }
    for (tc, &k) in triple.iter_mut().zip(&triple_vertex) {
        let sg = &segs[tc.seg];
        let w = 1e-3 * sg.range.width();
        tc.point = key_point(k);
        tc.t = sg.curve.closest(tc.point, Interval::new(tc.t - w, tc.t + w));
    }
    for sg in &mut segs {
        let Some((i0, i1)) = sg.ends else { continue };
        let w = 1e-3 * sg.range.width();
        let lo = sg.curve.closest(
            key_point(cut_vertex[i0]),
            Interval::new(sg.range.lo - w, sg.range.lo + w),
        );
        let hi = sg.curve.closest(
            key_point(cut_vertex[i1]),
            Interval::new(sg.range.hi - w, sg.range.hi + w),
        );
        sg.range = Interval::new(lo, hi);
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

    // edge pieces on demand, new-edge pieces now
    let mut by_edge: Vec<(Side, EdgeId, usize)> = cuts
        .iter()
        .enumerate()
        .map(|(i, c)| (c.side, c.edge, i))
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
        let push = |list: &mut Vec<Piece>, lo: (f64, VKey), hi: (f64, VKey), k: Option<usize>| {
            list.push(Piece {
                curve: sg.curve.clone(),
                range: Interval::new(lo.0, hi.0),
                start: lo.1,
                end: hi.1,
                name: k.map_or(sg.name, |k| sg.name.with_str("part").with(k as u64)),
                seg: Some(si),
                whole: None,
                side: None,
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
                    push(&mut pieces.list, (sg.range.lo, v0), (sg.range.hi, v1), None);
                } else {
                    let all: Vec<(f64, VKey)> = std::iter::once((sg.range.lo, v0))
                        .chain(marks)
                        .chain(std::iter::once((sg.range.hi, v1)))
                        .collect();
                    for (k, w) in all.windows(2).enumerate() {
                        push(&mut pieces.list, w[0], w[1], Some(k));
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
                    let v = VKey::New(points.len() as u32 - 1);
                    push(&mut pieces.list, (sg.range.lo, v), (sg.range.hi, v), None);
                } else {
                    let m = marks.len();
                    for k in 0..m {
                        let hi = if k + 1 == m {
                            (marks[0].0 + period, marks[0].1)
                        } else {
                            marks[k + 1]
                        };
                        push(&mut pieces.list, marks[k], hi, Some(k));
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
    if !canon.is_empty() {
        for job in &mut jobs {
            for h in job.2.iter_mut().flatten() {
                if let Some(&(q, f)) = canon.get(&h.0) {
                    *h = (q, h.1 != f);
                }
            }
            let bound: Vec<usize> = job.2.iter().flatten().map(|h| h.0).collect();
            for p in &mut job.3 {
                *p = canon.get(p).map_or(*p, |c| c.0);
            }
            job.3.sort_unstable();
            job.3.dedup();
            job.3.retain(|p| !bound.contains(p));
        }
    }
    let list = &pieces.list;
    let splits = par_map(jobs.len(), threads, 8, |j| {
        let (side, f, ref boundary, ref new_edges) = jobs[j];
        split_face(tools[side], f, list, boundary, new_edges, true).map(|s| s.regions)
    });
    let mut job_regs: Vec<std::ops::Range<usize>> = Vec::with_capacity(jobs.len());
    for (j, ((side, f, _, _), split)) in jobs.iter().zip(splits).enumerate() {
        let name = tools[*side].face(*f).name;
        let first = regs.len();
        for (k, loops) in split?.into_iter().enumerate() {
            regs.push(Reg {
                side: *side,
                face: *f,
                loops,
                name: name.with_str("split").with(k as u64),
                job: Some(j),
            });
        }
        job_regs.push(first..regs.len());
    }
    lap(&mut clock, 3);

    // 5: a point of each region, just outside and just inside its tool: kept where the
    // expression differs between them
    let mut partners: HashMap<(Side, FaceId), Vec<(Side, FaceId)>> = HashMap::new();
    for &[x, y] in &coplanar {
        partners.entry(x).or_default().push(y);
        partners.entry(y).or_default().push(x);
    }
    // whether a region lies inside tool k, by how it leans off k's face along a new edge they share
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
            None => region_points(body, r.face, &r.loops, list, 1),
        };
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
            let mut shadowed = false;
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
                        let on = partners.get(&(r.side, r.face)).and_then(|ps| {
                            ps.iter()
                                .find(|&&(sd, g)| sd == k && index[k].point_in_face(g, p) != Where::Outside)
                        });
                        let Some(&(_, g)) = on else {
                            return Ok(Err(k));
                        };
                        let other = tools[k];
                        let m = other
                            .face_normal(g, other.face(g).surface.project(p))
                            .ok_or(BoolError::Internal("no normal at a region's point"))?;
                        // the other tool's material is behind its face
                        if m.dot(normal.get()) > 0.0 {
                            inner[k] = true;
                        } else {
                            outer[k] = true;
                        }
                        shadowed |= k < r.side;
                    }
                }
            }
            Ok(Ok((inner, outer, shadowed)))
        };
        let mut found = None;
        let mut stuck: Vec<Side> = Vec::new();
        // the first point nearly always settles it; the rest are worked out only if not
        for pass in 0..2 {
            if pass == 1 {
                candidates = region_points(body, r.face, &r.loops, list, usize::MAX);
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
        let (p, (inner, outer, shadowed)) = match found {
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
                "REG {ri}: tool {} face {:?} probe {p:?} in {:?} out {:?} -> {fin} {fout} shadowed {shadowed}",
                r.side,
                r.face,
                on(&inner),
                on(&outer)
            );
        }
        Ok((fin != fout && !shadowed).then_some(!fin))
    };
    let class = match batch {
        None => par_map(regs.len(), threads, 32, |ri| full(ri, None)),
        Some(batch) => {
            // one point per region, all placed at once; the rest decided the full way
            batch.warm();
            let first: Vec<Option<Point3>> = par_map(regs.len(), threads, 32, |ri| {
                let r = &regs[ri];
                region_points(tools[r.side], r.face, &r.loops, list, 1)
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
                Some(0) => Ok(None),
                Some(1) => Ok(Some(false)),
                Some(2) => Ok(Some(true)),
                _ => full(ri, first[ri]),
            })
        }
    };
    let mut class: Vec<Option<bool>> = class.into_iter().collect::<R<_>>()?;
    let merge_start = std::time::Instant::now();

    // a new edge with the same verdict on both sides divides nothing: the face is split again
    // without it, each new region taking the verdict of the old ones it is made of
    let merged = par_map(jobs.len(), threads, 16, |j| -> R<Option<Vec<Judged>>> {
        let rr = job_regs[j].clone();
        if rr.len() < 2 {
            return Ok(None);
        }
        // the region each half-edge bounds, sorted for lookup
        let mut owners: Vec<((usize, bool), usize)> = rr
            .flat_map(|ri| regs[ri].loops.iter().flatten().map(move |&h| (h, ri)))
            .collect();
        owners.sort_unstable();
        let owner = |h: (usize, bool)| owners.binary_search_by(|x| x.0.cmp(&h)).ok().map(|i| owners[i].1);
        let (side, f, ref boundary, ref new_edges) = jobs[j];
        let keep: Vec<usize> = new_edges
            .iter()
            .copied()
            .filter(|&p| match (owner((p, true)), owner((p, false))) {
                (Some(a), Some(b)) => a == b || class[a] != class[b],
                _ => true,
            })
            .collect();
        if keep.len() == new_edges.len() {
            return Ok(None);
        }
        let regions = split_face(tools[side], f, list, boundary, &keep, true)?.regions;
        regions
            .into_iter()
            .map(|loops| {
                let first = *loops
                    .first()
                    .and_then(|l| l.first())
                    .ok_or(BoolError::Internal("empty region"))?;
                let old = owner(first).ok_or(BoolError::Internal("merged region of no old one"))?;
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
        let name = tools[side].face(f).name;
        for (k, (loops, c)) in m.into_iter().enumerate() {
            extra.push((
                Reg {
                    side,
                    face: f,
                    loops,
                    name: name.with_str("split").with(k as u64),
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
    let list = pieces.list;
    let mut used: Vec<usize> = kept
        .iter()
        .flat_map(|&(ri, _)| regs[ri].loops.iter().flatten().map(|h| h.0))
        .collect();
    used.sort_unstable();
    used.dedup();
    let canon = coincident(&list, &used);
    let mut out = Body::new();
    let shell = out.add_shell();
    let mut vmap: HashMap<VKey, VertexId> = HashMap::new();
    let mut emap: HashMap<usize, EdgeId> = HashMap::new();
    for &(ri, flip) in &kept {
        let r = &regs[ri];
        let src = tools[r.side].face(r.face);
        let face = out.add_face(shell, src.surface.clone(), src.same_sense != flip, r.name);
        for lp in &r.loops {
            let mut uses: Vec<(EdgeId, bool)> = Vec::with_capacity(lp.len());
            for &(p, fwd) in lp {
                let (p, fwd) = match canon.get(&p) {
                    Some(&(q, f)) => (q, fwd != f),
                    None => (p, fwd),
                };
                let e = match emap.get(&p) {
                    Some(&e) => e,
                    None => {
                        let pc = &list[p];
                        let mut vertex = |out: &mut Body, v: VKey| -> VertexId {
                            *vmap.entry(v).or_insert_with(|| match v {
                                VKey::Old(sd, id) => {
                                    let x = tools[sd].vertex(id);
                                    let t = grown.iter().filter(|g| g.0 == v).fold(x.tol, |t, g| t.max(g.1));
                                    out.add_tolerant_vertex(x.point, t, x.name)
                                }
                                VKey::New(i) => out.add_tolerant_vertex(
                                    points[i as usize],
                                    point_tols[i as usize],
                                    Name::feature(0).with_str("vertex").with(i as u64),
                                ),
                            })
                        };
                        let (v0, v1) = (vertex(&mut out, pc.start), vertex(&mut out, pc.end));
                        let e = out.add_edge(pc.curve.clone(), pc.range, v0, v1, pc.name);
                        emap.insert(p, e);
                        e
                    }
                };
                uses.push((e, fwd));
            }
            if flip {
                uses = uses.into_iter().rev().map(|(e, f)| (e, !f)).collect();
            }
            out.add_loop(face, &uses);
        }
    }
    out.rebuild_shells();
    lap(&mut clock, 5);
    Ok(out)
}

/// Pieces among `ps` with the same ends along the same curve as a lower one: piece -> (the
/// lower one, whether it runs the other way).
fn coincident(list: &[Piece], ps: &[usize]) -> HashMap<usize, (usize, bool)> {
    let key = |pc: &Piece| (pc.start.min(pc.end), pc.start.max(pc.end));
    let mut by_ends: Vec<((VKey, VKey), usize)> = ps.iter().map(|&p| (key(&list[p]), p)).collect();
    by_ends.sort_unstable();
    let mut canon = HashMap::new();
    for g in by_ends.chunk_by(|x, y| x.0 == y.0) {
        for (k, &(_, p)) in g.iter().enumerate().skip(1) {
            let pc = &list[p];
            let mid = pc.curve.eval(pc.range.mid());
            for &(_, q) in &g[..k] {
                if canon.contains_key(&q) {
                    continue;
                }
                let o = &list[q];
                if o.curve.eval(o.curve.closest(mid, o.range)).distance(mid) <= tol::LINEAR {
                    canon.insert(p, (q, pc.start != o.start));
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

/// Points inside a region of face `f` (its loops of pieces, or the face's own loops when
/// `loops` is empty), best first: a step in from its longest edges at a few places along
/// them, each step short enough that no other edge comes nearer than it (so the way in
/// crosses none).
fn region_points(body: &Body, f: FaceId, loops: &[Walk], list: &[Piece], max: usize) -> Vec<Point3> {
    // (curve, range, forward) of each edge
    let mut edges: Vec<(&Curve, Interval, bool)> = Vec::new();
    if loops.is_empty() {
        for &l in &body.face(f).loops {
            for &c in &body.lp(l).coedges {
                let co = body.coedge(c);
                let e = body.edge(co.edge);
                edges.push((&e.curve, e.range, co.forward));
            }
        }
    } else {
        for &(p, fwd) in loops.iter().flatten() {
            edges.push((&list[p].curve, list[p].range, fwd));
        }
    }
    let chord = |e: &(&Curve, Interval, bool)| {
        let m = e.0.eval(e.1.mid());
        e.0.eval(e.1.lo)
            .distance(m)
            .max(m.distance(e.0.eval(e.1.hi)))
            .max(1e-3)
    };
    let mut order: Vec<(f64, usize)> = edges.iter().enumerate().map(|(i, e)| (chord(e), i)).collect();
    order.sort_by(|a, b| b.0.total_cmp(&a.0));
    let clear = |p: Point3, step: f64, own: usize| {
        edges.iter().enumerate().all(|(i, &(c, r, _))| {
            i == own
                || !c.bbox(r).grow(step).contains(p)
                || c.eval(c.closest(p, r)).distance(p) >= 0.999 * step
        })
    };
    let mut out = Vec::new();
    for &(len, i) in order.iter().take(4) {
        let (c, r, fwd) = edges[i];
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
    if out.is_empty()
        && let Some(&(len, i)) = order.first()
    {
        let (c, r, fwd) = edges[i];
        out.extend(step_in(body, f, c, r, fwd, 0.5, (0.01 * len).min(0.05)));
    }
    out
}
