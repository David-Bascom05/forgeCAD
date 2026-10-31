use crate::rebuild::{RebuildError, rebuild_some};
use crate::selfx::crossing;
use crate::shell::offset;
use forge_geom::{Cone, Curve, CurveGeom, Cylinder, Line, Plane, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point3, UnitVec3, dmath, tol};
use forge_topo::{Body, EdgeId, FaceId, Name, Problem, VertexId, check};

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
            for &c in &base.lp(l).coedges {
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
    for e in base.edge_ids().filter(|&e| rim(e)) {
        for v in [base.edge(e).start, base.edge(e).end] {
            if !smooth[v.index()] {
                return Err(ThickenError::Crease {
                    vertex: base.vertex(v).name,
                });
            }
        }
    }

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
                for &c in &base.lp(l).coedges {
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
    for &f in &set {
        for &l in &base.face(f).loops {
            for &c in &base.lp(l).coedges {
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
                // round a closed rim a flat wall is a ring: two loops, no seam across it
                let ring = edge.start == edge.end && matches!(w, Surface::Plane(_));
                let nf = out.add_face(
                    shell,
                    w,
                    n.dot(away) > 0.0,
                    root.with_str("wall").with_name(edge.name),
                );
                let (bot, top) = (
                    elo[co.edge.index()].expect("made"),
                    ehi[co.edge.index()].expect("made"),
                );
                if ring {
                    out.add_loop(nf, &[(bot, co.forward)]);
                    out.add_loop(nf, &[(top, !co.forward)]);
                    continue;
                }
                let (cs, ce) = (base.coedge_start(c), base.coedge_end(c));
                let (up_e, up_s) = (rise_of(&mut out, ce), rise_of(&mut out, cs));
                out.add_loop(
                    nf,
                    &[(bot, co.forward), (up_e, true), (top, !co.forward), (up_s, false)],
                );
            }
        }
    }
    let mut verts = vec![false; out.vertices.len()];
    solve_v.iter().for_each(|v| verts[v.index()] = true);
    let mut edges = vec![false; out.edges.len()];
    solve_e.iter().for_each(|e| edges[e.index()] = true);
    rebuild_some(&mut out, &verts, &edges).map_err(ThickenError::Rebuild)?;
    let problems = check(&out);
    if !problems.is_empty() {
        return Err(ThickenError::Invalid(problems));
    }
    if let Some((a, b)) = crossing(&out, &vec![true; out.faces.len()]) {
        return Err(ThickenError::SelfIntersecting { a, b });
    }
    Ok(out)
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
    if matches!(s, Surface::Procedural(_)) {
        return None;
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
        // the top and a side meet at a right angle on the rim
        let side = Name::feature(1).with_str("side").with(0);
        assert!(matches!(
            thicken(&b, &[top, side], 0.0, 1.0, 2),
            Err(ThickenError::Crease { .. })
        ));
        assert!(matches!(
            thicken(&b, &[wall], 0.0, 2.5, 2),
            Err(ThickenError::Collapses { .. })
        ));
    }
}
