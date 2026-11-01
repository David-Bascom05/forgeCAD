use crate::rebuild::{RebuildError, rebuild_some, solve_point};
use crate::selfx::crossing;
use forge_geom::intersect::{CurveSurface, curve_surface};
use forge_geom::{Cone, Curve, CurveGeom, Cylinder, Line, Plane, Sphere, Surface};
use forge_math::{Frame, Interval, Point3, UnitVec3, Vec3, dmath, tol};
use forge_topo::{Body, CoedgeId, EdgeId, FaceId, Name, Problem, VertexId, check_against};
use std::collections::HashMap;

/// How far a chamfer cuts back from its edges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChamferSize {
    /// The same distance back along both faces.
    Equal(f64),
    /// `d1` back along `face`, `d2` along the other face of each edge.
    Distances { face: Name, d1: f64, d2: f64 },
    /// `d` back along `face`, the cut leaving it at `angle`.
    DistanceAngle { face: Name, d: f64, angle: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChamferError {
    NoSuchEdge {
        edge: Name,
    },
    /// A distance is no more than the tolerance, or the angle leaves no triangle to cut.
    BadSize,
    /// The reference face of a two-sided size does not bound this edge.
    NotOnFace {
        edge: Name,
    },
    /// The faces at this edge meet flat (tangent) or fold back on each other.
    Tangent {
        edge: Name,
    },
    /// The chamfer runs past the faces it cuts: this edge would vanish or turn inside out.
    TooLarge {
        edge: Name,
    },
    /// The chamfer runs into another part of the face it cuts, or through another face.
    Overflows {
        face: Name,
    },
    Unsupported {
        edge: Name,
        what: &'static str,
    },
    Rebuild(RebuildError),
    Invalid(Vec<Problem>),
}

/// One chamfered edge.
struct Bevel {
    edge: EdgeId,
    a: FaceId,
    b: FaceId,
    /// A's loop walks the edge from start to end.
    fa: bool,
    surface: Surface,
    same_sense: bool,
    name: Name,
    /// Where the cut meets A and B at the edge's start and end.
    at_start: [Point3; 2],
    at_end: [Point3; 2],
}

/// An end of an edge at a vertex: the edge, and whether it is the end at the curve's start.
type End = (EdgeId, bool);
type Path = Vec<(EdgeId, bool)>;

/// Bevels `edges` all at once: each becomes a face on an exact surface (a plane between two
/// lines, a cone or plane between two coaxial circles), the faces beside it are trimmed back,
/// and where chamfers meet they mitre (two) or get a flat corner patch (three).
pub fn chamfer(body: &Body, edges: &[Name], size: ChamferSize, feature: u64) -> Result<Body, ChamferError> {
    let src = body.compact();
    let root = Name::feature(feature);
    let mut bevels: Vec<Bevel> = Vec::new();
    let mut bevel_of: HashMap<EdgeId, usize> = HashMap::new();
    for &name in edges {
        let e = src
            .edge_by_name(name)
            .ok_or(ChamferError::NoSuchEdge { edge: name })?;
        if bevel_of.contains_key(&e) {
            continue;
        }
        bevel_of.insert(e, bevels.len());
        bevels.push(bevel(&src, e, size, root)?);
    }

    // corners: per vertex, (face, coedge arriving, coedge leaving) for each loop passing through it
    let mut corners: HashMap<VertexId, Vec<(FaceId, CoedgeId, CoedgeId)>> = HashMap::new();
    for l in src.loop_ids() {
        let lp = src.lp(l);
        for (i, c) in lp.coedges().enumerate() {
            let next = lp.coedge((i + 1) % lp.coedges().len());
            corners
                .entry(src.coedge_end(c))
                .or_default()
                .push((src.lp(l).face, c, next));
        }
    }
    let mut affected: Vec<VertexId> = Vec::new();
    for b in &bevels {
        let edge = src.edge(b.edge);
        for v in [edge.start, edge.end] {
            if !affected.contains(&v) {
                affected.push(v);
            }
        }
    }
    affected.sort();
    // faces trimmed or cut into, which the crossing check looks at
    let mut near = vec![false; src.faces.len()];
    for v in &affected {
        for c in &corners[v] {
            near[c.0.index()] = true;
        }
    }

    let mut out = Body::new();
    out.joins = src.joins.clone();
    for v in &src.vertices {
        out.add_vertex(v.point, v.name);
    }
    for e in &src.edges {
        out.add_edge(e.curve.clone(), e.range, e.start, e.end, e.name);
    }
    for _ in &src.shells {
        out.add_shell();
    }
    for (k, f) in src.faces.iter().enumerate() {
        let nf = out.add_face(f.shell, f.surface.clone(), f.same_sense, f.name);
        // a face no corner touches keeps its loops, so its box
        if let (false, Some(b)) = (near[k], f.bbox.get()) {
            out.faces[nf.index()].bbox.set(*b);
        }
    }
    let cfaces: Vec<FaceId> = bevels
        .iter()
        .map(|b| out.add_face(src.face(b.a).shell, b.surface.clone(), b.same_sense, b.name))
        .collect();
    let mut w = Work {
        out,
        solve_v: Vec::new(),
        fit_e: Vec::new(),
        end_vertex: HashMap::new(),
        contact: HashMap::new(),
        corner_path: HashMap::new(),
        cap: HashMap::new(),
        patches: Vec::new(),
        gone: Vec::new(),
    };
    for &v in &affected {
        w.vertex(&src, &bevels, &bevel_of, v, &corners[&v], root)?;
    }

    // contact edges, then every face's loops
    let mut contact_edge: HashMap<(EdgeId, FaceId), EdgeId> = HashMap::new();
    for b in &bevels {
        let e = src.edge(b.edge);
        // from the faces as wholes, unless both are pieces of one
        let whole = src.face(b.a).name.whole() != src.face(b.b).name.whole();
        for f in [b.a, b.b] {
            let s = w.contact[&(b.edge, true, f)];
            let t = w.contact[&(b.edge, false, f)];
            let n = src.face(f).name;
            let name = Name::NONE.with_set(&[if whole { n.whole() } else { n }, b.name]);
            let id = w.out.add_edge(e.curve.clone(), e.range, s, t, name);
            w.fit_e.push(id);
            contact_edge.insert((b.edge, f), id);
        }
    }
    for (&(e, at_start), &v) in &w.end_vertex {
        let edge = &mut w.out.edges[e.index()];
        if at_start {
            edge.start = v;
        } else {
            edge.end = v;
        }
    }
    for e in src.edge_ids() {
        let edge = &src.edges[e.index()];
        let moved = w.end_vertex.contains_key(&(e, true)) || w.end_vertex.contains_key(&(e, false));
        if edge.start == edge.end && moved && w.out.edges[e.index()].start != w.out.edges[e.index()].end {
            return Err(ChamferError::Unsupported {
                edge: edge.name,
                what: "a closed edge cut open at its vertex",
            });
        }
        if moved && !w.gone.contains(&e) {
            w.fit_e.push(e);
        }
        if bevel_of.contains_key(&e) || w.gone.contains(&e) {
            w.out.edges[e.index()].alive = false;
        }
    }
    let mut used: HashMap<EdgeId, bool> = HashMap::new();
    for f in src.face_ids() {
        for &l in &src.face(f).loops {
            let mut uses: Path = Vec::new();
            for c in src.lp(l).coedges() {
                let co = src.coedge(c);
                let e = match contact_edge.get(&(co.edge, f)) {
                    Some(&ce) => ce,
                    None => co.edge,
                };
                if !w.gone.contains(&e) {
                    uses.push((e, co.forward));
                }
                if let Some(p) = w.corner_path.get(&c) {
                    uses.extend(p.iter().copied());
                }
            }
            used.extend(uses.iter().copied());
            w.out.add_loop(f, &uses);
        }
    }
    // a chamfer face: its contact on A walked against A, the cap at that end from A to B, its
    // contact on B walked against B, the cap at the other end from B to A
    for (bi, b) in bevels.iter().enumerate() {
        let mut uses: Path = vec![(contact_edge[&(b.edge, b.a)], !b.fa)];
        uses.extend(w.cap_from(b.edge, b.fa, b.a));
        uses.push((contact_edge[&(b.edge, b.b)], b.fa));
        uses.extend(w.cap_from(b.edge, !b.fa, b.b));
        for &(e, fw) in &uses {
            used.insert(e, fw);
        }
        w.out.add_loop(cfaces[bi], &uses);
    }
    for (p, ring) in std::mem::take(&mut w.patches) {
        // against the other face's use of each patch edge
        let uses: Path = if used.get(&ring[0]) == Some(&true) {
            ring.iter().rev().map(|&e| (e, false)).collect()
        } else {
            ring.iter().map(|&e| (e, true)).collect()
        };
        // the face lies left of its loop: the plane's normal or its reverse
        let pts: Vec<Point3> = uses
            .iter()
            .map(|&(e, fw)| {
                let edge = w.out.edge(e);
                w.out.vertex(if fw { edge.start } else { edge.end }).point
            })
            .collect();
        let turn = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
        if let Surface::Plane(pl) = &w.out.face(p).surface {
            w.out.faces[p.index()].same_sense = pl.normal_vec().dot(turn) > 0.0;
        }
        w.out.add_loop(p, &uses);
    }

    // a chamfer that took an edge whole and left a body that isn't one is still too large
    let took = w.gone.first().map(|&e| src.edge(e).name);
    let Work {
        mut out,
        solve_v,
        fit_e,
        ..
    } = w;
    let mut verts = vec![false; out.vertices.len()];
    for v in solve_v {
        verts[v.index()] = true;
    }
    let mut fit = vec![false; out.edges.len()];
    for e in fit_e {
        fit[e.index()] = true;
    }
    rebuild_some(&mut out, &verts, &fit).map_err(|e| match e {
        RebuildError::Collapsed { edge } => ChamferError::TooLarge { edge },
        e => ChamferError::Rebuild(e),
    })?;
    let out = out.into_compact();
    let problems = check_against(&out, &src);
    let flagged: Vec<bool> = forge_topo::changed_faces(&out, &src)
        .into_iter()
        .zip(&out.faces)
        .map(|(c, f)| c || src.face_by_name(f.name).is_none_or(|g| near[g.index()]))
        .collect();
    if let Some((a, _)) = crossing(&out, &flagged) {
        return Err(ChamferError::Overflows { face: a });
    }
    if let (false, Some(edge)) = (problems.is_empty(), took) {
        return Err(ChamferError::TooLarge { edge });
    }
    if !problems.is_empty() {
        return Err(ChamferError::Invalid(problems));
    }
    Ok(out)
}

struct Work {
    out: Body,
    solve_v: Vec<VertexId>,
    fit_e: Vec<EdgeId>,
    /// The new vertex an untouched edge now ends at.
    end_vertex: HashMap<End, VertexId>,
    /// Where a chamfer's contact on a face ends: (edge, end at its start, face).
    contact: HashMap<(EdgeId, bool, FaceId), VertexId>,
    /// Edges spliced into a face's loop after the coedge arriving at a corner.
    corner_path: HashMap<CoedgeId, Path>,
    /// A chamfer's boundary across a vertex, from its contact on the first face to the other.
    cap: HashMap<End, (FaceId, Path)>,
    /// Corner patches and their edges in order round them.
    patches: Vec<(FaceId, [EdgeId; 3])>,
    /// Untouched edges the chamfer reaches the far end of: gone, that end the corner.
    gone: Vec<EdgeId>,
}

fn reverse(p: &[(EdgeId, bool)]) -> Path {
    p.iter().rev().map(|&(e, f)| (e, !f)).collect()
}

impl Work {
    fn vertex_at(&mut self, p: Point3, name: Name) -> VertexId {
        let v = self.out.add_vertex(p, name);
        self.solve_v.push(v);
        v
    }

    /// A new edge from `a` to `b`, a line between them until fitted.
    fn edge_between(&mut self, a: VertexId, b: VertexId, name: Name) -> EdgeId {
        let (pa, pb) = (self.out.vertex(a).point, self.out.vertex(b).point);
        let dir = (pb - pa).normalize().unwrap_or(UnitVec3::X);
        let e = self.out.add_edge(
            Curve::Line(Line::new(pa, dir)),
            Interval::new(0.0, pa.distance(pb)),
            a,
            b,
            name,
        );
        self.fit_e.push(e);
        e
    }

    fn cap_from(&self, e: EdgeId, at_start: bool, from: FaceId) -> Path {
        let (f, p) = &self.cap[&(e, at_start)];
        if *f == from { p.clone() } else { reverse(p) }
    }

    #[allow(clippy::too_many_arguments)]
    fn vertex(
        &mut self,
        src: &Body,
        bevels: &[Bevel],
        bevel_of: &HashMap<EdgeId, usize>,
        v: VertexId,
        around: &[(FaceId, CoedgeId, CoedgeId)],
        root: Name,
    ) -> Result<(), ChamferError> {
        let vname = src.vertex(v).name;
        let unsupported = |what| ChamferError::Unsupported { edge: vname, what };
        // faces and edge ends in turn round the vertex: face i lies between end i-1 and end i
        // from the least-named face: what follows is numbered in this order, the arenas' is no name
        let first = around
            .iter()
            .min_by_key(|c| src.face(c.0).name)
            .expect("a vertex in use");
        let mut ring: Vec<(FaceId, CoedgeId, CoedgeId)> = vec![*first];
        while ring.len() <= around.len() {
            let out = ring.last().expect("started").2;
            let Some(back) = src.partner(out) else {
                return Err(unsupported("an open edge"));
            };
            match around.iter().find(|c| c.1 == back) {
                Some(&c) if c == ring[0] => break,
                Some(&c) => ring.push(c),
                None => return Err(unsupported("a non-manifold vertex")),
            }
        }
        if ring.len() != around.len() {
            return Err(unsupported("a vertex where other than three faces meet"));
        }
        let f: Vec<FaceId> = ring.iter().map(|c| c.0).collect();
        let ends: Vec<End> = ring
            .iter()
            .map(|c| (src.coedge(c.2).edge, src.coedge(c.2).forward))
            .collect();
        let bev: Vec<Option<usize>> = ends.iter().map(|e| bevel_of.get(&e.0).copied()).collect();
        if ring.len() == 4 {
            return self.through_seam(src, bevels, &f, &ends, &bev, vname, root);
        }
        if ring.len() != 3 {
            return Err(unsupported("a vertex where other than three faces meet"));
        }
        let k = bev.iter().filter(|b| b.is_some()).count();
        let surf = |x: FaceId| src.face(x).surface.clone();
        // where chamfer b meets face x at end e (its cross-section there)
        let cg = |bi: usize, at_start: bool, x: FaceId| -> Point3 {
            let b = &bevels[bi];
            let pts = if at_start { b.at_start } else { b.at_end };
            if x == b.a { pts[0] } else { pts[1] }
        };
        // where an untouched edge leaving the vertex meets a chamfer, nearest the vertex
        let vp = src.vertex(v).point;
        let along = |end: End, s: &Surface| -> Result<Point3, ChamferError> {
            let edge = src.edge(end.0);
            let too_large = ChamferError::TooLarge { edge: edge.name };
            let nearest = |h: &[forge_geom::intersect::CurveHit]| {
                h.iter()
                    .map(|h| h.point)
                    .min_by(|a, b| a.distance(vp).total_cmp(&b.distance(vp)))
            };
            // not on the edge, but past this end of it: a corner the chamfer leaves on the far side of
            // a concave edge. The face beyond runs on that far, and the edge with it
            let on = match curve_surface(&edge.curve, edge.range, s) {
                CurveSurface::Hits(h) => nearest(&h),
                _ => None,
            };
            let past = || {
                let Curve::Line(_) = &edge.curve else { return None };
                let w = edge.range.width();
                let r = forge_math::Interval::new(edge.range.lo - 64.0 * w, edge.range.hi + 64.0 * w);
                match curve_surface(&edge.curve, r, s) {
                    CurveSurface::Hits(h) => nearest(&h),
                    _ => None,
                }
            };
            on.or_else(past).ok_or(too_large)
        };
        let mut n = 0;
        let mut name = |kind: &str| {
            n += 1;
            vname.with_name(root).with_str(kind).with(n)
        };
        // rotate so the chamfered ends come first, in ring order
        let i = match k {
            1 => (0..3).find(|&i| bev[i].is_some()).expect("one"),
            2 => (0..3)
                .find(|&i| bev[i].is_some() && bev[(i + 1) % 3].is_some())
                .expect("adjacent"),
            _ => 0,
        };
        let (i0, i1, i2) = (i, (i + 1) % 3, (i + 2) % 3);
        let smooth = |a: End, b: End| (end_tangent(src, a) + end_tangent(src, b)).length() <= 1e-6;
        let smooth_after = k == 1 && smooth(ends[i0], ends[i1]);
        let smooth_before = k == 1 && smooth(ends[i0], ends[i2]);
        match k {
            1 if smooth_after || smooth_before => {
                // the chain runs on unchamfered: the chamfer ends in a flat cap across the edge,
                // a triangle from the vertex to its two contacts
                let bi = bev[i0].expect("chamfered");
                let (e, at) = ends[i0];
                let (s, o, cont, tan) = if smooth_after {
                    (f[i1], f[i0], ends[i1], ends[i2])
                } else {
                    (f[i0], f[i1], ends[i2], ends[i1])
                };
                let pa = self.vertex_at(cg(bi, at, s), name("v"));
                let pb = self.vertex_at(cg(bi, at, o), name("v"));
                self.contact.insert((e, at, s), pa);
                self.contact.insert((e, at, o), pb);
                self.end_vertex.insert(tan, pb);
                let x1 = self.edge_between(v, pa, name("e"));
                let j = self.edge_between(pa, pb, name("e"));
                let x2 = self.edge_between(pb, v, name("e"));
                if smooth_after {
                    self.corner_path.insert(ring[i1].1, vec![(x1, false)]);
                    self.corner_path.insert(ring[i2].1, vec![(x2, false)]);
                    self.cap.insert(ends[i0], (o, vec![(j, false)]));
                } else {
                    self.corner_path.insert(ring[i0].1, vec![(x1, true)]);
                    self.corner_path.insert(ring[i2].1, vec![(x2, true)]);
                    self.cap.insert(ends[i0], (s, vec![(j, true)]));
                }
                let normal = (-end_tangent(src, cont))
                    .normalize()
                    .ok_or(unsupported("an edge with no direction"))?;
                let pf = self.out.add_face(
                    src.face(s).shell,
                    Surface::Plane(Plane::new(Frame::from_z(vp, normal))),
                    true,
                    root.with_str("end")
                        .with_name(vname)
                        .with_name(src.edge(e).name.whole()),
                );
                self.patches.push((pf, [x1, j, x2]));
            }
            1 => {
                let bi = bev[i0].expect("chamfered");
                let (e, at) = ends[i0];
                let c = &bevels[bi].surface;
                // the plane through an untouched edge's far vertex takes the whole edge: that vertex is
                // the corner (a chamfer beside an earlier one of the same size), unless it moves too
                let mut corner = |end: End, name: Name| -> Result<VertexId, ChamferError> {
                    let x = along(end, c)?;
                    let edge = src.edge(end.0);
                    let far = if end.1 { edge.end } else { edge.start };
                    let fixed = !bevels.iter().any(|b| {
                        let be = src.edge(b.edge);
                        be.start == far || be.end == far
                    });
                    if fixed && far != v && x.distance(src.vertex(far).point) <= tol::LINEAR {
                        self.gone.push(end.0);
                        return Ok(far);
                    }
                    Ok(self.vertex_at(x, name))
                };
                let p = corner(ends[i1], name("v"))?;
                let q = corner(ends[i2], name("v"))?;
                self.end_vertex.insert(ends[i1], p);
                self.end_vertex.insert(ends[i2], q);
                self.contact.insert((e, at, f[i0]), q);
                self.contact.insert((e, at, f[i1]), p);
                let x = self.edge_between(p, q, name("e"));
                self.corner_path.insert(ring[i2].1, vec![(x, true)]);
                self.cap.insert(ends[i0], (f[i0], vec![(x, false)]));
            }
            2 => {
                let (b0, b1) = (bev[i0].expect("chamfered"), bev[i1].expect("chamfered"));
                let (c0, c1) = (bevels[b0].surface.clone(), bevels[b1].surface.clone());
                let ((e0, at0), (e1, at1)) = (ends[i0], ends[i1]);
                let qg = cg(b0, at0, f[i1]).lerp(cg(b1, at1, f[i1]), 0.5);
                let mut rg = cg(b0, at0, f[i0]).lerp(cg(b1, at1, f[i2]), 0.5);
                let smooth = b0 == b1 || {
                    let t0 = end_tangent(src, ends[i0]);
                    let t1 = end_tangent(src, ends[i1]);
                    (t0 + t1).length() <= 1e-6
                };
                // where the mitre line C0 x C1 leaves: across the untouched edge, or through a face
                let exit = if smooth {
                    0
                } else {
                    let q = solve_point(&[surf(f[i1]), c0.clone(), c1.clone()], qg)
                        .ok_or(unsupported("chamfers that do not meet"))?;
                    let ra = solve_point(&[c0.clone(), c1.clone(), surf(f[i2])], rg);
                    let rb = solve_point(&[c0.clone(), c1.clone(), surf(f[i0])], rg);
                    match (ra, rb) {
                        (Some(a), Some(b)) if a.distance(b) <= tol::LINEAR => 0,
                        (Some(a), Some(b)) if a.distance(q) < b.distance(q) => {
                            rg = a;
                            2
                        }
                        (Some(_), Some(b)) => {
                            rg = b;
                            1
                        }
                        _ => return Err(unsupported("a mitre with no end")),
                    }
                };
                let q = self.vertex_at(qg, name("v"));
                self.contact.insert((e0, at0, f[i1]), q);
                self.contact.insert((e1, at1, f[i1]), q);
                match exit {
                    0 => {
                        let r = self.vertex_at(along(ends[i2], &c0)?, name("v"));
                        self.contact.insert((e0, at0, f[i0]), r);
                        self.contact.insert((e1, at1, f[i2]), r);
                        self.end_vertex.insert(ends[i2], r);
                        let j = self.edge_between(q, r, name("e"));
                        self.cap.insert(ends[i0], (f[i0], vec![(j, false)]));
                        self.cap.insert(ends[i1], (f[i1], vec![(j, true)]));
                    }
                    2 => {
                        // through face i2: chamfer 0 cuts it too
                        let r1 = self.vertex_at(rg, name("v"));
                        let r2 = self.vertex_at(along(ends[i2], &c0)?, name("v"));
                        self.contact.insert((e1, at1, f[i2]), r1);
                        self.contact.insert((e0, at0, f[i0]), r2);
                        self.end_vertex.insert(ends[i2], r2);
                        let j = self.edge_between(q, r1, name("e"));
                        let y = self.edge_between(r1, r2, name("e"));
                        self.corner_path.insert(ring[i2].1, vec![(y, true)]);
                        self.cap.insert(ends[i0], (f[i0], vec![(y, false), (j, false)]));
                        self.cap.insert(ends[i1], (f[i1], vec![(j, true)]));
                    }
                    _ => {
                        // through face i0: chamfer 1 cuts it too
                        let r1 = self.vertex_at(rg, name("v"));
                        let r2 = self.vertex_at(along(ends[i2], &c1)?, name("v"));
                        self.contact.insert((e0, at0, f[i0]), r1);
                        self.contact.insert((e1, at1, f[i2]), r2);
                        self.end_vertex.insert(ends[i2], r2);
                        let j = self.edge_between(q, r1, name("e"));
                        let y = self.edge_between(r1, r2, name("e"));
                        self.corner_path.insert(ring[i0].1, vec![(y, false)]);
                        self.cap.insert(ends[i0], (f[i0], vec![(j, false)]));
                        self.cap.insert(ends[i1], (f[i1], vec![(j, true), (y, true)]));
                    }
                }
            }
            _ => {
                // three chamfers: a flat patch through the three points where pairs meet on a face
                let b: Vec<usize> = bev.iter().map(|x| x.expect("all chamfered")).collect();
                let mut qs = Vec::new();
                for j in 0..3 {
                    let h = (j + 2) % 3;
                    let (eh, ej) = (ends[h], ends[j]);
                    let guess = cg(b[h], eh.1, f[j]).lerp(cg(b[j], ej.1, f[j]), 0.5);
                    let p = solve_point(
                        &[
                            surf(f[j]),
                            bevels[b[h]].surface.clone(),
                            bevels[b[j]].surface.clone(),
                        ],
                        guess,
                    )
                    .ok_or(unsupported("chamfers that do not meet"))?;
                    qs.push(p);
                }
                let mut normal = (qs[1] - qs[0])
                    .cross(qs[2] - qs[0])
                    .normalize()
                    .ok_or(unsupported("a corner with no patch"))?;
                let outward: Vec3 = f
                    .iter()
                    .map(|&x| face_normal_at(src, x, src.vertex(v).point))
                    .fold(Vec3::ZERO, |s, n| s + n);
                if normal.dot(outward) < 0.0 {
                    normal = -normal;
                }
                let patch = Surface::Plane(Plane::new(Frame::from_z(qs[0], normal)));
                let pf = self.out.add_face(
                    src.face(f[0]).shell,
                    patch,
                    true,
                    root.with_str("corner").with_name(vname),
                );
                let vs: Vec<VertexId> = qs.iter().map(|&p| self.vertex_at(p, name("v"))).collect();
                let mut ring_e = [EdgeId(0); 3];
                for j in 0..3 {
                    let (e, at) = ends[j];
                    self.contact.insert((e, at, f[j]), vs[j]);
                    self.contact.insert((e, at, f[(j + 1) % 3]), vs[(j + 1) % 3]);
                    ring_e[j] = self.edge_between(vs[j], vs[(j + 1) % 3], name("e"));
                    self.cap.insert(ends[j], (f[j], vec![(ring_e[j], true)]));
                }
                self.patches.push((pf, ring_e));
            }
        }
        Ok(())
    }

    /// Four faces where two chamfers on one surface cross a seam: the faces either side of each
    /// unchamfered edge are halves of one surface (a revolve's halves), so the chamfer runs
    /// straight through and the two seam edges end at its contacts.
    #[allow(clippy::too_many_arguments)]
    fn through_seam(
        &mut self,
        src: &Body,
        bevels: &[Bevel],
        f: &[FaceId],
        ends: &[End],
        bev: &[Option<usize>],
        vname: Name,
        root: Name,
    ) -> Result<(), ChamferError> {
        let unsupported = |what| ChamferError::Unsupported { edge: vname, what };
        let i = (0..2).find(|&i| bev[i].is_some()).unwrap_or(0);
        let (Some(b0), None, Some(b2), None) = (bev[i], bev[i + 1], bev[i + 2], bev[(i + 3) % 4]) else {
            return Err(unsupported("a vertex where other than three faces meet"));
        };
        let halves = |a: FaceId, b: FaceId| {
            let (a, b) = (src.face(a), src.face(b));
            a.surface == b.surface && a.same_sense == b.same_sense
        };
        let [f0, f1, f2, f3] = [f[i], f[i + 1], f[i + 2], f[(i + 3) % 4]];
        if !halves(f1, f2) || !halves(f3, f0) || bevels[b0].surface != bevels[b2].surface {
            return Err(unsupported("a vertex where other than three faces meet"));
        }
        let ((e0, at0), (e2, at2)) = (ends[i], ends[i + 2]);
        let pick = |b: usize, at: bool, x: FaceId| {
            let pts = if at { bevels[b].at_start } else { bevels[b].at_end };
            if x == bevels[b].a { pts[0] } else { pts[1] }
        };
        let mut n = 0;
        let mut name = |kind: &str| {
            n += 1;
            vname.with_name(root).with_str(kind).with(n)
        };
        let q = self.vertex_at(pick(b0, at0, f1), name("v"));
        let r = self.vertex_at(pick(b0, at0, f0), name("v"));
        self.contact.insert((e0, at0, f1), q);
        self.contact.insert((e2, at2, f2), q);
        self.contact.insert((e0, at0, f0), r);
        self.contact.insert((e2, at2, f3), r);
        self.end_vertex.insert(ends[i + 1], q);
        self.end_vertex.insert(ends[(i + 3) % 4], r);
        let j = self.edge_between(q, r, name("e"));
        self.cap.insert((e0, at0), (f1, vec![(j, true)]));
        self.cap.insert((e2, at2), (f2, vec![(j, true)]));
        Ok(())
    }
}

/// The edge's direction leaving the vertex at end `e`.
fn end_tangent(b: &Body, e: End) -> Vec3 {
    let edge = b.edge(e.0);
    let t = if e.1 { edge.range.lo } else { edge.range.hi };
    let d = edge.curve.deriv(t).d1.normalize().map_or(Vec3::ZERO, |u| u.get());
    if e.1 { d } else { -d }
}

/// The face's outward normal at a point on (or next to) it.
fn face_normal_at(b: &Body, f: FaceId, p: Point3) -> Vec3 {
    let face = b.face(f);
    let g = face.surface.gradient(p);
    if face.same_sense { g } else { -g }
}

/// The chamfer for edge `e`: its faces, its surface, and its cross-sections at both ends.
fn bevel(src: &Body, e: EdgeId, size: ChamferSize, root: Name) -> Result<Bevel, ChamferError> {
    let edge = src.edge(e);
    let ename = edge.name;
    let unsupported = |what| ChamferError::Unsupported { edge: ename, what };
    if edge.coedges.len() != 2 {
        return Err(unsupported("an edge without two faces"));
    }
    let (ca, cb) = (edge.coedges[0], edge.coedges[1]);
    let (a, b) = (src.coedge_face(ca), src.coedge_face(cb));
    if a == b {
        return Err(unsupported("a seam"));
    }
    let fa = src.coedge(ca).forward;
    let (sa, sb) = (&src.face(a).surface, &src.face(b).surface);
    // in-face directions away from the edge, at parameter t
    let section = |t: f64| -> (Point3, Vec3, Vec3, Vec3, Vec3, Vec3) {
        let d = edge.curve.deriv(t);
        let tan = d.d1.normalize().map_or(Vec3::ZERO, |u| u.get());
        let (na, nb) = (face_normal_at(src, a, d.p), face_normal_at(src, b, d.p));
        let s = if fa { 1.0 } else { -1.0 };
        (d.p, tan, na, nb, na.cross(tan) * s, nb.cross(tan) * -s)
    };
    let (_, _, _, _, wa, wb) = section(edge.range.mid());
    let cos = wa.dot(wb);
    if !(cos > -1.0 + 1e-9 && cos < 1.0 - 1e-9) {
        return Err(ChamferError::Tangent { edge: ename });
    }
    let interior = dmath::acos_clamped(cos);
    let (da, db) = match size {
        ChamferSize::Equal(d) => (d, d),
        ChamferSize::Distances { face, d1, d2 } => {
            if src.face(a).name == face {
                (d1, d2)
            } else if src.face(b).name == face {
                (d2, d1)
            } else {
                return Err(ChamferError::NotOnFace { edge: ename });
            }
        }
        ChamferSize::DistanceAngle { face, d, angle } => {
            if !(angle > tol::ANGULAR && interior + angle < dmath::PI - 1e-9) {
                return Err(ChamferError::BadSize);
            }
            let other = d * dmath::sin(angle) / dmath::sin(interior + angle);
            if src.face(a).name == face {
                (d, other)
            } else if src.face(b).name == face {
                (other, d)
            } else {
                return Err(ChamferError::NotOnFace { edge: ename });
            }
        }
    };
    if !(da > tol::MIN_LENGTH && db > tol::MIN_LENGTH) {
        return Err(ChamferError::BadSize);
    }
    // the point of a face's cross-section (the plane across the edge) at distance d from the edge
    let contact = |s: &Surface, p: Point3, tan: Vec3, w: Vec3, d: f64| -> Option<Point3> {
        let across = Surface::Plane(Plane::new(Frame::from_z(p, tan.normalize()?)));
        let ball = Surface::Sphere(Sphere {
            frame: Frame::WORLD.with_origin(p),
            radius: d,
        });
        solve_point(&[s.clone(), ball, across], p + w * d)
    };
    let ends = |t: f64| -> Option<[Point3; 2]> {
        let (p, tan, _, _, wa, wb) = section(t);
        Some([contact(sa, p, tan, wa, da)?, contact(sb, p, tan, wb, db)?])
    };
    let lost = || ChamferError::TooLarge { edge: ename };
    let at_start = ends(edge.range.lo).ok_or_else(lost)?;
    let at_end = ends(edge.range.hi).ok_or_else(lost)?;
    // away from the material, at the start section where the surface is checked
    let out = {
        let (_, _, na, nb, _, _) = section(edge.range.lo);
        na + nb
    };
    let (surface, same_sense) = match &edge.curve {
        Curve::Line(l) => {
            let along = |s: &Surface| match s {
                Surface::Plane(p) => p.normal_vec().dot(l.dir).abs() <= tol::ANGULAR,
                Surface::Cylinder(c) => c.frame.z().cross(l.dir).length() <= tol::ANGULAR,
                _ => false,
            };
            if !along(sa) || !along(sb) {
                return Err(unsupported(
                    "a straight edge on a face that is not straight along it",
                ));
            }
            let [pa, pb] = at_start;
            let n = (pb - pa).cross(l.dir.get()).normalize().ok_or_else(lost)?;
            let n = if n.dot(out) < 0.0 { -n } else { n };
            let frame = Frame::from_zx(pa, n, l.dir.get()).expect("perpendicular");
            (Surface::Plane(Plane::new(frame)), true)
        }
        Curve::Circle(k) => {
            let (o, z) = (k.frame.origin, k.frame.z());
            let coaxial = |s: &Surface| {
                let axis = |f: &Frame| {
                    let off = f.origin - o;
                    f.z().cross(z).length() <= tol::ANGULAR && (off - z * z.dot(off)).length() <= tol::LINEAR
                };
                match s {
                    Surface::Plane(p) => p.normal_vec().cross(z).length() <= tol::ANGULAR,
                    Surface::Cylinder(c) => axis(&c.frame),
                    Surface::Cone(c) => axis(&c.frame),
                    Surface::Sphere(c) => {
                        let off = c.frame.origin - o;
                        (off - z * z.dot(off)).length() <= tol::LINEAR
                    }
                    Surface::Torus(c) => axis(&c.frame),
                    Surface::Procedural(_) => {
                        forge_geom::spline::meet::spun(s).is_some_and(|w| axis(w.frame()))
                    }
                }
            };
            if !coaxial(sa) || !coaxial(sb) {
                return Err(unsupported(
                    "a circular edge between faces not turned about its axis",
                ));
            }
            let [pa, pb] = at_start;
            let rh = |p: Point3| {
                let off = p - o;
                let h = z.dot(off);
                ((off - z * h).length(), h)
            };
            let ((ra, ha), (rb, hb)) = (rh(pa), rh(pb));
            // the circle's own x, so the halves of a split circle get the very same surface
            let radial = k.frame.x().get();
            // a contact that reaches the axis (a cut wider than the radius) has nowhere to go
            let p0 = edge.curve.eval(edge.range.lo);
            let side = p0 - o - z * z.dot(p0 - o);
            for (r, q) in [(ra, pa), (rb, pb)] {
                if r <= tol::MIN_LENGTH || (q - o - z * z.dot(q - o)).dot(side) <= 0.0 {
                    return Err(lost());
                }
            }
            let surface = if (ha - hb).abs() <= tol::LINEAR {
                Surface::Plane(Plane::new(Frame::from_z(o + z * ha, z)))
            } else if (ra - rb).abs() <= tol::LINEAR {
                Surface::Cylinder(Cylinder {
                    frame: Frame::from_zx(o, z, radial).unwrap_or(k.frame),
                    radius: ra,
                })
            } else {
                // the cone through both circles, opening the way the radius grows
                let up = if (rb - ra) * (hb - ha) > 0.0 { z } else { -z };
                let half = dmath::atan2((rb - ra).abs(), (hb - ha).abs());
                let frame =
                    Frame::from_zx(o + z * ha, up, radial).unwrap_or_else(|| Frame::from_z(o + z * ha, up));
                Surface::Cone(
                    Cone::new(frame, ra, half).ok_or(unsupported("a chamfer cone too flat or steep"))?,
                )
            };
            let mid = pa.lerp(pb, 0.5);
            let same = surface.gradient(mid).dot(out) > 0.0;
            (surface, same)
        }
        Curve::Procedural(_) => {
            // a section edge of a wall, in a plane cap: the cut's cross-section is the same all along
            // it, so the chamfer is that line carried along the edge, square to it
            let Some(c) = forge_geom::spline::spline_of(&edge.curve) else {
                return Err(unsupported("a curved edge that is not a circle or a spline"));
            };
            let Some(normal) = forge_geom::spline::swept::plane_of(&c.c) else {
                return Err(unsupported("a spline edge not in one plane"));
            };
            let holds = |s: &Surface| match s {
                Surface::Plane(p) => p.normal_vec().cross(normal.get()).length() <= tol::ANGULAR,
                _ => forge_geom::spline::meet::extruded(s)
                    .is_some_and(|w| w.frame().z().cross(normal.get()).length() <= tol::ANGULAR),
            };
            if !holds(sa) || !holds(sb) {
                return Err(unsupported(
                    "a spline edge between faces whose section changes along it",
                ));
            }
            let [pa, pb] = at_start;
            let lo = c.c.deriv(edge.range.lo);
            let plane = lo
                .d1
                .normalize()
                .and_then(|t| Frame::from_zx(lo.p, t, normal.get()))
                .ok_or_else(lost)?;
            let (la, lb) = (plane.point_to_local(pa), plane.point_to_local(pb));
            let section = forge_geom::spline::Spline::from_pieces(
                1,
                vec![0.0, pa.distance(pb)],
                vec![[la.x, la.y, 0.0, 1.0], [lb.x, lb.y, 0.0, 1.0]],
            )
            .ok_or_else(lost)?;
            // (the whole curve's spine: pieces of the edge keep the one surface)
            let spine = forge_geom::spline::Spine::planar(c.c.clone(), normal).ok_or_else(lost)?;
            let frame = {
                let (at, back) = spine.turn_to(spine.domain().lo, edge.range.lo);
                Frame::from_zx(
                    at + (plane.origin - lo.p),
                    back(plane.z().get()).normalize().ok_or_else(lost)?,
                    back(plane.x().get()),
                )
                .ok_or_else(lost)?
            };
            let surface = forge_geom::spline::Swept::new(spine, std::sync::Arc::new(section), &frame)
                .ok_or_else(lost)?
                .surface();
            let mid = pa.lerp(pb, 0.5);
            let same = surface.gradient(mid).dot(out) > 0.0;
            (surface, same)
        }
        _ => return Err(unsupported("a curved edge that is not a circle")),
    };
    Ok(Bevel {
        edge: e,
        a,
        b,
        fa,
        surface,
        same_sense,
        name: root.with_str("chamfer").with_name(ename),
        at_start,
        at_end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Profile, extrude};
    use forge_math::Point2;

    fn boxed() -> Body {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::ORIGIN, 20.0, 10.0, 0)],
        };
        extrude(&p, 8.0, 1).unwrap()
    }

    fn top_edges() -> Vec<Name> {
        let root = Name::feature(1);
        (0..4)
            .map(|k| Name::NONE.with_set(&[root.with_str("end"), root.with_str("side").with(k)]))
            .collect()
    }

    #[test]
    fn one_edge_and_a_loop() {
        let b = boxed();
        let c = chamfer(&b, &top_edges()[..1], ChamferSize::Equal(2.0), 2).unwrap();
        assert_eq!(c.faces.len(), 7);
        let c = chamfer(&b, &top_edges(), ChamferSize::Equal(2.0), 2).unwrap();
        assert_eq!(c.faces.len(), 10);
        let c = chamfer(&b, &top_edges()[..2], ChamferSize::Equal(1.0), 2).unwrap();
        assert_eq!(c.faces.len(), 8);
    }

    #[test]
    fn every_edge_with_corner_patches() {
        let b = boxed();
        let all: Vec<Name> = b.edges.iter().map(|e| e.name).collect();
        let c = chamfer(&b, &all, ChamferSize::Equal(1.0), 2).unwrap();
        assert_eq!(c.faces.len(), 6 + 12 + 8);
    }

    #[test]
    fn unequal_mitre() {
        let b = boxed();
        let top = Name::feature(1).with_str("end");
        let size = ChamferSize::Distances {
            face: top,
            d1: 1.0,
            d2: 2.5,
        };
        let c = chamfer(&b, &top_edges()[..2], size, 2).unwrap();
        assert_eq!(c.faces.len(), 8);
    }

    #[test]
    fn too_large() {
        let b = boxed();
        assert!(matches!(
            chamfer(&b, &top_edges(), ChamferSize::Equal(5.0), 2),
            Err(ChamferError::TooLarge { .. } | ChamferError::Invalid(_))
        ));
    }

    #[test]
    fn circles() {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![
                Profile::rounded_rectangle(Point2::ORIGIN, 30.0, 20.0, 4.0, 0),
                Profile::circle(Point2::new(15.0, 10.0), 3.0, 50),
            ],
        };
        let b = extrude(&p, 6.0, 1).unwrap();
        let root = Name::feature(1);
        let top: Vec<Name> = (0..8)
            .chain([50])
            .map(|k| Name::NONE.with_set(&[root.with_str("end"), root.with_str("side").with(k)]))
            .collect();
        let c = chamfer(&b, &top, ChamferSize::Equal(1.0), 2).unwrap();
        assert_eq!(c.faces.len(), b.faces.len() + 9);
    }
}
