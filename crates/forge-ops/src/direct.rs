//! Direct edits of a solid's faces: offset, move or replace their surfaces and re-intersect the
//! neighbours, or delete them and let the neighbours close the gap. Every new surface is exact
//! (an offset, a moved copy or a given surface); a result that would not be a valid solid is
//! refused.

use crate::fillet::{FilletEdge, FilletError, FilletOptions, fillet_with};
use crate::rebuild::{RebuildError, reaches_apex, rebuild, rebuild_some};
use crate::selfx::crossing;
use crate::shell::offset;
use forge_geom::{CurveGeom, Surface, SurfaceGeom};
use forge_math::{Frame, Point3, Vec3, tol};
use forge_topo::{Body, EdgeId, FaceId, LoopId, Name, Problem, VertexId, check};
use std::collections::{HashMap, HashSet};

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
    /// The blends next to the changed faces could not be rolled again on their new faces.
    Reblend(FilletError),
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
    resurface(body, faces, true, |f| {
        let d = if f.same_sense { distance } else { -distance };
        let s = offset(&f.surface, d).ok_or(DirectError::Collapses { face: f.name })?;
        Ok((s, f.same_sense))
    })
}

/// Moves the faces by the rigid motion `motion` (local coordinates to world), re-intersecting
/// their neighbours.
pub fn move_faces(body: &Body, faces: &[Name], motion: &Frame) -> R<Body> {
    resurface(body, faces, false, |f| {
        Ok((f.surface.transformed(motion), f.same_sense))
    })
}

/// Puts face `face` on `surface`, facing the way it did, re-intersecting its neighbours.
pub fn replace_face(body: &Body, face: Name, surface: &Surface) -> R<Body> {
    let b = body.compact();
    let f = b.face(find(&b, face)?);
    // the side is judged at a point of the face: one of its edges' middles
    let e = b.edge(b.coedge(b.lp(f.loops[0]).coedge(0)).edge);
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
    resurface(&b, &[face], false, |_| Ok((surface.clone(), sense)))
}

/// Gives each named face the surface (and sense) `new` makes for it, and rebuilds. Blends that
/// roll on a changed face are taken off and rolled again on the new faces. A blend changed with
/// both its faces (one offset of a whole pocket) is first just rebuilt; when `sized` (an offset)
/// a changed blend rolls again with its new radius, else changing a blend itself only rebuilds.
fn resurface(
    body: &Body,
    faces: &[Name],
    sized: bool,
    new: impl Fn(&forge_topo::Face) -> R<(Surface, bool)>,
) -> R<Body> {
    let base = body.compact();
    let mut changed = vec![false; base.faces.len()];
    for &name in faces {
        changed[find(&base, name)?.index()] = true;
    }
    let (blends, corners) = blends_on(&base, &changed);
    let together = blends
        .iter()
        .all(|b| changed[b.face.index()] && b.rails.iter().all(|&f| changed[f.index()]));
    let plain = || -> R<Body> {
        let mut out = base.clone();
        set_surfaces(&mut out, faces, &new)?;
        rebuild(&mut out, &changed).map_err(DirectError::Rebuild)?;
        valid(out, &changed)
    };
    if blends.is_empty() || together {
        match plain() {
            Ok(b) => return Ok(b),
            Err(e) if blends.is_empty() || !sized => return Err(e),
            Err(_) => {}
        }
    } else if !sized && blends.iter().any(|b| changed[b.face.index()]) {
        return plain();
    }
    reblend(&base, faces, &blends, &corners, &new)
}

fn set_surfaces(
    out: &mut Body,
    faces: &[Name],
    new: &impl Fn(&forge_topo::Face) -> R<(Surface, bool)>,
) -> R<Vec<bool>> {
    let mut changed = vec![false; out.faces.len()];
    for &name in faces {
        let Some(f) = out.face_by_name(name) else {
            continue;
        };
        let (s, sense) = new(out.face(f))?;
        let face = &mut out.faces[f.index()];
        face.same_sense = sense;
        face.surface = s;
        changed[f.index()] = true;
        if reaches_apex(out, f, 0.0) {
            return Err(DirectError::Collapses { face: name });
        }
    }
    Ok(changed)
}

/// A rolling-ball blend found on a body: its face, its edges in loop order from a rail (where it
/// touches a face it rolls on: the first and third), the faces across its rails, and its radius.
struct Rolling {
    face: FaceId,
    edges: Vec<EdgeId>,
    rails: [FaceId; 2],
    radius: f64,
}

/// The face across edge `e` from face `f`.
fn across(b: &Body, e: EdgeId, f: FaceId) -> Option<FaceId> {
    b.edge(e)
        .coedges
        .iter()
        .map(|&c| b.coedge_face(c))
        .find(|&g| g != f)
}

/// Face `f`'s outward unit normal at `p`.
fn outward(b: &Body, f: FaceId, p: Point3) -> Vec3 {
    let face = b.face(f);
    let g = face.surface.gradient(p);
    let g = g.normalize().map_or(g, |u| u.get());
    if face.same_sense { g } else { -g }
}

/// Whether the faces at edge `e` meet tangent (one outward normal at its middle).
fn smooth(b: &Body, e: EdgeId) -> bool {
    let edge = b.edge(e);
    let p = edge.curve.eval(edge.range.mid());
    let [c0, c1] = edge.coedges[..] else {
        return false;
    };
    outward(b, b.coedge_face(c0), p).dot(outward(b, b.coedge_face(c1), p)) >= 1.0 - 1e-9
}

/// Face `f` as a blend of constant radius: a strip whose two opposite edges are tangent to faces
/// a ball of its radius touches both of, at each rail's middle.
fn rolling(b: &Body, f: FaceId) -> Option<Rolling> {
    let face = b.face(f);
    let radius = match &face.surface {
        Surface::Cylinder(c) => c.radius,
        Surface::Torus(t) => t.minor,
        _ => return None,
    };
    let [l] = face.loops[..] else { return None };
    let es: Vec<EdgeId> = b.lp(l).coedges.iter().map(|&c| b.coedge(c).edge).collect();
    if es.len() != 4 {
        return None;
    }
    let near = |x: f64| (x - radius).abs() <= tol::LINEAR;
    (0..2).find_map(|k| {
        let (r0, r1) = (es[k], es[k + 2]);
        let rails = [across(b, r0, f)?, across(b, r1, f)?];
        if rails[0] == rails[1] || rails.contains(&f) || !smooth(b, r0) || !smooth(b, r1) {
            return None;
        }
        // the ball's centre off each rail's middle, on the side both faces are a radius away
        let rolls = [r0, r1].iter().all(|&r| {
            let edge = b.edge(r);
            let p = edge.curve.eval(edge.range.mid());
            let n = outward(b, f, p);
            [1.0, -1.0].iter().any(|s| {
                let c = p + n * (s * radius);
                rails.iter().all(|&g| near(b.face(g).surface.implicit(c).abs()))
            })
        });
        rolls.then(|| {
            let mut edges = es.clone();
            edges.rotate_left(k);
            Rolling {
                face: f,
                edges,
                rails,
                radius,
            }
        })
    })
}

/// The blends rolling on a changed face (or changed themselves), with every blend and corner
/// patch they reach through their ends: what must come off together. Corners are faces tangent
/// to everything round them, touching only those blends and the faces they roll on.
fn blends_on(b: &Body, changed: &[bool]) -> (Vec<Rolling>, Vec<FaceId>) {
    let mut found: Vec<Option<Rolling>> = b.faces.iter().map(|_| None).collect();
    for f in b.face_ids() {
        found[f.index()] = rolling(b, f);
    }
    let mut taken = vec![false; b.faces.len()];
    let mut queue: Vec<FaceId> = b
        .face_ids()
        .filter(|&f| {
            found[f.index()]
                .as_ref()
                .is_some_and(|r| changed[f.index()] || r.rails.iter().any(|g| changed[g.index()]))
        })
        .collect();
    let mut corners = Vec::new();
    let edges_of = |f: FaceId| -> Vec<EdgeId> {
        b.face(f)
            .loops
            .iter()
            .flat_map(|&l| b.lp(l).coedges.iter().map(|&c| b.coedge(c).edge))
            .collect()
    };
    while let Some(f) = queue.pop() {
        if std::mem::replace(&mut taken[f.index()], true) {
            continue;
        }
        let Some(r) = &found[f.index()] else {
            // a corner: the blends round it come off too
            corners.push(f);
            queue.extend(
                edges_of(f)
                    .iter()
                    .filter_map(|&e| across(b, e, f))
                    .filter(|g| found[g.index()].is_some()),
            );
            continue;
        };
        for e in [r.edges[1], r.edges[3]] {
            let Some(x) = across(b, e, f) else { continue };
            if found[x.index()].is_some() {
                queue.push(x);
                continue;
            }
            let es = edges_of(x);
            let round: Vec<FaceId> = es.iter().filter_map(|&e| across(b, e, x)).collect();
            let rails = |g: FaceId| {
                round
                    .iter()
                    .any(|&h| found[h.index()].as_ref().is_some_and(|r| r.rails.contains(&g)))
            };
            if es.iter().all(|&e| smooth(b, e))
                && round.iter().all(|&g| found[g.index()].is_some() || rails(g))
            {
                queue.push(x);
            }
        }
    }
    let blends = b
        .face_ids()
        .filter(|f| taken[f.index()])
        .filter_map(|f| found[f.index()].take())
        .collect();
    (blends, corners)
}

/// Takes the blends and their corners off (each strip back to the edge it rounded, each corner
/// to its vertex), changes the faces, and rolls the blends again with their radii (a changed
/// blend's new one), carrying the old names over to what takes their place.
fn reblend(
    base: &Body,
    faces: &[Name],
    blends: &[Rolling],
    corners: &[FaceId],
    new: &impl Fn(&forge_topo::Face) -> R<(Surface, bool)>,
) -> R<Body> {
    let mut gone = vec![false; base.faces.len()];
    let mut strips: Vec<Strip> = Vec::new();
    for r in blends {
        gone[r.face.index()] = true;
        strips.push((r.face, r.edges.clone(), false));
    }
    for &c in corners {
        gone[c.index()] = true;
        let es = base
            .face(c)
            .loops
            .iter()
            .flat_map(|&l| base.lp(l).coedges.iter().map(|&co| base.coedge(co).edge))
            .collect();
        strips.push((c, es, true));
    }
    let Healed {
        body: mut sharp,
        joined,
    } = heal(base, &gone, &[], &strips)?;
    let changed = set_surfaces(&mut sharp, faces, new)?;
    rebuild(&mut sharp, &changed).map_err(DirectError::Rebuild)?;
    let sharp = valid(sharp, &changed)?;
    let mut picks = Vec::new();
    for (r, &e) in blends.iter().zip(&joined) {
        let face = base.face(r.face);
        let radius = if faces.contains(&face.name) {
            match new(face)?.0 {
                Surface::Cylinder(c) => c.radius,
                Surface::Torus(t) => t.minor,
                _ => r.radius,
            }
        } else {
            r.radius
        };
        picks.push(FilletEdge {
            edge: e,
            radius: radius.into(),
        });
    }
    let mut out =
        fillet_with(&sharp, &picks, &FilletOptions::default(), u64::MAX).map_err(DirectError::Reblend)?;
    carry_names(base, &gone, &sharp, &mut out);
    Ok(out)
}

/// Gives what the blends made again the names of what they replace: a face the name of the old
/// face with the same kept faces round it, then an edge or vertex the name of the old one with
/// the same faces round it, wherever that pairing is one to one.
fn carry_names(base: &Body, gone: &[bool], sharp: &Body, out: &mut Body) {
    fn key(names: impl Iterator<Item = Name>) -> Vec<Name> {
        let mut k: Vec<Name> = names.collect();
        k.sort();
        k.dedup();
        k
    }
    // old name -> new entity, for keys held by exactly one of each
    fn pair<T: Copy>(old: Vec<(Vec<Name>, Name)>, new: Vec<(Vec<Name>, T)>) -> Vec<(T, Name)> {
        let mut count: HashMap<&Vec<Name>, (usize, usize, Option<Name>)> = HashMap::new();
        for (k, n) in &old {
            let c = count.entry(k).or_default();
            c.0 += 1;
            c.2 = Some(*n);
        }
        for (k, _) in &new {
            count.entry(k).or_default().1 += 1;
        }
        new.iter()
            .filter_map(|(k, t)| match count[k] {
                (1, 1, Some(n)) => Some((*t, n)),
                _ => None,
            })
            .collect()
    }
    let faces_round = |b: &Body, f: FaceId| -> Vec<Name> {
        b.face(f)
            .loops
            .iter()
            .flat_map(|&l| b.lp(l).coedges.iter())
            .filter_map(|&c| across(b, b.coedge(c).edge, f))
            .map(|g| b.face(g).name)
            .collect()
    };
    let kept: HashSet<Name> = sharp.face_ids().map(|f| sharp.face(f).name).collect();
    let old = base
        .face_ids()
        .filter(|f| gone[f.index()])
        .map(|f| {
            (
                key(faces_round(base, f).into_iter().filter(|n| kept.contains(n))),
                base.face(f).name,
            )
        })
        .collect();
    let new = out
        .face_ids()
        .filter(|&f| !kept.contains(&out.face(f).name))
        .map(|f| {
            (
                key(faces_round(out, f).into_iter().filter(|n| kept.contains(n))),
                f,
            )
        })
        .collect();
    for (f, name) in pair(old, new) {
        out.faces[f.index()].name = name;
    }
    let edge_key =
        |b: &Body, e: EdgeId| key(b.edge(e).coedges.iter().map(|&c| b.face(b.coedge_face(c)).name));
    let had: HashSet<Name> = sharp.edge_ids().map(|e| sharp.edge(e).name).collect();
    let mut used: HashSet<Name> = out.edge_ids().map(|e| out.edge(e).name).collect();
    let old = base
        .edge_ids()
        .map(|e| (edge_key(base, e), base.edge(e).name))
        .collect();
    let new = out
        .edge_ids()
        .filter(|&e| !had.contains(&out.edge(e).name))
        .map(|e| (edge_key(out, e), e))
        .collect();
    for (e, name) in pair(old, new) {
        if used.insert(name) {
            out.edges[e.index()].name = name;
        }
    }
    let vertex_keys = |b: &Body| {
        let mut ks: Vec<Vec<Name>> = vec![Vec::new(); b.vertices.len()];
        for e in b.edge_ids() {
            let edge = b.edge(e);
            for &c in &edge.coedges {
                let n = b.face(b.coedge_face(c)).name;
                ks[edge.start.index()].push(n);
                ks[edge.end.index()].push(n);
            }
        }
        ks.into_iter().map(|k| key(k.into_iter())).collect::<Vec<_>>()
    };
    let (ko, kn) = (vertex_keys(base), vertex_keys(out));
    let had: HashSet<Name> = sharp.vertices.iter().map(|v| v.name).collect();
    let mut used: HashSet<Name> = out.vertices.iter().map(|v| v.name).collect();
    let old = ko
        .into_iter()
        .zip(&base.vertices)
        .filter(|(k, _)| !k.is_empty())
        .map(|(k, v)| (k, v.name))
        .collect();
    let new = kn
        .into_iter()
        .enumerate()
        .filter(|(v, k)| !k.is_empty() && !had.contains(&out.vertices[*v].name))
        .map(|(v, k)| (k, v))
        .collect();
    for (v, name) in pair(old, new) {
        if used.insert(name) {
            out.vertices[v].name = name;
        }
    }
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
            let es: Vec<EdgeId> = base.lp(l).coedges().map(|c| base.coedge(c).edge).collect();
            if es.iter().all(|&e| other(e, f).is_some_and(|g| gone[g.index()])) {
                dropped.push(l);
                es.iter().for_each(|&e| healed[e.index()] = true);
            }
        }
    }
    // the rest must be strips or corners whose every neighbour stays
    let mut strips: Vec<Strip> = Vec::new();
    for f in base.face_ids().filter(|f| gone[f.index()]) {
        let face = base.face(f);
        let border: Vec<EdgeId> = face
            .loops
            .iter()
            .flat_map(|&l| base.lp(l).coedges().map(|c| base.coedge(c).edge))
            .filter(|&e| other(e, f).is_some_and(|g| !gone[g.index()]))
            .collect();
        if border.iter().all(|&e| healed[e.index()]) {
            continue;
        }
        let fail = DirectError::CannotHeal { face: face.name };
        let [l] = face.loops[..] else { return Err(fail) };
        let es: Vec<EdgeId> = base.lp(l).coedges().map(|c| base.coedge(c).edge).collect();
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
        // a strip keeps its longer sides apart (first and third)
        let mut es = es;
        if es.len() == 4
            && length(&base, es[0]) + length(&base, es[2]) < length(&base, es[1]) + length(&base, es[3])
        {
            es.rotate_left(1);
        }
        let point = es.len() == 3;
        strips.push((f, es, point));
    }
    let mut out = heal(&base, &gone, &dropped, &strips);
    // a lone strip may have its long sides either way round
    if out.is_err() && strips.len() == 1 && !strips[0].2 {
        strips[0].1.rotate_left(1);
        out = heal(&base, &gone, &dropped, &strips);
    }
    out.map(|h| h.body)
}

fn closed(b: &Body, e: EdgeId) -> bool {
    b.edge(e).start == b.edge(e).end
}

fn length(b: &Body, e: EdgeId) -> f64 {
    let p = b.edge_points(e, 8);
    p.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// A strip or corner to collapse: its face, its edges in loop order, and whether it goes to one
/// point (a corner) or to one edge (a strip: its first and third edges become one).
type Strip = (FaceId, Vec<EdgeId>, bool);

/// A healed body, and for each edge of the base that a strip collapsed onto, the edge it became.
struct Healed {
    body: Body,
    joined: Vec<EdgeId>,
}

/// `base` without the `gone` faces and the `dropped` loops, each strip collapsed onto one edge
/// and each corner onto one vertex, then the moved vertices and edges re-solved.
fn heal(base: &Body, gone: &[bool], dropped: &[LoopId], strips: &[Strip]) -> R<Healed> {
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
    for (f, es, point) in strips {
        let fail = DirectError::CannotHeal {
            face: base.face(*f).name,
        };
        if *point {
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
        let (sides, ends) = ([es[0], es[2]], [es[1], es[3]]);
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
                .coedges()
                .filter_map(|c| {
                    let co = base.coedge(c);
                    match sub.get(&co.edge) {
                        None => Some((co.edge, co.forward)),
                        Some(None) => None,
                        Some(Some((e, flip))) => Some((*e, co.forward != *flip)),
                    }
                })
                .collect();
            if uses.len() != base.lp(l).coedges().len() {
                touched[nf.index()] = true;
            }
            if uses.is_empty() {
                return Err(DirectError::CannotHeal { face: face.name });
            }
            out.add_loop(nf, &uses);
        }
    }
    let mut verts = vec![false; out.vertices.len()];
    // a vertex merged into another merged later counts as that one
    merged.iter().for_each(|&v| verts[root(&vmap, v).index()] = true);
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
    let joined = joined
        .iter()
        .map(|&e| out.edge_by_name(base.edge(e).name).expect("joined edges stay"))
        .collect();
    Ok(Healed {
        body: valid(out, &changed)?,
        joined,
    })
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
    fn blends_roll_again() {
        let b = plate(vec![Profile::rectangle(Point2::ORIGIN, 20.0, 10.0, 0)], 8.0);
        let top = face("end");
        let edges: Vec<crate::FilletEdge> = (0..4)
            .map(|k| crate::FilletEdge {
                edge: b
                    .edge_by_name(Name::NONE.with_set(vec![top, face("side").with(k)]))
                    .unwrap(),
                radius: 1.0.into(),
            })
            .collect();
        let f = crate::fillet(&b, &edges, 2).unwrap();
        let names = |b: &Body| -> Vec<Name> {
            let mut n: Vec<Name> = b.face_ids().map(|f| b.face(f).name).collect();
            n.extend(b.edge_ids().map(|e| b.edge(e).name));
            n.extend(b.vertices.iter().map(|v| v.name));
            n.sort();
            n
        };
        // the top up 2 and a side out 1: its round and its corners roll again, keeping their names
        let m = move_faces(&f, &[top], &Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 2.0))).unwrap();
        assert!((bbox(&m).z.hi - 10.0).abs() < 1e-12);
        let o = offset_faces(&m, &[face("side").with(1)], 1.0).unwrap();
        assert!((bbox(&o).x.hi - 21.0).abs() < 1e-12, "{:?}", bbox(&o));
        assert_eq!(names(&o), names(&f));
        // the rounds offset alone, inward: they roll again with the smaller radius
        let rounds: Vec<Name> = o
            .face_ids()
            .map(|g| o.face(g).name)
            .filter(|&n| matches!(o.face(o.face_by_name(n).unwrap()).surface, Surface::Cylinder(_)))
            .collect();
        let r = offset_faces(&o, &rounds, -0.5).unwrap();
        assert!(r.face_ids().all(|g| match &r.face(g).surface {
            Surface::Cylinder(c) => c.radius == 0.5,
            _ => true,
        }));
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
        let edge = |k: u64| Name::NONE.with_set(&mut [top, face("side").with(k)]);
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
