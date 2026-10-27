use crate::rebuild::{RebuildError, rebuild};
use forge_geom::intersect::{SurfaceSurface, plane_plane};
use forge_geom::{Cone, Curve, Plane, Surface};
use forge_math::{Frame, Point3, UnitVec3, dmath, tol};
use forge_topo::{Body, Name, Problem, check};

#[derive(Clone, Debug, PartialEq)]
pub enum DraftError {
    /// The angle is zero or reaches a right angle.
    BadAngle,
    NoSuchFace {
        face: Name,
    },
    /// The face faces along the pull direction: it has no side to tilt.
    AlongPull {
        face: Name,
    },
    /// The face never crosses the neutral plane, so there is no line to tilt it about.
    NoNeutralLine {
        face: Name,
    },
    /// The face's surface has no exact drafted form (a sphere, a torus, a cylinder across the pull).
    NoExactSurface {
        face: Name,
    },
    Rebuild(RebuildError),
    Invalid(Vec<Problem>),
}

/// Tilts `faces` so each makes `angle` with the pull direction, `neutral`'s z, keeping where it
/// crosses the neutral plane (`neutral`'s xy) fixed. A positive angle narrows the part towards
/// the pull: a boss comes out tapered, a hole flares. A plane turns about its line on the neutral
/// plane, a cylinder or cone around the pull becomes a cone; the neighbouring edges are
/// re-intersected.
pub fn draft(body: &Body, faces: &[Name], neutral: &Frame, angle: f64) -> Result<Body, DraftError> {
    if !(angle.abs() > tol::ANGULAR && angle.abs() < dmath::FRAC_PI_2 - 1e-6) {
        return Err(DraftError::BadAngle);
    }
    let mut out = body.compact();
    let pull = neutral.z();
    let np = Plane::new(*neutral);
    let mut changed = vec![false; out.faces.len()];
    for &name in faces {
        let f = out
            .face_by_name(name)
            .ok_or(DraftError::NoSuchFace { face: name })?;
        let face = out.face(f);
        out.faces[f.index()].surface = drafted(&face.surface, face.same_sense, &np, pull, angle, name)?;
        changed[f.index()] = true;
    }
    rebuild(&mut out, &changed).map_err(DraftError::Rebuild)?;
    let problems = check(&out);
    if !problems.is_empty() {
        return Err(DraftError::Invalid(problems));
    }
    Ok(out)
}

fn drafted(
    s: &Surface,
    same_sense: bool,
    np: &Plane,
    pull: UnitVec3,
    angle: f64,
    name: Name,
) -> Result<Surface, DraftError> {
    let (sa, ca) = dmath::sin_cos(angle);
    // where the axis of a surface of revolution about the pull crosses the neutral plane
    let on_axis = |frame: &Frame| -> Result<Point3, DraftError> {
        if frame.z().cross(pull).length() > tol::ANGULAR {
            return Err(DraftError::NoExactSurface { face: name });
        }
        let o = frame.origin;
        Ok(o - pull * np.signed_distance(o))
    };
    // a boss's cone opens against the pull, a hole's along it
    let cone = |centre: Point3, radius: f64, x: forge_math::Vec3| -> Result<Surface, DraftError> {
        let outward = if same_sense { 1.0 } else { -1.0 };
        let z = if outward * angle > 0.0 { -pull } else { pull };
        let frame = Frame::from_zx(centre, z, x).unwrap_or_else(|| Frame::from_z(centre, z));
        Cone::new(frame, radius, angle.abs())
            .map(Surface::Cone)
            .ok_or(DraftError::NoExactSurface { face: name })
    };
    match s {
        Surface::Plane(p) => {
            let n = if same_sense {
                p.normal_vec()
            } else {
                -p.normal_vec()
            };
            let h = (n.get() - pull * n.dot(pull))
                .normalize()
                .ok_or(DraftError::AlongPull { face: name })?;
            let line = match plane_plane(p, np) {
                SurfaceSurface::Curves(cs) => match &cs[0].0 {
                    Curve::Line(l) => *l,
                    _ => unreachable!("planes meet in a line"),
                },
                _ => return Err(DraftError::NoNeutralLine { face: name }),
            };
            let n2 = (h * ca + pull * sa).normalize().expect("unit combination");
            let z = if same_sense { n2 } else { -n2 };
            let frame = Frame::from_zx(line.origin, z, line.dir.get()).expect("line lies across the normal");
            Ok(Surface::Plane(Plane::new(frame)))
        }
        Surface::Cylinder(c) => cone(on_axis(&c.frame)?, c.radius, c.frame.x().get()),
        Surface::Cone(c) => {
            let centre = on_axis(&c.frame)?;
            let v = c.frame.z().dot(centre - c.frame.origin);
            let r = c.radius_at(v);
            if r <= tol::MIN_LENGTH {
                return Err(DraftError::NoNeutralLine { face: name });
            }
            cone(centre, r, c.frame.x().get())
        }
        _ => Err(DraftError::NoExactSurface { face: name }),
    }
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

    fn sides(b: &Body) -> Vec<Name> {
        let root = Name::feature(1);
        let caps = [root.with_str("start"), root.with_str("end")];
        b.faces
            .iter()
            .map(|f| f.name)
            .filter(|n| !caps.contains(n))
            .collect()
    }

    #[test]
    fn box_sides_taper() {
        let b = boxed();
        let d = draft(&b, &sides(&b), &Frame::WORLD, 0.1).unwrap();
        let t = d
            .face_by_name(Name::feature(1).with_str("end"))
            .map(|f| d.face(f).clone())
            .unwrap();
        // the top shrinks by 8 tan(0.1) on every side
        let k = 8.0 * dmath::sin(0.1) / dmath::cos(0.1);
        let mut xs: Vec<f64> = Vec::new();
        for &l in &t.loops {
            for &c in &d.lp(l).coedges {
                xs.push(d.vertex(d.coedge_start(c)).point.x);
            }
        }
        xs.sort_by(f64::total_cmp);
        assert!(
            (xs[0] - k).abs() < 1e-9 && (xs[3] - (20.0 - k)).abs() < 1e-9,
            "{xs:?}"
        );
    }

    #[test]
    fn rounded_boss_and_hole() {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![
                Profile::rounded_rectangle(Point2::ORIGIN, 30.0, 20.0, 4.0, 0),
                Profile::circle(Point2::new(15.0, 10.0), 3.0, 50),
            ],
        };
        let b = extrude(&p, 6.0, 1).unwrap();
        let d = draft(
            &b,
            &sides(&b),
            &Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 2.0)),
            -0.05,
        )
        .unwrap();
        assert!(
            d.faces
                .iter()
                .filter(|f| matches!(f.surface, Surface::Cone(_)))
                .count()
                == 5
        );
    }

    #[test]
    fn rejects() {
        let b = boxed();
        let top = Name::feature(1).with_str("end");
        assert_eq!(
            draft(&b, &[top], &Frame::WORLD, 0.1),
            Err(DraftError::AlongPull { face: top })
        );
        assert_eq!(
            draft(&b, &sides(&b), &Frame::WORLD, 0.0),
            Err(DraftError::BadAngle)
        );
    }
}
