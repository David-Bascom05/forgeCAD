use crate::rebuild::{RebuildError, rebuild_some};
use crate::selfx::crossing;
use crate::shell::offset;
use forge_geom::{Cone, Curve, CurveGeom, Cylinder, Line, Plane, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point3, UnitVec3, dmath, tol};
use forge_topo::{Body, CoedgeId, EdgeId, FaceId, Name, Problem, VertexId, check_against};

#[derive(Clone, Debug, PartialEq)]
pub enum ThickenError {
    /// The thickness is no more than the tolerance.
    TooThin,
    NoSuchFace {
        face: Name,
    },
    /// The faces meet at an angle at this vertex on the set's rim: the side walls would not
    /// meet in one line there.
    Crease {
        vertex: Name,
    },
    /// The side wall along this rim edge has no exact surface (the normals along it do not
    /// sweep a plane, cylinder or cone).
    NoExactWall {
        edge: Name,
    },
    /// The thickness reaches past this face's centre of curvature.
    Collapses {
        face: Name,
    },
    SelfIntersecting {
        a: Name,
        b: Name,
    },
    Rebuild(RebuildError),
    Invalid(Vec<Problem>),
}

/// A solid made from `faces` of `body`: the region from `lo` to `hi` along their outward normals
/// (`lo` < `hi`; 0 is the faces themselves), closed round the set's rim by walls swept by the
/// normals. Both skins are exact offsets; each wall is a plane, cylinder or cone.
pub fn thicken(body: &Body, faces: &[Name], lo: f64, hi: f64, feature: u64) -> Result<Body, ThickenError> {
    if hi - lo <= tol::MIN_LENGTH {
        return Err(ThickenError::TooThin);
    }
    let base = body.compact();
    let mut chosen = vec![false; base.faces.len()];
    for &n in faces {
        let f = base.face_by_name(n).ok_or(ThickenError::NoSuchFace { face: n })?;
        chosen[f.index()] = true;
    }
    let set: Vec<FaceId> = base.face_ids().filter(|f| chosen[f.index()]).collect();
    // each vertex's outward normal, and whether the faces at it agree
    let mut normal: Vec<Option<UnitVec3>> = vec![None; base.vertices.len()];
    let mut smooth = vec![true; base.vertices.len()];
    for &f in &set {
        for &l in &base.face(f).loops {
            for c in base.lp(l).coedges() {
                let v = base.coedge_start(c);
                let n = outward(&base, f, base.vertex(v).point).ok_or(ThickenError::Crease {
                    vertex: base.vertex(v).name,
                })?;
                match normal[v.index()] {
                    None => normal[v.index()] = Some(n),
                    Some(m) => smooth[v.index()] &= m.dot(n) >= 1.0 - 1e-12,
                }
            }
        }
    }
    let rim = |e: EdgeId| {
        base.edge(e)
            .coedges
            .iter()
            .filter(|&&c| chosen[base.coedge_face(c).index()])
            .count()
            == 1
    };

    let root = Name::feature(feature);
    let mut out = Body::new();
    let shell = out.add_shell();
    let mut solve_v = Vec::new();
    let mut solve_e = Vec::new();
    // both skins: the far one faces out, the near one in
    let (mut vskins, mut eskins) = (Vec::new(), Vec::new());
    for (d, far) in [(lo, false), (hi, true)] {
        let tag = root.with_str(if far { "top" } else { "base" });
        let mut vmap: Vec<Option<VertexId>> = vec![None; base.vertices.len()];
        let mut emap: Vec<Option<EdgeId>> = vec![None; base.edges.len()];
        for &f in &set {
            let face = base.face(f);
            let s = if d == 0.0 {
                face.surface.clone()
            } else {
                let along = if face.same_sense { d } else { -d };
                offset(&face.surface, along).ok_or(ThickenError::Collapses { face: face.name })?
            };
            let nf = out.add_face(shell, s, face.same_sense == far, tag.with_name(face.name));
            for &l in &face.loops {
                let mut uses = Vec::new();
                for c in base.lp(l).coedges() {
                    let co = base.coedge(c);
                    let edge = base.edge(co.edge);
                    for v in [edge.start, edge.end] {
                        if vmap[v.index()].is_none() {
                            let x = base.vertex(v);
                            let p = x.point + normal[v.index()].expect("set vertex") * d;
                            let nv = out.add_vertex(p, tag.with_name(x.name));
                            if d != 0.0 && !smooth[v.index()] {
                                solve_v.push(nv);
                            }
                            vmap[v.index()] = Some(nv);
                        }
                    }
                    let e = *emap[co.edge.index()].get_or_insert_with(|| {
                        let (a, b) = (vmap[edge.start.index()], vmap[edge.end.index()]);
                        let ne = out.add_edge(
                            edge.curve.clone(),
                            edge.range,
                            a.expect("made"),
                            b.expect("made"),
                            tag.with_name(edge.name),
                        );
                        if d != 0.0 {
                            solve_e.push(ne);
                        }
                        ne
                    });
                    uses.push((e, co.forward == far));
                }
                if !far {
                    uses.reverse();
                }
                out.add_loop(nf, &uses);
            }
        }
        vskins.push(vmap);
        eskins.push(emap);
    }
    let (vlo, vhi, elo, ehi) = (&vskins[0], &vskins[1], &eskins[0], &eskins[1]);
    // a straight rise along the normal from each rim vertex, near skin to far
    let mut rise: Vec<Option<EdgeId>> = vec![None; base.vertices.len()];
    let mut rise_of = |out: &mut Body, v: VertexId| -> EdgeId {
        *rise[v.index()].get_or_insert_with(|| {
            let x = base.vertex(v);
            let n = normal[v.index()].expect("set vertex");
            let line = Line::new(x.point + n * lo, n);
            out.add_edge(
                Curve::Line(line),
                Interval::new(0.0, hi - lo),
                vlo[v.index()].expect("made"),
                vhi[v.index()].expect("made"),
                root.with_str("rise").with_name(x.name),
            )
        })
    };
    // each rim use: its face, its wall and the wall's sense
    struct Rim {
        c: CoedgeId,
        wall: Surface,
        sense: bool,
    }
    let mut rims: Vec<Rim> = Vec::new();
    for &f in &set {
        for &l in &base.face(f).loops {
            for c in base.lp(l).coedges() {
                let co = base.coedge(c);
                if !rim(co.edge) {
                    continue;
                }
                let edge = base.edge(co.edge);
                let no_wall = ThickenError::NoExactWall { edge: edge.name };
                let w = wall(&base, f, &edge.curve, edge.range).ok_or(no_wall.clone())?;
                let mid = edge.curve.eval(edge.range.mid());
                let dir = edge.curve.tangent(edge.range.mid()).ok_or(no_wall.clone())?;
                let away = dir.cross(outward(&base, f, mid).ok_or(no_wall.clone())?)
                    * if co.forward { 1.0 } else { -1.0 };
                let n = w.normal(w.project(mid)).ok_or(no_wall)?;
                rims.push(Rim {
                    c,
                    wall: w,
                    sense: n.dot(away) > 0.0,
                });
            }
        }
    }
    // Where the faces crease at a rim vertex (a thin wall's corner on its cap), the walls of the
    // rim uses either side of it must lie on one plane: they are one wall face, the vertex solved
    // onto it with the skins either side. Elsewhere at a crease the walls do not meet in one line
    // (`Crease`). Runs of rim uses joined so, each the uses after one another round the rim.
    let mut leaving: Vec<Vec<usize>> = vec![Vec::new(); base.vertices.len()];
    for (k, r) in rims.iter().enumerate() {
        leaving[base.coedge_start(r.c).index()].push(k);
    }
    let next = |k: usize| -> Result<Option<usize>, ThickenError> {
        let v = base.coedge_end(rims[k].c);
        if smooth[v.index()] {
            return Ok(None);
        }
        let crease = ThickenError::Crease {
            vertex: base.vertex(v).name,
        };
        match leaving[v.index()][..] {
            [j] if same_plane((&rims[j].wall, rims[j].sense), (&rims[k].wall, rims[k].sense)) => Ok(Some(j)),
            _ => Err(crease),
        }
    };
    let mut after: Vec<Option<usize>> = Vec::with_capacity(rims.len());
    for k in 0..rims.len() {
        after.push(next(k)?);
    }
    let mut before: Vec<Option<usize>> = vec![None; rims.len()];
    for (k, a) in after.iter().enumerate() {
        if let Some(j) = *a {
            before[j] = Some(k);
        }
    }
    let mut taken = vec![false; rims.len()];
    let mut rings = Vec::new();
    for k0 in 0..rims.len() {
        if taken[k0] {
            continue;
        }
        // back to the run's first use (round a closed run, to k0 itself)
        let mut first = k0;
        while let Some(b) = before[first] {
            if b == k0 {
                break;
            }
            first = b;
        }
        let mut run = vec![first];
        taken[first] = true;
        let mut at = first;
        let mut closed = false;
        while let Some(j) = after[at] {
            if j == first {
                closed = true;
                break;
            }
            run.push(j);
            taken[j] = true;
            at = j;
        }
        let r0 = &rims[run[0]];
        let edge0 = base.edge(base.coedge(r0.c).edge);
        let one_closed = run.len() == 1 && edge0.start == edge0.end;
        // round a closed rim a flat wall is a ring: two loops, no seam across it
        let ring = (closed || one_closed) && matches!(r0.wall, Surface::Plane(_));
        let nf = out.add_face(
            shell,
            r0.wall.clone(),
            r0.sense,
            root.with_str("wall").with_name(edge0.name),
        );
        let ends = |k: usize, top: bool| {
            let co = base.coedge(rims[k].c);
            let e = if top { ehi } else { elo };
            (e[co.edge.index()].expect("made"), co.forward != top)
        };
        let bots: Vec<(EdgeId, bool)> = run.iter().map(|&k| ends(k, false)).collect();
        let tops: Vec<(EdgeId, bool)> = run.iter().rev().map(|&k| ends(k, true)).collect();
        if ring {
            out.add_loop(nf, &bots);
            out.add_loop(nf, &tops);
            rings.push(nf);
            continue;
        }
        let (cs, ce) = (
            base.coedge_start(rims[run[0]].c),
            base.coedge_end(rims[run[run.len() - 1]].c),
        );
        let (up_e, up_s) = (rise_of(&mut out, ce), rise_of(&mut out, cs));
        let mut uses = bots;
        uses.push((up_e, true));
        uses.extend(tops);
        uses.push((up_s, false));
        out.add_loop(nf, &uses);
    }
    let mut verts = vec![false; out.vertices.len()];
    solve_v.iter().for_each(|v| verts[v.index()] = true);
    let mut edges = vec![false; out.edges.len()];
    solve_e.iter().for_each(|e| edges[e.index()] = true);
    rebuild_some(&mut out, &verts, &edges).map_err(ThickenError::Rebuild)?;
    // a ring's outer loop first: the one running counter-clockwise round its outward normal
    for f in rings {
        let face = out.face(f);
        let Some(n) = face.surface.normal(
            face.surface.project(
                out.vertex(out.edge(out.coedge(out.lp(face.loops[0]).coedge(0)).edge).start)
                    .point,
            ),
        ) else {
            continue;
        };
        let n = if face.same_sense { n } else { -n };
        let turn = |l: forge_topo::LoopId| {
            let mut pts = Vec::new();
            for c in out.lp(l).coedges() {
                let mut p = out.edge_points(out.coedge(c).edge, 16);
                if !out.coedge(c).forward {
                    p.reverse();
                }
                p.pop();
                pts.extend(p);
            }
            let mut a = forge_math::Vec3::ZERO;
            for k in 0..pts.len() {
                let (p, q) = (pts[k], pts[(k + 1) % pts.len()]);
                a = a + (p - Point3::ORIGIN).cross(q - Point3::ORIGIN);
            }
            a.dot(n.get())
        };
        if turn(face.loops[0]) < 0.0 {
            out.faces[f.index()].loops.swap(0, 1);
        }
    }
    let problems = check_against(&out, &base);
    if !problems.is_empty() {
        return Err(ThickenError::Invalid(problems));
    }
    if let Some((a, b)) = crossing(&out, &vec![true; out.faces.len()]) {
        return Err(ThickenError::SelfIntersecting { a, b });
    }
    Ok(out)
}

/// Whether two walls, each with its sense, are one plane facing the same way.
fn same_plane((a, sa): (&Surface, bool), (b, sb): (&Surface, bool)) -> bool {
    let (Surface::Plane(p), Surface::Plane(q)) = (a, b) else {
        return false;
    };
    let (m, n) = (p.frame.z().get(), q.frame.z().get());
    let (m, n) = (if sa { m } else { -m }, if sb { n } else { -n });
    m.cross(n).length() <= 1e-12
        && m.dot(n) > 0.0
        && m.dot(q.frame.origin - p.frame.origin).abs() <= tol::LINEAR * 1e-2
}

/// Face `f`'s outward normal at `p` (a point on it).
fn outward(b: &Body, f: FaceId, p: Point3) -> Option<UnitVec3> {
    let face = b.face(f);
    let n = face.surface.normal(face.surface.project(p))?;
    Some(if face.same_sense { n } else { -n })
}

/// The surface swept by face `f`'s normals along `c` over `range`, where that is a plane (along
/// a line on a plane, a cylinder's or cone's ruling; across the axis of a surface of revolution),
/// a cylinder (round a plane's circle) or a cone (round a circle of latitude).
fn wall(b: &Body, f: FaceId, c: &Curve, range: Interval) -> Option<Surface> {
    let s = &b.face(f).surface;
    if let Surface::Procedural(p) = s {
        // a spline's extruded wall: its normals all lie square to the extrusion, so along a
        // section (a curve at one height) they sweep that height's plane; along a ruling they are
        // one normal (the line case below). Any other wall built by construction has no rule here
        let e = p
            .spline_wall()
            .then(|| p.as_any())
            .flatten()?
            .downcast_ref::<forge_geom::spline::Extruded>()?;
        if !matches!(c, Curve::Line(_)) {
            let (o, z) = (e.frame().origin, e.frame().z());
            let h = |t: f64| z.dot(c.eval(t) - o);
            let h0 = h(range.mid());
            if [range.lo, range.hi]
                .iter()
                .any(|&t| (h(t) - h0).abs() > 1e-12 * (1.0 + h0.abs()))
            {
                return None;
            }
            let p = c.eval(range.mid());
            return Some(Surface::Plane(Plane::new(Frame::from_z(p, z))));
        }
    }
    let at = |t: f64| {
        let p = c.eval(t);
        s.normal(s.project(p)).map(|n| (p, n))
    };
    let (p, n) = at(range.mid())?;
    match c {
        Curve::Line(l) => {
            let m = l.dir.cross(n).normalize()?;
            for t in [range.lo, range.hi] {
                if at(t)?.1.dot(m).abs() > 1e-12 {
                    return None;
                }
            }
            Some(Surface::Plane(Plane::new(Frame::from_zx(p, m, l.dir.get())?)))
        }
        Curve::Circle(k) => {
            let (o, z) = (k.frame.origin, k.frame.z());
            let on_axis = match s {
                Surface::Plane(_) => true,
                Surface::Cylinder(x) => coaxial(&x.frame, o, z),
                Surface::Cone(x) => coaxial(&x.frame, o, z),
                Surface::Sphere(x) => coaxial(&x.frame, o, z),
                Surface::Torus(x) => coaxial(&x.frame, o, z),
                Surface::Procedural(_) => false,
            };
            if !on_axis {
                return None;
            }
            let radial = (p - o).normalize()?;
            let (a, h) = (n.dot(radial), n.dot(z));
            if n.dot(radial.cross(z)).abs() > 1e-12 {
                return None;
            }
            if a.abs() <= 1e-12 {
                return Some(Surface::Cylinder(Cylinder {
                    frame: k.frame,
                    radius: k.radius,
                }));
            }
            if h.abs() <= 1e-12 {
                return Some(Surface::Plane(Plane::new(k.frame)));
            }
            // the normal lines meet the axis: a cone widening the way they lean out
            let z = if a * h > 0.0 { z } else { -z };
            let half = dmath::atan2(a.abs(), h.abs());
            let frame = Frame::from_zx(o, z, k.frame.x().get())?;
            Cone::new(frame, k.radius, half).map(Surface::Cone)
        }
        _ => None,
    }
}

/// Whether the axis of `frame` is the line through `o` along `z`.
fn coaxial(frame: &Frame, o: Point3, z: UnitVec3) -> bool {
    let off = o - frame.origin;
    frame.z().cross(z).length() <= 1e-12
        && (off - frame.z() * frame.z().dot(off)).length() <= tol::LINEAR * 1e-2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Profile, extrude};
    use forge_math::Point2;

    fn plate(loops: Vec<crate::ProfileLoop>, h: f64) -> Body {
        let p = Profile {
            plane: Frame::WORLD,
            loops,
        };
        extrude(&p, h, 1).unwrap()
    }

    #[test]
    fn slabs() {
        let b = plate(
            vec![
                Profile::rectangle(Point2::ORIGIN, 20.0, 10.0, 0),
                Profile::circle(Point2::new(10.0, 5.0), 2.0, 50),
            ],
            8.0,
        );
        let top = Name::feature(1).with_str("end");
        let s = thicken(&b, &[top], 0.0, 2.0, 2).unwrap();
        assert_eq!(s.faces.len(), 7);
        let s = thicken(&b, &[top], -1.0, 1.0, 2).unwrap();
        assert_eq!(s.faces.len(), 7);
        // the hole's wall alone: a tube, its rims circles on the caps
        let wall = Name::feature(1).with_str("side").with(50);
        let s = thicken(&b, &[wall], 0.0, 1.0, 2).unwrap();
        assert_eq!(s.faces.len(), 4);
        // the top and a side meet at a right angle on the rim: their walls at the corner's ends are
        // one plane each (x = 0, x = 20), so the slabs join, mitred: 20 x 10 less the hole, 20 x 8,
        // and the 20 x 1 corner, each 1 thick
        let side = Name::feature(1).with_str("side").with(0);
        let s = thicken(&b, &[top, side], 0.0, 1.0, 2).unwrap();
        let v = forge_topo::mass::mass(&s).unwrap().volume;
        assert!((v - (380.0 - 4.0 * dmath::PI)).abs() < 1e-9, "{v}");
        // with the hole's wall: the crease between them is inside the set, the skins re-met there
        // (the top's slab outside r = 1, the wall's from r = 1 to 2)
        let s = thicken(&b, &[top, wall], 0.0, 1.0, 2).unwrap();
        let v = forge_topo::mass::mass(&s).unwrap().volume;
        assert!((v - (200.0 - dmath::PI + 24.0 * dmath::PI)).abs() < 1e-9, "{v}");
        // a crease on the rim whose walls are no one plane: a triangular prism's top and one side,
        // the walls at their corner two upright planes at the triangle's angle
        let tri = crate::ProfileLoop::new(vec![
            (
                crate::Segment::Line {
                    a: Point2::new(0.0, 0.0),
                    b: Point2::new(10.0, 0.0),
                },
                0,
            ),
            (
                crate::Segment::Line {
                    a: Point2::new(10.0, 0.0),
                    b: Point2::new(3.0, 6.0),
                },
                1,
            ),
            (
                crate::Segment::Line {
                    a: Point2::new(3.0, 6.0),
                    b: Point2::new(0.0, 0.0),
                },
                2,
            ),
        ]);
        let w = plate(vec![tri], 4.0);
        let side = Name::feature(1).with_str("side").with(0);
        assert!(matches!(
            thicken(&w, &[top, side], 0.0, 1.0, 2),
            Err(ThickenError::Crease { .. })
        ));
        assert!(matches!(
            thicken(&b, &[wall], 0.0, 2.5, 2),
            Err(ThickenError::Collapses { .. })
        ));
    }
}
