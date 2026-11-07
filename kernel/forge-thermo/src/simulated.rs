//! The formed part as the thickness model leaves it: the sheet's surface on the tool, trimmed to
//! the trim outline, and the same surface moved off the tool by the wall the model gives at each
//! point, closed round the trim by a rim. It is fitted from the model's grid (a mesh, not a
//! B-rep): for mass, for looking at and for interference, never for dimensions or joints.
use crate::stretch::Stretch;
use forge_math::{Frame, Point2, Point3, Vec3};
use forge_mesh::TriMesh;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// A quick hash for the maps here, all looked up by key only (never iterated): which hash does
/// not change what they answer.
#[derive(Default)]
struct Quick(u64);

impl Hasher for Quick {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }
    fn write_u32(&mut self, i: u32) {
        self.write_u64(i as u64);
    }
    fn write_i64(&mut self, i: i64) {
        self.write_u64(i as u64);
    }
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

type Fast = BuildHasherDefault<Quick>;

/// Which side of the simulated wall a triangle is on (`TriMesh::face_of`).
pub const TOOL_SIDE: u32 = 0;
pub const FREE_SIDE: u32 = 1;
pub const RIM: u32 = 2;

/// Volume, centre and inertia of a closed mesh, unit density.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshMass {
    pub volume: f64,
    pub centroid: Point3,
    /// About the centroid; diagonal the moments, off it the negated products (as forge-topo's).
    pub inertia: [[f64; 3]; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Simulated {
    /// Closed and facing out; `face_of` says which side each triangle is on.
    pub mesh: TriMesh,
    /// The wall at each vertex, mm, and its foot: the point on the tool side it stands off.
    pub wall: Vec<f64>,
    pub foot: Vec<Point3>,
    /// The model's wall, unsmoothed, at each vertex's point of the modelled part: what the colours
    /// show, the same on the modelled part and on this.
    pub raw: Vec<f64>,
    /// Each vertex's point of the model's sheet (its modelled point's nearest): where its colour
    /// comes from.
    pub sheet: Vec<Point3>,
    /// Each vertex's point on the modelled part: the colours are looked up from it.
    pub modelled: Vec<Point3>,
    /// The tool side's area, and the integral of the wall over the wall's middle surface (what
    /// the membrane conserves): the mesh's volume is within 0.5 % of it on formed parts.
    pub area: f64,
    pub sheet_volume: f64,
    pub mass: MeshMass,
}

/// The simulated part of `st`, a sheet `t` thick formed with its trim plane `frame` (z the pull)
/// and trimmed to `trim` (on the plane's axes).
pub fn simulated(frame: &Frame, trim: &[Point2], st: &Stretch, t: f64) -> Simulated {
    let sheet = Sheet {
        nodes: st.nodes.clone(),
        tris: st.tris.clone(),
        ratio: st.node_ratio.clone(),
        touched: st.touched.clone(),
    };
    fitted(frame, trim, &sheet, t)
}

/// The tool the sheet lies on: its surface's mesh (world) and the triangles the sheet meets.
pub struct OnTool<'a> {
    pub mesh: &'a TriMesh,
    pub contact: &'a [usize],
}

/// As `simulated`, finer: each of the model's triangles split in four `levels` times, and every
/// point the sheet touched put on the tool (or the table, the trim plane) where it lies within a
/// sheet's thickness of it: the model leaves it up to a quarter of t off, and its grid's
/// straight sides across the tool's rounds.
pub fn simulated_on(
    frame: &Frame,
    trim: &[Point2],
    st: &Stretch,
    t: f64,
    tool: &OnTool,
    levels: usize,
) -> Simulated {
    let mut sheet = Sheet {
        nodes: st.nodes.clone(),
        tris: st.tris.clone(),
        ratio: st.node_ratio.clone(),
        touched: st.touched.clone(),
    };
    for _ in 0..levels {
        sheet = sheet.split();
    }
    let near = Near::new(tool, 2.0 * t);
    for k in 0..sheet.nodes.len() {
        if !sheet.touched[k] {
            continue;
        }
        let p = sheet.nodes[k];
        let l = frame.point_to_local(p);
        // the table: the trim plane, under the sheet's flange
        let mut best = (l.z.abs(), frame.point_to_world(Point3::new(l.x, l.y, 0.0)));
        if let Some(q) = near.nearest(p) {
            let d = (q - p).length();
            if d < best.0 {
                best = (d, q);
            }
        }
        if best.0 <= t {
            sheet.nodes[k] = best.1;
        }
    }
    fitted(frame, trim, &sheet, t)
}

/// The sheet as the fit reads it: its nodes (world), triangles, each node's wall over t, and
/// whether it touched the tool.
struct Sheet {
    nodes: Vec<Point3>,
    tris: Vec<[u32; 3]>,
    ratio: Vec<f64>,
    touched: Vec<bool>,
}

impl Sheet {
    /// Each triangle in four, by its sides' middles: a middle touched where both its ends did.
    fn split(self) -> Sheet {
        let mut s = self;
        let mut mid: HashMap<(u32, u32), u32, Fast> = HashMap::default();
        let mut tris = Vec::with_capacity(s.tris.len() * 4);
        for &[a, b, c] in &s.tris.clone() {
            let mut m = |x: u32, y: u32| -> u32 {
                *mid.entry((x.min(y), x.max(y))).or_insert_with(|| {
                    let (i, j) = (x as usize, y as usize);
                    s.nodes.push(s.nodes[i] + (s.nodes[j] - s.nodes[i]) * 0.5);
                    s.ratio.push(0.5 * (s.ratio[i] + s.ratio[j]));
                    s.touched.push(s.touched[i] && s.touched[j]);
                    (s.nodes.len() - 1) as u32
                })
            };
            let (ab, bc, ca) = (m(a, b), m(b, c), m(c, a));
            tris.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
        }
        s.tris = tris;
        s
    }
}

/// The tool's triangles in cells, for the nearest point on it.
pub(crate) struct Near<'a> {
    tool: &'a OnTool<'a>,
    cell: f64,
    cells: HashMap<(i64, i64, i64), Vec<u32>, Fast>,
    /// Each tool triangle's box, by its index in the mesh.
    bounds: Vec<Bounds>,
}

impl<'a> Near<'a> {
    pub(crate) fn new(tool: &'a OnTool<'a>, reach: f64) -> Near<'a> {
        let cell = reach.max(0.5);
        let key = |v: f64| (v / cell).floor() as i64;
        let mut cells: HashMap<(i64, i64, i64), Vec<u32>, Fast> = HashMap::default();
        let mut boxes = vec![([0.0; 3], [0.0; 3]); tool.mesh.triangles.len()];
        for &k in tool.contact {
            let [a, b, c] = tool.mesh.triangles[k].map(|i| tool.mesh.positions[i as usize]);
            boxes[k] = bounds(a, b, c);
            let lo = [
                a.x.min(b.x).min(c.x),
                a.y.min(b.y).min(c.y),
                a.z.min(b.z).min(c.z),
            ];
            let hi = [
                a.x.max(b.x).max(c.x),
                a.y.max(b.y).max(c.y),
                a.z.max(b.z).max(c.z),
            ];
            for i in key(lo[0] - reach)..=key(hi[0] + reach) {
                for j in key(lo[1] - reach)..=key(hi[1] + reach) {
                    for l in key(lo[2] - reach)..=key(hi[2] + reach) {
                        cells.entry((i, j, l)).or_default().push(k as u32);
                    }
                }
            }
        }
        Near {
            tool,
            cell,
            cells,
            bounds: boxes,
        }
    }

    pub(crate) fn nearest(&self, p: Point3) -> Option<Point3> {
        let key = |v: f64| (v / self.cell).floor() as i64;
        let list = self.cells.get(&(key(p.x), key(p.y), key(p.z)))?;
        least(
            list,
            |k| below(p, &self.bounds[k as usize]),
            |k| {
                let [a, b, c] =
                    self.tool.mesh.triangles[k as usize].map(|i| self.tool.mesh.positions[i as usize]);
                let q = closest(p, a, b, c);
                ((q - p).length_sq(), q)
            },
        )
    }
}

/// Where the formed part lies on the tool: its surface (within `on` of it) or the trim plane
/// (flat on it), as the simulated part tells its side on the tool from the rest. Owned, for the
/// view's workers.
#[derive(Clone, Debug)]
pub struct ToolSide {
    frame: Frame,
    on: f64,
    tris: Vec<[Point3; 3]>,
    cell: f64,
    cells: HashMap<(i64, i64, i64), Vec<u32>, Fast>,
}

impl ToolSide {
    pub fn new(frame: &Frame, tool: &OnTool, on: f64) -> ToolSide {
        let tris: Vec<[Point3; 3]> = tool
            .contact
            .iter()
            .map(|&k| tool.mesh.triangles[k].map(|i| tool.mesh.positions[i as usize]))
            .collect();
        let cell = (4.0 * on).max(0.5);
        let key = |v: f64| (v / cell).floor() as i64;
        let mut cells: HashMap<(i64, i64, i64), Vec<u32>, Fast> = HashMap::default();
        for (k, [a, b, c]) in tris.iter().enumerate() {
            let lo = [
                a.x.min(b.x).min(c.x),
                a.y.min(b.y).min(c.y),
                a.z.min(b.z).min(c.z),
            ];
            let hi = [
                a.x.max(b.x).max(c.x),
                a.y.max(b.y).max(c.y),
                a.z.max(b.z).max(c.z),
            ];
            for i in key(lo[0] - on)..=key(hi[0] + on) {
                for j in key(lo[1] - on)..=key(hi[1] + on) {
                    for l in key(lo[2] - on)..=key(hi[2] + on) {
                        cells.entry((i, j, l)).or_default().push(k as u32);
                    }
                }
            }
        }
        ToolSide {
            frame: *frame,
            on,
            tris,
            cell,
            cells,
        }
    }

    /// Whether a triangle of the part lies on the tool's side: its middle on the tool, or flat on
    /// the trim plane.
    pub fn has(&self, t: [Point3; 3]) -> bool {
        let [a, b, c] = t;
        let g = Point3::ORIGIN
            + ((a - Point3::ORIGIN) + (b - Point3::ORIGIN) + (c - Point3::ORIGIN)) * (1.0 / 3.0);
        let flat = (b - a)
            .cross(c - a)
            .normalize()
            .is_some_and(|n| n.get().dot(self.frame.z().get()).abs() > 0.999);
        if flat && self.frame.point_to_local(g).z.abs() < self.on {
            return true;
        }
        let key = |v: f64| (v / self.cell).floor() as i64;
        self.cells.get(&(key(g.x), key(g.y), key(g.z))).is_some_and(|l| {
            l.iter().any(|&k| {
                let [a, b, c] = self.tris[k as usize];
                (closest(g, a, b, c) - g).length() < self.on
            })
        })
    }
}

/// `f` of each of `0..n` in order, on all cores a few thousand at a time (`par::map` takes them
/// one at a time, which costs more than these small items).
fn each<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    const CHUNK: usize = 2048;
    forge_math::par::map(n.div_ceil(CHUNK), |c| {
        (c * CHUNK..((c + 1) * CHUNK).min(n)).map(&f).collect::<Vec<T>>()
    })
    .into_iter()
    .flatten()
    .collect()
}

/// A triangle's box, for skipping triangles no nearer than one already found.
type Bounds = ([f64; 3], [f64; 3]);

fn bounds(a: Point3, b: Point3, c: Point3) -> Bounds {
    (
        [
            a.x.min(b.x).min(c.x),
            a.y.min(b.y).min(c.y),
            a.z.min(b.z).min(c.z),
        ],
        [
            a.x.max(b.x).max(c.x),
            a.y.max(b.y).max(c.y),
            a.z.max(b.z).max(c.z),
        ],
    )
}

/// The squared distance from `p` to a box: no point of the box is nearer.
fn below(p: Point3, (lo, hi): &Bounds) -> f64 {
    let d = |v: f64, l: f64, h: f64| {
        if v < l {
            l - v
        } else if v > h {
            v - h
        } else {
            0.0
        }
    };
    let (x, y, z) = (d(p.x, lo[0], hi[0]), d(p.y, lo[1], hi[1]), d(p.z, lo[2], hi[2]));
    x * x + y * y + z * z
}

/// Of the triangles `list` (by their place in it), the one whose `dist` from `p` is least, the
/// first of equals: the same one as trying each in order, without trying those whose box is
/// already farther than the best so far (a margin over rounding keeps that exact).
fn least<R>(list: &[u32], bound: impl Fn(u32) -> f64, dist: impl Fn(u32) -> (f64, R)) -> Option<R> {
    let first = (0..list.len()).min_by(|&a, &b| bound(list[a]).total_cmp(&bound(list[b])))?;
    let (mut d, mut r) = dist(list[first]);
    let mut at = first;
    for (i, &k) in list.iter().enumerate() {
        if i == first || bound(k) > d * (1.0 + 1e-9) {
            continue;
        }
        let (e, s) = dist(k);
        if e.total_cmp(&d).then(i.cmp(&at)).is_lt() {
            (d, r, at) = (e, s, i);
        }
    }
    Some(r)
}

/// The point of triangle abc nearest p (Ericson, Real-Time Collision Detection 5.1.5).
fn closest(p: Point3, a: Point3, b: Point3, c: Point3) -> Point3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let den = 1.0 / (va + vb + vc);
    a + ab * (vb * den) + ac * (vc * den)
}

fn fitted(frame: &Frame, trim: &[Point2], st: &Sheet, t: f64) -> Simulated {
    let poly = clean(trim);
    let convex = is_convex(&poly);
    let local: Vec<Point3> = st.nodes.iter().map(|p| frame.point_to_local(*p)).collect();
    let xy = |k: usize| Point2::new(local[k].x, local[k].y);
    // each node's normal: its triangles' by area, the side the sheet came from (off the tool)
    let mut normal = vec![Vec3::ZERO; st.nodes.len()];
    for v in &st.tris {
        let [a, b, c] = v.map(|i| st.nodes[i as usize]);
        let n = (b - a).cross(c - a);
        for &i in v {
            normal[i as usize] += n;
        }
    }
    let normal: Vec<Vec3> = normal
        .into_iter()
        .map(|n| n.normalize().map_or(frame.z().get(), |u| u.get()))
        .collect();
    // mitred: at a corner the node moves along the mean normal far enough that each side's
    // own wall is the model's, not less
    let mut lean = vec![(0.0, 0.0); st.nodes.len()];
    for v in &st.tris {
        let [a, b, c] = v.map(|i| st.nodes[i as usize]);
        let n = (b - a).cross(c - a);
        let (s, Some(u)) = (n.length(), n.normalize()) else {
            continue;
        };
        for &i in v {
            let l = &mut lean[i as usize];
            l.0 += s * normal[i as usize].dot(u.get());
            l.1 += s;
        }
    }
    let normal: Vec<Vec3> = normal
        .iter()
        .zip(&lean)
        .map(|(n, l)| *n * (1.0 / if l.1 > 0.0 { (l.0 / l.1).max(0.5) } else { 1.0 }))
        .collect();
    let wall: Vec<f64> = st.ratio.iter().map(|r| r * t).collect();

    let mut out = Builder::default();
    for v in &st.tris {
        let ids = v.map(|i| i as usize);
        let corners: Vec<Corner> = ids.iter().map(|&k| Corner::Node(k, xy(k))).collect();
        let clipped = if convex {
            clip(corners, &poly)
        } else {
            let c = ids.iter().fold(Point2::new(0.0, 0.0), |s, &k| {
                Point2::new(s.x + xy(k).x / 3.0, s.y + xy(k).y / 3.0)
            });
            if crate::build::inside(&poly, c).0 {
                corners
            } else {
                vec![]
            }
        };
        if clipped.len() < 3 {
            continue;
        }
        let at = |c: &Corner| -> usize {
            match *c {
                Corner::Node(k, _) => out.node(k, st.nodes[k], normal[k], wall[k]),
                Corner::Cut(p) => {
                    // on the triangle, by its corners' weights in the plane
                    let w = weights(p, ids.map(xy));
                    let mix3 =
                        |f: &dyn Fn(usize) -> Vec3| f(ids[0]) * w[0] + f(ids[1]) * w[1] + f(ids[2]) * w[2];
                    let pos = Point3::ORIGIN + mix3(&|k| st.nodes[k] - Point3::ORIGIN);
                    let n = mix3(&|k| normal[k]);
                    let h = wall[ids[0]] * w[0] + wall[ids[1]] * w[1] + wall[ids[2]] * w[2];
                    out.cut(p, pos, n, h)
                }
            }
        };
        let idx: Vec<usize> = clipped.iter().map(at).collect();
        for k in 1..idx.len() - 1 {
            if idx[0] != idx[k] && idx[k] != idx[k + 1] && idx[0] != idx[k + 1] {
                out.tris.push([idx[0], idx[k], idx[k + 1]]);
            }
        }
    }
    out.finish()
}

/// A corner of a clipped triangle: one of the sheet's nodes, or where the trim cuts it.
#[derive(Clone, Copy, Debug)]
enum Corner {
    Node(usize, Point2),
    Cut(Point2),
}

impl Corner {
    fn xy(&self) -> Point2 {
        match *self {
            Corner::Node(_, p) | Corner::Cut(p) => p,
        }
    }
}

/// The polygon without repeated points, counter-clockwise.
fn clean(trim: &[Point2]) -> Vec<Point2> {
    let mut p: Vec<Point2> = vec![];
    for &q in trim {
        if p.last().is_none_or(|l| (*l - q).length() > 1e-9) {
            p.push(q);
        }
    }
    while p.len() > 1 && (p[0] - p[p.len() - 1]).length() <= 1e-9 {
        p.pop();
    }
    let area: f64 = (0..p.len())
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    if area < 0.0 {
        p.reverse();
    }
    p
}

fn cross(o: Point2, a: Point2, b: Point2) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn is_convex(p: &[Point2]) -> bool {
    p.len() >= 3 && (0..p.len()).all(|i| cross(p[i], p[(i + 1) % p.len()], p[(i + 2) % p.len()]) >= -1e-9)
}

/// Sutherland-Hodgman: the triangle's part inside the convex polygon.
fn clip(mut poly: Vec<Corner>, clipper: &[Point2]) -> Vec<Corner> {
    for i in 0..clipper.len() {
        if poly.is_empty() {
            break;
        }
        let (a, b) = (clipper[i], clipper[(i + 1) % clipper.len()]);
        let side = |p: Point2| cross(a, b, p);
        let input = std::mem::take(&mut poly);
        for k in 0..input.len() {
            let (p, q) = (input[k], input[(k + 1) % input.len()]);
            let (sp, sq) = (side(p.xy()), side(q.xy()));
            if sp >= 0.0 {
                poly.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let f = sp / (sp - sq);
                let (u, v) = (p.xy(), q.xy());
                poly.push(Corner::Cut(Point2::new(
                    u.x + (v.x - u.x) * f,
                    u.y + (v.y - u.y) * f,
                )));
            }
        }
    }
    poly
}

/// A point's weights on a triangle in the plane.
fn weights(p: Point2, [a, b, c]: [Point2; 3]) -> [f64; 3] {
    let d = cross(a, b, c);
    if d.abs() < 1e-18 {
        return [1.0 / 3.0; 3];
    }
    let wa = cross(p, b, c) / d;
    let wb = cross(a, p, c) / d;
    [wa, wb, 1.0 - wa - wb]
}

#[derive(Default)]
struct Builder {
    pos: Vec<Point3>,
    normal: Vec<Vec3>,
    wall: Vec<f64>,
    nodes: HashMap<usize, usize>,
    cuts: HashMap<(i64, i64), usize>,
    tris: Vec<[usize; 3]>,
}

impl Builder {
    fn push(&mut self, p: Point3, n: Vec3, h: f64) -> usize {
        self.pos.push(p);
        self.normal.push(n);
        self.wall.push(h);
        self.pos.len() - 1
    }

    fn node(&mut self, k: usize, p: Point3, n: Vec3, h: f64) -> usize {
        if let Some(&i) = self.nodes.get(&k) {
            return i;
        }
        let i = self.push(p, n, h);
        self.nodes.insert(k, i);
        i
    }

    // two triangles cut on their shared side meet at the same point, to far below a micron
    fn cut(&mut self, at: Point2, p: Point3, n: Vec3, h: f64) -> usize {
        let key = ((at.x * 1e5).round() as i64, (at.y * 1e5).round() as i64);
        if let Some(&i) = self.cuts.get(&key) {
            return i;
        }
        let i = self.push(p, n, h);
        self.cuts.insert(key, i);
        i
    }

    fn finish(self) -> Simulated {
        let n = self.pos.len();
        let mut mesh = TriMesh::default();
        mesh.positions.extend(self.pos.iter().copied());
        mesh.positions
            .extend((0..n).map(|i| self.pos[i] + self.normal[i] * self.wall[i]));
        let u = |i: usize| i as u32;
        let (mut area, mut sheet_volume) = (0.0, 0.0);
        let mut directed: HashMap<(usize, usize), usize> = HashMap::new();
        let mid = |i: usize| self.pos[i] + self.normal[i] * (0.5 * self.wall[i]);
        for &[a, b, c] in &self.tris {
            area += 0.5
                * (self.pos[b] - self.pos[a])
                    .cross(self.pos[c] - self.pos[a])
                    .length();
            // over the wall's middle, where the sheet's area is what the membrane conserves
            let s = 0.5 * (mid(b) - mid(a)).cross(mid(c) - mid(a)).length();
            sheet_volume += s * (self.wall[a] + self.wall[b] + self.wall[c]) / 3.0;
            mesh.triangles.push([u(a), u(c), u(b)]);
            mesh.face_of.push(TOOL_SIDE);
            mesh.triangles.push([u(a + n), u(b + n), u(c + n)]);
            mesh.face_of.push(FREE_SIDE);
            for (p, q) in [(a, b), (b, c), (c, a)] {
                *directed.entry((p, q)).or_default() += 1;
            }
        }
        // the rim: round every side only one triangle has
        let mut rim: Vec<(usize, usize)> = directed
            .keys()
            .filter(|(p, q)| !directed.contains_key(&(*q, *p)))
            .copied()
            .collect();
        rim.sort_unstable();
        for (p, q) in rim {
            mesh.triangles.push([u(p), u(q), u(q + n)]);
            mesh.triangles.push([u(p), u(q + n), u(p + n)]);
            mesh.face_of.extend([RIM, RIM]);
        }
        let mass = mesh_mass(&mesh);
        Simulated {
            foot: self.pos.iter().chain(&self.pos).copied().collect(),
            wall: self.wall.iter().chain(&self.wall).copied().collect(),
            raw: self.wall.iter().chain(&self.wall).copied().collect(),
            sheet: self.pos.iter().chain(&self.pos).copied().collect(),
            modelled: self.pos.iter().chain(&self.pos).copied().collect(),
            mesh,
            area,
            sheet_volume,
            mass,
        }
    }
}

/// A closed mesh's mass properties: a tetrahedron from a point inside to each triangle.
pub fn mesh_mass(m: &TriMesh) -> MeshMass {
    let bb = m.bbox();
    let o = bb.center();
    let (mut vol, mut first, mut second) = (0.0, Vec3::ZERO, [[0.0f64; 3]; 3]);
    for t in &m.triangles {
        let [a, b, c] = t.map(|i| m.positions[i as usize] - o);
        let v = a.dot(b.cross(c)) / 6.0;
        vol += v;
        let s = a + b + c;
        first += s * (v / 4.0);
        for (i, row) in second.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x += v / 20.0 * (a[i] * a[j] + b[i] * b[j] + c[i] * c[j] + s[i] * s[j]);
            }
        }
    }
    if vol.abs() < 1e-300 {
        return MeshMass {
            volume: 0.0,
            centroid: o,
            inertia: [[0.0; 3]; 3],
        };
    }
    let g = first * (1.0 / vol);
    let s = |i: usize, j: usize| second[i][j] - vol * g[i] * g[j];
    let tr = s(0, 0) + s(1, 1) + s(2, 2);
    MeshMass {
        volume: vol,
        centroid: o + g,
        inertia: std::array::from_fn(|i| {
            std::array::from_fn(|j| if i == j { tr - s(i, i) } else { -s(i, j) })
        }),
    }
}

/// The blank before forming: the sheet cut to `size` (on the trim plane's axes, round the trim
/// box's middle), `t` thick, lying at `z` along the pull.
pub fn blank(frame: &Frame, middle: Point2, size: (f64, f64), z: f64, t: f64) -> TriMesh {
    let (w, l) = (size.0 * 0.5, size.1 * 0.5);
    let mut m = TriMesh::default();
    for (dz, (dx, dy)) in [0.0, t]
        .iter()
        .flat_map(|&dz| [(-w, -l), (w, -l), (w, l), (-w, l)].map(|c| (dz, c)))
    {
        m.positions
            .push(frame.point_to_world(Point3::new(middle.x + dx, middle.y + dy, z + dz)));
    }
    let quads = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ];
    for (f, q) in quads.iter().enumerate() {
        m.triangles.push([q[0], q[1], q[2]]);
        m.triangles.push([q[0], q[2], q[3]]);
        m.face_of.extend([f as u32, f as u32]);
    }
    m
}

/// The model's wall over the sheet as it lies on the tool: for any point near it, the wall at
/// its foot (the nearest point of the sheet), by the foot's triangle's corners.
#[derive(Clone, Debug, Default)]
pub struct WallField {
    /// The sheet's wall integrated over its middle surface: the material formed.
    pub volume: f64,
    pos: Vec<Point3>,
    tris: Vec<[u32; 3]>,
    /// The wall smoothed (what the simulated part's shape is made from) and as the model gives it
    /// (what its colours show).
    wall: Vec<f64>,
    raw: Vec<f64>,
    cell: f64,
    cells: HashMap<(i64, i64, i64), Vec<u32>, Fast>,
    bounds: Vec<Bounds>,
}

impl WallField {
    /// The tool side of a simulated sheet, looked up within `reach` of it.
    pub fn of(sim: &Simulated, reach: f64) -> WallField {
        let n = sim.mesh.positions.len();
        let tris: Vec<[u32; 3]> = sim
            .mesh
            .triangles
            .iter()
            .zip(&sim.mesh.face_of)
            .filter(|x| *x.1 == TOOL_SIDE)
            .map(|x| *x.0)
            .collect();
        let cell = reach.max(0.5);
        let key = |v: f64| (v / cell).floor() as i64;
        let mut cells: HashMap<(i64, i64, i64), Vec<u32>, Fast> = HashMap::default();
        let mut boxes = Vec::with_capacity(tris.len());
        for (k, t) in tris.iter().enumerate() {
            let [a, b, c] = t.map(|i| sim.mesh.positions[i as usize]);
            boxes.push(bounds(a, b, c));
            let lo = [
                a.x.min(b.x).min(c.x),
                a.y.min(b.y).min(c.y),
                a.z.min(b.z).min(c.z),
            ];
            let hi = [
                a.x.max(b.x).max(c.x),
                a.y.max(b.y).max(c.y),
                a.z.max(b.z).max(c.z),
            ];
            for i in key(lo[0] - reach)..=key(hi[0] + reach) {
                for j in key(lo[1] - reach)..=key(hi[1] + reach) {
                    for l in key(lo[2] - reach)..=key(hi[2] + reach) {
                        cells.entry((i, j, l)).or_default().push(k as u32);
                    }
                }
            }
        }
        // the model's wall from node to node is noisy where the sheet turns a corner: smoothed
        // over the sheet, its integral kept
        let pos = &sim.mesh.positions[..n];
        let mut wall = sim.wall.clone();
        let integral = |w: &[f64]| -> f64 {
            tris.iter()
                .map(|t| {
                    let [a, b, c] = t.map(|i| pos[i as usize]);
                    0.5 * (b - a).cross(c - a).length() * t.iter().map(|&i| w[i as usize]).sum::<f64>()
                })
                .sum()
        };
        let before = integral(&wall);
        let mut nb: Vec<Vec<u32>> = vec![vec![]; n];
        for t in &tris {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                nb[a as usize].push(b);
                nb[b as usize].push(a);
            }
        }
        for _ in 0..SMOOTH {
            let w0 = wall.clone();
            for (k, list) in nb.iter().enumerate() {
                if !list.is_empty() {
                    wall[k] = 0.5 * w0[k]
                        + 0.5 * list.iter().map(|&j| w0[j as usize]).sum::<f64>() / list.len() as f64;
                }
            }
        }
        let after = integral(&wall);
        if after > 0.0 {
            wall.iter_mut().for_each(|w| *w *= before / after);
        }
        WallField {
            volume: sim.sheet_volume,
            pos: pos.to_vec(),
            tris,
            wall,
            raw: sim.wall.clone(),
            cell,
            cells,
            bounds: boxes,
        }
    }

    /// The wall at `p`'s foot on the sheet, smoothed, and the foot (None: no sheet within reach).
    pub fn at(&self, p: Point3) -> Option<(f64, Point3)> {
        self.near(p, &self.wall)
    }

    /// The wall at `p`'s foot as the model gives it, unsmoothed: what the colours show.
    pub fn raw_at(&self, p: Point3) -> Option<(f64, Point3)> {
        self.near(p, &self.raw)
    }

    fn near(&self, p: Point3, wall: &[f64]) -> Option<(f64, Point3)> {
        let key = |v: f64| (v / self.cell).floor() as i64;
        let list = self.cells.get(&(key(p.x), key(p.y), key(p.z)))?;
        let (k, q) = least(
            list,
            |k| below(p, &self.bounds[k as usize]),
            |k| {
                let [a, b, c] = self.tris[k as usize].map(|i| self.pos[i as usize]);
                let q = closest(p, a, b, c);
                ((q - p).length_sq(), (k, q))
            },
        )?;
        let [a, b, c] = self.tris[k as usize].map(|i| self.pos[i as usize]);
        let w = bary(q, a, b, c);
        let [i, j, l] = self.tris[k as usize].map(|i| i as usize);
        Some((w[0] * wall[i] + w[1] * wall[j] + w[2] * wall[l], q))
    }
}

/// Passes of the smoothing of the model's wall over its sheet.
const SMOOTH: usize = 8;

/// A point's weights on triangle abc (it lies on it).
fn bary(q: Point3, a: Point3, b: Point3, c: Point3) -> [f64; 3] {
    let (v0, v1, v2) = (b - a, c - a, q - a);
    let (d00, d01, d11) = (v0.dot(v0), v0.dot(v1), v1.dot(v1));
    let (d20, d21) = (v2.dot(v0), v2.dot(v1));
    let den = d00 * d11 - d01 * d01;
    if den.abs() < 1e-300 {
        return [1.0 / 3.0; 3];
    }
    let v = (d11 * d20 - d01 * d21) / den;
    let w = (d00 * d21 - d01 * d20) / den;
    [1.0 - v - w, v, w]
}

/// The simulated part made from the modelled part itself: its mesh (fine on its rounds, sides
/// longer than `h` split) with every point off the tool moved toward its foot on the part's side
/// on the tool (on the tool's surface, or the flange on the trim plane) till it stands the model's
/// wall off it instead of t (`field`, smoothed); the wall then scaled so the solid holds the
/// model's material (the wall integrated over the sheet's middle surface). At t everywhere it is
/// the modelled part. None where the part has no side on the tool.
pub fn simulated_part(
    part: &forge_topo::Body,
    frame: &Frame,
    tool: &OnTool,
    field: &WallField,
    t: f64,
    h: f64,
) -> Option<Simulated> {
    built_on(part, frame, tool, Some(field), t, h, ANGLE)
}

/// What the simulated part reads of the modelled part: its tessellation (welded, sides split to
/// `h`) and, at each triangle's corners, the exact surface's outward normal. Its bytes key the
/// simulated part on disk: they are all of the part it depends on.
#[derive(Clone, Debug, PartialEq)]
pub struct PartMesh {
    pub pos: Vec<Point3>,
    pub tris: Vec<[u32; 3]>,
    pub faces: Vec<u32>,
    pub normals: Vec<[Vec3; 3]>,
}

/// `PartMesh` of `part`, None where it does not tessellate.
pub fn part_mesh(part: &forge_topo::Body, h: f64) -> Option<PartMesh> {
    tessellated(part, h, ANGLE)
}

/// `simulated_part` from the part's `PartMesh`.
pub fn simulated_from(
    m: &PartMesh,
    frame: &Frame,
    tool: &OnTool,
    field: &WallField,
    t: f64,
) -> Option<Simulated> {
    built_from(m, frame, tool, Some(field), t)
}

/// The tessellation's largest turn per side on the part's rounds.
pub(crate) const ANGLE: f64 = std::f64::consts::PI / 18.0;

/// `simulated_part`, or with no field the wall t everywhere: the modelled part again, as a mesh.
pub(crate) fn built_on(
    part: &forge_topo::Body,
    frame: &Frame,
    tool: &OnTool,
    field: Option<&WallField>,
    t: f64,
    h: f64,
    angle: f64,
) -> Option<Simulated> {
    built_from(&tessellated(part, h, angle)?, frame, tool, field, t)
}

fn tessellated(part: &forge_topo::Body, h: f64, angle: f64) -> Option<PartMesh> {
    let q = forge_mesh::Quality {
        chord: 0.02,
        max_angle: angle,
    };
    let m = forge_mesh::tessellate(part, &q).ok()?;
    // one point where faces meet, and points closer than a twentieth of a millimetre one: the
    // slivers the tessellation leaves along a face's edge would turn over when moved
    let mut pos: Vec<Point3> = vec![];
    let mut cells: HashMap<(i64, i64, i64), Vec<u32>, Fast> = HashMap::default();
    let key = |p: Point3| {
        (
            (p.x / WELD).floor() as i64,
            (p.y / WELD).floor() as i64,
            (p.z / WELD).floor() as i64,
        )
    };
    let mut weld = |p: Point3| -> u32 {
        let (x, y, z) = key(p);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    for &i in cells.get(&(x + dx, y + dy, z + dz)).into_iter().flatten() {
                        if (pos[i as usize] - p).length() < WELD {
                            return i;
                        }
                    }
                }
            }
        }
        pos.push(p);
        let i = (pos.len() - 1) as u32;
        cells.entry((x, y, z)).or_default().push(i);
        i
    };
    let (mut tris, mut faces): (Vec<[u32; 3]>, Vec<u32>) = m
        .triangles
        .iter()
        .zip(&m.face_of)
        .map(|(tri, &f)| (tri.map(|i| weld(m.positions[i as usize])), f))
        .filter(|(t, _)| t[0] != t[1] && t[1] != t[2] && t[0] != t[2])
        .unzip();
    refine(&mut pos, &mut tris, &mut faces, h);
    // the exact surface's normal at each triangle's corners, on all cores in order
    let normals = each(tris.len(), |k| {
        let [a, b, c] = tris[k].map(|i| pos[i as usize]);
        let near = (b - a).cross(c - a);
        tris[k].map(|i| face_normal(part, faces[k], pos[i as usize], near))
    });
    Some(PartMesh {
        pos,
        tris,
        faces,
        normals,
    })
}

fn built_from(
    pm: &PartMesh,
    frame: &Frame,
    tool: &OnTool,
    field: Option<&WallField>,
    t: f64,
) -> Option<Simulated> {
    let (pos, tris, faces) = (&pm.pos, &pm.tris, &pm.faces);
    // the part's triangles on the tool's surface, or on the trim plane facing it (the flange's)
    let near = Near::new(tool, 2.0 * t);
    let on = 0.05 * t;
    let is_side: Vec<bool> = each(tris.len(), |k| {
        let [a, b, c] = tris[k].map(|i| pos[i as usize]);
        let g = Point3::ORIGIN
            + ((a - Point3::ORIGIN) + (b - Point3::ORIGIN) + (c - Point3::ORIGIN)) * (1.0 / 3.0);
        let Some(n) = (b - a).cross(c - a).normalize() else {
            return false;
        };
        let flat = frame.point_to_local(g).z.abs() < on && n.get().dot(frame.z().get()).abs() > 0.999;
        flat || near.nearest(g).is_some_and(|x| (x - g).length() < on)
    });
    if !is_side.iter().any(|x| *x) {
        return None;
    }
    let mut on_side = vec![false; pos.len()];
    for (tri, s) in tris.iter().zip(&is_side) {
        if *s {
            tri.iter().for_each(|&i| on_side[i as usize] = true);
        }
    }
    // every other point's normal over the faces off the tool, its foot t in along it (the
    // modelled part is the side on the tool moved off it by t), and the wall there
    // the exact surface's normal at each point, of each face off the tool it is on
    let mut normal = vec![Vec3::ZERO; pos.len()];
    // each point's faces counted once (a point has few)
    let mut done: Vec<Vec<u32>> = vec![vec![]; pos.len()];
    for (k, (tri, s)) in tris.iter().zip(&is_side).enumerate() {
        if *s {
            continue;
        }
        for (c, &i) in tri.iter().enumerate() {
            let seen = &mut done[i as usize];
            if !seen.contains(&faces[k]) {
                seen.push(faces[k]);
                normal[i as usize] += pm.normals[k][c];
            }
        }
    }
    let normal: Vec<Vec3> = normal
        .into_iter()
        .map(|v| v.normalize().map_or(Vec3::ZERO, |u| u.get()))
        .collect();
    let foot: Vec<Point3> = (0..pos.len())
        .map(|i| {
            if on_side[i] {
                pos[i]
            } else {
                pos[i] - normal[i] * t
            }
        })
        .collect();
    let wall: Vec<f64> = each(foot.len(), |i| {
        field.and_then(|f| f.at(foot[i])).map_or(t, |x| x.0)
    });
    // the modelled part is the side on the tool moved off it by t: moved by the wall instead
    let mut mesh = TriMesh {
        positions: pos.clone(),
        triangles: tris.clone(),
        face_of: tris
            .iter()
            .zip(&is_side)
            .map(|(tri, s)| match (s, tri.iter().any(|&i| on_side[i as usize])) {
                (true, _) => TOOL_SIDE,
                (false, true) => RIM,
                _ => FREE_SIDE,
            })
            .collect(),
        ..TriMesh::default()
    };
    // each point's move in from t to the wall, smoothed over its neighbours off the tool: where
    // the tessellation's normals turn quickly (a round's ends) the moves stay in step
    let mut nb: Vec<Vec<u32>> = vec![vec![]; pos.len()];
    for tri in tris {
        for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            if !on_side[a as usize] && !on_side[b as usize] {
                nb[a as usize].push(b);
                nb[b as usize].push(a);
            }
        }
    }
    let smooth = |d: &mut Vec<Vec3>| {
        for _ in 0..MOVES {
            let d0 = std::mem::take(d);
            *d = each(d0.len(), |i| {
                let list = &nb[i];
                if !on_side[i] && !list.is_empty() {
                    let mean =
                        list.iter().fold(Vec3::ZERO, |s, &j| s + d0[j as usize]) * (1.0 / list.len() as f64);
                    d0[i] * 0.5 + mean * 0.5
                } else {
                    d0[i]
                }
            });
        }
    };
    let place = |mesh: &mut TriMesh, k: f64| {
        let mut d: Vec<Vec3> = (0..pos.len())
            .map(|i| {
                if on_side[i] {
                    Vec3::ZERO
                } else {
                    -normal[i] * (t - k * wall[i])
                }
            })
            .collect();
        smooth(&mut d);
        for i in 0..pos.len() {
            mesh.positions[i] = pos[i] + d[i];
        }
    };
    place(&mut mesh, 1.0);
    let (mut area, mut sheet_volume) = (0.0, 0.0);
    let side = tris.iter().zip(&is_side).filter(|x| *x.1).map(|x| x.0);
    for tri in side {
        let [a, b, c] = tri.map(|i| pos[i as usize]);
        let s = 0.5 * (b - a).cross(c - a).length();
        area += s;
        sheet_volume += s * tri.iter().map(|&i| wall[i as usize]).sum::<f64>() / 3.0;
    }
    // the material the model formed, kept: the part's own side has sharp corners and rounds the
    // model's sheet bridges, so the wall is scaled (by about 1 %) till the solid holds the
    // model's volume, its wall integrated over its middle surface
    let mut mass = mesh_mass(&mesh);
    let mut k = 1.0;
    if let Some(f) = field.filter(|f| f.volume > 0.0) {
        for _ in 0..3 {
            if mass.volume <= 0.0 {
                break;
            }
            k *= f.volume / mass.volume;
            place(&mut mesh, k);
            mass = mesh_mass(&mesh);
        }
        sheet_volume = f.volume;
    }
    // the colours: the model's own wall at each point of the modelled part, as painted there
    let (raw, sheet): (Vec<f64>, Vec<Point3>) = each(pos.len(), |i| {
        field.and_then(|f| f.raw_at(pos[i])).unwrap_or((wall[i], pos[i]))
    })
    .into_iter()
    .unzip();
    Some(Simulated {
        wall: wall.iter().map(|w| w * k).collect(),
        raw,
        sheet,
        modelled: pos.clone(),
        foot,
        mesh,
        area,
        sheet_volume,
        mass,
    })
}

/// Points closer than this are one, mm.
const WELD: f64 = 0.05;

/// Passes of the smoothing of the moves from the modelled part's outside.
const MOVES: usize = 8;

/// Splits sides longer than `h`, keeping the mesh conforming: a side split is split in both its
/// triangles.
fn refine(pos: &mut Vec<Point3>, tris: &mut Vec<[u32; 3]>, faces: &mut Vec<u32>, h: f64) {
    for _ in 0..12 {
        let len = |a: u32, b: u32, pos: &[Point3]| (pos[a as usize] - pos[b as usize]).length();
        let key = |a: u32, b: u32| (a.min(b), a.max(b));
        let mut split: HashMap<(u32, u32), u32, Fast> = HashMap::default();
        for &[a, b, c] in tris.iter() {
            let sides = [(a, b), (b, c), (c, a)];
            let (x, y) = sides
                .into_iter()
                .max_by(|p, q| len(p.0, p.1, pos).total_cmp(&len(q.0, q.1, pos)))
                .expect("three sides");
            if len(x, y, pos) > h && !split.contains_key(&key(x, y)) {
                pos.push(pos[x as usize] + (pos[y as usize] - pos[x as usize]) * 0.5);
                split.insert(key(x, y), (pos.len() - 1) as u32);
            }
        }
        if split.is_empty() {
            return;
        }
        let mut out = Vec::with_capacity(tris.len() * 2);
        let mut of = Vec::with_capacity(tris.len() * 2);
        for (&[a, b, c], &f) in tris.iter().zip(faces.iter()) {
            let was = out.len();
            let m = |x: u32, y: u32| split.get(&key(x, y)).copied();
            match (m(a, b), m(b, c), m(c, a)) {
                (None, None, None) => out.push([a, b, c]),
                (Some(p), None, None) => out.extend([[a, p, c], [p, b, c]]),
                (None, Some(p), None) => out.extend([[a, b, p], [a, p, c]]),
                (None, None, Some(p)) => out.extend([[a, b, p], [p, b, c]]),
                (Some(p), Some(q), None) => out.extend([[a, p, c], [p, b, q], [p, q, c]]),
                (None, Some(p), Some(q)) => out.extend([[a, b, q], [q, b, p], [q, p, c]]),
                (Some(p), None, Some(q)) => out.extend([[a, p, q], [p, b, c], [q, p, c]]),
                (Some(p), Some(q), Some(r)) => out.extend([[a, p, r], [p, b, q], [r, q, c], [p, q, r]]),
            }
            of.extend(std::iter::repeat_n(f, out.len() - was));
        }
        *tris = out;
        *faces = of;
    }
}

/// A body face's outward normal at a point near it; `near` (its triangle's) where it has none.
fn face_normal(body: &forge_topo::Body, f: u32, p: Point3, near: Vec3) -> Vec3 {
    use forge_geom::SurfaceGeom;
    let face = body.face(forge_topo::FaceId(f));
    let near = near.normalize().map_or(Vec3::ZERO, |u| u.get());
    match face.surface.normal(face.surface.project(p)) {
        Some(n) => {
            let n = if face.same_sense { n.get() } else { -n.get() };
            if n.dot(near) > 0.2 { n } else { near }
        }
        None => near,
    }
}
