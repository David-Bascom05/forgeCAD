use crate::cdt::{CdtError, triangulate_refined};
use crate::mesh::TriMesh;
use forge_geom::{Curve, Surface, SurfaceGeom};
use forge_math::{Point2, dmath};
use forge_topo::uv::{align_loops, draw_loop};
use forge_topo::{Body, FaceId};

/// How closely the triangles must follow the exact shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quality {
    /// Largest gap between a chord and the true curve or surface (mm).
    pub chord: f64,
    /// Largest angle a single chord may turn through (radians).
    pub max_angle: f64,
}

impl Default for Quality {
    fn default() -> Self {
        Quality {
            chord: 0.01,
            max_angle: dmath::PI / 12.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TessError {
    Face { face: FaceId, error: CdtError },
}

/// Angle step that keeps chords on a circle of radius `r` within quality.
fn arc_step(r: f64, q: &Quality) -> f64 {
    let by_chord = if q.chord >= r {
        dmath::PI
    } else {
        2.0 * dmath::acos(1.0 - q.chord / r)
    };
    by_chord.min(q.max_angle).max(1e-4)
}

/// Chords along an edge. On a face curved both ways a chord along the edge and one across
/// it make a triangle that sags by both their sags: there each gets half of the tolerance,
/// a step shorter by sqrt 2 (`fine`).
fn edge_segments(curve: &Curve, range: forge_math::Interval, q: &Quality, fine: bool) -> usize {
    let width = range.width();
    let n = match curve {
        Curve::Line(_) => 1.0,
        Curve::Circle(c) => width / arc_step(c.radius, q),
        Curve::Ellipse(e) => width / arc_step(e.major, q),
        Curve::Parabola(_) | Curve::Hyperbola(_) => 64.0,
        // the chain already follows the curvature; split each link by the chord its sag allows
        Curve::Intersection(c) => width * (c.sag / q.chord).sqrt().max(1.0),
        Curve::Procedural(c) => c.segments(range, q.chord) as f64,
    };
    // a curved edge gets two chords at least: a face bounded by a line and a short arc
    // still has three corners to triangulate
    let least = if matches!(curve, Curve::Line(_)) { 1 } else { 2 };
    let n = if fine && least == 2 {
        n * core::f64::consts::SQRT_2
    } else {
        n
    };
    (n.ceil() as usize).max(least)
}

/// Whether the surface curves both ways, so a triangle on it sags by its sag along each side.
fn doubly_curved(s: &Surface) -> bool {
    matches!(s, Surface::Sphere(_) | Surface::Torus(_))
}

/// The surface's parameter scale, so distances in the scaled plane roughly
/// match distances on the surface and the triangulation is well shaped.
fn uv_scale(s: &Surface) -> (f64, f64) {
    match s {
        Surface::Plane(_) => (1.0, 1.0),
        Surface::Cylinder(c) => (c.radius, 1.0),
        Surface::Cone(c) => (c.radius.max(1.0), 1.0 / c.cos_a()),
        Surface::Sphere(sp) => (sp.radius, sp.radius),
        Surface::Torus(t) => (t.major + t.minor, t.minor),
        Surface::Procedural(pr) => pr.uv_scale(),
    }
}

/// Grid steps for points inside a face, where the surface curves in both directions.
fn interior_steps(s: &Surface, q: &Quality) -> Option<(f64, f64)> {
    match s {
        Surface::Plane(_) => None,
        // straight along v: columns across u, rows only as far apart as keeps triangles shapely
        // (rows four columns apart let refinement near a boundary run on, splitting edges
        // into ones as long, and leave triangles across two columns: 3.5 chords deep)
        Surface::Cylinder(c) => Some((arc_step(c.radius, q), 2.0 * c.radius * arc_step(c.radius, q))),
        Surface::Cone(c) => {
            let r = c.radius.max(1.0);
            Some((arc_step(r, q), 2.0 * r * arc_step(r, q)))
        }
        // curved both ways: half the tolerance each way (see edge_segments), and a little
        // under, so a cell's diagonal does not sag by just about the whole tolerance and get
        // split by the refinement
        Surface::Sphere(sp) => {
            let a = 0.9 * arc_step(sp.radius, q) / core::f64::consts::SQRT_2;
            Some((a, a))
        }
        Surface::Torus(t) => Some((
            0.9 * arc_step(t.major + t.minor, q) / core::f64::consts::SQRT_2,
            0.9 * arc_step(t.minor, q) / core::f64::consts::SQRT_2,
        )),
        Surface::Procedural(pr) => {
            // arc steps on the tightest radius, turned into parameter steps by the uv scale
            let (r, (su, sv)) = (pr.min_radius(), pr.uv_scale());
            let l = arc_step(r, q) * r;
            Some((l / su, l / sv))
        }
    }
}

/// Bounds on the normal curvature of chords running mostly along u and mostly along v at a
/// regular point (zero at a singular one): each line's own, plus the twist between them.
fn line_curvatures(d: &forge_geom::surface::SurfDeriv) -> (f64, f64) {
    let Some(n) = d.du.cross(d.dv).normalize() else {
        return (0.0, 0.0);
    };
    let n = n.get();
    let (e, g) = (d.du.dot(d.du), d.dv.dot(d.dv));
    if e <= 0.0 || g <= 0.0 {
        return (0.0, 0.0);
    }
    let twist = (d.duv.dot(n) / (e * g).sqrt()).abs();
    ((d.duu.dot(n) / e).abs() + twist, (d.dvv.dot(n) / g).abs() + twist)
}

/// Grid points inside a procedural face's parameter box, each cell of an 8 x 8 split of the
/// box as dense as the tightest curvature at the nodes round it needs (never denser than the
/// face's tightest radius anywhere): a corner patch curves tightly in a corner and gently
/// elsewhere. Distances on the surface turn into parameter steps by the face's uv scale.
fn adaptive_grid(
    s: &Surface,
    q: &Quality,
    lo: Point2,
    hi: Point2,
    (su, sv): (f64, f64),
    out: &mut Vec<Point2>,
) {
    const G: usize = 8;
    let at = |i: usize, j: usize| {
        Point2::new(
            lo.x + (hi.x - lo.x) * i as f64 / G as f64,
            lo.y + (hi.y - lo.y) * j as f64 / G as f64,
        )
    };
    // per node: the tightest radius along u and along v (a blend bends across, hardly along)
    let mut node = [[(f64::INFINITY, f64::INFINITY); G + 1]; G + 1];
    for (i, row) in node.iter_mut().enumerate() {
        for (j, n) in row.iter_mut().enumerate() {
            let (ku, kv) = line_curvatures(&s.deriv(at(i, j)));
            *n = (1.0 / ku, 1.0 / kv);
        }
    }
    let r0 = match s {
        Surface::Procedural(pr) => pr.min_radius(),
        _ => f64::INFINITY,
    };
    let step = |r: f64| {
        let r = r.max(r0);
        if r.is_finite() {
            arc_step(r, q) * r
        } else {
            f64::INFINITY
        }
    };
    for ci in 0..G {
        for cj in 0..G {
            // the nodes of this cell and its neighbours
            let (mut ru, mut rv) = (f64::INFINITY, f64::INFINITY);
            for row in &node[ci.saturating_sub(1)..(ci + 3).min(G + 1)] {
                for &n in &row[cj.saturating_sub(1)..(cj + 3).min(G + 1)] {
                    ru = ru.min(n.0);
                    rv = rv.min(n.1);
                }
            }
            let (lu, lv) = (step(ru), step(rv));
            let (a, b) = (at(ci, cj), at(ci + 1, cj + 1));
            let nu = ((b.x - a.x) * su / lu).ceil().max(1.0) as usize;
            let nv = ((b.y - a.y) * sv / lv).ceil().max(1.0) as usize;
            // each cell owns its low sides; the box's own low sides are boundary
            for i in usize::from(ci == 0)..nu {
                for j in usize::from(cj == 0)..nv {
                    let u = a.x + (b.x - a.x) * i as f64 / nu as f64;
                    let v = a.y + (b.y - a.y) * j as f64 / nv as f64;
                    out.push(Point2::new(u * su, v * sv));
                }
            }
        }
    }
}

/// Mesh vertices joined along edges: `nbr[start[v]..start[v + 1]]` are v's neighbours, and
/// `edges[on[v]..on[v + 1]]` the edges v lies on.
struct Segments {
    start: Vec<u32>,
    nbr: Vec<u32>,
    on: Vec<u32>,
    edges: Vec<u32>,
}

/// A watertight mesh of a body: each edge is cut once and both faces share
/// those cut points, so neighbouring faces meet exactly.
pub fn tessellate(body: &Body, q: &Quality) -> Result<TriMesh, TessError> {
    if body.dead > 0 {
        return tessellate(&body.compact(), q);
    }
    let mut mesh = TriMesh::default();
    let vertex_index: Vec<u32> = body.vertices.iter().map(|v| mesh.add_vertex(v.point)).collect();

    // per edge: its parameter samples and their mesh indices (ends are the body's vertices)
    let mut edge_samples: Vec<Vec<(f64, u32)>> = Vec::with_capacity(body.edges.len());
    let (mut trig, mut pts) = (Vec::new(), Vec::new());
    for e in body.edge_ids() {
        let edge = body.edge(e);
        let fine = edge
            .coedges
            .iter()
            .any(|&c| doubly_curved(&body.face(body.coedge_face(c)).surface));
        let n = edge_segments(&edge.curve, edge.range, q, fine);
        let mut s = Vec::with_capacity(n + 1);
        s.push((edge.range.lo, vertex_index[edge.start.index()]));
        edge.curve.sample_interior(edge.range, n, &mut trig, &mut pts);
        for (i, &p) in pts.iter().enumerate() {
            let t = edge.range.lo + edge.range.width() * ((i + 1) as f64 / n as f64);
            s.push((t, mesh.add_vertex(p)));
        }
        s.push((edge.range.hi, vertex_index[edge.end.index()]));
        edge_samples.push(s);
    }

    // every mesh segment an edge's own samples make, as each sample's neighbours along its edges
    // (start offsets, then neighbours): a face's inner chord must never repeat one (four
    // triangles on one segment), as when a coarse chord across a cylinder lands exactly on an
    // edge between two other faces
    let nv = mesh.positions.len();
    let mut start = vec![0u32; nv + 1];
    for s in &edge_samples {
        for w in s.windows(2) {
            start[w[0].1 as usize + 1] += 1;
            start[w[1].1 as usize + 1] += 1;
        }
    }
    for i in 0..nv {
        start[i + 1] += start[i];
    }
    let mut fill = start.clone();
    let mut nbr = vec![0u32; start[nv] as usize];
    for s in &edge_samples {
        for w in s.windows(2) {
            let (a, b) = (w[0].1 as usize, w[1].1 as usize);
            nbr[fill[a] as usize] = b as u32;
            fill[a] += 1;
            nbr[fill[b] as usize] = a as u32;
            fill[b] += 1;
        }
    }
    let mut on = vec![0u32; nv + 1];
    for s in &edge_samples {
        for &(_, v) in s {
            on[v as usize + 1] += 1;
        }
    }
    for i in 0..nv {
        on[i + 1] += on[i];
    }
    let mut fill = on.clone();
    let mut edges = vec![0u32; on[nv] as usize];
    for (e, s) in edge_samples.iter().enumerate() {
        for &(_, v) in s {
            edges[fill[v as usize] as usize] = e as u32;
            fill[v as usize] += 1;
        }
    }
    let segments = Segments {
        start,
        nbr,
        on,
        edges,
    };
    for f in body.face_ids() {
        tessellate_face(body, f, &edge_samples, &segments, q, &mut mesh)
            .map_err(|error| TessError::Face { face: f, error })?;
    }
    Ok(mesh)
}

fn tessellate_face(
    body: &Body,
    f: FaceId,
    edge_samples: &[Vec<(f64, u32)>],
    segments: &Segments,
    q: &Quality,
    mesh: &mut TriMesh,
) -> Result<(), CdtError> {
    let face = body.face(f);
    let surface = &face.surface;
    let (su, sv) = uv_scale(surface);

    let mut uv: Vec<Point2> = Vec::new();
    let mut global: Vec<u32> = Vec::new();
    let mut rings: Vec<Vec<Point2>> = Vec::new();
    let mut ring_globals: Vec<Vec<u32>> = Vec::new();
    for &l in &face.loops {
        let mut samples = Vec::new();
        let mut ids = Vec::new();
        for &c in &body.lp(l).coedges {
            let co = body.coedge(c);
            let s = &edge_samples[co.edge.index()];
            // the edge's samples are already mesh vertices: their points, not evaluated again
            let mut take = |&(_, gi): &(f64, u32)| {
                samples.push(mesh.positions[gi as usize]);
                ids.push(gi);
            };
            if co.forward {
                s[..s.len() - 1].iter().for_each(&mut take);
            } else {
                s[1..].iter().rev().for_each(&mut take);
            }
        }
        let (pts, sample_of) = draw_loop(body, f, &samples);
        ring_globals.push(sample_of.iter().map(|&k| ids[k]).collect());
        rings.push(pts);
    }
    align_loops(body, f, rings.iter_mut());
    let mut loops: Vec<Vec<u32>> = Vec::new();
    for (pts, ids) in rings.into_iter().zip(ring_globals) {
        loops.push((uv.len() as u32..(uv.len() + pts.len()) as u32).collect());
        uv.extend(pts);
        global.extend(ids);
    }

    if loops.len() == 1
        && let Some(tris) = strips(surface, &uv, &global, q, mesh)
    {
        for [a, b, c] in tris {
            if a == b || b == c || c == a {
                continue;
            }
            mesh.triangles
                .push(if face.same_sense { [a, b, c] } else { [a, c, b] });
            mesh.face_of.push(f.0);
        }
        return Ok(());
    }

    let mut scaled: Vec<Point2> = uv.iter().map(|p| Point2::new(p.x * su, p.y * sv)).collect();
    let mut interior = Vec::new();
    // an analytic face's grid: cells clear of the boundary are triangulated straight off the
    // grid, the band between them and the boundary by the constrained Delaunay
    let mut grid: Option<Grid> = None;
    if let Some((du, dv)) = interior_steps(surface, q) {
        let (mut lo, mut hi) = (
            Point2::new(f64::INFINITY, f64::INFINITY),
            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for p in &uv {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        match surface {
            Surface::Procedural(_) => adaptive_grid(surface, q, lo, hi, (su, sv), &mut interior),
            _ => {
                let nu = ((hi.x - lo.x) / du).ceil().max(1.0) as usize;
                let nv = ((hi.y - lo.y) / dv).ceil().max(1.0) as usize;
                let mut g = Grid::new(lo, hi, nu, nv, &uv, &loops);
                // a cell's chords sag within the tolerance only where its steps come from the
                // surface's exact radii (not a cone's, whose radius grows along it)
                let exact = matches!(
                    surface,
                    Surface::Sphere(_) | Surface::Torus(_) | Surface::Cylinder(_)
                );
                let stairs = if g.any && exact { g.staircase() } else { None };
                g.settle(stairs.is_some());
                for i in 1..nu {
                    for j in 1..nv {
                        if !g.covered(i, j) {
                            let p = g.node(i, j);
                            interior.push(Point2::new(p.x * su, p.y * sv));
                        }
                    }
                }
                if let Some(stairs) = stairs {
                    for cycle in stairs {
                        loops.push((scaled.len() as u32..(scaled.len() + cycle.len()) as u32).collect());
                        for &(i, j) in &cycle {
                            let p = g.node(i, j);
                            scaled.push(Point2::new(p.x * su, p.y * sv));
                        }
                        g.cycles.borrow_mut().push(cycle);
                    }
                    grid = Some(g);
                }
            }
        }
    }

    // chords that stray from the surface by more than the tolerance get a surface point at
    // their middle, inserted into the same triangulation round after round (a thin band between
    // two boundary curves has no grid points at all; a chord across a radian of a cylinder needs
    // several halvings)
    let at = |p: Point2| surface.eval(Point2::new(p.x / su, p.y / sv));
    // each point's position is worked out once, however many edges and rounds test it (the
    // boundary's and the grid's are known already)
    let mut node_index: Vec<u32> = Vec::new();
    let mut known: Vec<Option<forge_math::Point3>> =
        global.iter().map(|&g| Some(mesh.positions[g as usize])).collect();
    if let Some(g) = &grid {
        node_index = g.place(surface, mesh);
        for cycle in g.cycles.borrow().iter() {
            known.extend(
                cycle
                    .iter()
                    .map(|&(i, j)| Some(mesh.positions[node_index[g.id(i, j)] as usize])),
            );
        }
    }
    let cache = std::cell::RefCell::new(known);
    let pos = |i: u32, p: Point2| {
        let mut c = cache.borrow_mut();
        if c.len() <= i as usize {
            c.resize(i as usize + 1, None);
        }
        *c[i as usize].get_or_insert_with(|| at(p))
    };
    let on_edge = |ia: u32, ib: u32| {
        let n = global.len() as u32;
        ia < n && ib < n && {
            let (x, y) = (global[ia as usize] as usize, global[ib as usize] as usize);
            x < segments.start.len() - 1
                && segments.nbr[segments.start[x] as usize..segments.start[x + 1] as usize]
                    .contains(&(y as u32))
        }
    };
    // a chord between two points of one edge cuts a flap off along it, and the face across the
    // edge, as nearly tangent as a run-out meeting a wall, can cut the very same flap: the two
    // faces would share a triangle
    let one_edge = |ia: u32, ib: u32| {
        let n = global.len() as u32;
        ia < n && ib < n && {
            let of = |i: u32| match global[i as usize] as usize {
                x if x + 1 < segments.on.len() => {
                    &segments.edges[segments.on[x] as usize..segments.on[x + 1] as usize]
                }
                _ => &[],
            };
            let ob = of(ib);
            of(ia).iter().any(|e| ob.contains(e))
        }
    };
    // a chord of a plane never strays from it
    let flat = matches!(surface, Surface::Plane(_));
    // on a sphere, torus or cylinder the grid's steps come from its exact radii, so a chord
    // between two inner points within one grid cell's span each way sags less than the
    // tolerance and needs no look
    let nb = global.len() as u32;
    let cell = match (surface, interior_steps(surface, q)) {
        (Surface::Sphere(_) | Surface::Torus(_) | Surface::Cylinder(_), Some((du, dv))) => {
            (du * su * (1.0 + 1e-9), dv * sv * (1.0 + 1e-9))
        }
        _ => (0.0, 0.0),
    };
    let strays = |ia: u32, ib: u32, a: Point2, b: Point2| {
        if ia >= nb && ib >= nb && (a.x - b.x).abs() <= cell.0 && (a.y - b.y).abs() <= cell.1 {
            return false;
        }
        on_edge(ia, ib)
            || (!flat
                && (one_edge(ia, ib)
                    || at(a.lerp(b, 0.5)).distance(pos(ia, a).lerp(pos(ib, b), 0.5)) > q.chord))
    };
    let tri = match triangulate_refined(&scaled, &loops, &interior, Some(&strays), 24) {
        Ok(t) => t,
        Err(e) => {
            if let Some(path) = std::env::var_os("FORGE_CDT_DUMP") {
                let pts: Vec<String> = scaled.iter().map(|p| format!("{:e},{:e}", p.x, p.y)).collect();
                let ls: Vec<String> = loops.iter().map(|l| format!("{}+{}", l[0], l.len())).collect();
                let ins: Vec<String> = interior.iter().map(|p| format!("{:e},{:e}", p.x, p.y)).collect();
                let text = format!("{}\n{}\n{}\n", pts.join(" "), ls.join(" "), ins.join(" "));
                let _ = std::fs::write(path, text);
            }
            return Err(e);
        }
    };
    let mut index = global;
    let known = cache.into_inner();
    if let Some(g) = &grid {
        for cycle in g.cycles.borrow().iter() {
            index.extend(cycle.iter().map(|&(i, j)| node_index[g.id(i, j)]));
        }
    }
    for (i, p) in tri.points.iter().enumerate().skip(index.len()) {
        let pos = known.get(i).copied().flatten().unwrap_or_else(|| at(*p));
        index.push(mesh.add_vertex(pos));
    }
    if let Some(g) = &grid {
        for (i, j) in g.inner_cells() {
            let [a, b, c, d] =
                [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)].map(|(x, y)| node_index[g.id(x, y)]);
            for t in [[a, b, c], [a, c, d]] {
                let t = if face.same_sense { t } else { [t[0], t[2], t[1]] };
                mesh.triangles.push(t);
                mesh.face_of.push(f.0);
            }
        }
    }

    for t in &tri.triangles {
        let [a, b, c] = t.map(|k| index[k as usize]);
        // two corners on one pole: the triangle collapses to the pole and is dropped
        if a == b || b == c || c == a {
            continue;
        }
        // counter-clockwise in (u, v) faces along du x dv, the surface's natural normal
        let t = if face.same_sense { [a, b, c] } else { [a, c, b] };
        mesh.triangles.push(t);
        mesh.face_of.push(f.0);
    }
    Ok(())
}

/// A face's interior grid in (u, v): `nu` x `nv` cells over the face's parameter box, and the
/// cells clear of its boundary (`inner`): no boundary segment's box meets the cell or any cell
/// round it, and the cell's middle is inside the face.
struct Grid {
    lo: Point2,
    hi: Point2,
    nu: usize,
    nv: usize,
    inner: Vec<bool>,
    any: bool,
    /// Per node: a corner of an inner cell.
    cover: Vec<bool>,
    /// The staircase cycles round the inner cells, once traced.
    cycles: std::cell::RefCell<Vec<Vec<(usize, usize)>>>,
}

impl Grid {
    fn new(lo: Point2, hi: Point2, nu: usize, nv: usize, uv: &[Point2], loops: &[Vec<u32>]) -> Grid {
        let (cw, ch) = ((hi.x - lo.x) / nu as f64, (hi.y - lo.y) / nv as f64);
        let mut touched = vec![false; nu * nv];
        // the cells a segment's box comes within a quarter cell of: no grid node of an untouched
        // cell lies on or right by the boundary
        let cell_of = |x: f64, w: f64, l: f64, n: usize| (((x - l) / w).floor().max(0.0) as usize).min(n - 1);
        let reach = 0.25;
        let segs = || {
            loops.iter().flat_map(|l| {
                (0..l.len()).map(move |k| (uv[l[k] as usize], uv[l[(k + 1) % l.len()] as usize]))
            })
        };
        for (a, b) in segs() {
            let (i0, i1) = (
                cell_of(a.x.min(b.x) - reach * cw, cw, lo.x, nu),
                cell_of(a.x.max(b.x) + reach * cw, cw, lo.x, nu),
            );
            let (j0, j1) = (
                cell_of(a.y.min(b.y) - reach * ch, ch, lo.y, nv),
                cell_of(a.y.max(b.y) + reach * ch, ch, lo.y, nv),
            );
            for i in i0..=i1 {
                for j in j0..=j1 {
                    touched[i * nv + j] = true;
                }
            }
        }
        // inside by the parity of crossings along each row's middle: each segment adds its
        // crossings to the rows it spans
        let mut inside = vec![false; nu * nv];
        let mut rows: Vec<Vec<f64>> = vec![Vec::new(); nv];
        let row_at = |y: f64| (y - lo.y) / ch - 0.5;
        for (a, b) in segs() {
            let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
            let j0 = row_at(y0).floor().max(0.0) as usize;
            let j1 = (row_at(y1).ceil().max(0.0) as usize).min(nv - 1);
            for (j, xs) in rows.iter_mut().enumerate().take(j1 + 1).skip(j0) {
                let v = lo.y + ch * (j as f64 + 0.5);
                if (a.y > v) != (b.y > v) {
                    xs.push(a.x + (v - a.y) / (b.y - a.y) * (b.x - a.x));
                }
            }
        }
        for (j, xs) in rows.iter_mut().enumerate() {
            xs.sort_by(f64::total_cmp);
            let mut k = 0;
            for i in 0..nu {
                let u = lo.x + cw * (i as f64 + 0.5);
                while k < xs.len() && xs[k] < u {
                    k += 1;
                }
                inside[i * nv + j] = k % 2 == 1;
            }
        }
        let clear = |i: usize, j: usize| !touched[i * nv + j];
        let mut inner: Vec<bool> = (0..nu * nv).map(|c| inside[c] && clear(c / nv, c % nv)).collect();
        // no two inner cells meeting at a corner only (their staircases would touch there)
        loop {
            let mut changed = false;
            for i in 0..nu.saturating_sub(1) {
                for j in 0..nv.saturating_sub(1) {
                    let at = |x: usize, y: usize| inner[x * nv + y];
                    let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i, j + 1), at(i + 1, j + 1));
                    if (a && d && !b && !c) || (b && c && !a && !d) {
                        inner[(i + 1) * nv + j + 1] = false;
                        inner[(i + 1) * nv + j] = false;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let any = inner.contains(&true);
        Grid {
            lo,
            hi,
            nu,
            nv,
            inner,
            any,
            cover: Vec::new(),
            cycles: Default::default(),
        }
    }

    fn node(&self, i: usize, j: usize) -> Point2 {
        Point2::new(
            self.lo.x + (self.hi.x - self.lo.x) * i as f64 / self.nu as f64,
            self.lo.y + (self.hi.y - self.lo.y) * j as f64 / self.nv as f64,
        )
    }

    fn id(&self, i: usize, j: usize) -> usize {
        i * (self.nv + 1) + j
    }

    fn is_inner(&self, i: usize, j: usize) -> bool {
        i < self.nu && j < self.nv && self.inner[i * self.nv + j]
    }

    /// Whether a node is a corner of an inner cell.
    fn covered(&self, i: usize, j: usize) -> bool {
        self.any && self.cover[self.id(i, j)]
    }

    /// Settles which nodes the inner cells use (none unless `any`).
    fn settle(&mut self, any: bool) {
        self.any = any;
        self.cover = vec![false; (self.nu + 1) * (self.nv + 1)];
        for c in 0..self.nu * self.nv {
            if self.inner[c] {
                let (i, j) = (c / self.nv, c % self.nv);
                for (x, y) in [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)] {
                    self.cover[x * (self.nv + 1) + y] = true;
                }
            }
        }
    }

    fn inner_cells(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        (0..self.nu * self.nv)
            .filter(|&c| self.inner[c])
            .map(|c| (c / self.nv, c % self.nv))
    }

    /// The boundary of the inner cells as cycles of nodes (each node on one at most once); None
    /// if a node would start two sides (cannot happen once corner-only contacts are gone).
    fn staircase(&self) -> Option<Vec<Vec<(usize, usize)>>> {
        // each inner cell's sides against a cell that is not: from node to node, cell on the left
        let mut next = vec![usize::MAX; (self.nu + 1) * (self.nv + 1)];
        let mut twice = false;
        let mut side = |from: usize, to: usize| {
            twice |= next[from] != usize::MAX;
            next[from] = to;
        };
        for (i, j) in self.inner_cells() {
            let (i1, j1) = (i + 1, j + 1);
            if j == 0 || !self.is_inner(i, j - 1) {
                side(self.id(i, j), self.id(i1, j));
            }
            if !self.is_inner(i1, j) {
                side(self.id(i1, j), self.id(i1, j1));
            }
            if !self.is_inner(i, j1) {
                side(self.id(i1, j1), self.id(i, j1));
            }
            if i == 0 || !self.is_inner(i - 1, j) {
                side(self.id(i, j1), self.id(i, j));
            }
        }
        if twice {
            return None;
        }
        let mut out = Vec::new();
        for start in 0..next.len() {
            if next[start] == usize::MAX {
                continue;
            }
            let mut cycle = Vec::new();
            let mut n = start;
            while next[n] != usize::MAX {
                cycle.push((n / (self.nv + 1), n % (self.nv + 1)));
                let m = next[n];
                next[n] = usize::MAX;
                n = m;
            }
            if n != start {
                return None;
            }
            out.push(cycle);
        }
        Some(out)
    }

    /// Mesh vertices for every node an inner cell or the staircase uses, row by row.
    fn place(&self, surface: &Surface, mesh: &mut TriMesh) -> Vec<u32> {
        let mut index = vec![u32::MAX; (self.nu + 1) * (self.nv + 1)];
        let (mut trig, mut pts) = (Vec::new(), Vec::new());
        for j in 1..self.nv {
            if !(1..self.nu).any(|i| self.cover[self.id(i, j)]) {
                continue;
            }
            let v = self.node(0, j).y;
            surface.eval_row(v, self.lo.x, self.hi.x, self.nu, &mut trig, &mut pts);
            for i in 1..self.nu {
                if self.covered(i, j) {
                    index[self.id(i, j)] = mesh.add_vertex(pts[i - 1]);
                }
            }
        }
        index
    }
}

/// Faces that are rectangles in parameter space (every revolve face, every
/// extrude wall) need no Delaunay: rows of points across u at each v sample
/// of the side edges, stitched strip by strip. Counter-clockwise in (u, v).
/// None when the face is not such a rectangle, or its two sides are sampled
/// at different v (stitching rows then would leave T-junctions).
fn strips(
    surface: &Surface,
    uv: &[Point2],
    global: &[u32],
    q: &Quality,
    mesh: &mut TriMesh,
) -> Option<Vec<[u32; 3]>> {
    let n = uv.len();
    let (mut u0, mut u1, mut v0, mut v1) =
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for p in uv {
        u0 = u0.min(p.x);
        u1 = u1.max(p.x);
        v0 = v0.min(p.y);
        v1 = v1.max(p.y);
    }
    let eps = 1e-9 * (u1 - u0).max(v1 - v0).max(1.0);
    let near = |a: f64, b: f64| (a - b).abs() <= eps;
    let start = (0..n).find(|&i| near(uv[i].x, u0) && near(uv[i].y, v0))?;
    // walk counter-clockwise from the (u0, v0) corner
    let ccw = forge_topo::uv::signed_area(uv) > 0.0;
    let at = |k: usize| if ccw { (start + k) % n } else { (start + n - k) % n };
    let mut sides: [Vec<usize>; 4] = Default::default();
    let mut side = 0;
    sides[0].push(at(0));
    for k in 1..=n {
        let i = at(k);
        let p = uv[i];
        let on = |s: usize| match s {
            0 => near(p.y, v0),
            1 => near(p.x, u1),
            2 => near(p.y, v1),
            _ => near(p.x, u0),
        };
        if !on(side) {
            return None;
        }
        sides[side].push(i);
        let corner = match side {
            0 => near(p.x, u1),
            1 => near(p.y, v1),
            2 => near(p.x, u0),
            _ => k == n,
        };
        if corner && side < 3 {
            side += 1;
            sides[side].push(i);
        }
    }
    if side != 3 {
        return None;
    }
    let [bottom, right, mut top, mut left] = sides;
    top.reverse();
    left.reverse();
    if left.len() != right.len() || left.iter().zip(&right).any(|(&a, &b)| !near(uv[a].y, uv[b].y)) {
        return None;
    }
    let row_count = |v: f64| -> usize {
        // length per unit u along the row, and the radius that sets the arc step
        let (len, r) = match surface {
            Surface::Plane(_) => return 1,
            Surface::Cylinder(c) => (c.radius, c.radius),
            Surface::Cone(c) => (c.radius_at(v), c.radius_at(v)),
            Surface::Sphere(sp) => (sp.radius * dmath::cos(v), sp.radius * dmath::cos(v)),
            Surface::Torus(t) => (
                t.major + t.minor * dmath::cos(v),
                t.major + t.minor * dmath::cos(v),
            ),
            Surface::Procedural(pr) => (pr.uv_scale().0, pr.min_radius()),
        };
        if r <= forge_math::tol::LINEAR {
            return 1;
        }
        let step = if doubly_curved(surface) {
            arc_step(r, q) / core::f64::consts::SQRT_2
        } else {
            arc_step(r, q)
        };
        ((u1 - u0) * len / (r * step)).ceil().max(1.0) as usize
    };
    // each row: (u, mesh index), increasing u
    let mut rows: Vec<Vec<(f64, u32)>> = Vec::with_capacity(left.len());
    let (mut trig, mut pts) = (Vec::new(), Vec::new());
    rows.push(bottom.iter().map(|&i| (uv[i].x, global[i])).collect());
    for j in 1..left.len() - 1 {
        let v = uv[left[j]].y;
        let m = row_count(v);
        let mut row = vec![(u0, global[left[j]])];
        surface.eval_row(v, u0, u1, m, &mut trig, &mut pts);
        for (k, &p) in pts.iter().enumerate() {
            row.push((u0 + (u1 - u0) * ((k + 1) as f64 / m as f64), mesh.add_vertex(p)));
        }
        row.push((u1, global[right[j]]));
        rows.push(row);
    }
    rows.push(top.iter().map(|&i| (uv[i].x, global[i])).collect());
    let mut tris = Vec::new();
    for w in rows.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let (mut i, mut k) = (0, 0);
        while i + 1 < a.len() || k + 1 < b.len() {
            if k + 1 >= b.len() || (i + 1 < a.len() && a[i + 1].0 <= b[k + 1].0) {
                tris.push([a[i].1, a[i + 1].1, b[k].1]);
                i += 1;
            } else {
                tris.push([a[i].1, b[k + 1].1, b[k].1]);
                k += 1;
            }
        }
    }
    Some(tris)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Point3, Vec3};
    use forge_ops::{Profile, extrude};

    fn closed(m: &TriMesh) {
        let open = m.open_edges();
        assert!(
            open.is_empty(),
            "{} open edges, e.g. {:?}",
            open.len(),
            &open[..open.len().min(5)]
        );
        assert_eq!(m.degenerate_triangles(1e-12), 0);
    }

    #[test]
    fn box_mesh_is_exact() {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)],
        };
        let b = extrude(&p, 5.0, 1).unwrap();
        let m = tessellate(&b, &Quality::default()).unwrap();
        closed(&m);
        assert_eq!(m.triangles.len(), 12);
        assert!((m.volume() - 1000.0).abs() < 1e-9, "{}", m.volume());
    }

    #[test]
    fn curved_parts_are_closed_and_close() {
        let q = Quality::default();
        let mut r = Rng::new(600);
        for _ in 0..60 {
            let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
                .normalize()
                .unwrap();
            let plane = Frame::from_z(Point3::new(r.range(-50.0, 50.0), r.range(-50.0, 50.0), 0.0), z);
            let (w, h) = (r.range(10.0, 60.0), r.range(10.0, 60.0));
            let rad = r.range(0.5, w.min(h) * 0.45);
            let hole = w.min(h) * 0.2;
            let p = Profile {
                plane,
                loops: vec![
                    Profile::rounded_rectangle(Point2::ORIGIN, w, h, rad, 0),
                    Profile::circle(Point2::new(w / 2.0, h / 2.0), hole, 50),
                ],
            };
            let d = r.range(1.0, 20.0) * if r.bool() { -1.0 } else { 1.0 };
            let b = extrude(&p, d, 1).unwrap();
            let m = tessellate(&b, &q).unwrap();
            closed(&m);
            let exact = p.normalized().unwrap().area() * d.abs();
            // inscribed chords lose at most chord x perimeter of area per unit depth
            let perimeter = 2.0 * (w + h) + dmath::TAU * hole;
            let err = (m.volume() - exact).abs();
            assert!(err <= q.chord * perimeter * d.abs(), "{} vs {exact}", m.volume());
        }
    }

    #[test]
    fn chords_stay_within_tolerance() {
        let q = Quality {
            chord: 0.005,
            max_angle: 0.5,
        };
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![
                Profile::circle(Point2::ORIGIN, 25.0, 0),
                Profile::circle(Point2::ORIGIN, 10.0, 1),
            ],
        };
        let b = extrude(&p, 4.0, 1).unwrap();
        let m = tessellate(&b, &q).unwrap();
        closed(&m);
        // every triangle centroid lies within the chord tolerance of its face
        for (t, &f) in m.triangles.iter().zip(&m.face_of) {
            let c = t
                .iter()
                .fold(Vec3::ZERO, |a, &i| a + m.positions[i as usize].to_vec())
                / 3.0;
            let d = b.face(FaceId(f)).surface.distance(Point3::new(c.x, c.y, c.z));
            assert!(d <= q.chord, "{d}");
        }
    }
}

#[cfg(test)]
mod revolve_tests {
    use super::*;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Vec2};
    use forge_ops::{Axis2, Profile, ProfileLoop, Segment, revolve};

    const Y: Axis2 = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };

    fn mesh_volume(p: &Profile, angle: f64) -> f64 {
        let b = revolve(p, Y, angle, 1).unwrap();
        let m = tessellate(
            &b,
            &Quality {
                chord: 0.002,
                max_angle: 0.1,
            },
        )
        .unwrap();
        let open = m.open_edges();
        assert!(open.is_empty(), "{} open edges", open.len());
        m.volume()
    }

    fn rel(a: f64, b: f64) -> f64 {
        (a - b).abs() / b.abs()
    }

    #[test]
    fn exact_volumes() {
        let q = dmath::FRAC_PI_2;
        let r = 5.0;
        let half = ProfileLoop::new(vec![
            (
                Segment::Arc {
                    center: Point2::ORIGIN,
                    radius: r,
                    start: -q,
                    sweep: 2.0 * q,
                },
                0,
            ),
            (
                Segment::Line {
                    a: Point2::new(0.0, r),
                    b: Point2::new(0.0, -r),
                },
                1,
            ),
        ]);
        let sphere = Profile {
            plane: Frame::WORLD,
            loops: vec![half],
        };
        let v = mesh_volume(&sphere, dmath::TAU);
        assert!(rel(v, 4.0 / 3.0 * dmath::PI * r * r * r) < 2e-3, "sphere {v}");

        let torus = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::circle(Point2::new(10.0, 0.0), 3.0, 0)],
        };
        let v = mesh_volume(&torus, dmath::TAU);
        assert!(
            rel(v, 2.0 * dmath::PI * dmath::PI * 10.0 * 9.0) < 2e-3,
            "torus {v}"
        );

        let tri = ProfileLoop::new(vec![
            (
                Segment::Line {
                    a: Point2::ORIGIN,
                    b: Point2::new(4.0, 0.0),
                },
                0,
            ),
            (
                Segment::Line {
                    a: Point2::new(4.0, 0.0),
                    b: Point2::new(0.0, 7.0),
                },
                1,
            ),
            (
                Segment::Line {
                    a: Point2::new(0.0, 7.0),
                    b: Point2::ORIGIN,
                },
                2,
            ),
        ]);
        let cone = Profile {
            plane: Frame::WORLD,
            loops: vec![tri],
        };
        let v = mesh_volume(&cone, dmath::TAU);
        assert!(rel(v, dmath::PI * 16.0 * 7.0 / 3.0) < 2e-3, "cone {v}");
    }

    #[test]
    fn pappus_on_random_profiles() {
        let mut rng = Rng::new(800);
        for _ in 0..25 {
            let x0 = rng.range(0.5, 15.0);
            let (w, h) = (rng.range(3.0, 15.0), rng.range(3.0, 15.0));
            let rad = rng.range(0.3, w.min(h) * 0.45);
            let mut loops = vec![Profile::rounded_rectangle(Point2::new(x0, 0.0), w, h, rad, 0)];
            if rng.bool() {
                loops.push(Profile::circle(
                    Point2::new(x0 + w / 2.0, h / 2.0),
                    w.min(h) * 0.2,
                    40,
                ));
            }
            let p = Profile {
                plane: Frame::WORLD,
                loops,
            };
            let angle = if rng.bool() {
                dmath::TAU
            } else {
                rng.range(0.3, 6.0)
            };
            // symmetric profile: the centroid is the rectangle's centre
            let want = angle * (x0 + w / 2.0) * p.normalized().unwrap().area();
            let v = mesh_volume(&p, angle);
            assert!(rel(v, want) < 3e-3, "{v} vs {want}");
        }
    }
}
