//! The local-imprint boolean: the part changes only where the tool meets it, each decision made
//! in closed form, with no face split by tracing regions and no point-in-face test per region.
//! It runs beside the general boolean (`boolean.rs`), never instead of it: a case it does not
//! cover returns false before touching the part, and the general boolean runs.
//!
//! Covered so far: an extruded profile (one loop of lines and arcs, or a whole circle) whose
//! section lies inside, and clear of the boundary of, each planar face square to its axis that it
//! passes through, and which no other face of the part comes near (thru holes, pockets, bosses,
//! slots that meet nothing else). Along the axis the part's material inside the tool is then a set
//! of slabs between those faces, so the faces the tool passes gain a hole, the wall stands where
//! the material changes, and a cap closes a wall that ends inside the tool.

use crate::boolean::{Op, vertex_names};
use crate::classify::{circle_in_face, extend_query, loop_in_face};
use forge_geom::intersect::{SurfaceSurface, plane_cylinder};
use forge_geom::{Curve, CurveGeom, Cylinder, Surface};
use forge_math::{Box3, Interval, Point3, UnitVec3, tol};
use forge_topo::{Body, EdgeId, FaceId, Name, ShellId, Unlinked, VertexId};

/// How far apart things must be for this path to decide them; anything nearer goes to the
/// general boolean.
const CLEAR: f64 = 10.0 * tol::LINEAR;

/// Which boolean runs: FORGE_IMPRINT=on takes this path where it can, =check also runs the
/// general boolean and panics where they differ. Off by default.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    On,
    Check,
}

static MODE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(u8::MAX);

pub fn mode() -> Mode {
    let m = MODE.load(std::sync::atomic::Ordering::Relaxed);
    let m = if m == u8::MAX {
        let m = match std::env::var("FORGE_IMPRINT").as_deref() {
            Ok("on") => 1,
            Ok("check") => 2,
            _ => 0,
        };
        MODE.store(m, std::sync::atomic::Ordering::Relaxed);
        m
    } else {
        m
    };
    [Mode::Off, Mode::On, Mode::Check][m as usize]
}

/// Sets the mode for the whole process (tests).
pub fn set_mode(m: Mode) {
    MODE.store(m as u8, std::sync::atomic::Ordering::Relaxed);
}

static TAKEN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// How many booleans this path has done so far in the process.
pub fn taken() -> usize {
    TAKEN.load(std::sync::atomic::Ordering::Relaxed)
}

/// What this path reads of a tool: a Body, or an extrude's entities before they are one.
pub(crate) trait Src {
    fn face_count(&self) -> usize;
    fn face(&self, f: FaceId) -> (&Surface, bool, Name);
    /// Face `f`'s uses when it has one loop.
    fn lone_loop(&self, f: FaceId) -> Option<Vec<(EdgeId, bool)>>;
    fn vertex_count(&self) -> usize;
    fn vertex(&self, v: VertexId) -> (Point3, f64, Name);
    fn edge_count(&self) -> usize;
    fn edge(&self, e: EdgeId) -> (&Curve, Interval, VertexId, VertexId, Name);
}

impl Src for Body {
    fn face_count(&self) -> usize {
        self.faces.len()
    }
    fn face(&self, f: FaceId) -> (&Surface, bool, Name) {
        let face = Body::face(self, f);
        (&face.surface, face.same_sense, face.name)
    }
    fn lone_loop(&self, f: FaceId) -> Option<Vec<(EdgeId, bool)>> {
        let face = Body::face(self, f);
        let [l] = face.loops[..] else { return None };
        Some(
            self.lp(l)
                .coedges()
                .map(|c| (self.coedge(c).edge, self.coedge(c).forward))
                .collect(),
        )
    }
    fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    fn vertex(&self, v: VertexId) -> (Point3, f64, Name) {
        let x = Body::vertex(self, v);
        (x.point, x.tol, x.name)
    }
    fn edge_count(&self) -> usize {
        self.edges.len()
    }
    fn edge(&self, e: EdgeId) -> (&Curve, Interval, VertexId, VertexId, Name) {
        let x = Body::edge(self, e);
        (&x.curve, x.range, x.start, x.end, x.name)
    }
}

impl Src for Unlinked {
    fn face_count(&self) -> usize {
        self.faces.len()
    }
    fn face(&self, f: FaceId) -> (&Surface, bool, Name) {
        let x = &self.faces[f.index()];
        (&x.0, x.1, x.2)
    }
    fn lone_loop(&self, f: FaceId) -> Option<Vec<(EdgeId, bool)>> {
        let mut ls = self.loops_of(f);
        let l = ls.next()?;
        ls.next().is_none().then(|| l.to_vec())
    }
    fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    fn vertex(&self, v: VertexId) -> (Point3, f64, Name) {
        let x = &self.vertices[v.index()];
        (x.0, tol::LINEAR, x.1)
    }
    fn edge_count(&self) -> usize {
        self.edges.len()
    }
    fn edge(&self, e: EdgeId) -> (&Curve, Interval, VertexId, VertexId, Name) {
        let x = &self.edges[e.index()];
        (&x.0, x.1, x.2, x.3, x.4)
    }
}

/// A tool as `forge_ops::extrude` builds it: one profile loop swept straight along `z`, its
/// walls in profile order (a whole circle is one wall, its seam both of the wall's sides).
struct Tool {
    z: UnitVec3,
    origin: Point3,
    /// Heights of the bottom and top caps above `origin`.
    h: [f64; 2],
    caps: [(FaceId, Vec<(EdgeId, bool)>); 2],
    walls: Vec<Wall>,
    /// The one wall's cylinder, for a whole circle.
    round: Option<Cylinder>,
    /// A box round the tool.
    bbox: Box3,
}

struct Wall {
    face: FaceId,
    bottom: EdgeId,
    top: EdgeId,
    /// Whether the bottom edge runs along the profile.
    fwd: bool,
    /// The upright edges at the wall's start and end along the profile.
    sides: [EdgeId; 2],
}

fn tool<S: Src>(b: &S) -> Option<Tool> {
    let mut walls = Vec::new();
    let mut caps = Vec::new();
    for f in (0..b.face_count() as u32).map(FaceId) {
        let cs = b.lone_loop(f)?;
        let wall = match cs[..] {
            [(bottom, fwd), (end, true), (top, back), (start, false)] if back != fwd => {
                let upright = |e: EdgeId| matches!(b.edge(e).0, Curve::Line(_));
                (upright(start) && upright(end)).then_some(Wall {
                    face: f,
                    bottom,
                    top,
                    fwd,
                    sides: [start, end],
                })
            }
            _ => None,
        };
        match wall {
            Some(w) => walls.push(w),
            None => caps.push((f, cs)),
        }
    }
    let n = walls.len();
    if n == 0 || caps.len() != 2 || (0..n).any(|k| walls[k].sides[1] != walls[(k + 1) % n].sides[0]) {
        return None;
    }
    let Curve::Line(l) = b.edge(walls[0].sides[0]).0 else {
        return None;
    };
    let z = l.dir;
    for w in &walls {
        let ok = match b.face(w.face).0 {
            Surface::Plane(p) => p.normal_vec().dot(z.get()).abs() <= 1e-12,
            Surface::Cylinder(c) => c.frame.z().dot(z.get()).abs() >= 1.0 - 1e-12,
            _ => false,
        };
        let Curve::Line(s) = b.edge(w.sides[0]).0 else {
            return None;
        };
        if !ok || s.dir.dot(z.get()) < 1.0 - 1e-12 {
            return None;
        }
    }
    // caps square to the axis, facing out of the tool: the bottom one down
    let mut ends = [None, None];
    for (f, cs) in caps {
        let (Surface::Plane(pl), same, _) = b.face(f) else {
            return None;
        };
        let out = pl.normal_vec().dot(z.get()) * if same { 1.0 } else { -1.0 };
        if out.abs() < 1.0 - 1e-12 {
            return None;
        }
        ends[usize::from(out > 0.0)] = Some((f, cs, pl.frame.origin));
    }
    let [Some((bottom, b_uses, origin)), Some((top, t_uses, at))] = ends else {
        return None;
    };
    let h = [0.0, (at - origin).dot(z.get())];
    let round = match (&walls[..], b.face(walls[0].face)) {
        ([w], (Surface::Cylinder(c), true, _)) if w.fwd => Some(*c),
        _ => None,
    };
    let bbox = match &round {
        // the two end disks'
        Some(cyl) => {
            let (r, z) = (cyl.radius, z.get());
            let reach = |a: f64| r * (1.0 - a * a).max(0.0).sqrt() + tol::LINEAR;
            let half = Point3::new(reach(z.x), reach(z.y), reach(z.z)) - Point3::ORIGIN;
            let mut bb = Box3::EMPTY;
            for h in h {
                let c = cyl.frame.origin + z * ((origin - cyl.frame.origin).dot(z) + h);
                bb.include(c - half);
                bb.include(c + half);
            }
            bb
        }
        None => walls.iter().fold(Box3::EMPTY, |bb, w| {
            let (c0, r0, ..) = b.edge(w.bottom);
            let (c1, r1, ..) = b.edge(w.top);
            bb.hull(&c0.bbox(r0)).hull(&c1.bbox(r1))
        }),
    };
    Some(Tool {
        z,
        origin,
        h,
        caps: [(bottom, b_uses), (top, t_uses)],
        walls,
        round,
        bbox,
    })
}

/// Where the tool's profile, swept, passes through a planar face of A square to it: one curve
/// per wall, between the uprights it meets.
struct Hit {
    f: FaceId,
    h: f64,
    /// The face's outward normal points up the axis.
    up: bool,
    /// Per wall: its curve, that curve's range, and whether it starts at the wall's start side.
    segs: Vec<(Curve, Interval, bool)>,
    /// Per upright (wall k's start side): where it crosses the face.
    corners: Vec<Point3>,
}

/// Whether face `f` stays clear of the tool (swept both ways along its axis), judged by the
/// face's surface or its box against the tool's.
fn clear_of(a: &Body, f: FaceId, t: &Tool) -> bool {
    let z = t.z.get();
    if let Some(cyl) = &t.round {
        let (fr, r) = (&cyl.frame, cyl.radius);
        let apart = match &a.face(f).surface {
            // apart, or the tool wholly inside the other cylinder
            Surface::Cylinder(c) if c.frame.z().dot(z).abs() >= 1.0 - 1e-12 => {
                let off = c.frame.origin - fr.origin;
                let d = (off - z * off.dot(z)).length();
                d > r + c.radius + CLEAR || d + r < c.radius - CLEAR
            }
            Surface::Plane(p) if p.normal_vec().dot(z).abs() <= 1e-12 => {
                p.signed_distance(fr.origin).abs() > r + CLEAR
            }
            _ => false,
        };
        if apart {
            return true;
        }
    }
    // the boxes in a frame along the axis
    let fr = match &t.round {
        Some(cyl) => cyl.frame,
        None => forge_math::Frame::from_z(t.origin, t.z),
    };
    let local = |bx: &Box3| {
        let mut l = Box3::EMPTY;
        for x in [bx.x.lo, bx.x.hi] {
            for y in [bx.y.lo, bx.y.hi] {
                for w in [bx.z.lo, bx.z.hi] {
                    l.include(fr.point_to_local(Point3::new(x, y, w)));
                }
            }
        }
        l
    };
    let l = local(&a.face_box(f));
    let lift = (fr.origin - t.origin).dot(z);
    if l.z.hi + lift < t.h[0] - CLEAR || l.z.lo + lift > t.h[1] + CLEAR {
        return true;
    }
    if let Some(cyl) = &t.round {
        let dx = l.x.lo.max(-l.x.hi).max(0.0);
        let dy = l.y.lo.max(-l.y.hi).max(0.0);
        return dx * dx + dy * dy > (cyl.radius + CLEAR) * (cyl.radius + CLEAR);
    }
    let p = local(&t.bbox).grow(CLEAR);
    !l.x.overlaps(&p.x) || !l.y.overlaps(&p.y)
}

/// The boolean of `a` with `b` done in place when `b` is a tool this path covers; false, with
/// `a` untouched, otherwise.
pub(crate) fn column<S: Src>(a: &mut Body, b: &S, op: Op) -> bool {
    match plan(a, b, op) {
        Ok((t, hits, wall_at)) => {
            build(a, b, &t, &hits, &wall_at, op == Op::Subtract);
            TAKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if crate::boolean::debug_on() {
                eprintln!("imprint: taken");
            }
            true
        }
        Err(why) => {
            if crate::boolean::debug_on() {
                eprintln!("imprint: declined: {why}");
            }
            false
        }
    }
}

type Plan = (Tool, Vec<Hit>, Vec<bool>);

/// What the tool does to `a`, or why this path leaves it to the general boolean.
fn plan<S: Src>(a: &Body, b: &S, op: Op) -> Result<Plan, &'static str> {
    if op == Op::Intersect {
        return Err("intersect");
    }
    let t = tool(b).ok_or("not an extruded profile")?;
    let z = t.z.get();
    let mut hits: Vec<Hit> = Vec::new();
    for f in a.faces_where(|x| x.overlaps(&t.bbox)) {
        let face = a.face(f);
        let pl = match &face.surface {
            Surface::Plane(pl) if pl.normal_vec().dot(z).abs() >= 1.0 - 1e-12 => pl,
            _ => {
                if clear_of(a, f, &t) {
                    continue;
                }
                return Err("a face not square to the axis comes near");
            }
        };
        let h = (pl.frame.origin - t.origin).dot(z);
        if h < t.h[0] - CLEAR || h > t.h[1] + CLEAR {
            continue;
        }
        if h < t.h[0] + CLEAR || h > t.h[1] - CLEAR {
            return Err("a face on a cap's plane");
        }
        // where the uprights cross the face's plane
        let corners: Vec<Point3> = t
            .walls
            .iter()
            .map(|w| {
                let Curve::Line(l) = b.edge(w.sides[0]).0 else {
                    unreachable!("uprights are lines")
                };
                l.eval(-pl.signed_distance(l.origin) / pl.normal_vec().dot(l.dir.get()))
            })
            .collect();
        // each wall's curve in the plane (one, a line or a circle, square to the axis)
        let mut curves: Vec<Curve> = Vec::with_capacity(t.walls.len());
        for w in &t.walls {
            let cs = match (&t.round, b.face(w.face).0) {
                (Some(cyl), _) => plane_cylinder(pl, cyl),
                (None, s) => crate::ssi::closed_form(&Surface::Plane(*pl), s),
            };
            let SurfaceSurface::Curves(mut cs) = cs else {
                return Err("a wall meets the plane in no curve");
            };
            if cs.len() != 1 {
                return Err("a wall meets the plane in no curve");
            }
            curves.push(cs.swap_remove(0).0);
        }
        let inside = match (&t.round, &curves[..]) {
            (Some(_), [Curve::Circle(circle)]) => circle_in_face(a, f, circle, CLEAR),
            _ => {
                let up = forge_math::Frame::WORLD.with_origin(Point3::ORIGIN + z * h);
                let pieces: Vec<(Curve, Interval)> = t
                    .walls
                    .iter()
                    .map(|w| {
                        let (c, r, ..) = b.edge(w.bottom);
                        (c.transformed(&up), r)
                    })
                    .collect();
                loop_in_face(a, f, &pieces, CLEAR)
            }
        };
        match inside {
            None => return Err("a face's boundary comes near the profile"),
            Some(true) => {}
            Some(false) => {
                // the general boolean cuts an upright where the face's box holds the crossing,
                // even off the face: a vertex this path would not make
                let bx = a.face_box(f).grow(CLEAR);
                if corners.iter().any(|&c| bx.contains(c)) {
                    return Err("an upright crosses a face's box off the face");
                }
                continue;
            }
        }
        let up = (pl.normal_vec().dot(z) > 0.0) == face.same_sense;
        let segs = section(b, &t, h, curves, &corners);
        hits.push(Hit {
            f,
            h,
            up,
            segs,
            corners,
        });
    }
    if hits.is_empty() {
        return Err("no face crossed");
    }
    hits.sort_by(|x, y| x.h.total_cmp(&y.h));
    // the material in each stretch between hits (and the caps): below a face facing up
    for w in hits.windows(2) {
        if w[0].up == w[1].up {
            return Err("faces out of turn");
        }
    }
    let n = hits.len();
    let solid: Vec<bool> = (0..=n)
        .map(|k| if k < n { hits[k].up } else { !hits[n - 1].up })
        .collect();
    let fill = op == Op::Union;
    let wall_at: Vec<bool> = solid.iter().map(|&m| m != fill).collect();
    Ok((t, hits, wall_at))
}

/// The corners at the low and high parameter ends of wall `k`'s curve (of `n`), which starts at
/// the wall's start side or its end side.
fn ends(n: usize, k: usize, starts: bool) -> (usize, usize) {
    match (n, starts) {
        (1, _) => (0, 0),
        (_, true) => (k, (k + 1) % n),
        (_, false) => ((k + 1) % n, k),
    }
}

/// The tool's section at height `h`: each wall's curve between the corners where the uprights
/// at its ends cross the plane, and whether it runs from the wall's start side.
fn section<S: Src>(
    b: &S,
    t: &Tool,
    h: f64,
    curves: Vec<Curve>,
    corners: &[Point3],
) -> Vec<(Curve, Interval, bool)> {
    let n = t.walls.len();
    let mut segs = Vec::with_capacity(n);
    for (k, curve) in curves.into_iter().enumerate() {
        let (p0, p1) = (corners[k], corners[(k + 1) % n]);
        let (span, starts) = match (&curve, curve.domain().period) {
            (Curve::Line(l), _) => {
                let (s0, s1) = ((p0 - l.origin).dot(l.dir.get()), (p1 - l.origin).dot(l.dir.get()));
                (Interval::new(s0.min(s1), s0.max(s1)), s0 <= s1)
            }
            (_, Some(p)) => {
                let full = Interval::new(curve.domain().lo, curve.domain().lo + p);
                let s0 = curve.closest(p0, full);
                if n == 1 {
                    (Interval::new(s0, s0 + p), true)
                } else {
                    let s1 = curve.closest(p1, full);
                    let (lo, hi) = (s0.min(s1), s0.max(s1));
                    // the stretch whose middle lies over the wall's bottom edge
                    let (bc, br, ..) = b.edge(t.walls[k].bottom);
                    let drop = t.z.get() * (h - t.h[0]);
                    let over = |s: Interval| {
                        let q = curve.eval(s.mid()) - drop;
                        bc.eval(bc.closest(q, br)).distance(q) <= 1e-6
                    };
                    let inner = Interval::new(lo, hi);
                    if over(inner) {
                        (inner, s0 == lo)
                    } else {
                        (Interval::new(hi, lo + p), s0 == hi)
                    }
                }
            }
            _ => unreachable!("a plane meets a wall in a line or a circle"),
        };
        segs.push((curve, span, starts));
    }
    segs
}

/// The vertices and edges `build` has made so far, made on first use: B's by their ids, new
/// vertices by number.
struct Made<'x, S> {
    b: &'x S,
    /// B's vertices, then B's edges, then the new vertices.
    ids: Vec<Option<u32>>,
}

impl<S: Src> Made<'_, S> {
    fn b_vertex(&mut self, a: &mut Body, v: VertexId) -> VertexId {
        let (p, tol, name) = self.b.vertex(v);
        VertexId(*self.ids[v.index()].get_or_insert_with(|| a.add_tolerant_vertex(p, tol, name).0))
    }
    /// A whole edge of B.
    fn b_edge(&mut self, a: &mut Body, e: EdgeId) -> EdgeId {
        let k = self.b.vertex_count() + e.index();
        if let Some(x) = self.ids[k] {
            return EdgeId(x);
        }
        let (curve, range, s, t, name) = self.b.edge(e);
        let (v0, v1) = (self.b_vertex(a, s), self.b_vertex(a, t));
        let x = a.add_edge(curve.clone(), range, v0, v1, name);
        self.ids[k] = Some(x.0);
        x
    }
    fn new_vertex(&mut self, a: &mut Body, i: usize, at: Point3, name: Name) -> VertexId {
        let k = self.b.vertex_count() + self.b.edge_count() + i;
        VertexId(*self.ids[k].get_or_insert_with(|| a.add_vertex(at, name).0))
    }
}

fn build<S: Src>(a: &mut Body, b: &S, t: &Tool, hits: &[Hit], wall_at: &[bool], flip: bool) {
    let root = Name::feature(0);
    static CUT: std::sync::OnceLock<Name> = std::sync::OnceLock::new();
    let cut = *CUT.get_or_init(|| root.with_str("cut"));
    let nw = t.walls.len();
    let nh = hits.len();
    let z = t.z.get();
    let ends = |k: usize, starts: bool| ends(nw, k, starts);
    // named as the general boolean names a curve's piece: by the faces, then the edges it ends on
    let side_name = |c: usize| b.edge(t.walls[c].sides[0]).4;
    // hit i's wall-k curve is name i * nw + k
    let mut names: Vec<Name> = Vec::with_capacity(nh * nw);
    for h in hits {
        for (k, w) in t.walls.iter().enumerate() {
            let (c0, c1) = ends(k, h.segs[k].2);
            names.push(
                cut.with_set(&mut [a.face(h.f).name, b.face(w.face).2])
                    .with(0)
                    .with_name(side_name(c0))
                    .with_name(side_name(c1))
                    .with(0),
            );
        }
    }
    // new vertices: hit i's corner c is point i * nw + c
    let points: Vec<Point3> = hits.iter().flat_map(|h| h.corners.iter().copied()).collect();
    let mut marks = Vec::with_capacity(2 * nw * nh);
    for (i, h) in hits.iter().enumerate() {
        for (k, name) in names[i * nw..(i + 1) * nw].iter().enumerate() {
            let (c0, c1) = ends(k, h.segs[k].2);
            marks.push(((i * nw + c0) as u32, *name));
            marks.push(((i * nw + c1) as u32, Name(name.0 ^ 1)));
        }
    }
    let vnames = vertex_names(&points, marks, root);
    let shell = ShellId(0);
    let mut made = Made {
        b,
        ids: vec![None; b.vertex_count() + b.edge_count() + points.len()],
    };
    // each hit face gains a hole
    let mut seg_edge = vec![None; nh * nw];
    for (i, h) in hits.iter().enumerate() {
        let mut cycle = vec![(0, true)];
        let mut at = ends(0, h.segs[0].2).1;
        while cycle.len() < nw {
            let k = (0..nw)
                .find(|k| {
                    !cycle.iter().any(|c| c.0 == *k)
                        && (ends(*k, h.segs[*k].2).0 == at || ends(*k, h.segs[*k].2).1 == at)
                })
                .expect("a closed section");
            let (c0, c1) = ends(k, h.segs[k].2);
            cycle.push((k, c0 == at));
            at = if c0 == at { c1 } else { c0 };
        }
        let ccw = match &h.segs[0].0 {
            Curve::Circle(c) if nw == 1 => c.frame.z().dot(z) > 0.0,
            _ => ends(0, h.segs[0].2).0 == 0,
        };
        let hole: Vec<(usize, bool)> = if ccw == h.up {
            cycle.iter().rev().map(|&(k, d)| (k, !d)).collect()
        } else {
            cycle
        };
        let mut uses = Vec::with_capacity(nw);
        for (k, d) in hole {
            let (c0, c1) = ends(k, h.segs[k].2);
            let (p0, p1) = (i * nw + c0, i * nw + c1);
            let v0 = made.new_vertex(a, p0, points[p0], vnames[p0]);
            let v1 = made.new_vertex(a, p1, points[p1], vnames[p1]);
            let e = a.add_edge(h.segs[k].0.clone(), h.segs[k].1, v0, v1, names[i * nw + k]);
            seg_edge[i * nw + k] = Some(e);
            uses.push((e, d));
        }
        let lp = a.add_loop(h.f, &uses);
        extend_query(a, h.f, lp);
    }

    let flipped = |uses: &mut Vec<(EdgeId, bool)>| {
        if flip {
            uses.reverse();
            for u in uses.iter_mut() {
                u.1 = !u.1;
            }
        }
    };
    let walls = wall_at.iter().filter(|&&w| w).count();
    // the uprights' pieces between events (the caps, then each hit), made on first use
    let mut side_piece: Vec<Option<EdgeId>> = vec![None; (nh + 1) * nw];
    for f in (0..b.face_count() as u32).map(FaceId) {
        let (surface, same_sense, src_name) = b.face(f);
        if let Some(k) = t.caps.iter().position(|c| c.0 == f) {
            if !wall_at[if k == 0 { 0 } else { nh }] {
                continue;
            }
            let face = a.add_face(shell, surface.clone(), same_sense != flip, src_name);
            let mut uses: Vec<(EdgeId, bool)> =
                t.caps[k].1.iter().map(|&(e, d)| (made.b_edge(a, e), d)).collect();
            flipped(&mut uses);
            a.add_loop(face, &uses);
            continue;
        }
        let Some(k) = t.walls.iter().position(|w| w.face == f) else {
            continue;
        };
        let w = &t.walls[k];
        for i in 0..=nh {
            if !wall_at[i] {
                continue;
            }
            // the curve along the bottom (i) or top (i + 1) of this stretch, and which way it runs
            // from the wall's start side
            let along = |i: usize| -> (Option<(usize, usize)>, bool) {
                if i == 0 || i == nh + 1 {
                    return (None, w.fwd);
                }
                let s = &hits[i - 1].segs[k];
                let d = match (&s.0, nw) {
                    (Curve::Circle(c), 1) => (c.frame.z().dot(z) > 0.0) == w.fwd,
                    _ => ends(k, s.2).0 == k,
                };
                (Some((i - 1, k)), d)
            };
            let (lo, d_lo) = along(i);
            let (hi, d_hi) = along(i + 1);
            let mut piece = |a: &mut Body, made: &mut Made<S>, j: usize| -> EdgeId {
                let c = (k + j) % nw;
                if let Some(e) = side_piece[c * (nh + 1) + i] {
                    return e;
                }
                let (curve, range, start, end, name) = b.edge(t.walls[c].sides[0]);
                let mut end_at = |a: &mut Body, i: usize| -> (VertexId, f64, Name) {
                    if i == 0 || i == nh + 1 {
                        let (v, s) = if i == 0 {
                            (start, range.lo)
                        } else {
                            (end, range.hi)
                        };
                        (made.b_vertex(a, v), s, b.vertex(v).2)
                    } else {
                        let p = (i - 1) * nw + c;
                        let id = made.new_vertex(a, p, points[p], vnames[p]);
                        (id, curve.closest(points[p], range), vnames[p])
                    }
                };
                let (v0, s0, n0) = end_at(a, i);
                let (v1, s1, n1) = end_at(a, i + 1);
                let name = if walls == 1 {
                    name
                } else {
                    name.piece(n0.with_name(n1))
                };
                let e = a.add_edge(curve.clone(), Interval::new(s0, s1), v0, v1, name);
                side_piece[c * (nh + 1) + i] = Some(e);
                e
            };
            let mut walk: Vec<(EdgeId, bool)> = Vec::with_capacity(4);
            for j in 0..4 {
                walk.push(match j {
                    0 => match lo {
                        None => (made.b_edge(a, w.bottom), d_lo),
                        Some((h, k)) => (seg_edge[h * nw + k].expect("made with the hole"), d_lo),
                    },
                    1 => (piece(a, &mut made, 1), true),
                    2 => match hi {
                        None => (made.b_edge(a, w.top), !d_hi),
                        Some((h, k)) => (seg_edge[h * nw + k].expect("made with the hole"), !d_hi),
                    },
                    _ => (piece(a, &mut made, 0), false),
                });
            }
            let kept = (0..=nh).filter(|&x| wall_at[x]).count();
            let name = if kept == 1 {
                src_name
            } else {
                let least = walk
                    .iter()
                    .map(|&(e, d)| (a.edge(e).name, d))
                    .min()
                    .expect("four uses");
                src_name.piece(least.0.with(least.1 as u64))
            };
            flipped(&mut walk);
            let face = a.add_face(shell, surface.clone(), same_sense != flip, name);
            a.add_loop(face, &walk);
        }
    }
}

/// Where two bodies differ, for the check mode (None: the same). Entities are matched by name,
/// not by id or order: each face's surface and sense, its loops as cycles of (edge name, way),
/// each edge's curve kind and ends, each vertex's point, points to within 1e-9.
pub(crate) fn diff(x: &Body, y: &Body) -> Option<String> {
    let close = |p: Point3, q: Point3| p.distance(q) <= 1e-9 * (1.0 + p.distance(Point3::ORIGIN));
    type Loop = Vec<(Name, bool)>;
    let faces = |b: &Body| {
        let mut out: Vec<(Name, bool, String, Vec<Loop>)> = b
            .face_ids()
            .map(|f| {
                let face = b.face(f);
                let mut loops: Vec<Loop> = face
                    .loops
                    .iter()
                    .map(|&l| {
                        let mut c: Loop = b
                            .lp(l)
                            .coedges()
                            .map(|c| (b.edge(b.coedge(c).edge).name, b.coedge(c).forward))
                            .collect();
                        let k = (0..c.len()).min_by_key(|&k| c[k]).unwrap_or(0);
                        c.rotate_left(k);
                        c
                    })
                    .collect();
                loops.sort();
                (face.name, face.same_sense, format!("{:?}", face.surface), loops)
            })
            .collect();
        out.sort_by(|p, q| (p.0, &p.3).cmp(&(q.0, &q.3)));
        out
    };
    let (fx, fy) = (faces(x), faces(y));
    if fx.len() != fy.len() {
        return Some(format!("{} faces vs {}", fx.len(), fy.len()));
    }
    for (p, q) in fx.iter().zip(&fy) {
        if p != q {
            return Some(format!("face {:?}: {:?} vs {:?}", p.0, p, q));
        }
    }
    // every edge the faces use, by name: its kind and its ends
    let edges = |b: &Body| {
        let mut out: Vec<(
            Name,
            Point3,
            Point3,
            Name,
            Name,
            std::mem::Discriminant<forge_geom::Curve>,
        )> = b
            .edge_ids()
            .map(|e| {
                let edge = b.edge(e);
                let (s, t) = (b.vertex(edge.start), b.vertex(edge.end));
                (
                    edge.name,
                    s.point,
                    t.point,
                    s.name,
                    t.name,
                    std::mem::discriminant(&edge.curve),
                )
            })
            .collect();
        out.sort_by_key(|e| (e.0, e.3, e.4));
        out
    };
    let (ex, ey) = (edges(x), edges(y));
    if ex.len() != ey.len() {
        return Some(format!("{} edges vs {}", ex.len(), ey.len()));
    }
    for (p, q) in ex.iter().zip(&ey) {
        if (p.0, p.3, p.4, p.5) != (q.0, q.3, q.4, q.5) || !close(p.1, q.1) || !close(p.2, q.2) {
            return Some(format!("edge {:?}: {:?} vs {:?}", p.0, p, q));
        }
    }
    None
}
