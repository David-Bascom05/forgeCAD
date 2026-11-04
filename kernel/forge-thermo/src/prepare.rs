//! What the user modelled, made into the solid whose surface the mould's is: `analyse` and `form`
//! read only that solid. Three ways in, each for either mould:
//! - `Given::Outer` (the default): a solid whose surface is the part's outside, the wall t thick
//!   inside it (a conversion, as sheet metal converts a solid). A male mould is its inward offset
//!   by t, a female one is the solid itself.
//! - `Given::Wall`: the thin-walled part itself (shelled, open at its foot); t is read from it, and a
//!   wall not of one thickness is refused. A male mould is its hollow, a female one the hollow and
//!   the wall together.
//! - `Given::Inner`: a solid whose surface is the part's inside, the sheet going outward. A male
//!   mould is the solid itself, a female one its outward offset by t.
use crate::build::{FormError, Spec, Tool, plane_frame};
use forge_bool::{Op, boolean};
use forge_math::{Point2, Point3, UnitVec3, Vec3, dmath, tol};
use forge_mesh::{Quality, TriMesh};
use forge_ops::Profile;
use forge_topo::{Body, FaceId, Name};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Given {
    Outer,
    Wall,
    Inner,
}

#[derive(Clone, Debug)]
pub struct Prepared {
    /// The solid whose surface the mould's is.
    pub shape: Body,
    /// The sheet's thickness: as given, or read from a wall.
    pub t: f64,
    /// A face of `shape` that is an offset of one the user made: (its name, the user's).
    pub origin: Vec<(Name, Name)>,
}

/// A wall's thickness may vary by this much (a fraction of it, or 0.02 mm) over all but this
/// fraction of its area before it is refused: what a shell's corners leave.
const WALL_TOL: f64 = 0.05;
const WALL_SPARE: f64 = 0.05;

fn mesh(b: &Body) -> Result<TriMesh, FormError> {
    forge_mesh::tessellate(
        b,
        &Quality {
            chord: 0.02,
            max_angle: dmath::PI / 24.0,
        },
    )
    .map_err(|_| FormError::Mesh)
}

/// The body's extent along `d`, and its flat faces at the extreme that faces `-d` (`low`) or `d`.
fn foot(b: &Body, d: UnitVec3, low: bool) -> Result<(f64, f64, Vec<Name>), FormError> {
    let m = mesh(b)?;
    let along = |p: Point3| (p - Point3::ORIGIN).dot(d.get());
    let (lo, hi) = m
        .positions
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, c), &p| {
            (a.min(along(p)), c.max(along(p)))
        });
    let z0 = if low { lo } else { hi };
    let want = if low { -1.0 } else { 1.0 };
    let names = b
        .face_ids()
        .filter(|&f| {
            let face = b.face(f);
            matches!(&face.surface, forge_geom::Surface::Plane(p)
                if (if face.same_sense { p.frame.z() } else { -p.frame.z() }).dot(d.get()) * want > 1.0 - 1e-9
                    && (along(p.frame.origin) - z0).abs() < tol::LINEAR)
        })
        .map(|f| b.face(f).name)
        .collect();
    Ok((lo, hi, names))
}

/// `body` offset by t (outward or in), opened at its foot, and the user's names of its faces.
fn offset(body: &Body, spec: &Spec, outward: bool, low: bool, feature: u64) -> Result<Prepared, FormError> {
    let (_, _, open) = foot(body, spec.pull, low)?;
    if open.is_empty() {
        return Err(FormError::NoOpening { tool: spec.tool });
    }
    let shape = forge_ops::offset_solid(body, spec.t, outward, &open, feature)
        .map_err(FormError::Offset)?
        .ok_or(FormError::Offset(forge_ops::ShellError::AllOpen))?;
    let root = Name::feature(feature);
    let origin = body
        .face_ids()
        .map(|f| body.face(f).name)
        .map(|n| (forge_ops::roles::offset(root, n), n))
        .collect();
    Ok(Prepared {
        shape,
        t: spec.t,
        origin,
    })
}

/// A body of the faces of one connected piece of `b` (its `k`th shell, as compacted).
fn piece(b: &Body, keep: &[FaceId]) -> Body {
    let mut out = b.clone();
    for f in b.face_ids() {
        if !keep.contains(&f) {
            out.kill_face(f);
        }
    }
    out.into_compact()
}

pub fn prepare(body: &Body, from: Given, spec: &Spec, feature: u64) -> Result<Prepared, FormError> {
    let male = spec.tool == Tool::Male;
    let same = |b: &Body| Prepared {
        shape: b.clone(),
        t: spec.t,
        origin: vec![],
    };
    match (from, male) {
        (Given::Outer, false) | (Given::Inner, true) => Ok(same(body)),
        (Given::Outer, true) => offset(body, spec, false, true, feature),
        (Given::Inner, false) => offset(body, spec, true, false, feature),
        (Given::Wall, _) => wall(body, spec, feature),
    }
}

/// The hollow under a wall (male) or the hollow and the wall (female), and the wall's thickness.
fn wall(w: &Body, spec: &Spec, feature: u64) -> Result<Prepared, FormError> {
    let male = spec.tool == Tool::Male;
    let d = spec.pull;
    let (lo, hi, rim) = foot(w, d, male)?;
    if rim.is_empty() {
        return Err(FormError::NoOpening { tool: spec.tool });
    }
    let z0 = if male { lo } else { hi };
    let frame = plane_frame(d, z0);
    let m = mesh(w)?;
    let (mut a, mut b) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in &m.positions {
        let l = frame.point_to_local(*p);
        a = Point2::new(a.x.min(l.x), a.y.min(l.y));
        b = Point2::new(b.x.max(l.x), b.y.max(l.y));
    }
    // a box over the wall from its foot to its far side, a little wider: less the wall, it is the
    // air round the wall and the hollow under it, apart
    let pad = 1.0 + spec.t.max(1.0);
    let id = feature ^ 0xb0c5;
    let p = Profile {
        plane: frame,
        loops: vec![Profile::rectangle(
            Point2::new(a.x - pad, a.y - pad),
            b.x - a.x + 2.0 * pad,
            b.y - a.y + 2.0 * pad,
            0,
        )],
    };
    let reach = (hi - lo) + pad;
    let boxed = forge_ops::extrude(&p, if male { reach } else { -reach }, id).map_err(FormError::Extrude)?;
    let cap = Name::feature(id).with_str("start");
    let air = boolean(&boxed, w, Op::Subtract, feature)
        .map_err(FormError::Bool)?
        .compact();
    let from_box = |n: Name| {
        boxed
            .face_ids()
            .map(|f| boxed.face(f).name)
            .any(|x| x != cap && (n == x || n.is_piece_of(x)))
    };
    let hollows: Vec<&forge_topo::Shell> = air
        .shells
        .iter()
        .filter(|s| !s.faces.iter().any(|&f| from_box(air.face(f).name)))
        .collect();
    if hollows.is_empty() {
        return Err(FormError::NotAWall);
    }
    let keep: Vec<FaceId> = hollows.iter().flat_map(|s| s.faces.iter().copied()).collect();
    let hollow = piece(&air, &keep);
    let t = thickness(&hollow, w, cap)?;
    let shape = if male {
        hollow
    } else {
        boolean(w, &hollow, Op::Union, feature).map_err(FormError::Bool)?
    };
    Ok(Prepared {
        shape,
        t,
        origin: vec![],
    })
}

/// The wall's thickness: from each triangle of the hollow's skin (not its cap) to the wall's
/// nearest triangle, by area; refused where it is not one.
fn thickness(hollow: &Body, w: &Body, cap: Name) -> Result<f64, FormError> {
    let (hm, wm) = (mesh(hollow)?, mesh(w)?);
    let inner: Vec<Name> = hollow.face_ids().map(|f| hollow.face(f).name).collect();
    let skin = |m: &TriMesh, b: &Body, k: usize, outer: bool| {
        let n = b.face(FaceId(m.face_of[k])).name;
        if outer {
            !inner.contains(&n)
        } else {
            n != cap && !n.is_piece_of(cap)
        }
    };
    let tris = |m: &TriMesh, k: usize| m.triangles[k].map(|i| m.positions[i as usize]);
    let outer: Vec<[Point3; 3]> = (0..wm.triangles.len())
        .filter(|&k| skin(&wm, w, k, true))
        .map(|k| tris(&wm, k))
        .collect();
    let mut seen: Vec<(f64, f64, Point3)> = vec![];
    for k in (0..hm.triangles.len()).filter(|&k| skin(&hm, hollow, k, false)) {
        let [a, b, c] = tris(&hm, k);
        let area = 0.5 * (b - a).cross(c - a).length();
        let mid = Point3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        let d = outer
            .iter()
            .map(|t| to_triangle(mid, t))
            .fold(f64::INFINITY, f64::min);
        seen.push((d, area, mid));
    }
    if seen.is_empty() {
        return Err(FormError::NotAWall);
    }
    // the usual thickness: the median by area
    let mut by: Vec<(f64, f64)> = seen.iter().map(|s| (s.0, s.1)).collect();
    by.sort_by(|x, y| x.0.total_cmp(&y.0));
    let total: f64 = by.iter().map(|s| s.1).sum();
    let mut acc = 0.0;
    let t = by
        .iter()
        .find(|s| {
            acc += s.1;
            acc >= 0.5 * total
        })
        .map_or(by[0].0, |s| s.0);
    let off: f64 = seen
        .iter()
        .filter(|s| (s.0 - t).abs() > (WALL_TOL * t).max(0.02))
        .map(|s| s.1)
        .sum();
    if off > WALL_SPARE * total {
        let min = seen.iter().min_by(|x, y| x.0.total_cmp(&y.0)).expect("some");
        let max = seen.iter().max_by(|x, y| x.0.total_cmp(&y.0)).expect("some");
        return Err(FormError::WallNotUniform {
            t,
            min: (min.0, min.2),
            max: (max.0, max.2),
        });
    }
    Ok(t)
}

/// The distance from `p` to a triangle.
fn to_triangle(p: Point3, t: &[Point3; 3]) -> f64 {
    let [a, b, c] = *t;
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let n = ab.cross(ac);
    let nn = n.dot(n);
    if nn > 0.0 {
        // the foot of the perpendicular, if it falls inside
        let u = ap.cross(ac).dot(n) / nn;
        let v = ab.cross(ap).dot(n) / nn;
        if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
            return (n.dot(ap) / nn.sqrt()).abs();
        }
    }
    let seg = |a: Point3, b: Point3| {
        let ab: Vec3 = b - a;
        let l = ab.dot(ab);
        let f = if l > 0.0 {
            ((p - a).dot(ab) / l).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (p - (a + ab * f)).length()
    };
    seg(a, b).min(seg(b, c)).min(seg(c, a))
}
