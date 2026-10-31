use forge_geom::intersect::{Contact, LineSurface, line_surface};
use forge_geom::{Curve, CurveGeom, Line, Surface, SurfaceGeom};
use forge_math::predicates::triple;
use forge_math::{Box2, Box3, Interval, Point2, Point3, Vec3, tol};
use forge_topo::uv::{UvLoop, face_uv_loops, point_in_loops};
use forge_topo::{Body, FaceId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    Inside,
    Outside,
    /// Within tolerance of the boundary.
    On,
}

/// A face's loops in parameter space, built the first time a query needs them and kept on
/// the face (`Face::query`) across booleans; a loop added in place is appended.
pub struct FaceInfo {
    /// Each boundary use with its edge's box.
    edges: Vec<(forge_topo::CoedgeId, Box3)>,
    /// The largest sag bound among the face's edges: an edge farther than
    /// this can never decide a point, so it is skipped without solving.
    reach: f64,
    /// The loops drawn in parameter space, only when a query needs them
    /// (planar faces are decided exactly without them).
    drawn: std::sync::OnceLock<Drawn>,
    /// For planar faces with many edges: edges bucketed by position across
    /// the first ray direction, so a query looks only at nearby edges.
    bands: Option<Bands>,
    /// A face simple enough to decide a point clear of its boundary in closed form.
    simple: std::sync::OnceLock<Option<Simple>>,
}

/// Faces a point is placed in by a comparison or two: a disk, a cylinder's band between two
/// circles about its axis, or a sphere's wedge between two half great circles. A point near a
/// seam is on the boundary too, as the face's edges have it.
#[derive(Clone, Copy)]
enum Simple {
    Disk {
        centre: Point3,
        r: f64,
    },
    Band {
        origin: Point3,
        axis: Vec3,
        lo: f64,
        hi: f64,
        /// The seam's ruling, from the axis.
        seam: Option<Vec3>,
    },
    /// The sphere on the inner side of both planes (`w` their unit normals into the face), or
    /// of either where the wedge is wider than a half turn (`convex` false).
    Wedge {
        centre: Point3,
        w: [Vec3; 2],
        convex: bool,
    },
}

/// How far from a simple face's boundary a point must be for the closed form to decide it;
/// nearer, the boundary is looked at.
const SIMPLE_MARGIN: f64 = 1e-3;

impl Simple {
    fn of(body: &Body, f: FaceId) -> Option<Simple> {
        let face = body.face(f);
        let [l] = face.loops[..] else { return None };
        let seam = |c: forge_topo::CoedgeId| body.partner(c).is_some_and(|o| body.coedge_face(o) == f);
        let full_circle = |c: forge_topo::CoedgeId| {
            let e = body.edge(body.coedge(c).edge);
            match &e.curve {
                Curve::Circle(ci) if e.start == e.end => Some(*ci),
                _ => None,
            }
        };
        let cs = body.lp(l).coedges();
        match &face.surface {
            Surface::Plane(pl) => {
                let [c] = cs.collect::<Vec<_>>()[..] else {
                    return None;
                };
                let ci = full_circle(c)?;
                let n = pl.frame.z().get();
                (ci.frame.z().get().cross(n).length() < 1e-12
                    && (ci.frame.origin - pl.frame.origin).dot(n).abs() < 1e-9)
                    .then_some(Simple::Disk {
                        centre: ci.frame.origin,
                        r: ci.radius,
                    })
            }
            Surface::Cylinder(cy) => {
                let (origin, axis) = (cy.frame.origin, cy.frame.z().get());
                let mut h: Vec<f64> = Vec::new();
                let mut ruling: Option<Vec3> = None;
                for c in cs {
                    if seam(c)
                        && let Curve::Line(line) = &body.edge(body.coedge(c).edge).curve
                    {
                        let d = line.origin - origin;
                        let off = d - axis * d.dot(axis);
                        if ruling.is_some_and(|r| (r - off).length() >= 1e-9) {
                            return None;
                        }
                        ruling = Some(off);
                        continue;
                    }
                    let ci = full_circle(c)?;
                    let d = ci.frame.origin - origin;
                    let off = d - axis * d.dot(axis);
                    if ci.frame.z().get().cross(axis).length() >= 1e-12
                        || off.length() >= 1e-9
                        || (ci.radius - cy.radius).abs() >= 1e-9
                    {
                        return None;
                    }
                    h.push(d.dot(axis));
                }
                let [a, b] = h[..] else { return None };
                Some(Simple::Band {
                    origin,
                    axis,
                    lo: a.min(b),
                    hi: a.max(b),
                    seam: ruling,
                })
            }
            Surface::Sphere(sp) => {
                let centre = sp.frame.origin;
                let [c0, c1] = cs.collect::<Vec<_>>()[..] else {
                    return None;
                };
                let (e0, e1) = (body.edge(body.coedge(c0).edge), body.edge(body.coedge(c1).edge));
                let distinct = body.coedge(c0).edge != body.coedge(c1).edge;
                if !distinct
                    || (e0.start, e0.end) != (e1.start, e1.end) && (e0.start, e0.end) != (e1.end, e1.start)
                {
                    return None;
                }
                let mut w = [Vec3::ZERO; 2];
                let mut mid = [Point3::ORIGIN; 2];
                for (k, c) in [c0, c1].into_iter().enumerate() {
                    let co = body.coedge(c);
                    let e = body.edge(co.edge);
                    let Curve::Circle(ci) = &e.curve else { return None };
                    if ci.frame.origin.distance(centre) >= 1e-9 || (ci.radius - sp.radius).abs() >= 1e-9 {
                        return None;
                    }
                    let d = e.curve.deriv(e.range.mid());
                    let along = if co.forward { d.d1 } else { -d.d1 };
                    let out = (d.p - centre) * if face.same_sense { 1.0 } else { -1.0 };
                    let n = ci.frame.z().get();
                    w[k] = if n.dot(out.cross(along)) > 0.0 { n } else { -n };
                    mid[k] = d.p;
                }
                Some(Simple::Wedge {
                    centre,
                    w,
                    convex: w[0].dot(mid[1] - centre) > 0.0,
                })
            }
            _ => None,
        }
    }

    /// Where `p`, on the face's surface, lies; None near the boundary.
    fn place(self, p: Point3) -> Option<Where> {
        let m = SIMPLE_MARGIN;
        match self {
            Simple::Disk { centre, r } => {
                let d = p.distance(centre);
                (d < r - m)
                    .then_some(Where::Inside)
                    .or((d > r + m).then_some(Where::Outside))
            }
            Simple::Band {
                origin,
                axis,
                lo,
                hi,
                seam,
            } => {
                let h = (p - origin).dot(axis);
                if let Some(r) = seam
                    && ((p - origin) - axis * h - r).length() <= m
                {
                    return None;
                }
                if h > lo + m && h < hi - m {
                    Some(Where::Inside)
                } else {
                    (h < lo - m || h > hi + m).then_some(Where::Outside)
                }
            }
            Simple::Wedge { centre, w, convex } => {
                let (a, b) = ((p - centre).dot(w[0]), (p - centre).dot(w[1]));
                let (inside, outside) = if convex {
                    (a > m && b > m, a < -m || b < -m)
                } else {
                    (a > m || b > m, a < -m && b < -m)
                };
                inside
                    .then_some(Where::Inside)
                    .or(outside.then_some(Where::Outside))
            }
        }
    }
}

const RAY_ANGLES: [f64; 3] = [0.1234, 1.9876, 4.0123];

struct Bands {
    origin: Point3,
    /// In-plane unit vector across the first ray direction.
    across: Vec3,
    w0: f64,
    /// Bands per unit of `across`.
    inv: f64,
    rows: Vec<Vec<u32>>,
}

/// A boundary use's sag bound and its edge's box (with margin: an edge in a coordinate plane
/// has a box a few ulps thin).
fn edge_entry(body: &Body, c: forge_topo::CoedgeId) -> (f64, Box3) {
    let id = body.coedge(c).edge;
    (
        sag_bound(&body.edge(id).curve),
        body.edge_box(id).grow(tol::LINEAR),
    )
}

/// An index entry whose loop left the face (the entry stays so band rows keep their indices).
const GONE: forge_topo::CoedgeId = forge_topo::CoedgeId(u32::MAX);

/// Face `f` is about to lose loops `gone` in place: their entries in its kept index (if any)
/// are marked gone; once most entries are, the index is dropped to be built afresh.
pub fn drop_query_loops(body: &mut Body, f: FaceId, gone: &[forge_topo::LoopId]) {
    let mut cs: Vec<forge_topo::CoedgeId> = gone.iter().flat_map(|&l| body.lp(l).coedges()).collect();
    cs.sort_unstable();
    let face = &mut body.faces[f.index()];
    let Some(q) = face.query.get_mut() else { return };
    let Some(info) = std::sync::Arc::get_mut(q).and_then(|a| a.downcast_mut::<FaceInfo>()) else {
        face.query.clear();
        return;
    };
    let mut dead = 0;
    for e in &mut info.edges {
        if e.0 != GONE && cs.binary_search(&e.0).is_ok() {
            *e = (GONE, Box3::EMPTY);
        }
        dead += usize::from(e.0 == GONE);
    }
    info.drawn = std::sync::OnceLock::new();
    info.simple = std::sync::OnceLock::new();
    if 2 * dead > info.edges.len() {
        face.query.clear();
    }
}

/// Face `f` gained the loop `lp` in place: its kept index (if any) takes the loop's edges.
pub fn extend_query(body: &mut Body, f: FaceId, lp: forge_topo::LoopId) {
    let entries: Vec<_> = body.lp(lp).coedges().map(|c| (c, edge_entry(body, c))).collect();
    let plane = match &body.face(f).surface {
        Surface::Plane(pl) => Some(*pl),
        _ => None,
    };
    let face = &mut body.faces[f.index()];
    let Some(q) = face.query.get_mut() else { return };
    let Some(info) = std::sync::Arc::get_mut(q).and_then(|a| a.downcast_mut::<FaceInfo>()) else {
        // shared with a copy of the body: build afresh when next asked
        face.query.clear();
        return;
    };
    for (c, (sag, b)) in entries {
        if let Some(bands) = &mut info.bands {
            bands.insert(info.edges.len() as u32, &b);
        }
        info.edges.push((c, b));
        info.reach = info.reach.max(sag);
    }
    if info.bands.is_none()
        && info.edges.len() >= 16
        && let Some(pl) = plane
    {
        info.bands = Some(Bands::build(&pl, &info.edges));
    }
    info.drawn = std::sync::OnceLock::new();
    info.simple = std::sync::OnceLock::new();
}

impl Bands {
    fn build(plane: &forge_geom::Plane, edges: &[(forge_topo::CoedgeId, Box3)]) -> Bands {
        let (sn, cs) = forge_math::dmath::sin_cos(RAY_ANGLES[0]);
        let d = plane.frame.x() * cs + plane.frame.y() * sn;
        let across = plane.frame.z().cross(d);
        let origin = plane.frame.origin;
        let span = |b: &Box3| {
            let mut r = Interval::EMPTY;
            for &x in &[b.x.lo, b.x.hi] {
                for &y in &[b.y.lo, b.y.hi] {
                    for &z in &[b.z.lo, b.z.hi] {
                        r.include((Point3::new(x, y, z) - origin).dot(across));
                    }
                }
            }
            r
        };
        let spans: Vec<Interval> = edges.iter().map(|(_, b)| span(b)).collect();
        let all = spans.iter().fold(Interval::EMPTY, |a, s| a.hull(s));
        let n = (edges.len() / 4).clamp(8, 512);
        let inv = n as f64 / all.width().max(1e-9);
        let mut rows = vec![Vec::new(); n];
        for (i, sp) in spans.iter().enumerate() {
            let (a, b) = (
                ((sp.lo - all.lo) * inv) as usize,
                ((sp.hi - all.lo) * inv) as usize,
            );
            for row in rows.iter_mut().take(b.min(n - 1) + 1).skip(a.min(n - 1)) {
                row.push(i as u32);
            }
        }
        Bands {
            origin,
            across,
            w0: all.lo,
            inv,
            rows,
        }
    }

    /// Adds edge `i` with box `b`; a box past the bands' span lands in the end rows.
    fn insert(&mut self, i: u32, b: &Box3) {
        let mut sp = Interval::EMPTY;
        for &x in &[b.x.lo, b.x.hi] {
            for &y in &[b.y.lo, b.y.hi] {
                for &z in &[b.z.lo, b.z.hi] {
                    sp.include((Point3::new(x, y, z) - self.origin).dot(self.across));
                }
            }
        }
        let n = self.rows.len();
        let row = |w: f64| (((w - self.w0) * self.inv).floor().max(0.0) as usize).min(n - 1);
        let (a, z) = (row(sp.lo), row(sp.hi));
        for r in &mut self.rows[a..=z] {
            r.push(i);
        }
    }

    /// Rows a box's span across meets.
    fn rows(&self, b: &Box3) -> std::ops::RangeInclusive<usize> {
        let mut sp = Interval::EMPTY;
        for &x in &[b.x.lo, b.x.hi] {
            for &y in &[b.y.lo, b.y.hi] {
                for &z in &[b.z.lo, b.z.hi] {
                    sp.include((Point3::new(x, y, z) - self.origin).dot(self.across));
                }
            }
        }
        let n = self.rows.len();
        let row = |w: f64| (((w - self.w0) * self.inv).floor().max(0.0) as usize).min(n - 1);
        row(sp.lo)..=row(sp.hi)
    }

    fn row(&self, w: f64) -> Option<usize> {
        let k = (w - self.w0) * self.inv;
        (k >= 0.0 && (k as usize) < self.rows.len()).then_some(k as usize)
    }

    /// Edges whose band range meets [w - r, w + r].
    fn near(&self, p: Point3, r: f64, out: &mut Vec<u32>) {
        let w = (p - self.origin).dot(self.across);
        let lo = ((w - r - self.w0) * self.inv).floor().max(0.0) as usize;
        let hi = (((w + r - self.w0) * self.inv).floor().max(0.0) as usize).min(self.rows.len() - 1);
        out.clear();
        for row in self.rows.iter().take(hi + 1).skip(lo) {
            out.extend_from_slice(row);
        }
        out.sort_unstable();
        out.dedup();
    }
}

pub struct Drawn {
    pub loops: Vec<UvLoop>,
    centre: Point2,
    /// The parameter window the loops are drawn in.
    window: Box2,
}

pub struct Index<'a> {
    pub body: &'a Body,
    pub bbox: Box3,
    /// Every face's box by id (empty for a removed face), for whole-body scans: made on the
    /// first, as most booleans on a large body make none.
    boxes: std::sync::OnceLock<Vec<Box3>>,
}

/// A box holding face `f` (cached on the face).
pub fn face_box(body: &Body, f: FaceId) -> Box3 {
    body.face_box(f)
}

/// Face `f`'s point-in-face index: built on first use, kept on the face.
fn info(body: &Body, f: FaceId) -> &FaceInfo {
    let face = body.face(f);
    let q = face.query.get_or_init(|| {
        let mut edges = Vec::new();
        let mut reach = tol::LINEAR;
        for &l in &face.loops {
            for c in body.lp(l).coedges() {
                let (e, b) = edge_entry(body, c);
                edges.push((c, b));
                reach = reach.max(e);
            }
        }
        let bands = match &face.surface {
            Surface::Plane(pl) if edges.len() >= 16 => Some(Bands::build(pl, &edges)),
            _ => None,
        };
        std::sync::Arc::new(FaceInfo {
            edges,
            reach,
            drawn: std::sync::OnceLock::new(),
            bands,
            simple: std::sync::OnceLock::new(),
        })
    });
    q.downcast_ref::<FaceInfo>()
        .expect("a face's query is a FaceInfo")
}

impl<'a> Index<'a> {
    pub fn new(body: &'a Body) -> Self {
        let ix = Index {
            body,
            bbox: Box3::EMPTY,
            boxes: std::sync::OnceLock::new(),
        };
        // a large body keeps hulls of its faces' boxes from one boolean to the next; a small
        // one (a tool) is cheaper scanned whole
        let bbox = if body.faces.len() > 64 {
            body.faces_box()
        } else {
            ix.boxes().iter().fold(Box3::EMPTY, |a, b| a.hull(b))
        };
        Index { bbox, ..ix }
    }

    /// A box guaranteed to hold face `f`; empty, overlapping nothing, for a removed face.
    pub fn face_box(&self, f: FaceId) -> Box3 {
        match self.boxes.get() {
            Some(b) => b[f.index()],
            None => self.body.live_face_box(f),
        }
    }

    pub fn boxes(&self) -> &[Box3] {
        self.boxes.get_or_init(|| {
            (0..self.body.faces.len() as u32)
                .map(FaceId)
                .map(|f| match self.body.face(f).alive {
                    true => self.body.face_box(f),
                    false => Box3::EMPTY,
                })
                .collect()
        })
    }

    fn info(&self, f: FaceId) -> &FaceInfo {
        info(self.body, f)
    }

    fn drawn(&self, f: FaceId) -> &Drawn {
        self.info(f).drawn.get_or_init(|| {
            let loops = face_uv_loops(self.body, f);
            let (mut u, mut v) = (Interval::EMPTY, Interval::EMPTY);
            for p in loops.iter().flat_map(|l| l.points.iter()) {
                u.include(p.x);
                v.include(p.y);
            }
            let centre = loops.first().map_or(Point2::ORIGIN, |l| {
                let n = l.points.len().max(1) as f64;
                let s = l.points.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.y));
                Point2::new(s.0 / n, s.1 / n)
            });
            Drawn {
                loops,
                centre,
                window: Box2::new(u, v),
            }
        })
    }

    /// The uses of face `f`'s edges whose boxes meet `b`, found through the face's bands where it
    /// keeps them, in no set order.
    pub fn coedges_meeting(&self, f: FaceId, b: &Box3) -> Vec<forge_topo::CoedgeId> {
        let info = self.info(f);
        let hit = |i: usize| {
            let (c, ref eb) = info.edges[i];
            (c != GONE && eb.overlaps(b)).then_some(c)
        };
        match &info.bands {
            Some(bands) => {
                let mut ids: Vec<u32> = bands.rows[bands.rows(b)].iter().flatten().copied().collect();
                ids.sort_unstable();
                ids.dedup();
                ids.into_iter().filter_map(|i| hit(i as usize)).collect()
            }
            None => (0..info.edges.len()).filter_map(hit).collect(),
        }
    }

    /// Where a point that lies on (or within tolerance of) face `f`'s surface
    /// falls relative to the face.
    pub fn point_in_face(&self, f: FaceId, p: Point3) -> Where {
        self.point_in_face_past(f, p, &[])
    }

    /// As `point_in_face`, for a point of a curve that crosses edges `crossed` nearby: within
    /// tolerance of only those it is not on the boundary but on the side of it the curve went
    /// (a stretch shorter than two tolerances, between two crossings, is still decided).
    pub fn point_in_face_past(&self, f: FaceId, p: Point3, crossed: &[forge_topo::EdgeId]) -> Where {
        let body = self.body;
        let face = body.face(f);
        let info = self.info(f);
        if let Some(w) = info
            .simple
            .get_or_init(|| Simple::of(body, f))
            .and_then(|s| s.place(p))
        {
            return w;
        }
        // nearest boundary point, exactly
        let mut best: Option<(f64, forge_topo::CoedgeId, f64)> = None;
        let mut near: Vec<u32> = Vec::new();
        let candidates: Box<dyn Iterator<Item = &(forge_topo::CoedgeId, Box3)>> = match &info.bands {
            Some(bands) => {
                bands.near(p, info.reach, &mut near);
                Box::new(near.iter().map(|&i| &info.edges[i as usize]))
            }
            None => Box::new(info.edges.iter()),
        };
        for &(c, ref b) in candidates {
            if c == GONE || box_distance(b, p) > info.reach.min(best.map_or(f64::INFINITY, |x| x.0)) {
                continue;
            }
            let e = body.edge(body.coedge(c).edge);
            let t = e.curve.closest(p, e.range);
            let d = e.curve.eval(t).distance(p);
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, c, t));
            }
        }
        if let Some((d, c, t)) = best {
            let co = body.coedge(c);
            let e = body.edge(co.edge);
            // within a tolerant vertex's ball counts as on it
            let reach = if t <= e.range.lo {
                body.vertex(e.start).tol
            } else if t >= e.range.hi {
                body.vertex(e.end).tol
            } else {
                tol::LINEAR
            };
            // a seam (an edge the face uses twice) has two sides that are both this face
            let interior = || {
                let seam = body.partner(c).is_some_and(|o| body.coedge_face(o) == f);
                !seam
                    && t > e.range.lo + 1e-9 * (1.0 + e.range.width())
                    && t < e.range.hi - 1e-9 * (1.0 + e.range.width())
            };
            // near only an edge the curve crossed: decided by side below
            let past = || {
                crossed.contains(&co.edge)
                    && d > 1e-3 * tol::LINEAR
                    && interior()
                    && !info.edges.iter().any(|&(o, ref b)| {
                        o != GONE
                            && !crossed.contains(&body.coedge(o).edge)
                            && box_distance(b, p) <= tol::LINEAR
                            && {
                                let oe = body.edge(body.coedge(o).edge);
                                oe.curve.eval(oe.curve.closest(p, oe.range)).distance(p) <= tol::LINEAR
                            }
                    })
            };
            if d <= reach && !past() {
                return Where::On;
            }
            // the polygon of the loop can be off by its chord sag near curved edges; decide those by side
            if d <= sag_bound(&e.curve) && interior() {
                let q = e.curve.eval(t);
                let along = if co.forward {
                    e.curve.deriv(t).d1
                } else {
                    -e.curve.deriv(t).d1
                };
                if let Some(n) = body.face_normal(f, face.surface.project(q)) {
                    let left = n.cross(along);
                    return if left.dot(p - q) > 0.0 {
                        Where::Inside
                    } else {
                        Where::Outside
                    };
                }
            }
        }
        if let Surface::Plane(plane) = &face.surface {
            for (k, angle) in RAY_ANGLES.into_iter().enumerate() {
                // the first direction can look at p's band only: an edge the ray meets overlaps it
                let subset: Option<&[u32]> = match (&info.bands, k) {
                    (Some(bands), 0) => match bands.row((p - bands.origin).dot(bands.across)) {
                        Some(r) => Some(&bands.rows[r]),
                        None => Some(&[]),
                    },
                    _ => None,
                };
                if let Some(inside) = planar_parity(body, plane, p, &info.edges, subset, angle) {
                    return if inside { Where::Inside } else { Where::Outside };
                }
            }
        }
        let ruling = match &face.surface {
            Surface::Cylinder(c) => Some((c.frame, None)),
            Surface::Cone(k) => Some((k.frame, Some(k.apex()))),
            _ => None,
        };
        if let Some((frame, apex)) = ruling
            && let Some(inside) = ruling_parity(body, &frame, apex, p, &info.edges)
        {
            return if inside { Where::Inside } else { Where::Outside };
        }
        let drawn = self.drawn(f);
        let uv = into_window(body, f, face.surface.project(p), drawn);
        if point_in_loops(uv, &drawn.loops) {
            Where::Inside
        } else {
            Where::Outside
        }
    }

    /// Inside, outside or on the solid.
    pub fn classify(&self, p: Point3) -> Where {
        if !self.bbox.contains(p) {
            return Where::Outside;
        }
        let boxes = self.boxes();
        for f in self.body.face_ids() {
            if boxes[f.index()].contains(p)
                && self.body.face(f).surface.distance(p) <= tol::LINEAR
                && self.point_in_face(f, p) != Where::Outside
            {
                return Where::On;
            }
        }
        'ray: for d in RAYS {
            let dir = Vec3::new(d[0], d[1], d[2])
                .normalize()
                .expect("fixed ray directions are nonzero");
            let line = Line::new(p, dir);
            let mut crossings = 0usize;
            for f in self.body.face_ids() {
                if !ray_hits_box(p, dir.get(), &boxes[f.index()]) {
                    continue;
                }
                match line_surface(&line, &self.body.face(f).surface) {
                    LineSurface::OnSurface => continue 'ray,
                    LineSurface::Hits(hits) => {
                        for h in hits.iter().filter(|h| h.t > tol::LINEAR) {
                            if h.contact == Contact::Touching {
                                continue 'ray;
                            }
                            match self.point_in_face(f, h.point) {
                                Where::Inside => crossings += 1,
                                Where::On => continue 'ray,
                                Where::Outside => {}
                            }
                        }
                    }
                }
            }
            return if crossings % 2 == 1 {
                Where::Inside
            } else {
                Where::Outside
            };
        }
        Where::On
    }
}

/// Whether a circle in planar face `f`'s plane lies inside the face (true) or outside it, every
/// boundary edge more than `clear` from the circle and none inside it; None when an edge comes
/// nearer, may lie inside, or the parity ray grazes. Only edges near the circle and in the
/// centre's band are looked at.
pub(crate) fn circle_in_face(
    body: &Body,
    f: FaceId,
    circle: &forge_geom::Circle,
    clear: f64,
) -> Option<bool> {
    let Surface::Plane(plane) = &body.face(f).surface else {
        return None;
    };
    let info = info(body, f);
    let (c, r) = (circle.frame.origin, circle.radius);
    let n = circle.frame.z().get();
    let reach = Box3::from_point(c).grow(r + clear);
    let mut near: Vec<u32> = Vec::new();
    let candidates: Box<dyn Iterator<Item = &(forge_topo::CoedgeId, Box3)>> = match &info.bands {
        Some(bands) => {
            bands.near(c, r + clear, &mut near);
            Box::new(near.iter().map(|&i| &info.edges[i as usize]))
        }
        None => Box::new(info.edges.iter()),
    };
    for &(co, ref b) in candidates {
        if co == GONE || !b.overlaps(&reach) {
            continue;
        }
        let e = body.edge(body.coedge(co).edge);
        let clear = match &e.curve {
            Curve::Line(l) => l.eval(l.closest(c, e.range)).distance(c) > r + clear,
            Curve::Circle(k) if k.frame.z().dot(n).abs() >= 1.0 - 1e-12 => {
                let d = k.frame.origin.distance(c);
                d > r + k.radius + clear || d + r < k.radius - clear
            }
            _ => false,
        };
        if !clear {
            return None;
        }
    }
    // no edge inside the disk or on the circle: the centre is where the circle is
    for (k, angle) in RAY_ANGLES.into_iter().enumerate() {
        let subset: Option<&[u32]> = match (&info.bands, k) {
            (Some(bands), 0) => match bands.row((c - bands.origin).dot(bands.across)) {
                Some(r) => Some(&bands.rows[r]),
                None => Some(&[]),
            },
            _ => None,
        };
        if let Some(inside) = planar_parity(body, plane, c, &info.edges, subset, angle) {
            return Some(inside);
        }
    }
    None
}

/// Whether a closed loop of lines and arcs (`pieces`, end to end) in planar face `f`'s plane
/// lies inside the face (true) or outside it, no boundary edge within `clear` of it or inside
/// it; None when that is not shown simply (an edge nearer, a curve of another kind, a grazing
/// parity ray).
pub(crate) fn loop_in_face(body: &Body, f: FaceId, pieces: &[(Curve, Interval)], clear: f64) -> Option<bool> {
    let Surface::Plane(plane) = &body.face(f).surface else {
        return None;
    };
    let info = info(body, f);
    let n = plane.normal_vec().get();
    // whole circles: an edge anywhere on an arc's circle is followed by the general boolean
    let reach = pieces
        .iter()
        .fold(Box3::EMPTY, |b, (c, r)| {
            let r = match c {
                Curve::Circle(_) => Interval::new(0.0, forge_math::dmath::TAU),
                _ => *r,
            };
            b.hull(&c.bbox(r))
        })
        .grow(clear);
    let mut near: Vec<u32> = Vec::new();
    let candidates: Box<dyn Iterator<Item = &(forge_topo::CoedgeId, Box3)>> = match &info.bands {
        Some(bands) => {
            bands.near(reach.center(), reach.diagonal() / 2.0, &mut near);
            Box::new(near.iter().map(|&i| &info.edges[i as usize]))
        }
        None => Box::new(info.edges.iter()),
    };
    for &(co, ref b) in candidates {
        if co == GONE || !b.overlaps(&reach) {
            continue;
        }
        let e = body.edge(body.coedge(co).edge);
        if !pieces
            .iter()
            .all(|(c, r)| apart(&e.curve, e.range, c, *r, n, clear))
        {
            return None;
        }
        // clear of the loop: wholly inside it or wholly outside, as its start is
        if in_loop(e.curve.eval(e.range.lo), pieces, plane)? {
            return None;
        }
    }
    let (c, r) = &pieces[0];
    let p = c.eval(r.mid());
    for (k, angle) in RAY_ANGLES.into_iter().enumerate() {
        let subset: Option<&[u32]> = match (&info.bands, k) {
            (Some(bands), 0) => match bands.row((p - bands.origin).dot(bands.across)) {
                Some(r) => Some(&bands.rows[r]),
                None => Some(&[]),
            },
            _ => None,
        };
        if let Some(inside) = planar_parity(body, plane, p, &info.edges, subset, angle) {
            return Some(inside);
        }
    }
    None
}

/// Whether two lines or arcs of a plane (normal `n`) are certainly more than `d` apart (arcs
/// judged as their whole circles).
fn apart(a: &Curve, ra: Interval, b: &Curve, rb: Interval, n: Vec3, d: f64) -> bool {
    let round = |c: &forge_geom::Circle| c.frame.z().dot(n).abs() >= 1.0 - 1e-12;
    match (a, b) {
        (Curve::Line(l), Curve::Line(m)) => {
            segment_gap(l.eval(ra.lo), l.eval(ra.hi), m.eval(rb.lo), m.eval(rb.hi)) > d
        }
        (Curve::Line(l), Curve::Circle(k)) | (Curve::Circle(k), Curve::Line(l)) if round(k) => {
            let r = if matches!(a, Curve::Line(_)) { ra } else { rb };
            let c = k.frame.origin;
            let (p, q) = (l.eval(r.lo), l.eval(r.hi));
            l.eval(l.closest(c, r)).distance(c) > k.radius + d
                || p.distance(c).max(q.distance(c)) < k.radius - d
        }
        (Curve::Circle(k), Curve::Circle(m)) if round(k) && round(m) => {
            let g = k.frame.origin.distance(m.frame.origin);
            g > k.radius + m.radius + d || g + k.radius < m.radius - d || g + m.radius < k.radius - d
        }
        _ => false,
    }
}

/// The least distance between segments pq and uv.
fn segment_gap(p: Point3, q: Point3, u: Point3, v: Point3) -> f64 {
    let at = |a: Point3, b: Point3, x: Point3| {
        let ab = b - a;
        let l = ab.dot(ab);
        let t = if l > 0.0 {
            ((x - a).dot(ab) / l).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (a + ab * t).distance(x)
    };
    let ends = at(p, q, u).min(at(p, q, v)).min(at(u, v, p)).min(at(u, v, q));
    // crossing segments meet inside both: the ends' distances miss that
    let (d1, d2) = (q - p, v - u);
    let m = d1.cross(d2);
    if m.length() > 0.0 {
        let s = (u - p).cross(d2).dot(m) / m.dot(m);
        let t = (u - p).cross(d1).dot(m) / m.dot(m);
        if (0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&t) {
            return 0.0;
        }
    }
    ends
}

/// Whether point `p` of the plane lies inside the loop: ray parity over its pieces; None if
/// the ray grazes one.
fn in_loop(p: Point3, pieces: &[(Curve, Interval)], plane: &forge_geom::Plane) -> Option<bool> {
    let n = plane.normal_vec().get();
    'ray: for angle in RAY_ANGLES {
        let (sn, cs) = forge_math::dmath::sin_cos(angle);
        let d = plane.frame.x() * cs + plane.frame.y() * sn;
        let mut inside = false;
        for (c, r) in pieces {
            match plane_ray_crossings(n, p, d, c, *r) {
                Some(k) => inside ^= k % 2 == 1,
                None => continue 'ray,
            }
        }
        return Some(inside);
    }
    None
}

/// Shifts parameters by whole periods into the face's drawing window (the
/// nearest-to-centre shift misplaces points near a window edge when the face
/// spans most of a period).
fn into_window(body: &Body, f: FaceId, uv: Point2, info: &Drawn) -> Point2 {
    let d = body.face(f).surface.domain();
    let fit = |x: f64, w: Interval, c: f64, period: Option<f64>| match period {
        Some(p) => [0.0, -p, p, -2.0 * p, 2.0 * p]
            .iter()
            .map(|k| x + k)
            .find(|&y| y >= w.lo - 1e-12 && y <= w.hi + 1e-12)
            .unwrap_or_else(|| x + p * ((c - x) / p).round()),
        None => x,
    };
    Point2::new(
        fit(uv.x, info.window.u, info.centre.x, d.u.period),
        fit(uv.y, info.window.v, info.centre.y, d.v.period),
    )
}

/// Fixed, deliberately irregular ray directions: a ray along one of these
/// rarely grazes a model edge, and the next one is tried if it does.
const RAYS: [[f64; 3]; 8] = [
    [0.5773, 0.6071, 0.5463],
    [-0.3149, 0.8352, 0.4510],
    [0.7231, -0.2017, 0.6606],
    [-0.6418, -0.5528, 0.5316],
    [0.1937, 0.4123, -0.8902],
    [-0.8836, 0.3091, -0.3518],
    [0.4402, -0.8577, -0.2657],
    [-0.0912, -0.2384, 0.9669],
];

/// How far a loop polygon (default sampling, pi/16 per segment) can sit from
/// a curved edge, plus tolerance.
fn sag_bound(c: &Curve) -> f64 {
    let r = match c {
        Curve::Line(_) => 0.0,
        Curve::Circle(k) => k.radius,
        Curve::Ellipse(e) => e.major * e.major / e.minor.max(tol::LINEAR),
        Curve::Parabola(_) | Curve::Hyperbola(_) => 1e3,
        Curve::Intersection(k) => return 2.0 * k.sag + tol::LINEAR,
        Curve::Procedural(_) => return forge_topo::uv::PROC_SAG + tol::LINEAR,
    };
    0.01 * r + tol::LINEAR
}

/// Exact even-odd test for a point of a planar face: a ray from p in the
/// plane at `angle` to its x axis, counting crossings with each edge in
/// closed form (lines, circles, ellipses). None when the ray meets a vertex
/// or an edge the closed forms do not cover, so another direction is tried.
fn planar_parity(
    body: &Body,
    plane: &forge_geom::Plane,
    p: Point3,
    edges: &[(forge_topo::CoedgeId, Box3)],
    subset: Option<&[u32]>,
    angle: f64,
) -> Option<bool> {
    let (sn, cs) = forge_math::dmath::sin_cos(angle);
    let d = plane.frame.x() * cs + plane.frame.y() * sn;
    let mut inside = false;
    let all: Box<dyn Iterator<Item = &(forge_topo::CoedgeId, Box3)>> = match subset {
        Some(ix) => Box::new(ix.iter().map(|&i| &edges[i as usize])),
        None => Box::new(edges.iter()),
    };
    for &(c, ref b) in all {
        if c == GONE || !ray_hits_box(p, d, b) {
            continue;
        }
        let e = body.edge(body.coedge(c).edge);
        // a seam cannot occur on a plane; each use of an edge here is one crossing set
        if plane_ray_crossings(plane.frame.z().get(), p, d, &e.curve, e.range)? % 2 == 1 {
            inside = !inside;
        }
    }
    Some(inside)
}

/// How many times the ray p + s d (s > 0), in the plane with normal `n`, crosses a line or conic
/// of that plane over `range`, in closed form. None when it passes through an end, runs along
/// the curve, or the curve is of another kind.
pub(crate) fn plane_ray_crossings(
    n: Vec3,
    p: Point3,
    d: Vec3,
    curve: &Curve,
    range: Interval,
) -> Option<usize> {
    let hits: Vec<(f64, f64)> = match curve {
        Curve::Line(l) => {
            // ray p + s d against segment o + t dir, t in range
            let w = l.origin - p;
            // which way along the ray the line lies decides parity: its sign is exact
            let den = triple(n, d, l.dir.get());
            if den == 0.0 {
                if l.dir.cross(w).dot(n).abs() <= tol::LINEAR {
                    return None; // the ray runs along this edge
                }
                return Some(0);
            }
            let along = triple(n, l.dir.get(), w);
            let s = if along != 0.0 && (along > 0.0) == (den < 0.0) {
                1.0
            } else {
                -1.0
            };
            let t = d.cross(w).dot(n) / -den;
            if !t.is_finite() {
                return None;
            }
            vec![(s, t)]
        }
        Curve::Circle(k) => conic_ray(&k.frame, k.radius, k.radius, p, d),
        Curve::Ellipse(el) => conic_ray(&el.frame, el.major, el.minor, p, d),
        _ => return None,
    };
    let periodic = curve.domain().period;
    let mut n = 0;
    for (s, t) in hits {
        if s <= 0.0 {
            continue;
        }
        let t = match periodic {
            Some(per) => range.lo + (t - range.lo).rem_euclid(per),
            None => t,
        };
        let span = range.width().max(1e-300);
        let margin = 1e-9 * (1.0 + span);
        if (t - range.lo).abs() <= margin || (t - range.hi).abs() <= margin {
            return None; // through a vertex
        }
        if t > range.lo && t < range.hi {
            n += 1;
        }
    }
    Some(n)
}

/// Exact even-odd test for a point of a cylinder or cone face (axis the frame's z): the ruling
/// from p away from the apex (up the axis on a cylinder) leaves the bounded face, so the edges it
/// crosses decide; an edge crosses it where it meets the plane of the axis and p on p's side, in
/// closed form for lines and conics. None when the ruling meets a vertex, runs along an edge,
/// passes within a tolerance of p, or meets an edge the closed forms do not cover.
fn ruling_parity(
    body: &Body,
    frame: &forge_math::Frame,
    apex: Option<Point3>,
    p: Point3,
    edges: &[(forge_topo::CoedgeId, Box3)],
) -> Option<bool> {
    let z = frame.z().get();
    let rel = p - frame.origin;
    let radial = (rel - z * rel.dot(z)).normalize()?.get();
    let w = match apex {
        Some(a) => (p - a).normalize()?.get(),
        None => z,
    };
    // normal of the plane holding the axis and p
    let m = z.cross(radial);
    let mut inside = false;
    for &(c, ref b) in edges {
        if c == GONE || !ray_hits_box(p, w, b) {
            continue;
        }
        let e = body.edge(body.coedge(c).edge);
        let ts: Vec<f64> = match &e.curve {
            Curve::Line(l) => {
                let (den, at) = (l.dir.get().dot(m), (l.origin - frame.origin).dot(m));
                if den.abs() <= 1e-12 {
                    // in the plane on p's side: along the ruling
                    if at.abs() <= tol::LINEAR && (l.origin - frame.origin).dot(radial) > 0.0 {
                        return None;
                    }
                    continue;
                }
                vec![-at / den]
            }
            Curve::Circle(k) => conic_plane(&k.frame, k.radius, k.radius, frame.origin, m)?,
            Curve::Ellipse(el) => conic_plane(&el.frame, el.major, el.minor, frame.origin, m)?,
            _ => return None,
        };
        let periodic = e.curve.domain().period;
        for t in ts {
            let h = e.curve.eval(t);
            let (side, along) = ((h - frame.origin).dot(radial), (h - p).dot(w));
            if side.abs() <= tol::LINEAR || along.abs() <= 10.0 * tol::LINEAR {
                return None;
            }
            if side < 0.0 || along < 0.0 {
                continue;
            }
            let t = match periodic {
                Some(per) => e.range.lo + (t - e.range.lo).rem_euclid(per),
                None => t,
            };
            let margin = 1e-9 * (1.0 + e.range.width());
            if (t - e.range.lo).abs() <= margin || (t - e.range.hi).abs() <= margin {
                return None; // through a vertex
            }
            if t > e.range.lo && t < e.range.hi {
                inside = !inside;
            }
        }
    }
    Some(inside)
}

/// Where a conic o + a cos t x + b sin t y crosses the plane through `at` with normal `m`: its
/// parameters (none if it misses). None when it touches the plane or lies in it.
fn conic_plane(f: &forge_math::Frame, a: f64, b: f64, at: Point3, m: Vec3) -> Option<Vec<f64>> {
    let (ca, cb, cc) = (a * f.x().dot(m), b * f.y().dot(m), (f.origin - at).dot(m));
    let r = (ca * ca + cb * cb).sqrt();
    if r <= 1e-12 * (a + b) {
        return (cc.abs() > tol::LINEAR).then(Vec::new);
    }
    let k = -cc / r;
    if k.abs() > 1.0 + 1e-9 {
        return Some(Vec::new());
    }
    if k.abs() >= 1.0 - 1e-9 {
        return None;
    }
    let (phi, d) = (forge_math::dmath::atan2(cb, ca), forge_math::dmath::acos(k));
    Some(vec![phi - d, phi + d])
}

/// Where the ray p + s d (s any sign) meets a conic o + a cos t x + b sin t y
/// lying in a plane that contains the ray: (s, t) pairs. A tangent ray gives
/// none (it touches without crossing).
fn conic_ray(f: &forge_math::Frame, a: f64, b: f64, p: Point3, d: Vec3) -> Vec<(f64, f64)> {
    let q = f.point_to_local(p);
    let v = f.vec_to_local(d);
    // ((qx + s vx) / a)^2 + ((qy + s vy) / b)^2 = 1
    let (qx, qy, vx, vy) = (q.x / a, q.y / b, v.x / a, v.y / b);
    let (aa, bb, cc) = (
        vx * vx + vy * vy,
        2.0 * (qx * vx + qy * vy),
        qx * qx + qy * qy - 1.0,
    );
    let disc = bb * bb - 4.0 * aa * cc;
    if disc <= 1e-12 * (bb * bb + (4.0 * aa * cc).abs()) || aa == 0.0 {
        return Vec::new();
    }
    let r = disc.sqrt();
    let qq = -0.5 * (bb + if bb >= 0.0 { r } else { -r });
    [qq / aa, cc / qq]
        .into_iter()
        .map(|s| (s, forge_math::dmath::atan2(qy + s * vy, qx + s * vx)))
        .collect()
}

fn box_distance(b: &Box3, p: Point3) -> f64 {
    let gap = |i: Interval, x: f64| (i.lo - x).max(x - i.hi).max(0.0);
    let (dx, dy, dz) = (gap(b.x, p.x), gap(b.y, p.y), gap(b.z, p.z));
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub(crate) fn ray_hits_box(o: Point3, d: Vec3, b: &Box3) -> bool {
    let (mut t0, mut t1) = (0.0f64, f64::INFINITY);
    for (oi, di, iv) in [(o.x, d.x, b.x), (o.y, d.y, b.y), (o.z, d.z, b.z)] {
        if di.abs() < 1e-300 {
            if oi < iv.lo || oi > iv.hi {
                return false;
            }
            continue;
        }
        let (a, c) = ((iv.lo - oi) / di, (iv.hi - oi) / di);
        t0 = t0.max(a.min(c));
        t1 = t1.min(a.max(c));
        if t0 > t1 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Vec2, dmath};
    use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};

    const Y: Axis2 = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };

    fn agree(body: &Body, inside: impl Fn(Point3) -> Option<bool>, size: f64, seed: u64) {
        let ix = Index::new(body);
        let mut r = Rng::new(seed);
        let mut checked = 0;
        for _ in 0..3000 {
            let p = Point3::new(r.range(-size, size), r.range(-size, size), r.range(-size, size));
            let Some(want) = inside(p) else { continue };
            let got = ix.classify(p);
            assert_eq!(got, if want { Where::Inside } else { Where::Outside }, "{p:?}");
            checked += 1;
        }
        assert!(checked > 2000);
    }

    /// None when the point is too close to call.
    fn margin(d: f64) -> Option<bool> {
        (d.abs() > 1e-6).then_some(d < 0.0)
    }

    #[test]
    fn box_and_plate() {
        let b = extrude(
            &Profile {
                plane: Frame::WORLD,
                loops: vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)],
            },
            5.0,
            1,
        )
        .unwrap();
        agree(
            &b,
            |p| {
                margin(
                    (-p.x)
                        .max(p.x - 10.0)
                        .max(-p.y)
                        .max(p.y - 20.0)
                        .max(-p.z)
                        .max(p.z - 5.0),
                )
            },
            25.0,
            1,
        );
        let plate = Profile {
            plane: Frame::WORLD,
            loops: vec![
                Profile::rectangle(Point2::ORIGIN, 30.0, 20.0, 0),
                Profile::circle(Point2::new(15.0, 10.0), 4.0, 10),
            ],
        };
        let b = extrude(&plate, 6.0, 1).unwrap();
        agree(
            &b,
            |p| {
                let hole = 4.0 - ((p.x - 15.0).powi(2) + (p.y - 10.0).powi(2)).sqrt();
                margin(
                    (-p.x)
                        .max(p.x - 30.0)
                        .max(-p.y)
                        .max(p.y - 20.0)
                        .max(-p.z)
                        .max(p.z - 6.0)
                        .max(hole),
                )
            },
            35.0,
            2,
        );
    }

    #[test]
    fn revolved_solids() {
        let q = dmath::FRAC_PI_2;
        let half = ProfileLoop::new(vec![
            (
                Segment::Arc {
                    center: Point2::ORIGIN,
                    radius: 5.0,
                    start: -q,
                    sweep: 2.0 * q,
                },
                0,
            ),
            (
                Segment::Line {
                    a: Point2::new(0.0, 5.0),
                    b: Point2::new(0.0, -5.0),
                },
                1,
            ),
        ]);
        let sphere = revolve(
            &Profile {
                plane: Frame::WORLD,
                loops: vec![half],
            },
            Y,
            dmath::TAU,
            1,
        )
        .unwrap();
        agree(&sphere, |p| margin(p.to_vec().length() - 5.0), 8.0, 3);

        let torus = revolve(
            &Profile {
                plane: Frame::WORLD,
                loops: vec![Profile::circle(Point2::new(10.0, 0.0), 3.0, 0)],
            },
            Y,
            dmath::TAU,
            1,
        )
        .unwrap();
        agree(
            &torus,
            |p| {
                margin({
                    let d = (p.x * p.x + p.z * p.z).sqrt() - 10.0;
                    (d * d + p.y * p.y).sqrt() - 3.0
                })
            },
            15.0,
            4,
        );

        let rect = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(5.0, 0.0), 4.0, 10.0, 0)],
        };
        let quarter = revolve(&rect, Y, q, 1).unwrap();
        agree(
            &quarter,
            |p| {
                let rho = (p.x * p.x + p.z * p.z).sqrt();
                let phi = dmath::atan2(-p.z, p.x);
                let ang = (-phi).max(phi - q) * rho;
                margin((5.0 - rho).max(rho - 9.0).max(-p.y).max(p.y - 10.0).max(ang))
            },
            12.0,
            5,
        );
    }

    #[test]
    fn points_on_the_boundary() {
        let b = extrude(
            &Profile {
                plane: Frame::WORLD,
                loops: vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)],
            },
            5.0,
            1,
        )
        .unwrap();
        let ix = Index::new(&b);
        assert_eq!(ix.classify(Point3::new(5.0, 5.0, 5.0)), Where::On);
        assert_eq!(ix.classify(Point3::new(10.0, 20.0, 5.0)), Where::On);
        assert_eq!(ix.classify(Point3::new(5.0, 5.0, 2.5)), Where::Inside);
    }
}

#[cfg(test)]
mod planar_vs_polygon {
    use super::*;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Vec2};
    use forge_ops::{Axis2, Profile, revolve};

    #[test]
    fn agree() {
        let rect = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(5.0, 0.0), 4.0, 10.0, 0)],
        };
        let body = revolve(
            &rect,
            Axis2 {
                origin: Point2::ORIGIN,
                dir: Vec2::Y,
            },
            forge_math::dmath::FRAC_PI_2,
            1,
        )
        .unwrap();
        let ix = Index::new(&body);
        let mut r = Rng::new(5);
        for f in body.face_ids() {
            let Surface::Plane(pl) = body.face(f).surface else {
                continue;
            };
            for _ in 0..2000 {
                let p = pl
                    .frame
                    .plane_point(Point2::new(r.range(-12.0, 12.0), r.range(-12.0, 12.0)));
                let info = ix.info(f);
                // the drawn polygon is only trustworthy away from curved edges
                let near = info.edges.iter().any(|&(c, _)| {
                    let e = body.edge(body.coedge(c).edge);
                    e.curve.eval(e.curve.closest(p, e.range)).distance(p) < 0.1
                });
                if near {
                    continue;
                }
                let Some(fast) = planar_parity(&body, &pl, p, &info.edges, None, 0.1234) else {
                    continue;
                };
                let drawn = ix.drawn(f);
                let uv = into_window(&body, f, body.face(f).surface.project(p), drawn);
                let slow = point_in_loops(uv, &drawn.loops);
                assert_eq!(fast, slow, "face {f:?} p {p:?}");
            }
        }
    }
}

#[cfg(test)]
mod simple_vs_polygon {
    use super::*;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Vec2, dmath};
    use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};

    #[test]
    fn agree() {
        let q = dmath::FRAC_PI_2;
        let half = Profile {
            plane: Frame::WORLD,
            loops: vec![ProfileLoop::new(vec![
                (
                    Segment::Arc {
                        center: Point2::ORIGIN,
                        radius: 3.0,
                        start: -q,
                        sweep: 2.0 * q,
                    },
                    0,
                ),
                (
                    Segment::Line {
                        a: Point2::new(0.0, 3.0),
                        b: Point2::new(0.0, -3.0),
                    },
                    1,
                ),
            ])],
        };
        let axis = Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        };
        let disk = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::circle(Point2::new(1.0, 2.0), 2.5, 0)],
        };
        let bodies = [
            revolve(&half, axis, dmath::TAU, 1).unwrap(),
            revolve(&half, axis, 2.0, 1).unwrap(),
            revolve(&half, axis, 4.0, 1).unwrap(),
            extrude(&disk, 4.0, 1).unwrap(),
            extrude(&disk, -4.0, 1).unwrap(),
        ];
        let mut r = Rng::new(7);
        let mut decided = 0;
        for body in &bodies {
            let ix = Index::new(body);
            for f in body.face_ids() {
                let Some(simple) = Simple::of(body, f) else {
                    continue;
                };
                let surface = &body.face(f).surface;
                for _ in 0..500 {
                    let raw = Point3::new(r.range(-6.0, 6.0), r.range(-6.0, 6.0), r.range(-6.0, 6.0));
                    let p = surface.eval(surface.project(raw));
                    let Some(fast) = simple.place(p) else { continue };
                    let near = ix.info(f).edges.iter().any(|&(c, _)| {
                        let e = body.edge(body.coedge(c).edge);
                        e.curve.eval(e.curve.closest(p, e.range)).distance(p) < 0.05
                    });
                    if near {
                        continue;
                    }
                    let drawn = ix.drawn(f);
                    let uv = into_window(body, f, surface.project(p), drawn);
                    let slow = if point_in_loops(uv, &drawn.loops) {
                        Where::Inside
                    } else {
                        Where::Outside
                    };
                    assert_eq!(fast, slow, "face {f:?} {surface:?} p {p:?}");
                    decided += 1;
                }
            }
        }
        assert!(decided > 1000);
    }
}
