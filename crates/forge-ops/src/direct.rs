//! Direct edits of a solid's faces: offset, move or replace their surfaces and re-intersect the
//! neighbours, or delete them and let the neighbours close the gap. Every new surface is exact
//! (an offset, a moved copy or a given surface); a result that would not be a valid solid is
//! refused.

use crate::rebuild::{RebuildError, reaches_apex, rebuild, rebuild_some};
use crate::selfx::crossing;
use crate::shell::offset;
use forge_geom::{CurveGeom, Surface, SurfaceGeom};
use forge_math::Frame;
use forge_topo::{Body, EdgeId, FaceId, LoopId, Name, Problem, VertexId, check};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum DirectError {
    NoSuchFace {
        face: Name,
    },
    /// The face's offset vanishes (a radius reaches zero) or it would pass through a cone's apex.
    Collapses {
        face: Name,
    },
    /// The new surface runs across the face rather than facing the way it did.
    Sideways {
        face: Name,
    },
    /// Deleting this face leaves a gap its neighbours cannot close by extending: it is not a
    /// strip or corner between faces that meet, nor bounded by holes in faces that span it.
    CannotHeal {
        face: Name,
    },
    /// This face would come out on the wrong side of its own edges: turned inside out, or run
    /// through by a neighbour.
    Turned {
        face: Name,
    },
    /// Two faces run through each other.
    SelfIntersecting {
        a: Name,
        b: Name,
    },
    Rebuild(RebuildError),
    Invalid(Vec<Problem>),
}

type R<T> = Result<T, DirectError>;

fn find(body: &Body, name: Name) -> R<FaceId> {
    body.face_by_name(name)
        .ok_or(DirectError::NoSuchFace { face: name })
}

/// Moves each face `distance` along its outward normal (negative goes inward): a plane shifts,
/// a cylinder, cone, sphere or torus changes radius; the neighbours are re-intersected.
pub fn offset_faces(body: &Body, faces: &[Name], distance: f64) -> R<Body> {
    resurface(body, faces, |f| {
        let d = if f.same_sense { distance } else { -distance };
        let s = offset(&f.surface, d).ok_or(DirectError::Collapses { face: f.name })?;
        Ok((s, f.same_sense))
    })
}

/// Moves the faces by the rigid motion `motion` (local coordinates to world), re-intersecting
/// their neighbours.
pub fn move_faces(body: &Body, faces: &[Name], motion: &Frame) -> R<Body> {
    resurface(body, faces, |f| Ok((f.surface.transformed(motion), f.same_sense)))
}

/// Puts face `face` on `surface`, facing the way it did, re-intersecting its neighbours.
pub fn replace_face(body: &Body, face: Name, surface: &Surface) -> R<Body> {
    let b = body.compact();
    let f = b.face(find(&b, face)?);
    // the side is judged at a point of the face: one of its edges' middles
    let e = b.edge(b.coedge(b.lp(f.loops[0]).coedges[0]).edge);
    let p = e.curve.eval(e.range.mid());
    let dot = match (
        f.surface.normal(f.surface.project(p)),
        surface.normal(surface.project(p)),
    ) {
        (Some(a), Some(b)) => a.dot(b),
        _ => 0.0,
    };
    if dot.abs() < 1e-3 {
        return Err(DirectError::Sideways { face });
    }
    let sense = f.same_sense == (dot > 0.0);
    resurface(&b, &[face], |_| Ok((surface.clone(), sense)))
}

/// Gives each named face the surface (and sense) `new` makes for it, and rebuilds.
fn resurface(body: &Body, faces: &[Name], new: impl Fn(&forge_topo::Face) -> R<(Surface, bool)>) -> R<Body> {
    let mut out = body.compact();
    let mut changed = vec![false; out.faces.len()];
    for &name in faces {
        let f = find(&out, name)?;
        let (s, sense) = new(out.face(f))?;
        let face = &mut out.faces[f.index()];
        face.same_sense = sense;
        face.surface = s;
        changed[f.index()] = true;
        if reaches_apex(&out, f, 0.0) {
            return Err(DirectError::Collapses { face: name });
        }
    }
    rebuild(&mut out, &changed).map_err(DirectError::Rebuild)?;
    valid(out, &changed)
}

fn valid(out: Body, changed: &[bool]) -> R<Body> {
    // faces run through each other show first as faces turned inside out: name the crossing
    let problems = check(&out);
    if let Some((a, b)) = crossing(&out, changed) {
        return Err(DirectError::SelfIntersecting { a, b });
    }
    if let Some(Problem::FaceWrongSide { face, .. }) = problems.first()
        && problems
            .iter()
            .all(|p| matches!(p, Problem::FaceWrongSide { .. }))
    {
        return Err(DirectError::Turned {
            face: out.face(*face).name,
        });
    }
    if !problems.is_empty() {
        return Err(DirectError::Invalid(problems));
    }
    Ok(out)
}

/// Removes the faces and heals the gap from the faces around it, exactly: a hole or boss whose
/// rim is a whole loop of a face goes as that loop does (the face's own surface fills it); a
/// four-sided strip (a chamfer, a fillet) goes as its two long sides' faces meet in one edge;
/// a three-sided corner goes as its three neighbours meet in one point. Anything else is
/// refused.
pub fn delete_faces(body: &Body, faces: &[Name]) -> R<Body> {
    let base = body.compact();
    let mut gone = vec![false; base.faces.len()];
    for &n in faces {
        gone[find(&base, n)?.index()] = true;
    }
    let other = |e: EdgeId, f: FaceId| {
        base.edge(e)
            .coedges
            .iter()
            .map(|&c| base.coedge_face(c))
            .find(|&g| g != f)
    };
    // loops of kept faces that run wholly along deleted faces close over
    let mut dropped: Vec<LoopId> = Vec::new();
    let mut healed = vec![false; base.edges.len()];
    for f in base.face_ids().filter(|f| !gone[f.index()]) {
        for &l in &base.face(f).loops {
            let es: Vec<EdgeId> = base.lp(l).coedges.iter().map(|&c| base.coedge(c).edge).collect();
            if es.iter().all(|&e| other(e, f).is_some_and(|g| gone[g.index()])) {
                dropped.push(l);
                es.iter().for_each(|&e| healed[e.index()] = true);
            }
        }
    }
    // the rest must be strips or corners whose every neighbour stays
    let mut strips: Vec<(FaceId, Vec<EdgeId>)> = Vec::new();
    for f in base.face_ids().filter(|f| gone[f.index()]) {
        let face = base.face(f);
        let border: Vec<EdgeId> = face
            .loops
            .iter()
            .flat_map(|&l| base.lp(l).coedges.iter().map(|&c| base.coedge(c).edge))
            .filter(|&e| other(e, f).is_some_and(|g| !gone[g.index()]))
            .collect();
        if border.iter().all(|&e| healed[e.index()]) {
            continue;
        }
        let fail = DirectError::CannotHeal { face: face.name };
        let [l] = face.loops[..] else { return Err(fail) };
        let es: Vec<EdgeId> = base.lp(l).coedges.iter().map(|&c| base.coedge(c).edge).collect();
        let mut around: Vec<FaceId> = Vec::new();
        for &e in &es {
            match other(e, f) {
                Some(g) if !gone[g.index()] && !healed[e.index()] => around.push(g),
                _ => return Err(fail),
            }
        }
        around.sort();
        around.dedup();
        if !(3..=4).contains(&es.len()) || around.len() != es.len() || es.iter().any(|&e| closed(&base, e)) {
            return Err(fail);
        }
        strips.push((f, es));
    }
    // a lone strip may have its long sides either way round: the longer pair first
    let tries = if strips.len() == 1 && strips[0].1.len() == 4 {
        2
    } else {
        1
    };
    let mut last = None;
    for k in 0..tries {
        match heal(&base, &gone, &dropped, &strips, k == 1) {
            Ok(b) => return Ok(b),
            Err(e) => last = Some(e),
        }
    }
    Err(last.expect("tried at least once"))
}

fn closed(b: &Body, e: EdgeId) -> bool {
    b.edge(e).start == b.edge(e).end
}

fn length(b: &Body, e: EdgeId) -> f64 {
    let p = b.edge_points(e, 8);
    p.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// `base` without the `gone` faces and the `dropped` loops, each strip collapsed onto one edge
/// (its longer sides kept apart, or the shorter when `swap`) and each corner onto one vertex,
/// then the moved vertices and edges re-solved.
fn heal(
    base: &Body,
    gone: &[bool],
    dropped: &[LoopId],
    strips: &[(FaceId, Vec<EdgeId>)],
    swap: bool,
) -> R<Body> {
    let mut vmap: Vec<VertexId> = (0..base.vertices.len() as u32).map(VertexId).collect();
    // an edge dropped (None), or replaced by another, run the same way or not
    let mut sub: HashMap<EdgeId, Option<(EdgeId, bool)>> = HashMap::new();
    let mut merged: Vec<VertexId> = Vec::new();
    let mut joined: Vec<EdgeId> = Vec::new();
    fn root(vmap: &[VertexId], mut v: VertexId) -> VertexId {
        while vmap[v.index()] != v {
            v = vmap[v.index()];
        }
        v
    }
    for (f, es) in strips {
        let fail = DirectError::CannotHeal {
            face: base.face(*f).name,
        };
        if es.len() == 3 {
            let keep = root(&vmap, base.edge(es[0]).start);
            for &e in es {
                for v in [base.edge(e).start, base.edge(e).end] {
                    let r = root(&vmap, v);
                    if r != keep {
                        vmap[r.index()] = keep;
                    }
                }
                sub.insert(e, None);
            }
            merged.push(keep);
            continue;
        }
        let longer = length(base, es[0]) + length(base, es[2]) >= length(base, es[1]) + length(base, es[3]);
        let (sides, ends) = if longer != swap {
            ([es[0], es[2]], [es[1], es[3]])
        } else {
            ([es[1], es[3]], [es[0], es[2]])
        };
        let [s1, s2] = sides;
        let on = |s: EdgeId, v: VertexId| base.edge(s).start == v || base.edge(s).end == v;
        for &e in &ends {
            let (a, b) = (base.edge(e).start, base.edge(e).end);
            let (keep, merge) = if on(s1, a) { (a, b) } else { (b, a) };
            if !on(s1, keep) || !on(s2, merge) {
                return Err(fail);
            }
            let (keep, merge) = (root(&vmap, keep), root(&vmap, merge));
            if keep != merge {
                vmap[merge.index()] = keep;
            }
            merged.push(keep);
            sub.insert(e, None);
        }
        let flip = root(&vmap, base.edge(s2).start) != root(&vmap, base.edge(s1).start);
        sub.insert(s2, Some((s1, flip)));
        joined.push(s1);
    }
    let mut out = Body::new();
    for v in &base.vertices {
        out.add_vertex(v.point, v.name);
    }
    for e in &base.edges {
        out.add_edge(
            e.curve.clone(),
            e.range,
            root(&vmap, e.start),
            root(&vmap, e.end),
            e.name,
        );
    }
    for _ in &base.shells {
        out.add_shell();
    }
    let mut touched = vec![false; base.faces.len()];
    for f in base.face_ids() {
        if gone[f.index()] {
            continue;
        }
        let face = base.face(f);
        let nf = out.add_face(face.shell, face.surface.clone(), face.same_sense, face.name);
        for &l in &face.loops {
            if dropped.contains(&l) {
                touched[nf.index()] = true;
                continue;
            }
            let uses: Vec<(EdgeId, bool)> = base
                .lp(l)
                .coedges
                .iter()
                .filter_map(|&c| {
                    let co = base.coedge(c);
                    match sub.get(&co.edge) {
                        None => Some((co.edge, co.forward)),
                        Some(None) => None,
                        Some(Some((e, flip))) => Some((*e, co.forward != *flip)),
                    }
                })
                .collect();
            if uses.len() != base.lp(l).coedges.len() {
                touched[nf.index()] = true;
            }
            if uses.is_empty() {
                return Err(DirectError::CannotHeal { face: face.name });
            }
            out.add_loop(nf, &uses);
        }
    }
    let mut verts = vec![false; out.vertices.len()];
    merged.iter().for_each(|v| verts[v.index()] = true);
    let mut edges = vec![false; out.edges.len()];
    for e in &mut out.edges {
        e.alive &= !e.coedges.is_empty();
    }
    for e in out.edge_ids() {
        let edge = out.edge(e);
        if verts[edge.start.index()] || verts[edge.end.index()] || joined.contains(&e) {
            edges[e.index()] = true;
            for &c in &edge.coedges {
                touched[out.coedge_face(c).index()] = true;
            }
        }
    }
    rebuild_some(&mut out, &verts, &edges).map_err(DirectError::Rebuild)?;
    let out = out.compact();
    let changed: Vec<bool> = out
        .faces
        .iter()
        .map(|f| {
            let i = base.face_by_name(f.name).expect("kept faces keep their names");
            touched[i.index()]
        })
        .collect();
    valid(out, &changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChamferSize, Profile, chamfer, extrude};
    use forge_math::{Point2, Point3, Vec3, dmath};

    fn plate(loops: Vec<crate::ProfileLoop>, h: f64) -> Body {
        let p = Profile {
            plane: Frame::WORLD,
            loops,
        };
        extrude(&p, h, 1).unwrap()
    }

    fn face(role: &str) -> Name {
        Name::feature(1).with_str(role)
    }

    fn bbox(b: &Body) -> forge_math::Box3 {
        forge_math::Box3::from_points(b.vertices.iter().map(|v| v.point))
    }

    #[test]
    fn offset_and_move() {
        let b = plate(vec![Profile::rectangle(Point2::ORIGIN, 20.0, 10.0, 0)], 8.0);
        let top = face("end");
        let o = offset_faces(&b, &[top], 2.0).unwrap();
        assert!((bbox(&o).z.hi - 10.0).abs() < 1e-12);
        let o = offset_faces(&b, &[face("side").with(1)], -3.0).unwrap();
        assert!((bbox(&o).x.hi - 17.0).abs() < 1e-12, "{:?}", bbox(&o));
        let m = move_faces(&b, &[top], &Frame::WORLD.with_origin(Point3::new(0.0, 0.0, -2.0))).unwrap();
        assert!((bbox(&m).z.hi - 6.0).abs() < 1e-12);
        // tilt the top about the y axis through its middle
        let axis = Frame::from_z(Point3::new(10.0, 5.0, 8.0), Vec3::Y.normalize().unwrap());
        let tilt = axis.rotated_about_z(0.1).compose(&axis.inverse());
        let m = move_faces(&b, &[top], &tilt).unwrap();
        assert!((bbox(&m).z.hi - (8.0 + 10.0 * dmath::sin(0.1) / dmath::cos(0.1))).abs() < 1e-9);
        // through the bottom: refused
        assert!(offset_faces(&b, &[top], -9.0).is_err());
    }

    #[test]
    fn replace_with_a_plane() {
        let b = plate(vec![Profile::rectangle(Point2::ORIGIN, 20.0, 10.0, 0)], 8.0);
        let slant = Frame::from_z(
            Point3::new(0.0, 0.0, 10.0),
            Vec3::new(-0.2, 0.0, -1.0).normalize().unwrap(),
        );
        let r = replace_face(&b, face("end"), &Surface::Plane(forge_geom::Plane::new(slant))).unwrap();
        assert!(
            (bbox(&r).z.hi - 10.0).abs() < 1e-9 && (bbox(&r).x.hi - 20.0).abs() < 1e-9,
            "{:?}",
            bbox(&r)
        );
        let side = Frame::from_z(Point3::ORIGIN, Vec3::Z.normalize().unwrap());
        assert!(
            replace_face(
                &b,
                face("side").with(0),
                &Surface::Plane(forge_geom::Plane::new(side))
            )
            .is_err()
        );
    }

    #[test]
    fn delete_holes_and_chamfers() {
        let b = plate(
            vec![
                Profile::rectangle(Point2::ORIGIN, 20.0, 10.0, 0),
                Profile::circle(Point2::new(10.0, 5.0), 2.0, 50),
            ],
            8.0,
        );
        let d = delete_faces(&b, &[face("side").with(50)]).unwrap();
        assert_eq!(d.faces.len(), 6);
        let top = face("end");
        let edge = |k: u64| Name::NONE.with_set(vec![top, face("side").with(k)]);
        let c = chamfer(&b, &[edge(0)], ChamferSize::Equal(1.0), 2).unwrap();
        let strip = |c: &Body| -> Vec<Name> {
            c.faces
                .iter()
                .map(|f| f.name)
                .filter(|n| b.face_by_name(*n).is_none())
                .collect()
        };
        let d = delete_faces(&c, &strip(&c)).unwrap();
        assert_eq!(d.faces.len(), b.faces.len());
        assert!((bbox(&d).z.hi - 8.0).abs() < 1e-12 && bbox(&d).y.lo.abs() < 1e-12);
        // of two chamfers mitred at a corner, neither can go alone, nor both (they meet)
        let c = chamfer(&b, &[edge(0), edge(1)], ChamferSize::Equal(1.0), 2).unwrap();
        assert!(delete_faces(&c, &strip(&c)[..1]).is_err());
        assert!(matches!(
            delete_faces(&c, &strip(&c)),
            Err(DirectError::CannotHeal { .. })
        ));
        // a side face of a box has nothing to close over it
        assert_eq!(
            delete_faces(&b, &[top]),
            Err(DirectError::CannotHeal { face: top })
        );
    }
}
