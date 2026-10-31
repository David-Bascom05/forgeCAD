//! The local-imprint boolean: the part changes only where the tool meets it, each decision made
//! in closed form, with no face split by tracing regions and no point-in-face test per region.
//! It runs beside the general boolean (`boolean.rs`), never instead of it: a case it does not
//! cover returns false before touching the part, and the general boolean runs.
//!
//! Covered: an extruded profile (one loop of lines and arcs, or a whole circle) in a column of the
//! part where every face it comes near is square to its axis (a plane) or along it (a wall: a
//! plane or cylinder between two such planes). The walls' footprints cross the profile's in
//! closed form, so each wall splits into a grid of cells along it and up it, each kept or not
//! by which side of the other solid it lies, and each square face loses what the tool covers
//! and gains the profile's stretches over it. The result goes to the general boolean's `apply`
//! as its plan, pieces and faces named as that boolean names them.

use crate::boolean::{
    Op, Piece, Plan, Spec, VKey, apply, curve_at, part_frame, piece_name, rank_names, region_name,
    surface_frame, vertex_names,
};
use crate::classify::{edges_near, in_loop, in_planar_face};
use forge_geom::intersect::SurfaceSurface;
use forge_geom::{Curve, CurveGeom, Cylinder, Plane, Surface, SurfaceGeom};
use forge_math::{Box3, Frame, Interval, Point3, UnitVec3, Vec3, tol};
use forge_topo::{Body, EdgeId, FaceId, LoopId, Name, Unlinked, VertexId};

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
    /// How many uses face `f`'s loop has, when it has one loop.
    fn loop_len(&self, f: FaceId) -> Option<usize>;
    /// The `i`th use of face `f`'s one loop.
    fn loop_use(&self, f: FaceId, i: usize) -> (EdgeId, bool);
    /// Face `f`'s uses when it has one loop.
    fn lone_loop(&self, f: FaceId) -> Option<Vec<(EdgeId, bool)>> {
        Some((0..self.loop_len(f)?).map(|i| self.loop_use(f, i)).collect())
    }
    fn vertex_count(&self) -> usize;
    fn vertex(&self, v: VertexId) -> (Point3, f64, Name);
    fn edge(&self, e: EdgeId) -> (&Curve, Interval, VertexId, VertexId, Name);
    /// Face `f`'s box, where one is kept.
    fn bbox(&self, f: FaceId) -> Option<Box3>;
    /// The tool as a Body, for what only reads Bodies (rare: a tool flush on a face).
    fn body(&self) -> std::borrow::Cow<'_, Body>;
}

impl Src for Body {
    fn body(&self) -> std::borrow::Cow<'_, Body> {
        std::borrow::Cow::Borrowed(self)
    }
    fn face_count(&self) -> usize {
        self.faces.len()
    }
    fn face(&self, f: FaceId) -> (&Surface, bool, Name) {
        let face = Body::face(self, f);
        (&face.surface, face.same_sense, face.name)
    }
    fn loop_len(&self, f: FaceId) -> Option<usize> {
        let [l] = Body::face(self, f).loops[..] else {
            return None;
        };
        Some(self.lp(l).coedges().len())
    }
    fn loop_use(&self, f: FaceId, i: usize) -> (EdgeId, bool) {
        let c = self.coedge(self.lp(Body::face(self, f).loops[0]).coedge(i));
        (c.edge, c.forward)
    }
    fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    fn vertex(&self, v: VertexId) -> (Point3, f64, Name) {
        let x = Body::vertex(self, v);
        (x.point, x.tol, x.name)
    }
    fn edge(&self, e: EdgeId) -> (&Curve, Interval, VertexId, VertexId, Name) {
        let x = Body::edge(self, e);
        (&x.curve, x.range, x.start, x.end, x.name)
    }
    fn bbox(&self, f: FaceId) -> Option<Box3> {
        Body::face(self, f).bbox.get().copied()
    }
}

impl Src for Unlinked {
    fn body(&self) -> std::borrow::Cow<'_, Body> {
        std::borrow::Cow::Owned(self.clone().into_body())
    }
    fn face_count(&self) -> usize {
        self.faces.len()
    }
    fn face(&self, f: FaceId) -> (&Surface, bool, Name) {
        let x = &self.faces[f.index()];
        (&x.0, x.1, x.2)
    }
    fn loop_len(&self, f: FaceId) -> Option<usize> {
        Some(Unlinked::lone_loop(self, f)?.len())
    }
    fn loop_use(&self, f: FaceId, i: usize) -> (EdgeId, bool) {
        Unlinked::lone_loop(self, f).expect("one loop")[i]
    }
    fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    fn vertex(&self, v: VertexId) -> (Point3, f64, Name) {
        let x = &self.vertices[v.index()];
        (x.0, tol::LINEAR, x.1)
    }
    fn edge(&self, e: EdgeId) -> (&Curve, Interval, VertexId, VertexId, Name) {
        let x = &self.edges[e.index()];
        (&x.0, x.1, x.2, x.3, x.4)
    }
    fn bbox(&self, _: FaceId) -> Option<Box3> {
        None
    }
}

/// A tool as `forge_ops::extrude` builds it: one profile loop swept straight along `z` from
/// `origin` through `h`, two caps square to it and walls along it.
struct Tool {
    z: UnitVec3,
    origin: Point3,
    h: f64,
    /// The bottom and top caps.
    caps: [FaceId; 2],
    walls: Vec<FaceId>,
    /// The one wall's cylinder, for a whole circle.
    round: Option<Cylinder>,
    /// A box round the tool.
    bbox: Box3,
}

fn tool(b: &dyn Src) -> Option<Tool> {
    let mut walls = Vec::new();
    let mut caps = Vec::new();
    let mut z = None;
    for f in (0..b.face_count() as u32).map(FaceId) {
        let n = b.loop_len(f)?;
        let u = |i: usize| b.loop_use(f, i);
        // a wall's loop: bottom, upright going up, top the other way, upright going down
        match (n == 4).then(|| [u(0), u(1), u(2), u(3)]) {
            Some([(_, fwd), (up, true), (_, back), (down, false)]) if back != fwd => {
                let (Curve::Line(l), Curve::Line(_)) = (b.edge(up).0, b.edge(down).0) else {
                    return None;
                };
                z.get_or_insert(l.dir);
                walls.push(f);
            }
            _ if caps.len() == 2 || !matches!(b.face(f).0, Surface::Plane(_)) => return None,
            _ => caps.push(f),
        }
    }
    let z: UnitVec3 = z?;
    // along a world axis only: the general boolean cuts curves where they cross a face's box,
    // which for a wall square to a world plane is where they cross the face itself
    let zv = z.get();
    if caps.len() != 2 || zv.x.abs().max(zv.y.abs()).max(zv.z.abs()) < 1.0 - 1e-12 {
        return None;
    }
    // caps square to the axis, facing out of the tool: the bottom one down
    let mut ends = [None, None];
    for &f in &caps {
        let (Surface::Plane(pl), same, _) = b.face(f) else {
            return None;
        };
        let out = pl.normal_vec().dot(z.get()) * if same { 1.0 } else { -1.0 };
        if out.abs() < 1.0 - 1e-12 {
            return None;
        }
        ends[usize::from(out > 0.0)] = Some((f, pl.frame.origin));
    }
    let [Some((bottom, origin)), Some((top, at))] = ends else {
        return None;
    };
    let h = (at - origin).dot(z.get());
    let round = match (&walls[..], b.face(walls[0])) {
        ([_], (Surface::Cylinder(c), true, _)) => Some(*c),
        _ => None,
    };
    // the caps' edges' boxes, as the general boolean boxes the tool's faces (an arc's is wider
    // than its circle's ball in a turned frame, and faces there are cut by its box)
    let mut bbox = Box3::EMPTY;
    for f in [bottom, top] {
        for i in 0..b.loop_len(f)? {
            let (c, r, ..) = b.edge(b.loop_use(f, i).0);
            bbox = bbox.hull(&c.bbox(r));
        }
    }
    Some(Tool {
        z,
        origin,
        h,
        caps: [bottom, top],
        walls,
        round,
        bbox: bbox.grow(tol::LINEAR),
    })
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
        None => Frame::from_z(t.origin, t.z),
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
    if l.z.hi + lift < -CLEAR || l.z.lo + lift > t.h + CLEAR {
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
        Ok(p) => {
            apply(a, b, &p);
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

/// A face along the axis, of A (side 0) or B (side 1): its loop is a curve at one height, an
/// upright, a curve at another height and an upright (one seam twice for a whole circle).
struct Wall {
    side: usize,
    face: FaceId,
    bottom: EdgeId,
    top: EdgeId,
    /// Heights of the bottom and top curves.
    z: [f64; 2],
    /// The uprights at the bottom curve's low and high parameter ends, and their bottom and top
    /// vertices.
    ups: [EdgeId; 2],
    up_v: [[VertexId; 2]; 2],
    /// The bottom curve at height 0, and its range.
    foot: Curve,
    range: Interval,
    /// Along the foot then up runs counterclockwise about the face's outward normal.
    ccw: bool,
    /// Where the foot is cut: its ends (`End`) and where walls of the other side cross it, by
    /// parameter.
    breaks: Vec<(f64, Brk)>,
    /// Each stretch between breaks: its middle on the foot and the foot's way there.
    mids: Vec<(Point3, Vec3)>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Brk {
    End(usize),
    X(usize),
}

/// Where a wall of B crosses a wall of A, at height 0.
struct Cross {
    b: usize,
    a: usize,
    p: Point3,
}

/// A vertex of the result: an existing one, a crossing's at a height, or an upright's at a
/// height (heights by level).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Vk {
    Old(usize, VertexId),
    X(usize, usize),
    Up(usize, EdgeId, usize),
}

/// A curve of the result: an existing edge, or where two faces meet: the vertical line at a
/// crossing, A's square face `F` with B's wall `k` (`H`), a cap with A's wall `j` (`C`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Ck {
    Edge(usize, EdgeId),
    V(usize),
    H(FaceId, usize),
    C(usize, usize),
}

/// A stretch of a face's new boundary along one curve, between two vertices; `mid` lies on it
/// and `dir` is its way there.
#[derive(Clone, Copy, Debug)]
struct Arc {
    ck: Ck,
    from: Vk,
    to: Vk,
    mid: Point3,
    dir: Vec3,
    /// On a wall: its ends' places in the wall's grid (a seam's two sides are one vertex).
    grid: Option<[(usize, usize); 2]>,
}

/// Where an arc ends, for joining arcs into loops.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    V(Vk),
    G(usize, usize),
}

impl Arc {
    fn node(&self, end: usize) -> Node {
        match self.grid {
            Some(g) => Node::G(g[end].0, g[end].1),
            None => Node::V(if end == 0 { self.from } else { self.to }),
        }
    }
}

/// A square face of A within the tool's span.
struct Square {
    f: FaceId,
    h: f64,
    /// The face's outward normal points up the axis.
    up: bool,
}

type R<T> = Result<T, &'static str>;

struct Column<'x> {
    a: &'x Body,
    b: &'x dyn Src,
    z: Vec3,
    origin: Point3,
    h: f64,
    caps: [FaceId; 2],
    levels: Vec<f64>,
    walls: Vec<Wall>,
    /// B's walls are `walls[..nb]`.
    nb: usize,
    squares: Vec<Square>,
    xs: Vec<Cross>,
    /// Per B wall, per stretch: the squares over it, low to high.
    over: Vec<Vec<Vec<usize>>>,
    /// Per A wall, per stretch: inside the tool's section.
    inside: Vec<Vec<bool>>,
    /// Vertices the general boolean puts on uprights off the other face (its box holds them).
    /// (the upright, its side, the vertex, the other body's face whose curve cut it)
    extra: Vec<(EdgeId, usize, Vk, Name)>,
    /// Per cap: the square face it lies on, where the tool stands on (or sinks from) one; the cap
    /// goes and the face loses the tool's section, bounded by B's own edges.
    glued: [Option<usize>; 2],
    carriers: std::cell::RefCell<Vec<(Ck, std::rc::Rc<Carrier>)>>,
}

/// The surface normal of a face pointing out of its solid, at `p`.
fn outward(s: &Surface, same: bool, p: Point3) -> Option<Vec3> {
    let n = s.normal(s.project(p))?.get();
    Some(if same { n } else { -n })
}

/// A curve of a plane square to `z`: a line, or a circle about `z`.
fn flat(c: &Curve, z: Vec3) -> bool {
    match c {
        Curve::Line(l) => l.dir.dot(z).abs() <= 1e-12,
        Curve::Circle(k) => k.frame.z().dot(z).abs() >= 1.0 - 1e-12,
        _ => false,
    }
}

impl Column<'_> {
    fn height(&self, p: Point3) -> f64 {
        (p - self.origin).dot(self.z)
    }

    fn src(&self, side: usize) -> &dyn Src {
        if side == 0 { self.a } else { self.b }
    }

    fn lvl(&self, h: f64) -> usize {
        self.levels
            .iter()
            .position(|&l| (l - h).abs() <= tol::LINEAR)
            .expect("a level")
    }

    /// A wall from face `f`'s loop, or None if it is not one.
    fn wall(&self, side: usize, f: FaceId) -> Option<Wall> {
        let src = self.src(side);
        let (surface, same, _) = src.face(f);
        match surface {
            Surface::Plane(p) if p.normal_vec().dot(self.z).abs() <= 1e-12 => {}
            Surface::Cylinder(c) if c.frame.z().dot(self.z).abs() >= 1.0 - 1e-12 => {}
            _ => return None,
        }
        let uses = src.lone_loop(f)?;
        if uses.len() != 4 {
            return None;
        }
        let upright = |e: EdgeId| match src.edge(e).0 {
            Curve::Line(l) => l.dir.dot(self.z).abs() >= 1.0 - 1e-12,
            _ => false,
        };
        let r = (0..2).find(|&r| {
            flat(src.edge(uses[r].0).0, self.z)
                && flat(src.edge(uses[r + 2].0).0, self.z)
                && upright(uses[r + 1].0)
                && upright(uses[(r + 3) % 4].0)
        })?;
        let hz = |e: EdgeId| {
            let (_, _, v, ..) = src.edge(e);
            self.height(src.vertex(v).0)
        };
        let (e0, e1) = (uses[r], uses[r + 2]);
        let (lo, hi) = if hz(e0.0) < hz(e1.0) { (e0, e1) } else { (e1, e0) };
        let z = [hz(lo.0), hz(hi.0)];
        if z[1] - z[0] <= CLEAR {
            return None;
        }
        let (curve, range, start, end, _) = src.edge(lo.0);
        let ups_of = |v: VertexId| {
            [uses[r + 1].0, uses[(r + 3) % 4].0].into_iter().find(|&u| {
                let (_, _, s, t, _) = src.edge(u);
                s == v || t == v
            })
        };
        let ups = [ups_of(start)?, ups_of(end)?];
        let up_v = ups.map(|u| {
            let (_, _, s, t, _) = src.edge(u);
            if self.height(src.vertex(s).0) < self.height(src.vertex(t).0) {
                [s, t]
            } else {
                [t, s]
            }
        });
        let foot = curve.transformed(&Frame::WORLD.with_origin(Point3::ORIGIN - self.z * z[0]));
        let mid = curve.eval(range.mid());
        let n = outward(surface, same, mid)?;
        let along = curve.deriv(range.mid()).d1;
        // walked forward with the face on the left, the bottom curve has the face above it
        let ccw = n.cross(along).dot(self.z) > 0.0;
        Some(Wall {
            side,
            face: f,
            bottom: lo.0,
            top: hi.0,
            z,
            ups,
            up_v,
            foot,
            range,
            ccw,
            breaks: vec![(range.lo, Brk::End(0)), (range.hi, Brk::End(1))],
            mids: Vec::new(),
        })
    }
}

/// Where two curves of the plane square to `z` (lines, or circles about `z`) cross within
/// their ranges: (parameter on each, point). None where they come near touching, near an end,
/// or run together.
fn meet(c: &Curve, rc: Interval, d: &Curve, rd: Interval, z: Vec3) -> Option<Vec<(f64, f64, Point3)>> {
    // the points where the whole curves cross
    let pts: Vec<Point3> = match (c, d) {
        (Curve::Line(l), Curve::Line(m)) => {
            let n = l.dir.get().cross(m.dir.get()).dot(z);
            if n.abs() <= 1e-9 {
                let off = m.origin - l.origin;
                let gap = (off - l.dir.get() * off.dot(l.dir.get())).length();
                return (gap > CLEAR).then(Vec::new);
            }
            let t = (m.origin - l.origin).cross(m.dir.get()).dot(z) / n;
            vec![l.eval(t)]
        }
        (Curve::Line(l), Curve::Circle(k)) | (Curve::Circle(k), Curve::Line(l)) => {
            let off = k.frame.origin - l.origin;
            let t = off.dot(l.dir.get());
            let foot = l.eval(t);
            let dd = foot.distance(k.frame.origin);
            if (dd - k.radius).abs() <= CLEAR {
                return None;
            }
            if dd > k.radius {
                return Some(Vec::new());
            }
            let w = (k.radius * k.radius - dd * dd).sqrt();
            vec![l.eval(t - w), l.eval(t + w)]
        }
        (Curve::Circle(k), Curve::Circle(m)) => {
            let off = m.frame.origin - k.frame.origin;
            let off = off - z * off.dot(z);
            let dd = off.length();
            let (r1, r2) = (k.radius, m.radius);
            if (dd - (r1 + r2)).abs() <= CLEAR || (dd - (r1 - r2).abs()).abs() <= CLEAR {
                return None;
            }
            if dd > r1 + r2 || dd < (r1 - r2).abs() {
                return Some(Vec::new());
            }
            let u = off * (1.0 / dd);
            let a = (dd * dd + r1 * r1 - r2 * r2) / (2.0 * dd);
            let w = (r1 * r1 - a * a).max(0.0).sqrt();
            let base = k.frame.origin + u * a;
            let side = z.cross(u);
            vec![base - side * w, base + side * w]
        }
        _ => return None,
    };
    let mut out = Vec::new();
    for p in pts {
        let (tc, td) = (within(c, rc, p)?, within(d, rd, p)?);
        if let (Some(tc), Some(td)) = (tc, td) {
            out.push((tc, td, p));
        }
    }
    Some(out)
}

/// Point `p` of curve `c`'s carrier: its parameter if it lies within `r`, None (inner) if
/// outside; None (outer) if it lies within CLEAR of an end.
fn within(c: &Curve, r: Interval, p: Point3) -> Option<Option<f64>> {
    let t = c.closest(p, r);
    let q = c.eval(t);
    let near_end = c.eval(r.lo).distance(p) <= CLEAR || c.eval(r.hi).distance(p) <= CLEAR;
    if near_end {
        return None;
    }
    Some((q.distance(p) <= tol::LINEAR).then_some(t))
}

/// The distance from `p` to curve `c` over `r`.
fn gap(c: &Curve, r: Interval, p: Point3) -> f64 {
    c.eval(c.closest(p, r)).distance(p)
}

/// Twice... the area swept about `o` (projected on `n`) as curve `c` runs from `t0` to `t1`:
/// summed round a closed loop, its area (positive counterclockwise about `n`).
fn swept(c: &Curve, t0: f64, t1: f64, o: Point3, n: Vec3) -> f64 {
    match c {
        Curve::Circle(k) => {
            let (p0, p1) = (c.eval(t0), c.eval(t1));
            let ctr = k.frame.origin - o;
            0.5 * (ctr.cross(p1 - p0).dot(n) + k.radius * k.radius * k.frame.z().dot(n) * (t1 - t0))
        }
        _ => 0.5 * (c.eval(t0) - o).cross(c.eval(t1) - o).dot(n),
    }
}

/// Splits arcs into closed cycles (each vertex one way in, one way out), runs along one curve
/// joined.
fn cycles(arcs: Vec<Arc>) -> R<Vec<Vec<Arc>>> {
    let mut from: Vec<(Node, usize)> = arcs.iter().enumerate().map(|(i, a)| (a.node(0), i)).collect();
    from.sort();
    let mut to: Vec<Node> = arcs.iter().map(|a| a.node(1)).collect();
    to.sort();
    if from.windows(2).any(|w| w[0].0 == w[1].0) || to.windows(2).any(|w| w[0] == w[1]) {
        return Err("a vertex with two ways on");
    }
    let mut used = vec![false; arcs.len()];
    let mut out = Vec::new();
    for s in 0..arcs.len() {
        if used[s] {
            continue;
        }
        let mut cyc: Vec<Arc> = Vec::new();
        let mut cur = s;
        loop {
            used[cur] = true;
            match cyc.last_mut() {
                Some(l) if l.ck == arcs[cur].ck => {
                    l.to = arcs[cur].to;
                    if let (Some(g), Some(h)) = (&mut l.grid, arcs[cur].grid) {
                        g[1] = h[1];
                    }
                }
                _ => cyc.push(arcs[cur]),
            }
            let k = from
                .binary_search_by(|x| x.0.cmp(&arcs[cur].node(1)))
                .map_err(|_| "an open boundary")?;
            let nx = from[k].1;
            if nx == s {
                break;
            }
            if used[nx] {
                return Err("an open boundary");
            }
            cur = nx;
        }
        if cyc.len() > 1 && cyc[0].ck == cyc[cyc.len() - 1].ck {
            let last = cyc.pop().expect("two arcs");
            cyc[0].from = last.from;
            if let (Some(g), Some(h)) = (&mut cyc[0].grid, last.grid) {
                g[0] = h[0];
            }
            cyc[0].mid = last.mid;
            cyc[0].dir = last.dir;
        }
        out.push(cyc);
    }
    Ok(out)
}

/// A curve of the result with where its parameter runs.
struct Carrier {
    curve: Curve,
    /// A closed curve's period.
    period: Option<f64>,
    /// An edge's range (the whole line for a new line).
    window: Interval,
    /// A new curve's name before its ends: the faces meeting there and which of their curves.
    base: Name,
}

impl Column<'_> {
    fn point(&self, v: Vk) -> Point3 {
        match v {
            Vk::Old(s, id) => self.src(s).vertex(id).0,
            Vk::X(x, l) => self.xs[x].p + self.z * self.levels[l],
            Vk::Up(s, u, l) => {
                let Curve::Line(line) = self.src(s).edge(u).0 else {
                    unreachable!("uprights are lines")
                };
                let t = (self.levels[l] - self.height(line.origin)) / line.dir.dot(self.z);
                line.eval(t)
            }
        }
    }

    /// The vertex of wall `w` at break `i` and height `zz`.
    fn vk(&self, w: usize, i: usize, zz: f64) -> Vk {
        let wall = &self.walls[w];
        match wall.breaks[i].1 {
            Brk::X(x) => Vk::X(x, self.lvl(zz)),
            Brk::End(e) => {
                for k in 0..2 {
                    if (wall.z[k] - zz).abs() <= tol::LINEAR {
                        return Vk::Old(wall.side, wall.up_v[e][k]);
                    }
                }
                Vk::Up(wall.side, wall.ups[e], self.lvl(zz))
            }
        }
    }

    fn stretch_mid(&self, w: usize, i: usize) -> (Point3, Vec3) {
        self.walls[w].mids[i]
    }

    fn face_out(&self, side: usize, f: FaceId, p: Point3) -> R<Vec3> {
        let (s, same, _) = self.src(side).face(f);
        outward(s, same, p).ok_or("a face without a normal")
    }

    /// Whether A has material over stretch `i` of B's wall `k` at height `zz`.
    fn material(&self, k: usize, i: usize, zz: f64) -> R<bool> {
        let over = &self.over[k][i];
        if let Some(&s) = over.iter().find(|&&s| self.squares[s].h > zz) {
            return Ok(self.squares[s].up);
        }
        if let Some(&s) = over.last() {
            return Ok(!self.squares[s].up);
        }
        // no face over it within the tool: the wall crossing at either end says
        let wall = &self.walls[k];
        for (j, sign) in [(i, 1.0), (i + 1, -1.0)] {
            if let (t, Brk::X(x)) = wall.breaks[j] {
                let aw = &self.walls[self.xs[x].a];
                let at = self.xs[x].p + self.z * (0.5 * (aw.z[0] + aw.z[1]));
                let n = self.face_out(0, aw.face, at)?;
                return Ok((wall.foot.deriv(t).d1 * sign).dot(n) < 0.0);
            }
        }
        Err("no material known along the tool")
    }

    /// The grid of wall `w`: its cells along the foot and up between levels, the kept ones'
    /// boundary as arcs.
    fn wall_arcs(&self, w: usize, keep_b: bool) -> R<Option<(Vec<Arc>, Vec<f64>)>> {
        let wall = &self.walls[w];
        let mut zs = vec![wall.z[0]];
        zs.extend(
            self.levels
                .iter()
                .copied()
                .filter(|&l| l > wall.z[0] + tol::LINEAR && l < wall.z[1] - tol::LINEAR),
        );
        zs.push(wall.z[1]);
        let (nc, nr) = (wall.breaks.len() - 1, zs.len() - 1);
        let mut kept = vec![false; nc * nr];
        for i in 0..nc {
            for r in 0..nr {
                let zm = 0.5 * (zs[r] + zs[r + 1]);
                kept[i * nr + r] = if wall.side == 1 {
                    self.material(w, i, zm)? == keep_b
                } else {
                    !(self.inside[w - self.nb][i] && zm > 0.0 && zm < self.h)
                };
            }
        }
        if wall.side == 0 && kept.iter().all(|&k| k) {
            return Ok(None);
        }
        let at = |s: f64, zz: f64| wall.foot.eval(s) + self.z * zz;
        let along = |s: f64| wall.foot.deriv(s).d1;
        let k_at = |i: usize, r: usize| kept[i * nr + r];
        // across the grid inside the wall: a vertical line at a crossing, or a face's curve
        let level_ck = |i: usize, zz: f64| -> R<Ck> {
            if wall.side == 1 {
                let l = self.lvl(zz);
                self.over[w][i]
                    .iter()
                    .find(|&&s| self.lvl(self.squares[s].h) == l)
                    .map(|&s| Ck::H(self.squares[s].f, w))
                    .ok_or("a wall's material changes with no face there")
            } else if zz.abs() <= tol::LINEAR {
                Ok(Ck::C(0, w))
            } else if (zz - self.h).abs() <= tol::LINEAR {
                Ok(Ck::C(1, w))
            } else {
                Err("a wall's material changes off the caps")
            }
        };
        let cross_ck = |i: usize| -> Ck {
            match wall.breaks[i].1 {
                Brk::X(x) => Ck::V(x),
                Brk::End(e) => Ck::Edge(wall.side, wall.ups[e]),
            }
        };
        let mut arcs = Vec::new();
        for i in 0..nc {
            let (s0, s1) = (wall.breaks[i].0, wall.breaks[i + 1].0);
            let sm = 0.5 * (s0 + s1);
            for r in 0..nr {
                if !k_at(i, r) {
                    continue;
                }
                let (z0, z1) = (zs[r], zs[r + 1]);
                let zm = 0.5 * (z0 + z1);
                let mut side = |ck: Ck, from: (usize, f64), to: (usize, f64), mid: Point3, dir: Vec3| {
                    let (f, t) = (self.vk(w, from.0, from.1), self.vk(w, to.0, to.1));
                    let row = |zz: f64| zs.iter().position(|&x| x == zz).expect("a row");
                    let (gf, gt) = ((from.0, row(from.1)), (to.0, row(to.1)));
                    let (from, to, dir, g) = if wall.ccw {
                        (f, t, dir, [gf, gt])
                    } else {
                        (t, f, -dir, [gt, gf])
                    };
                    arcs.push(Arc {
                        ck,
                        from,
                        to,
                        mid,
                        dir,
                        grid: Some(g),
                    });
                };
                if r == 0 || !k_at(i, r - 1) {
                    let ck = if r == 0 {
                        Ck::Edge(wall.side, wall.bottom)
                    } else {
                        level_ck(i, z0)?
                    };
                    side(ck, (i, z0), (i + 1, z0), at(sm, z0), along(sm));
                }
                if r + 1 == nr || !k_at(i, r + 1) {
                    let ck = if r + 1 == nr {
                        Ck::Edge(wall.side, wall.top)
                    } else {
                        level_ck(i, z1)?
                    };
                    side(ck, (i + 1, z1), (i, z1), at(sm, z1), -along(sm));
                }
                if i + 1 == nc || !k_at(i + 1, r) {
                    side(cross_ck(i + 1), (i + 1, z0), (i + 1, z1), at(s1, zm), self.z);
                }
                if i == 0 || !k_at(i - 1, r) {
                    side(cross_ck(i), (i, z1), (i, z0), at(s0, zm), -self.z);
                }
            }
        }
        Ok(Some((arcs, zs)))
    }
}

/// What the tool does to `a`, as the general boolean's plan; or why this path leaves it to that
/// boolean.
fn plan(a: &Body, b: &dyn Src, op: Op) -> R<Plan> {
    if op == Op::Intersect {
        return Err("intersect");
    }
    let t = tool(b).ok_or("not an extruded profile")?;
    let z = t.z.get();
    let mut c = Column {
        a,
        b,
        z,
        origin: t.origin,
        h: t.h,
        caps: t.caps,
        levels: vec![0.0, t.h],
        walls: Vec::new(),
        nb: 0,
        squares: Vec::new(),
        xs: Vec::new(),
        over: Vec::new(),
        inside: Vec::new(),
        extra: Vec::new(),
        glued: [None, None],
        carriers: Default::default(),
    };
    // A's faces near the tool first (most declines are found there), then B's walls
    let mut a_walls = Vec::new();
    let mut sides: Vec<FaceId> = Vec::new();
    let mut clear: Vec<FaceId> = Vec::new();
    for f in a.faces_where(|x| x.overlaps(&t.bbox)) {
        let face = a.face(f);
        if let Surface::Plane(pl) = &face.surface
            && pl.normal_vec().dot(z).abs() >= 1.0 - 1e-12
        {
            let h = c.height(pl.frame.origin);
            if h < -CLEAR || h > t.h + CLEAR {
                continue;
            }
            let up = (pl.normal_vec().dot(z) > 0.0) == face.same_sense;
            if h < CLEAR || h > t.h - CLEAR {
                // exactly on a cap's plane, the tool on its far side for a union, its near side
                // for a cut
                let cap = match (h.abs() <= tol::LINEAR, (h - t.h).abs() <= tol::LINEAR) {
                    (true, _) => 0,
                    (_, true) => 1,
                    _ => return Err("a face near a cap's plane"),
                };
                if (op == Op::Subtract) != ((cap == 1) == up) || c.glued[cap].is_some() {
                    return Err("a face on a cap's plane");
                }
                c.glued[cap] = Some(c.squares.len());
            }
            c.squares.push(Square { f, h, up });
            continue;
        }
        if clear_of(a, f, &t) {
            sides.push(f);
            clear.push(f);
            continue;
        }
        sides.push(f);
        let w = c
            .wall(0, f)
            .ok_or("a face near the tool neither square to it nor along it")?;
        let near = |x: f64| x.abs() <= CLEAR || (x - t.h).abs() <= CLEAR;
        if near(w.z[0]) || near(w.z[1]) {
            return Err("a wall ends on a cap's plane");
        }
        if w.z[1] < 0.0 || w.z[0] > t.h {
            continue;
        }
        a_walls.push(w);
    }
    for &f in &t.walls {
        let w = c.wall(1, f).ok_or("a tool wall not along the axis")?;
        c.walls.push(w);
    }
    c.nb = c.walls.len();
    c.walls.append(&mut a_walls);
    // a square face's edges near the tool lie on those faces (not on a neighbour in its plane)
    let reach = t.bbox.grow(CLEAR);
    let mut near = Vec::new();
    for sq in &c.squares {
        near.clear();
        edges_near(a, sq.f, &reach, &mut near);
        for &co in &near {
            let e = a.coedge(co).edge;
            let on_side = a
                .edge(e)
                .coedges
                .iter()
                .any(|&d| sides.contains(&a.lp(a.coedge(d).lp).face));
            if !on_side {
                return Err("a square face's edge near the tool between faces in its plane");
            }
        }
    }
    // the heights things happen at, within the tool
    let mut levels: Vec<f64> = c.squares.iter().map(|s| s.h).collect();
    levels.extend(c.walls[c.nb..].iter().flat_map(|w| w.z));
    levels.extend([0.0, t.h]);
    levels.sort_by(f64::total_cmp);
    levels.dedup_by(|x, y| (*x - *y).abs() <= tol::LINEAR);
    if levels.windows(2).any(|w| w[1] - w[0] <= CLEAR) {
        return Err("two heights too near");
    }
    c.levels = levels;
    // where the footprints cross
    let n = c.walls.len();
    for k in 0..c.nb {
        for j in c.nb..n {
            let (wk, wj) = (&c.walls[k], &c.walls[j]);
            let hits = meet(&wk.foot, wk.range, &wj.foot, wj.range, z).ok_or("walls come near touching")?;
            for (sk, sj, p) in hits {
                let x = c.xs.len();
                c.xs.push(Cross { b: k, a: j, p });
                c.walls[k].breaks.push((sk, Brk::X(x)));
                c.walls[j].breaks.push((sj, Brk::X(x)));
            }
        }
    }
    // no wall's end near another side's wall
    for k in 0..c.nb {
        for j in c.nb..n {
            for (u, v) in [(k, j), (j, k)] {
                let (wu, wv) = (&c.walls[u], &c.walls[v]);
                for s in [wu.range.lo, wu.range.hi] {
                    if gap(&wv.foot, wv.range, wu.foot.eval(s)) <= CLEAR {
                        return Err("a wall's end near the other side's wall");
                    }
                }
            }
        }
    }
    for w in &mut c.walls {
        w.breaks.sort_by(|x, y| x.0.total_cmp(&y.0));
        w.mids = w
            .breaks
            .windows(2)
            .map(|b| {
                let d = w.foot.deriv(0.5 * (b[0].0 + b[1].0));
                (d.p, d.d1)
            })
            .collect();
    }
    // A's stretches inside the tool, the faces over B's
    let base = Plane::new(Frame::from_z(t.origin, t.z));
    let section: Vec<(Curve, Interval)> = c.walls[..c.nb]
        .iter()
        .map(|w| (w.foot.clone(), w.range))
        .collect();
    for j in c.nb..n {
        let mut ins = Vec::new();
        for i in 0..c.walls[j].breaks.len() - 1 {
            let (p, _) = c.stretch_mid(j, i);
            ins.push(in_loop(p, &section, &base).ok_or("a ray grazes the tool's section")?);
        }
        c.inside.push(ins);
    }
    for k in 0..c.nb {
        let mut overs = Vec::new();
        for i in 0..c.walls[k].breaks.len() - 1 {
            let (p, _) = c.stretch_mid(k, i);
            let mut over = Vec::new();
            for (s, sq) in c.squares.iter().enumerate() {
                let q = p + z * sq.h;
                if a.face_box(sq.f).contains(q) && in_planar_face(a, sq.f, q).ok_or("a ray grazes")? {
                    over.push(s);
                }
            }
            over.sort_by(|&x, &y| c.squares[x].h.total_cmp(&c.squares[y].h));
            if over.windows(2).any(|w| c.squares[w[0]].up == c.squares[w[1]].up) {
                return Err("faces out of turn");
            }
            overs.push(over);
        }
        c.over.push(overs);
    }
    // a face the tool stands on holds its whole section, no wall of A crossing it
    for s in c.glued.into_iter().flatten() {
        if !c.xs.is_empty() || c.inside.iter().flatten().any(|&x| x) {
            return Err("a wall of A in a face the tool stands on");
        }
        if !c.over.iter().flatten().all(|o| o.contains(&s)) {
            return Err("the tool's section off the face it stands on");
        }
    }
    // the general boolean cuts an upright where the other face's box holds the crossing, even
    // off that face: a vertex there too, a box edge near it not taken
    let boxed = |bx: &Box3, q: Point3| -> R<bool> {
        let inside = bx.contains(q);
        // a planar face's box is thin across its plane: only its extents count there
        let edge_near = [(bx.x, q.x), (bx.y, q.y), (bx.z, q.z)].iter().any(|&(r, v)| {
            r.width() > 4.0 * CLEAR && ((v - r.lo).abs() <= CLEAR || (v - r.hi).abs() <= CLEAR)
        });
        if inside != bx.grow(CLEAR).contains(q) || inside && edge_near {
            return Err("an upright crosses near a face's box");
        }
        Ok(inside)
    };
    for k in 0..c.nb {
        let wall = &c.walls[k];
        let last = wall.breaks.len() - 2;
        for (i, at, e) in [(0, wall.range.lo, 0), (last, wall.range.hi, 1)] {
            for (s, sq) in c.squares.iter().enumerate() {
                let q = wall.foot.eval(at) + z * sq.h;
                if !c.over[k][i].contains(&s) && boxed(&a.face_box(sq.f), q)? {
                    let u = wall.ups[e];
                    let by = Body::face(a, sq.f).name;
                    c.extra.push((u, 1, Vk::Up(1, u, c.lvl(sq.h)), by));
                }
            }
        }
    }
    let cap_box = |cap: usize| {
        c.walls[..c.nb]
            .iter()
            .fold(Box3::EMPTY, |bx, w| {
                let (curve, range, ..) = b.edge(if cap == 0 { w.bottom } else { w.top });
                bx.hull(&curve.bbox(range))
            })
            .grow(tol::LINEAR)
    };
    for j in c.nb..n {
        let wall = &c.walls[j];
        let last = wall.breaks.len() - 2;
        for (cap, hc) in [(0, 0.0), (1, t.h)] {
            if !(wall.z[0] < hc && hc < wall.z[1]) {
                continue;
            }
            let bx = cap_box(cap);
            for (i, at, e) in [(0, wall.range.lo, 0), (last, wall.range.hi, 1)] {
                let q = wall.foot.eval(at) + z * hc;
                if !c.inside[j - c.nb][i] && boxed(&bx, q)? {
                    let u = wall.ups[e];
                    let by = b.face(c.caps[cap]).2;
                    c.extra.push((u, 0, Vk::Up(0, u, c.lvl(hc)), by));
                }
            }
        }
    }
    for &(u, side, ..) in &c.extra {
        if side == 0 && !(c.nb..n).any(|j| c.touches(j) && c.walls[j].ups.contains(&u)) {
            return Err("a vertex on an upright of a wall left alone");
        }
    }
    // nor on an edge of a face clear of the tool, where it crosses a cap's plane in its box
    for (cap, hc) in [(0, 0.0), (1, t.h)] {
        let bx = cap_box(cap);
        for &f in &clear {
            for &l in &a.face(f).loops {
                for co in a.lp(l).coedges() {
                    let e = a.edge(a.coedge(co).edge);
                    let (h0, h1) = (
                        c.height(e.curve.eval(e.range.lo)),
                        c.height(e.curve.eval(e.range.hi)),
                    );
                    if (h0 - hc) * (h1 - hc) >= 0.0 {
                        continue;
                    }
                    let q = match &e.curve {
                        Curve::Line(_) => e
                            .curve
                            .eval(e.range.lo + (e.range.hi - e.range.lo) * (hc - h0) / (h1 - h0)),
                        _ if a.edge_box(a.coedge(co).edge).overlaps(&bx) => {
                            return Err("a curved edge beside the tool crosses a cap's box");
                        }
                        _ => continue,
                    };
                    if boxed(&bx, q)? {
                        return Err("an edge beside the tool crosses a cap's plane in its box");
                    }
                }
            }
        }
    }
    c.finish(op)
}

/// The vertices along a curve, by parameter.
type Along = Vec<(f64, Vk)>;
/// A curve's first piece, its vertices, whether it closes, its new pieces' name before their ends.
type Run = (usize, Along, bool, Name);

/// A face of the result: where it comes from, its loops, and for a face of A changed in place,
/// the loops it gives up.
struct Out {
    side: usize,
    face: FaceId,
    loops: Vec<Vec<Arc>>,
    extend: bool,
    remove: Vec<LoopId>,
}

impl Column<'_> {
    /// Whether a vertical line reaches crossing vertex `v` (else no curve ends there).
    fn real(&self, v: Vk) -> bool {
        match v {
            Vk::X(x, l) => {
                let aw = &self.walls[self.xs[x].a];
                let h = self.levels[l];
                h >= aw.z[0].max(0.0) - tol::LINEAR && h <= aw.z[1].min(self.h) + tol::LINEAR
            }
            _ => true,
        }
    }

    /// Whether A's wall `j` changes: the tool crosses it or holds some of it.
    fn touches(&self, j: usize) -> bool {
        self.walls[j].breaks.len() > 2 || self.inside[j - self.nb].iter().any(|&x| x)
    }

    /// Curve `ck`'s carrier, worked out once (`sample`: a point on it, for a new curve).
    fn carrier(&self, ck: Ck, sample: Point3) -> R<std::rc::Rc<Carrier>> {
        if let Some(c) = self.carriers.borrow().iter().find(|c| c.0 == ck) {
            return Ok(c.1.clone());
        }
        let c = std::rc::Rc::new(self.make_carrier(ck, sample)?);
        self.carriers.borrow_mut().push((ck, c.clone()));
        Ok(c)
    }

    fn make_carrier(&self, ck: Ck, sample: Point3) -> R<Carrier> {
        let new = |sa: &Surface, sb: &Surface, na: Name, nb: Name, near: Point3| -> R<Carrier> {
            let SurfaceSurface::Curves(cs) = crate::ssi::closed_form(sa, sb) else {
                return Err("faces meet in no curve");
            };
            let far = |c: &Curve| match c {
                Curve::Line(l) => {
                    let off = near - l.origin;
                    (off - l.dir.get() * off.dot(l.dir.get())).length()
                }
                c => {
                    let d = c.domain();
                    let w = Interval::new(d.lo, d.lo + d.period.unwrap_or(d.hi - d.lo));
                    gap(c, w, near)
                }
            };
            let ci = (0..cs.len())
                .min_by(|&x, &y| far(&cs[x].0).total_cmp(&far(&cs[y].0)))
                .ok_or("faces meet in no curve")?;
            let curve = cs[ci].0.clone();
            let d = curve.domain();
            let (period, window) = match d.period {
                Some(p) => (Some(p), Interval::new(d.lo, d.lo + p)),
                None => (None, Interval::new(f64::NEG_INFINITY, f64::INFINITY)),
            };
            static CUT: std::sync::OnceLock<Name> = std::sync::OnceLock::new();
            let cut = *CUT.get_or_init(|| Name::feature(0).with_str("cut"));
            let base = cut.with_set(&[na.whole(), nb.whole()]);
            Ok(Carrier {
                curve,
                period,
                window,
                base,
            })
        };
        let face = |side: usize, f: FaceId| self.src(side).face(f);
        match ck {
            Ck::Edge(side, e) => {
                let (curve, range, s, t, _) = self.src(side).edge(e);
                let period = curve.domain().period.filter(|_| s == t);
                Ok(Carrier {
                    curve: curve.clone(),
                    period,
                    window: range,
                    base: Name::NONE,
                })
            }
            Ck::V(x) => {
                let (fa, fb) = (
                    face(0, self.walls[self.xs[x].a].face),
                    face(1, self.walls[self.xs[x].b].face),
                );
                new(fa.0, fb.0, fa.2, fb.2, self.xs[x].p)
            }
            Ck::H(f, k) => {
                let (fa, fb) = (face(0, f), face(1, self.walls[k].face));
                new(fa.0, fb.0, fa.2, fb.2, sample)
            }
            Ck::C(cap, j) => {
                let (fa, fb) = (face(0, self.walls[j].face), face(1, self.caps[cap]));
                new(fa.0, fb.0, fa.2, fb.2, sample)
            }
        }
    }

    /// The two faces a new curve lies on.
    fn faces_of(&self, ck: Ck) -> ((&Surface, bool, Name), (&Surface, bool, Name)) {
        let face = |side: usize, f: FaceId| self.src(side).face(f);
        match ck {
            Ck::V(x) => (
                face(0, self.walls[self.xs[x].a].face),
                face(1, self.walls[self.xs[x].b].face),
            ),
            Ck::H(f, k) => (face(0, f), face(1, self.walls[k].face)),
            Ck::C(cap, j) => (face(0, self.walls[j].face), face(1, self.caps[cap])),
            Ck::Edge(..) => unreachable!("an edge's own pieces are named by the edge"),
        }
    }

    /// The edge a new curve is cut by at vertex `v` (as the general boolean names its pieces).
    fn cut_edge(&self, ck: Ck, v: Vk) -> R<(usize, EdgeId)> {
        let h = |l: usize| self.levels[l];
        let at = |l: usize, z: f64| (h(l) - z).abs() <= tol::LINEAR;
        match (ck, v) {
            (Ck::V(x), Vk::X(_, l)) => {
                let (bw, aw) = (&self.walls[self.xs[x].b], &self.walls[self.xs[x].a]);
                Ok(if at(l, 0.0) {
                    (1, bw.bottom)
                } else if at(l, self.h) {
                    (1, bw.top)
                } else if at(l, aw.z[0]) {
                    (0, aw.bottom)
                } else {
                    (0, aw.top)
                })
            }
            (Ck::H(..), Vk::X(x, l)) => {
                let aw = &self.walls[self.xs[x].a];
                Ok((0, if at(l, aw.z[0]) { aw.bottom } else { aw.top }))
            }
            (Ck::C(cap, _), Vk::X(x, _)) => {
                let bw = &self.walls[self.xs[x].b];
                Ok((1, if cap == 0 { bw.bottom } else { bw.top }))
            }
            (Ck::H(..) | Ck::C(..), Vk::Up(side, u, _)) => Ok((side, u)),
            _ => Err("a new curve ends off any edge"),
        }
    }

    fn param(&self, car: &Carrier, p: Point3) -> f64 {
        match (&car.curve, car.period) {
            (Curve::Line(l), _) => (p - l.origin).dot(l.dir.get()),
            (c, Some(per)) => {
                let t = c.closest(p, car.window);
                let t = car.window.lo + (t - car.window.lo).rem_euclid(per);
                if car.window.lo + per - t <= 1e-12 {
                    car.window.lo
                } else {
                    t
                }
            }
            (c, None) => c.closest(p, car.window),
        }
    }

    /// An arc's parameters from its start to its end on its carrier, the way it runs.
    fn span(&self, car: &Carrier, a: &Arc) -> (f64, f64) {
        let (mut t0, mut t1) = (
            self.param(car, self.point(a.from)),
            self.param(car, self.point(a.to)),
        );
        let fwd = car.curve.deriv(self.param(car, a.mid)).d1.dot(a.dir) > 0.0;
        if let Some(p) = car.period {
            if fwd && t1 <= t0 + 1e-12 {
                t1 += p;
            } else if !fwd && t1 >= t0 - 1e-12 {
                t1 -= p;
            }
        }
        if a.from == a.to && car.period.is_none() {
            t0 = t1;
        }
        (t0, t1)
    }

    /// A loop's area about normal `n` (positive counterclockwise).
    fn area(&self, cyc: &[Arc], n: Vec3) -> R<f64> {
        let mut sum = 0.0;
        for a in cyc {
            let car = self.carrier(a.ck, a.mid)?;
            let (t0, t1) = self.span(&car, a);
            sum += swept(&car.curve, t0, t1, self.origin, n);
        }
        Ok(sum)
    }

    fn arcs_of_square(&self, si: usize) -> R<Option<(Vec<Arc>, Vec<LoopId>)>> {
        let (a, sq) = (self.a, &self.squares[si]);
        let nf = if sq.up { self.z } else { -self.z };
        let mut arcs = Vec::new();
        // the tool's stretches over the face, the face's side outside the tool on their left
        for k in 0..self.nb {
            for i in 0..self.walls[k].breaks.len() - 1 {
                if !self.over[k][i].contains(&si) {
                    continue;
                }
                let (p, d) = self.stretch_mid(k, i);
                let mid = p + self.z * sq.h;
                let nb = self.face_out(1, self.walls[k].face, mid)?;
                let (f, t) = (self.vk(k, i, sq.h), self.vk(k, i + 1, sq.h));
                let (from, to, dir) = if nf.cross(d).dot(nb) > 0.0 {
                    (f, t, d)
                } else {
                    (t, f, -d)
                };
                let ck = match self.glued.iter().position(|&g| g == Some(si)) {
                    Some(0) => Ck::Edge(1, self.walls[k].bottom),
                    Some(_) => Ck::Edge(1, self.walls[k].top),
                    None => Ck::H(sq.f, k),
                };
                arcs.push(Arc {
                    ck,
                    from,
                    to,
                    mid,
                    dir,
                    grid: None,
                });
            }
        }
        // the loops holding A's walls' curves the tool cuts or covers
        let mut touched: Vec<LoopId> = Vec::new();
        let mut cut: Vec<(EdgeId, usize)> = Vec::new();
        for j in self.nb..self.walls.len() {
            if !self.touches(j) {
                continue;
            }
            let w = &self.walls[j];
            for (e, zz) in [(w.bottom, w.z[0]), (w.top, w.z[1])] {
                if (zz - sq.h).abs() > tol::LINEAR {
                    continue;
                }
                for &c in &a.edge(e).coedges {
                    let lp = a.coedge(c).lp;
                    if a.lp(lp).face == sq.f {
                        if !touched.contains(&lp) {
                            touched.push(lp);
                        }
                        cut.push((e, j));
                    }
                }
            }
        }
        if arcs.is_empty() && touched.is_empty() {
            return Ok(None);
        }
        for &lp in &touched {
            for c in a.lp(lp).coedges() {
                let (e, fwd) = (a.coedge(c).edge, a.coedge(c).forward);
                let edge = a.edge(e);
                let Some(&(_, j)) = cut.iter().find(|x| x.0 == e) else {
                    let (s, t) = (Vk::Old(0, edge.start), Vk::Old(0, edge.end));
                    let dv = edge.curve.deriv(edge.range.mid());
                    let (from, to, dir) = if fwd { (s, t, dv.d1) } else { (t, s, -dv.d1) };
                    arcs.push(Arc {
                        ck: Ck::Edge(0, e),
                        from,
                        to,
                        mid: dv.p,
                        dir,
                        grid: None,
                    });
                    continue;
                };
                let n = self.walls[j].breaks.len() - 1;
                let (p, d) = self.stretch_mid(j, 0);
                let q = p + self.z * sq.h;
                let inc = edge.curve.deriv(edge.curve.closest(q, edge.range)).d1.dot(d) > 0.0;
                let along = fwd == inc;
                for k in 0..n {
                    let i = if along { k } else { n - 1 - k };
                    if self.inside[j - self.nb][i] {
                        continue;
                    }
                    let (p, d) = self.stretch_mid(j, i);
                    let (f, t) = (self.vk(j, i, sq.h), self.vk(j, i + 1, sq.h));
                    let (from, to, dir) = if along { (f, t, d) } else { (t, f, -d) };
                    arcs.push(Arc {
                        ck: Ck::Edge(0, e),
                        from,
                        to,
                        mid: p + self.z * sq.h,
                        dir,
                        grid: None,
                    });
                }
            }
        }
        Ok(Some((arcs, touched)))
    }

    fn arcs_of_cap(&self, cap: usize, keep_b: bool) -> R<Vec<Arc>> {
        let hc = if cap == 0 { 0.0 } else { self.h };
        let nc = self.face_out(1, self.caps[cap], self.origin + self.z * hc)?;
        let mut arcs = Vec::new();
        for k in 0..self.nb {
            let wall = &self.walls[k];
            for i in 0..wall.breaks.len() - 1 {
                if self.material(k, i, hc)? != keep_b {
                    continue;
                }
                let (p, d) = self.stretch_mid(k, i);
                let mid = p + self.z * hc;
                let nb = self.face_out(1, wall.face, mid)?;
                let (f, t) = (self.vk(k, i, hc), self.vk(k, i + 1, hc));
                let (from, to, dir) = if nc.cross(d).dot(nb) < 0.0 {
                    (f, t, d)
                } else {
                    (t, f, -d)
                };
                let e = if cap == 0 { wall.bottom } else { wall.top };
                arcs.push(Arc {
                    ck: Ck::Edge(1, e),
                    from,
                    to,
                    mid,
                    dir,
                    grid: None,
                });
            }
        }
        for j in self.nb..self.walls.len() {
            let wall = &self.walls[j];
            if !(wall.z[0] < hc && hc < wall.z[1]) {
                continue;
            }
            for i in 0..wall.breaks.len() - 1 {
                if !self.inside[j - self.nb][i] {
                    continue;
                }
                let (p, d) = self.stretch_mid(j, i);
                let mid = p + self.z * hc;
                let na = self.face_out(0, wall.face, mid)?;
                let left = nc.cross(d).dot(na);
                let (f, t) = (self.vk(j, i, hc), self.vk(j, i + 1, hc));
                let (from, to, dir) = if (left < 0.0) == keep_b {
                    (f, t, d)
                } else {
                    (t, f, -d)
                };
                arcs.push(Arc {
                    ck: Ck::C(cap, j),
                    from,
                    to,
                    mid,
                    dir,
                    grid: None,
                });
            }
        }
        Ok(arcs)
    }

    fn finish(&self, op: Op) -> R<Plan> {
        let a = self.a;
        let keep_b = op == Op::Subtract;
        let mut outs: Vec<Out> = Vec::new();
        let mut involved: Vec<FaceId> = Vec::new();
        for w in 0..self.walls.len() {
            let Some((arcs, zs)) = self.wall_arcs(w, keep_b)? else {
                continue;
            };
            let wall = &self.walls[w];
            if wall.side == 0 {
                involved.push(wall.face);
            }
            // each loop's corners along the foot and up, counterclockwise for an outer loop
            let corners = |c: &[Arc]| -> Vec<(f64, f64)> {
                c.iter()
                    .map(|a| {
                        let g = a.grid.expect("a wall's arcs are on its grid")[0];
                        (wall.breaks[g.0].0, zs[g.1])
                    })
                    .collect()
            };
            let (mut outer, mut holes): (Vec<Vec<Vec<Arc>>>, Vec<Vec<Arc>>) = (Vec::new(), Vec::new());
            for cyc in cycles(arcs)? {
                let poly = corners(&cyc);
                let area: f64 = (0..poly.len())
                    .map(|i| {
                        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
                        p.0 * q.1 - q.0 * p.1
                    })
                    .sum();
                if (area > 0.0) == wall.ccw {
                    outer.push(vec![cyc]);
                } else {
                    holes.push(cyc);
                }
            }
            for hole in holes {
                let (x, y) = corners(&hole)[0];
                // the outer loop round the hole's corner (moved a hair into it)
                let k = (0..outer.len())
                    .find(|&k| {
                        let poly = corners(&outer[k][0]);
                        let (x, y) = (x + 1e-9 * (1.0 + x.abs()), y + 1e-9 * (1.0 + y.abs()));
                        let mut inside = false;
                        for i in 0..poly.len() {
                            let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
                            if (p.1 > y) != (q.1 > y) && x < p.0 + (y - p.1) * (q.0 - p.0) / (q.1 - p.1) {
                                inside = !inside;
                            }
                        }
                        inside
                    })
                    .ok_or("a hole in no piece of a wall")?;
                outer[k].push(hole);
            }
            for loops in outer {
                outs.push(Out {
                    side: wall.side,
                    face: wall.face,
                    loops,
                    extend: false,
                    remove: Vec::new(),
                });
            }
        }
        for si in 0..self.squares.len() {
            let Some((arcs, touched)) = self.arcs_of_square(si)? else {
                continue;
            };
            let sq = &self.squares[si];
            let nf = if sq.up { self.z } else { -self.z };
            let mut cyc = cycles(arcs)?;
            // the tool's stretches alone close only holes; an outer cycle goes first
            let mut new_outer = 0;
            for k in 0..cyc.len() {
                if !touched.is_empty() && self.area(&cyc[k], nf)? > 0.0 {
                    cyc[new_outer..=k].rotate_right(1);
                    new_outer += 1;
                }
            }
            let mut old_outer = 0;
            for &lp in &touched {
                let mut sum = 0.0;
                for c in a.lp(lp).coedges() {
                    let e = a.edge(a.coedge(c).edge);
                    let (t0, t1) = (e.range.lo, e.range.hi);
                    let (t0, t1) = if a.coedge(c).forward { (t0, t1) } else { (t1, t0) };
                    sum += swept(&e.curve, t0, t1, self.origin, nf);
                }
                old_outer += usize::from(sum > 0.0);
            }
            involved.push(sq.f);
            if new_outer == old_outer {
                outs.push(Out {
                    side: 0,
                    face: sq.f,
                    loops: cyc,
                    extend: true,
                    remove: touched,
                });
            } else if !(new_outer == 0 && cyc.is_empty() && a.face(sq.f).loops.len() == touched.len()) {
                return Err("a face splits in two");
            }
        }
        for cap in (0..2).filter(|&cap| self.glued[cap].is_none()) {
            let arcs = self.arcs_of_cap(cap, keep_b)?;
            if arcs.is_empty() {
                continue;
            }
            let nc = self.face_out(
                1,
                self.caps[cap],
                self.origin + self.z * if cap == 0 { 0.0 } else { self.h },
            )?;
            let (mut outer, mut holes) = (Vec::new(), Vec::new());
            // the tool's stretches alone close only the cap's outline
            let walls_in = arcs.iter().any(|a| matches!(a.ck, Ck::C(..)));
            for c in cycles(arcs)? {
                if !walls_in || self.area(&c, nc)? > 0.0 {
                    outer.push(vec![c]);
                } else {
                    holes.push(c);
                }
            }
            let plane = Plane::new(Frame::from_z(self.origin, UnitVec3::new_unchecked(nc)));
            for hole in holes {
                let k = if outer.len() == 1 {
                    0
                } else {
                    let mut found = None;
                    for (k, o) in outer.iter().enumerate() {
                        let mut pieces = Vec::new();
                        for arc in &o[0] {
                            let car = self.carrier(arc.ck, arc.mid)?;
                            let (t0, t1) = self.span(&car, arc);
                            pieces.push((car.curve.clone(), Interval::new(t0.min(t1), t0.max(t1))));
                        }
                        if in_loop(hole[0].mid, &pieces, &plane).ok_or("a ray grazes")? {
                            found = Some(k);
                        }
                    }
                    found.ok_or("a hole in no piece of a cap")?
                };
                outer[k].push(hole);
            }
            for loops in outer {
                outs.push(Out {
                    side: 1,
                    face: self.caps[cap],
                    loops,
                    extend: false,
                    remove: Vec::new(),
                });
            }
        }
        self.assemble(outs, involved, op)
    }

    fn assemble(&self, outs: Vec<Out>, involved: Vec<FaceId>, op: Op) -> R<Plan> {
        let a = self.a;
        // each curve's carrier and the vertices on it, by parameter
        let mut curves: std::collections::BTreeMap<Ck, (std::rc::Rc<Carrier>, Along)> = Default::default();
        for arc in outs.iter().flat_map(|o| o.loops.iter().flatten()) {
            if let std::collections::btree_map::Entry::Vacant(slot) = curves.entry(arc.ck) {
                let car = self.carrier(arc.ck, arc.mid)?;
                let mut pts = Vec::new();
                if let Ck::Edge(side, e) = arc.ck {
                    let (_, range, s, t, _) = self.src(side).edge(e);
                    pts.push((range.lo, Vk::Old(side, s)));
                    if t != s {
                        pts.push((range.hi, Vk::Old(side, t)));
                    }
                }
                slot.insert((car, pts));
            }
            let (car, pts) = curves.get(&arc.ck).expect("just put");
            let mut add = Vec::new();
            // a whole closed curve starts anywhere: its start is no vertex of its own
            let ends = if arc.from == arc.to && !self.real(arc.from) {
                &[][..]
            } else {
                &[arc.from, arc.to][..]
            };
            for &v in ends {
                if !pts.iter().any(|p| p.1 == v) && !add.iter().any(|p: &(f64, Vk)| p.1 == v) {
                    add.push((self.param(car, self.point(v)), v));
                }
            }
            curves.get_mut(&arc.ck).expect("just put").1.extend(add);
        }
        for &(u, side, v, _) in &self.extra {
            if let Some((car, pts)) = curves.get_mut(&Ck::Edge(side, u))
                && !pts.iter().any(|p| p.1 == v)
            {
                let t = match &car.curve {
                    Curve::Line(l) => (self.point(v) - l.origin).dot(l.dir.get()),
                    c => c.closest(self.point(v), car.window),
                };
                pts.push((t, v));
            }
        }
        let mut new_ids: std::collections::BTreeMap<Vk, u32> = Default::default();
        let mut points: Vec<Point3> = Vec::new();
        let mut key = |v: Vk, points: &mut Vec<Point3>| match v {
            Vk::Old(s, id) => VKey::Old(s, id),
            v => VKey::New(*new_ids.entry(v).or_insert_with(|| {
                points.push(self.point(v));
                points.len() as u32 - 1
            })),
        };
        let mut pieces: Vec<Piece> = Vec::new();
        // per curve: first piece, the vertices along it
        let mut runs: std::collections::BTreeMap<Ck, Run> = Default::default();
        for (ck, (car, mut pts)) in curves {
            pts.sort_by(|x, y| x.0.total_cmp(&y.0));
            let closed = car.period.is_some();
            let m = pts.len();
            let count = if closed { m } else { m - 1 };
            let first = pieces.len();
            for i in 0..count {
                let (t0, v0) = pts[i];
                let (mut t1, v1) = pts[(i + 1) % m];
                if i + 1 == m {
                    t1 += car.period.expect("closed");
                }
                let (side, whole, of) = match ck {
                    Ck::Edge(s, e) => (Some(s), (count == 1).then_some((s, e)), (count > 1).then_some(e)),
                    _ => (None, None, None),
                };
                let name = match ck {
                    Ck::Edge(s, e) => self.src(s).edge(e).4,
                    _ => Name::NONE,
                };
                pieces.push(Piece {
                    curve: car.curve.clone(),
                    range: Interval::new(t0, t1),
                    start: key(v0, &mut points),
                    end: key(v1, &mut points),
                    name,
                    seg: (side.is_none()).then_some(0),
                    whole,
                    side,
                    of,
                });
            }
            runs.insert(ck, (first, pts, closed, car.base));
        }
        // the faces' loops as pieces
        let mut specs: Vec<Spec> = Vec::new();
        for o in &outs {
            let mut loops = Vec::new();
            for cyc in &o.loops {
                let mut lp = Vec::new();
                for arc in cyc {
                    let (first, pts, closed, _) = &runs[&arc.ck];
                    let m = pts.len();
                    let find = |v: Vk| pts.iter().position(|p| p.1 == v).expect("a vertex on its curve");
                    let (i0, i1) = if arc.from == arc.to && !self.real(arc.from) {
                        (0, 0)
                    } else {
                        (find(arc.from), find(arc.to))
                    };
                    let car = self.carrier(arc.ck, arc.mid)?;
                    let fwd = car.curve.deriv(self.param(&car, arc.mid)).d1.dot(arc.dir) > 0.0;
                    let mut i = i0;
                    loop {
                        if fwd {
                            if !closed && i + 1 >= m {
                                return Err("an arc runs off its curve");
                            }
                            lp.push((first + i, true));
                            i = (i + 1) % m;
                        } else {
                            if !closed && i == 0 {
                                return Err("an arc runs off its curve");
                            }
                            i = (i + m - 1) % m;
                            lp.push((first + i, false));
                        }
                        if i == i1 {
                            break;
                        }
                    }
                }
                loops.push(lp);
            }
            specs.push(Spec {
                side: o.side,
                from: o.face,
                loops,
                name: Name::NONE,
                foreign: Vec::new(),
                whole: false,
                extend: o.extend,
                remove: o.remove.clone(),
                preset: None,
            });
        }
        let mut uses = vec![0u8; pieces.len()];
        for &(p, _) in specs.iter().flat_map(|s| s.loops.iter().flatten()) {
            uses[p] = uses[p].saturating_add(1);
        }
        if uses.iter().any(|&u| u > 2) {
            return Err("an edge four faces meet at");
        }
        // an edge of A cut here belongs only to faces rebuilt here
        for (ck, (first, pts, closed, _)) in &runs {
            if let Ck::Edge(0, e) = *ck {
                let n = if *closed { pts.len() } else { pts.len() - 1 };
                let _ = first;
                if n > 1
                    && a.edge(e)
                        .coedges
                        .iter()
                        .any(|&c| !involved.contains(&a.lp(a.coedge(c).lp).face))
                {
                    return Err("an edge cut here bounds a face left alone");
                }
            }
        }
        // new curves' pieces named by the edges they run between, the vertices by those
        let mut marks: Vec<(u32, Name, Name)> = Vec::new();
        let mut keys: Vec<(Name, usize, Name, Name, Name, Ck)> = Vec::new();
        for (ck, (first, pts, closed, base)) in &runs {
            if matches!(ck, Ck::Edge(..)) {
                continue;
            }
            let n = if *closed { pts.len() } else { pts.len() - 1 };
            // as the general boolean's `trim`: by the edges a piece runs between, either way round
            for i in 0..n {
                if uses[first + i] == 0 {
                    continue;
                }
                let (v0, v1) = (pts[i].1, pts[(i + 1) % pts.len()].1);
                let (s0, e0) = self.cut_edge(*ck, v0)?;
                let (s1, e1) = self.cut_edge(*ck, v1)?;
                let (a, b) = (self.src(s0).edge(e0).4.whole(), self.src(s1).edge(e1).4.whole());
                let name = base.with(0x5e7).with_name(a.min(b)).with_name(a.max(b));
                keys.push((*base, first + i, a, b, name, *ck));
            }
        }
        // pieces alike, of one pair of faces on any of its curves, ranked by where they lie in the
        // frame of the lesser whole's surface (`rank_pairs`)
        // (the first in the general boolean's order of face pairs says which frame)
        let of_a = |ck: Ck| match ck {
            Ck::V(x) => self.walls[self.xs[x].a].face,
            Ck::H(f, _) => f,
            Ck::C(_, j) => self.walls[j].face,
            Ck::Edge(..) => unreachable!("an edge's own pieces are named by the edge"),
        };
        keys.sort_unstable_by_key(|k| (k.0, of_a(k.5), k.1));
        for g in keys.chunk_by(|x, y| x.0 == y.0) {
            // the vertices between two pieces alike on one curve tie by which way the curve runs,
            // and this path's carrier may run the other way
            if (0..g.len()).any(|i| (0..i).any(|j| (g[i].4, g[i].5) == (g[j].4, g[j].5))) {
                return Err("two pieces of one curve between the same edges");
            }
            let mut names: Vec<Name> = g.iter().map(|k| k.4).collect();
            let (f0, f1) = self.faces_of(g[0].5);
            let frame = surface_frame(if f0.2.whole() <= f1.2.whole() { f0.0 } else { f1.0 });
            rank_names(&mut names, &|k| {
                let pc = &pieces[g[k].1];
                frame.point_to_local(curve_at(&pc.curve, pc.range, true))
            });
            for (k, &name) in g.iter().zip(&names) {
                let (base, p, a, b) = (k.0, k.1, k.2, k.3);
                pieces[p].name = name;
                if let VKey::New(i) = pieces[p].start {
                    marks.push((i, base.with_name(a), name.with(0)));
                }
                if let VKey::New(i) = pieces[p].end {
                    marks.push((i, base.with_name(b), name.with(1)));
                }
            }
        }
        // a vertex no new curve ends at is named by the edge it cuts
        for &(u, side, v, by) in &self.extra {
            if let Some(&i) = new_ids.get(&v)
                && !marks.iter().any(|m| m.0 == i && m.1 != m.2)
            {
                static CUT: std::sync::OnceLock<Name> = std::sync::OnceLock::new();
                let cut = *CUT.get_or_init(|| Name::feature(0).with_str("cut"));
                let n = self.src(side).edge(u).4.whole();
                let walls = if side == 1 {
                    0..self.nb
                } else {
                    self.nb..self.walls.len()
                };
                for w in &self.walls[walls] {
                    if w.ups.contains(&u) {
                        let m = cut
                            .with_set(&[self.src(side).face(w.face).2.whole(), by.whole()])
                            .with_name(n);
                        marks.push((i, m, m));
                    }
                }
            }
        }
        let vnames = vertex_names(&points, marks, Name::feature(0), &part_frame(self.a), None);
        let vname = |v: VKey| match v {
            VKey::Old(s, id) => self.src(s).vertex(id).2,
            VKey::New(i) => vnames[i as usize],
        };
        for (ck, (first, pts, closed, _)) in &runs {
            let Ck::Edge(s, e) = *ck else { continue };
            let n = if *closed { pts.len() } else { pts.len() - 1 };
            let used: Vec<usize> = (*first..first + n).filter(|&p| uses[p] > 0).collect();
            if n > 1 && used.len() > 1 {
                let edge = self.src(s).edge(e).4;
                for p in used {
                    pieces[p].name = piece_name(edge, &pieces[p], &vname);
                }
            }
        }
        // a face of which one piece is kept keeps its name
        let mut at = 0;
        while at < specs.len() {
            let mut end = at + 1;
            while end < specs.len() && (specs[end].side, specs[end].from) == (specs[at].side, specs[at].from)
            {
                end += 1;
            }
            let name = self.src(specs[at].side).face(specs[at].from).2;
            for s in &mut specs[at..end] {
                s.name = if end == at + 1 {
                    name
                } else {
                    region_name(name, &s.loops, &pieces)
                };
            }
            at = end;
        }
        // edges of a cap flush on a face are named as the cuts they become, as the general boolean's
        let mut flush = Vec::new();
        let pairs: Vec<[(usize, FaceId); 2]> = (0..2)
            .filter_map(|cap| Some([(0, self.squares[self.glued[cap]?].f), (1, self.caps[cap])]))
            .collect();
        if !pairs.is_empty() {
            let tool = self.b.body();
            let whole = |sd: usize, e: EdgeId| match runs.get(&Ck::Edge(sd, e)) {
                Some((first, pts, closed, _)) if pts.len() - usize::from(!*closed) == 1 => Some(*first),
                _ => None,
            };
            let kept_faces = specs.iter().map(|s| (s.side, s.from)).collect();
            flush = crate::boolean::flush_edges_as_cuts(
                &[self.a, &*tool],
                kept_faces,
                &mut pieces,
                &uses,
                &pairs,
                &whole,
                Name::feature(0),
            );
        }
        let kept = specs
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.side == 1 && op == Op::Subtract))
            .collect();
        let tols = vec![tol::LINEAR; points.len()];
        Ok(Plan {
            pieces,
            split: Vec::new(),
            survivors: Vec::new(),
            specs,
            kept,
            points,
            vnames,
            flush,
            tols,
            grown: Vec::new(),
            alias: Vec::new(),
            wholes: Vec::new(),
            involved,
            untouched_kept: Vec::new(),
        })
    }
}
/// Where two bodies differ, for the check mode (None: the same). Entities are matched by name,
/// not by id or order: each face's surface and sense, its loops as cycles of (edge name, way),
/// each edge's curve kind, its ends' names and its curve at both ends and its middle, points to
/// within 1e-9.
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
                // the outer loop must come first; holes in any order
                if !loops.is_empty() {
                    loops[1..].sort();
                }
                (face.name, face.same_sense, format!("{:?}", face.surface), loops)
            })
            .collect();
        out.sort_by(|p, q| (p.0, &p.3).cmp(&(q.0, &q.3)));
        out
    };
    let (fx, fy) = (faces(x), faces(y));
    if fx.len() != fy.len() {
        let only = |p: &[(Name, bool, String, Vec<Loop>)], q: &[(Name, bool, String, Vec<Loop>)]| {
            p.iter()
                .filter(|f| !q.iter().any(|g| g.0 == f.0))
                .map(|f| format!("{:?} {:?}", f.0, f.3))
                .collect::<Vec<_>>()
        };
        return Some(format!(
            "{} faces vs {}: only here {:?}, only there {:?}",
            fx.len(),
            fy.len(),
            only(&fx, &fy),
            only(&fy, &fx)
        ));
    }
    for (p, q) in fx.iter().zip(&fy) {
        if p != q {
            return Some(format!("face {:?}: {:?} vs {:?}", p.0, p, q));
        }
    }
    // every edge the faces use, by name: its kind and its ends
    // every edge by name: its kind, its ends' names, and its curve at both ends and between
    let edges = |b: &Body| {
        type Seen = (Name, Name, Name, std::mem::Discriminant<Curve>, [Point3; 3]);
        let mut out: Vec<Seen> = b
            .edge_ids()
            .map(|e| {
                let edge = b.edge(e);
                let (s, t) = (b.vertex(edge.start), b.vertex(edge.end));
                let r = edge.range;
                let at = [r.lo, r.mid(), r.hi].map(|u| edge.curve.eval(u));
                (edge.name, s.name, t.name, std::mem::discriminant(&edge.curve), at)
            })
            .collect();
        out.sort_by_key(|e| (e.0, e.1, e.2));
        out
    };
    let (ex, ey) = (edges(x), edges(y));
    if ex.len() != ey.len() {
        return Some(format!("{} edges vs {}", ex.len(), ey.len()));
    }
    for (p, q) in ex.iter().zip(&ey) {
        if (p.0, p.1, p.2, p.3) != (q.0, q.1, q.2, q.3) || (0..3).any(|i| !close(p.4[i], q.4[i])) {
            return Some(format!("edge {:?}: {:?} vs {:?}", p.0, p, q));
        }
    }
    None
}
