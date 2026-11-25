//! A body's faces for the checker where `lower` cannot take them exactly: each face's own surface
//! over a parameter box that encloses the face (no trim). The enclosure is proven from the
//! face's box (`Body::face_box`: the hull of its edges' boxes, and for spheres and tori the
//! surface's own extent), which holds the face:
//! - a plane's face lies in the hull of its boundary;
//! - a cylinder's or a cone's face is ruled: every point lies on a generator between two points
//!   of its boundary, so in the hull of the boundary too;
//! - spheres' and tori's boxes are grown by the kernel to hold the surface over the face's span.
//!
//! The box's corners, projected into the surface's frame, bound the face's parameters: a plane's
//! (p, q) by the corners' range; a cylinder's or cone's height by the corners' range along the
//! axis, its angle by the corners' angles about the axis when the corners' rectangle across the
//! axis does not hold the axis (a convex set the axis is outside of spans less than half a turn,
//! its extreme angles at its corners), else the whole turn. A superset of the face is checked:
//! a clearance proven against it holds for the face; a collision may be reported that the face
//! itself does not make (conservative).
//!
//! Procedural and NURBS faces are not enclosed here (`None`): the workholding compiler meshes
//! them with forge-mesh's certified triangles.
use crate::lower::{LowerError, lower_face, mu};
use forge_cam_geom::check::{Face, Surf, V3};
use forge_cam_geom::ival::Ival;
use forge_geom::surface::Surface;
use forge_math::{Point3, Vec3, dmath};
use forge_topo::{Body, FaceId};

/// How a face went to the checker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    /// `lower`'s exact face (its trim kept).
    Exact,
    /// Its own surface over a box of parameters enclosing it.
    Enclosed,
}

fn v3(v: Vec3) -> V3 {
    [v.x, v.y, v.z]
}
fn p3(p: Point3) -> V3 {
    [p.x, p.y, p.z]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The face's box's 8 corners.
fn corners(body: &Body, fid: FaceId) -> [V3; 8] {
    let b = body.face_box(fid);
    let mut c = [[0.0; 3]; 8];
    for (i, p) in c.iter_mut().enumerate() {
        *p = [
            if i & 1 == 0 { b.x.lo } else { b.x.hi },
            if i & 2 == 0 { b.y.lo } else { b.y.hi },
            if i & 4 == 0 { b.z.lo } else { b.z.hi },
        ];
    }
    c
}

/// The angles about the axis (frame origin o, x, y) that points of the box can have: a range
/// `(a0, a1)` with a0 < a1 ≤ a0 + 2π (the whole turn when the box's rectangle across the axis
/// holds it), padded by `pad` radians.
fn angle_span(c: &[V3; 8], o: V3, x: V3, y: V3, pad: f64) -> (f64, f64) {
    let tau = std::f64::consts::TAU;
    // the corners across the axis: their rectangle in (x, y) holds the box's projection
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for p in c {
        let d = sub(*p, o);
        let q = [dot(d, x), dot(d, y)];
        for i in 0..2 {
            lo[i] = lo[i].min(q[i]);
            hi[i] = hi[i].max(q[i]);
        }
    }
    if lo[0] <= 0.0 && hi[0] >= 0.0 && lo[1] <= 0.0 && hi[1] >= 0.0 {
        return (0.0, tau);
    }
    // outside the rectangle: its angles are within less than half a turn, extreme at its corners;
    // measured from the rectangle's middle's angle so none wraps
    let mid = dmath::atan2(0.5 * (lo[1] + hi[1]), 0.5 * (lo[0] + hi[0]));
    let (mut a0, mut a1) = (f64::INFINITY, f64::NEG_INFINITY);
    for (u, v) in [(lo[0], lo[1]), (hi[0], lo[1]), (lo[0], hi[1]), (hi[0], hi[1])] {
        let mut a = dmath::atan2(v, u) - mid;
        while a > std::f64::consts::PI {
            a -= tau;
        }
        while a <= -std::f64::consts::PI {
            a += tau;
        }
        a0 = a0.min(a);
        a1 = a1.max(a);
    }
    (mid + a0 - pad, mid + a1 + pad)
}

/// Pieces of at most a quarter turn over (a0, a1): each piece's middle angle and its half width's
/// half tangent (the rational map's parameter bound), as `lower`'s `revolve` cuts them.
fn pieces(a0: f64, a1: f64) -> Vec<(f64, f64)> {
    let w = a1 - a0;
    let n = ((w / std::f64::consts::FRAC_PI_2) - 1e-12).ceil().max(1.0) as usize;
    let half = 0.5 * w / n as f64;
    let th = dmath::sin(0.5 * half) / dmath::cos(0.5 * half) * (1.0 + 1e-12);
    (0..n).map(|i| (a0 + half * (2 * i + 1) as f64, th)).collect()
}

fn turn(x: V3, y: V3, a: f64) -> (V3, V3) {
    let (s, c) = dmath::sin_cos(a);
    (
        [c * x[0] + s * y[0], c * x[1] + s * y[1], c * x[2] + s * y[2]],
        [-s * x[0] + c * y[0], -s * x[1] + c * y[1], -s * x[2] + c * y[2]],
    )
}

/// The face for the checker: `lower`'s exact face when it takes it, else its surface over an
/// enclosing parameter box; `Err` for a procedural or NURBS face (left to the mesh).
pub fn enclose_face(body: &Body, fid: FaceId, k: usize, out: &mut Vec<Face>) -> Result<How, LowerError> {
    let n0 = out.len();
    let why = match lower_face(body, fid, k, out) {
        Ok(()) => return Ok(How::Exact),
        Err(e) => {
            out.truncate(n0);
            e
        }
    };
    let bb = body.face_box(fid);
    let scale = [bb.x.lo, bb.y.lo, bb.z.lo, bb.x.hi, bb.y.hi, bb.z.hi]
        .iter()
        .fold(0.0f64, |m, x| m.max(x.abs()));
    let m = mu(scale);
    let c = corners(body, fid);
    let pad = 1e-9;
    let range = |o: V3, d: V3| {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &c {
            let t = dot(sub(*p, o), d);
            lo = lo.min(t);
            hi = hi.max(t);
        }
        Ival::new(lo - m, hi + m)
    };
    let f = body.face(fid);
    match &f.surface {
        Surface::Plane(pl) => {
            let o = p3(pl.frame.origin);
            let (x, y) = (v3(pl.frame.x().get()), v3(pl.frame.y().get()));
            out.push(Face::new(
                Surf::Plane { o, u: x, v: y },
                range(o, x),
                range(o, y),
                None,
                m,
                k as u32,
            ));
        }
        Surface::Cylinder(cy) => {
            let fr = cy.frame;
            let o = p3(fr.origin);
            let (x, y, z) = (v3(fr.x().get()), v3(fr.y().get()), v3(fr.z().get()));
            let (a0, a1) = angle_span(&c, o, x, y, pad);
            let q = range(o, z);
            for (am, th) in pieces(a0, a1) {
                let (xm, ym) = turn(x, y, am);
                let surf = Surf::Cyl {
                    o,
                    x: xm,
                    y: ym,
                    ax: z,
                    r: cy.radius,
                };
                out.push(Face::new(surf, Ival::new(-th, th), q, None, m, k as u32));
            }
        }
        Surface::Cone(cn) => {
            let fr = cn.frame;
            let o = p3(fr.origin);
            let (x, y, z) = (v3(fr.x().get()), v3(fr.y().get()), v3(fr.z().get()));
            let (a0, a1) = angle_span(&c, o, x, y, pad);
            let q = range(o, z);
            for (am, th) in pieces(a0, a1) {
                let (xm, ym) = turn(x, y, am);
                let surf = Surf::Cone {
                    o,
                    x: xm,
                    y: ym,
                    ax: z,
                    r0: cn.radius,
                    k: cn.tan_a(),
                };
                out.push(Face::new(surf, Ival::new(-th, th), q, None, m, k as u32));
            }
        }
        Surface::Sphere(sp) => {
            // azimuth from the box; elevation the whole half turn (q = tan(φ/2) over φ in
            // [−π/2, π/2]: [−1, 1])
            let fr = sp.frame;
            let o = p3(fr.origin);
            let (x, y, z) = (v3(fr.x().get()), v3(fr.y().get()), v3(fr.z().get()));
            let (a0, a1) = angle_span(&c, o, x, y, pad);
            let e = 1.0 + 1e-12;
            for (am, th) in pieces(a0, a1) {
                let (xm, ym) = turn(x, y, am);
                let surf = Surf::Sphere {
                    o,
                    x: xm,
                    y: ym,
                    ax: z,
                    r: sp.radius,
                };
                out.push(Face::new(
                    surf,
                    Ival::new(-th, th),
                    Ival::new(-e, e),
                    None,
                    m,
                    k as u32,
                ));
            }
        }
        Surface::Torus(t) if !t.is_spindle() => {
            // the angle about the axis from the box; the tube's whole turn, in quarters about
            // their middles (q = tan(ψ/2) over ψ in a quarter turn)
            let fr = t.frame;
            let o = p3(fr.origin);
            let (x, y, z) = (v3(fr.x().get()), v3(fr.y().get()), v3(fr.z().get()));
            let (a0, a1) = angle_span(&c, o, x, y, pad);
            let tq = dmath::sin(std::f64::consts::FRAC_PI_8) / dmath::cos(std::f64::consts::FRAC_PI_8)
                * (1.0 + 1e-12);
            for (am, th) in pieces(a0, a1) {
                let (xm, ym) = turn(x, y, am);
                for j in 0..4 {
                    let (sm, cm) = dmath::sin_cos(std::f64::consts::FRAC_PI_2 * j as f64);
                    let surf = Surf::Torus {
                        o,
                        x: xm,
                        y: ym,
                        ax: z,
                        big: t.major,
                        r: t.minor,
                        cm,
                        sm,
                    };
                    out.push(Face::new(
                        surf,
                        Ival::new(-th, th),
                        Ival::new(-tq, tq),
                        None,
                        m,
                        k as u32,
                    ));
                }
            }
        }
        _ => return Err(why),
    }
    Ok(How::Enclosed)
}
