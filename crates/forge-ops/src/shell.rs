use crate::rebuild::{RebuildError, rebuild};
use forge_geom::surface::{Proc, Procedural, SurfDeriv};
use forge_geom::{Cone, Cylinder, Domain2, Plane, Sphere, Surface, Torus};
use forge_math::{Box2, Box3, Frame, Point2, Point3, UnitVec3, Vec3, dmath, tol};
use forge_topo::{Body, EdgeId, Name, Problem, VertexId, check};

#[derive(Clone, Debug, PartialEq)]
pub enum ShellError {
    /// The thickness is no more than the tolerance.
    TooThin,
    NoSuchFace {
        face: Name,
    },
    /// Every face was opened: nothing would be left.
    AllOpen,
    /// Two opened faces share this edge (not supported yet).
    AdjacentOpen {
        edge: Name,
    },
    /// The thickness reaches past this face's centre of curvature: its offset vanishes.
    Collapses {
        face: Name,
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
    for e in base.edge_ids() {
        let edge = base.edge(e);
        if edge.coedges.iter().all(|&c| opened[base.coedge_face(c).index()]) {
            return Err(ShellError::AdjacentOpen { edge: edge.name });
        }
    }
    let mut moved = base.clone();
    let shift = if outward { thickness } else { -thickness };
    let changed: Vec<bool> = opened.iter().map(|&o| !o).collect();
    for (face, _) in moved.faces.iter_mut().zip(&changed).filter(|c| *c.1) {
        let d = if face.same_sense { shift } else { -shift };
        face.surface = offset(&face.surface, d).ok_or(ShellError::Collapses { face: face.name })?;
    }
    rebuild(&mut moved, &changed).map_err(ShellError::Rebuild)?;
    let root = Name::feature(feature);
    let rename = |n: Name| n.with_name(root).with_str("offset");
    let (outer, inner, outer_new, inner_new) = if outward {
        (&moved, &base, true, false)
    } else {
        (&base, &moved, false, true)
    };
    let mut out = Body::new();
    let shell = out.add_shell();
    let copy = |src: &Body, flip: bool, renamed: bool, out: &mut Body| -> Vec<Option<EdgeId>> {
        let name = |n: Name| if renamed { rename(n) } else { n };
        let mut emap: Vec<Option<EdgeId>> = vec![None; src.edges.len()];
        let mut vmap: Vec<Option<VertexId>> = vec![None; src.vertices.len()];
        for f in src.face_ids() {
            if opened[f.index()] {
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
                for &c in &src.lp(l).coedges {
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
    for f in base.face_ids() {
        if !opened[f.index()] {
            continue;
        }
        let face = base.face(f);
        for (k, (&lo, &li)) in outer.face(f).loops.iter().zip(&inner.face(f).loops).enumerate() {
            let rim = out.add_face(
                shell,
                face.surface.clone(),
                face.same_sense,
                face.name.with_name(root).with_str("rim").with(k as u64),
            );
            let uses = |b: &Body, l, map: &[Option<EdgeId>]| -> Vec<(EdgeId, bool)> {
                b.lp(l)
                    .coedges
                    .iter()
                    .map(|&c| {
                        let co = b.coedge(c);
                        (map[co.edge.index()].expect("shared with a kept face"), co.forward)
                    })
                    .collect()
            };
            out.add_loop(rim, &uses(outer, lo, &emap_out));
            let mut back: Vec<(EdgeId, bool)> = uses(inner, li, &emap_in)
                .into_iter()
                .map(|(e, fw)| (e, !fw))
                .collect();
            back.reverse();
            out.add_loop(rim, &back);
        }
    }
    let out = out.compact();
    let problems = check(&out);
    if !problems.is_empty() {
        return Err(ShellError::Invalid(problems));
    }
    Ok(out)
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
        Surface::Torus(t) => Surface::Torus(Torus::new(t.frame, t.major, positive(t.minor + d)?)?),
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
        Surface::Procedural(p) => {
            if p.min_radius() <= -d + tol::MIN_LENGTH {
                return None;
            }
            Surface::Procedural(Proc(std::sync::Arc::new(Offset { base: p.clone(), d })))
        }
    })
}

/// A procedural surface moved `d` along its normal.
#[derive(Debug)]
struct Offset {
    base: Proc,
    d: f64,
}

impl Offset {
    /// The point, du and dv, from the base's derivatives and its normal's (Weingarten).
    fn first(&self, uv: Point2) -> (Point3, Vec3, Vec3) {
        let b = self.base.deriv(uv);
        let n = b.du.cross(b.dv);
        let len = n.length();
        if len == 0.0 {
            return (b.p, b.du, b.dv);
        }
        let unit = n / len;
        let dn = |m: Vec3| (m - unit * unit.dot(m)) / len;
        let nu = dn(b.duu.cross(b.dv) + b.du.cross(b.duv));
        let nv = dn(b.duv.cross(b.dv) + b.du.cross(b.dvv));
        (b.p + unit * self.d, b.du + nu * self.d, b.dv + nv * self.d)
    }
}

impl Procedural for Offset {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        let (p, du, dv) = self.first(uv);
        let (su, sv) = self.base.uv_scale();
        let h = 1e-6;
        let (hu, hv) = (h / su.max(1e-9), h / sv.max(1e-9));
        let (_, u1, v1) = self.first(Point2::new(uv.x + hu, uv.y));
        let (_, u0, v0) = self.first(Point2::new(uv.x - hu, uv.y));
        let (_, _, v3) = self.first(Point2::new(uv.x, uv.y + hv));
        let (_, _, v2) = self.first(Point2::new(uv.x, uv.y - hv));
        SurfDeriv {
            p,
            du,
            dv,
            duu: (u1 - u0) / (2.0 * hu),
            duv: (v1 - v0) / (2.0 * hu),
            dvv: (v3 - v2) / (2.0 * hv),
        }
    }
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        self.base.normal(uv)
    }
    fn domain(&self) -> Domain2 {
        self.base.domain()
    }
    fn project(&self, p: Point3) -> Point2 {
        self.base.project(p)
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        self.base.bbox(uv).grow(self.d.abs())
    }
    fn implicit(&self, p: Point3) -> f64 {
        let uv = self.project(p);
        match self.base.normal(uv) {
            Some(n) => n.dot(p - self.first(uv).0),
            None => self.base.implicit(p) - self.d,
        }
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.base
            .normal(self.project(p))
            .map_or_else(|| self.base.gradient(p), |n| n.get())
    }
    fn transformed(&self, f: &Frame) -> Proc {
        Proc(std::sync::Arc::new(Offset {
            base: self.base.transformed(f),
            d: self.d,
        }))
    }
    fn uv_scale(&self) -> (f64, f64) {
        self.base.uv_scale()
    }
    fn min_radius(&self) -> f64 {
        (self.base.min_radius() + self.d).max(10.0 * tol::MIN_LENGTH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Profile, extrude};
    use forge_math::Point2;

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
        // a corner radius under the thickness vanishes
        assert!(matches!(
            shell(&b, 5.0, false, &[top], 2),
            Err(ShellError::Collapses { .. })
        ));
        assert!(shell(&b, 2.0, true, &[top], 2).is_ok());
    }
}
