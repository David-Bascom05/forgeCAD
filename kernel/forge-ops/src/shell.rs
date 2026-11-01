use crate::rebuild::{RebuildError, reaches_apex, rebuild};
use crate::selfx::crossing;
use forge_geom::{Cone, Curve, CurveGeom, Cylinder, Line, Plane, Sphere, Surface, SurfaceGeom, Torus};
use forge_math::{Box2, Interval, dmath, tol};
use forge_topo::{Body, EdgeId, FaceId, Name, Problem, VertexId, check_against};

#[derive(Clone, Debug, PartialEq)]
pub enum ShellError {
    /// The thickness is no more than the tolerance.
    TooThin,
    NoSuchFace {
        face: Name,
    },
    /// Every face was opened: nothing would be left.
    AllOpen,
    /// The thickness reaches past this face's centre of curvature: its offset vanishes.
    Collapses {
        face: Name,
    },
    /// The walls run through each other here: too thick for the part.
    SelfIntersecting {
        a: Name,
        b: Name,
    },
    Rebuild(RebuildError),
    Invalid(Vec<Problem>),
}

/// Hollows `body` to walls `thickness` thick, inside its skin or (`outward`) outside it, with the
/// faces `open` removed. Each face's surface is offset exactly (a plane moves, a cylinder, cone,
/// sphere or torus changes radius, a procedural surface gets a procedural offset) and the offset
/// faces are re-intersected with their neighbours; an opened face leaves a rim on its own surface
/// between the two skins.
///
/// The walls are built directly, not as the body minus a shrunk copy: the shrunk copy needs the
/// same offset surfaces intersected along the same edges, and once it exists the shell is just
/// both skins plus the rims, with no second pass to find what the boolean would only rediscover.
pub fn shell(
    body: &Body,
    thickness: f64,
    outward: bool,
    open: &[Name],
    feature: u64,
) -> Result<Body, ShellError> {
    if thickness <= tol::MIN_LENGTH {
        return Err(ShellError::TooThin);
    }
    let base = body.compact();
    let mut opened = vec![false; base.faces.len()];
    for &n in open {
        let f = base.face_by_name(n).ok_or(ShellError::NoSuchFace { face: n })?;
        opened[f.index()] = true;
    }
    if opened.iter().all(|&o| o) {
        return Err(ShellError::AllOpen);
    }
    // an opened plane all of whose edges its neighbours leave tangentially: their offsets never reach
    // it, so it moves with them and the wall ends in strips square to it (`rim_strips`)
    let strip: Vec<bool> = base
        .face_ids()
        .map(|f| opened[f.index()] && rolls_off_all_round(&base, f, &opened))
        .collect();
    let shift = if outward { thickness } else { -thickness };
    let along = |f: &forge_topo::Face| if f.same_sense { shift } else { -shift };
    // faces whose offsets vanish go, and their neighbours meet instead
    let gone: Vec<bool> = (0..base.faces.len() as u32)
        .map(FaceId)
        .zip(&opened)
        .map(|(f, &o)| !o && face_offset(&base, f, along(base.face(f))).is_none())
        .collect();
    let mut moved = if gone.iter().any(|&g| g) {
        collapse(&base, &gone)?
    } else {
        base.clone()
    };
    let stays = |n: Name| open.contains(&n) && !base.face_by_name(n).is_some_and(|f| strip[f.index()]);
    let open_moved: Vec<bool> = moved.faces.iter().map(|f| stays(f.name)).collect();
    let changed: Vec<bool> = open_moved.iter().map(|&o| !o).collect();
    for f in (0..moved.faces.len() as u32)
        .map(FaceId)
        .filter(|f| changed[f.index()])
    {
        let face = moved.face(f);
        let s = face_offset(&moved, f, along(face)).ok_or(ShellError::Collapses { face: face.name })?;
        moved.faces[f.index()].surface = s;
    }
    // a cone offset towards its apex can close up within the face
    // (a cone that already came to a point stays one: its apex just moves along the axis)
    let pointed = |n: Name| base.face_by_name(n).is_some_and(|f| reaches_apex(&base, f, 0.0));
    let apex = |b: &Body, slack: f64| {
        b.face_ids()
            .find(|&f| reaches_apex(b, f, slack) && !pointed(b.face(f).name))
    };
    if let Some(f) = apex(&moved, 0.0) {
        return Err(ShellError::Collapses {
            face: moved.face(f).name,
        });
    }
    let before = moved.clone();
    if let Err(e) = rebuild(&mut moved, &changed) {
        // vertices move up to the thickness along a cone's axis: past its apex, the face is gone
        return Err(match apex(&before, thickness).or(apex(&before, -thickness)) {
            Some(f) => ShellError::Collapses {
                face: before.face(f).name,
            },
            None => ShellError::Rebuild(e),
        });
    }
    let root = Name::feature(feature);
    let rename = |n: Name| n.with_name(root).with_str("offset");
    let (outer, inner, outer_new, inner_new) = if outward {
        (&moved, &base, true, false)
    } else {
        (&base, &moved, false, true)
    };
    let skip_of = |b: &Body| -> Vec<bool> { b.faces.iter().map(|f| open.contains(&f.name)).collect() };
    let mut out = Body::new();
    out.joins = base.joins.clone();
    let shell = out.add_shell();
    let copy = |src: &Body, flip: bool, renamed: bool, out: &mut Body| -> Vec<Option<EdgeId>> {
        let name = |n: Name| if renamed { rename(n) } else { n };
        let skip = skip_of(src);
        let mut emap: Vec<Option<EdgeId>> = vec![None; src.edges.len()];
        let mut vmap: Vec<Option<VertexId>> = vec![None; src.vertices.len()];
        for f in src.face_ids() {
            if skip[f.index()] {
                continue;
            }
            let face = src.face(f);
            let nf = out.add_face(
                shell,
                face.surface.clone(),
                face.same_sense != flip,
                name(face.name),
            );
            for &l in &face.loops {
                let mut uses = Vec::new();
                for c in src.lp(l).coedges() {
                    let co = src.coedge(c);
                    let e = *emap[co.edge.index()].get_or_insert_with(|| {
                        let edge = src.edge(co.edge);
                        let mut vert = |v: VertexId| {
                            *vmap[v.index()].get_or_insert_with(|| {
                                out.add_vertex(src.vertex(v).point, name(src.vertex(v).name))
                            })
                        };
                        let (a, b) = (vert(edge.start), vert(edge.end));
                        out.add_edge(edge.curve.clone(), edge.range, a, b, name(edge.name))
                    });
                    uses.push((e, co.forward != flip));
                }
                if flip {
                    uses.reverse();
                }
                out.add_loop(nf, &uses);
            }
        }
        emap
    };
    let emap_out = copy(outer, false, outer_new, &mut out);
    let emap_in = copy(inner, true, inner_new, &mut out);
    let mut links: std::collections::HashMap<(VertexId, VertexId), EdgeId> = std::collections::HashMap::new();
    for f in base.face_ids() {
        if !opened[f.index()] {
            continue;
        }
        let face = base.face(f);
        let (fo, fi) = (
            outer.face_by_name(face.name).expect("opened faces stay"),
            inner.face_by_name(face.name).expect("opened faces stay"),
        );
        if outer.face(fo).loops.len() != inner.face(fi).loops.len() {
            return Err(ShellError::Collapses { face: face.name });
        }
        for (k, (&lo, &li)) in outer.face(fo).loops.iter().zip(&inner.face(fi).loops).enumerate() {
            let mut wall = Rims {
                out: &mut out,
                shell,
                links: &mut links,
                emap_out: &emap_out,
                emap_in: &emap_in,
                outer,
                inner,
                name: face.name.with_name(root).with_str("rim").with(k as u64),
            };
            // edges the rim stops at: the other side of them is open too
            let free: Vec<bool> = base
                .lp(face.loops[k])
                .coedges()
                .map(|c| {
                    let e = base.edge(base.coedge(c).edge);
                    !e.coedges.iter().all(|&o| opened[base.coedge_face(o).index()])
                })
                .collect();
            if !strip[f.index()] && free.iter().all(|&x| x) {
                wall.band(face, lo, li, k)?;
                continue;
            }
            let pairs: Vec<_> = outer.lp(lo).coedges().zip(inner.lp(li).coedges()).collect();
            if pairs.len() != free.len() || outer.lp(lo).coedges().len() != inner.lp(li).coedges().len() {
                return Err(ShellError::Collapses { face: face.name });
            }
            if strip[f.index()] {
                wall.strips(face, &pairs)?;
            } else {
                wall.runs(face, &pairs, &free)?;
            }
        }
    }
    let out = out.into_compact();
    let problems = check_against(&out, &base);
    if let Some((a, b)) = crossing(&out, &vec![true; out.faces.len()]) {
        return Err(ShellError::SelfIntersecting { a, b });
    }
    // a wall thicker than the part puts one skin through another without their faces crossing
    if let [Problem::ShellsOverlap { face }] = problems[..] {
        let a = out.face(face).name;
        return Err(ShellError::SelfIntersecting { a, b: a });
    }
    if !problems.is_empty() {
        return Err(ShellError::Invalid(problems));
    }
    Ok(out)
}

/// Whether every edge of `f`, an opened plane, is a smooth one to a face that stays, on a line or circle.
fn rolls_off_all_round(body: &Body, f: FaceId, opened: &[bool]) -> bool {
    let face = body.face(f);
    let Surface::Plane(_) = &face.surface else {
        return false;
    };
    let n = face
        .surface
        .gradient(face.surface.eval(forge_math::Point2::new(0.0, 0.0)));
    face.loops.iter().all(|&l| {
        body.lp(l).coedges().all(|c| {
            let e = body.edge(body.coedge(c).edge);
            let Some(o) = body.partner(c) else { return false };
            let g = body.coedge_face(o);
            let q = e.curve.eval(e.range.mid());
            matches!(e.curve, Curve::Line(_) | Curve::Circle(_))
                && !opened[g.index()]
                && body.face(g).surface.gradient(q).cross(n).length() <= 1e-6
        })
    })
}

/// The faces closing a wall where a face was opened, between its two skins.
struct Rims<'a> {
    out: &'a mut Body,
    shell: forge_topo::ShellId,
    links: &'a mut std::collections::HashMap<(VertexId, VertexId), EdgeId>,
    emap_out: &'a [Option<EdgeId>],
    emap_in: &'a [Option<EdgeId>],
    outer: &'a Body,
    inner: &'a Body,
    name: Name,
}

type Pair = (forge_topo::CoedgeId, forge_topo::CoedgeId);

impl Rims<'_> {
    /// An opened face's edge in the result, on each skin, the way the face runs it.
    fn uses(&self, p: Pair) -> Option<((EdgeId, bool), (EdgeId, bool))> {
        let (a, b) = (self.outer.coedge(p.0), self.inner.coedge(p.1));
        Some((
            (self.emap_out[a.edge.index()]?, a.forward),
            (self.emap_in[b.edge.index()]?, b.forward),
        ))
    }

    fn head(&self, u: (EdgeId, bool)) -> VertexId {
        let e = self.out.edge(u.0);
        if u.1 { e.end } else { e.start }
    }

    fn tail(&self, u: (EdgeId, bool)) -> VertexId {
        let e = self.out.edge(u.0);
        if u.1 { e.start } else { e.end }
    }

    /// The edge from `a` on one skin to `b` on the other, one for all the faces it borders: along
    /// `on` if given (the edge of two opened faces), else straight. As a use from `a` to `b`.
    fn link(
        &mut self,
        a: VertexId,
        b: VertexId,
        on: Option<&Curve>,
        name: Name,
    ) -> Result<(EdgeId, bool), ShellError> {
        if let Some(&e) = self.links.get(&(a, b)) {
            return Ok((e, true));
        }
        if let Some(&e) = self.links.get(&(b, a)) {
            return Ok((e, false));
        }
        let (p, q) = (self.out.vertex(a).point, self.out.vertex(b).point);
        let fail = ShellError::Collapses { face: self.name };
        let (curve, range) = match on {
            Some(Curve::Circle(k)) => {
                let at = |x: forge_math::Point3| {
                    let v = x - k.frame.origin;
                    dmath::atan2(v.dot(k.frame.y().get()), v.dot(k.frame.x().get()))
                };
                let (s, mut t) = (at(p), at(q));
                if t < s {
                    t += dmath::TAU;
                }
                if t - s > dmath::PI {
                    return Err(fail);
                }
                (Curve::Circle(*k), Interval::new(s, t))
            }
            Some(Curve::Line(_)) | None => {
                let d = q - p;
                let dir = d.normalize().ok_or(fail)?;
                (Curve::Line(Line::new(p, dir)), Interval::new(0.0, d.length()))
            }
            Some(_) => return Err(fail),
        };
        let e = self.out.add_edge(curve, range, a, b, name);
        self.links.insert((a, b), e);
        Ok((e, true))
    }

    /// A whole band on the face's own surface, between a loop on each skin.
    fn band(
        &mut self,
        face: &forge_topo::Face,
        lo: forge_topo::LoopId,
        li: forge_topo::LoopId,
        k: usize,
    ) -> Result<(), ShellError> {
        let rim = self
            .out
            .add_face(self.shell, face.surface.clone(), face.same_sense, self.name);
        let uses = |b: &Body, l, map: &[Option<EdgeId>]| -> Option<Vec<(EdgeId, bool)>> {
            b.lp(l)
                .coedges()
                .map(|c| {
                    let co = b.coedge(c);
                    Some((map[co.edge.index()]?, co.forward))
                })
                .collect()
        };
        let fail = || ShellError::Collapses { face: face.name };
        let front = uses(self.outer, lo, self.emap_out).ok_or_else(fail)?;
        let mut back: Vec<(EdgeId, bool)> = uses(self.inner, li, self.emap_in)
            .ok_or_else(fail)?
            .into_iter()
            .map(|(e, fw)| (e, !fw))
            .collect();
        back.reverse();
        // the rim round a hole of the open face has the inner skin's loop outside
        if k == 0 {
            self.out.add_loop(rim, &front);
            self.out.add_loop(rim, &back);
        } else {
            self.out.add_loop(rim, &back);
            self.out.add_loop(rim, &front);
        }
        Ok(())
    }

    /// Bands on the face's own surface, one for each run of edges `free`: they end along the edges
    /// it shares with other opened faces.
    fn runs(&mut self, face: &forge_topo::Face, pairs: &[Pair], free: &[bool]) -> Result<(), ShellError> {
        let n = pairs.len();
        let Some(begin) = (0..n).find(|&i| free[i] && !free[(i + n - 1) % n]) else {
            return Ok(()); // open all round: nothing of its wall is left
        };
        let shared_curve = |i: usize| self.outer.edge(self.outer.coedge(pairs[i].0).edge).curve.clone();
        let mut i = begin;
        let mut count = 0;
        loop {
            if !free[i] {
                i = (i + 1) % n;
                count += 1;
                if count >= n {
                    break;
                }
                continue;
            }
            let start = i;
            let mut run = Vec::new();
            while free[i] && run.len() < n {
                run.push(
                    self.uses(pairs[i])
                        .ok_or(ShellError::Collapses { face: face.name })?,
                );
                i = (i + 1) % n;
                count += 1;
            }
            let before = shared_curve((start + n - 1) % n);
            let after = shared_curve(i);
            let last = run[run.len() - 1];
            let (out_end, in_end) = (self.head(last.0), self.head(last.1));
            let (out_start, in_start) = (self.tail(run[0].0), self.tail(run[0].1));
            let there = self.link(
                out_end,
                in_end,
                Some(&after),
                self.out.vertex(out_end).name.with_str("link"),
            )?;
            let back = self.link(
                in_start,
                out_start,
                Some(&before),
                self.out.vertex(out_start).name.with_str("link"),
            )?;
            let name = self.name.with(start as u64);
            let rim = self
                .out
                .add_face(self.shell, face.surface.clone(), face.same_sense, name);
            let mut uses: Vec<(EdgeId, bool)> = run.iter().map(|u| u.0).collect();
            uses.push(there);
            uses.extend(run.iter().rev().map(|u| (u.1.0, !u.1.1)));
            uses.push(back);
            self.out.add_loop(rim, &uses);
            if count >= n {
                break;
            }
        }
        Ok(())
    }

    /// Where an opened plane's neighbours leave it tangentially, the wall ends in a strip square to
    /// it at each edge, facing in over the opening.
    fn strips(&mut self, face: &forge_topo::Face, pairs: &[Pair]) -> Result<(), ShellError> {
        let fail = ShellError::Collapses { face: face.name };
        let Surface::Plane(pl) = &face.surface else {
            return Err(fail);
        };
        let n = if face.same_sense {
            pl.frame.z()
        } else {
            -pl.frame.z()
        };
        // an opened face's coedges on the skin holding its true place
        let at_plane = if self
            .outer
            .face_by_name(face.name)
            .is_some_and(|f| self.outer.face(f).surface == face.surface)
        {
            self.outer
        } else {
            self.inner
        };
        for (i, &p) in pairs.iter().enumerate() {
            let u = self.uses(p).ok_or(fail.clone())?;
            let c = if std::ptr::eq(at_plane, self.outer) {
                p.0
            } else {
                p.1
            };
            let co = at_plane.coedge(c);
            let edge = at_plane.edge(co.edge);
            let d = edge.curve.deriv(edge.range.mid());
            let along = if co.forward { d.d1 } else { -d.d1 };
            let facing = n.cross(along);
            let surface = match &edge.curve {
                Curve::Line(l) => {
                    let z = l.dir.cross(n.get()).normalize().ok_or(fail.clone())?;
                    Surface::Plane(Plane::new(forge_math::Frame::from_z(l.origin, z)))
                }
                Curve::Circle(k) => Surface::Cylinder(Cylinder {
                    frame: k.frame,
                    radius: k.radius,
                }),
                _ => return Err(fail),
            };
            let same = surface.gradient(d.p).dot(facing) > 0.0;
            let name = self.name.with(i as u64);
            let rim = self.out.add_face(self.shell, surface, same, name);
            let (out_end, in_end) = (self.head(u.0), self.head(u.1));
            let (out_start, in_start) = (self.tail(u.0), self.tail(u.1));
            let there = self.link(
                out_end,
                in_end,
                None,
                self.out.vertex(out_end).name.with_str("link"),
            )?;
            let back = self.link(
                in_start,
                out_start,
                None,
                self.out.vertex(out_start).name.with_str("link"),
            )?;
            self.out.add_loop(rim, &[u.0, there, (u.1.0, !u.1.1), back]);
        }
        Ok(())
    }
}

/// `body` without the faces flagged `gone`, each a strip whose two long sides' faces now meet in
/// one edge (a rounded corner thinner than the wall), or a closed band whose end loops close (a
/// hole thinner than the wall).
fn collapse(body: &Body, gone: &[bool]) -> Result<Body, ShellError> {
    use std::collections::HashMap;
    let mut vmap: Vec<VertexId> = (0..body.vertices.len() as u32).map(VertexId).collect();
    // an edge dropped (None), or replaced by another run the same way or not
    let mut sub: HashMap<EdgeId, Option<(EdgeId, bool)>> = HashMap::new();
    let other_face = |e: EdgeId, f: forge_topo::FaceId| {
        body.edge(e)
            .coedges
            .iter()
            .map(|&c| body.coedge_face(c))
            .find(|&g| g != f)
    };
    for f in body.face_ids() {
        if !gone[f.index()] {
            continue;
        }
        let face = body.face(f);
        let fail = ShellError::Collapses { face: face.name };
        let [l] = face.loops[..] else { return Err(fail) };
        let es: Vec<EdgeId> = body.lp(l).coedges().map(|c| body.coedge(c).edge).collect();
        let Surface::Cylinder(cyl) = &face.surface else {
            return Err(fail);
        };
        let along_axis = |e: EdgeId| match &body.edge(e).curve {
            forge_geom::Curve::Line(line) => line.dir.cross(cyl.frame.z()).length() <= 1e-9,
            _ => false,
        };
        let sides: Vec<EdgeId> = es.iter().copied().filter(|&e| along_axis(e)).collect();
        let ends: Vec<EdgeId> = es.iter().copied().filter(|&e| !along_axis(e)).collect();
        if es.len() != 4 || sides.len() != 2 || ends.len() != 2 {
            return Err(fail);
        }
        for &e in es.iter() {
            match other_face(e, f) {
                Some(g) if gone[g.index()] => return Err(fail),
                _ => {}
            }
        }
        if sides[0] == sides[1] {
            // a closed band (a hole's wall): its end circles close up
            for &e in &ends {
                let g = other_face(e, f).ok_or(fail.clone())?;
                if !body
                    .face(g)
                    .loops
                    .iter()
                    .any(|&l| body.lp(l).coedges().len() == 1)
                {
                    return Err(fail);
                }
                sub.insert(e, None);
            }
            sub.insert(sides[0], None);
            continue;
        }
        let (s1, s2) = (sides[0], sides[1]);
        for &e in &ends {
            let edge = body.edge(e);
            let on = |s: EdgeId, v: VertexId| body.edge(s).start == v || body.edge(s).end == v;
            let (a, b) = (edge.start, edge.end);
            let (keep, merge) = if on(s1, a) { (a, b) } else { (b, a) };
            if !on(s1, keep) || !on(s2, merge) {
                return Err(fail);
            }
            vmap[merge.index()] = keep;
            sub.insert(e, None);
        }
        let flip = vmap[body.edge(s2).start.index()] != body.edge(s1).start;
        sub.insert(s2, Some((s1, flip)));
        sub.insert(s1, Some((s1, false)));
    }
    let mut out = Body::new();
    out.joins = body.joins.clone();
    for v in &body.vertices {
        out.add_vertex(v.point, v.name);
    }
    for e in &body.edges {
        out.add_edge(
            e.curve.clone(),
            e.range,
            vmap[e.start.index()],
            vmap[e.end.index()],
            e.name,
        );
    }
    for _ in &body.shells {
        out.add_shell();
    }
    for f in body.face_ids() {
        if gone[f.index()] {
            continue;
        }
        let face = body.face(f);
        let nf = out.add_face(face.shell, face.surface.clone(), face.same_sense, face.name);
        for &l in &face.loops {
            let uses: Vec<(EdgeId, bool)> = body
                .lp(l)
                .coedges()
                .filter_map(|c| {
                    let co = body.coedge(c);
                    match sub.get(&co.edge) {
                        None => Some((co.edge, co.forward)),
                        Some(None) => None,
                        Some(Some((e, flip))) => Some((*e, co.forward != *flip)),
                    }
                })
                .collect();
            if !uses.is_empty() {
                out.add_loop(nf, &uses);
            }
        }
    }
    Ok(out.into_compact())
}

/// The surface `d` away along `s`'s natural normal, or None where it vanishes (a radius reaches
/// zero).
pub fn offset(s: &Surface, d: f64) -> Option<Surface> {
    let positive = |r: f64| (r > tol::MIN_LENGTH).then_some(r);
    Some(match s {
        Surface::Plane(p) => {
            Surface::Plane(Plane::new(p.frame.with_origin(p.frame.origin + p.frame.z() * d)))
        }
        Surface::Cylinder(c) => Surface::Cylinder(Cylinder {
            frame: c.frame,
            radius: positive(c.radius + d)?,
        }),
        Surface::Sphere(c) => Surface::Sphere(Sphere {
            frame: c.frame,
            radius: positive(c.radius + d)?,
        }),
        Surface::Torus(t) => {
            let r = positive(t.minor + d)?;
            Surface::Torus(Torus::new(t.frame, t.major, r).or_else(|| Torus::spindle(t.frame, t.major, r))?)
        }
        Surface::Cone(c) => {
            // every point moves d along the normal: the same cone, radius d / cos(a) wider
            let half = dmath::atan2(c.sin_a(), c.cos_a());
            let r = c.radius + d / c.cos_a();
            let frame = if r >= 0.0 {
                c.frame
            } else {
                c.frame
                    .with_origin(c.frame.origin + c.frame.z() * (-r / c.tan_a()))
            };
            Surface::Cone(Cone::new(frame, r.max(0.0), half)?)
        }
        Surface::Procedural(p) => forge_geom::blend::Offset::surface(p, d)?,
    })
}

/// Face `f`'s surface moved `d` along its normal; a procedural one is refused only where it would
/// close up over the stretch of it the face spans (a blend's run-outs past its ends may be tighter).
fn face_offset(body: &Body, f: FaceId, d: f64) -> Option<Surface> {
    let Surface::Procedural(p) = &body.face(f).surface else {
        return offset(&body.face(f).surface, d);
    };
    let dom = p.domain();
    let mut uv = Box2::new(dom.u.interval(), dom.v.interval());
    if dom.u.period.is_none() && dom.v.period.is_none() {
        let (mut su, mut sv) = (Interval::EMPTY, Interval::EMPTY);
        for l in forge_topo::uv::face_uv_loops(body, f) {
            for q in &l.points {
                su.include(q.x);
                sv.include(q.y);
            }
        }
        if !su.is_empty() {
            uv = Box2::new(su, sv);
        }
    }
    forge_geom::blend::Offset::surface_over(p, d, p.mesh_radius_over(uv))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Profile, extrude};
    use forge_math::{Frame, Point2};

    fn plate(loops: Vec<crate::ProfileLoop>, h: f64) -> Body {
        extrude(
            &Profile {
                plane: Frame::WORLD,
                loops,
            },
            h,
            1,
        )
        .unwrap()
    }

    #[test]
    fn open_box_and_closed_box() {
        let b = plate(vec![Profile::rectangle(Point2::ORIGIN, 20.0, 10.0, 0)], 8.0);
        let top = Name::feature(1).with_str("end");
        let s = shell(&b, 1.0, false, &[top], 2).unwrap();
        assert_eq!(s.faces.len(), 11);
        let s = shell(&b, 1.0, false, &[], 2).unwrap();
        assert_eq!((s.faces.len(), s.shells.len()), (12, 2));
        let s = shell(&b, 1.5, true, &[top], 2).unwrap();
        assert_eq!(s.faces.len(), 11);
    }

    #[test]
    fn rounded_plate_with_hole() {
        let b = plate(
            vec![
                Profile::rounded_rectangle(Point2::ORIGIN, 30.0, 20.0, 4.0, 0),
                Profile::circle(Point2::new(15.0, 10.0), 3.0, 50),
            ],
            6.0,
        );
        let top = Name::feature(1).with_str("end");
        let s = shell(&b, 1.0, false, &[top], 2).unwrap();
        assert_eq!(s.shells.len(), 1);
        // the corners vanish, but the hole's wall grows through the inner skin
        let got = shell(&b, 5.0, false, &[top], 2);
        assert!(
            matches!(got, Err(ShellError::SelfIntersecting { .. })),
            "{:?}",
            got.map(|b| b.faces.len())
        );
        assert!(shell(&b, 2.0, true, &[top], 2).is_ok());
    }

    /// The rim round a hole of the open face lists its outer loop first, as every planar face
    /// must: the inner skin's loop, inward or outward.
    #[test]
    fn hole_rim_outer_loop_first() {
        let b = plate(
            vec![
                Profile::rectangle(Point2::ORIGIN, 30.0, 20.0, 0),
                Profile::circle(Point2::new(15.0, 10.0), 3.0, 50),
            ],
            6.0,
        );
        let top = Name::feature(1).with_str("end");
        for out in [false, true] {
            let s = shell(&b, 1.0, out, &[top], 2).unwrap();
            for f in s.face_ids().filter(|&f| s.face(f).loops.len() > 1) {
                let first = &forge_topo::uv::face_uv_loops(&s, f)[0];
                assert_eq!(
                    forge_topo::uv::signed_area(&first.points) > 0.0,
                    s.face(f).same_sense,
                    "outward {out}: face {f:?} lists a hole first"
                );
            }
        }
    }
}
